use catchlight_core::Result;
use catchlight_core::media::DecodedImage;
use image::DynamicImage;
use image::imageops::FilterType;
use std::path::Path;

const DCT_SIZE: usize = 32;
const HASH_BLOCK: usize = 8;

#[allow(clippy::needless_range_loop)]
fn dct_1d(input: &[f64; DCT_SIZE]) -> [f64; DCT_SIZE] {
    let n = DCT_SIZE as f64;
    let mut output = [0.0f64; DCT_SIZE];
    for k in 0..DCT_SIZE {
        let mut sum = 0.0;
        for (i, &val) in input.iter().enumerate().take(DCT_SIZE) {
            sum += val * ((std::f64::consts::PI * (2 * i + 1) as f64 * k as f64) / (2.0 * n)).cos();
        }
        let scale = if k == 0 {
            (1.0 / n).sqrt()
        } else {
            (2.0 / n).sqrt()
        };
        output[k] = scale * sum;
    }
    output
}

fn dct_2d(input: &[[f64; DCT_SIZE]; DCT_SIZE]) -> [[f64; DCT_SIZE]; DCT_SIZE] {
    let mut temp = [[0.0f64; DCT_SIZE]; DCT_SIZE];
    let mut output = [[0.0f64; DCT_SIZE]; DCT_SIZE];

    for y in 0..DCT_SIZE {
        temp[y] = dct_1d(&input[y]);
    }

    for x in 0..DCT_SIZE {
        let mut col = [0.0f64; DCT_SIZE];
        for y in 0..DCT_SIZE {
            col[y] = temp[y][x];
        }
        let dct_col = dct_1d(&col);
        for y in 0..DCT_SIZE {
            output[y][x] = dct_col[y];
        }
    }
    output
}

/// Compute a 64-bit perceptual hash using DCT on a 32×32 grayscale image.
#[allow(clippy::needless_range_loop)]
pub fn compute_phash(img: &DynamicImage) -> u64 {
    let gray = img
        .resize_exact(DCT_SIZE as u32, DCT_SIZE as u32, FilterType::Lanczos3)
        .to_luma8();

    let mut pixels = [[0.0f64; DCT_SIZE]; DCT_SIZE];
    for y in 0..DCT_SIZE {
        for x in 0..DCT_SIZE {
            pixels[y][x] = f64::from(gray.get_pixel(x as u32, y as u32)[0]);
        }
    }

    let dct = dct_2d(&pixels);

    let mut block = [[0.0f64; HASH_BLOCK]; HASH_BLOCK];
    for y in 0..HASH_BLOCK {
        for x in 0..HASH_BLOCK {
            block[y][x] = dct[y][x];
        }
    }

    let mean: f64 = block
        .iter()
        .flat_map(|row| row.iter())
        .copied()
        .sum::<f64>()
        / (HASH_BLOCK * HASH_BLOCK) as f64;

    let mut hash: u64 = 0;
    for y in 0..HASH_BLOCK {
        for x in 0..HASH_BLOCK {
            if block[y][x] > mean {
                hash |= 1 << (y * HASH_BLOCK + x);
            }
        }
    }
    hash
}

pub fn compute_dhash(path: &Path) -> Result<u64> {
    let img = image::open(path).map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

    let gray = img.resize_exact(9, 8, FilterType::Lanczos3).to_luma8();
    let mut hash: u64 = 0;

    for y in 0..8 {
        for x in 0..8 {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1 << (y * 8 + x);
            }
        }
    }

    Ok(hash)
}

pub fn dhash_from_decoded(decoded: &DecodedImage) -> u64 {
    let img = decoded.to_dynamic_image();
    let gray = img
        .resize_exact(9, 8, image::imageops::FilterType::Lanczos3)
        .to_luma8();
    let mut hash: u64 = 0;
    for y in 0..8 {
        for x in 0..8 {
            if gray.get_pixel(x, y)[0] > gray.get_pixel(x + 1, y)[0] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    hash
}

pub fn phash_from_decoded(decoded: &DecodedImage) -> u64 {
    compute_phash(&decoded.to_dynamic_image())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hamming_distance;
    use image::{ImageBuffer, Rgb, RgbImage};
    use std::fs;

    fn solid_image(width: u32, height: u32, value: u8) -> RgbImage {
        ImageBuffer::from_fn(width, height, |_, _| Rgb([value, value, value]))
    }

    fn save_image(img: &RgbImage, path: &Path) {
        img.save(path).expect("test image should save");
    }

    #[test]
    fn solid_white_dhash_is_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("white.png");
        save_image(&solid_image(64, 64, 255), &path);

        let h1 = compute_dhash(&path).unwrap();
        let h2 = compute_dhash(&path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn solid_black_dhash_is_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("black.png");
        save_image(&solid_image(64, 64, 0), &path);

        let h1 = compute_dhash(&path).unwrap();
        let h2 = compute_dhash(&path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn white_and_black_have_very_different_dhashes() {
        let dir = tempfile::tempdir().unwrap();
        let white_path = dir.path().join("white.png");
        let black_path = dir.path().join("black.png");
        let split_path = dir.path().join("split.png");

        save_image(&solid_image(64, 64, 255), &white_path);
        save_image(&solid_image(64, 64, 0), &black_path);

        let split: RgbImage = ImageBuffer::from_fn(64, 64, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255, 255, 255])
            } else {
                Rgb([0, 0, 0])
            }
        });
        save_image(&split, &split_path);

        let white_hash = compute_dhash(&white_path).unwrap();
        let black_hash = compute_dhash(&black_path).unwrap();
        let split_hash = compute_dhash(&split_path).unwrap();

        let white_vs_split = hamming_distance(white_hash, split_hash);
        let black_vs_split = hamming_distance(black_hash, split_hash);
        assert!(
            white_vs_split >= 10,
            "white vs black/white pattern distance should be high (got {white_vs_split})"
        );
        assert!(
            black_vs_split >= 10,
            "black vs black/white pattern distance should be high (got {black_vs_split})"
        );
        assert_ne!(white_hash, split_hash);
        assert_ne!(black_hash, split_hash);
    }

    #[test]
    fn identical_images_have_same_dhash() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.png");
        let path_b = dir.path().join("b.png");

        let img = solid_image(64, 64, 128);
        save_image(&img, &path_a);
        fs::copy(&path_a, &path_b).unwrap();

        let hash_a = compute_dhash(&path_a).unwrap();
        let hash_b = compute_dhash(&path_b).unwrap();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn solid_white_phash_is_consistent() {
        let img = image::DynamicImage::ImageRgb8(solid_image(64, 64, 255));
        let h1 = compute_phash(&img);
        let h2 = compute_phash(&img);
        assert_eq!(h1, h2);
    }

    #[test]
    fn solid_black_phash_is_consistent() {
        let img = image::DynamicImage::ImageRgb8(solid_image(64, 64, 0));
        let h1 = compute_phash(&img);
        let h2 = compute_phash(&img);
        assert_eq!(h1, h2);
    }

    #[test]
    fn identical_images_have_same_phash() {
        let img_a = image::DynamicImage::ImageRgb8(solid_image(64, 64, 128));
        let img_b = image::DynamicImage::ImageRgb8(solid_image(64, 64, 128));
        assert_eq!(compute_phash(&img_a), compute_phash(&img_b));
    }

    #[test]
    fn different_structures_have_different_phashes() {
        let left_right: RgbImage = ImageBuffer::from_fn(64, 64, |x, _| {
            if x < 32 {
                Rgb([255, 255, 255])
            } else {
                Rgb([0, 0, 0])
            }
        });
        let top_bottom: RgbImage = ImageBuffer::from_fn(64, 64, |_, y| {
            if y < 32 {
                Rgb([255, 255, 255])
            } else {
                Rgb([0, 0, 0])
            }
        });
        let lr_hash = compute_phash(&image::DynamicImage::ImageRgb8(left_right));
        let tb_hash = compute_phash(&image::DynamicImage::ImageRgb8(top_bottom));
        assert_ne!(lr_hash, tb_hash);
        assert!(hamming_distance(lr_hash, tb_hash) >= 1);
    }

    #[test]
    fn phash_from_decoded_matches_compute_phash() {
        let rgba: RgbImage = solid_image(48, 48, 200);
        let width = rgba.width();
        let height = rgba.height();
        let mut rgba_buf = vec![0u8; (width * height * 4) as usize];
        for (i, pixel) in rgba.pixels().enumerate() {
            let base = i * 4;
            rgba_buf[base] = pixel[0];
            rgba_buf[base + 1] = pixel[1];
            rgba_buf[base + 2] = pixel[2];
            rgba_buf[base + 3] = 255;
        }
        let decoded = DecodedImage {
            rgba: rgba_buf,
            width,
            height,
        };
        let from_decoded = phash_from_decoded(&decoded);
        let from_dynamic = compute_phash(&decoded.to_dynamic_image());
        assert_eq!(from_decoded, from_dynamic);
    }
}

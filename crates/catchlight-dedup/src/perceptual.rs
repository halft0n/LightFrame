use catchlight_core::media::DecodedImage;
use catchlight_core::Result;
use image::imageops::FilterType;
use std::path::Path;

pub fn compute_dhash(path: &Path) -> Result<u64> {
    let img = image::open(path)
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

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
}

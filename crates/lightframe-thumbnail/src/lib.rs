use lightframe_core::Result;
use lightframe_core::config;
use lightframe_core::decode::{self, is_heic_path};
use lightframe_core::media::{DecodedImage, ThumbnailSize};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

pub fn thumb_path(blake3_hash: &str, size: ThumbnailSize) -> PathBuf {
    let cache_dir = config::thumb_cache_dir();
    if blake3_hash.len() < 4 {
        return cache_dir
            .join("_invalid")
            .join(format!("{blake3_hash}.webp"));
    }
    let prefix = &blake3_hash[..2];
    let sub = &blake3_hash[2..4];
    let size_str = match size {
        ThumbnailSize::Micro => "micro",
        ThumbnailSize::Small => "small",
        ThumbnailSize::Large => "large",
    };
    cache_dir
        .join(prefix)
        .join(sub)
        .join(format!("{blake3_hash}_{size_str}.webp"))
}

fn open_source_image(src: &Path) -> Result<image::DynamicImage> {
    if is_heic_path(src) {
        warn!(
            path = %src.display(),
            "HEIC/HEIF thumbnail skipped: optional libheif support not enabled"
        );
    }

    let decoded = decode::decode_image(src)?;
    Ok(decoded.to_dynamic_image())
}

pub fn generate(src: &Path, hash: &str, size: ThumbnailSize) -> Result<PathBuf> {
    let out = thumb_path(hash, size);

    if out.exists() && !thumb_file_needs_regeneration(&out) {
        return Ok(out);
    }

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img =
        open_source_image(src).map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    let pixels = size.pixels();
    let thumb = img.thumbnail(pixels, pixels);

    thumb
        .save(&out)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    debug!(path = %out.display(), size = pixels, "thumbnail generated");
    Ok(out)
}

pub fn generate_micro_blob(src: &Path) -> Result<Vec<u8>> {
    let img =
        open_source_image(src).map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    let thumb = img.thumbnail(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());

    thumb
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    Ok(buf.into_inner())
}

/// Returns true when the cached thumbnail file is missing or empty (corrupt).
pub fn thumb_file_needs_regeneration(path: &Path) -> bool {
    match path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    }
}

pub fn generate_from_decoded(
    decoded: &DecodedImage,
    hash: &str,
    size: ThumbnailSize,
) -> Result<PathBuf> {
    let out = thumb_path(hash, size);
    if out.exists() && !thumb_file_needs_regeneration(&out) {
        return Ok(out);
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let img = decoded.to_dynamic_image();
    let pixels = size.pixels();
    let thumb = img.thumbnail(pixels, pixels);
    thumb
        .save(&out)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    debug!(path = %out.display(), size = pixels, "thumbnail generated");
    Ok(out)
}

/// Force-regenerates a thumbnail from a decoded image, overwriting any existing file.
pub fn regenerate_from_decoded(
    decoded: &DecodedImage,
    hash: &str,
    size: ThumbnailSize,
) -> Result<PathBuf> {
    let out = thumb_path(hash, size);
    if out.exists() {
        let _ = std::fs::remove_file(&out);
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let img = decoded.to_dynamic_image();
    let pixels = size.pixels();
    let thumb = img.thumbnail(pixels, pixels);
    thumb
        .save(&out)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    debug!(path = %out.display(), size = pixels, "thumbnail regenerated");
    Ok(out)
}

/// Force-regenerates a thumbnail from a source file, overwriting any existing file.
pub fn regenerate(src: &Path, hash: &str, size: ThumbnailSize) -> Result<PathBuf> {
    let out = thumb_path(hash, size);
    if out.exists() {
        let _ = std::fs::remove_file(&out);
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let img =
        open_source_image(src).map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    let pixels = size.pixels();
    let thumb = img.thumbnail(pixels, pixels);
    thumb
        .save(&out)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    debug!(path = %out.display(), size = pixels, "thumbnail regenerated");
    Ok(out)
}

pub fn micro_blob_from_decoded(decoded: &DecodedImage) -> Result<Vec<u8>> {
    let img = decoded.to_dynamic_image();
    let thumb = img.thumbnail(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn generate_heic_skips_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let heic = dir.path().join("photo.heic");
        std::fs::write(&heic, b"fake heic").unwrap();
        let hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let err = generate(&heic, hash, ThumbnailSize::Small).unwrap_err();
        assert!(err.to_string().contains("HEIC"));
    }

    #[test]
    fn generate_avif_thumbnail_when_decode_available() {
        let dir = tempfile::tempdir().unwrap();
        let avif = dir.path().join("photo.avif");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(32, 32, |x, y| Rgb([(x * 8) as u8, (y * 8) as u8, 64]));
        img.save_with_format(&avif, image::ImageFormat::Avif)
            .expect("write avif");

        if lightframe_core::decode::decode_image(&avif).is_err() {
            return;
        }

        let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let out = generate(&avif, hash, ThumbnailSize::Small).expect("generate avif thumb");
        assert!(out.exists());
    }
}

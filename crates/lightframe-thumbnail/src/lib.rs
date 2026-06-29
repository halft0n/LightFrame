use lightframe_core::Result;
use lightframe_core::config;
use lightframe_core::media::{DecodedImage, ThumbnailSize};
use std::path::{Path, PathBuf};
use tracing::debug;

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

pub fn generate(src: &Path, hash: &str, size: ThumbnailSize) -> Result<PathBuf> {
    let out = thumb_path(hash, size);

    if out.exists() {
        return Ok(out);
    }

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img = image::open(src).map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    let pixels = size.pixels();
    let thumb = img.thumbnail(pixels, pixels);

    thumb
        .save(&out)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    debug!(path = %out.display(), size = pixels, "thumbnail generated");
    Ok(out)
}

pub fn generate_micro_blob(src: &Path) -> Result<Vec<u8>> {
    let img = image::open(src).map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    let thumb = img.thumbnail(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());

    thumb
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;

    Ok(buf.into_inner())
}

pub fn generate_from_decoded(
    decoded: &DecodedImage,
    hash: &str,
    size: ThumbnailSize,
) -> Result<PathBuf> {
    let out = thumb_path(hash, size);
    if out.exists() {
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

pub fn micro_blob_from_decoded(decoded: &DecodedImage) -> Result<Vec<u8>> {
    let img = decoded.to_dynamic_image();
    let thumb = img.thumbnail(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| lightframe_core::Error::Thumbnail(e.to_string()))?;
    Ok(buf.into_inner())
}

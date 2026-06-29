use crate::Result;
use crate::media::DecodedImage;
use std::path::Path;

// Optional decode backends (enable via Cargo features, not enabled by default):
// - HEIC/HEIF: requires `libheif` (C library) — not wired in; files are indexed but skipped.
// - AVIF decode: `image` crate's `avif-native` feature (libdav1d) via `lightframe-core/avif-native`.
//   AVIF encoding works with the default `avif` feature; decoding needs the native backend.

/// Image formats handled explicitly by the decode pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormatKind {
    Avif,
    Heic,
    Other,
}

pub fn file_extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

pub fn detect_image_format(path: &Path) -> ImageFormatKind {
    match file_extension_lower(path).as_deref() {
        Some("avif") => ImageFormatKind::Avif,
        Some("heic" | "heif") => ImageFormatKind::Heic,
        _ => ImageFormatKind::Other,
    }
}

pub fn is_heic_path(path: &Path) -> bool {
    matches!(detect_image_format(path), ImageFormatKind::Heic)
}

pub fn is_avif_path(path: &Path) -> bool {
    matches!(detect_image_format(path), ImageFormatKind::Avif)
}

/// Returns false for HEIC/HEIF, which require optional native libheif support.
pub fn is_decode_supported(path: &Path) -> bool {
    !is_heic_path(path)
}

pub fn decode_image(path: &Path) -> Result<DecodedImage> {
    if is_heic_path(path) {
        return Err(crate::Error::Other(
            "HEIC/HEIF decoding requires optional libheif; skipping decode".into(),
        ));
    }

    let img = image::open(path).map_err(|e| {
        let prefix = match detect_image_format(path) {
            ImageFormatKind::Avif => "AVIF decode failed",
            ImageFormatKind::Heic => "HEIC decode failed",
            ImageFormatKind::Other => "decode failed",
        };
        crate::Error::Other(format!("{prefix}: {e}"))
    })?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn detect_avif_and_heic_extensions() {
        assert_eq!(
            detect_image_format(Path::new("/photos/vacation.avif")),
            ImageFormatKind::Avif
        );
        assert_eq!(
            detect_image_format(Path::new("/photos/vacation.AVIF")),
            ImageFormatKind::Avif
        );
        assert_eq!(
            detect_image_format(Path::new("/photos/vacation.heic")),
            ImageFormatKind::Heic
        );
        assert_eq!(
            detect_image_format(Path::new("/photos/vacation.heif")),
            ImageFormatKind::Heic
        );
        assert_eq!(
            detect_image_format(Path::new("/photos/vacation.jpg")),
            ImageFormatKind::Other
        );
    }

    #[test]
    fn heic_is_not_decode_supported() {
        assert!(!is_decode_supported(Path::new("photo.heic")));
        assert!(!is_decode_supported(Path::new("photo.heif")));
        assert!(is_decode_supported(Path::new("photo.avif")));
        assert!(is_decode_supported(Path::new("photo.jpg")));
    }

    #[test]
    fn decode_heic_returns_error_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.heic");
        std::fs::write(&path, b"not a real heic").unwrap();

        match decode_image(&path) {
            Err(e) => assert!(e.to_string().contains("HEIC/HEIF")),
            Ok(_) => panic!("expected HEIC decode to fail"),
        }
    }

    #[test]
    fn decode_avif_roundtrip_or_graceful_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.avif");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 30) as u8, (y * 30) as u8, 128]));
        img.save_with_format(&path, image::ImageFormat::Avif)
            .expect("write avif");

        match decode_image(&path) {
            Ok(decoded) => {
                assert_eq!(decoded.width, 8);
                assert_eq!(decoded.height, 8);
                assert_eq!(decoded.rgba.len(), 8 * 8 * 4);
            }
            Err(e) => {
                assert!(
                    e.to_string().contains("AVIF"),
                    "expected AVIF-specific decode error, got: {e}"
                );
            }
        }
    }
}

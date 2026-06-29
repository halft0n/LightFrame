use crate::Result;
use crate::media::DecodedImage;
use exif::{In, Tag};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

// Optional decode backends (enable via Cargo features, not enabled by default):
// - HEIC/HEIF: requires `libheif` (C library) — not wired in; files are indexed but skipped.
// - AVIF decode: `image` crate's `avif-native` feature (libdav1d) via `lightframe-core/avif-native`.
//   AVIF encoding works with the default `avif` feature; decoding needs the native backend.

/// Recognized RAW camera extensions (TIFF-based and related container formats).
pub const RAW_EXTENSIONS: &[&str] = &[
    "raw", "cr2", "cr3", "nef", "nrw", "arw", "dng", "orf", "rw2", "pef", "raf", "rwl", "3fr",
    "srw",
];

const SCAN_CHUNK: usize = 512 * 1024;
const MAX_JPEG_SIZE: u64 = 30 * 1024 * 1024;
const MIN_JPEG_SIZE: usize = 128;

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

pub fn is_raw_path(path: &Path) -> bool {
    file_extension_lower(path).is_some_and(|ext| RAW_EXTENSIONS.contains(&ext.as_str()))
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

/// Extract embedded JPEG preview bytes from RAW files without full demosaic decode.
///
/// Most RAW formats store a larger preview JPEG plus a smaller EXIF thumbnail.
/// This tries the largest embedded JPEG first, then the EXIF thumbnail.
pub fn extract_raw_preview_bytes(path: &Path) -> Result<Vec<u8>> {
    if let Ok(jpeg) = scan_largest_jpeg(path) {
        return Ok(jpeg);
    }

    extract_exif_thumbnail(path)
}

/// Extract and decode embedded JPEG preview from RAW files.
pub fn extract_raw_preview(path: &Path) -> Result<DecodedImage> {
    let jpeg = extract_raw_preview_bytes(path)?;
    decode_jpeg_bytes(&jpeg)
}

fn decode_jpeg_bytes(jpeg: &[u8]) -> Result<DecodedImage> {
    let img = image::load_from_memory(jpeg).map_err(|e| {
        crate::Error::Other(format!("embedded RAW preview JPEG decode failed: {e}"))
    })?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn extract_exif_thumbnail(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new()
        .read_from_container(&mut reader)
        .map_err(|e| crate::Error::Other(format!("EXIF read failed: {e}")))?;

    for ifd in [In::THUMBNAIL, In::PRIMARY] {
        let Some(offset) = exif
            .get_field(Tag::JPEGInterchangeFormat, ifd)
            .and_then(|f| f.value.get_uint(0))
        else {
            continue;
        };
        let Some(length) = exif
            .get_field(Tag::JPEGInterchangeFormatLength, ifd)
            .and_then(|f| f.value.get_uint(0))
        else {
            continue;
        };
        if length == 0 {
            continue;
        }
        if let Ok(jpeg) = read_jpeg_at_offset(path, u64::from(offset), u64::from(length))
            && is_valid_jpeg(&jpeg)
        {
            return Ok(jpeg);
        }
    }

    Err(crate::Error::Other(
        "no embedded JPEG preview found in RAW file".into(),
    ))
}

fn read_jpeg_at_offset(path: &Path, offset: u64, length: u64) -> Result<Vec<u8>> {
    if length > MAX_JPEG_SIZE {
        return Err(crate::Error::Other(
            "embedded JPEG segment too large".into(),
        ));
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let len = length as usize;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn scan_largest_jpeg(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < 4 {
        return Err(crate::Error::Other("file too small for JPEG scan".into()));
    }

    let mut best: Option<Vec<u8>> = None;
    let mut scan_pos: u64 = 0;

    while scan_pos < file_len {
        let chunk_start = scan_pos.saturating_sub(1);
        file.seek(SeekFrom::Start(chunk_start))?;
        let chunk_end = (chunk_start + SCAN_CHUNK as u64).min(file_len);
        let chunk_len = (chunk_end - chunk_start) as usize;
        let mut chunk = vec![0u8; chunk_len];
        file.read_exact(&mut chunk)?;

        for i in 0..chunk.len().saturating_sub(1) {
            if chunk[i] != 0xFF || chunk[i + 1] != 0xD8 {
                continue;
            }
            let abs_start = chunk_start + i as u64;
            let remaining = file_len - abs_start;
            if let Ok(jpeg) = extract_jpeg_segment(path, abs_start, remaining)
                && is_valid_jpeg(&jpeg)
                && best
                    .as_ref()
                    .is_none_or(|current| jpeg.len() > current.len())
            {
                best = Some(jpeg);
            }
        }

        scan_pos = scan_pos.saturating_add(SCAN_CHUNK as u64);
        if scan_pos == 0 {
            break;
        }
    }

    best.ok_or_else(|| crate::Error::Other("no embedded JPEG found via scan".into()))
}

fn extract_jpeg_segment(path: &Path, start: u64, max_len: u64) -> Result<Vec<u8>> {
    let read_len = max_len.min(MAX_JPEG_SIZE) as usize;
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buf = vec![0u8; read_len];
    let n = file.read(&mut buf)?;
    buf.truncate(n);

    let Some(eoi_pos) = find_eoi(&buf) else {
        return Err(crate::Error::Other("JPEG EOI marker not found".into()));
    };
    buf.truncate(eoi_pos + 2);
    Ok(buf)
}

fn find_eoi(data: &[u8]) -> Option<usize> {
    data.windows(2).rposition(|w| w == [0xFF, 0xD9])
}

fn is_valid_jpeg(data: &[u8]) -> bool {
    data.len() >= MIN_JPEG_SIZE
        && data.starts_with(&[0xFF, 0xD8])
        && data.ends_with(&[0xFF, 0xD9])
        && image::load_from_memory(data).is_ok()
}

pub fn decode_image(path: &Path) -> Result<DecodedImage> {
    if is_heic_path(path) {
        return Err(crate::Error::Other(
            "HEIC/HEIF decoding requires optional libheif; skipping decode".into(),
        ));
    }

    if is_raw_path(path) {
        match extract_raw_preview(path) {
            Ok(decoded) => return Ok(decoded),
            Err(e) => {
                tracing::debug!(path = %path.display(), "RAW preview extraction failed: {e}");
            }
        }
    }

    let img = image::open(path).map_err(|e| {
        let prefix = match detect_image_format(path) {
            ImageFormatKind::Avif => "AVIF decode failed",
            ImageFormatKind::Heic => "HEIC decode failed",
            ImageFormatKind::Other if is_raw_path(path) => "RAW decode failed",
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
    use image::{ImageBuffer, Rgb, RgbImage};

    fn sample_jpeg_bytes() -> Vec<u8> {
        let img: RgbImage =
            ImageBuffer::from_fn(16, 16, |x, y| Rgb([(x * 15) as u8, (y * 15) as u8, 128]));
        let mut jpeg = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .expect("encode jpeg");
        jpeg
    }

    fn write_raw_fixture(
        name: &str,
        prefix: &[u8],
        jpeg: &[u8],
        suffix: &[u8],
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        data.extend_from_slice(jpeg);
        data.extend_from_slice(suffix);
        std::fs::write(&path, &data).unwrap();
        (dir, path)
    }

    #[test]
    fn is_raw_path_recognizes_all_extensions() {
        for ext in RAW_EXTENSIONS {
            let name = format!("/photos/sample.{ext}");
            let path = Path::new(&name);
            assert!(is_raw_path(path), ".{ext} should be RAW");
        }
        assert!(!is_raw_path(Path::new("/photos/sample.jpg")));
        assert!(!is_raw_path(Path::new("/photos/sample.png")));
    }

    #[test]
    fn extract_raw_preview_finds_largest_embedded_jpeg() {
        let small = sample_jpeg_bytes();
        let large_img: RgbImage =
            ImageBuffer::from_fn(32, 32, |x, y| Rgb([(x * 7) as u8, (y * 7) as u8, 64]));
        let mut large = Vec::new();
        large_img
            .write_to(
                &mut std::io::Cursor::new(&mut large),
                image::ImageFormat::Jpeg,
            )
            .expect("encode large jpeg");

        let (_dir, path) = write_raw_fixture(
            "preview.cr2",
            b"TIFF\x00\x01fake-header-padding\x00",
            &large,
            b"\x00\xFF\xD8",
        );
        // Smaller decoy JPEG after the large one — scanner should pick the largest valid JPEG.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        use std::io::Write;
        file.write_all(&small).unwrap();

        let decoded = extract_raw_preview(&path).expect("extract preview");
        assert_eq!(decoded.width, 32);
        assert_eq!(decoded.height, 32);
    }

    #[test]
    fn extract_raw_preview_bytes_returns_jpeg_payload() {
        let jpeg = sample_jpeg_bytes();
        let (_dir, path) = write_raw_fixture(
            "bytes.nef",
            b"\x00\x00TIFF-like-header",
            &jpeg,
            b"\x00trailer",
        );

        let bytes = extract_raw_preview_bytes(&path).expect("extract bytes");
        assert!(bytes.starts_with(&[0xFF, 0xD8]));
        assert!(bytes.ends_with(&[0xFF, 0xD9]));
    }

    #[test]
    fn extract_raw_preview_missing_jpeg_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.cr2");
        std::fs::write(&path, b"not a raw with jpeg").unwrap();
        assert!(extract_raw_preview(&path).is_err());
    }

    #[test]
    fn decode_image_raw_uses_embedded_preview() {
        let jpeg = sample_jpeg_bytes();
        let (_dir, path) = write_raw_fixture("decode.arw", b"ARW-header", &jpeg, b"");

        let decoded = decode_image(&path).expect("decode via embedded preview");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
    }

    #[test]
    fn decode_image_raw_without_preview_falls_through_and_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.dng");
        std::fs::write(&path, b"no jpeg here").unwrap();

        match decode_image(&path) {
            Err(e) => assert!(
                e.to_string().contains("RAW"),
                "expected RAW-specific error, got: {e}"
            ),
            Ok(_) => panic!("expected RAW decode to fail without preview"),
        }
    }

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

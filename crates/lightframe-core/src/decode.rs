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

#[cfg(feature = "raw-decode")]
fn decode_raw_full(path: &Path) -> Result<DecodedImage> {
    let raw = rawloader::decode_file(path)
        .map_err(|e| crate::Error::Other(format!("RAW decode: {e}")))?;

    let data = match raw.data {
        rawloader::RawImageData::Integer(ref d) => d,
        rawloader::RawImageData::Float(_) => {
            return Err(crate::Error::Other(
                "unsupported RAW data format (float)".into(),
            ));
        }
    };

    let width = raw.width;
    let height = raw.height;
    if width == 0 || height == 0 || data.len() != width * height {
        return Err(crate::Error::Other("invalid RAW dimensions".into()));
    }

    let cfa = raw.cropped_cfa();
    let wb = if raw.wb_coeffs.iter().all(|c| *c == 0.0) {
        raw.neutralwb()
    } else {
        raw.wb_coeffs
    };

    let max_val = raw
        .whitelevels
        .iter()
        .copied()
        .max()
        .filter(|&v| v > 0)
        .map(f32::from)
        .unwrap_or_else(|| data.iter().copied().max().unwrap_or(65535).max(1) as f32);

    let mut rgba = vec![0u8; width * height * 4];
    demosaic_bilinear(data, width, height, &cfa, &wb, max_val, &mut rgba);

    let (rgba, out_width, out_height) =
        apply_raw_orientation(rgba, width as u32, height as u32, raw.orientation);

    Ok(DecodedImage {
        rgba,
        width: out_width,
        height: out_height,
    })
}

#[cfg(feature = "raw-decode")]
fn cfa_color_channel(cfa: &rawloader::CFA, row: usize, col: usize) -> usize {
    match cfa.color_at(row, col) {
        0 => 0,     // R
        1 | 3 => 1, // G (including emerald)
        2 => 2,     // B
        _ => 1,
    }
}

#[cfg(feature = "raw-decode")]
fn cfa_matches_channel(cfa: &rawloader::CFA, row: usize, col: usize, target: usize) -> bool {
    cfa_color_channel(cfa, row, col) == target
}

#[cfg(feature = "raw-decode")]
fn demosaic_bilinear(
    data: &[u16],
    width: usize,
    height: usize,
    cfa: &rawloader::CFA,
    wb: &[f32; 4],
    max_val: f32,
    output: &mut [u8],
) {
    let wb_g = if wb[1] > 0.0 { wb[1] } else { 1.0 };
    let wb_r = if wb[0] > 0.0 { wb[0] / wb_g } else { 1.0 };
    let wb_b = if wb[2] > 0.0 { wb[2] / wb_g } else { 1.0 };

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let color = cfa_color_channel(cfa, y, x);
            let raw_val = data[idx] as f32;

            let (r, g, b) = match color {
                0 => {
                    let r = raw_val * wb_r;
                    let g = avg_neighbors_color(data, width, height, x, y, cfa, 1);
                    let b = avg_neighbors_color(data, width, height, x, y, cfa, 2) * wb_b;
                    (r, g, b)
                }
                1 => {
                    let r = avg_neighbors_color(data, width, height, x, y, cfa, 0) * wb_r;
                    let g = raw_val;
                    let b = avg_neighbors_color(data, width, height, x, y, cfa, 2) * wb_b;
                    (r, g, b)
                }
                2 => {
                    let r = avg_neighbors_color(data, width, height, x, y, cfa, 0) * wb_r;
                    let g = avg_neighbors_color(data, width, height, x, y, cfa, 1);
                    let b = raw_val * wb_b;
                    (r, g, b)
                }
                _ => (raw_val, raw_val, raw_val),
            };

            let out_idx = idx * 4;
            output[out_idx] = linear_to_srgb_byte(r / max_val);
            output[out_idx + 1] = linear_to_srgb_byte(g / max_val);
            output[out_idx + 2] = linear_to_srgb_byte(b / max_val);
            output[out_idx + 3] = 255;
        }
    }
}

#[cfg(feature = "raw-decode")]
fn linear_to_srgb_byte(linear: f32) -> u8 {
    let v = linear.clamp(0.0, 1.0).powf(1.0 / 2.2);
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(feature = "raw-decode")]
fn avg_neighbors_color(
    data: &[u16],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    cfa: &rawloader::CFA,
    target_color: usize,
) -> f32 {
    let mut sum = 0.0_f32;
    let mut count = 0u32;

    for dy in [-1i32, 0, 1] {
        for dx in [-1i32, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nx = nx as usize;
                let ny = ny as usize;
                if cfa_matches_channel(cfa, ny, nx, target_color) {
                    sum += data[ny * width + nx] as f32;
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        sum / count as f32
    } else {
        data[y * width + x] as f32
    }
}

#[cfg(feature = "raw-decode")]
fn apply_raw_orientation(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    orientation: rawloader::Orientation,
) -> (Vec<u8>, u32, u32) {
    if matches!(
        orientation,
        rawloader::Orientation::Normal | rawloader::Orientation::Unknown
    ) {
        return (rgba, width, height);
    }

    let (transpose, flip_h, flip_v) = orientation.to_flips();
    let mut buf = rgba;
    let (mut w, mut h) = (width, height);

    if flip_h {
        buf = flip_horizontal_rgba(&buf, w, h);
    }
    if flip_v {
        buf = flip_vertical_rgba(&buf, w, h);
    }
    if transpose {
        (buf, w, h) = transpose_rgba(buf, w, h);
    }

    (buf, w, h)
}

#[cfg(feature = "raw-decode")]
fn flip_horizontal_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst = (y * w + (w - 1 - x)) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    out
}

#[cfg(feature = "raw-decode")]
fn flip_vertical_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst = ((h - 1 - y) * w + x) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    out
}

#[cfg(feature = "raw-decode")]
fn transpose_rgba(rgba: Vec<u8>, width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dst = (x * h + y) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    (out, height, width)
}

pub fn decode_image(path: &Path) -> Result<DecodedImage> {
    if is_heic_path(path) {
        return Err(crate::Error::Other(
            "HEIC/HEIF decoding requires optional libheif; skipping decode".into(),
        ));
    }

    if is_raw_path(path) {
        #[cfg(feature = "raw-decode")]
        match decode_raw_full(path) {
            Ok(decoded) => return Ok(decoded),
            Err(e) => {
                tracing::debug!(
                    path = %path.display(),
                    "full RAW decode failed: {e}, trying embedded preview"
                );
            }
        }

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
    fn is_raw_path_mixed_case_extensions() {
        assert!(is_raw_path(Path::new("/photos/sample.CR2")));
        assert!(is_raw_path(Path::new("/photos/sample.Nef")));
        assert!(is_raw_path(Path::new("/photos/sample.DNG")));
        assert!(!is_raw_path(Path::new("/photos/sample.JPG")));
    }

    #[test]
    fn detect_image_format_empty_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.jpg");
        std::fs::write(&empty, []).unwrap();

        assert_eq!(detect_image_format(&empty), ImageFormatKind::Other);
        assert_eq!(detect_image_format(Path::new("")), ImageFormatKind::Other);
    }

    #[test]
    fn detect_image_format_truncated_file_uses_extension() {
        let dir = tempfile::tempdir().unwrap();
        let truncated = dir.path().join("truncated.avif");
        std::fs::write(&truncated, &[0x00, 0x01, 0x02]).unwrap();

        assert_eq!(detect_image_format(&truncated), ImageFormatKind::Avif);
        assert!(decode_image(&truncated).is_err());
    }

    #[test]
    fn is_decode_supported_rejects_unsupported_heic_formats() {
        assert!(!is_decode_supported(Path::new("photo.HEIC")));
        assert!(!is_decode_supported(Path::new("photo.HeIf")));
    }

    #[test]
    fn decode_image_empty_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.png");
        std::fs::write(&empty, []).unwrap();

        assert!(decode_image(&empty).is_err());
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

    #[cfg(feature = "raw-decode")]
    mod raw_decode {
        use super::*;
        use rawloader::CFA;

        /// 4x4 RGGB Bayer with distinct per-channel values.
        fn sample_bayer_rggb() -> ([u16; 16], CFA) {
            let cfa = CFA::new("RGGB");
            let mut data = [0u16; 16];
            for y in 0..4 {
                for x in 0..4 {
                    let val = match cfa_color_channel(&cfa, y, x) {
                        0 => 4000, // R
                        1 => 2000, // G
                        2 => 1000, // B
                        _ => 2000,
                    };
                    data[y * 4 + x] = val;
                }
            }
            (data, cfa)
        }

        #[test]
        fn demosaic_bilinear_produces_rgb_for_rggb_pattern() {
            let (data, cfa) = sample_bayer_rggb();
            let mut rgba = vec![0u8; 4 * 4 * 4];
            let wb = [1.0_f32; 4];
            demosaic_bilinear(&data, 4, 4, &cfa, &wb, 4000.0, &mut rgba);

            // Top-left is a red pixel — R channel should dominate.
            assert!(rgba[0] > rgba[1], "red pixel R > G");
            assert!(rgba[0] > rgba[2], "red pixel R > B");

            // (1,1) is a blue pixel — all channels populated with non-zero values.
            let b_idx = (1 * 4 + 1) * 4;
            assert!(rgba[b_idx] > 0);
            assert!(rgba[b_idx + 1] > 0);
            assert!(rgba[b_idx + 2] > 0);

            // Every pixel has full alpha.
            for chunk in rgba.chunks(4) {
                assert_eq!(chunk[3], 255);
            }
        }

        #[test]
        fn demosaic_handles_zero_white_balance_coeffs() {
            let (data, cfa) = sample_bayer_rggb();
            let mut rgba = vec![0u8; 4 * 4 * 4];
            let wb = [0.0_f32; 4];
            demosaic_bilinear(&data, 4, 4, &cfa, &wb, 4000.0, &mut rgba);
            assert!(rgba.iter().any(|&v| v > 0));
        }

        #[test]
        fn apply_raw_orientation_rotate90_swaps_dimensions() {
            let mut rgba = vec![0u8; 2 * 3 * 4];
            for (i, px) in rgba.chunks_mut(4).enumerate() {
                px[0] = i as u8;
                px[3] = 255;
            }
            let (out, w, h) = apply_raw_orientation(rgba, 2, 3, rawloader::Orientation::Rotate90);
            assert_eq!(w, 3);
            assert_eq!(h, 2);
            assert_eq!(out.len(), 2 * 3 * 4);
        }

        #[test]
        fn decode_image_raw_falls_back_to_preview_when_full_decode_fails() {
            let jpeg = sample_jpeg_bytes();
            let (_dir, path) = write_raw_fixture("fallback.cr2", b"fake-raw-header", &jpeg, b"");

            let decoded = decode_image(&path).expect("should fall back to embedded preview");
            assert_eq!(decoded.width, 16);
            assert_eq!(decoded.height, 16);
        }
    }
}

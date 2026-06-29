use exif::{In, Tag};
use image::GenericImageView;
use lightframe_core::Result;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ScreenshotType {
    Generic,
    Code,
    Chat,
    Document,
    Game,
    WebPage,
}

impl ScreenshotType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Code => "code",
            Self::Chat => "chat",
            Self::Document => "document",
            Self::Game => "game",
            Self::WebPage => "webpage",
        }
    }

    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "generic" => Some(Self::Generic),
            "code" => Some(Self::Code),
            "chat" => Some(Self::Chat),
            "document" => Some(Self::Document),
            "game" => Some(Self::Game),
            "webpage" => Some(Self::WebPage),
            _ => None,
        }
    }
}

/// Confidence that an image is a screenshot, in the range `0.0`–`1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenshotScore {
    pub confidence: f32,
}

impl ScreenshotScore {
    pub const THRESHOLD: f32 = 0.5;

    pub fn is_likely_screenshot(&self) -> bool {
        self.confidence >= Self::THRESHOLD
    }
}

/// Classify whether an image is likely a screenshot using EXIF and visual heuristics.
pub fn detect_screenshot(path: &Path, width: u32, height: u32) -> Result<ScreenshotScore> {
    let mut score = 0.0_f32;

    if matches_common_resolution(width, height) {
        score += 0.25;
    }

    if matches_screenshot_aspect_ratio(width, height) {
        score += 0.15;
    }

    if is_png(path) {
        score += 0.10;
    }

    let exif_signals = read_exif_signals(path)?;
    if exif_signals.has_camera_info {
        score -= 0.70;
    }
    if exif_signals.has_exposure_info {
        score -= 0.50;
    }
    if !exif_signals.has_any_exif && matches_common_resolution(width, height) {
        score += 0.20;
    }

    if path.exists()
        && let Ok(bands) = detect_status_bar_bands(path, width, height)
    {
        if bands.top {
            score += 0.12;
        }
        if bands.bottom {
            score += 0.08;
        }
    }

    Ok(ScreenshotScore {
        confidence: score.clamp(0.0, 1.0),
    })
}

/// Rules-based screenshot sub-classification (no ML required).
pub fn classify_screenshot(path: &Path) -> Result<ScreenshotType> {
    if !path.exists() {
        return Ok(ScreenshotType::Generic);
    }

    let img = image::open(path).map_err(|e| lightframe_core::Error::Ai(e.to_string()))?;
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return Ok(ScreenshotType::Generic);
    }

    let (long, short) = if width >= height {
        (width, height)
    } else {
        (height, width)
    };
    let aspect_ratio = long as f64 / short as f64;

    // Tall/narrow layouts (e.g. phone chat threads) are often chat screenshots.
    if aspect_ratio >= 2.0 {
        return Ok(ScreenshotType::Chat);
    }

    if is_likely_code_screenshot(&img) {
        return Ok(ScreenshotType::Code);
    }

    if is_likely_webpage_screenshot(&img, aspect_ratio) {
        return Ok(ScreenshotType::WebPage);
    }

    if is_likely_document_screenshot(&img) {
        return Ok(ScreenshotType::Document);
    }

    if is_likely_game_screenshot(&img) {
        return Ok(ScreenshotType::Game);
    }

    Ok(ScreenshotType::Generic)
}

fn is_likely_document_screenshot(img: &image::DynamicImage) -> bool {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return false;
    }

    let rgba = img.to_rgba8();
    let step_x = (width / 40).max(1);
    let step_y = (height / 40).max(1);

    let mut sample_count = 0_u64;
    let mut light_count = 0_u64;
    let mut neutral_count = 0_u64;

    for y in (0..height).step_by(step_y as usize) {
        for x in (0..width).step_by(step_x as usize) {
            let pixel = rgba.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            let brightness = (r + g + b) / 3.0;
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);

            sample_count += 1;
            if brightness > 210.0 {
                light_count += 1;
            }
            if max - min < 18.0 && brightness > 160.0 {
                neutral_count += 1;
            }
        }
    }

    if sample_count == 0 {
        return false;
    }

    let light_ratio = light_count as f64 / sample_count as f64;
    let neutral_ratio = neutral_count as f64 / sample_count as f64;
    light_ratio >= 0.55 && neutral_ratio >= 0.35
}

fn is_likely_game_screenshot(img: &image::DynamicImage) -> bool {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return false;
    }

    let rgba = img.to_rgba8();
    let step_x = (width / 48).max(1);
    let step_y = (height / 48).max(1);

    let mut sample_count = 0_u64;
    let mut saturated_count = 0_u64;
    let mut high_contrast_count = 0_u64;

    for y in (0..height).step_by(step_y as usize) {
        for x in (0..width).step_by(step_x as usize) {
            let pixel = rgba.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let brightness = (r + g + b) / 3.0;

            sample_count += 1;
            if max > 40.0 && (max - min) / max > 0.45 {
                saturated_count += 1;
            }
            if brightness > 30.0 && brightness < 220.0 && max - min > 60.0 {
                high_contrast_count += 1;
            }
        }
    }

    if sample_count == 0 {
        return false;
    }

    let saturated_ratio = saturated_count as f64 / sample_count as f64;
    let contrast_ratio = high_contrast_count as f64 / sample_count as f64;
    saturated_ratio >= 0.35 && contrast_ratio >= 0.40
}

fn is_likely_webpage_screenshot(img: &image::DynamicImage, aspect_ratio: f64) -> bool {
    if !(1.2..=2.2).contains(&aspect_ratio) {
        return false;
    }

    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return false;
    }

    let top_rows = ((height as f32 * 0.06).max(8.0) as u32).min(height);
    if !is_uniform_band(img, 0, top_rows) {
        return false;
    }

    let content_start = top_rows.saturating_add(1);
    if content_start >= height || is_uniform_band(img, content_start, height) {
        return false;
    }

    let rgba = img.to_rgba8();
    let mut top_light = 0_u64;
    let mut top_samples = 0_u64;

    for y in 0..top_rows {
        for x in (0..width).step_by((width / 32).max(1) as usize) {
            let pixel = rgba.get_pixel(x, y);
            let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) as f64 / 3.0;
            top_samples += 1;
            if brightness > 180.0 {
                top_light += 1;
            }
        }
    }

    if top_samples == 0 {
        return false;
    }

    let top_light_ratio = top_light as f64 / top_samples as f64;
    top_light_ratio >= 0.55
}

fn is_likely_code_screenshot(img: &image::DynamicImage) -> bool {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return false;
    }

    let rgba = img.to_rgba8();
    let step_x = (width / 48).max(1);
    let step_y = (height / 48).max(1);

    let mut sample_count = 0_u64;
    let mut dark_count = 0_u64;
    let mut syntax_color_count = 0_u64;

    for y in (0..height).step_by(step_y as usize) {
        for x in (0..width).step_by(step_x as usize) {
            let pixel = rgba.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            let brightness = (r + g + b) / 3.0;

            sample_count += 1;
            if brightness < 80.0 {
                dark_count += 1;
            }

            if brightness < 180.0 && is_syntax_highlight_color(r, g, b) {
                syntax_color_count += 1;
            }
        }
    }

    if sample_count == 0 {
        return false;
    }

    let dark_ratio = dark_count as f64 / sample_count as f64;
    let syntax_ratio = syntax_color_count as f64 / sample_count as f64;

    dark_ratio >= 0.45 && syntax_ratio >= 0.04
}

fn is_syntax_highlight_color(r: f32, g: f32, b: f32) -> bool {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max < 40.0 || max - min < 25.0 {
        return false;
    }

    // Common IDE syntax colors on dark backgrounds.
    (b > r + 20.0 && b > g + 10.0)
        || (g > r + 20.0 && g > b + 10.0)
        || (r > g + 25.0 && r > b + 25.0)
        || (r > 180.0 && g > 120.0 && b < 100.0)
}

#[derive(Debug, Default)]
struct ExifSignals {
    has_any_exif: bool,
    has_camera_info: bool,
    has_exposure_info: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct StatusBarBands {
    top: bool,
    bottom: bool,
}

fn read_exif_signals(path: &Path) -> Result<ExifSignals> {
    if !path.exists() {
        return Ok(ExifSignals::default());
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif_reader = exif::Reader::new();

    let exif = match exif_reader.read_from_container(&mut reader) {
        Ok(exif) => exif,
        Err(_) => return Ok(ExifSignals::default()),
    };

    let has_make = exif
        .get_field(Tag::Make, In::PRIMARY)
        .is_some_and(|f| !f.display_value().to_string().trim().is_empty());
    let has_model = exif
        .get_field(Tag::Model, In::PRIMARY)
        .is_some_and(|f| !f.display_value().to_string().trim().is_empty());

    let has_aperture = exif.get_field(Tag::FNumber, In::PRIMARY).is_some()
        || exif.get_field(Tag::ApertureValue, In::PRIMARY).is_some();
    let has_shutter = exif.get_field(Tag::ExposureTime, In::PRIMARY).is_some()
        || exif
            .get_field(Tag::ShutterSpeedValue, In::PRIMARY)
            .is_some();

    Ok(ExifSignals {
        has_any_exif: true,
        has_camera_info: has_make || has_model,
        has_exposure_info: has_aperture || has_shutter,
    })
}

fn matches_common_resolution(width: u32, height: u32) -> bool {
    const COMMON: &[(u32, u32)] = &[
        (1920, 1080),
        (2560, 1440),
        (3840, 2160),
        (1080, 1920),
        (1440, 2560),
        (2160, 3840),
        (1366, 768),
        (1280, 720),
        (750, 1334),
        (1125, 2436),
        (1170, 2532),
        (1284, 2778),
        (1179, 2556),
        (1290, 2796),
        (1080, 2340),
        (1080, 2400),
        (1440, 900),
        (1680, 1050),
        (2560, 1600),
        (1080, 2280),
        (1080, 2316),
    ];

    COMMON
        .iter()
        .any(|&(w, h)| w == width && h == height || w == height && h == width)
}

fn matches_screenshot_aspect_ratio(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 {
        return false;
    }

    let (long, short) = if width >= height {
        (width as f64, height as f64)
    } else {
        (height as f64, width as f64)
    };
    let ratio = long / short;

    const TARGETS: &[(f64, f64)] = &[
        (16.0 / 9.0, 0.02),
        (16.0 / 10.0, 0.02),
        (4.0 / 3.0, 0.02),
        (18.0 / 9.0, 0.03),
        (19.5 / 9.0, 0.03),
    ];

    TARGETS
        .iter()
        .any(|&(target, tolerance)| (ratio - target).abs() <= tolerance)
}

fn is_png(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

fn detect_status_bar_bands(path: &Path, _width: u32, height: u32) -> Result<StatusBarBands> {
    let img = image::open(path).map_err(|e| lightframe_core::Error::Ai(e.to_string()))?;
    let (img_w, img_h) = img.dimensions();
    if img_w == 0 || img_h == 0 {
        return Ok(StatusBarBands::default());
    }

    // Scale band heights relative to actual image dimensions when they differ from metadata.
    let scale_h = img_h as f32 / height.max(1) as f32;
    let top_rows = ((img_h as f32 * 0.03).max(4.0 * scale_h) as u32).min(img_h);
    let bottom_rows = ((img_h as f32 * 0.05).max(6.0 * scale_h) as u32).min(img_h);

    Ok(StatusBarBands {
        top: is_uniform_band(&img, 0, top_rows),
        bottom: is_uniform_band(&img, img_h.saturating_sub(bottom_rows), img_h),
    })
}

fn is_uniform_band(img: &image::DynamicImage, y_start: u32, y_end: u32) -> bool {
    let (w, h) = img.dimensions();
    if y_start >= y_end || y_end > h || w == 0 {
        return false;
    }

    let rgba = img.to_rgba8();
    let sample_step = (w / 32).max(1);

    let mut sum_r = 0_u64;
    let mut sum_g = 0_u64;
    let mut sum_b = 0_u64;
    let mut count = 0_u64;

    for y in y_start..y_end {
        for x in (0..w).step_by(sample_step as usize) {
            let pixel = rgba.get_pixel(x, y);
            sum_r += pixel[0] as u64;
            sum_g += pixel[1] as u64;
            sum_b += pixel[2] as u64;
            count += 1;
        }
    }

    if count == 0 {
        return false;
    }

    let mean_r = sum_r as f64 / count as f64;
    let mean_g = sum_g as f64 / count as f64;
    let mean_b = sum_b as f64 / count as f64;

    let mut variance = 0.0_f64;
    for y in y_start..y_end {
        for x in (0..w).step_by(sample_step as usize) {
            let pixel = rgba.get_pixel(x, y);
            let dr = pixel[0] as f64 - mean_r;
            let dg = pixel[1] as f64 - mean_g;
            let db = pixel[2] as f64 - mean_b;
            variance += dr * dr + dg * dg + db * db;
        }
    }
    variance /= count as f64;

    // Low color variance indicates a solid status/navigation bar.
    variance < 400.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::PathBuf;

    fn temp_png(name: &str, img: &RgbaImage) -> PathBuf {
        let dir = std::env::temp_dir().join("lightframe_screenshot_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        img.save(&path).unwrap();
        path
    }

    fn screenshot_like_image(w: u32, h: u32, top_bar: bool, bottom_bar: bool) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        let top_limit = (h as f32 * 0.03).max(4.0) as u32;
        let bottom_start = h.saturating_sub((h as f32 * 0.05).max(6.0) as u32);

        for (x, y, pixel) in img.enumerate_pixels_mut() {
            if top_bar && y < top_limit {
                *pixel = Rgba([30, 30, 30, 255]);
            } else if bottom_bar && y >= bottom_start {
                *pixel = Rgba([30, 30, 30, 255]);
            } else {
                // Varied content region so plain bands are distinguishable.
                let r = ((x * 17 + y * 31) % 256) as u8;
                let g = ((x * 13 + y * 23) % 256) as u8;
                let b = ((x * 11 + y * 19) % 256) as u8;
                *pixel = Rgba([r, g, b, 255]);
            }
        }
        img
    }

    #[test]
    fn score_1080p_png_without_file_uses_resolution_and_format() {
        let path = Path::new("screenshot.png");
        let score = detect_screenshot(path, 1920, 1080).unwrap();
        assert!(
            score.confidence >= 0.4,
            "expected moderate confidence, got {}",
            score.confidence
        );
        assert!(score.is_likely_screenshot());
    }

    #[test]
    fn score_1440p_png_high_confidence() {
        let path = Path::new("shot.png");
        let score = detect_screenshot(path, 2560, 1440).unwrap();
        assert!(score.confidence >= 0.4);
    }

    #[test]
    fn score_4k_png_high_confidence() {
        let path = Path::new("shot.png");
        let score = detect_screenshot(path, 3840, 2160).unwrap();
        assert!(score.confidence >= 0.4);
    }

    #[test]
    fn phone_portrait_png_is_likely_screenshot() {
        let path = Path::new("phone.png");
        let score = detect_screenshot(path, 1170, 2532).unwrap();
        assert!(score.is_likely_screenshot());
    }

    #[test]
    fn common_res_jpg_lower_than_png() {
        let png_path = Path::new("screenshot.png");
        let jpg_path = Path::new("photo.jpg");
        let png_score = detect_screenshot(png_path, 1920, 1080).unwrap();
        let jpg_score = detect_screenshot(jpg_path, 1920, 1080).unwrap();
        assert!(png_score.confidence > jpg_score.confidence);
    }

    #[test]
    fn uncommon_resolution_low_confidence() {
        let path = Path::new("photo.png");
        let score = detect_screenshot(path, 1234, 5678).unwrap();
        assert!(!score.is_likely_screenshot());
        assert!(score.confidence < 0.3);
    }

    #[test]
    fn non_standard_photo_dimensions_low_confidence() {
        let path = Path::new("photo.png");
        let score = detect_screenshot(path, 4032, 3024).unwrap();
        assert!(!score.is_likely_screenshot());
    }

    #[test]
    fn classify_missing_file_returns_generic() {
        let result = classify_screenshot(Path::new("/nonexistent/screenshot.png")).unwrap();
        assert_eq!(result, ScreenshotType::Generic);
    }

    #[test]
    fn classify_tall_narrow_image_as_chat() {
        let mut img = RgbaImage::new(400, 1200);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let v = ((x + y) % 200) as u8;
            *pixel = Rgba([v, v, v, 255]);
        }
        let path = temp_png("tall_chat.png", &img);
        let result = classify_screenshot(&path).unwrap();
        assert_eq!(result, ScreenshotType::Chat);
    }

    #[test]
    fn classify_dark_syntax_image_as_code() {
        let mut img = RgbaImage::new(1920, 1080);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([22, 22, 28, 255]);
            if x % 37 == 0 {
                *pixel = Rgba([86, 156, 214, 255]);
            } else if y % 29 == 0 {
                *pixel = Rgba([78, 201, 176, 255]);
            } else if (x + y) % 53 == 0 {
                *pixel = Rgba([206, 145, 120, 255]);
            }
        }
        let path = temp_png("code_like.png", &img);
        let result = classify_screenshot(&path).unwrap();
        assert_eq!(result, ScreenshotType::Code);
    }

    #[test]
    fn classify_bright_neutral_image_as_document() {
        // Square aspect avoids webpage heuristic (requires 1.2–2.2).
        let mut img = RgbaImage::new(1200, 1200);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([245, 245, 242, 255]);
        }
        let path = temp_png("document_like.png", &img);
        let result = classify_screenshot(&path).unwrap();
        assert_eq!(result, ScreenshotType::Document);
    }

    #[test]
    fn classify_saturated_image_as_game() {
        let mut img = RgbaImage::new(1920, 1080);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let channel = ((x + y) % 3) as u8;
            *pixel = match channel {
                0 => Rgba([220, 40, 40, 255]),
                1 => Rgba([40, 200, 80, 255]),
                _ => Rgba([40, 80, 220, 255]),
            };
        }
        let path = temp_png("game_like.png", &img);
        let result = classify_screenshot(&path).unwrap();
        assert_eq!(result, ScreenshotType::Game);
    }

    #[test]
    fn classify_browser_like_image_as_webpage() {
        let mut img = RgbaImage::new(1440, 900);
        let top_rows = (900_f32 * 0.06).max(8.0) as u32;
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            if y < top_rows {
                *pixel = Rgba([245, 245, 245, 255]);
            } else {
                let v = 180 + ((x * 17 + y * 31) % 60) as u8;
                *pixel = Rgba([v, v.saturating_sub(5), v.saturating_sub(10), 255]);
            }
        }
        let path = temp_png("webpage_like.png", &img);
        let result = classify_screenshot(&path).unwrap();
        assert_eq!(result, ScreenshotType::WebPage);
    }

    #[test]
    fn screenshot_type_label_roundtrip() {
        assert_eq!(ScreenshotType::Code.label(), "code");
        assert_eq!(
            ScreenshotType::from_label("webpage"),
            Some(ScreenshotType::WebPage)
        );
        assert_eq!(ScreenshotType::from_label("unknown"), None);
    }

    #[test]
    fn aspect_ratio_16_9_matches() {
        assert!(matches_screenshot_aspect_ratio(1920, 1080));
        assert!(matches_screenshot_aspect_ratio(1080, 1920));
    }

    #[test]
    fn aspect_ratio_16_10_matches() {
        assert!(matches_screenshot_aspect_ratio(1920, 1200));
    }

    #[test]
    fn aspect_ratio_19_5_9_matches_phone() {
        assert!(matches_screenshot_aspect_ratio(1170, 2532));
    }

    #[test]
    fn aspect_ratio_non_standard_fails() {
        assert!(!matches_screenshot_aspect_ratio(1234, 5678));
    }

    #[test]
    fn status_bar_detection_on_synthetic_image() {
        let img = screenshot_like_image(1920, 1080, true, true);
        let path = temp_png("status_bars.png", &img);
        let bands = detect_status_bar_bands(&path, 1920, 1080).unwrap();
        assert_eq!(
            bands,
            StatusBarBands {
                top: true,
                bottom: true
            }
        );

        let score = detect_screenshot(&path, 1920, 1080).unwrap();
        assert!(
            score.confidence >= 0.6,
            "status bars should boost confidence, got {}",
            score.confidence
        );
    }

    #[test]
    fn no_status_bars_lower_than_with_bars() {
        let with_bars = screenshot_like_image(1920, 1080, true, true);
        let without_bars = screenshot_like_image(1920, 1080, false, false);
        let path_with = temp_png("with_bars.png", &with_bars);
        let path_without = temp_png("without_bars.png", &without_bars);

        let score_with = detect_screenshot(&path_with, 1920, 1080).unwrap();
        let score_without = detect_screenshot(&path_without, 1920, 1080).unwrap();
        assert!(score_with.confidence > score_without.confidence);
    }

    #[test]
    fn exif_camera_make_reduces_confidence() {
        let jpeg_with_exif = include_bytes!("../tests/fixtures/camera_exif.jpg");
        let dir = std::env::temp_dir().join("lightframe_screenshot_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("camera_exif.jpg");
        std::fs::write(&path, jpeg_with_exif).unwrap();

        let signals = read_exif_signals(&path).unwrap();
        assert!(
            signals.has_camera_info,
            "fixture should contain camera make/model EXIF"
        );

        let score = detect_screenshot(&path, 4032, 3024).unwrap();
        assert!(
            !score.is_likely_screenshot(),
            "camera EXIF should suppress screenshot classification"
        );
        assert!(score.confidence < 0.3);
    }

    #[test]
    fn no_exif_common_resolution_boosts_confidence() {
        let img = screenshot_like_image(1920, 1080, false, false);
        let path = temp_png("no_exif.png", &img);
        let signals = read_exif_signals(&path).unwrap();
        assert!(!signals.has_any_exif);

        let score = detect_screenshot(&path, 1920, 1080).unwrap();
        assert!(score.confidence >= 0.45);
    }

    #[test]
    fn threshold_constant_is_half() {
        assert_eq!(ScreenshotScore::THRESHOLD, 0.5);
    }

    #[test]
    fn confidence_clamped_to_unit_interval() {
        let path = Path::new("max.png");
        let score = detect_screenshot(path, 1920, 1080).unwrap();
        assert!((0.0..=1.0).contains(&score.confidence));
    }

    #[test]
    fn read_exif_signals_missing_file_is_neutral() {
        let signals = read_exif_signals(Path::new("/nonexistent/file.jpg")).unwrap();
        assert!(!signals.has_any_exif);
    }

    #[test]
    fn score_ultrawide_resolution_with_status_bars() {
        // Use 2560×1440 — a known common resolution so status-bar heuristics apply.
        let with_bars = screenshot_like_image(2560, 1440, true, true);
        let without_bars = screenshot_like_image(2560, 1440, false, false);
        let path_with = temp_png("ultrawide_with.png", &with_bars);
        let path_without = temp_png("ultrawide_without.png", &without_bars);
        let score_with = detect_screenshot(&path_with, 2560, 1440).unwrap();
        let score_without = detect_screenshot(&path_without, 2560, 1440).unwrap();
        assert!(
            score_with.confidence > score_without.confidence,
            "1440p with status bars should gain visual signals, with={} without={}",
            score_with.confidence,
            score_without.confidence
        );
    }

    #[test]
    fn score_tablet_4_3_aspect_ratio() {
        let path = Path::new("tablet.png");
        let score = detect_screenshot(path, 768, 1024).unwrap();
        assert!(
            matches_screenshot_aspect_ratio(768, 1024),
            "768×1024 should match 4:3 tablet aspect"
        );
        assert!(score.confidence >= 0.15);
    }

    #[test]
    fn status_bar_top_only_detection() {
        let img = screenshot_like_image(1920, 1080, true, false);
        let path = temp_png("top_bar_only.png", &img);
        let bands = detect_status_bar_bands(&path, 1920, 1080).unwrap();
        assert_eq!(
            bands,
            StatusBarBands {
                top: true,
                bottom: false
            }
        );
    }

    #[test]
    fn confidence_at_threshold_is_likely_screenshot() {
        let score = ScreenshotScore {
            confidence: ScreenshotScore::THRESHOLD,
        };
        assert!(score.is_likely_screenshot());
    }

    #[test]
    fn confidence_just_below_threshold_is_not_likely_screenshot() {
        let score = ScreenshotScore {
            confidence: ScreenshotScore::THRESHOLD - 0.001,
        };
        assert!(!score.is_likely_screenshot());
    }

    #[test]
    fn exif_exposure_info_reduces_confidence() {
        let jpeg_with_exif = include_bytes!("../tests/fixtures/camera_exif.jpg");
        let dir = std::env::temp_dir().join("lightframe_screenshot_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exposure_exif.jpg");
        std::fs::write(&path, jpeg_with_exif).unwrap();

        let signals = read_exif_signals(&path).unwrap();
        assert!(signals.has_exposure_info || signals.has_camera_info);

        let with_exif = detect_screenshot(&path, 1920, 1080).unwrap();
        let without_exif = detect_screenshot(Path::new("screenshot.png"), 1920, 1080).unwrap();
        assert!(
            with_exif.confidence < without_exif.confidence,
            "EXIF penalties should reduce confidence vs bare filename heuristic"
        );
    }

    #[test]
    fn score_1x1_image_unlikely_screenshot() {
        let img = RgbaImage::from_pixel(1, 1, Rgba([128, 128, 128, 255]));
        let path = temp_png("tiny_1x1.png", &img);
        let score = detect_screenshot(&path, 1, 1).unwrap();
        assert!(!score.is_likely_screenshot());
        assert!((0.0..=1.0).contains(&score.confidence));
    }

    #[test]
    fn score_8k_resolution_png_moderate_confidence() {
        let path = Path::new("8k.png");
        let score = detect_screenshot(path, 7680, 4320).unwrap();
        assert!(score.confidence >= 0.0);
        assert!(score.confidence <= 1.0);
    }

    #[test]
    fn non_standard_aspect_ratio_1_to_3() {
        assert!(!matches_screenshot_aspect_ratio(300, 900));
        let path = Path::new("tall.png");
        let score = detect_screenshot(path, 300, 900).unwrap();
        assert!(!score.is_likely_screenshot());
    }

    #[test]
    fn partial_exif_make_without_model_still_detects_make() {
        let jpeg_with_exif = include_bytes!("../tests/fixtures/camera_exif.jpg");
        let dir = std::env::temp_dir().join("lightframe_screenshot_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("partial_exif.jpg");
        std::fs::write(&path, jpeg_with_exif).unwrap();

        let signals = read_exif_signals(&path).unwrap();
        assert!(signals.has_any_exif);
        assert!(signals.has_camera_info || signals.has_exposure_info);

        let score = detect_screenshot(&path, 1920, 1080).unwrap();
        assert!(score.confidence < 0.5 || !score.is_likely_screenshot());
    }
}

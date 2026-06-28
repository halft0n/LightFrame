use catchlight_core::media::MediaType;
use catchlight_core::Result;
use std::path::Path;

pub fn detect_screenshot(path: &Path, width: u32, height: u32) -> Result<bool> {
    let common_resolutions = [
        (1920, 1080), (2560, 1440), (3840, 2160),
        (1080, 1920), (1440, 2560), (2160, 3840),
        (1366, 768), (1280, 720), (750, 1334), (1125, 2436),
        (1170, 2532), (1284, 2778), (1179, 2556), (1290, 2796),
        (1080, 2340), (1080, 2400),
    ];

    let is_common_res = common_resolutions
        .iter()
        .any(|&(w, h)| w == width && h == height);

    if !is_common_res {
        return Ok(false);
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    Ok(ext == "png")
}

pub fn classify_screenshot(_path: &Path) -> Result<MediaType> {
    // TODO: use CLIP ONNX or Python sidecar for sub-classification
    Ok(MediaType::Screenshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_1080p_png_as_screenshot() {
        let path = Path::new("screenshot.png");
        assert!(detect_screenshot(path, 1920, 1080).unwrap());
    }

    #[test]
    fn detect_1440p_png_as_screenshot() {
        let path = Path::new("shot.png");
        assert!(detect_screenshot(path, 2560, 1440).unwrap());
    }

    #[test]
    fn detect_4k_png_as_screenshot() {
        let path = Path::new("shot.png");
        assert!(detect_screenshot(path, 3840, 2160).unwrap());
    }

    #[test]
    fn phone_portrait_png_is_screenshot() {
        let path = Path::new("phone.png");
        assert!(detect_screenshot(path, 1170, 2532).unwrap());
    }

    #[test]
    fn common_res_but_jpg_not_screenshot() {
        let path = Path::new("photo.jpg");
        assert!(!detect_screenshot(path, 1920, 1080).unwrap());
    }

    #[test]
    fn uncommon_resolution_not_screenshot() {
        let path = Path::new("photo.png");
        assert!(!detect_screenshot(path, 1234, 5678).unwrap());
    }

    #[test]
    fn non_standard_photo_dimensions_not_screenshot() {
        let path = Path::new("photo.png");
        assert!(!detect_screenshot(path, 4032, 3024).unwrap());
    }

    #[test]
    fn classify_returns_screenshot_type() {
        let result = classify_screenshot(Path::new("any.png")).unwrap();
        assert_eq!(result, MediaType::Screenshot);
    }
}

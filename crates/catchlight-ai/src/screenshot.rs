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

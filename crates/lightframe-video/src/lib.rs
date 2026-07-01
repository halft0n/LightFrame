use lightframe_core::Result;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

pub async fn extract_frame(video: &Path, output: &Path, timestamp_secs: f64) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-ss",
            &timestamp_secs.to_string(),
            "-i",
            &video.to_string_lossy(),
            "-vframes",
            "1",
            "-q:v",
            "2",
            output.to_string_lossy().as_ref(),
        ])
        .output()
        .await
        .map_err(|e| lightframe_core::Error::Other(format!("ffmpeg not found: {e}")))?;

    if !status.status.success() {
        return Err(lightframe_core::Error::Other(
            String::from_utf8_lossy(&status.stderr).to_string(),
        ));
    }

    debug!(video = %video.display(), timestamp = timestamp_secs, "frame extracted");
    Ok(())
}

pub async fn get_duration(video: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &video.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| lightframe_core::Error::Other(format!("ffprobe not found: {e}")))?;

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str
        .trim()
        .parse::<f64>()
        .map_err(|e| lightframe_core::Error::Other(format!("failed to parse duration: {e}")))
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    which::which("ffmpeg").ok()
}

/// Trim a video using ffmpeg -c copy (no re-encode).
/// `in_sec` and `out_sec` define the trim window.
pub async fn trim_export(input: &Path, output: &Path, in_sec: f64, out_sec: f64) -> Result<()> {
    if in_sec >= out_sec {
        return Err(lightframe_core::Error::Other(
            "trim in_sec must be less than out_sec".to_string(),
        ));
    }
    if in_sec < 0.0 {
        return Err(lightframe_core::Error::Other(
            "trim in_sec must be non-negative".to_string(),
        ));
    }

    let ffmpeg = find_ffmpeg().ok_or_else(|| {
        lightframe_core::Error::Other("ffmpeg not found on PATH; please install ffmpeg".to_string())
    })?;

    let status = Command::new(ffmpeg)
        .args([
            "-y",
            "-ss",
            &format!("{:.3}", in_sec),
            "-i",
            &input.to_string_lossy(),
            "-to",
            &format!("{:.3}", out_sec - in_sec),
            "-c",
            "copy",
            &output.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| lightframe_core::Error::Other(format!("ffmpeg execution failed: {e}")))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(lightframe_core::Error::Other(format!(
            "ffmpeg trim failed: {stderr}"
        )));
    }

    debug!(
        input = %input.display(),
        output = %output.display(),
        in_sec,
        out_sec,
        "video trimmed"
    );
    Ok(())
}

/// Video trim edit parameters, serializable to JSON.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VideoEditParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_trim_in_sec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_trim_out_sec: Option<f64>,
}

impl VideoEditParams {
    pub fn new(in_sec: f64, out_sec: f64) -> Self {
        Self {
            video_trim_in_sec: Some(in_sec),
            video_trim_out_sec: Some(out_sec),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.video_trim_in_sec.is_none() && self.video_trim_out_sec.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_ffmpeg_does_not_panic() {
        let _result = find_ffmpeg();
    }

    #[tokio::test]
    async fn extract_frame_nonexistent_video() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("frame.jpg");
        let result = extract_frame(Path::new("/nonexistent/video.mp4"), &output, 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_duration_nonexistent_video() {
        let result = get_duration(Path::new("/nonexistent/video.mp4")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn trim_export_rejects_invalid_range() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.mp4");
        let output = dir.path().join("output.mp4");
        std::fs::write(&input, b"fake").unwrap();

        // in >= out
        let result = trim_export(&input, &output, 5.0, 5.0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("less than"));

        // in > out
        let result = trim_export(&input, &output, 10.0, 5.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn trim_export_rejects_negative_in() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.mp4");
        let output = dir.path().join("output.mp4");
        std::fs::write(&input, b"fake").unwrap();

        let result = trim_export(&input, &output, -1.0, 5.0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-negative"));
    }

    #[tokio::test]
    async fn trim_export_fails_for_nonexistent_input() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output.mp4");
        let result = trim_export(Path::new("/nonexistent/video.mp4"), &output, 0.0, 5.0).await;
        // Either ffmpeg not found or it fails on the file
        assert!(result.is_err());
    }

    #[test]
    fn video_edit_params_serialization() {
        let params = VideoEditParams::new(2.5, 10.0);
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"video_trim_in_sec\":2.5"));
        assert!(json.contains("\"video_trim_out_sec\":10.0"));

        let parsed: VideoEditParams = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, params);
    }

    #[test]
    fn video_edit_params_deserialization_from_partial() {
        let json = r#"{"video_trim_in_sec":1.0}"#;
        let params: VideoEditParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.video_trim_in_sec, Some(1.0));
        assert_eq!(params.video_trim_out_sec, None);
    }

    #[test]
    fn video_edit_params_empty_check() {
        let empty = VideoEditParams {
            video_trim_in_sec: None,
            video_trim_out_sec: None,
        };
        assert!(empty.is_empty());

        let not_empty = VideoEditParams::new(0.0, 5.0);
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn video_edit_params_skip_serializing_none() {
        let params = VideoEditParams {
            video_trim_in_sec: Some(1.0),
            video_trim_out_sec: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(!json.contains("video_trim_out_sec"));
    }
}

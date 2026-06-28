use catchlight_core::Result;
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
            &output.to_string_lossy().to_string(),
        ])
        .output()
        .await
        .map_err(|e| catchlight_core::Error::Other(format!("ffmpeg not found: {e}")))?;

    if !status.status.success() {
        return Err(catchlight_core::Error::Other(
            String::from_utf8_lossy(&status.stderr).to_string(),
        ));
    }

    debug!(video = %video.display(), timestamp = timestamp_secs, "frame extracted");
    Ok(())
}

pub async fn get_duration(video: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            &video.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| catchlight_core::Error::Other(format!("ffprobe not found: {e}")))?;

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str
        .trim()
        .parse::<f64>()
        .map_err(|e| catchlight_core::Error::Other(format!("failed to parse duration: {e}")))
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    which::which("ffmpeg").ok()
}

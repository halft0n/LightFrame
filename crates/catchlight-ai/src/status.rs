use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatus {
    pub python_available: bool,
    pub clip_available: bool,
    pub face_available: bool,
    pub status_message: String,
}

fn is_python_installed() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_sidecar_module_available() -> bool {
    Command::new("python3")
        .args(["-c", "import catchlight_ai_py"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn check_ai_status() -> AiStatus {
    let python_installed = is_python_installed();
    let sidecar_available = python_installed && is_sidecar_module_available();

    let (clip_available, face_available, status_message) = if !python_installed {
        (
            false,
            false,
            "Python 3 is not installed".to_string(),
        )
    } else if !sidecar_available {
        (
            false,
            false,
            "Python AI extension (catchlight_ai_py) not found".to_string(),
        )
    } else {
        (
            false,
            false,
            "AI extension found; CLIP and face models not yet loaded".to_string(),
        )
    };

    AiStatus {
        python_available: sidecar_available,
        clip_available,
        face_available,
        status_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ai_status_returns_struct() {
        let status = check_ai_status();
        assert!(
            !status.status_message.is_empty(),
            "status message should not be empty"
        );
    }
}

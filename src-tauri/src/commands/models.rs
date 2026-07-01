use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize)]
pub struct ModelStatus {
    pub models_dir: String,
    pub clip_available: bool,
    pub face_available: bool,
    pub models: Vec<lightframe_ai::ModelFileStatus>,
}

#[tauri::command]
pub async fn get_model_status() -> Result<ModelStatus, String> {
    Ok(ModelStatus {
        models_dir: lightframe_ai::models::models_dir()
            .to_string_lossy()
            .to_string(),
        clip_available: lightframe_ai::models::clip_model_available(),
        face_available: lightframe_ai::models::face_model_available(),
        models: lightframe_ai::all_model_statuses(),
    })
}

#[derive(Clone, serde::Serialize)]
struct ModelDownloadProgress {
    filename: String,
    downloaded: u64,
    total: u64,
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    filename: String,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;

    if state
        .downloading
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("model download already in progress".to_string());
    }

    struct DownloadingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DownloadingGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = DownloadingGuard(std::sync::Arc::clone(&state.downloading));

    let model = lightframe_ai::model_by_filename(&filename)
        .ok_or_else(|| format!("unknown model: {filename}"))?;

    let cancel = state.download_cancel.clone();
    cancel.store(false, Ordering::Relaxed);

    let emit_filename = filename.clone();
    let path = tokio::task::spawn_blocking(move || {
        lightframe_ai::download_model_cancellable(
            model,
            move |downloaded, total| {
                let _ = app.emit(
                    "model-download-progress",
                    ModelDownloadProgress {
                        filename: emit_filename.clone(),
                        downloaded,
                        total,
                    },
                );
            },
            Some(&cancel),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state
        .download_cancel
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
pub fn open_models_dir() -> Result<(), String> {
    lightframe_ai::models::ensure_models_dir().map_err(|e| e.to_string())?;
    let path = lightframe_ai::models::models_dir();
    open_in_file_manager(&path)
}

fn open_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open file manager: {e}"))?;
    }
    Ok(())
}

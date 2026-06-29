use crate::state::AppState;
use lightframe_core::media::{MediaType, ThumbnailSize};
use lightframe_db::Database;
use lightframe_thumbnail::{thumb_file_needs_regeneration, thumb_path};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const PAGE_SIZE: i64 = 200;

pub fn media_needs_thumbnail_regeneration(db: &Database, media_id: i64, hash: &str) -> bool {
    if thumb_file_needs_regeneration(&thumb_path(hash, ThumbnailSize::Micro))
        || thumb_file_needs_regeneration(&thumb_path(hash, ThumbnailSize::Small))
    {
        return true;
    }
    db.get_micro_thumb(media_id)
        .ok()
        .flatten()
        .is_none_or(|blob| blob.is_empty())
}

pub fn regenerate_thumbnails_for_media(state: &AppState, media_id: i64) -> Result<bool, String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    let Some(hash) = media.blake3_hash.clone() else {
        return Ok(false);
    };

    if !media_needs_thumbnail_regeneration(&state.db, media_id, &hash) {
        return Ok(false);
    }

    let path = Path::new(&media.path);
    if !path.is_file() {
        tracing::warn!(media_id, path = %media.path, "source missing, skipping thumbnail regen");
        return Ok(false);
    }

    let is_image = matches!(
        media.media_type,
        MediaType::Photo | MediaType::Raw | MediaType::Screenshot
    );

    if is_image {
        let db = Arc::clone(&state.db);
        let hash_clone = hash.clone();
        let path_buf = path.to_path_buf();
        let regenerated = std::thread::spawn(move || -> Result<bool, String> {
            let decoded = lightframe_core::decode::decode_image(&path_buf)
                .map_err(|e| format!("decode failed: {e}"))?;

            lightframe_thumbnail::regenerate_from_decoded(
                &decoded,
                &hash_clone,
                ThumbnailSize::Micro,
            )
            .map_err(|e| e.to_string())?;
            lightframe_thumbnail::regenerate_from_decoded(
                &decoded,
                &hash_clone,
                ThumbnailSize::Small,
            )
            .map_err(|e| e.to_string())?;

            if let Ok(micro) = lightframe_thumbnail::micro_blob_from_decoded(&decoded) {
                let _ = db.set_micro_thumb(media_id, &micro);
            }
            Ok(true)
        })
        .join()
        .map_err(|_| "thumbnail regeneration thread panicked".to_string())??;

        state.thumb_cache.invalidate_media(media_id);
        return Ok(regenerated);
    }

    if matches!(media.media_type, MediaType::Video) {
        if lightframe_video::find_ffmpeg().is_none() {
            return Ok(false);
        }

        let temp_frame = thumb_path(&hash, ThumbnailSize::Small).with_extension("frame.jpg");
        if let Some(parent) = temp_frame.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let rt = tokio::runtime::Handle::current();
        let extracted =
            rt.block_on(async { lightframe_video::extract_frame(path, &temp_frame, 1.0).await });

        if extracted.is_err() || !temp_frame.exists() {
            return Ok(false);
        }

        let hash_clone = hash.clone();
        let frame = temp_frame.clone();
        let regenerated = std::thread::spawn(move || -> Result<bool, String> {
            lightframe_thumbnail::regenerate(&frame, &hash_clone, ThumbnailSize::Micro)
                .map_err(|e| e.to_string())?;
            lightframe_thumbnail::regenerate(&frame, &hash_clone, ThumbnailSize::Small)
                .map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&frame);
            Ok(true)
        })
        .join()
        .map_err(|_| "video thumbnail regeneration thread panicked".to_string())??;

        if regenerated {
            let db = Arc::clone(&state.db);
            let hash_for_micro = hash.clone();
            if let Ok(micro) = std::thread::spawn(move || {
                let small = thumb_path(&hash_for_micro, ThumbnailSize::Small);
                lightframe_thumbnail::generate_micro_blob(&small)
            })
            .join()
                && let Ok(blob) = micro
            {
                let _ = db.set_micro_thumb(media_id, &blob);
            }
        }

        state.thumb_cache.invalidate_media(media_id);
        return Ok(regenerated);
    }

    Ok(false)
}

#[derive(serde::Serialize, Clone)]
pub struct ThumbnailRegenProgress {
    pub processed: i64,
    pub total: i64,
    pub regenerated: i64,
    pub status: String,
}

#[derive(serde::Serialize)]
pub struct ThumbnailRegenResult {
    pub regenerated: i64,
}

pub async fn regenerate_all_thumbnails(
    app: AppHandle,
    state: &AppState,
) -> Result<ThumbnailRegenResult, String> {
    let total = state.db.get_media_count().map_err(|e| e.to_string())?;
    let mut processed = 0i64;
    let mut regenerated = 0i64;
    let mut offset = 0i64;

    let emit = |processed: i64, regenerated: i64, status: &str| {
        let payload = ThumbnailRegenProgress {
            processed,
            total,
            regenerated,
            status: status.to_string(),
        };
        if let Err(e) = app.emit("thumbnail-regen-progress", &payload) {
            tracing::warn!("failed to emit thumbnail-regen-progress: {e}");
        }
    };

    emit(0, 0, "running");

    while offset < total {
        let batch = state
            .db
            .get_all_media(PAGE_SIZE, offset)
            .map_err(|e| e.to_string())?;

        if batch.is_empty() {
            break;
        }

        for media in batch {
            processed += 1;
            match regenerate_thumbnails_for_media(state, media.id) {
                Ok(true) => regenerated += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(media_id = media.id, "thumbnail regen failed: {e}"),
            }
            emit(processed, regenerated, "running");
        }

        offset += PAGE_SIZE;
    }

    emit(processed, regenerated, "complete");

    Ok(ThumbnailRegenResult { regenerated })
}

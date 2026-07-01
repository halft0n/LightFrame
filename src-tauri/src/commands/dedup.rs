use crate::state::AppState;
use lightframe_db::DuplicateGroupDetail;
use serde::Serialize;
use tauri::State;

use super::remove_media_from_disk;

#[derive(Serialize)]
pub struct DedupScanResult {
    pub exact_groups: usize,
    pub perceptual_groups: usize,
    pub total_duplicates: usize,
}
#[tauri::command]
pub async fn run_dedup_scan(state: State<'_, AppState>) -> Result<DedupScanResult, String> {
    use std::sync::atomic::Ordering;

    if state
        .dedup_scanning
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("dedup scan already in progress".to_string());
    }

    struct DedupScanningGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DedupScanningGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = DedupScanningGuard(std::sync::Arc::clone(&state.dedup_scanning));

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.clear_duplicate_groups().map_err(|e| e.to_string())?;

        let exact_groups = db.find_exact_duplicates().map_err(|e| e.to_string())?;
        let perceptual_groups = db
            .find_perceptual_duplicates(10)
            .map_err(|e| e.to_string())?;

        let total_duplicates = exact_groups
            .iter()
            .chain(perceptual_groups.iter())
            .map(|g| g.members.len())
            .sum();

        Ok(DedupScanResult {
            exact_groups: exact_groups.len(),
            perceptual_groups: perceptual_groups.len(),
            total_duplicates,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_groups(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroupDetail>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.list_duplicate_groups().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_duplicate_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_duplicate_groups_count().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_duplicate(
    state: State<'_, AppState>,
    group_id: i64,
    keep_media_id: i64,
    delete_files: bool,
) -> Result<(), String> {
    let db = state.db.clone();
    let removed_ids: Vec<i64> = tokio::task::spawn_blocking(move || {
        let group = db
            .get_duplicate_group_by_id(group_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("group {group_id} not found"))?;

        let others: Vec<i64> = group
            .members
            .iter()
            .filter(|m| m.media_id != keep_media_id)
            .map(|m| m.media_id)
            .collect();

        if delete_files {
            for &media_id in &others {
                let item = db
                    .get_media_by_id(media_id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("media {media_id} not found"))?;
                db.set_deleted(media_id, true).map_err(|e| e.to_string())?;
                db.permanently_delete_media(media_id)
                    .map_err(|e| e.to_string())?;
                remove_media_from_disk(&item.path, item.blake3_hash.as_deref(), &db);
            }
        } else {
            for &media_id in &others {
                db.set_deleted(media_id, true).map_err(|e| e.to_string())?;
            }
        }
        db.delete_duplicate_group(group_id)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(others)
    })
    .await
    .map_err(|e| e.to_string())??;

    for media_id in removed_ids {
        state.thumb_cache.invalidate_media(media_id);
        crate::face_protocol::invalidate_face_cache_for_media(&state, media_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn dismiss_duplicate_group(
    state: State<'_, AppState>,
    group_id: i64,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.delete_duplicate_group(group_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

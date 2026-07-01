use crate::state::AppState;
use lightframe_ai::AiStatus;
use lightframe_core::media::MediaFile;
use lightframe_db::{Memory, Person};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::{db_err, truncate_utf8, validate_media_path};

#[derive(Serialize)]
pub struct SearchResult {
    pub media_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub relevance: f32,
}

#[derive(Serialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SearchResult>,
    pub used_semantic: bool,
}

const SEMANTIC_SEARCH_THRESHOLD: f32 = 0.15;
const MAX_SEMANTIC_QUERY_LEN: usize = 512;

/// CLIP vector search when text embedding is available; otherwise FTS5 keyword fallback.
#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    query_text: String,
    limit: Option<usize>,
) -> Result<SemanticSearchResponse, String> {
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let query_text = if query_text.len() > MAX_SEMANTIC_QUERY_LEN {
        truncate_utf8(&query_text, MAX_SEMANTIC_QUERY_LEN).to_string()
    } else {
        query_text
    };
    let trimmed = query_text.trim();
    if trimmed.is_empty() {
        return Ok(SemanticSearchResponse {
            results: Vec::new(),
            used_semantic: false,
        });
    }

    let text_embedding = {
        let mut ai = state.ai.lock().await;
        ai.ensure_python()
            .await
            .map_err(|e| format!("semantic_search: {e}"))?;
        ai.compute_text_embedding(trimmed)
            .await
            .map_err(|e| format!("semantic_search({trimmed}): {e}"))?
    };

    if let Some(embedding) = text_embedding {
        tracing::info!(query = trimmed, "semantic search using CLIP text embedding");
        let matches = state
            .db
            .semantic_search_by_embedding(&embedding, SEMANTIC_SEARCH_THRESHOLD, limit)
            .map_err(|e| db_err("semantic_search", trimmed, e))?;

        return Ok(SemanticSearchResponse {
            used_semantic: true,
            results: matches
                .into_iter()
                .map(|(m, score)| SearchResult {
                    media_id: m.id,
                    file_name: m.filename,
                    file_path: m.path,
                    relevance: score,
                })
                .collect(),
        });
    }

    tracing::info!(
        query = trimmed,
        "CLIP text embedding unavailable, falling back to FTS5"
    );
    let media = state
        .db
        .search_media(trimmed, limit as i64, 0)
        .map_err(|e| db_err("semantic_search", trimmed, e))?;

    Ok(SemanticSearchResponse {
        used_semantic: false,
        results: media
            .into_iter()
            .enumerate()
            .map(|(i, m)| SearchResult {
                media_id: m.id,
                file_name: m.filename,
                file_path: m.path,
                relevance: 1.0 - (i as f32 * 0.01).min(0.9),
            })
            .collect(),
    })
}
#[tauri::command]
pub fn get_on_this_day(state: State<'_, AppState>, limit: i64) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    state
        .db
        .get_on_this_day_media(limit)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn generate_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.generate_memories().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_memories(state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    state.db.list_memories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memory_media(
    state: State<'_, AppState>,
    memory_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_memory_media(memory_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ai_status() -> AiStatus {
    lightframe_ai::check_ai_status()
}
#[derive(Serialize)]
pub struct SimilarPhoto {
    pub media_id: i64,
    pub similarity: f32,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct FaceInfo {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub person_id: Option<i64>,
}

fn face_record_to_info(r: lightframe_db::FaceDetectionRecord) -> FaceInfo {
    FaceInfo {
        id: r.id,
        media_id: r.media_id,
        bbox: [r.bbox_x, r.bbox_y, r.bbox_x + r.bbox_w, r.bbox_y + r.bbox_h],
        confidence: r.confidence,
        person_id: r.person_id,
    }
}

const SIMILAR_THRESHOLD: f32 = 0.65;

#[tauri::command]
pub async fn compute_clip_embedding(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<(), String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    validate_media_path(&state.db, &media.path)?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let mut ai = state.ai.lock().await;
    ai.ensure_python().await.map_err(|e| e.to_string())?;

    let embedding = ai
        .compute_embedding(path)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "CLIP embedding unavailable — place a CLIP ONNX model in the data/models directory or install the Python AI extension".to_string()
        })?;

    state
        .db
        .store_clip_embedding(media_id, &embedding)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BatchEmbedResult {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn compute_clip_embeddings_batch(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<BatchEmbedResult, String> {
    let limit = limit.unwrap_or(32).clamp(1, 256) as i64;
    let candidates = state
        .db
        .list_media_without_clip_embedding(limit)
        .map_err(|e| e.to_string())?;

    if candidates.is_empty() {
        return Ok(BatchEmbedResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }

    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    for (media_id, path_str) in &candidates {
        if let Err(e) = validate_media_path(&state.db, path_str) {
            failed += 1;
            errors.push(format!("media {media_id}: {e}"));
            continue;
        }

        let path = std::path::Path::new(path_str);
        if !path.is_file() {
            failed += 1;
            errors.push(format!("media {media_id}: file not found"));
            continue;
        }

        let result = {
            let ai = state.ai.lock().await;
            ai.compute_embedding(path).await
        };

        match result {
            Ok(Some(embedding)) => {
                if let Err(e) = state.db.store_clip_embedding(*media_id, &embedding) {
                    failed += 1;
                    errors.push(format!("media {media_id}: {e}"));
                } else {
                    succeeded += 1;
                }
            }
            Ok(None) => {
                failed += 1;
                errors.push(format!("media {media_id}: CLIP embedding unavailable"));
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("media {media_id}: {e}"));
            }
        }
    }

    Ok(BatchEmbedResult {
        processed: candidates.len(),
        succeeded,
        failed,
        errors,
    })
}

#[tauri::command]
pub async fn find_similar_photos(
    state: State<'_, AppState>,
    media_id: i64,
    limit: Option<usize>,
) -> Result<Vec<SimilarPhoto>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 100);

    if state
        .db
        .get_clip_embedding(media_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        compute_clip_embedding(state.clone(), media_id).await?;
    }

    let similar = state
        .db
        .find_similar_media(media_id, SIMILAR_THRESHOLD, limit)
        .map_err(|e| e.to_string())?;

    let ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();
    let media_map = state.db.get_media_by_ids(&ids).map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(similar.len());
    for (id, score) in similar {
        let media = media_map
            .get(&id)
            .ok_or_else(|| format!("media {id} not found"))?;
        results.push(SimilarPhoto {
            media_id: id,
            similarity: score,
            file_name: media.filename.clone(),
            file_path: media.path.clone(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn detect_faces(
    state: State<'_, AppState>,
    media_id: i64,
) -> Result<Vec<FaceInfo>, String> {
    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    detect_and_store_faces_for_media(&state, media_id).await?;

    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[tauri::command]
pub fn get_faces(state: State<'_, AppState>, media_id: i64) -> Result<Vec<FaceInfo>, String> {
    let records = state
        .db
        .get_faces_for_media(media_id)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[derive(Serialize, Clone)]
pub struct FaceDetectionProgress {
    pub processed: i64,
    pub total: i64,
    pub faces_found: i64,
    pub status: String,
}

#[derive(Serialize)]
pub struct FaceDetectionBatchResult {
    pub media_processed: i64,
    pub faces_found: i64,
}

#[tauri::command]
pub async fn detect_faces_batch(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<FaceDetectionBatchResult, String> {
    use std::sync::atomic::Ordering;

    if state
        .face_detecting
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("face detection already in progress".to_string());
    }

    struct FaceDetectingGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for FaceDetectingGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = FaceDetectingGuard(std::sync::Arc::clone(&state.face_detecting));

    let media_ids = state
        .db
        .get_media_ids_without_faces(500)
        .map_err(|e| e.to_string())?;

    let total = media_ids.len() as i64;
    let mut processed = 0i64;
    let mut faces_found = 0i64;

    let emit_progress = |processed: i64, faces_found: i64, status: &str| {
        let payload = FaceDetectionProgress {
            processed,
            total,
            faces_found,
            status: status.to_string(),
        };
        if let Err(e) = app.emit("face-detection-progress", &payload) {
            tracing::warn!("failed to emit face-detection-progress: {e}");
        }
    };

    emit_progress(0, 0, "detecting");

    {
        let mut ai = state.ai.lock().await;
        ai.ensure_python().await.map_err(|e| e.to_string())?;
    }

    for media_id in media_ids {
        match detect_and_store_faces_for_media(&state, media_id).await {
            Ok(count) => faces_found += count,
            Err(e) => tracing::warn!(media_id, "face detection failed: {e}"),
        }
        processed += 1;
        emit_progress(processed, faces_found, "detecting");
    }

    emit_progress(processed, faces_found, "complete");

    Ok(FaceDetectionBatchResult {
        media_processed: processed,
        faces_found,
    })
}

async fn detect_and_store_faces_for_media(state: &AppState, media_id: i64) -> Result<i64, String> {
    let media = state
        .db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media {media_id} not found"))?;

    validate_media_path(&state.db, &media.path)?;

    let path = std::path::Path::new(&media.path);
    if !path.is_file() {
        return Err(format!("media file not found: {}", media.path));
    }

    let faces = {
        let ai = state.ai.lock().await;
        ai.detect_faces_in_image(path)
            .await
            .map_err(|e| e.to_string())?
    };

    let count = faces.len() as i64;
    let inputs: Vec<lightframe_db::FaceDetectionInput> = faces
        .iter()
        .map(|f| lightframe_db::FaceDetectionInput {
            bbox: f.bbox,
            confidence: f.confidence,
            embedding: f.embedding.clone(),
        })
        .collect();
    // Invalidate old face cache before storing new detections
    crate::face_protocol::invalidate_face_cache_for_media(state, media_id);
    state
        .db
        .store_face_detections(media_id, &inputs)
        .map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub fn get_person_faces(
    state: State<'_, AppState>,
    person_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<FaceInfo>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    let records = state
        .db
        .get_faces_for_person(person_id, limit, offset)
        .map_err(|e| e.to_string())?;

    Ok(records.into_iter().map(face_record_to_info).collect())
}

#[tauri::command]
pub fn list_persons(state: State<'_, AppState>) -> Result<Vec<Person>, String> {
    state.db.list_persons().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_person_media(
    state: State<'_, AppState>,
    person_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<MediaFile>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    state
        .db
        .get_person_media(person_id, limit, offset)
        .map_err(|e| e.to_string())
}

pub(crate) fn rename_person_impl(
    state: &AppState,
    person_id: i64,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("person name cannot be empty".to_string());
    }
    state
        .db
        .rename_person(person_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_person(
    state: State<'_, AppState>,
    person_id: i64,
    name: String,
) -> Result<(), String> {
    rename_person_impl(&state, person_id, name)
}

#[derive(Serialize)]
pub struct PersonClusterInfo {
    pub person_id: i64,
    pub name: Option<String>,
    pub face_count: i64,
    pub avg_intra_cluster_distance: f32,
}

const DEFAULT_FACE_CLUSTER_THRESHOLD: f32 = 0.45;

#[tauri::command]
pub async fn cluster_faces(
    state: State<'_, AppState>,
    threshold: Option<f32>,
) -> Result<Vec<PersonClusterInfo>, String> {
    let threshold = threshold.unwrap_or(DEFAULT_FACE_CLUSTER_THRESHOLD);
    if threshold.is_nan() || !(0.0..=1.0).contains(&threshold) {
        return Err("cluster threshold must be between 0.0 and 1.0".into());
    }
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        db.unassign_faces_from_unnamed_persons()
            .map_err(|e| e.to_string())?;
        let faces = db
            .get_unassigned_face_embeddings()
            .map_err(|e| e.to_string())?;
        if faces.is_empty() {
            return Ok(Vec::new());
        }

        let clusters = lightframe_ai::cluster_face_embeddings(&faces, threshold);
        let cluster_inputs: Vec<_> = clusters
            .iter()
            .map(|c| (c.face_ids.clone(), None))
            .collect();
        let person_ids = db
            .cluster_and_assign_faces(&cluster_inputs)
            .map_err(|e| e.to_string())?;
        Ok(clusters
            .into_iter()
            .zip(person_ids)
            .map(|(cluster, person_id)| PersonClusterInfo {
                person_id,
                name: None,
                face_count: cluster.face_ids.len() as i64,
                avg_intra_cluster_distance: cluster.avg_intra_cluster_distance,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn merge_persons(
    state: State<'_, AppState>,
    person_id_a: i64,
    person_id_b: i64,
) -> Result<(), String> {
    if person_id_a == person_id_b {
        return Err("cannot merge a person with itself".to_string());
    }
    state
        .db
        .merge_persons(person_id_a, &[person_id_b])
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn split_face_from_person(
    state: State<'_, AppState>,
    face_id: i64,
    new_person_name: Option<String>,
) -> Result<i64, String> {
    state
        .db
        .split_face_from_person(face_id, new_person_name.as_deref())
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod rename_person_tests {
    use super::*;
    use crate::state::ScanStatus;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn test_app_state(db: Arc<lightframe_db::Database>) -> AppState {
        AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: tempfile::tempdir().unwrap().into_path(),
        }
    }

    #[test]
    fn rename_person_rejects_empty_and_whitespace_names() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db.clone());
        let person_id = db.create_person(Some("Original")).unwrap();

        for name in ["", "  ", "\t", "   \n  "] {
            let err = rename_person_impl(&state, person_id, name.to_string()).unwrap_err();
            assert!(err.contains("cannot be empty"), "name={name:?}: {err}");
        }
    }

    #[test]
    fn rename_person_trims_whitespace() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(lightframe_db::Database::open(&root.path().join("library.db")).unwrap());
        let state = test_app_state(db.clone());
        let person_id = db.create_person(Some("Original")).unwrap();

        rename_person_impl(&state, person_id, "  Alicia  ".to_string()).unwrap();
        let persons = db.list_persons().unwrap();
        assert_eq!(persons[0].name.as_deref(), Some("Alicia"));
    }
}

#[cfg(test)]
mod face_detection_tests {
    use super::*;
    use crate::state::ScanStatus;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    #[tokio::test]
    async fn detect_faces_releases_ai_lock_on_not_found() {
        let root = tempfile::tempdir().unwrap();
        let db = Arc::new(
            lightframe_db::Database::open(root.path().join("library.db").as_path()).unwrap(),
        );
        let state = AppState {
            db,
            config: lightframe_core::config::AppConfig::default(),
            scan_status: ScanStatus::new(),
            scan_concurrency: 4,
            scan_queue: crate::state::ScanQueue::new(),
            face_detecting: Arc::new(AtomicBool::new(false)),
            dedup_scanning: Arc::new(AtomicBool::new(false)),
            thumb_regenerating: Arc::new(AtomicBool::new(false)),
            active_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            watch_manager: crate::watcher::WatchManager::new(),
            thumb_cache: crate::thumb_cache::ThumbCache::new(),
            ai: Arc::new(tokio::sync::Mutex::new(lightframe_ai::AiDispatcher::new())),
            face_cache_dir: tempfile::tempdir().unwrap().into_path(),
        };

        let err = detect_and_store_faces_for_media(&state, 99999)
            .await
            .unwrap_err();
        assert!(err.contains("not found"));

        let lock = state.ai.try_lock();
        assert!(
            lock.is_ok(),
            "AI mutex should be released after early return"
        );
    }
}

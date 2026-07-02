mod commands;
mod face_protocol;
mod image_edit;
mod logging;
mod memory;
mod memory_budget;
mod original_protocol;
mod protocol_utils;
mod scan;
mod state;
mod thumb_cache;
mod thumb_protocol;
mod thumb_regen;
mod watcher;

#[doc(hidden)]
pub use image_edit::export_edited_image;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    let app_state = match AppState::new() {
        Ok(state) => state,
        Err(e) => {
            let msg = format!("LightFrame failed to start: {e}");
            eprintln!("{msg}");
            if let Some(dir) = dirs::data_local_dir() {
                let crash_log = dir.join("lightframe").join("crash.log");
                let _ = std::fs::create_dir_all(crash_log.parent().unwrap());
                let _ = std::fs::write(&crash_log, &msg);
            }
            std::process::exit(1);
        }
    };
    let _guard = logging::init_logging(&app_state.config.log);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let log_dir = crate::logging::log_directory();
                crate::logging::cleanup_logs(&log_dir);
            }
        })
        .setup(|app| {
            lightframe_ai::cleanup_partial_downloads();

            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            if let Err(e) = watcher::start(&handle, &state) {
                tracing::warn!("failed to start folder watcher: {e}");
            }

            // Recover interrupted rebuild if staging tables exist from a crash
            if state.db.has_pending_rebuild().unwrap_or(false) {
                tracing::info!("pending rebuild detected — scheduling choice restoration");
                let db_for_recovery = Arc::clone(&state.db);
                let scan_running = state.scan_queue.running_flag();
                tauri::async_runtime::spawn(async move {
                    // Wait for any initial/watcher-triggered scans
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let mut wait_ms = 5000u64;
                    while scan_running
                        .load(std::sync::atomic::Ordering::SeqCst)
                        && wait_ms < 1_800_000
                    {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        wait_ms += 2000;
                    }
                    match db_for_recovery.restore_rebuild_choices() {
                        Ok((fav, edits, albums, faces)) => {
                            tracing::info!(
                                "startup recovery: restored {fav} favorites, {edits} edits, {albums} album items, {faces} faces"
                            );
                        }
                        Err(e) => {
                            tracing::error!("startup recovery: restore failed: {e}");
                        }
                    }
                });
            }

            // Spawn periodic memory budget check (every 60s)
            let thumb_cache = Arc::clone(&state.thumb_cache);
            tauri::async_runtime::spawn(async move {
                let cpus = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                let pressure_tracker = memory_budget::PressureTracker::new();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    if let Some((total, available)) = memory_budget::get_system_memory() {
                        if memory_budget::memory_data_unavailable(available, total) {
                            continue;
                        }
                        if pressure_tracker.check(available, total) {
                            memory_budget::log_pressure(available, total);
                            thumb_cache.resize(
                                memory_budget::PRESSURE_BUDGET.micro_cap,
                                memory_budget::PRESSURE_BUDGET.standard_cap,
                            );
                        } else {
                            let budget = memory_budget::compute_thumb_budget(available, cpus);
                            thumb_cache.resize(budget.micro_cap, budget.standard_cap);
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::check_for_updates,
            commands::get_config,
            commands::add_watched_folder,
            commands::remove_watched_folder,
            commands::list_watched_folders,
            commands::get_media_list,
            commands::get_media_page,
            commands::get_media_count,
            commands::get_media_by_folder,
            commands::get_media_count_by_folder,
            commands::batch_export,
            commands::get_media_by_id,
            commands::scan_folder,
            commands::get_scan_status,
            commands::start_watching,
            commands::stop_watching,
            commands::get_timeline_groups,
            commands::get_media_neighbors,
            commands::get_media_window,
            commands::run_dedup_scan,
            commands::get_duplicate_groups,
            commands::get_duplicate_count,
            commands::resolve_duplicate,
            commands::dismiss_duplicate_group,
            commands::get_location_groups,
            commands::get_media_by_location,
            commands::get_location_stats,
            commands::get_media_by_type,
            commands::get_media_count_by_type,
            commands::get_screenshots,
            commands::get_screenshot_count,
            commands::get_model_status,
            commands::download_model,
            commands::cancel_download,
            commands::open_models_dir,
            commands::create_album,
            commands::delete_album,
            commands::update_album,
            commands::set_album_cover,
            commands::list_albums,
            commands::add_to_album,
            commands::remove_from_album,
            commands::get_album_media,
            commands::toggle_favorite,
            commands::get_favorites,
            commands::get_favorites_count,
            commands::is_favorite,
            commands::delete_media,
            commands::get_deleted_media,
            commands::restore_media,
            commands::permanently_delete,
            commands::batch_delete_media,
            commands::batch_add_to_album,
            commands::batch_toggle_favorite,
            commands::batch_restore_media,
            commands::batch_permanent_delete,
            commands::search_media,
            commands::search_media_count,
            commands::semantic_search,
            commands::create_smart_album,
            commands::list_smart_albums,
            commands::delete_smart_album,
            commands::get_smart_album_media,
            commands::generate_memories,
            commands::get_on_this_day,
            commands::list_memories,
            commands::get_memory_media,
            commands::get_ai_status,
            commands::compute_clip_embedding,
            commands::compute_clip_embeddings_batch,
            commands::find_similar_photos,
            commands::detect_faces,
            commands::detect_faces_batch,
            commands::get_faces,
            commands::get_person_faces,
            commands::list_persons,
            commands::get_person_media,
            commands::rename_person,
            commands::cluster_faces,
            commands::merge_persons,
            commands::split_face_from_person,
            commands::create_manual_face,
            commands::list_person_names,
            commands::save_edit,
            commands::get_edit,
            commands::revert_edit,
            commands::export_edited,
            commands::has_edits,
            commands::get_log_directory,
            commands::get_log_files,
            commands::cleanup_logs,
            commands::get_log_config,
            commands::set_log_config,
            commands::regenerate_thumbnails,
            commands::regenerate_thumbnail_single,
            commands::get_media_with_geo,
            commands::get_geo_clusters,
            commands::reset_database,
            commands::get_memory_budget,
            commands::save_video_trim,
            commands::get_video_trim,
            commands::export_trimmed_video,
            commands::get_video_duration,
            commands::rebuild_cache,
            commands::get_pinned_items,
            commands::pin_item,
            commands::unpin_item,
            commands::create_person_group,
            commands::rename_person_group,
            commands::delete_person_group,
            commands::set_group_cover,
            commands::add_person_to_group,
            commands::remove_person_from_group,
            commands::list_person_groups,
            commands::get_group_members,
        ])
        .register_uri_scheme_protocol("thumb", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            thumb_protocol::handle(&state, request.uri().path())
        })
        .register_uri_scheme_protocol("face", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            face_protocol::handle(&state, request.uri().path())
        })
        .register_uri_scheme_protocol("original", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            original_protocol::handle(&state, request.uri().path())
        })
        .run(tauri::generate_context!())
        .expect("error while running LightFrame");
}

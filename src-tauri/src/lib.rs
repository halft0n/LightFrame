mod commands;
mod image_edit;
mod original_protocol;
mod scan;
mod state;
mod thumb_protocol;
mod watcher;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("catchlight=debug".parse().unwrap()),
        )
        .init();

    let app_state = AppState::new().expect("failed to initialize application state");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            if let Err(e) = watcher::start(&handle, &state) {
                tracing::warn!("failed to start folder watcher: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
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
            commands::create_smart_album,
            commands::list_smart_albums,
            commands::delete_smart_album,
            commands::get_smart_album_media,
            commands::generate_memories,
            commands::get_on_this_day,
            commands::list_memories,
            commands::get_memory_media,
            commands::get_ai_status,
            commands::list_persons,
            commands::get_person_media,
            commands::rename_person,
            commands::save_edit,
            commands::get_edit,
            commands::revert_edit,
            commands::export_edited,
            commands::has_edits,
        ])
        .register_uri_scheme_protocol("thumb", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            thumb_protocol::handle(&state, request.uri().path())
        })
        .register_uri_scheme_protocol("original", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            original_protocol::handle(&state, request.uri().path())
        })
        .run(tauri::generate_context!())
        .expect("error while running CatchLight");
}

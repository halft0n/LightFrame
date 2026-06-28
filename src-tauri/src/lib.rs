mod commands;
mod scan;
mod state;
mod thumb_protocol;

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
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::add_watched_folder,
            commands::remove_watched_folder,
            commands::list_watched_folders,
            commands::get_media_list,
            commands::get_media_count,
            commands::get_media_by_id,
            commands::scan_folder,
            commands::get_scan_status,
            commands::get_timeline_groups,
            commands::get_media_neighbors,
        ])
        .register_uri_scheme_protocol("thumb", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            thumb_protocol::handle(&state, request.uri().path())
        })
        .run(tauri::generate_context!())
        .expect("error while running CatchLight");
}

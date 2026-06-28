mod commands;

use tracing_subscriber::EnvFilter;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("catchlight=debug".parse().unwrap()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CatchLight");
}

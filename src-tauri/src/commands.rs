#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Welcome to CatchLight, {}!", name)
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

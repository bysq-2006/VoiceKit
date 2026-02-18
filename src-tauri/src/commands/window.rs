use crate::utils::window_ext;

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) {
    window_ext::show_no_activate(&app, "main");
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) {
    window_ext::hide(&app, "main");
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

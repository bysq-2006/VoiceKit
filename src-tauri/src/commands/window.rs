use crate::utils::window_ext;
use tauri::Manager;

#[tauri::command]
pub fn show_window(
    app: tauri::AppHandle,
    width: Option<u32>,
    height: Option<u32>,
) {
    // 如果提供了尺寸，先设置窗口大小
    if let (Some(w), Some(h)) = (width, height) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_size(
                tauri::Size::Physical(tauri::PhysicalSize::new(w, h))
            );
        }
    }
    
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

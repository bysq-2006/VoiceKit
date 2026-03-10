use tauri::{State, Manager};
use crate::models::state::AppState;
use crate::utils::window_ext;

// ============== 窗口控制 ==============

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

// ============== 录音控制 ==============

#[tauri::command]
pub fn set_recording(
    app: tauri::AppHandle,
    state: State<AppState>,
    recording: bool,
) -> Result<(), String> {
    crate::utils::recording_state::set(&state, &app, recording);
    Ok(())
}

#[tauri::command]
pub fn hide_and_stop_recording(app: tauri::AppHandle) {
    window_ext::hide(&app, "main");
}

#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> bool {
    *state.is_recording.lock().unwrap()
}

use tauri::{Emitter, Manager, State};
use crate::models::state::AppState;

#[tauri::command]
pub fn set_recording(
    app: tauri::AppHandle,
    state: State<AppState>,
    recording: bool,
) -> Result<(), String> {
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = recording;
    }
    
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", recording);
    }
    
    Ok(())
}

#[tauri::command]
pub fn hide_and_stop_recording(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> bool {
    *state.is_recording.lock().unwrap()
}

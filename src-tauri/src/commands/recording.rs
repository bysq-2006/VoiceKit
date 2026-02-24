use tauri::State;
use crate::models::state::AppState;
use crate::utils::window_ext;

#[tauri::command]
pub fn set_recording(
    app: tauri::AppHandle,
    state: State<AppState>,
    recording: bool,
) -> Result<(), String> {
    crate::utils::recording::set(&state, &app, recording);
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

use crate::models::state::AppState;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

static DEBOUNCE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn try_run(f: impl FnOnce()) -> bool {
    let Ok(_guard) = DEBOUNCE_LOCK.try_lock() else { return false };
    f();
    thread::sleep(Duration::from_millis(100));
    true
}

fn emit(app_handle: &tauri::AppHandle, state: bool) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", state);
    }
}

pub fn toggle(state: &AppState, app_handle: &tauri::AppHandle) -> bool {
    try_run(|| {
        let new_state = {
            let mut r = state.is_recording.lock().unwrap();
            *r = !*r;
            *r
        };
        emit(app_handle, new_state);
        log::info!("录音: {}", new_state);
    })
}

pub fn set(state: &AppState, app_handle: &tauri::AppHandle, recording: bool) -> bool {
    try_run(|| {
        *state.is_recording.lock().unwrap() = recording;
        emit(app_handle, recording);
        log::info!("录音: {}", recording);
    })
}

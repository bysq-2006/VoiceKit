use crate::models::state::AppState;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

static LAST_TRIGGER: AtomicU64 = AtomicU64::new(0);
const DEBOUNCE_MS: u64 = 200;

fn try_run(f: impl FnOnce()) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let last = LAST_TRIGGER.load(Ordering::SeqCst);
    if now.saturating_sub(last) < DEBOUNCE_MS {
        return false;
    }
    
    LAST_TRIGGER.store(now, Ordering::SeqCst);
    f();
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

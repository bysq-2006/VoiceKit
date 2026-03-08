//! 录音状态管理器
//! 
//! 职责：
//! - 管理录音的开启/关闭状态（带防抖保护，防止快速连击）
//! - 向前端发送录音状态变更事件，驱动UI动画（如麦克风按钮的波纹效果）
//! 
//! 主入口：
//! - `toggle()` - 切换录音状态（快捷键 Shift+E 调用）
//! - `set()`    - 直接设置录音状态（UI按钮或自动停止调用）

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

pub fn toggle(state: &AppState, app_handle: &tauri::AppHandle) -> bool {
    try_run(|| {
        let new_state = {
            let mut r = state.is_recording.lock().unwrap();
            *r = !*r;
            *r
        };
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", new_state);
        }
        log::info!("录音: {}", new_state);
    })
}

pub fn set(state: &AppState, app_handle: &tauri::AppHandle, recording: bool) -> bool {
    try_run(|| {
        *state.is_recording.lock().unwrap() = recording;
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", recording);
        }
        log::info!("录音: {}", recording);
    })
}

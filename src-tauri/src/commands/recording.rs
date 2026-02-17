use tauri::{Emitter, Manager, State};
use crate::models::state::AppState;

/// 切换录音状态（后端控制，前端只请求）
#[tauri::command]
pub fn toggle_recording(app: tauri::AppHandle) {
    // 从 app 获取状态
    let state = app.state::<AppState>();
    
    // 获取当前状态并切换
    let mut is_recording = state.is_recording.lock().unwrap();
    *is_recording = !*is_recording;
    let new_state = *is_recording;
    drop(is_recording); // 先释放锁，再发送事件
    
    // 发送事件通知主窗口前端
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", new_state);
    }
}

/// 隐藏窗口并停止录音
#[tauri::command]
pub fn hide_and_stop_recording(app: tauri::AppHandle) {
    // 1. 隐藏窗口
    if let Some(window) = app.get_webview_window("main") {
        let _: Result<(), _> = window.hide();
    }
    
    // 2. 停止录音（如果正在录音）
    let state = app.state::<AppState>();
    let mut is_recording = state.is_recording.lock().unwrap();
    
    if *is_recording {
        *is_recording = false;
        drop(is_recording); // 释放锁再发送事件
        
        // 通知主窗口前端录音已停止
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", false);
        }
    }
}

/// 获取录音状态（前端初始化用）
#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> bool {
    *state.is_recording.lock().unwrap()
}

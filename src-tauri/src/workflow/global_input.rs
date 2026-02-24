//! 全局输入监听

use crate::models::state::AppState;
use rdev::{listen, EventType, Key};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

/// 是否为单独按下的修饰键（不触发取消）
fn is_modifier_only(key: &Key) -> bool {
    matches!(key, Key::ShiftLeft | Key::ShiftRight |
                  Key::ControlLeft | Key::ControlRight |
                  Key::Alt | Key::AltGr |
                  Key::MetaLeft | Key::MetaRight)
}

/// 启动全局输入监听（录音状态下任意输入取消录音）
pub fn init(app_state: Arc<AppState>, app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        log::info!("全局输入监听已启动");

        if let Err(e) = listen(move |event| {
            // 只在录音状态下处理
            if !*app_state.is_recording.lock().unwrap() {
                return;
            }

            // 如果正在模拟输入，忽略
            if app_state.is_simulating_input.load(Ordering::SeqCst) {
                return;
            }

            // 检测到非修饰键的键盘输入或鼠标输入，取消录音
            match event.event_type {
                EventType::KeyPress(key) if !is_modifier_only(&key) => {
                    log::info!("录音状态下检测到按键，取消录音");
                    crate::utils::recording::set(&app_state, &app_handle, false);
                }
                EventType::ButtonPress(_) => {
                    log::info!("录音状态下检测到鼠标点击，取消录音");
                    crate::utils::recording::set(&app_state, &app_handle, false);
                }
                _ => {}
            }
        }) {
            log::error!("全局输入监听错误: {:?}", e);
        }
    });
}

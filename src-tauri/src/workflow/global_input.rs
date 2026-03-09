//! 全局输入监听

use crate::models::state::AppState;
use rdev::{listen, EventType, Key};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use tauri::Manager;

#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// 是否为单独按下的修饰键（不触发取消）
fn is_modifier_only(key: &Key) -> bool {
    matches!(key, Key::ShiftLeft | Key::ShiftRight |
                  Key::ControlLeft | Key::ControlRight |
                  Key::Alt | Key::AltGr |
                  Key::MetaLeft | Key::MetaRight)
}

/// 获取全局鼠标位置 (Windows)
#[cfg(windows)]
fn get_mouse_pos() -> Option<(i32, i32)> {
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point).is_ok() {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_mouse_pos() -> Option<(i32, i32)> {
    // 非 Windows 平台暂时返回 None
    None
}

/// 检查鼠标位置是否在窗口内
fn is_point_in_window(window: &tauri::WebviewWindow, x: i32, y: i32) -> bool {
    if let Ok(pos) = window.outer_position() {
        if let Ok(size) = window.outer_size() {
            let win_x = pos.x;
            let win_y = pos.y;
            let win_w = size.width as i32;
            let win_h = size.height as i32;
            
            return x >= win_x && x <= win_x + win_w && y >= win_y && y <= win_y + win_h;
        }
    }
    false
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
                    crate::utils::recording_state::set(&app_state, &app_handle, false);
                }
                EventType::ButtonPress(btn) => {
                    // 获取鼠标位置
                    if let Some((x, y)) = get_mouse_pos() {
                        // 检查是否在主窗口内
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if is_point_in_window(&window, x, y) {
                                log::info!("录音状态下检测到窗口内鼠标点击 {:?}，不取消录音", btn);
                                return;
                            }
                        }
                    }
                    log::info!("录音状态下检测到窗口外鼠标点击 {:?}，取消录音", btn);
                    crate::utils::recording_state::set(&app_state, &app_handle, false);
                }
                _ => {}
            }
        }) {
            log::error!("全局输入监听错误: {:?}", e);
        }
    });
}

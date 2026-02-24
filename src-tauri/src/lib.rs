mod asr;
mod commands;
mod models;
mod tray;
mod utils;
mod workflow;

use models::buffer::{AudioBuffer, TextBuffer};
use models::config::AppConfig;
use models::state::AppState;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::window::show_window(app.clone());
                        let state = app.state::<AppState>();
                        let mut is_recording = state.is_recording.lock().unwrap();
                        *is_recording = !*is_recording;
                        let new_state = *is_recording;
                        drop(is_recording);
                        
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("recording-state-changed", new_state);
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::window::show_window,
            commands::recording::hide_and_stop_recording,
            commands::window::quit_app,
            commands::recording::get_recording_state,
            commands::recording::set_recording,
            commands::settings::open_settings,
            commands::settings::close_settings_window,
            commands::settings::get_config,
            commands::settings::sync_config,
            commands::settings::test_asr_config,
        ])
        .setup(|app| {
            // 创建共享的资源
            let config = Arc::new(Mutex::new(AppConfig::default()));
            let audio_buffer = Arc::new(AudioBuffer::new());
            let text_buffer = Arc::new(TextBuffer::new());
            
            // 初始化 ASR 管理器（被动模式，无监控线程）
            let asr_manager = asr::init_asr_manager(
                audio_buffer.clone(),
                text_buffer.clone(),
                config.clone(),
            );
            
            // 创建并管理 AppState
            let state = AppState::new(
                asr_manager,
                audio_buffer,
                text_buffer,
                config,
            );
            app.manage(state);
            
            // 初始化配置
            let state = app.state::<AppState>();
            if let Err(e) = state.init_config(&app.handle()) {
                log::error!("初始化配置失败: {}", e);
            }
            
            // 读取快捷键配置
            let shortcut_str = {
                let config = state.config.lock().unwrap();
                config.shortcut.clone()
            };
            
            // 克隆 AppState 用于其他模块
            let state_clone = Arc::new(state.inner().clone());
            
            // 注册快捷键，失败时使用默认快捷键
            if let Err(e) = utils::shortcut::init_shortcut(app, &shortcut_str) {
                log::warn!("注册快捷键 '{}' 失败: {}, 使用默认快捷键 Shift+E", shortcut_str, e);
                utils::shortcut::init_shortcut(app, "Shift+E")
                    .map_err(|e| format!("注册默认快捷键失败: {}", e))?;
            }
            tray::setup_tray(app)?;

            let app_handle = app.handle().clone();
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            if handle.get_webview_window("settings").is_none() {
                                if let Some(main) = handle.get_webview_window("main") {
                                    if let Ok(false) = main.is_focused() {
                                        commands::recording::hide_and_stop_recording(handle);
                                    }
                                }
                            }
                        });
                    }
                });
            }

            // 启动录音监控线程
            workflow::recorder::init_recorder(state_clone.clone());
            
            // 启动输入模拟器（从 TextBuffer 读取）
            workflow::input_simulator::init_input_simulator(state_clone.clone());
            
            log::info!("ASR 管理器已就绪，等待 VAD 模块调用");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

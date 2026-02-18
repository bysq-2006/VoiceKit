mod commands;
mod models;
mod tray;
mod utils;
mod workflow;

use models::state::AppState;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    
    tauri::Builder::default()
        .manage(AppState::new())
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
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            if let Err(e) = state.init_config(&app.handle()) {
                log::error!("初始化配置失败: {}", e);
            }
            
            let config = state.config.lock().unwrap();
            let shortcut_str = config.shortcut.clone();
            drop(config);
            
            utils::shortcut::init_shortcut(app, &shortcut_str)?;
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

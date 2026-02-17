mod commands;
mod models;
mod tray;
mod utils;  // 工具函数模块

use models::state::AppState;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())  // 添加 store 插件
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))  // 开机自启插件
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        // 显示窗口
                        commands::window::show_window(app.clone());
                        // 同时切换录音状态（仅显示窗口时使用）
                        commands::recording::toggle_recording(app.clone());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::window::show_window,
            commands::recording::hide_and_stop_recording,
            commands::window::quit_app,
            commands::recording::get_recording_state,
            commands::recording::toggle_recording,
            commands::settings::open_settings,
            commands::settings::close_settings_window,
            commands::settings::get_config,      // 新增：获取配置
            commands::settings::sync_config,     // 新增：同步配置
            commands::settings::open_link,       // 新增：打开链接
        ])
        .setup(|app| {
            // 初始化配置（从 store 加载）
            let state = app.state::<AppState>();
            state.init_config(&app.handle())?;
            
            // 获取加载后的配置来初始化快捷键
            let config = state.config.lock().unwrap();
            let shortcut_str = config.shortcut.clone();
            drop(config);  // 释放锁
            
            // 注册快捷键
            utils::shortcut::init_shortcut(app, &shortcut_str)?;

            // 设置系统托盘
            tray::setup_tray(app)?;

            // 主窗口失去焦点时自动隐藏（延迟判断，避免拖动时误判）
            let app_handle = app.handle().clone();
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        // 延迟 200ms 检查，拖动窗口会快速重新获得焦点
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            // 再次确认：设置窗口没打开 且 主窗口确实没焦点 才隐藏
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

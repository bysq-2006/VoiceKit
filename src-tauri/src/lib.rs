mod commands;
mod state;
mod tray;

use state::AppState;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState, GlobalShortcutExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
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
            commands::settings::open_settings
        ])
        .setup(|app| {
            // 注册 Shift+E 快捷键
            let shortcut = tauri_plugin_global_shortcut::Shortcut::new(
                Some(Modifiers::SHIFT),
                Code::KeyE,
            );
            app.global_shortcut().register(shortcut)?;

            // 设置系统托盘
            tray::setup_tray(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

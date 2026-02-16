mod commands;
mod store;
mod tray;

use store::state::AppState;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState, GlobalShortcutExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())  // 添加 store 插件
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
            commands::settings::get_config,      // 新增：获取配置
            commands::settings::sync_config,     // 新增：同步配置
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
            let shortcut = parse_shortcut(&shortcut_str)?;
            app.global_shortcut().register(shortcut)?;

            // 设置系统托盘
            tray::setup_tray(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 将字符串解析为快捷键（简单实现）
fn parse_shortcut(s: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    // 简单解析 "Shift+E" 格式
    let parts: Vec<&str> = s.split('+').collect();
    if parts.len() != 2 {
        // 默认返回 Shift+E
        return Ok(tauri_plugin_global_shortcut::Shortcut::new(
            Some(Modifiers::SHIFT),
            Code::KeyE,
        ));
    }
    
    let modifier = match parts[0].trim() {
        "Shift" => Some(Modifiers::SHIFT),
        "Ctrl" | "Control" => Some(Modifiers::CONTROL),
        "Alt" => Some(Modifiers::ALT),
        "Cmd" | "Command" => Some(Modifiers::META),
        _ => None,
    };
    
    let code = match parts[1].trim() {
        "E" => Code::KeyE,
        "R" => Code::KeyR,
        "Space" => Code::Space,
        _ => Code::KeyE,
    };
    
    Ok(tauri_plugin_global_shortcut::Shortcut::new(modifier, code))
}

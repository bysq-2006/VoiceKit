mod asr;
mod commands;
mod models;
mod tray;
mod utils;
mod workflow;

use models::buffer::{AudioBuffer, TextBuffer};
use models::config::AppConfig;
use models::state::AppState;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::ShortcutState;
use std::sync::{Arc, Mutex};

const DEFAULT_SHORTCUT: &str = "Shift+E";

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
                        // 根据当前主题设置窗口尺寸
                        let state = app.state::<AppState>();
                        let (width, height) = match state.config.lock().unwrap().theme {
                            models::config::Theme::Default => (Some(240), Some(150)),
                            models::config::Theme::Google => (Some(400), Some(100)),
                        };
                        commands::theme::show_window(app.clone(), width, height);
                        crate::utils::recording_state::toggle(&state, &app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::theme::show_window,
            commands::theme::hide_window,
            commands::theme::hide_and_stop_recording,
            commands::theme::quit_app,
            commands::theme::get_recording_state,
            commands::theme::set_recording,
            commands::settings::open_settings,
            commands::settings::close_settings_window,
            commands::settings::get_config,
            commands::settings::sync_config,
            commands::asr_test::test_asr_config,
        ])
        .setup(|app| {
            let config = Arc::new(Mutex::new(AppConfig::default()));
            let audio_buffer = Arc::new(AudioBuffer::new());
            let text_buffer = Arc::new(TextBuffer::new());
            
            let asr_manager = asr::init_asr_manager(
                audio_buffer.clone(),
                text_buffer.clone(),
                config.clone(),
            );
            
            let state = AppState::new(asr_manager, audio_buffer, text_buffer, config);
            app.manage(state);
            
            let state: tauri::State<AppState> = app.state();
            if let Err(e) = state.init_config(&app.handle()) {
                log::error!("初始化配置失败: {}", e);
            }
            
            // 初始化自启动
            let auto_start = state.config.lock().unwrap().auto_start;
            let manager = app.autolaunch();
            if let Err(e) = if auto_start { manager.enable() } else { manager.disable() } {
                log::error!("初始化自启动失败: {}", e);
            }
            
            // 注册快捷键（失败尝试默认）
            let shortcut = state.config.lock().unwrap().shortcut.clone();
            for (i, sc) in [shortcut.as_str(), DEFAULT_SHORTCUT].iter().enumerate() {
                if utils::shortcut::init_shortcut(app, sc).is_ok() { break; }
                if i == 1 { log::error!("快捷键注册失败，应用继续运行"); }
            }
            
            // 克隆 state 用于后续使用
            let state_clone = Arc::new(state.inner().clone());

            tray::setup_tray(app)?;
            workflow::recorder::init_recorder(state_clone.clone());
            workflow::input_simulator::init_input_simulator(state_clone.clone());
            workflow::asr_controller::init_asr_controller(state_clone.clone());
            workflow::global_input::init(state_clone, app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

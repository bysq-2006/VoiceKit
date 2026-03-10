use tauri::{AppHandle, Manager, WebviewWindow, Emitter};
use tauri_plugin_autostart::ManagerExt;
use crate::models::{state::AppState, config::AppConfig};

const LABEL: &str = "settings";
const URL: &str = "/src/settings.html";

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }

    const W: f64 = 320.0;
    const H: f64 = 400.0;
    const BOTTOM_GAP: f64 = 60.0; // 距离底部留出托盘区域
    const RIGHT_GAP: f64 = 12.0;  // 距离右边距

    // 获取屏幕尺寸，默认居中
    let (x, y) = app.primary_monitor()
        .map(|m| m.map(|m| {
            let screen_w = m.size().width as f64 / m.scale_factor();
            let screen_h = m.size().height as f64 / m.scale_factor();
            let x = screen_w - W - RIGHT_GAP;
            let y = screen_h - H - BOTTOM_GAP;
            (x, y)
        }))
        .ok()
        .flatten()
        .unwrap_or((0.0, 0.0));

    let w = WebviewWindow::builder(&app, LABEL, tauri::WebviewUrl::App(URL.into()))
        .title("设置")
        .inner_size(W, H)
        .position(x, y)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .decorations(false)
        .transparent(true)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn close_settings_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}

#[tauri::command]
pub fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn sync_config(
    app: AppHandle,
    state: tauri::State<AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    let old = state.config.lock().unwrap().clone();
    state.update_config(&app, new_config.clone())?;
    
    // 处理快捷键变化
    if old.shortcut != new_config.shortcut {
        let _ = crate::utils::shortcut::update_shortcut(&app, &new_config.shortcut);
    }
    
    // 处理开机自启动变化
    if old.auto_start != new_config.auto_start {
        let autostart_manager = app.autolaunch();
        if new_config.auto_start {
            autostart_manager.enable().map_err(|e| format!("启用开机自启动失败: {}", e))?;
        } else {
            autostart_manager.disable().map_err(|e| format!("禁用开机自启动失败: {}", e))?;
        }
    }
    
    // 广播配置更新事件给所有窗口，通知所有 useConfig 实例同步
    let _ = app.emit("config-updated", new_config);
    
    Ok(())
}

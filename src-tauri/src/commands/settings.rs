use tauri::{AppHandle, Manager, WebviewWindow};
use crate::models::{state::AppState, config::AppConfig};

const LABEL: &str = "settings";
const URL: &str = "/src/settings.html";

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }

    let main = app.get_webview_window("main").ok_or("主窗口不存在")?;
    
    let pos = main.outer_position().map_err(|e| e.to_string())?;
    let size = main.outer_size().map_err(|e| e.to_string())?;
    
    let settings_w = 320.0;
    let settings_h = 400.0;  // 固定高度，足够显示所有内容
    let gap = 8.0;
    
    let x = pos.x as f64;
    let y = pos.y as f64 + size.height as f64 + gap;

    // 在屏幕外创建窗口
    let w = WebviewWindow::builder(&app, LABEL, tauri::WebviewUrl::App(URL.into()))
        .title("设置")
        .inner_size(settings_w, settings_h)
        .position(-10000.0, -10000.0)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .decorations(false)
        .transparent(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // 移动到正确位置
    let _ = w.set_position(tauri::Position::Physical(tauri::PhysicalPosition { 
        x: x as i32, 
        y: y as i32 
    }));
    
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // 最后显示
    let _ = w.show();
    let _ = w.set_focus();
    
    let app_handle = app.clone();
    w.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            if let Some(w) = app_handle.get_webview_window(LABEL) {
                let _ = w.close();
            }
        }
    });
    
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
    let old = state.config.lock().unwrap().shortcut.clone();
    state.update_config(&app, new_config.clone())?;
    
    if old != new_config.shortcut {
        let _ = crate::utils::shortcut::update_shortcut(&app, &new_config.shortcut);
    }
    Ok(())
}

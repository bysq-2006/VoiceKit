use tauri::{AppHandle, Manager, WebviewWindow};
use crate::models::{state::AppState, config::AppConfig};
use crate::asr::factory::ASRFactory;

const LABEL: &str = "settings";
const URL: &str = "/src/settings.html";

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }

    // 获取主窗口位置和大小（使用 inner 避免装饰边框的影响）
    let main = app.get_webview_window("main").ok_or("主窗口不存在")?;
    let pos = main.inner_position().map_err(|e| e.to_string())?;
    let size = main.inner_size().map_err(|e| e.to_string())?;
    
    // 计算设置窗口位置：主窗口正下方居中
    let settings_w = 480.0;
    let gap = 8.0;
    
    let x = pos.x as f64 + (size.width as f64 - settings_w) / 2.0;
    let y = pos.y as f64 + size.height as f64 + gap;

    let w = WebviewWindow::builder(&app, LABEL, tauri::WebviewUrl::App(URL.into()))
        .title("设置")
        .inner_size(settings_w, 480.0)
        .position(x, y)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .decorations(false)
        .transparent(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let _ = w.show();
    let _ = w.set_focus();
    
    // 设置窗口失去焦点时自动关闭
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

#[tauri::command]
pub async fn test_asr_config(config: crate::asr::factory::ASRConfig) -> Result<(), String> {
    ASRFactory::test_config(&config).await.map_err(|e| e.to_string())
}



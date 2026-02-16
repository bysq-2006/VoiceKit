use tauri::Manager;

/// 打开设置窗口
#[tauri::command]
pub async fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    // 如果窗口已存在，直接显示
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 创建新窗口
    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("src/settings.html".into()),
    )
    .title("设置")
    .inner_size(500.0, 400.0)
    .min_inner_size(500.0, 400.0)
    .max_inner_size(500.0, 400.0)
    .center()
    .decorations(true)
    .resizable(false)
    .build()
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

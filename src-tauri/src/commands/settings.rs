use tauri::Manager;
use crate::store::config::AppConfig;

/// 获取完整配置（前端调用，返回 JSON）
#[tauri::command]
pub fn get_config(state: tauri::State<crate::store::state::AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().unwrap().clone();
    Ok(config)
}

/// 同步配置（前端传入 JSON，后端保存并应用）
#[tauri::command]
pub async fn sync_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
    config: AppConfig,  // 前端传入的 JSON 自动反序列化为 AppConfig
) -> Result<(), String> {
    // 1. 保存到 store
    config.save(&app)?;
    
    // 2. 更新内存中的配置
    *state.config.lock().unwrap() = config.clone();
    
    // 3. 应用配置（如重新注册快捷键等）
    apply_config(&app, &config).await?;
    
    Ok(())
}

/// 应用配置（内部函数）
async fn apply_config(_app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    // TODO: 根据新配置应用更改
    // 例如：重新注册快捷键、更新主题、设置自启动等
    
    // 示例：重新注册快捷键
    // crate::shortcut::register(app, &config.shortcut)?;
    
    // 示例：设置自启动
    // app.autolaunch().set_auto_launch(config.auto_start).map_err(|e| e.to_string())?;
    
    println!("配置已应用: shortcut={}, theme={:?}", config.shortcut, config.theme);
    
    Ok(())
}

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

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use crate::models::config::AppConfig;

/// 获取完整配置（前端调用，返回 JSON）
#[tauri::command]
pub fn get_config(state: tauri::State<crate::models::state::AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().unwrap().clone();
    Ok(config)
}

/// 同步配置（前端传入 JSON，后端保存并应用）
#[tauri::command]
pub async fn sync_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::models::state::AppState>,
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

/// 打开外部链接
#[tauri::command]
pub async fn open_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// 应用配置（内部函数）
async fn apply_config(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    
    // 1. 重新注册快捷键（实时生效）
    crate::utils::shortcut::update_shortcut(&app.clone(), &config.shortcut)?;
    
    // 2. 设置开机自启
    let autostart_manager = app.autolaunch();
    if config.auto_start {
        autostart_manager.enable().map_err(|e| format!("启用自启动失败: {}", e))?;
    } else {
        autostart_manager.disable().map_err(|e| format!("禁用自启动失败: {}", e))?;
    }
    
    println!("配置已应用: shortcut={}, auto_start={}", config.shortcut, config.auto_start);
    
    Ok(())
}

/// 关闭设置窗口
#[tauri::command]
pub fn close_settings_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _: Result<(), _> = window.close();
    }
}

/// 打开设置弹窗（定位到主窗口旁边，失去焦点关闭）
#[tauri::command]
pub async fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    // 如果窗口已存在，先关闭
    if let Some(window) = app.get_webview_window("settings") {
        let _: Result<(), _> = window.close();
    }

    // 获取主窗口位置和缩放因子
    let main_window = app.get_webview_window("main")
        .ok_or("主窗口未找到")?;
    
    // 获取主窗口相对于屏幕的位置（物理像素）
    let main_pos = main_window.outer_position()
        .map_err(|e| e.to_string())?;
    
    // 获取主窗口内部尺寸（逻辑像素）
    let main_inner_size = main_window.inner_size()
        .map_err(|e| e.to_string())?;
    
    // 获取缩放因子
    let scale_factor = main_window.scale_factor()
        .unwrap_or(1.0);

    // 设置弹窗尺寸（逻辑像素）
    let popup_width = 280.0;
    let popup_height = 320.0;

    // 计算位置：转换为逻辑坐标
    // outer_position 是物理坐标，需要除以 scale_factor
    let x = (main_pos.x as f64) / scale_factor;
    // 减去 8px 偏移，让弹窗更贴近主窗口
    let y = (main_pos.y as f64) / scale_factor + main_inner_size.height as f64 - 48.0;

    // 创建弹窗窗口
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("src/settings.html".into()),
    )
    .title("设置")
    .inner_size(popup_width, popup_height)
    .position(x, y)
    .decorations(false)      // 无边框
    .resizable(false)        // 不可调整大小
    .always_on_top(true)     // 保持在最前
    .skip_taskbar(true)      // 不显示在任务栏
    .build()
    .map_err(|e| e.to_string())?;

    // 监听失去焦点事件，自动关闭
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            // 失去焦点时关闭窗口
            if let Some(win) = app_handle.get_webview_window("settings") {
                let _: Result<(), _> = win.close();
            }
        }
    });

    Ok(())
}

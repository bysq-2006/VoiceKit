use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use crate::models::{state::AppState, config::{AppConfig, AsrConfig, XunfeiConfig, DoubaoConfig}};

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
    
    Ok(())
}

/// 测试 ASR 配置
/// 尝试建立 WebSocket 连接来验证配置是否正确
#[tauri::command]
pub async fn test_asr_config(config: AsrConfig) -> Result<(), String> {
    match config.provider.as_str() {
        "xunfei" => test_xunfei_config(&config.xunfei).await,
        "doubao" => test_doubao_config(&config.doubao).await,
        _ => Err(format!("未知的 ASR 提供商: {}", config.provider)),
    }
}

/// 测试讯飞 ASR 配置
async fn test_xunfei_config(config: &XunfeiConfig) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use tokio_tungstenite::connect_async;

    type HmacSha256 = Hmac<Sha256>;

    const XFYUN_HOST: &str = "iat.cn-huabei-1.xf-yun.com";
    const XFYUN_WS_URL: &str = "wss://iat.cn-huabei-1.xf-yun.com/v1";

    let app_id = config.app_id.as_ref().ok_or("请提供 App ID")?;
    let api_key = config.api_key.as_ref().ok_or("请提供 API Key")?;
    let api_secret = config.api_secret.as_ref().ok_or("请提供 API Secret")?;

    // 生成鉴权 URL
    let date = httpdate::fmt_http_date(std::time::SystemTime::now());
    let signature_origin = format!("host: {}\ndate: {}\nGET /v1 HTTP/1.1", XFYUN_HOST, date);

    let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())
        .map_err(|e| format!("HMAC 错误: {}", e))?;
    mac.update(signature_origin.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());

    let authorization_origin = format!(
        "api_key=\"{}\", algorithm=\"hmac-sha256\", headers=\"host date request-line\", signature=\"{}\"",
        api_key, signature
    );
    let authorization = BASE64.encode(authorization_origin.as_bytes());

    let url = format!(
        "{}?authorization={}&date={}&host={}",
        XFYUN_WS_URL,
        urlencoding::encode(&authorization),
        urlencoding::encode(&date),
        XFYUN_HOST
    );

    // 尝试建立连接
    match tokio::time::timeout(std::time::Duration::from_secs(5), connect_async(&url)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("连接失败: {}", e)),
        Err(_) => Err("连接超时".to_string()),
    }
}

/// 测试豆包 ASR 配置
async fn test_doubao_config(config: &DoubaoConfig) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use tokio_tungstenite::connect_async;

    const DOUBAO_WS_URL: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async";
    const RESOURCE_ID: &str = "volc.seedasr.sauc.concurrent";

    let app_id = config.app_id.as_ref().ok_or("请提供 App ID")?;
    let api_key = config.api_key.as_ref().ok_or("请提供 API Key")?;
    let connect_id = uuid::Uuid::new_v4().to_string();
    
    // 生成 WebSocket Key (RFC 6455)
    let mut ws_key_bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut ws_key_bytes);
    let ws_key = BASE64.encode(&ws_key_bytes);

    // 构建带鉴权 Header 的 WebSocket 请求
    let request = http::Request::builder()
        .method("GET")
        .uri(DOUBAO_WS_URL)
        .header("Host", "openspeech.bytedance.com")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Key", ws_key)
        .header("Sec-WebSocket-Version", "13")
        .header("X-Api-App-Key", app_id.clone())
        .header("X-Api-Access-Key", api_key.clone())
        .header("X-Api-Resource-Id", RESOURCE_ID)
        .header("X-Api-Connect-Id", connect_id)
        .body(())
        .map_err(|e| format!("构建请求失败: {}", e))?;

    // 尝试建立连接
    match tokio::time::timeout(std::time::Duration::from_secs(5), connect_async(request)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("连接失败: {}", e)),
        Err(_) => Err("连接超时".to_string()),
    }
}

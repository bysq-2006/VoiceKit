use crate::models::config::{AsrConfig, DoubaoConfig, FunasrConfig, XunfeiConfig};

/// 测试 ASR 配置
/// 尝试建立 WebSocket 连接来验证配置是否正确
#[tauri::command]
pub async fn test_asr_config(config: AsrConfig) -> Result<(), String> {
    match config.provider.as_str() {
        "xunfei" => test_xunfei_config(&config.xunfei).await,
        "doubao" => test_doubao_config(&config.doubao).await,
        "funasr" => test_funasr_config(&config.funasr).await,
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

/// 测试本地 FunASR 配置
async fn test_funasr_config(config: &FunasrConfig) -> Result<(), String> {
    use futures::SinkExt;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let host = config.host.trim();
    if host.is_empty() {
        return Err("请提供 Host".to_string());
    }
    if config.port == 0 {
        return Err("请提供有效的 Port".to_string());
    }

    let ws_url = format!("ws://{}:{}/ws/asr", host, config.port);

    match tokio::time::timeout(std::time::Duration::from_secs(5), connect_async(&ws_url)).await {
        Ok(Ok((mut ws, _))) => {
            // 握手成功即视为可达；发送 finish 让服务端主动结束会话
            let _ = ws
                .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                .await;
            let _ = ws.close(None).await;
            Ok(())
        }
        Ok(Err(e)) => Err(format!("连接失败: {}", e)),
        Err(_) => Err("连接超时".to_string()),
    }
}

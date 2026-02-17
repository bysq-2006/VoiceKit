mod protocol;
mod streaming;

use async_trait::async_trait;
use crate::asr::{ASRProvider, ASRResult, ASRError, StreamingASRSession};
use crate::asr::doubao::protocol::{DoubaoProtocol, RequestPayload};
use crate::asr::doubao::streaming::DoubaoStreamingSession;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

/// 豆包 ASR 配置
#[derive(Debug, Clone)]
pub struct DoubaoConfig {
    /// App ID（火山引擎控制台获取）
    pub app_id: String,
    /// Access Token（火山引擎控制台获取）
    pub access_key: String,
    /// 资源 ID
    /// - 小时版: volc.bigasr.sauc.duration 或 volc.seedasr.sauc.duration
    /// - 并发版: volc.bigasr.sauc.concurrent 或 volc.seedasr.sauc.concurrent
    pub resource_id: String,
    /// WebSocket 接口地址
    pub ws_url: String,
}

impl Default for DoubaoConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            access_key: String::new(),
            resource_id: "volc.seedasr.sauc.concurrent".to_string(),
            // 默认使用双向流式优化版
            ws_url: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async".to_string(),
        }
    }
}

/// 豆包 ASR Provider
pub struct DoubaoASR {
    config: DoubaoConfig,
    protocol: DoubaoProtocol,
}

impl DoubaoASR {
    pub fn new(config: DoubaoConfig) -> Self {
        let protocol = DoubaoProtocol::new();
        Self { config, protocol }
    }
    
    /// 构建 WebSocket 连接请求（添加鉴权 Header）
    fn build_request(&self) -> ASRResult<tokio_tungstenite::tungstenite::http::Request<()>> {
        let mut request = self.config.ws_url
            .clone()
            .into_client_request()
            .map_err(|e| ASRError::Network(format!("无效URL: {}", e)))?;
        
        let headers = request.headers_mut();
        
        // 添加鉴权 Header
        headers.insert(
            "X-Api-App-Key",
            HeaderValue::from_str(&self.config.app_id)
                .map_err(|e| ASRError::Config(format!("无效AppId: {}", e)))?,
        );
        headers.insert(
            "X-Api-Access-Key",
            HeaderValue::from_str(&self.config.access_key)
                .map_err(|e| ASRError::Config(format!("无效AccessKey: {}", e)))?,
        );
        headers.insert(
            "X-Api-Resource-Id",
            HeaderValue::from_str(&self.config.resource_id)
                .map_err(|e| ASRError::Config(format!("无效ResourceId: {}", e)))?,
        );
        headers.insert(
            "X-Api-Connect-Id",
            HeaderValue::from_str(&uuid::Uuid::new_v4().to_string())
                .map_err(|e| ASRError::Config(format!("生成UUID失败: {}", e)))?,
        );
        
        Ok(request)
    }
    
    /// 构建初始化请求 payload
    fn build_init_payload(&self) -> RequestPayload {
        RequestPayload {
            user: None,
            audio: Some(protocol::AudioConfig {
                format: "pcm".to_string(),
                codec: Some("raw".to_string()),
                rate: Some(16000),
                bits: Some(16),
                channel: Some(1),
                language: None,
            }),
            request: Some(protocol::RequestConfig {
                model_name: "bigmodel".to_string(),
                enable_nonstream: Some(false),
                enable_itn: Some(true),
                enable_punc: Some(true),
                enable_ddc: Some(false),
                show_utterances: Some(false),
                ..Default::default()
            }),
        }
    }
}

#[async_trait]
impl ASRProvider for DoubaoASR {
    fn name(&self) -> &str {
        "doubao"
    }
    
    async fn health_check(&self) -> ASRResult<()> {
        // 尝试建立连接然后立即关闭
        let request = self.build_request()?;
        let (ws_stream, response) = connect_async(request)
            .await
            .map_err(|e| ASRError::Network(format!("连接失败: {}", e)))?;
        
        // 检查响应头
        if let Some(log_id) = response.headers().get("X-Tt-Logid") {
            log::info!("豆包ASR健康检查成功, log_id: {:?}", log_id);
        }
        
        // 立即关闭连接
        drop(ws_stream);
        
        Ok(())
    }
    
    async fn recognize(&self, audio_data: Vec<u8>) -> ASRResult<String> {
        // 使用流式识别，收集最终结果
        let mut session = self.start_streaming().await?;
        
        // 分包发送音频（每包 200ms = 6400 bytes @ 16kHz/16bit）
        const CHUNK_SIZE: usize = 6400;
        let mut sequence: i32 = 1;
        
        for chunk in audio_data.chunks(CHUNK_SIZE) {
            session.send_audio(chunk.to_vec(), sequence).await?;
            sequence += 1;
            
            // 小延迟避免发送过快
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        // 发送结束包
        session.finish().await?;
        
        // 收集所有结果
        let mut final_text = String::new();
        let mut last_text = String::new();
        
        loop {
            match session.receive_result().await? {
                Some(result) => {
                    if result.is_final {
                        final_text = result.text;
                        break;
                    } else {
                        // 中间结果，继续等待
                        last_text = result.text;
                    }
                }
                None => {
                    // 识别结束，使用最后收到的文本
                    if final_text.is_empty() {
                        final_text = last_text;
                    }
                    break;
                }
            }
        }
        
        if final_text.is_empty() {
            return Err(ASRError::RecognitionFailed("未能识别出任何内容".to_string()));
        }
        
        Ok(final_text)
    }
    
    async fn start_streaming(&self) -> ASRResult<Box<dyn StreamingASRSession>> {
        let request = self.build_request()?;
        
        let (ws_stream, response) = connect_async(request)
            .await
            .map_err(|e| ASRError::Network(format!("WebSocket连接失败: {}", e)))?;
        
        // 记录 log_id 用于排错
        if let Some(log_id) = response.headers().get("X-Tt-Logid") {
            log::info!("豆包ASR连接成功, log_id: {:?}", log_id);
        }
        
        let init_payload = self.build_init_payload();
        
        let session = DoubaoStreamingSession::new(
            ws_stream,
            self.protocol.clone(),
            init_payload,
        ).await?;
        
        Ok(Box::new(session))
    }
}

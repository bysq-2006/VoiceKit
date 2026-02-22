use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::XunfeiConfig;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;

type HmacSha256 = Hmac<Sha256>;
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

const XFYUN_WS_URL: &str = "wss://iat.cn-huabei-1.xf-yun.com/v1";
const XFYUN_HOST: &str = "iat.cn-huabei-1.xf-yun.com";

/// 讯飞 ASR 提供商
#[derive(Clone)]
pub struct XunfeiAsr {
    app_id: String,
    api_key: String,
    api_secret: String,
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    ws_sink: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Message>>>>,
    status: Arc<AtomicU8>,
    is_connected: Arc<AtomicBool>,
}

// 请求数据结构
#[derive(Serialize)]
struct RequestData {
    header: RequestHeader,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameter: Option<serde_json::Value>,
    payload: Payload,
}

#[derive(Serialize)]
struct RequestHeader {
    status: u8,
    app_id: String,
}

#[derive(Serialize)]
struct Payload {
    #[serde(skip_serializing_if = "Option::is_none")]
    audio: Option<AudioPayload>,
}

#[derive(Serialize)]
struct AudioPayload {
    audio: String,
    sample_rate: u16,
    encoding: String,
}

// 响应数据结构
#[derive(Deserialize)]
struct ResponseData {
    header: ResponseHeader,
    #[serde(default)]
    payload: Option<ResponsePayload>,
}

#[derive(Deserialize)]
struct ResponseHeader {
    code: i32,
    #[serde(default)]
    message: String,
    status: u8,
}

#[derive(Deserialize)]
struct ResponsePayload {
    #[serde(default)]
    result: Option<ResultData>,
}

#[derive(Deserialize)]
struct ResultData {
    text: String,
}

// 词元结构
#[derive(Deserialize)]
struct ResultText {
    ws: Vec<WordSlice>,
}

#[derive(Deserialize)]
struct WordSlice {
    cw: Vec<WordCell>,
}

#[derive(Deserialize)]
struct WordCell {
    w: String,
}

impl XunfeiAsr {
    pub fn new(
        config: XunfeiConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Result<Self, String> {
        let app_id = config
            .app_id
            .clone()
            .ok_or("讯飞 ASR 需要 app_id")?;
        let api_key = config
            .api_key
            .clone()
            .ok_or("讯飞 ASR 需要 api_key")?;
        let api_secret = config
            .api_secret
            .clone()
            .ok_or("讯飞 ASR 需要 api_secret")?;

        Ok(Self {
            app_id,
            api_key,
            api_secret,
            audio_buffer,
            text_buffer,
            ws_sink: Arc::new(Mutex::new(None)),
            status: Arc::new(AtomicU8::new(0)),
            is_connected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 生成鉴权 URL
    fn create_url(&self) -> String {
        let date = httpdate::fmt_http_date(std::time::SystemTime::now());
        let signature_origin = format!("host: {}\ndate: {}\nGET /v1 HTTP/1.1", XFYUN_HOST, date);

        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes()).expect("HMAC key creation");
        mac.update(signature_origin.as_bytes());
        let signature = BASE64.encode(mac.finalize().into_bytes());

        let authorization_origin = format!(
            "api_key=\"{}\", algorithm=\"hmac-sha256\", headers=\"host date request-line\", signature=\"{}\"",
            self.api_key, signature
        );
        let authorization = BASE64.encode(authorization_origin.as_bytes());

        format!(
            "{}?authorization={}&date={}&host={}",
            XFYUN_WS_URL,
            urlencoding::encode(&authorization),
            urlencoding::encode(&date),
            XFYUN_HOST
        )
    }

    /// 构建帧数据 (status: 0=首帧, 1=中间帧, 2=结束帧)
    fn build_frame(&self, status: u8, audio_b64: String) -> String {
        if status == 0 {
            log::debug!("讯飞 ASR 第一帧音频数据长度: {} bytes", audio_b64.len() * 3 / 4);
        }

        let parameter = if status == 0 {
            Some(serde_json::json!({
                "iat": {
                    "domain": "slm",
                    "language": "mul_cn",
                    "accent": "mandarin",
                    "result": { "encoding": "utf8", "compress": "raw", "format": "json" }
                }
            }))
        } else {
            None
        };

        serde_json::to_string(&RequestData {
            header: RequestHeader { status, app_id: self.app_id.clone() },
            parameter,
            payload: Payload {
                audio: Some(AudioPayload {
                    audio: audio_b64,
                    sample_rate: 16000,
                    encoding: "raw".to_string(),
                }),
            },
        }).unwrap()
    }

    /// 解析识别结果
    fn parse_result(text: &str) -> Option<String> {
        let decoded = BASE64.decode(text).ok()?;
        let json_str = String::from_utf8(decoded).ok()?;
        let result: ResultText = serde_json::from_str(&json_str).ok()?;
        
        let text: String = result.ws.iter()
            .flat_map(|s| &s.cw)
            .map(|c| c.w.as_str())
            .collect();
        
        if text.is_empty() { None } else { Some(text) }
    }

    /// 启动 WebSocket 监听
    async fn start_listening(&self, mut ws_stream: futures::stream::SplitStream<WsStream>) {
        let text_buffer = self.text_buffer.clone();
        let status = self.status.clone();
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            let mut last_sent = String::new();

            while let Some(Ok(msg)) = ws_stream.next().await {
                match msg {
                    Message::Text(text) => {
                        let Ok(data) = serde_json::from_str::<ResponseData>(&text) else {
                            log::error!("讯飞 ASR 消息解析失败");
                            continue;
                        };

                        if data.header.code != 0 {
                            log::error!("讯飞 ASR 服务端错误: code={}, message={}", 
                                data.header.code, data.header.message);
                            is_connected.store(false, Ordering::SeqCst);
                            break;
                        }

                        // 解析并实时写入增量
                        if let Some(text) = data.payload.and_then(|p| p.result)
                            .and_then(|r| Self::parse_result(&r.text)) {
                            if text.len() > last_sent.len() {
                                text_buffer.write(text[last_sent.len()..].to_string());
                                last_sent = text;
                            }
                        }

                        if data.header.status == 2 {
                            log::info!("讯飞 ASR 会话结束, 结果: {}", last_sent);
                            is_connected.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                    Message::Close(frame) => {
                        if let Some(f) = frame {
                            log::info!("讯飞 ASR 连接关闭: code={}, reason={}", f.code, f.reason);
                        }
                        is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                    _ => {}
                }
            }

            status.store(0, Ordering::SeqCst);
            log::info!("讯飞 ASR 监听任务结束");
        });
    }
}

#[async_trait]
impl crate::asr::provider::AsrProvider for XunfeiAsr {
    async fn start(&self) -> Result<(), String> {
        log::info!("讯飞 ASR 开始");

        let url = self.create_url();
        log::debug!("讯飞 ASR 连接 URL: {}", url);
        
        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| format!("WebSocket 连接失败: {}", e))?;

        let (mut ws_sink, ws_stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);
        
        log::info!("讯飞 ASR WebSocket 连接成功");

        self.start_listening(ws_stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let self_clone = self.clone();
        tokio::spawn(async move {
            let mut buf = vec![0i16; 3200];
            let mut frames = 0;
            let mut total_samples = 0;

            loop {
                if !self_clone.is_connected.load(Ordering::SeqCst) {
                    log::warn!("讯飞 ASR 连接已断开，停止发送");
                    break;
                }

                if let Ok(msg) = rx.try_recv() {
                    let _ = ws_sink.send(msg).await;
                }

                let count = self_clone.audio_buffer.read(&mut buf);
                if count == 0 {
                    if self_clone.audio_buffer.is_finished() { break; }
                    continue;
                }

                total_samples += count;
                let amp = buf[..count].iter().map(|s| s.abs()).max().unwrap_or(0);
                if amp < 100 {
                    log::debug!("讯飞 ASR 静音帧 #{} (amp={})", frames, amp);
                } else if frames == 0 {
                    log::info!("讯飞 ASR 第一帧振幅: {}", amp);
                }

                let bytes: Vec<u8> = buf[..count].iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect();
                let audio_b64 = BASE64.encode(&bytes);

                let status = self_clone.status.load(Ordering::SeqCst);
                let frame = if status == 0 {
                    log::info!("讯飞 ASR 发送第一帧");
                    self_clone.status.store(1, Ordering::SeqCst);
                    self_clone.build_frame(0, audio_b64)
                } else {
                    self_clone.build_frame(1, audio_b64)
                };

                if let Err(e) = ws_sink.send(Message::Text(frame)).await {
                    log::error!("讯飞 ASR 发送失败: {}", e);
                    self_clone.is_connected.store(false, Ordering::SeqCst);
                    break;
                }

                frames += 1;
                if frames % 10 == 0 {
                    log::debug!("讯飞 ASR 已发送 {} 帧", frames);
                }

                if self_clone.audio_buffer.is_finished() && self_clone.audio_buffer.is_empty() {
                    break;
                }
            }

            log::info!("讯飞 ASR 共发送 {} 帧, {} samples (约 {:.1} 秒)", 
                frames, total_samples, total_samples as f32 / 16000.0);

            if self_clone.is_connected.load(Ordering::SeqCst) {
                let _ = ws_sink.send(Message::Text(self_clone.build_frame(2, String::new()))).await;
                let _ = ws_sink.close().await;
            }
            
            log::info!("讯飞 ASR 发送任务结束");
        });

        Ok(())
    }

    async fn stop(&self) {
        log::info!("讯飞 ASR 停止");
        self.is_connected.store(false, Ordering::SeqCst);

        if let Some(sink) = self.ws_sink.lock().await.as_ref() {
            let _ = sink.send(Message::Text(self.build_frame(2, String::new()))).await;
        }
        *self.ws_sink.lock().await = None;
    }
}

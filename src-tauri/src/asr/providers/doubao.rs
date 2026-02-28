use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::DoubaoConfig;
use crate::utils::text_diff::compute_diff;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_URL: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async";
const RESOURCE_ID: &str = "volc.seedasr.sauc.duration";

#[derive(Clone)]
pub struct DoubaoAsr {
    app_id: String,
    api_key: String,
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    ws_sink: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Message>>>>,
    is_connected: Arc<AtomicBool>,
    /// 缓存上一次的识别结果，用于增量更新
    text_cache: Arc<Mutex<String>>,
}

#[derive(Serialize)]
struct RequestPayload {
    user: serde_json::Value,
    audio: serde_json::Value,
    request: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct ResponsePayload {
    #[serde(default)] code: i32,
    #[serde(default)] error: Option<String>,
    #[serde(default)] result: Option<ResultItem>,
}

#[derive(Deserialize, Debug)]
struct ResultItem {
    #[serde(default)] text: String,
    #[serde(default)] utterances: Vec<Utterance>,
}

#[derive(Deserialize, Debug)]
struct Utterance {
    text: String,
    #[serde(default)] definite: bool,
}

impl DoubaoAsr {
    pub fn new(
        config: DoubaoConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Result<Self, String> {
        Ok(Self {
            app_id: config.app_id.ok_or("需要 app_id")?,
            api_key: config.api_key.ok_or("需要 api_key")?,
            audio_buffer,
            text_buffer,
            ws_sink: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(AtomicBool::new(false)),
            text_cache: Arc::new(Mutex::new(String::new())),
        })
    }

    fn build_request(&self) -> http::Request<()> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let mut key_bytes = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key_bytes);
        
        http::Request::builder()
            .method("GET")
            .uri(WS_URL)
            .header("Host", "openspeech.bytedance.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Key", BASE64.encode(&key_bytes))
            .header("Sec-WebSocket-Version", "13")
            .header("X-Api-App-Key", &self.app_id)
            .header("X-Api-Access-Key", &self.api_key)
            .header("X-Api-Resource-Id", RESOURCE_ID)
            .header("X-Api-Connect-Id", uuid::Uuid::new_v4().to_string())
            .body(())
            .unwrap()
    }

    fn build_init_packet(&self, reqid: &str) -> Vec<u8> {
        let payload = RequestPayload {
            user: serde_json::json!({"uid": "user"}),
            audio: serde_json::json!({
                "format": "pcm", "rate": 16000, "bits": 16, "channel": 1, "codec": "raw"
            }),
            request: serde_json::json!({
                "model_name": "bigmodel", "reqid": reqid, "sequence": 1,
                "show_utterances": true, "enable_punc": true,
                "enable_vad": true, "end_window_size": 800
            }),
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let header = [0x11, 0x10, 0x10, 0x00];
        
        let mut packet = Vec::with_capacity(8 + bytes.len());
        packet.extend_from_slice(&header);
        packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        packet.extend_from_slice(&bytes);
        packet
    }

    fn build_audio_packet(audio: &[u8], seq: i32) -> Vec<u8> {
        let is_last = seq < 0;
        let header = [
            if is_last { 0x12 } else { 0x11 },
            if is_last { 0x20 } else { 0x20 },
            0x00, 0x00
        ];
        let mut packet = Vec::with_capacity(12 + audio.len());
        packet.extend_from_slice(&header);
        if is_last { packet.extend_from_slice(&seq.to_be_bytes()); }
        packet.extend_from_slice(&(audio.len() as u32).to_be_bytes());
        packet.extend_from_slice(audio);
        packet
    }

    fn parse_response(data: &[u8]) -> Option<(i32, ResponsePayload)> {
        if data.len() < 8 { return None; }
        
        let header_size = (data[0] & 0x0F) as usize * 4;
        let flags = data[1] & 0x0F;
        let has_seq = (flags & 0x01) != 0 || flags == 0x03;
        
        let (seq, payload) = if has_seq {
            if data.len() < header_size + 8 { return None; }
            let seq = i32::from_be_bytes([data[header_size], data[header_size+1], data[header_size+2], data[header_size+3]]);
            let len = u32::from_be_bytes([data[header_size+4], data[header_size+5], data[header_size+6], data[header_size+7]]) as usize;
            if data.len() < header_size + 8 + len { return None; }
            (seq, &data[header_size+8..header_size+8+len])
        } else {
            if data.len() < header_size + 4 { return None; }
            let len = u32::from_be_bytes([data[header_size], data[header_size+1], data[header_size+2], data[header_size+3]]) as usize;
            if data.len() < header_size + 4 + len { return None; }
            (0, &data[header_size+4..header_size+4+len])
        };
        
        serde_json::from_slice::<ResponsePayload>(payload).ok().map(|r| (seq, r))
    }

    async fn start_listening(&self, stream: futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>) {
        let text_buf = self.text_buffer.clone();
        let connected = self.is_connected.clone();
        let text_cache = self.text_cache.clone();

        tokio::spawn(async move {
            futures::pin_mut!(stream);
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        if let Some((seq, resp)) = Self::parse_response(&data) {
                            log::info!("豆包原始响应: seq={}, code={}, error={:?}, result={:?}", seq, resp.code, resp.error, resp.result);
                            if resp.error.is_some() || (resp.code != 0 && resp.code != 1000) {
                                if resp.code >= 2000 { connected.store(false, Ordering::SeqCst); }
                                continue;
                            }

                            if let Some(result) = &resp.result {
                                let mut cache = text_cache.lock().await;
                                let new_text = &result.text;
                                
                                if *new_text != *cache {
                                    let (backspace, addition) = compute_diff(&cache, new_text);
                                    if backspace > 0 {
                                        text_buf.send_backspaces(backspace);
                                    }
                                    if !addition.is_empty() {
                                        text_buf.write(addition);
                                    }
                                    *cache = new_text.clone();
                                }
                                
                                // 会话结束，清空缓存
                                if seq < 0 {
                                    cache.clear();
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            connected.store(false, Ordering::SeqCst);
        });
    }

    pub async fn start(&self) -> Result<(), String> {
        let (ws_stream, _) = connect_async(self.build_request()).await
            .map_err(|e| format!("连接失败: {}", e))?;
        
        let (mut sink, stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);

        let init_packet = self.build_init_packet(&uuid::Uuid::new_v4().to_string());
        sink.send(Message::Binary(init_packet)).await
            .map_err(|e| format!("发送 init 包失败: {}", e))?;

        self.start_listening(stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        tokio::spawn(async move {
            let mut buf = vec![0i16; 3200];
            let mut seq: i32 = 2;

            loop {
                if !this.is_connected.load(Ordering::SeqCst) { break; }
                if let Ok(msg) = rx.try_recv() { let _ = sink.send(msg).await; }

                let n = this.audio_buffer.read(&mut buf);
                if n == 0 {
                    if this.audio_buffer.is_finished() { break; }
                    tokio::task::yield_now().await;
                    continue;
                }

                let bytes: Vec<u8> = buf[..n].iter().flat_map(|s| s.to_le_bytes()).collect();
                
                if sink.send(Message::Binary(Self::build_audio_packet(&bytes, seq))).await.is_err() {
                    break;
                }
                seq += 1;
                
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            if this.is_connected.load(Ordering::SeqCst) {
                let _ = sink.send(Message::Binary(Self::build_audio_packet(&[], -(seq - 1)))).await;
                let _ = sink.close().await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        self.is_connected.store(false, Ordering::SeqCst);
        *self.ws_sink.lock().await = None;
        self.text_cache.lock().await.clear();
    }
}

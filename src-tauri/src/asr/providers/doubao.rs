use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::DoubaoConfig;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// 小时版配置（更便宜，按使用时长计费）
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
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    result: Option<ResultItem>,
}

#[derive(Deserialize, Debug)]
struct ResultItem {
    #[serde(default)]
    text: String,
    #[serde(default)]
    utterances: Vec<Utterance>,
}

#[derive(Deserialize, Debug)]
struct Utterance {
    text: String,
    #[serde(default)]
    definite: bool,
}

impl DoubaoAsr {
    pub fn new(
        config: DoubaoConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Result<Self, String> {
        log::info!("[豆包] 创建 ASR 实例");
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
        
        let req = http::Request::builder()
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
            .unwrap();
        
        log::info!("[豆包] 构建请求: app_id={}, resource_id={}", self.app_id, RESOURCE_ID);
        req
    }

    fn build_init_packet(&self, reqid: &str) -> Vec<u8> {
        let payload = RequestPayload {
            user: serde_json::json!({"uid": "user"}),
            audio: serde_json::json!({
                "format": "pcm", "rate": 16000, "bits": 16, "channel": 1, "codec": "raw"
            }),
            request: serde_json::json!({
                "model_name": "bigmodel", "reqid": reqid, "sequence": 1,
                "show_utterances": true, "enable_punc": true
            }),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let bytes = json.as_bytes();
        
        // Header: version=1, header_size=1, msg_type=1, flags=0, serial=1, compress=0
        let header = [0x11, 0x10, 0x10, 0x00];
        
        let mut packet = Vec::with_capacity(8 + bytes.len());
        packet.extend_from_slice(&header);
        packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        packet.extend_from_slice(bytes);
        log::info!("[豆包] init 包: {} bytes", packet.len());
        packet
    }

    fn build_audio_packet(audio: &[u8], seq: i32) -> Vec<u8> {
        let is_last = seq < 0;
        // Header: version=1, header_size=1/2, msg_type=2, flags=0/2/3, serial=0, compress=0
        let header = [
            if is_last { 0x12 } else { 0x11 },
            if is_last { 0x20 } else { 0x20 },
            0x00, 0x00
        ];
        
        let mut packet = Vec::with_capacity(12 + audio.len());
        packet.extend_from_slice(&header);
        if is_last {
            packet.extend_from_slice(&seq.to_be_bytes());
        }
        packet.extend_from_slice(&(audio.len() as u32).to_be_bytes());
        packet.extend_from_slice(audio);
        packet
    }

    fn parse_response(data: &[u8]) -> Option<(i32, ResponsePayload)> {
        // 打印前16字节用于调试
        let hex: String = data.iter().take(16).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
        log::info!("[豆包] 收到: {} bytes, hex={}", data.len(), hex);
        
        if data.len() < 8 { 
            log::warn!("[豆包] 数据太短: {} < 8", data.len());
            return None; 
        }
        
        let header_size = (data[0] & 0x0F) as usize * 4;
        let flags = (data[1] & 0x0F) as u8; // 获取 message type specific flags
        
        // 根据 flags 判断是否有 sequence number
        // flags: 0b0000 = 无sequence, 0b0001 = 有sequence(正), 0b0010/0b0011 = 最后一包
        let has_sequence = (flags & 0x01) != 0 || flags == 0x03;
        
        let (seq, payload_len, payload_offset) = if has_sequence {
            // 格式: header + sequence(4) + payload_size(4) + payload
            if data.len() < header_size + 8 {
                log::warn!("[豆包] 数据不足以解析头(含seq): {} < {}", data.len(), header_size + 8);
                return None;
            }
            let seq_offset = header_size;
            let size_offset = header_size + 4;
            let payload_offset = header_size + 8;
            let seq = i32::from_be_bytes([data[seq_offset], data[seq_offset+1], data[seq_offset+2], data[seq_offset+3]]);
            let payload_len = u32::from_be_bytes([data[size_offset], data[size_offset+1], data[size_offset+2], data[size_offset+3]]) as usize;
            (seq, payload_len, payload_offset)
        } else {
            // 格式: header + payload_size(4) + payload (无sequence)
            if data.len() < header_size + 4 {
                log::warn!("[豆包] 数据不足以解析头(无seq): {} < {}", data.len(), header_size + 4);
                return None;
            }
            let size_offset = header_size;
            let payload_offset = header_size + 4;
            let payload_len = u32::from_be_bytes([data[size_offset], data[size_offset+1], data[size_offset+2], data[size_offset+3]]) as usize;
            (0, payload_len, payload_offset) // seq = 0 表示无sequence
        };
        
        log::info!("[豆包] 解析: header_size={}, flags={:04b}, has_seq={}, seq={}, payload_len={}", 
                   header_size, flags, has_sequence, seq, payload_len);
        
        if data.len() < payload_offset + payload_len { 
            log::warn!("[豆包] 数据不完整: {} < {}", data.len(), payload_offset + payload_len);
            return None; 
        }
        
        let payload = &data[payload_offset..payload_offset + payload_len];
        let json_str = String::from_utf8_lossy(payload);
        log::info!("[豆包] JSON: {}", json_str);
        
        match serde_json::from_slice::<ResponsePayload>(payload) {
            Ok(r) => {
                log::info!("[豆包] 解析成功: code={:?}, error={:?}", r.code, r.error);
                Some((seq, r))
            }
            Err(e) => {
                log::error!("[豆包] JSON解析失败: {}", e);
                None
            }
        }
    }

    async fn start_listening(&self, mut stream: futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>) {
        let text_buf = self.text_buffer.clone();
        let connected = self.is_connected.clone();
        let cache = self.text_cache.clone();

        log::info!("[豆包] 启动接收任务");
        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        if let Some((seq, resp)) = Self::parse_response(&data) {
                            // 检查 error 字段
                            if let Some(err) = &resp.error {
                                log::error!("[豆包] 服务端错误: {}", err);
                                connected.store(false, Ordering::SeqCst);
                                break;
                            }
                            
                            // code=0 或 code=1000 都表示成功
                            if resp.code != 0 && resp.code != 1000 {
                                log::error!("[豆包] 错误码: {}", resp.code);
                                if resp.code >= 2000 { connected.store(false, Ordering::SeqCst); }
                                continue;
                            }

                            if let Some(result) = &resp.result {
                                log::info!("[豆包] 结果: text='{}', utterances={}", result.text, result.utterances.len());
                                for utt in &result.utterances {
                                    let mut c = cache.lock().await;
                                    let text = if utt.text.starts_with(&*c) && !c.is_empty() {
                                        &utt.text[c.len()..]
                                    } else {
                                        &utt.text
                                    };
                                    log::info!("[豆包] 增量: '{}' (cache='{}', definite={})", text, c, utt.definite);
                                    if !text.is_empty() { text_buf.write(text.to_string()); }
                                    *c = utt.text.clone();
                                    if utt.definite { c.clear(); }
                                }
                                if result.utterances.is_empty() && !result.text.is_empty() {
                                    log::info!("[豆包] 完整文本: '{}'", result.text);
                                    text_buf.write(result.text.clone());
                                }
                            }
                            if seq < 0 { cache.lock().await.clear(); }
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        log::info!("[豆包] 收到 Close: {:?}", frame);
                        connected.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        log::debug!("[豆包] 收到 Ping");
                    }
                    Ok(other) => {
                        log::info!("[豆包] 收到其他消息: {:?}", other);
                    }
                    Err(e) => {
                        log::error!("[豆包] WebSocket 错误: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
            log::info!("[豆包] 接收任务结束");
            connected.store(false, Ordering::SeqCst);
        });
    }

    pub async fn start(&self) -> Result<(), String> {
        log::info!("[豆包] ========== 开始 ==========");
        
        let (ws_stream, response) = connect_async(self.build_request()).await
            .map_err(|e| format!("连接失败: {}", e))?;
        
        log::info!("[豆包] 连接成功! 状态: {:?}", response.status());
        
        let (mut sink, stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);

        let reqid = uuid::Uuid::new_v4().to_string();
        let init_packet = self.build_init_packet(&reqid);
        sink.send(Message::Binary(init_packet)).await
            .map_err(|e| format!("发送 init 包失败: {}", e))?;
        log::info!("[豆包] init 包已发送");

        self.start_listening(stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        tokio::spawn(async move {
            log::info!("[豆包] 发送任务启动");
            // 豆包建议单包 200ms 音频最优: 16000Hz * 0.2s = 3200 samples
            let mut buf = vec![0i16; 3200];
            let mut seq: i32 = 2;
            let mut total = 0usize;

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
                total += n;
                
                if seq % 100 == 0 {
                    log::info!("[豆包] 发送音频包: seq={}, samples={}, total={}", seq, n, total);
                }
                
                if sink.send(Message::Binary(Self::build_audio_packet(&bytes, seq))).await.is_err() {
                    log::error!("[豆包] 发送音频包失败");
                    this.is_connected.store(false, Ordering::SeqCst);
                    break;
                }
                seq += 1;
                
                // 控制发包间隔约 100-200ms
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

                if this.audio_buffer.is_finished() && this.audio_buffer.is_empty() { break; }
            }

            log::info!("[豆包] 发送任务结束，共发送 {} samples", total);

            if this.is_connected.load(Ordering::SeqCst) {
                let end_seq = -(seq - 1);
                log::info!("[豆包] 发送结束包 (seq={})", end_seq);
                let _ = sink.send(Message::Binary(Self::build_audio_packet(&[], end_seq))).await;
                let _ = sink.close().await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        log::info!("[豆包] stop()");
        self.is_connected.store(false, Ordering::SeqCst);
        *self.ws_sink.lock().await = None;
        self.text_cache.lock().await.clear();
    }
}

use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::DoubaoConfig;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
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
                "show_utterances": true, "enable_punc": true
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

    async fn start_listening(&self, stream: futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
        log_file: Arc<Mutex<tokio::fs::File>>) {
        let text_buf = self.text_buffer.clone();
        let connected = self.is_connected.clone();
        let cache = self.text_cache.clone();

        tokio::spawn(async move {
            let mut last_recv_time = std::time::Instant::now();
            let mut recv_count = 0u64;
            
            futures::pin_mut!(stream);
            while let Some(msg_result) = stream.next().await {
                let now = std::time::Instant::now();
                let elapsed_since_last = now.duration_since(last_recv_time);
                last_recv_time = now;
                recv_count += 1;
                
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        let header = if data.len() >= 2 {
                            format!("{:02x} {:02x}", data[0], data[1])
                        } else {
                            "too_short".to_string()
                        };
                        
                        let log_entry = format!(
                            "\n===== [{} #{}] 收到包 | 距上次: {:?} =====\nheader: {}, 数据长度: {} bytes\n",
                            timestamp, recv_count, elapsed_since_last, header, data.len()
                        );
                        
                        let _ = log_file.lock().await.write_all(log_entry.as_bytes()).await;
                        println!("[豆包调试] #{} 收到包, header={}, 长度={}", recv_count, header, data.len());
                        
                        if let Some((seq, resp)) = Self::parse_response(&data) {
                            let result_detail = if let Some(ref result) = resp.result {
                                if let Some(utt) = result.utterances.last() {
                                    format!("utterance: text='{}' definite={}", utt.text, utt.definite)
                                } else {
                                    format!("result.text='{}'", result.text)
                                }
                            } else {
                                "no result".to_string()
                            };
                            
                            let parsed_log = format!(
                                "解析: seq={}, code={}, error={:?}, {}\n",
                                seq, resp.code, resp.error, result_detail
                            );
                            
                            let _ = log_file.lock().await.write_all(parsed_log.as_bytes()).await;
                            println!("[豆包调试] 解析: seq={}, code={}, {}", seq, resp.code, result_detail);
                            
                            if resp.error.is_some() || (resp.code != 0 && resp.code != 1000) {
                                let err_log = format!("!!! 错误响应: code={}, error={:?}\n", resp.code, resp.error);
                                let _ = log_file.lock().await.write_all(err_log.as_bytes()).await;
                                println!("[豆包调试] !!! 错误响应: {:?}", resp.error);
                                if resp.code >= 2000 { connected.store(false, Ordering::SeqCst); }
                                continue;
                            }

                            if let Some(result) = &resp.result {
                                if let Some(utt) = result.utterances.last() {
                                    let mut c = cache.lock().await;
                                    
                                    // 计算增量（处理豆包文本修正）
                                    let text = if utt.text.starts_with(&*c) && !c.is_empty() {
                                        &utt.text[c.len()..]
                                    } else if !c.is_empty() {
                                        let common = c.chars().zip(utt.text.chars())
                                            .take_while(|(a, b)| a == b).count();
                                        let pos = utt.text.char_indices().nth(common).map(|(i, _)| i).unwrap_or(0);
                                        &utt.text[pos..]
                                    } else {
                                        &utt.text
                                    };
                                    
                                    let text_log = format!(
                                        "处理: cache='{}' -> new='{}', 增量='{}', definite={}\n",
                                        c, utt.text, text, utt.definite
                                    );
                                    let _ = log_file.lock().await.write_all(text_log.as_bytes()).await;
                                    println!("[豆包调试] {}", text_log.trim());
                                    
                                    if !text.is_empty() { text_buf.write(text.to_string()); }
                                    *c = utt.text.clone();
                                    if utt.definite { c.clear(); }
                                } else if !result.text.is_empty() {
                                    let text_log = format!("直接写入: '{}'\n", result.text);
                                    let _ = log_file.lock().await.write_all(text_log.as_bytes()).await;
                                    println!("[豆包调试] 直接写入: '{}'", result.text);
                                    text_buf.write(result.text.clone());
                                }
                            }
                            if seq < 0 { 
                                let end_log = "收到结束标记(seq<0)，清空缓存\n";
                                let _ = log_file.lock().await.write_all(end_log.as_bytes()).await;
                                println!("[豆包调试] {}", end_log.trim());
                                cache.lock().await.clear(); 
                            }
                        } else {
                            let parse_err = "!!! 解析响应失败\n";
                            let _ = log_file.lock().await.write_all(parse_err.as_bytes()).await;
                            println!("[豆包调试] {}", parse_err.trim());
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        let close_log = format!("!!! WebSocket 关闭: {:?}\n", frame);
                        let _ = log_file.lock().await.write_all(close_log.as_bytes()).await;
                        println!("[豆包调试] {}", close_log.trim());
                        break;
                    }
                    Ok(other) => {
                        let other_log = format!("收到其他消息类型: {:?}\n", other);
                        let _ = log_file.lock().await.write_all(other_log.as_bytes()).await;
                        println!("[豆包调试] {}", other_log.trim());
                    }
                    Err(e) => {
                        let err_log = format!("!!! WebSocket 错误: {}\n", e);
                        let _ = log_file.lock().await.write_all(err_log.as_bytes()).await;
                        println!("[豆包调试] {}", err_log.trim());
                        break;
                    }
                }
            }
            
            let end_log = format!("\n===== [{}] 监听结束，共收到 {} 个包 =====\n", 
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), recv_count);
            let _ = log_file.lock().await.write_all(end_log.as_bytes()).await;
            println!("[豆包调试] 监听结束，共收到 {} 个包", recv_count);
            connected.store(false, Ordering::SeqCst);
        });
    }

    pub async fn start(&self) -> Result<(), String> {
        let (ws_stream, _) = connect_async(self.build_request()).await
            .map_err(|e| format!("连接失败: {}", e))?;
        
        let (mut sink, stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);

        // 创建日志文件
        let log_path = format!("doubao_log_{}.txt", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let log_file = match tokio::fs::File::create(&log_path).await {
            Ok(f) => {
                println!("[豆包日志] 开始记录到文件: {}", log_path);
                Arc::new(Mutex::new(f))
            }
            Err(e) => {
                eprintln!("[豆包日志] 创建日志文件失败: {}", e);
                // 创建一个虚拟的文件，写入会失败但不会 panic
                Arc::new(Mutex::new(tokio::fs::File::create("/dev/null").await.unwrap()))
            }
        };

        let init_packet = self.build_init_packet(&uuid::Uuid::new_v4().to_string());
        let init_log = format!("发送 init 包，长度: {} bytes\n", init_packet.len());
        let _ = log_file.lock().await.write_all(init_log.as_bytes()).await;
        println!("[豆包调试] {}", init_log.trim());
        
        sink.send(Message::Binary(init_packet)).await
            .map_err(|e| format!("发送 init 包失败: {}", e))?;

        let recv_log_file = log_file.clone();
        self.start_listening(stream, recv_log_file).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        let send_log_file = log_file.clone();
        tokio::spawn(async move {
            let mut buf = vec![0i16; 3200];
            let mut seq: i32 = 2;
            let mut send_count = 0u64;
            let mut last_send_time = std::time::Instant::now();

            loop {
                if !this.is_connected.load(Ordering::SeqCst) { 
                    println!("[豆包调试] 发送循环: 连接已断开，退出");
                    break; 
                }
                
                // 检查是否有控制消息
                if let Ok(msg) = rx.try_recv() { 
                    let msg_type = match &msg {
                        Message::Binary(_) => "Binary",
                        Message::Close(_) => "Close",
                        _ => "Other",
                    };
                    println!("[豆包调试] 发送循环: 收到控制消息({})", msg_type);
                    if sink.send(msg).await.is_err() {
                        println!("[豆包调试] 发送循环: 发送控制消息失败");
                        break;
                    }
                }

                // 读取音频数据
                let n = this.audio_buffer.read(&mut buf);
                if n == 0 {
                    if this.audio_buffer.is_finished() { 
                        println!("[豆包调试] 发送循环: 音频缓冲区完成，退出");
                        break; 
                    }
                    tokio::task::yield_now().await;
                    continue;
                }

                let bytes: Vec<u8> = buf[..n].iter().flat_map(|s| s.to_le_bytes()).collect();
                let packet = Self::build_audio_packet(&bytes, seq);
                
                send_count += 1;
                let now = std::time::Instant::now();
                let interval = now.duration_since(last_send_time);
                last_send_time = now;
                
                let send_log = format!(
                    "[发送 #{}] seq={}, 音频{}samples({}bytes), 距上次{:?}\n",
                    send_count, seq, n, bytes.len(), interval
                );
                let _ = send_log_file.lock().await.write_all(send_log.as_bytes()).await;
                
                if send_count <= 5 || send_count % 10 == 0 {
                    println!("[豆包调试] {}", send_log.trim());
                }
                
                if sink.send(Message::Binary(packet)).await.is_err() {
                    let err_log = "!!! 发送音频包失败\n";
                    let _ = send_log_file.lock().await.write_all(err_log.as_bytes()).await;
                    println!("[豆包调试] {}", err_log.trim());
                    break;
                }
                seq += 1;
                
                // 控制发包间隔约 150ms（豆包建议 100-200ms）
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            let end_log = format!("\n[{}] 发送循环结束，共发送 {} 个包\n",
                chrono::Local::now().format("%H:%M:%S%.3f"), send_count);
            let _ = send_log_file.lock().await.write_all(end_log.as_bytes()).await;
            println!("[豆包调试] {}", end_log.trim());

            if this.is_connected.load(Ordering::SeqCst) {
                println!("[豆包调试] 发送结束包 seq=-{}", seq - 1);
                let _ = sink.send(Message::Binary(Self::build_audio_packet(&[], -(seq - 1)))).await;
                let _ = sink.close().await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        println!("[豆包调试] stop() 被调用");
        self.is_connected.store(false, Ordering::SeqCst);
        *self.ws_sink.lock().await = None;
        self.text_cache.lock().await.clear();
    }
}

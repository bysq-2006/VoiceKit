use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::FunasrConfig;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Clone)]
pub struct FunasrAsr {
    host: String,
    port: u16,
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    ws_sink: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Message>>>>,
    is_connected: Arc<AtomicBool>,
}

struct SenderStats {
    started_at: Instant,
    last_log_at: Instant,
    sent_samples_total: u64,
    sent_frames_total: u64,
}

impl SenderStats {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_log_at: now,
            sent_samples_total: 0,
            sent_frames_total: 0,
        }
    }

    fn on_sent(&mut self, sent_samples: usize, buf_len: usize) {
        self.sent_samples_total += sent_samples as u64;
        self.sent_frames_total += 1;

        let now = Instant::now();
        if now.duration_since(self.last_log_at) >= Duration::from_secs(1) {
            let elapsed = now.duration_since(self.started_at).as_secs_f64().max(1e-6);
            let out_rate = self.sent_samples_total as f64 / elapsed;
            let rt = out_rate / 16000.0;
            let avg_frame_samples = if self.sent_frames_total > 0 {
                self.sent_samples_total as f64 / self.sent_frames_total as f64
            } else {
                0.0
            };
            let avg_frame_ms = avg_frame_samples / 16000.0 * 1000.0;
            log::info!(
                "[ASR-SEND] out≈{:.1}Hz expect=16000Hz rt={:.3} frames={} sent_total={} avg_frame={:.1}samples/{:.1}ms buf_len={}",
                out_rate,
                rt,
                self.sent_frames_total,
                self.sent_samples_total,
                avg_frame_samples,
                avg_frame_ms,
                buf_len
            );
            self.last_log_at = now;
        }
    }
}

impl FunasrAsr {
    pub fn new(
        config: FunasrConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Result<Self, String> {
        let host = config.host.trim().to_string();
        if host.is_empty() {
            return Err("本地 FunASR 需要 host".to_string());
        }
        if config.port == 0 {
            return Err("本地 FunASR 需要有效 port".to_string());
        }

        Ok(Self {
            host,
            port: config.port,
            audio_buffer,
            text_buffer,
            ws_sink: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(AtomicBool::new(false)),
        })
    }

    fn ws_url(&self) -> String {
        format!("ws://{}:{}/ws/asr", self.host, self.port)
    }

    async fn start_listening(
        &self,
        stream: futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let text_buffer = self.text_buffer.clone();
        let connected = self.is_connected.clone();

        tokio::spawn(async move {
            futures::pin_mut!(stream);

            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let Ok(payload) = serde_json::from_str::<Value>(&text) else {
                            continue;
                        };

                        let event_type = payload
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        match event_type {
                            "final" => {
                                if let Some(t) = payload.get("text").and_then(|v| v.as_str()) {
                                    if !t.is_empty() {
                                        log::info!("[ASR-RECV] final len={} text={}", t.chars().count(), t);
                                        text_buffer.push_text(t);
                                    }
                                }
                            }
                            "done" => {
                                log::info!("[ASR-RECV] done");
                                connected.store(false, Ordering::SeqCst);
                                break;
                            }
                            "error" => {
                                let msg = payload
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                log::error!("FunASR 返回错误: {}", msg);
                                connected.store(false, Ordering::SeqCst);
                                break;
                            }
                            _ => {}
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("FunASR 监听失败: {}", e);
                        break;
                    }
                }
            }

            connected.store(false, Ordering::SeqCst);
        });
    }

    pub async fn start(&self) -> Result<(), String> {
        log::info!("FunASR 开始: url={}", self.ws_url());
        let (ws_stream, _) = connect_async(self.ws_url())
            .await
            .map_err(|e| format!("连接本地 FunASR 失败: {}", e))?;

        let (mut sink, stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);

        self.start_listening(stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        tokio::spawn(async move {
            // 100ms @ 16kHz => 1600 samples
            const FRAME_SAMPLES: usize = 1600;
            // 从 AudioBuffer 拉取的小块缓存
            let mut read_buf = vec![0i16; 1600];
            // 组帧缓存：严格攒满 1600 samples 才发一帧
            let mut pending: Vec<i16> = Vec::with_capacity(FRAME_SAMPLES * 2);
            let mut stats = SenderStats::new();
            let mut send_finish = true;

            loop {
                if !this.is_connected.load(Ordering::SeqCst) {
                    send_finish = false;
                    break;
                }
                if let Ok(msg) = rx.try_recv() {
                    let _ = sink.send(msg).await;
                }

                let n = this.audio_buffer.read(&mut read_buf);
                if n == 0 {
                    if this.audio_buffer.is_finished() {
                        break;
                    }
                    tokio::task::yield_now().await;
                    continue;
                }

                pending.extend_from_slice(&read_buf[..n]);

                while pending.len() >= FRAME_SAMPLES {
                    let frame: Vec<i16> = pending.drain(..FRAME_SAMPLES).collect();
                    let mut bytes = Vec::with_capacity(FRAME_SAMPLES * 2);
                    for s in &frame {
                        bytes.extend_from_slice(&s.to_le_bytes());
                    }

                    if sink.send(Message::Binary(bytes)).await.is_err() {
                        this.is_connected.store(false, Ordering::SeqCst);
                        send_finish = false;
                        break;
                    }

                    stats.on_sent(FRAME_SAMPLES, this.audio_buffer.len());
                }

                if !send_finish {
                    break;
                }

                if this.audio_buffer.is_finished() && this.audio_buffer.is_empty() {
                    // 录音结束时把不足 100ms 的尾包也发出，避免最后一句被吞
                    if !pending.is_empty() {
                        let mut bytes = Vec::with_capacity(pending.len() * 2);
                        for s in &pending {
                            bytes.extend_from_slice(&s.to_le_bytes());
                        }
                        if sink.send(Message::Binary(bytes)).await.is_ok() {
                            stats.on_sent(pending.len(), this.audio_buffer.len());
                        }
                        pending.clear();
                    }
                    log::info!("[ASR-SEND] 音频流结束，准备发送 finish");
                    break;
                }
            }

            if send_finish && this.is_connected.load(Ordering::SeqCst) {
                log::info!("[ASR-SEND] send finish(cmd=finish)");
                let _ = sink
                    .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                    .await;
                let _ = sink.close().await;
            }

            log::info!(
                "[ASR-SEND] sender exit frames={} samples={}",
                stats.sent_frames_total,
                stats.sent_samples_total
            );
        });

        Ok(())
    }

    pub async fn stop(&self) {
        log::info!("FunASR 停止: 发送 finish 并等待发送协程退出");
        if let Some(sink) = self.ws_sink.lock().await.as_ref() {
            let _ = sink
                .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                .await;
        }
        self.is_connected.store(false, Ordering::SeqCst);
        *self.ws_sink.lock().await = None;
    }
}


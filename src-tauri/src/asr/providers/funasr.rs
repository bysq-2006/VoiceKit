use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::FunasrConfig;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const FRAME_SAMPLES: usize = 1600; // 100ms @ 16kHz
const FINISH_CMD: &str = "{\"cmd\":\"finish\"}";

#[derive(Clone)]
pub struct FunasrAsr {
    host: String,
    port: u16,
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    ws_sink: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Message>>>>,
    is_connected: Arc<AtomicBool>,
    session_has_audio: Arc<AtomicBool>,
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
            session_has_audio: Arc::new(AtomicBool::new(false)),
        })
    }

    fn ws_url(&self) -> String {
        format!("ws://{}:{}/ws/asr", self.host, self.port)
    }

    fn pcm_to_bytes(samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        bytes
    }

    async fn send_finish_text(
        sink: &tokio::sync::mpsc::Sender<Message>,
    ) {
        let _ = sink.send(Message::Text(FINISH_CMD.to_string())).await;
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
        self.session_has_audio.store(false, Ordering::SeqCst);

        self.start_listening(stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        tokio::spawn(async move {
            let mut read_buf = vec![0i16; 1600];
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
                    let bytes = Self::pcm_to_bytes(&frame);

                    if sink.send(Message::Binary(bytes)).await.is_err() {
                        log::warn!("[ASR-SEND] binary send failed -> mark disconnected");
                        this.is_connected.store(false, Ordering::SeqCst);
                        send_finish = false;
                        break;
                    }
                    this.session_has_audio.store(true, Ordering::SeqCst);

                    stats.on_sent(FRAME_SAMPLES, this.audio_buffer.len());
                }

                if !send_finish {
                    break;
                }

                if this.audio_buffer.is_finished() && this.audio_buffer.is_empty() {
                    if !pending.is_empty() {
                        let bytes = Self::pcm_to_bytes(&pending);
                        if sink.send(Message::Binary(bytes)).await.is_ok() {
                            stats.on_sent(pending.len(), this.audio_buffer.len());
                            this.session_has_audio.store(true, Ordering::SeqCst);
                        }
                        pending.clear();
                    }
                    log::info!("[ASR-SEND] 音频流结束，准备发送 finish");
                    break;
                }
            }

            let has_audio = this.session_has_audio.load(Ordering::SeqCst);
            if send_finish && this.is_connected.load(Ordering::SeqCst) && has_audio {
                log::info!("[ASR-SEND] send finish(cmd=finish)");
                let _ = sink
                    .send(Message::Text(FINISH_CMD.to_string()))
                    .await;
                let _ = sink.close().await;
            } else if send_finish && this.is_connected.load(Ordering::SeqCst) && !has_audio {
                log::info!("[ASR-SEND] skip finish: no audio sent in this session");
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
        let has_audio = self.session_has_audio.load(Ordering::SeqCst);
        log::info!(
            "FunASR 停止: 发送 finish 并等待发送协程退出 is_connected={} has_audio={} audio_finished={} audio_len={}",
            self.is_connected.load(Ordering::SeqCst),
            has_audio,
            self.audio_buffer.is_finished(),
            self.audio_buffer.len()
        );
        if has_audio {
            if let Some(sink) = self.ws_sink.lock().await.as_ref() {
                Self::send_finish_text(sink).await;
            }
        } else {
            log::info!("FunASR 停止: 跳过 finish（当前会话未发送音频）");
        }
        self.session_has_audio.store(false, Ordering::SeqCst);
        self.is_connected.store(false, Ordering::SeqCst);
        *self.ws_sink.lock().await = None;
    }
}


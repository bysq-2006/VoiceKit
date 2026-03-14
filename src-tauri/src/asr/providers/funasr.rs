use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::FunasrConfig;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
                                        text_buffer.push_text(t);
                                    }
                                }
                            }
                            "done" => {
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
            let mut buf = vec![0i16; 1600];

            loop {
                if !this.is_connected.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(msg) = rx.try_recv() {
                    let _ = sink.send(msg).await;
                }

                let n = this.audio_buffer.read(&mut buf);
                if n == 0 {
                    if this.audio_buffer.is_finished() {
                        break;
                    }
                    tokio::task::yield_now().await;
                    continue;
                }

                let mut bytes = Vec::with_capacity(n * 2);
                for s in &buf[..n] {
                    bytes.extend_from_slice(&s.to_le_bytes());
                }

                if sink.send(Message::Binary(bytes)).await.is_err() {
                    this.is_connected.store(false, Ordering::SeqCst);
                    break;
                }

                if this.audio_buffer.is_finished() && this.audio_buffer.is_empty() {
                    break;
                }
            }

            if this.is_connected.load(Ordering::SeqCst) {
                let _ = sink
                    .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                    .await;
                let _ = sink.close().await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        self.is_connected.store(false, Ordering::SeqCst);
        if let Some(sink) = self.ws_sink.lock().await.as_ref() {
            let _ = sink
                .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                .await;
        }
        *self.ws_sink.lock().await = None;
    }
}


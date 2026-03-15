use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::FunasrConfig;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
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

#[derive(Deserialize)]
struct FunasrEvent {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    text: String,
}

impl FunasrAsr {
    pub fn new(
        config: FunasrConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Result<Self, String> {
        let host = config.host.trim().to_string();
        if host.is_empty() {
            return Err("FunASR 需要 host".to_string());
        }
        if config.port == 0 {
            return Err("FunASR 需要有效的 port".to_string());
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
        mut stream: futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let text_buffer = self.text_buffer.clone();
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let Ok(event) = serde_json::from_str::<FunasrEvent>(&text) else {
                            continue;
                        };

                        match event.r#type.as_str() {
                            "final" => {
                                let final_text = event.text.trim();
                                if !final_text.is_empty() {
                                    text_buffer.push_text(final_text);
                                }
                            }
                            "done" => {
                                is_connected.store(false, Ordering::SeqCst);
                                break;
                            }
                            "error" => {
                                is_connected.store(false, Ordering::SeqCst);
                                break;
                            }
                            _ => {}
                        }
                    }
                    Ok(Message::Close(_)) => {
                        is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }

            is_connected.store(false, Ordering::SeqCst);
        });
    }

    pub async fn start(&self) -> Result<(), String> {
        let url = self.ws_url();
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| format!("FunASR 连接失败: {}", e))?;

        let (mut ws_sink, ws_stream) = ws_stream.split();
        self.is_connected.store(true, Ordering::SeqCst);
        self.start_listening(ws_stream).await;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(16);
        *self.ws_sink.lock().await = Some(tx);

        let this = self.clone();
        tokio::spawn(async move {
            const CHUNK_SAMPLES_100MS: usize = 1600; // 16kHz * 0.1s
            let mut read_buf = vec![0i16; CHUNK_SAMPLES_100MS];
            let mut pending_samples: Vec<i16> = Vec::with_capacity(CHUNK_SAMPLES_100MS * 2);

            loop {
                if !this.is_connected.load(Ordering::SeqCst) {
                    break;
                }

                if let Ok(msg) = rx.try_recv() {
                    if ws_sink.send(msg).await.is_err() {
                        this.is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                }

                let n = this.audio_buffer.read(&mut read_buf);
                if n == 0 {
                    if this.audio_buffer.is_finished() {
                        break;
                    }
                    tokio::task::yield_now().await;
                    continue;
                }

                pending_samples.extend_from_slice(&read_buf[..n]);

                while pending_samples.len() >= CHUNK_SAMPLES_100MS {
                    let chunk: Vec<i16> = pending_samples.drain(..CHUNK_SAMPLES_100MS).collect();
                    let audio_bytes: Vec<u8> =
                        chunk.into_iter().flat_map(|s| s.to_le_bytes()).collect();

                    if ws_sink.send(Message::Binary(audio_bytes)).await.is_err() {
                        this.is_connected.store(false, Ordering::SeqCst);
                        break;
                    }

                    // 与服务端 chunk_ms=100 对齐
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }

                if this.audio_buffer.is_finished() && this.audio_buffer.is_empty() {
                    if !pending_samples.is_empty() {
                        let audio_bytes: Vec<u8> = pending_samples
                            .iter()
                            .flat_map(|s| s.to_le_bytes())
                            .collect();
                        let _ = ws_sink.send(Message::Binary(audio_bytes)).await;
                        pending_samples.clear();
                    }
                    break;
                }
            }

            if this.is_connected.load(Ordering::SeqCst) {
                let _ = ws_sink
                    .send(Message::Text("{\"cmd\":\"finish\"}".to_string()))
                    .await;
                let _ = ws_sink.close().await;
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

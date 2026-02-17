use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;

use crate::asr::{StreamingASRSession, ASRResult, ASRError, ASRResultData};
use crate::asr::doubao::protocol::{DoubaoProtocol, RequestPayload, ParsedResponse};

/// 豆包流式 ASR 会话
pub struct DoubaoStreamingSession {
    /// 发送通道（用于向 WebSocket 发送音频数据）
    sender: mpsc::UnboundedSender<Vec<u8>>,
    /// 接收通道（用于接收识别结果）
    result_receiver: mpsc::UnboundedReceiver<ASRResult<Option<ASRResultData>>>,
    /// 协议处理器
    _protocol: DoubaoProtocol,
    /// 是否已结束
    finished: bool,
    /// 是否已取消
    cancelled: bool,
}

impl DoubaoStreamingSession {
    pub async fn new(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        protocol: DoubaoProtocol,
        init_payload: RequestPayload,
    ) -> ASRResult<Self> {
        // 创建通道
        let (sender, mut receiver) = mpsc::unbounded_channel::<Vec<u8>>();
        let (result_sender, result_receiver) = mpsc::unbounded_channel();
        
        // 发送初始化请求
        let init_packet = protocol.build_full_client_request(&init_payload)?;
        
        // 启动 WebSocket 处理任务
        tokio::spawn(async move {
            Self::websocket_handler(
                ws_stream,
                init_packet,
                &mut receiver,
                result_sender,
            ).await;
        });
        
        Ok(Self {
            sender,
            result_receiver,
            _protocol: protocol,
            finished: false,
            cancelled: false,
        })
    }
    
    /// WebSocket 处理循环
    async fn websocket_handler(
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        init_packet: Vec<u8>,
        receiver: &mut mpsc::UnboundedReceiver<Vec<u8>>,
        result_sender: mpsc::UnboundedSender<ASRResult<Option<ASRResultData>>>,
    ) {
        use tokio_tungstenite::tungstenite::Message;
        
        // 发送初始化包
        if let Err(e) = ws_stream.send(Message::Binary(init_packet)).await {
            let _ = result_sender.send(Err(ASRError::WebSocket(format!("发送初始化包失败: {}", e))));
            return;
        }
        
        let protocol = DoubaoProtocol::new();
        let mut current_text = String::new();
        let mut last_definite_text = String::new();
        
        loop {
            tokio::select! {
                // 处理要发送的音频数据
                Some(audio_data) = receiver.recv() => {
                    if let Err(e) = ws_stream.send(Message::Binary(audio_data)).await {
                        let _ = result_sender.send(Err(ASRError::WebSocket(format!("发送音频数据失败: {}", e))));
                        break;
                    }
                }
                
                // 处理接收到的消息
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            match protocol.parse_response(&data) {
                                Ok(ParsedResponse::ServerResponse { response, is_last }) => {
                                    // 检查错误码
                                    if let Some(code) = response.code {
                                        if code != 1000 && code != 0 {
                                            let msg = response.message.unwrap_or_default();
                                            let _ = result_sender.send(Err(
                                                ASRError::RecognitionFailed(format!("错误码 {}: {}", code, msg))
                                            ));
                                            break;
                                        }
                                    }
                                    
                                    // 提取识别文本
                                    if let Some(result) = response.result {
                                        if let Some(text) = result.text {
                                            // 构建结果
                                            let is_final = is_last || result.definite;
                                            
                                            // 对于优化版 bigmodel_async，只在有变化时更新
                                            if !text.is_empty() && text != current_text {
                                                current_text = text.clone();
                                                
                                                let result_data = ASRResultData {
                                                    text: text.clone(),
                                                    is_final,
                                                    utterance_id: response.sequence.map(|s| s as u32),
                                                };
                                                
                                                if result.definite {
                                                    last_definite_text = text;
                                                }
                                                
                                                if result_sender.send(Ok(Some(result_data))).is_err() {
                                                    break;
                                                }
                                            }
                                            
                                            // 如果是最后一包，发送结束信号
                                            if is_last {
                                                // 发送最终结果
                                                if !last_definite_text.is_empty() {
                                                    let final_result = ASRResultData {
                                                        text: last_definite_text,
                                                        is_final: true,
                                                        utterance_id: None,
                                                    };
                                                    let _ = result_sender.send(Ok(Some(final_result)));
                                                }
                                                let _ = result_sender.send(Ok(None));
                                                break;
                                            }
                                        } else if is_last {
                                            // 没有文本但收到结束标志
                                            let _ = result_sender.send(Ok(None));
                                            break;
                                        }
                                    } else if is_last {
                                        let _ = result_sender.send(Ok(None));
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = result_sender.send(Err(e));
                                    break;
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            let _ = result_sender.send(Ok(None));
                            break;
                        }
                        Some(Ok(_)) => {
                            // 忽略其他类型的消息
                        }
                        Some(Err(e)) => {
                            let _ = result_sender.send(Err(ASRError::WebSocket(format!("WebSocket错误: {}", e))));
                            break;
                        }
                        None => {
                            // 连接关闭
                            let _ = result_sender.send(Ok(None));
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl StreamingASRSession for DoubaoStreamingSession {
    async fn send_audio(&mut self, audio_chunk: Vec<u8>, sequence: i32) -> ASRResult<()> {
        if self.finished || self.cancelled {
            return Err(ASRError::Cancelled);
        }
        
        // 构建音频数据包
        let packet = self._protocol.build_audio_packet(&audio_chunk, sequence, false);
        
        // 发送到 WebSocket 处理任务
        self.sender
            .send(packet)
            .map_err(|_| ASRError::WebSocket("发送通道已关闭".to_string()))?;
        
        Ok(())
    }
    
    async fn finish(&mut self) -> ASRResult<()> {
        if self.finished || self.cancelled {
            return Ok(());
        }
        
        self.finished = true;
        
        // 发送最后一包（负包）
        let last_packet = self._protocol.build_audio_packet(&[], -1, true);
        
        self.sender
            .send(last_packet)
            .map_err(|_| ASRError::WebSocket("发送通道已关闭".to_string()))?;
        
        Ok(())
    }
    
    async fn receive_result(&mut self) -> ASRResult<Option<ASRResultData>> {
        if self.cancelled {
            return Err(ASRError::Cancelled);
        }
        
        match self.result_receiver.recv().await {
            Some(result) => result,
            None => Ok(None), // 通道关闭，识别结束
        }
    }
    
    async fn cancel(&mut self) -> ASRResult<()> {
        self.cancelled = true;
        
        // 关闭发送通道，这会触发 WebSocket 任务结束
        drop(self.sender.clone());
        
        Ok(())
    }
}

use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::DoubaoConfig;
use std::sync::Arc;

/// 豆包 ASR 提供商
#[derive(Clone)]
pub struct DoubaoAsr {
    _config: DoubaoConfig,
    _audio_buffer: Arc<AudioBuffer>,
    _text_buffer: Arc<TextBuffer>,
}

impl DoubaoAsr {
    pub fn new(
        config: DoubaoConfig,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
    ) -> Self {
        Self {
            _config: config,
            _audio_buffer: audio_buffer,
            _text_buffer: text_buffer,
        }
    }
}

impl DoubaoAsr {
    pub async fn start(&self) -> Result<(), String> {
        log::info!("豆包 ASR 开始");

        // TODO: 实现豆包 WebSocket ASR 连接
        // 1. 建立 WebSocket 连接
        // 2. 从 audio_buffer 循环读取音频数据
        // 3. 发送音频数据到 WebSocket
        // 4. 接收识别结果并写入 text_buffer

        Ok(())
    }

    pub async fn stop(&self) {
        log::info!("豆包 ASR 停止");
        // TODO: 关闭 WebSocket 连接
    }
}

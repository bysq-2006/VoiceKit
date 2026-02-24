use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::AppConfig;
use std::sync::{Arc, Mutex};

/// ASR 提供商枚举（替代 trait object，避免 async-trait 依赖）
pub enum AsrProvider {
    Xunfei(super::providers::xunfei::XunfeiAsr),
    Doubao(super::providers::doubao::DoubaoAsr),
}

impl AsrProvider {
    pub async fn start(&self) -> Result<(), String> {
        match self {
            AsrProvider::Xunfei(p) => p.start().await,
            AsrProvider::Doubao(p) => p.start().await,
        }
    }

    pub async fn stop(&self) {
        match self {
            AsrProvider::Xunfei(p) => p.stop().await,
            AsrProvider::Doubao(p) => p.stop().await,
        }
    }
}

/// ASR 管理器
/// 
/// 职责：根据配置创建并启动对应的 ASR 提供商
pub struct AsrManager {
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    config: Arc<Mutex<AppConfig>>,
}

impl AsrManager {
    pub fn new(
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
        config: Arc<Mutex<AppConfig>>,
    ) -> Self {
        Self {
            audio_buffer,
            text_buffer,
            config,
        }
    }

    /// 根据当前配置创建 ASR 提供商
    pub fn create_provider(&self) -> Result<AsrProvider, String> {
        let asr_config = self.config.lock().unwrap().asr.clone();
        
        match asr_config.provider.as_str() {
            "xunfei" => {
                let provider = super::providers::xunfei::XunfeiAsr::new(
                    asr_config.xunfei.clone(),
                    self.audio_buffer.clone(),
                    self.text_buffer.clone(),
                )?;
                Ok(AsrProvider::Xunfei(provider))
            }
            _ => {
                let provider = super::providers::doubao::DoubaoAsr::new(
                    asr_config.doubao.clone(),
                    self.audio_buffer.clone(),
                    self.text_buffer.clone(),
                );
                Ok(AsrProvider::Doubao(provider))
            }
        }
    }
}

/// 初始化 ASR 管理器
pub fn init_asr_manager(
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    config: Arc<Mutex<AppConfig>>,
) -> Arc<AsrManager> {
    Arc::new(AsrManager::new(audio_buffer, text_buffer, config))
}

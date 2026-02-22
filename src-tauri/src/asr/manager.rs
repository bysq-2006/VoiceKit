use super::provider::AsrProvider;
use super::providers::doubao::DoubaoAsr;
use super::providers::xunfei::XunfeiAsr;
use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::models::config::AsrConfig;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// ASR 管理器
pub struct AsrManager {
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    is_recording: Arc<std::sync::Mutex<bool>>,
}

impl AsrManager {
    pub fn new(
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
        is_recording: Arc<std::sync::Mutex<bool>>,
    ) -> Self {
        Self {
            audio_buffer,
            text_buffer,
            is_recording,
        }
    }

    /// 启动 ASR 监控线程
    pub fn start_monitoring(self: Arc<Self>, config: AsrConfig) {
        thread::spawn(move || {
            let mut was_recording = false;
            let mut current_provider: Option<Arc<dyn AsrProvider>> = None;

            loop {
                let is_recording = *self.is_recording.lock().unwrap();

                if is_recording && !was_recording {
                    log::info!("检测到录音开始，启动 ASR: {}", config.provider);

                    // 根据配置创建对应的 ASR 提供商
                    let provider: Arc<dyn AsrProvider> = match config.provider.as_str() {
                        "xunfei" => {
                            match XunfeiAsr::new(
                                config.clone(),
                                self.audio_buffer.clone(),
                                self.text_buffer.clone(),
                            ) {
                                Ok(xunfei) => Arc::new(xunfei),
                                Err(e) => {
                                    log::error!("讯飞 ASR 初始化失败: {}", e);
                                    continue;
                                }
                            }
                        }
                        _ => Arc::new(DoubaoAsr::new(
                            config.clone(),
                            self.audio_buffer.clone(),
                            self.text_buffer.clone(),
                        )),
                    };

                    // 在异步运行时中启动 ASR
                    let provider_clone = provider.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = provider_clone.start().await {
                            log::error!("ASR 启动失败: {}", e);
                        }
                    });

                    current_provider = Some(provider);
                    was_recording = true;
                } else if !is_recording && was_recording {
                    log::info!("检测到录音停止，停止 ASR");
                    
                    if let Some(provider) = current_provider.take() {
                        tauri::async_runtime::spawn(async move {
                            provider.stop().await;
                        });
                    }
                    
                    self.text_buffer.finish();
                    was_recording = false;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });
    }
}

/// 初始化 ASR 管理器
pub fn init_asr_manager(
    audio_buffer: Arc<AudioBuffer>,
    text_buffer: Arc<TextBuffer>,
    is_recording: Arc<std::sync::Mutex<bool>>,
    config: AsrConfig,
) {
    let manager = Arc::new(AsrManager::new(audio_buffer, text_buffer, is_recording));
    manager.start_monitoring(config);
    log::info!("ASR 管理器已启动");
}

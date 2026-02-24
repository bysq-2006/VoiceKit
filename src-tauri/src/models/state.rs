use std::sync::{Arc, Mutex, RwLock};
use crate::models::config::AppConfig;
use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::asr::manager::{AsrManager, AsrProvider};

pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub audio_buffer: Arc<AudioBuffer>,
    pub text_buffer: Arc<TextBuffer>,
    /// ASR 管理器
    pub asr_manager: Arc<AsrManager>,
    /// 当前缓存的 ASR Provider（避免重复创建）
    pub current_provider: Arc<RwLock<Option<AsrProvider>>>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            is_recording: self.is_recording.clone(),
            config: self.config.clone(),
            audio_buffer: self.audio_buffer.clone(),
            text_buffer: self.text_buffer.clone(),
            asr_manager: self.asr_manager.clone(),
            current_provider: self.current_provider.clone(),
        }
    }
}

impl AppState {
    pub fn new(
        asr_manager: Arc<AsrManager>,
        audio_buffer: Arc<AudioBuffer>,
        text_buffer: Arc<TextBuffer>,
        config: Arc<Mutex<AppConfig>>,
    ) -> Self {
        Self {
            is_recording: Arc::new(Mutex::new(false)),
            config,
            audio_buffer,
            text_buffer,
            asr_manager,
            current_provider: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn init_config(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let mut config = AppConfig::load(app)?;
        // 如果快捷键为空，使用默认值
        if config.shortcut.trim().is_empty() {
            config.shortcut = crate::models::config::AppConfig::default().shortcut;
        }
        *self.config.lock().unwrap() = config;
        Ok(())
    }
    
    pub fn update_config(&self, app: &tauri::AppHandle, new_config: AppConfig) -> Result<(), String> {
        new_config.save(app)?;
        *self.config.lock().unwrap() = new_config;
        Ok(())
    }
}

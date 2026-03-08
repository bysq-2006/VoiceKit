use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use crate::models::config::AppConfig;
use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::asr::manager::AsrManager;

pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,      // 录音状态：是否正在录音
    pub config: Arc<Mutex<AppConfig>>,       // 应用配置：快捷键、ASR设置等
    pub audio_buffer: Arc<AudioBuffer>,      // 音频缓冲区：录音数据暂存
    pub text_buffer: Arc<TextBuffer>,        // 文本缓冲区：识别结果暂存
    pub asr_manager: Arc<AsrManager>,        // ASR管理器：语音识别服务协调
    pub is_simulating_input: AtomicBool,     // 输入模拟标志：是否正在模拟键盘输入
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            is_recording: self.is_recording.clone(),
            config: self.config.clone(),
            audio_buffer: self.audio_buffer.clone(),
            text_buffer: self.text_buffer.clone(),
            asr_manager: self.asr_manager.clone(),
            is_simulating_input: AtomicBool::new(self.is_simulating_input.load(Ordering::SeqCst)),
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
            is_simulating_input: AtomicBool::new(false),
        }
    }

    pub fn init_config(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let mut config = AppConfig::load(app)?;
        if config.shortcut.trim().is_empty() {
            config.shortcut = AppConfig::default().shortcut;
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

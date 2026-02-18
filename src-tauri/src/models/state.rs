use std::sync::Mutex;
use std::sync::Arc;
use crate::models::config::AppConfig;
use crate::models::buffer::{AudioBuffer, TextBuffer};

#[derive(Clone)]
pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub audio_buffer: Arc<AudioBuffer>,
    pub text_buffer: Arc<TextBuffer>,
}

impl AppState {
    pub fn new() -> Self {
        let text_buffer = Arc::new(TextBuffer::new());
        
        // 初始化测试文字（用于测试输入模块）
        text_buffer.write("这是一段测试文本，用于验证模拟输入模块是否正常工作。".to_string());
        text_buffer.write("Hello World! 123".to_string());
        text_buffer.write("测试完成。".to_string());
        
        Self {
            is_recording: Arc::new(Mutex::new(false)),
            config: Arc::new(Mutex::new(AppConfig::default())),
            audio_buffer: Arc::new(AudioBuffer::new()),
            text_buffer,
        }
    }
    
    pub fn init_config(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let config = AppConfig::load(app)?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }
    
    pub fn update_config(&self, app: &tauri::AppHandle, new_config: AppConfig) -> Result<(), String> {
        new_config.save(app)?;
        *self.config.lock().unwrap() = new_config;
        Ok(())
    }
}

use std::sync::Mutex;
use crate::models::config::AppConfig;

pub struct AppState {
    /// 是否正在语音转文字（录音中）
    pub is_recording: Mutex<bool>,
    
    /// 应用配置（与 is_recording 同级）
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            config: Mutex::new(AppConfig::default()),
        }
    }
    
    /// 初始化配置（从 store 加载）
    pub fn init_config(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let config = AppConfig::load(app)?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }
}

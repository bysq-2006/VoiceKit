use std::sync::Mutex;
use crate::models::config::AppConfig;

pub struct AppState {
    pub is_recording: Mutex<bool>,
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            config: Mutex::new(AppConfig::default()),
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

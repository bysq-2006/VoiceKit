use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use crate::models::config::AppConfig;
use crate::models::buffer::{AudioBuffer, TextBuffer};
use crate::asr::manager::AsrManager;

pub struct AppState {
    pub is_recording: Arc<Mutex<bool>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub audio_buffer: Arc<AudioBuffer>,
    pub text_buffer: Arc<TextBuffer>,
    pub asr_manager: Arc<AsrManager>,
    pub is_simulating_input: AtomicBool,
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

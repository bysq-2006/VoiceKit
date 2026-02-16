use std::sync::Mutex;

pub struct AppState {
    /// 是否正在语音转文字（录音中）
    pub is_recording: Mutex<bool>,
    // 窗口是否显示由AppHandle控制
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
        }
    }
}
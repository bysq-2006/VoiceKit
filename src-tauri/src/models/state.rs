use std::sync::Mutex;
use crate::models::config::AppConfig;
use crate::asr::{ASRProvider, ASRFactory};
use std::sync::Arc;

pub struct AppState {
    /// 是否正在语音转文字（录音中）
    pub is_recording: Mutex<bool>,
    
    /// 应用配置（与 is_recording 同级）
    pub config: Mutex<AppConfig>,
    
    /// ASR Provider（使用 Box<dyn Trait> 实现运行时多态）
    pub asr_provider: Mutex<Option<Arc<dyn ASRProvider>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            config: Mutex::new(AppConfig::default()),
            asr_provider: Mutex::new(None),
        }
    }
    
    /// 初始化配置（从 store 加载）
    pub fn init_config(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let config = AppConfig::load(app)?;
        
        // 检查是否需要初始化 ASR Provider（在获取锁之前）
        let should_init_asr = config.is_asr_configured();
        let asr_config = config.asr.clone();
        
        // 更新配置锁
        *self.config.lock().unwrap() = config;
        
        // 初始化 ASR Provider（如果需要）
        if should_init_asr {
            match ASRFactory::create(&asr_config) {
                Ok(provider) => {
                    *self.asr_provider.lock().unwrap() = Some(provider);
                    log::info!("ASR Provider 初始化成功");
                }
                Err(e) => {
                    log::error!("ASR Provider 初始化失败: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// 更新配置并重新初始化 ASR Provider
    pub fn update_config(&self, app: &tauri::AppHandle, new_config: AppConfig) -> Result<(), String> {
        // 检查是否需要初始化 ASR
        let should_init_asr = new_config.is_asr_configured();
        let asr_config = new_config.asr.clone();
        
        // 保存配置
        new_config.save(app)?;
        
        // 更新内存中的配置
        *self.config.lock().unwrap() = new_config;
        
        // 重新初始化 ASR Provider
        if should_init_asr {
            match ASRFactory::create(&asr_config) {
                Ok(provider) => {
                    *self.asr_provider.lock().unwrap() = Some(provider);
                    log::info!("ASR Provider 重新初始化成功");
                }
                Err(e) => {
                    log::error!("ASR Provider 重新初始化失败: {}", e);
                }
            }
        } else {
            // 清除 ASR Provider
            *self.asr_provider.lock().unwrap() = None;
        }
        
        Ok(())
    }
    
    /// 获取 ASR Provider（如果已初始化）
    pub fn get_asr_provider(&self) -> Option<Arc<dyn ASRProvider>> {
        self.asr_provider.lock().unwrap().clone()
    }
    
    /// 检查 ASR 是否可用
    pub fn is_asr_available(&self) -> bool {
        self.asr_provider.lock().unwrap().is_some()
    }
}

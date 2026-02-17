use serde::{Deserialize, Serialize};
use crate::asr::factory::{ASRConfig, ASRProviderType};

/// 应用配置（所有配置项集中管理）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// API 密钥（兼容旧配置，已迁移到 asr.doubao.access_key）
    #[serde(default)]
    pub api_key: Option<String>,
    
    /// 全局快捷键，默认 Shift+E
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    
    /// 主题
    #[serde(default)]
    pub theme: Theme,
    
    /// 开机自启
    #[serde(default)]
    pub auto_start: bool,
    
    /// ASR 配置
    #[serde(default)]
    pub asr: ASRConfig,
}

fn default_shortcut() -> String {
    "Shift+E".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

impl AppConfig {
    /// 从 store 加载配置
    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        use tauri_plugin_store::StoreExt;
        
        let store = app.store("config.json")
            .map_err(|e| e.to_string())?;
        
        let mut config: AppConfig = if let Some(value) = store.get("config") {
            serde_json::from_value(value)
                .map_err(|e| format!("解析配置失败: {}", e))?
        } else {
            AppConfig::default()
        };
        
        // 配置迁移：将旧版 api_key 迁移到新版 asr.doubao.access_key
        if config.asr.doubao.access_key.is_none() {
            if let Some(ref api_key) = config.api_key {
                config.asr.doubao.access_key = Some(api_key.clone());
            }
        }
        
        Ok(config)
    }

    /// 保存配置到 store
    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        use tauri_plugin_store::StoreExt;
        
        let store = app.store("config.json")
            .map_err(|e| e.to_string())?;
        
        let value = serde_json::to_value(self)
            .map_err(|e| e.to_string())?;
        
        store.set("config", value);
        store.save().map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    /// 获取 ASR 提供商类型
    pub fn asr_provider(&self) -> ASRProviderType {
        self.asr.provider
    }
    
    /// 检查 ASR 配置是否完整
    pub fn is_asr_configured(&self) -> bool {
        match self.asr.provider {
            ASRProviderType::Doubao => {
                self.asr.doubao.app_id.is_some() && 
                self.asr.doubao.access_key.is_some()
            }
        }
    }
}

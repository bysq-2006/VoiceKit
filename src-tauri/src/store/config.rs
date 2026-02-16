use serde::{Deserialize, Serialize};

/// 应用配置（所有配置项集中管理）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// API 密钥
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
        
        let config = if let Some(value) = store.get("config") {
            serde_json::from_value(value)
                .map_err(|e| format!("解析配置失败: {}", e))?
        } else {
            AppConfig::default()
        };
        
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
}

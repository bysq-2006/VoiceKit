use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    
    #[serde(default)]
    pub theme: Theme,
    
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
    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        use tauri_plugin_store::StoreExt;
        
        let store = app.store("config.json")
            .map_err(|e| e.to_string())?;
        
        let config: AppConfig = if let Some(value) = store.get("config") {
            serde_json::from_value(value)
                .map_err(|e| format!("解析配置失败: {}", e))?
        } else {
            AppConfig::default()
        };
        
        Ok(config)
    }

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

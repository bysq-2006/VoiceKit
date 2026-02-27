use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default = "default_shortcut")]
    pub shortcut: String,

    #[serde(default)]
    pub theme: Theme,

    #[serde(default)]
    pub auto_start: bool,

    /// ASR 配置（多服务商，同时存储）
    #[serde(default)]
    pub asr: AsrConfig,
}

/// ASR 全局配置（包含所有服务商的配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfig {
    /// 当前使用的服务商: doubao / xunfei
    #[serde(default = "default_asr_provider")]
    pub provider: String,

    /// 豆包配置
    #[serde(default)]
    pub doubao: DoubaoConfig,

    /// 讯飞配置
    #[serde(default)]
    pub xunfei: XunfeiConfig,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            provider: default_asr_provider(),
            doubao: DoubaoConfig::default(),
            xunfei: XunfeiConfig::default(),
        }
    }
}

/// 豆包 ASR 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DoubaoConfig {
    #[serde(default)]
    pub app_id: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,
}

/// 讯飞 ASR 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XunfeiConfig {
    #[serde(default)]
    pub app_id: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub api_secret: Option<String>,
}

fn default_asr_provider() -> String {
    "doubao".to_string()
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

        let store = app.store("config.json").map_err(|e| e.to_string())?;

        let config: AppConfig = if let Some(value) = store.get("config") {
            serde_json::from_value(value).map_err(|e| format!("解析配置失败: {}", e))?
        } else {
            AppConfig::default()
        };

        Ok(config)
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        use tauri_plugin_store::StoreExt;

        let store = app.store("config.json").map_err(|e| e.to_string())?;

        let value = serde_json::to_value(self).map_err(|e| e.to_string())?;

        store.set("config", value);
        store.save().map_err(|e| e.to_string())?;

        Ok(())
    }
}

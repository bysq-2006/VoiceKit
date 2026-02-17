use crate::asr::{ASRProvider, ASRResult, ASRError};
use crate::asr::doubao::{DoubaoASR, DoubaoConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// ASR 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ASRProviderType {
    /// 豆包 ASR
    Doubao,
    // 以后可以扩展：
    // Whisper,
    // Azure,
    // Local,
}

impl Default for ASRProviderType {
    fn default() -> Self {
        ASRProviderType::Doubao
    }
}

impl std::str::FromStr for ASRProviderType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "doubao" => Ok(ASRProviderType::Doubao),
            // "whisper" => Ok(ASRProviderType::Whisper),
            _ => Err(format!("未知的 ASR 提供商: {}", s)),
        }
    }
}

impl std::fmt::Display for ASRProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ASRProviderType::Doubao => write!(f, "doubao"),
        }
    }
}

/// ASR 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASRConfig {
    /// 使用的 ASR 提供商
    #[serde(default)]
    pub provider: ASRProviderType,
    
    /// 豆包配置
    #[serde(flatten)]
    pub doubao: DoubaoASRConfig,
}

impl Default for ASRConfig {
    fn default() -> Self {
        Self {
            provider: ASRProviderType::Doubao,
            doubao: DoubaoASRConfig::default(),
        }
    }
}

/// 豆包 ASR 配置（用于序列化/反序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubaoASRConfig {
    /// App ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// Access Key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    /// 资源 ID
    #[serde(default = "default_resource_id")]
    pub resource_id: String,
    /// WebSocket URL
    #[serde(default = "default_ws_url")]
    pub ws_url: String,
}

impl Default for DoubaoASRConfig {
    fn default() -> Self {
        Self {
            app_id: None,
            access_key: None,
            resource_id: default_resource_id(),
            ws_url: default_ws_url(),
        }
    }
}

fn default_resource_id() -> String {
    "volc.seedasr.sauc.concurrent".to_string()
}

fn default_ws_url() -> String {
    "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async".to_string()
}

/// ASR 工厂
pub struct ASRFactory;

impl ASRFactory {
    /// 创建 ASR Provider
    /// 
    /// # Arguments
    /// * `config` - ASR 配置
    ///
    /// # Returns
    /// 实现了 ASRProvider trait 的对象
    pub fn create(config: &ASRConfig) -> ASRResult<Arc<dyn ASRProvider>> {
        match config.provider {
            ASRProviderType::Doubao => {
                let doubao_config = Self::build_doubao_config(config)?;
                Ok(Arc::new(DoubaoASR::new(doubao_config)))
            }
        }
    }
    
    /// 根据配置构建豆包配置
    fn build_doubao_config(config: &ASRConfig) -> ASRResult<DoubaoConfig> {
        let app_id = config.doubao.app_id.clone()
            .ok_or_else(|| ASRError::Config("豆包 ASR 需要配置 app_id".to_string()))?;
        
        let access_key = config.doubao.access_key.clone()
            .ok_or_else(|| ASRError::Config("豆包 ASR 需要配置 access_key".to_string()))?;
        
        Ok(DoubaoConfig {
            app_id,
            access_key,
            resource_id: config.doubao.resource_id.clone(),
            ws_url: config.doubao.ws_url.clone(),
        })
    }
    
    /// 测试配置是否有效
    pub async fn test_config(config: &ASRConfig) -> ASRResult<()> {
        let provider = Self::create(config)?;
        provider.health_check().await
    }
}

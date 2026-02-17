use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ASRError {
    #[error("网络错误: {0}")]
    Network(String),
    
    #[error("认证失败: {0}")]
    Auth(String),
    
    #[error("识别失败: {0}")]
    RecognitionFailed(String),
    
    #[error("用户取消")]
    Cancelled,
    
    #[error("配置错误: {0}")]
    Config(String),
    
    #[error("音频错误: {0}")]
    Audio(String),
    
    #[error("WebSocket错误: {0}")]
    WebSocket(String),
    
    #[error("协议错误: {0}")]
    Protocol(String),
    
    #[error("超时")]
    Timeout,
}

pub type ASRResult<T> = Result<T, ASRError>;

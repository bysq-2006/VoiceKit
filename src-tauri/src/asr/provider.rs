/// ASR 提供商 trait
/// 不同的 ASR 服务需要实现此 trait
#[async_trait::async_trait]
pub trait AsrProvider: Send + Sync {
    /// 开始 ASR 会话
    /// 从 audio_buffer 读取音频，识别结果写入 text_buffer
    async fn start(&self) -> Result<(), String>;

    /// 停止 ASR 会话
    async fn stop(&self);
}

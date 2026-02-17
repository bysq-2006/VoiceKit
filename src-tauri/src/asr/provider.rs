use async_trait::async_trait;
use super::error::ASRResult;

/// ASR 识别结果
#[derive(Debug, Clone)]
pub struct ASRResultData {
    /// 识别出的文本
    pub text: String,
    /// 是否是最终结果
    pub is_final: bool,
    /// 语句编号（用于流式识别时的断句）
    pub utterance_id: Option<u32>,
}

/// ASR Provider Trait - 所有 ASR 服务实现此接口
#[async_trait]
pub trait ASRProvider: Send + Sync {
    /// 执行语音识别（非流式，一次性返回完整结果）
    /// 
    /// # Arguments
    /// * `audio_data` - PCM S16LE 格式音频数据，采样率 16000Hz，单声道
    ///
    /// # Returns
    /// 识别出的文本
    async fn recognize(&self, audio_data: Vec<u8>) -> ASRResult<String>;
    
    /// 开始流式识别
    /// 
    /// # Returns
    /// 流式识别会话，用于发送音频数据和接收识别结果
    async fn start_streaming(&self) -> ASRResult<Box<dyn StreamingASRSession>>;
    
    /// 获取提供商名称
    fn name(&self) -> &str;
    
    /// 测试连接是否可用
    async fn health_check(&self) -> ASRResult<()>;
}

/// 流式 ASR 会话
#[async_trait]
pub trait StreamingASRSession: Send + Sync {
    /// 发送音频数据包
    /// 
    /// # Arguments
    /// * `audio_chunk` - 音频数据片段（建议每包 100-200ms，即 3200-6400 字节 @ 16kHz/16bit）
    /// * `sequence` - 包序号
    async fn send_audio(&mut self, audio_chunk: Vec<u8>, sequence: i32) -> ASRResult<()>;
    
    /// 发送最后一包（结束识别）
    async fn finish(&mut self) -> ASRResult<()>;
    
    /// 接收识别结果
    /// 
    /// # Returns
    /// * `Some(ASRResultData)` - 识别结果
    /// * `None` - 识别结束
    async fn receive_result(&mut self) -> ASRResult<Option<ASRResultData>>;
    
    /// 取消识别
    async fn cancel(&mut self) -> ASRResult<()>;
}

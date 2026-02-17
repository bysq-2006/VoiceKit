use serde::{Deserialize, Serialize};

/// 豆包二进制协议处理器
#[derive(Debug, Clone)]
pub struct DoubaoProtocol;

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageType {
    /// 端上发送包含请求参数的 full client request
    FullClientRequest = 0x01,
    /// 端上发送包含音频数据的 audio only request
    AudioOnlyRequest = 0x02,
    /// 服务端下发包含识别结果的 full server response
    FullServerResponse = 0x09,
    /// 服务端处理错误时下发的消息
    ErrorResponse = 0x0F,
}

/// 消息类型特定标志
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageFlags {
    /// header后4个字节不为sequence number
    NoSeqNum = 0x00,
    /// header后4个字节为sequence number且为正
    WithSeqNum = 0x01,
    /// header后4个字节不为sequence number，仅指示此为最后一包（负包）
    LastPacketNoSeq = 0x02,
    /// header后4个字节为sequence number且需要为负数（最后一包/负包）
    LastPacketWithSeq = 0x03,
}

/// 序列化方法
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SerializationMethod {
    /// 无序列化
    None = 0x00,
    /// JSON 格式
    Json = 0x01,
}

/// 压缩方法
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionMethod {
    /// 无压缩
    None = 0x00,
    /// Gzip 压缩
    Gzip = 0x01,
}

impl DoubaoProtocol {
    pub fn new() -> Self {
        Self
    }
    
    /// 构建 full client request（初始化请求）
    pub fn build_full_client_request(
        &self,
        payload: &RequestPayload,
    ) -> crate::asr::ASRResult<Vec<u8>> {
        let payload_json = serde_json::to_vec(payload)
            .map_err(|e| crate::asr::ASRError::Protocol(format!("序列化失败: {}", e)))?;
        
        Ok(self.build_packet(
            MessageType::FullClientRequest,
            MessageFlags::NoSeqNum,
            SerializationMethod::Json,
            CompressionMethod::None,
            None,
            &payload_json,
        ))
    }
    
    /// 构建音频数据包
    pub fn build_audio_packet(
        &self,
        audio_data: &[u8],
        sequence: i32,
        is_last: bool,
    ) -> Vec<u8> {
        let flags = if is_last {
            MessageFlags::LastPacketWithSeq
        } else {
            MessageFlags::WithSeqNum
        };
        
        self.build_packet(
            MessageType::AudioOnlyRequest,
            flags,
            SerializationMethod::None,
            CompressionMethod::None,
            Some(sequence),
            audio_data,
        )
    }
    
    /// 构建数据包
    fn build_packet(
        &self,
        msg_type: MessageType,
        flags: MessageFlags,
        serialization: SerializationMethod,
        compression: CompressionMethod,
        sequence: Option<i32>,
        payload: &[u8],
    ) -> Vec<u8> {
        // 计算 header 大小（基础4字节 + 可选4字节sequence）
        let has_sequence = flags == MessageFlags::WithSeqNum || flags == MessageFlags::LastPacketWithSeq;
        let header_size = if has_sequence { 8 } else { 4 };
        let header_size_value = (header_size / 4) as u8; // header size value = 实际大小 / 4
        
        // 构建 header (4字节基础)
        // Byte 0: Protocol version (4 bits) + Header size (4 bits)
        let byte0 = (0x01 << 4) | (header_size_value & 0x0F);
        
        // Byte 1: Message type (4 bits) + Message flags (4 bits)
        let byte1 = ((msg_type as u8) << 4) | ((flags as u8) & 0x0F);
        
        // Byte 2: Serialization method (4 bits) + Compression (4 bits)
        let byte2 = ((serialization as u8) << 4) | ((compression as u8) & 0x0F);
        
        // Byte 3: Reserved
        let byte3 = 0x00;
        
        // 构建数据包
        let mut packet = Vec::with_capacity(header_size + 4 + payload.len());
        
        // Header bytes
        packet.push(byte0);
        packet.push(byte1);
        packet.push(byte2);
        packet.push(byte3);
        
        // Optional sequence number (4 bytes, big-endian)
        if let Some(seq) = sequence {
            packet.extend_from_slice(&seq.to_be_bytes());
        }
        
        // Payload size (4 bytes, big-endian)
        let payload_size = payload.len() as u32;
        packet.extend_from_slice(&payload_size.to_be_bytes());
        
        // Payload
        packet.extend_from_slice(payload);
        
        packet
    }
    
    /// 解析服务端响应
    pub fn parse_response(&self, data: &[u8]) -> crate::asr::ASRResult<ParsedResponse> {
        if data.len() < 8 {
            return Err(crate::asr::ASRError::Protocol("响应数据太短".to_string()));
        }
        
        // 解析 header
        let byte0 = data[0];
        let byte1 = data[1];
        let byte2 = data[2];
        let _byte3 = data[3];
        
        let _protocol_version = (byte0 >> 4) & 0x0F;
        let header_size_value = (byte0 & 0x0F) as usize;
        let header_size = header_size_value * 4;
        
        let msg_type_value = (byte1 >> 4) & 0x0F;
        let flags_value = byte1 & 0x0F;
        
        let serialization_value = (byte2 >> 4) & 0x0F;
        let _compression_value = byte2 & 0x0F;
        
        let msg_type = match msg_type_value {
            0x01 => MessageType::FullClientRequest,
            0x02 => MessageType::AudioOnlyRequest,
            0x09 => MessageType::FullServerResponse,
            0x0F => MessageType::ErrorResponse,
            _ => return Err(crate::asr::ASRError::Protocol(format!("未知消息类型: {}", msg_type_value))),
        };
        
        // 检查是否有 sequence number
        let has_sequence = flags_value == 0x01 || flags_value == 0x03;
        let is_last = flags_value == 0x02 || flags_value == 0x03;
        
        // 解析 sequence
        let mut offset = 4;
        let _sequence = if has_sequence {
            if data.len() < 8 {
                return Err(crate::asr::ASRError::Protocol("数据长度不足以包含sequence".to_string()));
            }
            let seq_bytes = &data[4..8];
            let seq = i32::from_be_bytes([seq_bytes[0], seq_bytes[1], seq_bytes[2], seq_bytes[3]]);
            offset = 8;
            Some(seq)
        } else {
            None
        };
        
        // 检查是否有 header 扩展
        if header_size > offset {
            offset = header_size;
        }
        
        // 解析 payload size
        if data.len() < offset + 4 {
            return Err(crate::asr::ASRError::Protocol("数据长度不足以包含payload size".to_string()));
        }
        let payload_size_bytes = &data[offset..offset+4];
        let payload_size = u32::from_be_bytes([
            payload_size_bytes[0], payload_size_bytes[1], 
            payload_size_bytes[2], payload_size_bytes[3]
        ]) as usize;
        
        offset += 4;
        
        // 提取 payload
        if data.len() < offset + payload_size {
            return Err(crate::asr::ASRError::Protocol(format!(
                "数据长度不足: 需要 {} 字节, 实际 {} 字节",
                offset + payload_size,
                data.len()
            )));
        }
        let payload = &data[offset..offset + payload_size];
        
        // 根据消息类型处理
        match msg_type {
            MessageType::FullServerResponse => {
                // 解析 JSON 响应
                if serialization_value == 0x01 && !payload.is_empty() {
                    let response: ServerResponse = serde_json::from_slice(payload)
                        .map_err(|e| crate::asr::ASRError::Protocol(format!("解析响应JSON失败: {}", e)))?;
                    
                    return Ok(ParsedResponse::ServerResponse { 
                        response, 
                        is_last 
                    });
                } else {
                    return Ok(ParsedResponse::ServerResponse { 
                        response: ServerResponse::default(), 
                        is_last 
                    });
                }
            }
            MessageType::ErrorResponse => {
                let error_msg = String::from_utf8_lossy(payload).to_string();
                return Err(crate::asr::ASRError::RecognitionFailed(format!("服务端错误: {}", error_msg)));
            }
            _ => {
                return Err(crate::asr::ASRError::Protocol(format!("意外的消息类型: {:?}", msg_type)));
            }
        }
    }
}

impl Default for DoubaoProtocol {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析后的响应
#[derive(Debug, Clone)]
pub enum ParsedResponse {
    ServerResponse {
        response: ServerResponse,
        is_last: bool,
    },
}

/// 请求 Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestConfig {
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_nonstream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_itn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_punc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_ddc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_utterances: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_speech_rate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_volume: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_lid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_emotion_detection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_gender_detection: Option<bool>,
}

/// 服务端响应
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reqid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<RecognitionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecognitionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utterances: Option<Vec<Utterance>>,
    #[serde(default)]
    pub definite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utterance {
    pub text: String,
    #[serde(default)]
    pub definite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additions: Option<serde_json::Value>,
}

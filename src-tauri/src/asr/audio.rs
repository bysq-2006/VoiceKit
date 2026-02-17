//! 音频录制和处理模块
//! 
//! 用于从麦克风录制音频，并转换为 ASR 服务需要的格式

use crate::asr::{ASRResult, ASRError};
use std::sync::{Arc, Mutex};

/// 音频录制器
pub struct AudioRecorder {
    /// 采样率
    sample_rate: u32,
    /// 声道数
    channels: u16,
    /// 录音数据缓冲区
    buffer: Arc<Mutex<Vec<i16>>>,
    /// 是否正在录音
    is_recording: Arc<Mutex<bool>>,
}

impl AudioRecorder {
    /// 创建新的音频录制器
    /// 
    /// # Arguments
    /// * `sample_rate` - 采样率（豆包 ASR 要求 16000）
    /// * `channels` - 声道数（豆包 ASR 要求 1）
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            buffer: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
        }
    }
    
    /// 开始录音
    pub fn start_recording(&self) -> ASRResult<()> {
        let mut is_recording = self.is_recording.lock().unwrap();
        if *is_recording {
            return Err(ASRError::Audio("已经在录音中".to_string()));
        }
        
        // 清空缓冲区
        let mut buffer = self.buffer.lock().unwrap();
        buffer.clear();
        
        *is_recording = true;
        
        // TODO: 使用 cpal 实际开始录音
        // 目前先返回成功，实际实现需要添加 cpal 的录音逻辑
        
        Ok(())
    }
    
    /// 停止录音
    /// 
    /// # Returns
    /// 录制的 PCM 音频数据（S16LE 格式）
    pub fn stop_recording(&self) -> ASRResult<Vec<u8>> {
        let mut is_recording = self.is_recording.lock().unwrap();
        if !*is_recording {
            return Err(ASRError::Audio("没有正在进行的录音".to_string()));
        }
        
        *is_recording = false;
        
        // 将 i16 数据转换为 u8 数据（小端序）
        let buffer = self.buffer.lock().unwrap();
        let mut pcm_data = Vec::with_capacity(buffer.len() * 2);
        
        for &sample in buffer.iter() {
            pcm_data.extend_from_slice(&sample.to_le_bytes());
        }
        
        Ok(pcm_data)
    }
    
    /// 获取当前录音状态
    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }
    
    /// 添加音频数据到缓冲区
    pub fn push_audio_data(&self, data: &[i16]) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(data);
    }
    
    /// 获取当前已录制的时长（秒）
    pub fn get_recorded_duration(&self) -> f32 {
        let buffer = self.buffer.lock().unwrap();
        let sample_count = buffer.len() as f32;
        let channel_count = self.channels as f32;
        let sample_rate = self.sample_rate as f32;
        
        sample_count / channel_count / sample_rate
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        // 默认配置：16kHz 采样率，单声道（符合豆包 ASR 要求）
        Self::new(16000, 1)
    }
}

/// 将 WAV 文件转换为 PCM S16LE 数据
/// 
/// # Arguments
/// * `wav_data` - WAV 文件数据
///
/// # Returns
/// PCM S16LE 数据（16kHz, 单声道）
pub fn wav_to_pcm(wav_data: &[u8]) -> ASRResult<Vec<u8>> {
    use hound::WavReader;
    use std::io::Cursor;
    
    let reader = WavReader::new(Cursor::new(wav_data))
        .map_err(|e| ASRError::Audio(format!("解析 WAV 失败: {}", e)))?;
    
    let spec = reader.spec();
    
    // 读取样本
    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();
    
    // 如果采样率不是 16000，需要重采样（简化版，使用线性插值）
    let resampled = if spec.sample_rate != 16000 {
        resample_linear(&samples, spec.sample_rate, 16000, spec.channels)
    } else {
        samples
    };
    
    // 如果是立体声，转换为单声道
    let mono = if spec.channels == 2 {
        stereo_to_mono(&resampled)
    } else {
        resampled
    };
    
    // 转换为 u8 数据
    let mut pcm_data = Vec::with_capacity(mono.len() * 2);
    for sample in mono {
        pcm_data.extend_from_slice(&sample.to_le_bytes());
    }
    
    Ok(pcm_data)
}

/// 线性插值重采样（简化版）
fn resample_linear(input: &[i16], from_rate: u32, to_rate: u32, channels: u16) -> Vec<i16> {
    if from_rate == to_rate {
        return input.to_vec();
    }
    
    let channels = channels as usize;
    let input_frames = input.len() / channels;
    let output_frames = (input_frames as f64 * to_rate as f64 / from_rate as f64) as usize;
    
    let mut output = Vec::with_capacity(output_frames * channels);
    
    for i in 0..output_frames {
        let src_pos = i as f64 * from_rate as f64 / to_rate as f64;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;
        
        for ch in 0..channels {
            let idx1 = (src_idx * channels + ch).min(input.len() - 1);
            let idx2 = ((src_idx + 1) * channels + ch).min(input.len() - 1);
            
            let sample1 = input[idx1] as f64;
            let sample2 = input[idx2] as f64;
            let sample = sample1 + frac * (sample2 - sample1);
            
            output.push(sample as i16);
        }
    }
    
    output
}

/// 立体声转单声道
fn stereo_to_mono(stereo: &[i16]) -> Vec<i16> {
    if stereo.len() % 2 != 0 {
        // 奇数个样本，去掉最后一个
        let _stereo = &stereo[..stereo.len() - 1];
    }
    
    stereo
        .chunks_exact(2)
        .map(|chunk| {
            let left = chunk[0] as i32;
            let right = chunk[1] as i32;
            ((left + right) / 2) as i16
        })
        .collect()
}

/// 将 PCM S16LE 数据保存为 WAV 文件
/// 
/// # Arguments
/// * `pcm_data` - PCM S16LE 数据
/// * `sample_rate` - 采样率
/// * `channels` - 声道数
///
/// # Returns
/// WAV 文件数据
pub fn pcm_to_wav(pcm_data: &[u8], sample_rate: u32, channels: u16) -> ASRResult<Vec<u8>> {
    use hound::WavSpec;
    use std::io::Cursor;
    
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| ASRError::Audio(format!("创建 WAV 写入器失败: {}", e)))?;
        
        // 将 u8 数据转换为 i16
        for chunk in pcm_data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            writer.write_sample(sample)
                .map_err(|e| ASRError::Audio(format!("写入样本失败: {}", e)))?;
        }
        
        writer.finalize()
            .map_err(|e| ASRError::Audio(format!("完成 WAV 写入失败: {}", e)))?;
    }
    
    Ok(cursor.into_inner())
}

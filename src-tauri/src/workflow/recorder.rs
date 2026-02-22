use crate::models::buffer::AudioBuffer;
use crate::models::state::AppState;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const TARGET_SAMPLE_RATE: u32 = 16000;

/// 重采样器：将输入采样率转换为16kHz
/// 使用简单下采样：每 3 个采样取平均（如 48kHz -> 16kHz）
struct Resampler;

impl Resampler {
    fn new(_input_rate: u32) -> Self {
        Self
    }

    /// 处理输入采样，返回16kHz的i16数据
    fn process(&mut self, input: &[f64]) -> Vec<i16> {
        // 每 3 个输入采样平均为 1 个输出采样（3:1 下采样）
        input.chunks_exact(3)
            .map(|chunk| {
                let avg = (chunk[0] + chunk[1] + chunk[2]) / 3.0;
                (avg * 32767.0).clamp(-32768.0, 32767.0) as i16
            })
            .collect()
    }
}

/// 音频录制器
pub struct AudioRecorder;

impl AudioRecorder {
    /// 启动录音监控线程
    pub fn start_monitoring(self: Arc<Self>, app_state: Arc<AppState>) {
        thread::spawn(move || {
            let mut was_recording = false;
            let mut current_stream: Option<Stream> = None;

            loop {
                let is_recording = *app_state.is_recording.lock().unwrap();

                if is_recording && !was_recording {
                    log::info!("开始录音");
                    app_state.audio_buffer.clear();
                    current_stream = Self::start_stream(app_state.audio_buffer.clone());
                    if current_stream.is_none() {
                        *app_state.is_recording.lock().unwrap() = false;
                    }
                    was_recording = true;
                } else if !is_recording && was_recording {
                    log::info!("停止录音");
                    drop(current_stream.take());
                    app_state.audio_buffer.finish();
                    was_recording = false;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });
    }

    /// 启动录音流
    fn start_stream(audio_buffer: Arc<AudioBuffer>) -> Option<Stream> {
        let host = cpal::default_host();
        let device = host.default_input_device()?;
        let config = device.default_input_config().ok()?;

        log::info!("录音设备: {:?}, 格式: {:?}", device.name(), config);
        log::info!("将重采样到: {}Hz 单声道", TARGET_SAMPLE_RATE);

        let stream = match config.sample_format() {
            SampleFormat::I16 => Self::build_stream::<i16>(&device, &config.into(), audio_buffer),
            SampleFormat::F32 => Self::build_stream::<f32>(&device, &config.into(), audio_buffer),
            _ => {
                log::error!("不支持的采样格式: {:?}", config.sample_format());
                return None;
            }
        };

        match stream {
            Ok(s) => {
                s.play().ok()?;
                Some(s)
            }
            Err(e) => {
                log::error!("创建录音流失败: {}", e);
                None
            }
        }
    }

    /// 构建录音流（统一处理i16和f32）
    fn build_stream<T>(
        device: &cpal::Device,
        config: &StreamConfig,
        audio_buffer: Arc<AudioBuffer>,
    ) -> Result<Stream, cpal::BuildStreamError>
    where
        T: Sample + SizedSample + Send + 'static,
        f64: From<T>,
    {
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;
        let resampler = Arc::new(std::sync::Mutex::new(Resampler::new(sample_rate)));

        device.build_input_stream(
            config,
            move |data: &[T], _| {
                // 混音 + 转换为f64
                let mono: Vec<f64> = data
                    .chunks(channels)
                    .map(|c| c.iter().map(|&s| f64::from(s)).sum::<f64>() / channels as f64)
                    .collect();

                // 重采样并写入
                let out = resampler.lock().unwrap().process(&mono);
                if !out.is_empty() {
                    audio_buffer.write(&out);
                }
            },
            |e| log::error!("录音错误: {}", e),
            None,
        )
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self
    }
}

pub fn init_recorder(app_state: Arc<AppState>) {
    Arc::new(AudioRecorder).start_monitoring(app_state);
    log::info!("录音监控已启动");
}

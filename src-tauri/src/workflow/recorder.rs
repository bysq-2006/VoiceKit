use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static IS_RECORDING: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref AUDIO_BUFFER: Arc<std::sync::Mutex<Vec<i16>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
}

pub fn start() {
    {
        let mut buffer = AUDIO_BUFFER.lock().unwrap();
        buffer.clear();
    }
    
    IS_RECORDING.store(true, Ordering::SeqCst);
    log::info!("开始录音");
    
    let buffer_clone = AUDIO_BUFFER.clone();
    std::thread::spawn(move || {
        if let Err(e) = record_blocking(buffer_clone) {
            log::error!("录音失败: {}", e);
        }
    });
}

pub fn stop() {
    IS_RECORDING.store(false, Ordering::SeqCst);
    log::info!("停止录音");
}

pub fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::SeqCst)
}

pub fn get_audio_data() -> Vec<i16> {
    AUDIO_BUFFER.lock().unwrap().clone()
}

pub fn clear_buffer() {
    AUDIO_BUFFER.lock().unwrap().clear();
}

fn record_blocking(audio_buffer: Arc<std::sync::Mutex<Vec<i16>>>) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or_else(|| "找不到默认输入设备".to_string())?;
    
    log::info!("使用输入设备: {:?}", device.name());
    
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };
    
    let stream = device.build_input_stream(
        &config,
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            if IS_RECORDING.load(Ordering::SeqCst) {
                if let Ok(mut buffer) = audio_buffer.lock() {
                    buffer.extend_from_slice(data);
                }
            }
        },
        move |err| {
            log::error!("音频输入错误: {}", err);
        },
        None,
    ).map_err(|e| format!("构建输入流失败: {}", e))?;
    
    stream.play()
        .map_err(|e| format!("开始录音失败: {}", e))?;
    
    log::info!("音频流已开始");
    
    while IS_RECORDING.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    drop(stream);
    
    let sample_count = AUDIO_BUFFER.lock().unwrap().len();
    log::info!("录音结束，缓冲 {} 样本 ({:.2}秒)", 
        sample_count, 
        sample_count as f32 / 16000.0
    );
    
    Ok(())
}

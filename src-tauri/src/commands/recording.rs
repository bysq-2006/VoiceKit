use tauri::{Emitter, Manager, State};
use crate::models::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static IS_RECORDING_FLAG: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref AUDIO_BUFFER: Arc<std::sync::Mutex<Vec<i16>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
}

#[tauri::command]
pub async fn toggle_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    let was_recording = {
        let is_recording = state.is_recording.lock().unwrap();
        *is_recording
    };
    
    if was_recording {
        stop_recording(app.clone()).await?;
    } else {
        start_recording(app.clone()).await?;
    }
    
    Ok(())
}

async fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    {
        let mut buffer = AUDIO_BUFFER.lock().unwrap();
        buffer.clear();
    }
    
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }
    IS_RECORDING_FLAG.store(true, Ordering::SeqCst);
    
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", true);
    }
    
    log::info!("开始录音");
    
    let buffer_clone = AUDIO_BUFFER.clone();
    std::thread::spawn(move || {
        if let Err(e) = record_audio_blocking(buffer_clone) {
            log::error!("录音失败: {}", e);
        }
    });
    
    Ok(())
}

fn record_audio_blocking(audio_buffer: Arc<std::sync::Mutex<Vec<i16>>>) -> Result<(), String> {
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
            if IS_RECORDING_FLAG.load(Ordering::SeqCst) {
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
    
    while IS_RECORDING_FLAG.load(Ordering::SeqCst) {
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

async fn stop_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
    }
    IS_RECORDING_FLAG.store(false, Ordering::SeqCst);
    
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", false);
    }
    
    log::info!("停止录音");
    
    Ok(())
}

#[tauri::command]
pub fn hide_and_stop_recording(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    
    let state = app.state::<AppState>();
    let mut is_recording = state.is_recording.lock().unwrap();
    
    if *is_recording {
        *is_recording = false;
        IS_RECORDING_FLAG.store(false, Ordering::SeqCst);
        drop(is_recording);
        
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", false);
        }
        
        log::info!("录音已取消");
    }
}

#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> bool {
    *state.is_recording.lock().unwrap()
}

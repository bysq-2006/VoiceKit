use tauri::{Emitter, Manager, State};
use crate::models::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// 全局录音控制
static IS_RECORDING_FLAG: AtomicBool = AtomicBool::new(false);

// 全局音频缓冲区 - 使用静态变量存储录音数据
lazy_static::lazy_static! {
    static ref AUDIO_BUFFER: Arc<std::sync::Mutex<Vec<i16>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
}

/// 切换录音状态（后端控制，前端只请求）
#[tauri::command]
pub async fn toggle_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    // 获取当前状态
    let was_recording = {
        let is_recording = state.is_recording.lock().unwrap();
        *is_recording
    };
    
    if was_recording {
        // 正在录音 -> 停止录音并开始识别
        stop_recording_and_recognize(app.clone()).await?;
    } else {
        // 未录音 -> 开始录音
        start_recording(app.clone()).await?;
    }
    
    Ok(())
}

/// 开始录音
async fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    // 检查 ASR 是否可用
    if !state.is_asr_available() {
        return Err("ASR 未配置，请先在设置中配置豆包 API".to_string());
    }
    
    // 清空音频缓冲区
    {
        let mut buffer = AUDIO_BUFFER.lock().unwrap();
        buffer.clear();
    }
    
    // 更新状态为录音中
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }
    IS_RECORDING_FLAG.store(true, Ordering::SeqCst);
    
    // 通知前端状态改变
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", true);
    }
    
    log::info!("开始录音");
    
    // 在独立线程中启动录音（因为 cpal::Stream 不是 Send）
    let buffer_clone = AUDIO_BUFFER.clone();
    std::thread::spawn(move || {
        if let Err(e) = record_audio_blocking(buffer_clone) {
            log::error!("录音失败: {}", e);
        }
    });
    
    Ok(())
}

/// 阻塞式录音（在独立线程中运行）
fn record_audio_blocking(audio_buffer: Arc<std::sync::Mutex<Vec<i16>>>) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    
    let host = cpal::default_host();
    
    // 获取默认输入设备
    let device = host.default_input_device()
        .ok_or_else(|| "找不到默认输入设备".to_string())?;
    
    log::info!("使用输入设备: {:?}", device.name());
    
    // 配置音频格式：16kHz, 单声道, i16
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // 构建输入流
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
    
    // 开始录音
    stream.play()
        .map_err(|e| format!("开始录音失败: {}", e))?;
    
    log::info!("音频流已开始");
    
    // 等待直到停止录音
    while IS_RECORDING_FLAG.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    // 停止录音
    drop(stream);
    
    let sample_count = AUDIO_BUFFER.lock().unwrap().len();
    log::info!("录音结束，缓冲 {} 样本 ({:.2}秒)", 
        sample_count, 
        sample_count as f32 / 16000.0
    );
    
    Ok(())
}

/// 停止录音并开始识别
async fn stop_recording_and_recognize(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    
    // 更新状态为停止
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
    }
    IS_RECORDING_FLAG.store(false, Ordering::SeqCst);
    
    // 通知前端状态改变
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("recording-state-changed", false);
    }
    
    log::info!("停止录音，开始识别");
    
    // 等待录音线程完成
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    
    // 获取录制的音频数据
    let audio_data: Vec<u8> = {
        let buffer = AUDIO_BUFFER.lock().unwrap();
        if buffer.is_empty() {
            log::warn!("没有录制到音频数据");
            hide_and_stop_recording(app.clone());
            return Ok(());
        }
        
        // 将 i16 转换为 u8 (小端序)
        let mut pcm_data = Vec::with_capacity(buffer.len() * 2);
        for &sample in buffer.iter() {
            pcm_data.extend_from_slice(&sample.to_le_bytes());
        }
        pcm_data
    };
    
    log::info!("录制了 {} 字节音频数据", audio_data.len());
    
    // 获取 ASR Provider
    let provider = state.get_asr_provider()
        .ok_or_else(|| "ASR Provider 未初始化".to_string())?;
    
    // 在后台执行识别，不阻塞主线程
    tokio::spawn(async move {
        // 执行识别
        match provider.recognize(audio_data).await {
            Ok(text) => {
                log::info!("识别结果: {}", text);
                
                // 发送识别结果到前端
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("recognition-result", text.clone());
                }
                
                // 模拟键盘输入识别结果
                if let Err(e) = simulate_input(&text).await {
                    log::error!("模拟输入失败: {}", e);
                }
            }
            Err(e) => {
                log::error!("识别失败: {}", e);
                
                // 发送错误到前端
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("recognition-error", e.to_string());
                }
            }
        }
        
        // 识别完成，只重置状态，不隐藏窗口
        let state = app.state::<AppState>();
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
        drop(is_recording);
        
        // 通知前端状态已重置
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", false);
        }
    });
    
    Ok(())
}

/// 模拟键盘输入
async fn simulate_input(text: &str) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};
    
    log::info!("模拟键盘输入: {}", text);
    
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("创建 Enigo 失败: {}", e))?;
    
    // 输入文本
    enigo.text(text)
        .map_err(|e| format!("输入文本失败: {}", e))?;
    
    Ok(())
}

/// 隐藏窗口并停止录音
#[tauri::command]
pub fn hide_and_stop_recording(app: tauri::AppHandle) {
    // 1. 隐藏窗口
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    
    // 2. 停止录音（如果正在录音）
    let state = app.state::<AppState>();
    let mut is_recording = state.is_recording.lock().unwrap();
    
    if *is_recording {
        *is_recording = false;
        IS_RECORDING_FLAG.store(false, Ordering::SeqCst);
        drop(is_recording); // 释放锁再发送事件
        
        // 通知主窗口前端录音已停止
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("recording-state-changed", false);
        }
        
        log::info!("录音已取消");
    }
}

/// 获取录音状态（前端初始化用）
#[tauri::command]
pub fn get_recording_state(state: State<AppState>) -> bool {
    *state.is_recording.lock().unwrap()
}

use crate::asr::manager::AsrProvider;
use crate::models::state::AppState;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// ASR 控制器
/// 
/// 监控录音状态，开始录音时启动 ASR，停止录音时停止 ASR
pub fn init_asr_controller(app_state: Arc<AppState>) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(run_asr_loop(app_state));
    });
    
    log::info!("ASR 控制器已启动");
}

async fn run_asr_loop(app_state: Arc<AppState>) {
    let mut was_recording = false;
    let mut current_provider: Option<AsrProvider> = None;
    
    loop {
        let is_recording = *app_state.is_recording.lock().unwrap();
        
        // 开始录音：启动 ASR
        if is_recording && !was_recording {
            log::info!("ASR: 检测到录音开始，启动 ASR");
            
            match app_state.asr_manager.create_provider() {
                Ok(provider) => {
                    if let Err(e) = provider.start().await {
                        log::error!("ASR 启动失败: {}", e);
                    } else {
                        current_provider = Some(provider);
                        log::info!("ASR 已启动");
                    }
                }
                Err(e) => {
                    log::error!("创建 ASR Provider 失败: {}", e);
                }
            }
            
            was_recording = true;
        }
        
        // 停止录音：停止 ASR
        if !is_recording && was_recording {
            log::info!("ASR: 检测到录音停止，停止 ASR");
            
            if let Some(provider) = current_provider.take() {
                provider.stop().await;
                // 等待 2 秒让服务端返回最后结果
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                log::info!("ASR 已停止");
            }
            
            was_recording = false;
        }
        
        thread::sleep(Duration::from_millis(50));
    }
}

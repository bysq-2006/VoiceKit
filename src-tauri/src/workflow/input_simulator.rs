use crate::models::state::AppState;
use enigo::{Enigo, Keyboard, Direction};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 输入模拟器
/// 持续从 TextBuffer 读取文字并模拟键盘输入，队列有内容就写，没有就等待
pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    /// 启动输入模拟线程
    pub fn start(self: Arc<Self>, app_state: Arc<AppState>) {
        thread::spawn(move || {
            let mut enigo = Enigo::new(&enigo::Settings::default())
                .expect("Failed to create Enigo instance");

            log::info!("输入模拟器已启动，等待 TextBuffer 数据...");

            // 持续从 TextBuffer 读取并输入
            loop {
                // 从队列读取文字（阻塞等待直到有数据）
                match app_state.text_buffer.read() {
                    Some(text) => {
                        log::info!("从 TextBuffer 读取到文字: {}", text);
                        self.type_text(&mut enigo, &text, &app_state);
                    }
                    None => {
                        // 队列已结束（finish 被调用），清空等待状态继续下一轮
                        log::debug!("TextBuffer 已 finish，继续等待新数据");
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        });
    }

    /// 模拟键盘输入文字
    fn type_text(&self, enigo: &mut Enigo, text: &str, app_state: &Arc<AppState>) {
        use std::sync::atomic::Ordering;
        // 设置模拟输入标志
        app_state.is_simulating_input.store(true, Ordering::SeqCst);
        
        for ch in text.chars() {
            // 处理换行符
            if ch == '\n' {
                if let Err(e) = enigo.key(enigo::Key::Return, Direction::Click) {
                    log::error!("输入换行符失败: {}", e);
                }
                continue;
            }
            
            // 处理普通字符
            if let Err(e) = enigo.text(&ch.to_string()) {
                log::error!("输入字符 '{}' 失败: {}", ch, e);
            }
            
            // 添加微小延迟，模拟真实输入
            thread::sleep(Duration::from_millis(10));
        }
        
        // 清除模拟输入标志
        app_state.is_simulating_input.store(false, Ordering::SeqCst);
        
        log::info!("输入完成: {}", text);
    }
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化输入模拟器
pub fn init_input_simulator(app_state: Arc<AppState>) {
    Arc::new(InputSimulator::new()).start(app_state);
    log::info!("输入模拟器模块已加载");
}

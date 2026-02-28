use crate::models::state::AppState;
use enigo::{Enigo, Keyboard, Direction};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    pub fn start(self: Arc<Self>, app_state: Arc<AppState>) {
        thread::spawn(move || {
            let mut enigo = Enigo::new(&enigo::Settings::default())
                .expect("Failed to create Enigo instance");

            loop {
                match app_state.text_buffer.read() {
                    Some(text) => self.type_text(&mut enigo, &text, &app_state),
                    None => thread::sleep(Duration::from_millis(50)),
                }
            }
        });
    }

    fn type_text(&self, enigo: &mut Enigo, text: &str, app_state: &Arc<AppState>) {
        use std::sync::atomic::Ordering;
        app_state.is_simulating_input.store(true, Ordering::SeqCst);

        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                // 批量退格标记：\x01 + 4字节大端count
                '\x01' if i + 4 < chars.len() => {
                    let count = ((chars[i + 1] as u32) << 24)
                        | ((chars[i + 2] as u32) << 16)
                        | ((chars[i + 3] as u32) << 8)
                        | (chars[i + 4] as u32);
                    for _ in 0..count {
                        let _ = enigo.key(enigo::Key::Backspace, Direction::Click);
                    }
                    i += 5;
                }
                // 换行
                '\n' => {
                    let _ = enigo.key(enigo::Key::Return, Direction::Click);
                    i += 1;
                }
                // 普通字符
                ch => {
                    let _ = enigo.text(&ch.to_string());
                    i += 1;
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }

        app_state.is_simulating_input.store(false, Ordering::SeqCst);
    }
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_input_simulator(app_state: Arc<AppState>) {
    Arc::new(InputSimulator::new()).start(app_state);
}

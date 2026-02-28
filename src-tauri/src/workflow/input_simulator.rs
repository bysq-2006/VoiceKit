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
                match app_state.text_buffer.pop() {
                    Some(ch) => self.process_char(&mut enigo, ch, &app_state),
                    None => break,
                }
            }
        });
    }

    fn process_char(&self, enigo: &mut Enigo, ch: char, app_state: &Arc<AppState>) {
        use std::sync::atomic::Ordering;
        app_state.is_simulating_input.store(true, Ordering::SeqCst);

        match ch {
            '\x08' => {
                // 退格键
                let _ = enigo.key(enigo::Key::Backspace, Direction::Click);
                thread::sleep(Duration::from_millis(20));
            }
            ch => {
                // 普通字符
                let _ = enigo.text(&ch.to_string());
                thread::sleep(Duration::from_millis(20));
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

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct AudioBuffer {
    data: Mutex<VecDeque<i16>>,
    cond: Condvar,
    is_finished: AtomicBool,
}

impl AudioBuffer {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
            is_finished: AtomicBool::new(false),
        }
    }

    pub fn write(&self, samples: &[i16]) {
        let mut data = self.data.lock().unwrap();
        data.extend(samples);
        self.cond.notify_one();
    }

    pub fn read(&self, buf: &mut [i16]) -> usize {
        let mut data = self.data.lock().unwrap();
        
        while data.is_empty() && !self.is_finished.load(Ordering::SeqCst) {
            data = self.cond.wait(data).unwrap();
        }
        
        let mut count = 0;
        for slot in buf.iter_mut() {
            if let Some(sample) = data.pop_front() {
                *slot = sample;
                count += 1;
            } else {
                break;
            }
        }
        
        count
    }

    pub fn finish(&self) {
        self.is_finished.store(true, Ordering::SeqCst);
        self.cond.notify_all();
    }

    pub fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        data.clear();
        self.is_finished.store(false, Ordering::SeqCst);
    }

    pub fn is_finished(&self) -> bool {
        self.is_finished.load(Ordering::SeqCst)
    }

    /// 获取当前缓冲区中的采样点数量
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AudioBuffer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextBuffer {
    segments: Mutex<VecDeque<String>>,
    cond: Condvar,
    is_finished: AtomicBool,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            segments: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
            is_finished: AtomicBool::new(false),
        }
    }

    pub fn write(&self, text: String) {
        let mut segments = self.segments.lock().unwrap();
        segments.push_back(text);
        self.cond.notify_one();
    }

    /// 批量发送退格键
    /// 格式: \x01 + 4字节大端count
    pub fn send_backspaces(&self, count: usize) {
        if count == 0 { return; }
        let mut data = String::new();
        data.push('\x01');
        data.push(((count >> 24) & 0xFF) as u8 as char);
        data.push(((count >> 16) & 0xFF) as u8 as char);
        data.push(((count >> 8) & 0xFF) as u8 as char);
        data.push((count & 0xFF) as u8 as char);
        self.write(data);
    }

    pub fn read(&self) -> Option<String> {
        let mut segments = self.segments.lock().unwrap();
        
        loop {
            if let Some(text) = segments.pop_front() {
                return Some(text);
            }
            
            if self.is_finished.load(Ordering::SeqCst) {
                return None;
            }
            
            segments = self.cond.wait(segments).unwrap();
        }
    }

    pub fn finish(&self) {
        self.is_finished.store(true, Ordering::SeqCst);
        self.cond.notify_all();
    }

    pub fn clear(&self) {
        let mut segments = self.segments.lock().unwrap();
        segments.clear();
        self.is_finished.store(false, Ordering::SeqCst);
    }

    pub fn is_finished(&self) -> bool {
        self.is_finished.load(Ordering::SeqCst)
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

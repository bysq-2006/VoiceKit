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

    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AudioBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// 文本输入队列 - 每个字符是一个独立的队列元素
/// 特殊字符：\x08 (ASCII退格) 表示退格，'\n' 表示回车
pub struct TextBuffer {
    chars: Mutex<VecDeque<char>>,
    cond: Condvar,
    is_finished: AtomicBool,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            chars: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
            is_finished: AtomicBool::new(false),
        }
    }

    /// 添加普通文本（每个字符作为一个独立元素）
    pub fn push_text(&self, text: &str) {
        let mut chars = self.chars.lock().unwrap();
        for ch in text.chars() {
            chars.push_back(ch);
        }
        self.cond.notify_one();
    }

    /// 添加退格键（count 个独立的 '\b' 字符）
    pub fn push_backspaces(&self, count: usize) {
        if count == 0 { return; }
        let mut chars = self.chars.lock().unwrap();
        for _ in 0..count {
            chars.push_back('\x08');
        }
        self.cond.notify_one();
    }

    /// 从队列取出一个字符（阻塞等待）
    pub fn pop(&self) -> Option<char> {
        let mut chars = self.chars.lock().unwrap();
        
        loop {
            if let Some(ch) = chars.pop_front() {
                return Some(ch);
            }
            
            if self.is_finished.load(Ordering::SeqCst) {
                return None;
            }
            
            chars = self.cond.wait(chars).unwrap();
        }
    }

    pub fn finish(&self) {
        self.is_finished.store(true, Ordering::SeqCst);
        self.cond.notify_all();
    }

    pub fn clear(&self) {
        let mut chars = self.chars.lock().unwrap();
        chars.clear();
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

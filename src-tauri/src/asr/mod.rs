//! ASR 模块

pub mod manager;
mod providers;

pub use manager::{init_asr_manager, AsrManager, AsrProvider};

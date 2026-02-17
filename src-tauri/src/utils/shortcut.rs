//! 快捷键解析工具

use tauri_plugin_global_shortcut::{Code, Modifiers, GlobalShortcutExt};

/// 注册快捷键（先注销所有，再注册新的）
pub fn update_shortcut<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>, 
    shortcut_str: &str
) -> Result<(), String> {
    // 1. 注销所有已注册的快捷键
    let manager = app.global_shortcut();
    manager.unregister_all()
        .map_err(|e| format!("注销快捷键失败: {}", e))?;
    
    // 2. 解析新的快捷键
    let shortcut = parse_shortcut(shortcut_str)?;
    
    // 3. 注册新的快捷键
    manager.register(shortcut)
        .map_err(|e| format!("注册快捷键失败: {}", e))?;
    
    Ok(())
}

/// 初始化时注册快捷键（setup 中使用）
pub fn init_shortcut<R: tauri::Runtime>(
    app: &tauri::App<R>, 
    shortcut_str: &str
) -> Result<(), String> {
    let shortcut = parse_shortcut(shortcut_str)?;
    app.global_shortcut().register(shortcut)
        .map_err(|e| format!("初始化快捷键失败: {}", e))?;
    Ok(())
}

/// 将字符串解析为快捷键
/// 支持格式: "Ctrl+Shift+A", "Alt+F1", "Shift+Space", "Cmd+K" 等
pub fn parse_shortcut(s: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    
    if parts.len() < 2 {
        return Err("快捷键必须包含至少一个修饰键和主键".to_string());
    }
    
    // 解析修饰键（可能多个）
    let mut modifiers = Modifiers::empty();
    let main_key = parts.last().unwrap();
    
    for part in &parts[..parts.len()-1] {
        match *part {
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Ctrl" | "Control" => modifiers |= Modifiers::CONTROL,
            "Alt" => modifiers |= Modifiers::ALT,
            "Cmd" | "Command" | "Meta" | "Super" => modifiers |= Modifiers::META,
            _ => return Err(format!("未知的修饰键: {}", part)),
        }
    }
    
    if modifiers.is_empty() {
        return Err("快捷键必须包含至少一个修饰键".to_string());
    }
    
    // 解析主键
    let code = parse_key_code(main_key)?;
    
    Ok(tauri_plugin_global_shortcut::Shortcut::new(
        Some(modifiers),
        code,
    ))
}

/// 解析主键字符串为 Code
pub fn parse_key_code(key: &str) -> Result<Code, String> {
    // 字母键 A-Z
    if key.len() == 1 {
        let ch = key.chars().next().unwrap().to_ascii_uppercase();
        if ch >= 'A' && ch <= 'Z' {
            return match ch {
                'A' => Ok(Code::KeyA),
                'B' => Ok(Code::KeyB),
                'C' => Ok(Code::KeyC),
                'D' => Ok(Code::KeyD),
                'E' => Ok(Code::KeyE),
                'F' => Ok(Code::KeyF),
                'G' => Ok(Code::KeyG),
                'H' => Ok(Code::KeyH),
                'I' => Ok(Code::KeyI),
                'J' => Ok(Code::KeyJ),
                'K' => Ok(Code::KeyK),
                'L' => Ok(Code::KeyL),
                'M' => Ok(Code::KeyM),
                'N' => Ok(Code::KeyN),
                'O' => Ok(Code::KeyO),
                'P' => Ok(Code::KeyP),
                'Q' => Ok(Code::KeyQ),
                'R' => Ok(Code::KeyR),
                'S' => Ok(Code::KeyS),
                'T' => Ok(Code::KeyT),
                'U' => Ok(Code::KeyU),
                'V' => Ok(Code::KeyV),
                'W' => Ok(Code::KeyW),
                'X' => Ok(Code::KeyX),
                'Y' => Ok(Code::KeyY),
                'Z' => Ok(Code::KeyZ),
                _ => unreachable!(),
            };
        }
        // 数字键 0-9
        if ch >= '0' && ch <= '9' {
            return match ch {
                '0' => Ok(Code::Digit0),
                '1' => Ok(Code::Digit1),
                '2' => Ok(Code::Digit2),
                '3' => Ok(Code::Digit3),
                '4' => Ok(Code::Digit4),
                '5' => Ok(Code::Digit5),
                '6' => Ok(Code::Digit6),
                '7' => Ok(Code::Digit7),
                '8' => Ok(Code::Digit8),
                '9' => Ok(Code::Digit9),
                _ => unreachable!(),
            };
        }
    }
    
    // 特殊键
    match key {
        "Space" | " " => Ok(Code::Space),
        "Enter" | "Return" => Ok(Code::Enter),
        "Tab" => Ok(Code::Tab),
        "Escape" | "Esc" => Ok(Code::Escape),
        "Backspace" => Ok(Code::Backspace),
        "Delete" | "Del" => Ok(Code::Delete),
        "Insert" | "Ins" => Ok(Code::Insert),
        "Home" => Ok(Code::Home),
        "End" => Ok(Code::End),
        "PageUp" => Ok(Code::PageUp),
        "PageDown" => Ok(Code::PageDown),
        "Up" | "ArrowUp" => Ok(Code::ArrowUp),
        "Down" | "ArrowDown" => Ok(Code::ArrowDown),
        "Left" | "ArrowLeft" => Ok(Code::ArrowLeft),
        "Right" | "ArrowRight" => Ok(Code::ArrowRight),
        // 功能键 F1-F24
        "F1" => Ok(Code::F1),
        "F2" => Ok(Code::F2),
        "F3" => Ok(Code::F3),
        "F4" => Ok(Code::F4),
        "F5" => Ok(Code::F5),
        "F6" => Ok(Code::F6),
        "F7" => Ok(Code::F7),
        "F8" => Ok(Code::F8),
        "F9" => Ok(Code::F9),
        "F10" => Ok(Code::F10),
        "F11" => Ok(Code::F11),
        "F12" => Ok(Code::F12),
        "F13" => Ok(Code::F13),
        "F14" => Ok(Code::F14),
        "F15" => Ok(Code::F15),
        "F16" => Ok(Code::F16),
        "F17" => Ok(Code::F17),
        "F18" => Ok(Code::F18),
        "F19" => Ok(Code::F19),
        "F20" => Ok(Code::F20),
        "F21" => Ok(Code::F21),
        "F22" => Ok(Code::F22),
        "F23" => Ok(Code::F23),
        "F24" => Ok(Code::F24),
        // 符号键
        "Minus" | "-" => Ok(Code::Minus),
        "Equal" | "=" => Ok(Code::Equal),
        "BracketLeft" | "[" => Ok(Code::BracketLeft),
        "BracketRight" | "]" => Ok(Code::BracketRight),
        "Semicolon" | ";" => Ok(Code::Semicolon),
        "Quote" | "'" => Ok(Code::Quote),
        "Backslash" | "\\" => Ok(Code::Backslash),
        "Comma" | "," => Ok(Code::Comma),
        "Period" | "." => Ok(Code::Period),
        "Slash" | "/" => Ok(Code::Slash),
        "Backquote" | "`" | "~" => Ok(Code::Backquote),
        _ => Err(format!("不支持的按键: {}", key)),
    }
}

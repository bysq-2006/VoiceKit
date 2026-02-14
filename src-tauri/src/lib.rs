// 窗口控制命令
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState, GlobalShortcutExt};

/// 显示窗口
#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _: Result<(), _> = window.show();
        let _: Result<(), _> = window.set_focus();
    }
}

/// 隐藏窗口（程序继续运行）
#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _: Result<(), _> = window.hide();
    }
}

/// 退出程序
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            show_window,
            hide_window,
            quit_app
        ])
        .setup(|app| {
            // 注册 Shift+E 快捷键
            let shortcut = tauri_plugin_global_shortcut::Shortcut::new(
                Some(Modifiers::SHIFT),
                Code::KeyE,
            );
            app.global_shortcut().register(shortcut)?;
            
            // 设置系统托盘
            setup_tray(app)?;
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 设置系统托盘
fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    
    // 创建菜单项
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    
    // 创建菜单
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ]
    )?;
    
    // 创建托盘图标
    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("语音助手")
        .build(app)?;
    
    // 处理托盘菜单事件
    tray.on_menu_event(|app: &tauri::AppHandle, event| {
        match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        }
    });
    
    // 点击托盘图标显示窗口
    tray.on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
        if let TrayIconEvent::Click { .. } = event {
            let app = tray.app_handle();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });
    
    Ok(())
}

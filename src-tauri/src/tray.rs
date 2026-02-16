use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use crate::commands::window;

/// 设置系统托盘
pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
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
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)?;

    // 处理托盘菜单事件
    tray.on_menu_event(|app: &tauri::AppHandle, event| {
        match event.id.as_ref() {
            "show" => {
                window::show_window(app.clone());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        }
    });

    // 点击托盘图标处理：左键显示窗口，右键弹出菜单
    tray.on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
        match event {
            // 左键点击：显示窗口
            TrayIconEvent::Click { button: MouseButton::Left, .. } => {
                window::show_window(tray.app_handle().clone());
            }
            // 右键点击：弹出菜单（由 Tauri 自动处理）
            _ => {}
        }
    });

    Ok(())
}

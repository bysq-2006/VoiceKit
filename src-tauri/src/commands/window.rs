use tauri::Manager;

/// 显示窗口
#[tauri::command]
pub fn show_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _: Result<(), _> = window.show();
        let _: Result<(), _> = window.set_focus();
    }
}

/// 退出程序
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

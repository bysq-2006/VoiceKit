use tauri::{AppHandle, Manager};

/// 显示窗口但不激活
pub fn show_no_activate(app: &AppHandle, label: &str) {
    let Some(window) = app.get_webview_window(label) else { return };
    
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST, 
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_SHOWWINDOW
        };
        
        let hwnd = windows::Win32::Foundation::HWND(window.hwnd().unwrap().0 as *mut _);
        
        // 先取消置顶（清理状态）
        let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, 
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        
        // 置顶显示但不激活
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, 
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    
    #[cfg(not(windows))]
    {
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
}

/// 隐藏窗口
pub fn hide(app: &AppHandle, label: &str) {
    let Some(window) = app.get_webview_window(label) else { return };
    
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_NOTOPMOST, 
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_HIDEWINDOW
        };
        
        let hwnd = windows::Win32::Foundation::HWND(window.hwnd().unwrap().0 as *mut _);
        
        let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, 
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW);
    }
    
    #[cfg(not(windows))]
    {
        let _ = window.set_always_on_top(false);
        let _ = window.hide();
    }
}

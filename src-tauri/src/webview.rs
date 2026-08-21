use tauri::WebviewWindow;

#[derive(Clone, Copy)]
pub enum MemoryUsageLevel {
    Normal,
    Low,
}

#[cfg(windows)]
pub fn set_webview_memory_level(window: &WebviewWindow, level: MemoryUsageLevel) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows_core::Interface;

    let _ = window.with_webview(move |webview| unsafe {
        if let Ok(core) = webview.controller().CoreWebView2() {
            if let Ok(webview19) = core.cast::<ICoreWebView2_19>() {
                let target = match level {
                    MemoryUsageLevel::Normal => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
                    MemoryUsageLevel::Low => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
                };
                let _ = webview19.SetMemoryUsageTargetLevel(target);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn set_webview_memory_level(_window: &WebviewWindow, _level: MemoryUsageLevel) {}

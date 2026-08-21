use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Manager, WindowEvent};

use crate::tray::setup_tray;
use crate::webview::{set_webview_memory_level, MemoryUsageLevel};

/// Shared flag marking that the window visibility has been handled by the
/// user (close-to-tray) or by the frontend show flow, so the 1.5s fallback
/// must not override it.
pub struct WindowHandledFlag(pub Arc<AtomicBool>);

#[tauri::command]
pub fn mark_window_handled(flag: tauri::State<'_, WindowHandledFlag>) {
    flag.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Hides the main window and creates the tray icon so the user can bring it
/// back. The tray icon only exists while the window is hidden.
#[tauri::command]
pub fn hide_to_tray(app: tauri::AppHandle, flag: tauri::State<'_, WindowHandledFlag>) {
    flag.0.store(true, Ordering::SeqCst);
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.hide();
    set_webview_memory_level(&window, MemoryUsageLevel::Low);
    if app.tray_by_id("main").is_none() {
        let _ = setup_tray(&app);
    }
}

/// Removes the tray icon once the main window is visible again.
#[tauri::command]
pub fn restore_from_tray(app: tauri::AppHandle) {
    app.remove_tray_by_id("main");
}

pub fn setup_close_handling(
    app: &tauri::App,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(window) = app.get_webview_window("main") {
        let target = window.clone();
        window.on_window_event(move |event| match event {
            // Closing the window terminates the app; hiding to the tray is
            // an explicit user action handled by the `hide_to_tray` command.
            WindowEvent::Focused(focused) => {
                set_webview_memory_level(
                    &target,
                    if *focused {
                        MemoryUsageLevel::Normal
                    } else {
                        MemoryUsageLevel::Low
                    },
                );
            }
            _ => {}
        });
    }
    Ok(())
}

pub fn ensure_window_shown(app: &tauri::App, handled: Arc<AtomicBool>) {
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        // Skip the forced show if the user or the frontend already handled
        // the window visibility (e.g. close-to-tray within the first 1.5s).
        if handled.load(Ordering::SeqCst) {
            return;
        }
        if let Some(window) = handle.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                return;
            }
            let _ = window.show();
            let _ = window.set_focus();
        }
    });
}

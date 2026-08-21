mod settings;
mod templates;
mod tray;
mod webview;
mod window;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Manager;

use settings::{load_settings, save_settings, get_data_dir, set_data_dir};
use templates::{
    create_template, delete_templates, get_template_dir, import_code_template,
    import_parameter_template, list_templates, open_in_explorer, update_template,
};
use window::{
    exit_app, hide_to_tray, mark_window_handled, restore_from_tray,
    setup_close_handling, ensure_window_shown, WindowHandledFlag,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let window_handled = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            mark_window_handled,
            exit_app,
            hide_to_tray,
            restore_from_tray,
            load_settings,
            save_settings,
            get_data_dir,
            set_data_dir,
            list_templates,
            create_template,
            update_template,
            delete_templates,
            get_template_dir,
            open_in_explorer,
            import_code_template,
            import_parameter_template
        ])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.manage(WindowHandledFlag(window_handled.clone()));

            // No tray icon at startup: it is created on demand when the
            // window is hidden to the tray.
            setup_close_handling(app)?;
            ensure_window_shown(app, window_handled.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

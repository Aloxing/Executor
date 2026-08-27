mod android_projects;
mod common;
pub mod core;
mod imports;
mod templates;
mod workspace;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Manager;

use core::settings::{load_settings, save_settings, get_data_dir, set_data_dir};
use core::window::{
    exit_app, hide_to_tray, mark_window_handled, restore_from_tray,
    setup_close_handling, ensure_window_shown, WindowHandledFlag,
};
use android_projects::{
    add_android_project, delete_android_project, delete_android_projects,
    detach_android_project, get_android_project_dir, import_android_projects,
    list_android_projects, reload_android_project, update_android_project,
};
use imports::{create_import_queue, delete_queues, list_import_queues};
use templates::{
    create_template, delete_templates, get_template_dir, import_code_template,
    import_parameter_template, list_templates, open_in_explorer, read_parameter_json,
    update_template, write_parameter_json,
};
use workspace::ensure_workspace_areas;

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
            import_parameter_template,
            read_parameter_json,
            write_parameter_json,
            list_import_queues,
            create_import_queue,
            delete_queues,
            list_android_projects,
            add_android_project,
            update_android_project,
            get_android_project_dir,
            reload_android_project,
            delete_android_project,
            delete_android_projects,
            detach_android_project,
            import_android_projects,
            ensure_workspace_areas
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

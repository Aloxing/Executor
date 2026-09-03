mod android_projects;
mod builds;
mod common;
pub mod core;
mod configs;
mod devices;
mod imports;
mod outputs;
mod records;
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
use core::webview::disable_browser_accelerator_keys;
use android_projects::{
    add_android_project, delete_android_project, delete_android_projects,
    detach_android_project, get_android_project_dir, import_android_project,
    import_android_projects, list_android_projects, reload_android_project,
    update_android_project,
};
use builds::{
    add_build_project, create_build_queue, delete_build_queues, get_build_logs_dir,
    list_build_queues, remove_build_project, run_project_build, stop_project_build, BuildRegistry,
};
use outputs::{copy_output_file, list_outputs, remove_output_file, remove_outputs};
use devices::{
    list_android_devices, start_device_logcat, stop_device_logcat, DeviceRegistry,
};
use records::{list_records, remove_record_item, remove_records};
use configs::{
    add_config_project, create_config_queue, delete_config_projects, delete_config_queues,
    execute_config_project, list_config_queues, read_project_parameter,
    record_all_config_projects, record_config_project, refresh_project_parameter,
    reload_config_project, remove_config_project, reset_project_code, save_config_template,
    start_config_project, update_config_project, write_project_parameter,
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
            import_android_project,
            list_config_queues,
            create_config_queue,
            add_config_project,
            remove_config_project,
            save_config_template,
            start_config_project,
            delete_config_queues,
            delete_config_projects,
            update_config_project,
            reload_config_project,
            record_config_project,
            record_all_config_projects,
            read_project_parameter,
            write_project_parameter,
            refresh_project_parameter,
            execute_config_project,
            reset_project_code,
            list_build_queues,
            create_build_queue,
            add_build_project,
            remove_build_project,
            delete_build_queues,
            run_project_build,
            stop_project_build,
            get_build_logs_dir,
            list_outputs,
            remove_outputs,
            remove_output_file,
            copy_output_file,
            list_android_devices,
            start_device_logcat,
            stop_device_logcat,
            list_records,
            remove_records,
            remove_record_item,
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
            // Live build process ids, used by the stop-build action.
            app.manage(BuildRegistry::default());
            // Live adb logcat process ids, used by the device-log stop.
            app.manage(DeviceRegistry::default());

            // No tray icon at startup: it is created on demand when the
            // window is hidden to the tray.
            setup_close_handling(app)?;
            // The app owns Ctrl+F / Ctrl+S etc. through its own shortcut
            // system, so the WebView2 browser accelerators must be off.
            if let Some(window) = app.get_webview_window("main") {
                disable_browser_accelerator_keys(&window);
            }
            ensure_window_shown(app, window_handled.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

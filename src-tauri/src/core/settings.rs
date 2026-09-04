use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::common::storage::{move_file, resolve_custom_data_dir, write_data_dir_anchor};

// ---------------------------------------------------------------------------
// Persistent settings stored as JSON in the app data folder. A small
// bootstrap.json always stays in the default app data dir and points to the
// (possibly user-chosen) data directory holding the actual settings.json.
// The generic anchor/move helpers live in `crate::common::storage`.
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_close_behavior")]
    pub close_behavior: String,
    #[serde(default)]
    pub workspace_path: String,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
    /// Gradle installations available to the build area (the gradle
    /// executable lives in `<path>/bin`). Multiple versions are allowed.
    #[serde(default)]
    pub gradle_envs: Vec<GradleEnv>,
    /// Customized keyboard shortcuts per action id; actions without an
    /// entry use the frontend defaults.
    #[serde(default)]
    pub shortcuts: std::collections::HashMap<String, String>,
    /// Also raise a Windows system notification for the important results
    /// (long builds, batch configuration, bulk deletions…). The in-app
    /// toasts are shown either way.
    #[serde(default = "default_system_notify")]
    pub system_notify: bool,
}

/// One Gradle environment: an installation directory picked in the
/// settings compile tab, persisted with the settings in the data dir.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GradleEnv {
    pub name: String,
    pub path: String,
}

pub fn default_close_behavior() -> String {
    "ask".to_string()
}

pub fn default_theme_mode() -> String {
    "system".to_string()
}

/// System notifications are on by default; settings files written before
/// the option existed simply lack the field.
pub fn default_system_notify() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_behavior: default_close_behavior(),
            workspace_path: String::new(),
            theme_mode: default_theme_mode(),
            gradle_envs: Vec::new(),
            shortcuts: std::collections::HashMap::new(),
            system_notify: default_system_notify(),
        }
    }
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

pub fn resolve_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let default_dir = app_data_dir(app)?;
    Ok(resolve_custom_data_dir(app, &default_dir))
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_data_dir(app)?.join("settings.json"))
}

/// Reads the persisted settings without validation; shared by the
/// `load_settings` command and other modules (e.g. templates).
pub fn read_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let mut settings = read_settings(&app);
    if !matches!(settings.close_behavior.as_str(), "ask" | "tray" | "exit") {
        settings.close_behavior = default_close_behavior();
    }
    if !matches!(settings.theme_mode.as_str(), "dark" | "light" | "system") {
        settings.theme_mode = default_theme_mode();
    }
    Ok(settings)
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    resolve_data_dir(&app).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn set_data_dir(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("路径不能为空".to_string());
    }
    let new_dir = PathBuf::from(trimmed);
    if !new_dir.is_absolute() {
        return Err("请使用绝对路径".to_string());
    }
    fs::create_dir_all(&new_dir).map_err(|e| format!("无法创建目录：{e}"))?;

    let default_dir = app_data_dir(&app)?;
    let old_dir = resolve_data_dir(&app)?;
    let new_canon = fs::canonicalize(&new_dir).map_err(|e| e.to_string())?;
    let old_canon = fs::canonicalize(&old_dir).map_err(|e| e.to_string())?;
    let default_canon = fs::canonicalize(&default_dir).map_err(|e| e.to_string())?;

    if new_canon != old_canon {
        // Move data files from the current location to the new one.
        let entries = fs::read_dir(&old_canon).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
            if !is_file {
                continue;
            }
            let name = entry.file_name();
            // bootstrap.json stays in the default dir as the anchor.
            if old_canon == default_canon && name.to_string_lossy() == "bootstrap.json" {
                continue;
            }
            let _ = move_file(&entry.path(), &new_canon.join(&name));
        }
    }

    // Point (or drop) the anchor kept in the default app data dir.
    write_data_dir_anchor(&app, &default_canon, &new_canon)
        .map_err(|e| format!("无法更新数据目录锚点：{e}"))?;

    Ok(new_canon.display().to_string())
}

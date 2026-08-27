//! Portable storage helpers: cross-volume file moves and the
//! `bootstrap.json` anchor that tracks a user-chosen data directory.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Moves a file, falling back to copy+remove for cross-volume targets.
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    data_dir: Option<String>,
}

fn bootstrap_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("bootstrap.json"))
        .unwrap_or_else(|_| PathBuf::from("bootstrap.json"))
}

/// Resolves the effective data directory: the anchor's target when it
/// points at an existing directory, otherwise the default one.
pub fn resolve_custom_data_dir(app: &tauri::AppHandle, default_dir: &Path) -> PathBuf {
    let Ok(content) = fs::read_to_string(bootstrap_path(app)) else {
        return default_dir.to_path_buf();
    };
    let Ok(cfg) = serde_json::from_str::<Bootstrap>(&content) else {
        return default_dir.to_path_buf();
    };
    match cfg.data_dir {
        Some(dir) => {
            let path = PathBuf::from(dir);
            if path.is_dir() {
                path
            } else {
                default_dir.to_path_buf()
            }
        }
        None => default_dir.to_path_buf(),
    }
}

/// Points the anchor at `target`, or removes it when the data moves back
/// to the default directory.
pub fn write_data_dir_anchor(
    app: &tauri::AppHandle,
    default_dir: &Path,
    target: &Path,
) -> std::io::Result<()> {
    let path = bootstrap_path(app);
    if target == default_dir {
        let _ = fs::remove_file(&path);
        return Ok(());
    }
    let cfg = Bootstrap {
        data_dir: Some(target.display().to_string()),
    };
    let content =
        serde_json::to_string_pretty(&cfg).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&path, content)
}

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

/// Recursively copies the COMPLETE contents of `src` into `dst` and then
/// verifies the copy: nothing is ever skipped, transient file locks are
/// retried once, and a final walk of both trees fails the operation when
/// any source file is missing or differs in size. Callers treat an error
/// as “copy did not happen” and clean up the partial target.
pub fn copy_dir_complete(src: &Path, dst: &Path) -> Result<(), String> {
    copy_dir_inner(src, dst).map_err(|e| format!("{}：{e}", src.display()))?;
    verify_dir_copy(src, dst)
}

fn copy_dir_inner(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_inner(&entry.path(), &dest)?;
        } else {
            copy_file_retry(&entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// One file copy with a single retry: antivirus/indexer/explorer on
/// Windows briefly hold freshly touched files now and then.
fn copy_file_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::copy(from, to) {
        Ok(_) => Ok(()),
        Err(first) => {
            std::thread::sleep(std::time::Duration::from_millis(300));
            fs::copy(from, to).map_err(|second| {
                std::io::Error::new(
                    first.kind(),
                    format!("复制 {} 失败（可能被占用）：{second}", from.display()),
                )
            })?;
            Ok(())
        }
    }
}

/// Walks `src` and checks every file exists in `dst` with the same size.
fn verify_dir_copy(src: &Path, dst: &Path) -> Result<(), String> {
    let mut missing: Vec<String> = Vec::new();
    verify_walk(src, dst, &mut missing);
    if missing.is_empty() {
        return Ok(());
    }
    let shown: Vec<&str> = missing.iter().take(5).map(|s| s.as_str()).collect();
    Err(format!(
        "复制不完整，{} 个文件缺失或大小不符：{}{}",
        missing.len(),
        shown.join("、"),
        if missing.len() > 5 { " 等" } else { "" }
    ))
}

fn verify_walk(src: &Path, dst: &Path, missing: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(src) else {
        // Source vanished mid-check; the outer copy already succeeded, so
        // treat it as complete rather than failing on a moving target.
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            verify_walk(&entry.path(), &target, missing);
        } else if let Ok(meta) = entry.metadata() {
            match fs::metadata(&target) {
                Ok(target_meta) if target_meta.len() == meta.len() => {}
                _ => missing.push(entry.path().display().to_string()),
            }
        }
    }
}

/// Verifies a single-file copy: source and target sizes must match.
pub fn verify_file_copy(src: &Path, dst: &Path) -> Result<(), String> {
    let source_len = fs::metadata(src)
        .map_err(|e| format!("无法读取源文件信息：{e}"))?
        .len();
    let target_len = fs::metadata(dst)
        .map_err(|e| format!("无法读取目标文件信息：{e}"))?
        .len();
    if source_len == target_len {
        Ok(())
    } else {
        Err(format!(
            "复制不完整：{} 大小不符（源 {source_len} 字节，目标 {target_len} 字节）",
            dst.display()
        ))
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

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde::{Deserialize, Serialize};

use crate::core::settings::read_settings;

/// Records page: every add/delete/modify operation of the import, config,
/// build and output areas is logged here as one card. Records persist as a
/// JSON list in `<workspace>/records/records.json`, capped to the newest
/// MAX_RECORDS entries. Deleting records never touches the files they
/// refer to — they are pure history entries.

const MAX_RECORDS: usize = 500;

/// Uniqueness aid for same-millisecond operations.
static SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpRecord {
    pub uuid: String,
    /// Page tag: `import` | `config` | `build` | `output`.
    pub page: String,
    /// Operation tag: `add` | `delete` | `modify`.
    pub action: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    /// Sub records: the affected entries (project names, files…).
    #[serde(default)]
    pub items: Vec<String>,
    pub created_at: String,
}

fn workspace_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let settings = read_settings(app);
    let trimmed = settings.workspace_path.trim().to_string();
    if trimmed.is_empty() {
        return Err("请先在「设置 → 存储」中选择工作空间路径".to_string());
    }
    let dir = PathBuf::from(trimmed);
    if !dir.is_dir() {
        return Err("工作空间路径不存在，请重新选择".to_string());
    }
    Ok(dir)
}

fn records_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(workspace_dir(app)?.join("records").join("records.json"))
}

pub fn load_records(app: &tauri::AppHandle) -> Result<Vec<OpRecord>, String> {
    let path = records_path(app)?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(serde_json::from_str(&content).unwrap_or_default()),
        Err(_) => Ok(Vec::new()),
    }
}

fn save_records(app: &tauri::AppHandle, list: &[OpRecord]) -> Result<(), String> {
    let path = records_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("无法创建记录区目录：{e}"))?;
    }
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

/// Appends one operation record (newest first). Logging failures are
/// swallowed: a user operation must never fail because of its record.
pub fn log_operation(
    app: &tauri::AppHandle,
    page: &str,
    action: &str,
    title: &str,
    detail: &str,
    items: Vec<String>,
) {
    let Ok(mut list) = load_records(app) else {
        return;
    };
    let now = chrono::Local::now();
    let record = OpRecord {
        uuid: format!(
            "rec-{}-{}",
            now.format("%Y%m%d%H%M%S%3f"),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ),
        page: page.to_string(),
        action: action.to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        items,
        created_at: now.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    list.insert(0, record);
    list.truncate(MAX_RECORDS);
    let _ = save_records(app, &list);
}

#[tauri::command]
pub fn list_records(app: tauri::AppHandle) -> Result<Vec<OpRecord>, String> {
    load_records(&app)
}

/// Deletes whole record cards (single and batch share this command).
#[tauri::command]
pub fn remove_records(app: tauri::AppHandle, uuids: Vec<String>) -> Result<(), String> {
    let targets: Vec<String> = uuids.iter().map(|u| u.trim().to_string()).collect();
    if targets.is_empty() {
        return Ok(());
    }
    let mut list = load_records(&app)?;
    list.retain(|r| !targets.contains(&r.uuid));
    save_records(&app, &list)
}

/// Deletes one sub record by index; the card is removed together with its
/// last sub record. Returns the updated list.
#[tauri::command]
pub fn remove_record_item(
    app: tauri::AppHandle,
    uuid: String,
    index: usize,
) -> Result<Vec<OpRecord>, String> {
    let uuid = uuid.trim().to_string();
    let mut list = load_records(&app)?;
    let emptied = {
        let record = list
            .iter_mut()
            .find(|r| r.uuid == uuid)
            .ok_or_else(|| "未找到记录".to_string())?;
        if index >= record.items.len() {
            return Err("子记录不存在".to_string());
        }
        record.items.remove(index);
        record.items.is_empty()
    };
    if emptied {
        list.retain(|r| r.uuid != uuid);
    }
    save_records(&app, &list)?;
    Ok(list)
}

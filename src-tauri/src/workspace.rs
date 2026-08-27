use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::core::settings::read_settings;

/// One workspace area: a page-owned folder under the workspace root,
/// tracked in `<workspace>/workspace.json` for in-app persistence.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceArea {
    pub name: String,
    pub label: String,
    pub folder: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WorkspaceMeta {
    #[serde(default)]
    areas: Vec<WorkspaceArea>,
}

/// Page areas that get a dedicated folder in the workspace. The folder
/// names are the pages' English names.
const AREAS: &[(&str, &str, &str)] = &[
    ("import", "导入区", "import"),
    ("config", "配置区", "config"),
    ("build", "构建区", "build"),
    ("output", "产出区", "output"),
    ("records", "记录", "records"),
    ("templates", "模板", "templates"),
];

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

fn meta_path(dir: &PathBuf) -> PathBuf {
    dir.join("workspace.json")
}

fn load_meta(dir: &PathBuf) -> WorkspaceMeta {
    fs::read_to_string(meta_path(dir))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Creates the area folders for every page (import/config/build/output/
/// records/templates) inside the workspace and persists the structure to
/// `<workspace>/workspace.json`. Existing `createdAt` values are kept, so
/// repeated calls only repair missing folders.
#[tauri::command]
pub fn ensure_workspace_areas(
    app: tauri::AppHandle,
    now: String,
) -> Result<Vec<WorkspaceArea>, String> {
    let dir = workspace_dir(&app)?;
    let existing = load_meta(&dir);

    let mut areas = Vec::with_capacity(AREAS.len());
    for (name, label, folder) in AREAS {
        fs::create_dir_all(dir.join(folder))
            .map_err(|e| format!("无法创建「{label}」文件夹：{e}"))?;
        let created_at = existing
            .areas
            .iter()
            .find(|a| a.name == *name)
            .map(|a| a.created_at.clone())
            .unwrap_or_else(|| now.clone());
        areas.push(WorkspaceArea {
            name: (*name).to_string(),
            label: (*label).to_string(),
            folder: (*folder).to_string(),
            created_at,
        });
    }

    let meta = WorkspaceMeta {
        areas: areas.clone(),
    };
    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(meta_path(&dir), content).map_err(|e| format!("无法写入工作空间元数据：{e}"))?;
    Ok(areas)
}

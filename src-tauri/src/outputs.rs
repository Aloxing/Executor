use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::builds::BuildProject;
use crate::core::settings::read_settings;

/// Output area: every successful build records its artifacts here as one
/// card. Records persist as a JSON list in `<workspace>/output/outputs.json`;
/// the artifact files themselves stay where the build produced them and are
/// only deleted through the explicit delete actions (真删除).

/// One artifact file of a successful build.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutputFile {
    pub name: String,
    pub path: String,
}

/// One output card: the artifacts of one successful build plus the project
/// info fetched at collection time.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutputRecord {
    pub uuid: String,
    pub project_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Template name shown as the card tag; absent for direct disk builds
    /// without config-area info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    /// Where the project info came from: `config` or `build`.
    pub info_source: String,
    /// Build type; only Android for now (artifacts = apk files).
    pub build_type: String,
    /// Project directory the build ran in.
    pub root_path: String,
    pub files: Vec<OutputFile>,
    /// When the artifacts were recorded (build success time).
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

fn outputs_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(workspace_dir(app)?.join("output").join("outputs.json"))
}

pub fn load_records(app: &tauri::AppHandle) -> Result<Vec<OutputRecord>, String> {
    let path = outputs_path(app)?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(serde_json::from_str(&content).unwrap_or_default()),
        Err(_) => Ok(Vec::new()),
    }
}

fn save_records(app: &tauri::AppHandle, list: &[OutputRecord]) -> Result<(), String> {
    let path = outputs_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("无法创建产出区目录：{e}"))?;
    }
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_outputs(app: tauri::AppHandle) -> Result<Vec<OutputRecord>, String> {
    load_records(&app)
}

/// Deletes output cards (single and batch share this): the artifact files
/// are really deleted from disk, missing files are tolerated.
#[tauri::command]
pub fn remove_outputs(app: tauri::AppHandle, uuids: Vec<String>) -> Result<(), String> {
    let targets: Vec<String> = uuids.iter().map(|u| u.trim().to_string()).collect();
    if targets.is_empty() {
        return Ok(());
    }
    let mut list = load_records(&app)?;
    let removed_names: Vec<String> = list
        .iter()
        .filter(|r| targets.contains(&r.uuid))
        .map(|r| r.project_name.clone())
        .collect();
    for record in list.iter().filter(|r| targets.contains(&r.uuid)) {
        delete_files(&record.files);
    }
    list.retain(|r| !targets.contains(&r.uuid));
    save_records(&app, &list)?;
    crate::records::log_operation(
        &app,
        "output",
        "delete",
        "删除产出卡片",
        "产出文件已真删除",
        removed_names,
    );
    Ok(())
}

/// Deletes one artifact file of a card; the card is removed too when its
/// last file is gone. Returns the updated list.
#[tauri::command]
pub fn remove_output_file(
    app: tauri::AppHandle,
    uuid: String,
    file_path: String,
) -> Result<Vec<OutputRecord>, String> {
    let uuid = uuid.trim().to_string();
    let file_path = file_path.trim().to_string();
    let mut list = load_records(&app)?;
    let record = list
        .iter_mut()
        .find(|r| r.uuid == uuid)
        .ok_or_else(|| "未找到产出记录".to_string())?;
    let Some(file) = record.files.iter().find(|f| f.path == file_path) else {
        return Err("未找到要删除的产出文件".to_string());
    };
    let file_name = file.name.clone();
    delete_files(&[OutputFile {
        name: String::new(),
        path: file_path.clone(),
    }]);
    record.files.retain(|f| f.path != file_path);
    if record.files.is_empty() {
        list.retain(|r| r.uuid != uuid);
    }
    save_records(&app, &list)?;
    crate::records::log_operation(
        &app,
        "output",
        "delete",
        "删除产出文件",
        "文件已真删除",
        vec![file_name],
    );
    Ok(list)
}

/// Copies one artifact file to a destination chosen in the frontend.
#[tauri::command]
pub fn copy_output_file(app: tauri::AppHandle, src: String, dest: String) -> Result<(), String> {
    let src = PathBuf::from(src.trim());
    let dest = PathBuf::from(dest.trim());
    if !src.is_file() {
        return Err("产出文件不存在，可能已被移动或删除".to_string());
    }
    if dest.as_os_str().is_empty() {
        return Err("请选择复制目标".to_string());
    }
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("无法创建目标目录：{e}"))?;
    }
    fs::copy(&src, &dest).map_err(|e| format!("复制文件失败：{e}"))?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    crate::records::log_operation(
        &app,
        "output",
        "modify",
        "复制产出文件",
        &format!("目标：{}", dest.display()),
        vec![name],
    );
    Ok(())
}

/// Deletes artifact files from disk; missing files are tolerated so stale
/// records can always be cleaned up.
fn delete_files(files: &[OutputFile]) {
    for file in files {
        let path = Path::new(file.path.trim());
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
    }
}

/// Records the artifacts of one successful build and returns their count.
///
/// Project info is fetched from the config area (matched by root path or
/// package name); direct disk builds fall back to the build record itself.
/// Imported config projects keep their artifacts in `<package folder>/output`;
/// everything else is scanned recursively for the build type's artifacts
/// (Android: `*.apk`).
pub fn collect_build_outputs(
    app: &tauri::AppHandle,
    project: &BuildProject,
) -> Result<usize, String> {
    let root = PathBuf::from(project.root_path.trim());

    // Project info: config area first, then the build record.
    let mut project_name = project.name.clone();
    let mut package_name = project.package_name.clone();
    let mut template_name: Option<String> = None;
    let mut info_source = "build".to_string();
    let mut imported = false;
    if let Ok(config_queues) = crate::configs::load_queues(app) {
        let root_str = project.root_path.trim();
        let matched = config_queues
            .iter()
            .flat_map(|q| q.projects.iter())
            .find(|p| {
                (!root_str.is_empty() && p.root_path.trim() == root_str)
                    || (package_name.is_some() && p.package_name == package_name)
            });
        if let Some(cfg) = matched {
            project_name = cfg.name.clone();
            if package_name.is_none() {
                package_name = cfg.package_name.clone();
            }
            template_name = cfg.template_name.clone();
            info_source = "config".to_string();
            imported = cfg.source == "imported";
        }
    }

    // Artifact detection.
    let mut files = if imported {
        collect_files_in(&root.join("output"))
    } else {
        let mut found = Vec::new();
        scan_apks(&root, &mut found);
        found.sort_by(|a, b| a.path.cmp(&b.path));
        found
    };
    // De-duplicate by path (defensive; scans never overlap).
    files.dedup_by(|a, b| a.path == b.path);
    if files.is_empty() {
        return Ok(0);
    }

    let now = chrono::Local::now();
    let record = OutputRecord {
        uuid: format!("out-{}", now.format("%Y%m%d%H%M%S%3f")),
        project_name,
        package_name,
        template_name,
        info_source,
        build_type: "android".to_string(),
        root_path: project.root_path.clone(),
        files,
        created_at: now.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let count = record.files.len();
    let names: Vec<String> = record.files.iter().map(|f| f.name.clone()).collect();
    let project_name = record.project_name.clone();

    let mut list = load_records(app)?;
    // Same-package records are overwritten instead of appended: repeated
    // builds of one project must never pile up duplicate cards. Records
    // without a package name (direct disk builds) match by their project
    // directory instead.
    list.retain(|r| {
        let pkg_match = match (&record.package_name, &r.package_name) {
            (Some(a), Some(b)) => !a.is_empty() && a == b,
            _ => false,
        };
        let path_match =
            !record.root_path.trim().is_empty() && r.root_path == record.root_path;
        !(pkg_match || path_match)
    });
    // Newest cards first.
    list.insert(0, record);
    save_records(app, &list)?;
    crate::records::log_operation(
        app,
        "output",
        "add",
        "记录产出物",
        &format!("项目：{project_name}"),
        names,
    );
    Ok(count)
}

/// Collects every file under `dir` (recursively); a missing directory
/// yields an empty list.
fn collect_files_in(dir: &Path) -> Vec<OutputFile> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files_in(&path));
        } else if let Some(name) = path.file_name() {
            files.push(OutputFile {
                name: name.to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }
    files
}

/// Recursively scans for Android artifacts (`*.apk`), skipping only the
/// `.git` directory for speed.
fn scan_apks(dir: &Path, out: &mut Vec<OutputFile>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name() else { continue };
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == ".git" {
                continue;
            }
            scan_apks(&path, out);
        } else if name.to_lowercase().ends_with(".apk") {
            out.push(OutputFile {
                name: name.to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }
}

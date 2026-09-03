use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::imports::{load_queues, save_queues};

/// An Android project attached to an import queue. Records persist in
/// `<workspace>/import/android.json`; the imported contents are copied to
/// `<workspace>/import/package/<package_name>/`. The owning queue keeps the
/// package name in its `packages` field (see `crate::imports`).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AndroidProject {
    pub app_name: String,
    pub package_name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    /// One of `pending` (未导入), `importing` (导入中) or `imported` (已导入).
    pub import_status: String,
    pub queue_uuid: String,
    /// Imported location (`<workspace>/import/package/<package name>`),
    /// computed on read for display only; never persisted to android.json.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

fn workspace_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let settings = crate::core::settings::read_settings(app);
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

fn import_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(workspace_dir(app)?.join("import"))
}

fn android_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(import_dir(app)?.join("android.json"))
}

fn package_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(import_dir(app)?.join("package"))
}

pub fn load_projects(app: &tauri::AppHandle) -> Result<Vec<AndroidProject>, String> {
    let path = android_path(app)?;
    let mut list: Vec<AndroidProject> = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    // Attach the display-only imported location of every existing folder.
    if let Ok(base) = package_dir(app) {
        for project in list.iter_mut() {
            let dir = base.join(&project.package_name);
            if dir.is_dir() {
                project.location = Some(dir.display().to_string());
            }
        }
    }
    Ok(list)
}

pub fn save_projects(app: &tauri::AppHandle, list: &[AndroidProject]) -> Result<(), String> {
    let dir = import_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建导入区目录：{e}"))?;
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(android_path(app)?, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_android_projects(app: tauri::AppHandle) -> Result<Vec<AndroidProject>, String> {
    load_projects(&app)
}

/// Records a new Android project under a queue. Only the information is
/// stored at this point; files are copied on `import_android_projects` (or
/// when the root path changes through `update_android_project`).
#[tauri::command]
pub fn add_android_project(
    app: tauri::AppHandle,
    queue_uuid: String,
    app_name: String,
    package_name: String,
    root_path: String,
    created_at: String,
    updated_at: String,
) -> Result<AndroidProject, String> {
    let queue_uuid = queue_uuid.trim().to_string();
    let app_name = app_name.trim().to_string();
    let package_name = package_name.trim().to_string();
    let root_path = root_path.trim().to_string();
    if app_name.is_empty() {
        return Err("应用名称不能为空".to_string());
    }
    if package_name.is_empty() {
        return Err("应用包名不能为空".to_string());
    }
    if root_path.is_empty() {
        return Err("下载路径不能为空".to_string());
    }

    let mut queues = load_queues(&app)?;
    let Some(queue) = queues.iter_mut().find(|q| q.uuid == queue_uuid) else {
        return Err("队列不存在".to_string());
    };

    let mut projects = load_projects(&app)?;
    if projects.iter().any(|p| p.package_name == package_name) {
        return Err(format!("应用包名「{package_name}」已存在"));
    }

    let project = AndroidProject {
        app_name,
        package_name: package_name.clone(),
        root_path,
        created_at,
        updated_at,
        import_status: "pending".to_string(),
        queue_uuid,
        location: None,
    };
    projects.push(project.clone());
    save_projects(&app, &projects)?;

    // Track the package name inside the owning queue record.
    queue.packages.push(package_name);
    save_queues(&app, &queues)?;
    crate::records::log_operation(
        &app,
        "import",
        "add",
        "添加项目",
        &format!("包名：{}", project.package_name),
        vec![project.app_name.clone()],
    );
    Ok(project)
}

/// Updates an Android project identified by its original package name. The
/// package name itself may also change: the `package/<name>` folder is
/// renamed and every queue reference is updated. When the root path
/// changes, the package folder is cleared and the new source directory is
/// copied in immediately. The copy runs on the async thread pool so the UI
/// stays responsive for large projects.
#[tauri::command]
pub async fn update_android_project(
    app: tauri::AppHandle,
    package_name: String,
    new_package_name: String,
    app_name: String,
    root_path: String,
    updated_at: String,
) -> Result<AndroidProject, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_update(
            &app,
            &package_name,
            &new_package_name,
            &app_name,
            &root_path,
            &updated_at,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

fn run_update(
    app: &tauri::AppHandle,
    package_name: &str,
    new_package_name: &str,
    app_name: &str,
    root_path: &str,
    updated_at: &str,
) -> Result<AndroidProject, String> {
    let package_name = package_name.trim().to_string();
    let new_package_name = new_package_name.trim().to_string();
    let app_name = app_name.trim().to_string();
    let root_path = root_path.trim().to_string();
    if app_name.is_empty() {
        return Err("应用名称不能为空".to_string());
    }
    if new_package_name.is_empty() {
        return Err("应用包名不能为空".to_string());
    }
    if root_path.is_empty() {
        return Err("项目根目录路径不能为空".to_string());
    }

    let mut projects = load_projects(app)?;
    let Some(index) = projects.iter().position(|p| p.package_name == package_name) else {
        return Err(format!("应用包名「{package_name}」不存在"));
    };
    if new_package_name != package_name
        && projects.iter().any(|p| p.package_name == new_package_name)
    {
        return Err(format!("应用包名「{new_package_name}」已存在"));
    }

    if projects[index].root_path != root_path {
        let source = PathBuf::from(&root_path);
        if !source.is_dir() {
            return Err("所选下载路径不存在".to_string());
        }
        let target = package_dir(app)?.join(&package_name);
        clear_dir(&target).map_err(|e| format!("无法清空原有内容：{e}"))?;
        copy_dir_all(&source, &target).map_err(|e| format!("复制文件失败：{e}"))?;
    }

    // Package rename: move the on-disk folder and update queue references.
    if new_package_name != package_name {
        let base = package_dir(app)?;
        let old_dir = base.join(&package_name);
        if old_dir.is_dir() {
            fs::rename(&old_dir, base.join(&new_package_name))
                .map_err(|e| format!("无法重命名包名文件夹：{e}"))?;
        }
        let mut queues = load_queues(app)?;
        for queue in queues.iter_mut() {
            if let Some(pos) = queue.packages.iter().position(|p| *p == package_name) {
                queue.packages[pos] = new_package_name.clone();
            }
        }
        save_queues(app, &queues)?;
    }

    projects[index].package_name = new_package_name;
    projects[index].app_name = app_name;
    projects[index].root_path = root_path;
    projects[index].updated_at = updated_at.to_string();
    let updated = projects[index].clone();
    save_projects(app, &projects)?;
    crate::records::log_operation(
        app,
        "import",
        "modify",
        "修改项目",
        &format!("包名：{}", updated.package_name),
        vec![updated.app_name.clone()],
    );
    // Config cards of this package follow the rename (unrecorded ones).
    if updated.package_name != package_name {
        crate::configs::sync_import_rename(app, &package_name, &updated.package_name);
    }
    Ok(updated)
}

/// Deletes the imported contents of a project and re-imports it from the
/// recorded download path. Runs on the async thread pool so the UI stays
/// responsive for large projects.
#[tauri::command]
pub async fn reload_android_project(
    app: tauri::AppHandle,
    package_name: String,
) -> Result<AndroidProject, String> {
    tauri::async_runtime::spawn_blocking(move || run_reload(&app, &package_name))
        .await
        .map_err(|e| e.to_string())?
}

fn run_reload(app: &tauri::AppHandle, package_name: &str) -> Result<AndroidProject, String> {
    let package_name = package_name.trim().to_string();
    let mut projects = load_projects(app)?;
    let Some(index) = projects.iter().position(|p| p.package_name == package_name) else {
        return Err(format!("应用包名「{package_name}」不存在"));
    };
    let source = PathBuf::from(projects[index].root_path.trim());
    if !source.is_dir() {
        return Err("下载路径不存在，请检查后重试".to_string());
    }
    let target = package_dir(app)?.join(&package_name);
    clear_dir(&target).map_err(|e| format!("无法清空已导入内容：{e}"))?;
    copy_dir_all(&source, &target).map_err(|e| format!("重新导入失败：{e}"))?;
    projects[index].import_status = "imported".to_string();
    let updated = projects[index].clone();
    save_projects(app, &projects)?;
    crate::records::log_operation(
        app,
        "import",
        "modify",
        "重新导入项目",
        "已清空导入内容并从下载路径重新复制",
        vec![updated.app_name.clone()],
    );
    Ok(updated)
}

/// Returns the on-disk location of an imported project
/// (`<workspace>/import/package/<package name>`) for locating it in the
/// file manager.
#[tauri::command]
pub fn get_android_project_dir(
    app: tauri::AppHandle,
    package_name: String,
) -> Result<String, String> {
    let dir = package_dir(&app)?.join(package_name.trim());
    if !dir.is_dir() {
        return Err("项目尚未导入或文件夹不存在".to_string());
    }
    Ok(dir.display().to_string())
}

/// Deletes one or more Android projects by package name (batch deletion).
/// Removes the records, strips the package names from their queues and
/// deletes the on-disk package folders.
#[tauri::command]
pub fn delete_android_projects(
    app: tauri::AppHandle,
    package_names: Vec<String>,
) -> Result<(), String> {
    let targets: std::collections::HashSet<String> =
        package_names.iter().map(|n| n.trim().to_string()).collect();
    if targets.is_empty() {
        return Ok(());
    }
    let mut projects = load_projects(&app)?;
    let removed: Vec<String> = projects
        .iter()
        .filter(|p| targets.contains(&p.package_name))
        .map(|p| p.package_name.clone())
        .collect();
    if removed.is_empty() {
        return Err("未找到要删除的项目".to_string());
    }
    projects.retain(|p| !targets.contains(&p.package_name));
    save_projects(&app, &projects)?;

    // Strip the package names from every queue record.
    let mut queues = load_queues(&app)?;
    for queue in queues.iter_mut() {
        queue.packages.retain(|p| !targets.contains(p));
    }
    save_queues(&app, &queues)?;

    // Cleanup of the package folders (retry once in case a process such as
    // Explorer briefly holds a directory).
    let base = package_dir(&app)?;
    let mut failed: Vec<String> = Vec::new();
    // Kept for the config-area cascade after the record was written.
    let removed_pkgs = removed.clone();
    for name in &removed {
        let dir = base.join(name);
        if !dir.is_dir() {
            continue;
        }
        if fs::remove_dir_all(&dir).is_err() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if fs::remove_dir_all(&dir).is_err() {
                failed.push(dir.display().to_string());
            }
        }
    }
    crate::records::log_operation(
        &app,
        "import",
        "delete",
        "批量删除项目",
        "记录与导入文件夹已删除",
        removed,
    );
    // Unrecorded config cards of these packages lost their data source.
    crate::configs::drop_unrecorded_by_packages(&app, &removed_pkgs);
    if !failed.is_empty() {
        return Err(format!(
            "记录已删除，但文件夹删除失败（可能被占用）：{}",
            failed.join("、")
        ));
    }
    Ok(())
}

/// Detaches an Android project from its queue without deleting anything:
/// the record stays in `android.json` (queue uuid cleared) and the package
/// folder is kept, so the project remains visible in the import directory.
#[tauri::command]
pub fn detach_android_project(app: tauri::AppHandle, package_name: String) -> Result<(), String> {
    let package_name = package_name.trim().to_string();
    let mut projects = load_projects(&app)?;
    let Some(index) = projects.iter().position(|p| p.package_name == package_name) else {
        return Err(format!("应用包名「{package_name}」不存在"));
    };
    let queue_uuid = std::mem::take(&mut projects[index].queue_uuid);
    save_projects(&app, &projects)?;

    // Drop the package name from the owning queue record.
    if !queue_uuid.is_empty() {
        let mut queues = load_queues(&app)?;
        if let Some(queue) = queues.iter_mut().find(|q| q.uuid == queue_uuid) {
            queue.packages.retain(|p| *p != package_name);
            save_queues(&app, &queues)?;
        }
    }
    crate::records::log_operation(
        &app,
        "import",
        "modify",
        "从队列移除项目",
        "记录与导入文件保留，仅解除队列关联",
        vec![package_name],
    );
    Ok(())
}

/// Deletes an Android project record, removes the package name from its
/// queue and deletes the on-disk package folder.
#[tauri::command]
pub fn delete_android_project(app: tauri::AppHandle, package_name: String) -> Result<(), String> {
    let package_name = package_name.trim().to_string();
    let mut projects = load_projects(&app)?;
    let Some(index) = projects.iter().position(|p| p.package_name == package_name) else {
        return Err(format!("应用包名「{package_name}」不存在"));
    };
    let queue_uuid = projects[index].queue_uuid.clone();
    projects.remove(index);
    save_projects(&app, &projects)?;

    // Drop the package name from the owning queue record.
    let mut queues = load_queues(&app)?;
    if let Some(queue) = queues.iter_mut().find(|q| q.uuid == queue_uuid) {
        queue.packages.retain(|p| *p != package_name);
        save_queues(&app, &queues)?;
    }
    crate::records::log_operation(
        &app,
        "import",
        "delete",
        "删除项目",
        "记录与导入文件夹已删除",
        vec![package_name.clone()],
    );
    // Unrecorded config cards of this package lost their data source.
    crate::configs::drop_unrecorded_by_packages(&app, &[package_name.clone()]);

    // Cleanup of the package folder (retry once in case a process such as
    // Explorer briefly holds the directory).
    let dir = package_dir(&app)?.join(&package_name);
    if dir.is_dir() {
        if fs::remove_dir_all(&dir).is_err() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if fs::remove_dir_all(&dir).is_err() {
                return Err(format!(
                    "记录已删除，但文件夹删除失败（可能被占用）：{}",
                    dir.display()
                ));
            }
        }
    }
    Ok(())
}

/// Copies the recorded root directory of every Android project in the queue
/// into `<workspace>/import/package/<package_name>/` and marks each project
/// as imported. Returns the queue's projects with their new statuses. Runs
/// on the async thread pool so the UI stays responsive for large projects.
#[tauri::command]
pub async fn import_android_projects(
    app: tauri::AppHandle,
    queue_uuid: String,
) -> Result<Vec<AndroidProject>, String> {
    tauri::async_runtime::spawn_blocking(move || run_import(&app, &queue_uuid))
        .await
        .map_err(|e| e.to_string())?
}

fn run_import(app: &tauri::AppHandle, queue_uuid: &str) -> Result<Vec<AndroidProject>, String> {
    let queue_uuid = queue_uuid.trim().to_string();
    let mut projects = load_projects(app)?;
    let indices: Vec<usize> = projects
        .iter()
        .enumerate()
        .filter(|(_, p)| p.queue_uuid == queue_uuid)
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        return Err("该队列下没有可导入的 Android 项目".to_string());
    }

    let mut failed: Vec<String> = Vec::new();
    let recorded: Vec<String> = indices
        .iter()
        .map(|&i| projects[i].package_name.clone())
        .collect();
    for index in indices {
        let source = PathBuf::from(projects[index].root_path.trim());
        let package_name = projects[index].package_name.clone();
        if !source.is_dir() {
            failed.push(format!("{package_name}（下载路径不存在）"));
            continue;
        }
        let target = package_dir(app)?.join(&package_name);
        let result = clear_dir(&target)
            .and_then(|()| fs::create_dir_all(&target))
            .and_then(|()| copy_dir_all(&source, &target));
        if result.is_err() {
            failed.push(package_name);
            continue;
        }
        projects[index].import_status = "imported".to_string();
    }
    save_projects(app, &projects)?;

    crate::records::log_operation(
        app,
        "import",
        "modify",
        "记录全部项目",
        "队列下的项目已复制到导入目录",
        recorded,
    );
    if !failed.is_empty() {
        return Err(format!("部分项目导入失败：{}", failed.join("、")));
    }
    Ok(projects
        .iter()
        .filter(|p| p.queue_uuid == queue_uuid)
        .cloned()
        .collect())
}

/// Copies the recorded root directory of a single Android project into
/// `<workspace>/import/package/<package_name>/` and marks it as imported.
/// Same behavior as the queue-wide import, scoped to one project. Runs on
/// the async thread pool so the UI stays responsive for large projects.
#[tauri::command]
pub async fn import_android_project(
    app: tauri::AppHandle,
    package_name: String,
) -> Result<AndroidProject, String> {
    tauri::async_runtime::spawn_blocking(move || run_import_one(&app, &package_name))
        .await
        .map_err(|e| e.to_string())?
}

fn run_import_one(app: &tauri::AppHandle, package_name: &str) -> Result<AndroidProject, String> {
    let package_name = package_name.trim().to_string();
    let mut projects = load_projects(app)?;
    let index = projects
        .iter()
        .position(|p| p.package_name == package_name)
        .ok_or_else(|| format!("未找到包名为「{package_name}」的项目"))?;
    let source = PathBuf::from(projects[index].root_path.trim());
    if !source.is_dir() {
        return Err(format!("项目「{package_name}」的下载路径不存在，无法导入"));
    }
    let target = package_dir(app)?.join(&package_name);
    clear_dir(&target)
        .and_then(|()| fs::create_dir_all(&target))
        .and_then(|()| copy_dir_all(&source, &target))
        .map_err(|e| format!("导入项目「{package_name}」失败：{e}"))?;
    projects[index].import_status = "imported".to_string();
    save_projects(app, &projects)?;
    crate::records::log_operation(
        app,
        "import",
        "modify",
        "记录项目",
        "项目已复制到导入目录",
        vec![package_name],
    );
    Ok(projects[index].clone())
}

/// Removes every entry inside `dir` without deleting `dir` itself.
fn clear_dir(dir: &Path) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Recursively copies the contents of `src` into `dst`.
///
/// Gradle-regenerated directories (`.gradle`, `build`, `.kotlin`) are
/// skipped: they are locked while a build is running — which used to make
/// the whole copy fail — and they are large, regenerable caches/artifacts.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            let name = entry.file_name();
            if matches!(name.to_string_lossy().as_ref(), ".gradle" | "build" | ".kotlin") {
                continue;
            }
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

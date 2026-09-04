use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::core::settings::read_settings;

/// Live build child-process ids, keyed by project uuid, so a running
/// build can be stopped (whole process tree) from the frontend.
#[derive(Default)]
pub struct BuildRegistry {
    pids: Mutex<HashMap<String, u32>>,
}

// ----------------------------------------------------------------------
// Persistent log cache (`<workspace>/build/logs`)
// ----------------------------------------------------------------------

/// Cached log files kept before the oldest are pruned.
const MAX_CACHED_LOGS: usize = 100;

/// Every build / device-log session is also appended to
/// `<workspace>/build/logs/<kind>-<id>-<timestamp>.log`, so full histories
/// survive restarts while the in-app pages only keep their tail.
pub(crate) struct LogCache {
    file: Mutex<Option<fs::File>>,
}

impl LogCache {
    /// Opens (or silently skips, logging must never fail a session) the
    /// cache file of one session and prunes the oldest cached logs.
    pub(crate) fn new(app: &tauri::AppHandle, kind: &str, id: &str) -> Arc<Self> {
        let file = (|| -> Option<fs::File> {
            let dir = logs_dir(app)?;
            fs::create_dir_all(&dir).ok()?;
            // Serials may contain characters illegal in file names.
            let safe_id: String = id
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let path = dir.join(format!("{kind}-{safe_id}-{stamp}.log"));
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok()
        })();
        if file.is_some() {
            prune_old_logs(app);
        }
        Arc::new(Self {
            file: Mutex::new(file),
        })
    }

    /// Appends one line (or multi-line chunk) to the cache file.
    pub(crate) fn write(&self, text: &str) {
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let Some(file) = guard.as_mut() else {
            return;
        };
        let _ = writeln!(file, "{text}");
    }
}

fn logs_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let settings = read_settings(app);
    let trimmed = settings.workspace_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed).join("build").join("logs"))
}

/// Keeps only the newest MAX_CACHED_LOGS files in the logs directory.
fn prune_old_logs(app: &tauri::AppHandle) {
    let Some(dir) = logs_dir(app) else { return };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        files.push((modified, path));
    }
    if files.len() <= MAX_CACHED_LOGS {
        return;
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in files.iter().skip(MAX_CACHED_LOGS) {
        let _ = fs::remove_file(path);
    }
}

/// Path of the log-cache directory (created on demand) for opening it in
/// the explorer from the build page.
#[tauri::command]
pub fn get_build_logs_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = logs_dir(&app)
        .ok_or_else(|| "请先在「设置 → 存储」中选择工作空间路径".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建日志缓存目录：{e}"))?;
    Ok(dir.display().to_string())
}

/// One project attached to a build queue. Only the record is kept here
/// (no copying): config-area projects reference the config directory,
/// disk projects reference the picked directory directly. Queues persist
/// as a JSON list in `<workspace>/build/queues.json`.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildProject {
    pub uuid: String,
    pub name: String,
    /// `config` (a configured project of the config area) or `disk`.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// Project directory the build commands run in.
    pub root_path: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuildQueue {
    pub name: String,
    pub uuid: String,
    pub queue_type: String,
    pub created_at: String,
    #[serde(default)]
    pub projects: Vec<BuildProject>,
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

fn build_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(workspace_dir(app)?.join("build"))
}

fn queues_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(build_dir(app)?.join("queues.json"))
}

pub fn load_queues(app: &tauri::AppHandle) -> Result<Vec<BuildQueue>, String> {
    let path = queues_path(app)?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(serde_json::from_str(&content).unwrap_or_default()),
        Err(_) => Ok(Vec::new()),
    }
}

pub fn save_queues(app: &tauri::AppHandle, list: &[BuildQueue]) -> Result<(), String> {
    let dir = build_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建构建区目录：{e}"))?;
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(queues_path(app)?, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_build_queues(app: tauri::AppHandle) -> Result<Vec<BuildQueue>, String> {
    load_queues(&app)
}

/// Creates a new build queue. The uuid is generated by the frontend and
/// must be unique; the timestamp comes from the frontend in local time.
#[tauri::command]
pub fn create_build_queue(
    app: tauri::AppHandle,
    name: String,
    uuid: String,
    queue_type: String,
    created_at: String,
) -> Result<BuildQueue, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("队列名称不能为空".to_string());
    }
    let uuid = uuid.trim().to_string();
    if uuid.is_empty() {
        return Err("队列编号不能为空".to_string());
    }
    let mut list = load_queues(&app)?;
    if list.iter().any(|q| q.uuid == uuid) {
        return Err("队列编号已存在，请重新生成".to_string());
    }

    let queue = BuildQueue {
        name,
        uuid,
        queue_type: queue_type.trim().to_string(),
        created_at,
        projects: Vec::new(),
    };
    list.push(queue.clone());
    save_queues(&app, &list)?;
    crate::records::log_operation(
        &app,
        "build",
        "add",
        "创建构建队列",
        &format!("类型：{}", queue.queue_type),
        vec![queue.name.clone()],
    );
    Ok(queue)
}

/// Attaches a project record to the queue; only the address (plus name
/// and package name) is kept, nothing is copied. Duplicates are rejected
/// by root path within the queue.
#[tauri::command]
pub fn add_build_project(
    app: tauri::AppHandle,
    queue_uuid: String,
    project: BuildProject,
) -> Result<BuildQueue, String> {
    let queue_uuid = queue_uuid.trim().to_string();
    if project.name.trim().is_empty() {
        return Err("项目名称不能为空".to_string());
    }
    let root_path = project.root_path.trim().to_string();
    if root_path.is_empty() {
        return Err("项目地址不能为空".to_string());
    }
    if !Path::new(&root_path).is_dir() {
        return Err(format!("项目目录不存在：{root_path}"));
    }
    let mut list = load_queues(&app)?;
    let queue = list
        .iter_mut()
        .find(|q| q.uuid == queue_uuid)
        .ok_or_else(|| "未找到要添加项目的构建队列".to_string())?;
    if queue.projects.iter().any(|p| p.root_path == root_path) {
        return Err("该项目地址已在队列中，无需重复添加".to_string());
    }
    let mut project = project;
    project.root_path = root_path;
    queue.projects.push(project);
    let updated = queue.clone();
    save_queues(&app, &list)?;
    let added = updated.projects.last().map(|p| p.name.clone()).unwrap_or_default();
    crate::records::log_operation(
        &app,
        "build",
        "add",
        "添加构建项目",
        "仅记录项目地址，不复制文件",
        vec![added],
    );
    Ok(updated)
}

/// Removes a project record from its queue; files are never touched.
#[tauri::command]
pub fn remove_build_project(
    app: tauri::AppHandle,
    queue_uuid: String,
    project_uuid: String,
) -> Result<BuildQueue, String> {
    let queue_uuid = queue_uuid.trim().to_string();
    let project_uuid = project_uuid.trim().to_string();
    let mut list = load_queues(&app)?;
    let queue = list
        .iter_mut()
        .find(|q| q.uuid == queue_uuid)
        .ok_or_else(|| "未找到构建队列".to_string())?;
    let before = queue.projects.len();
    let removed_name = queue
        .projects
        .iter()
        .find(|p| p.uuid == project_uuid)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    queue.projects.retain(|p| p.uuid != project_uuid);
    if queue.projects.len() == before {
        return Err("未找到要删除的项目".to_string());
    }
    let updated = queue.clone();
    save_queues(&app, &list)?;
    crate::records::log_operation(
        &app,
        "build",
        "delete",
        "移除构建卡片",
        "仅移除记录，项目文件不受影响",
        vec![removed_name],
    );
    Ok(updated)
}

/// Clears every project record of one queue; the queue itself is kept
/// and nothing on disk is touched (build queues only record addresses).
#[tauri::command]
pub fn clear_build_queue(app: tauri::AppHandle, queue_uuid: String) -> Result<BuildQueue, String> {
    let queue_uuid = queue_uuid.trim().to_string();
    let mut list = load_queues(&app)?;
    let queue = list
        .iter_mut()
        .find(|q| q.uuid == queue_uuid)
        .ok_or_else(|| "未找到构建队列".to_string())?;
    if queue.projects.is_empty() {
        return Err("队列下暂无项目，无需清空".to_string());
    }
    let removed_names: Vec<String> = queue.projects.iter().map(|p| p.name.clone()).collect();
    queue.projects.clear();
    let updated = queue.clone();
    save_queues(&app, &list)?;
    crate::records::log_operation(
        &app,
        "build",
        "delete",
        "清空构建队列",
        &format!("移除 {} 个项目卡片，项目文件不受影响", removed_names.len()),
        removed_names,
    );
    Ok(updated)
}

/// Deletes build queues by uuid (single and batch share this command).
/// Build queues only record project addresses, so nothing on disk is
/// ever touched.
#[tauri::command]
pub fn delete_build_queues(app: tauri::AppHandle, uuids: Vec<String>) -> Result<(), String> {
    let targets: HashSet<String> = uuids.iter().map(|u| u.trim().to_string()).collect();
    if targets.is_empty() {
        return Ok(());
    }
    let mut list = load_queues(&app)?;
    let removed_names: Vec<String> = list
        .iter()
        .filter(|q| targets.contains(&q.uuid))
        .map(|q| q.name.clone())
        .collect();
    if removed_names.is_empty() {
        return Err("未找到要删除的队列".to_string());
    }
    list.retain(|q| !targets.contains(&q.uuid));
    save_queues(&app, &list)?;
    crate::records::log_operation(
        &app,
        "build",
        "delete",
        "删除构建队列",
        "仅删除队列与卡片记录，项目文件不受影响",
        removed_names,
    );
    Ok(())
}

// ----------------------------------------------------------------------
// Build execution with streamed logs
// ----------------------------------------------------------------------

/// One log line pushed to the frontend through the `build-log` event.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildLogPayload {
    project_uuid: String,
    /// `status` (flow markers), `stdout` (command output) or `done`.
    kind: String,
    line: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<bool>,
}

fn emit_log(
    app: &tauri::AppHandle,
    project_uuid: &str,
    kind: &str,
    line: &str,
    success: Option<bool>,
) {
    let _ = app.emit(
        "build-log",
        BuildLogPayload {
            project_uuid: project_uuid.to_string(),
            kind: kind.to_string(),
            line: line.to_string(),
            success,
        },
    );
}

/// Runs the Android build flow of one project: `<gradle env>/bin/gradle
/// wrapper` first, then `gradlew <task args>` in the project directory
/// when the wrapper step succeeded. Every output line is streamed to the
/// frontend through the `build-log` event. Runs on the async thread pool
/// so the UI stays responsive.
#[tauri::command]
pub async fn run_project_build(
    app: tauri::AppHandle,
    project_uuid: String,
    gradle_env_path: String,
    task_args: Vec<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_build(&app, &project_uuid, &gradle_env_path, task_args)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn run_build(
    app: &tauri::AppHandle,
    project_uuid: &str,
    gradle_env_path: &str,
    task_args: Vec<String>,
) -> Result<(), String> {
    let project_uuid = project_uuid.trim().to_string();
    let task_args: Vec<String> = task_args
        .iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect();
    if task_args.is_empty() {
        return Err("构建任务参数为空".to_string());
    }

    let list = load_queues(app)?;
    let project = list
        .iter()
        .flat_map(|q| q.projects.iter())
        .find(|p| p.uuid == project_uuid)
        .ok_or_else(|| "未找到项目".to_string())?
        .clone();
    let root = PathBuf::from(project.root_path.trim());
    if !root.is_dir() {
        return Err(format!("项目目录不存在：{}", root.display()));
    }

    // Resolve the actual executable: the extension-less `gradle` entry is
    // a POSIX shell script, so on Windows prefer `gradle.bat` (std wraps
    // .bat launches through cmd with correct quoting since 1.77.2).
    let gradle_bin_dir = PathBuf::from(gradle_env_path.trim()).join("bin");
    let Some(gradle_bin) = resolve_executable(&gradle_bin_dir, "gradle") else {
        return Err(format!(
            "Gradle 环境中未找到 gradle：{}",
            gradle_bin_dir.display()
        ));
    };

    // Persistent cache: the full log lands in `build/logs/`, while the
    // in-app page only keeps its tail.
    let cache = LogCache::new(app, "build", &project_uuid);
    let status = |line: &str| {
        cache.write(line);
        emit_log(app, &project_uuid, "status", line, None);
    };

    status(&format!("== 开始构建「{}」==", project.name));

    // Step 1: gradle wrapper with the selected environment.
    status(&format!("> \"{}\" wrapper", gradle_bin.display()));
    if let Err(e) = run_program(
        app,
        &project_uuid,
        &gradle_bin,
        &["wrapper".to_string()],
        &root,
        &cache,
    ) {
        status(&format!("gradle wrapper 执行失败：{e}"));
        emit_log(app, &project_uuid, "done", "", Some(false));
        return Err(format!("gradle wrapper 执行失败：{e}"));
    }
    status("gradle wrapper 执行成功，开始项目构建…");

    // Step 2: gradlew <args> inside the project directory.
    let Some(gradlew) = resolve_executable(&root, "gradlew") else {
        status(&format!("项目根目录未找到 gradlew：{}", root.display()));
        emit_log(app, &project_uuid, "done", "", Some(false));
        return Err(format!("项目根目录未找到 gradlew：{}", root.display()));
    };
    let args = task_args.join(" ");
    status(&format!("> \"{}\" {args}", gradlew.display()));
    match run_program(app, &project_uuid, &gradlew, &task_args, &root, &cache) {
        Ok(()) => {
            status("== 构建成功 ==");
            crate::records::log_operation(
                app,
                "build",
                "modify",
                "构建成功",
                &format!("gradlew {}", task_args.join(" ")),
                vec![project.name.clone()],
            );
            // Record the artifacts in the output area (imported projects:
            // <package folder>/output; others: recursive apk scan).
            match crate::outputs::collect_build_outputs(app, &project) {
                Ok(0) => status("未发现产出物（导入项目查找 output 目录，其它项目扫描 apk）"),
                Ok(n) => status(&format!("已记录 {n} 个产出物到产出区")),
                Err(e) => status(&format!("产出物记录失败：{e}")),
            }
            emit_log(app, &project_uuid, "done", "", Some(true));
            Ok(())
        }
        Err(e) => {
            status(&format!("构建失败：{e}"));
            emit_log(app, &project_uuid, "done", "", Some(false));
            Err(format!("gradlew 执行失败：{e}"))
        }
    }
}

/// Resolves the runnable file of `name` inside `dir`: on Windows the
/// `.bat` variant is preferred (the extension-less file is a POSIX shell
/// script), otherwise the plain file.
fn resolve_executable(dir: &Path, name: &str) -> Option<PathBuf> {
    if cfg!(windows) {
        let bat = dir.join(format!("{name}.bat"));
        if bat.is_file() {
            return Some(bat);
        }
    }
    let plain = dir.join(name);
    if plain.is_file() {
        return Some(plain);
    }
    None
}

/// Runs one program in `cwd`, streaming its output to the frontend. The
/// program is spawned directly (no shell string quoting). Output lines
/// are coalesced into periodic chunks so a chatty build can never flood
/// the webview, and the child pid is registered for stop support.
fn run_program(
    app: &tauri::AppHandle,
    project_uuid: &str,
    program: &Path,
    args: &[String],
    cwd: &Path,
    cache: &Arc<LogCache>,
) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: keep console windows from flashing.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动命令：{e}"))?;

    // Register the pid so stop_project_build can kill the process tree.
    if let Ok(mut pids) = app.state::<BuildRegistry>().pids.lock() {
        pids.insert(project_uuid.to_string(), child.id());
    }

    let result = stream_child(app, project_uuid, &mut child, cache);

    if let Ok(mut pids) = app.state::<BuildRegistry>().pids.lock() {
        pids.remove(project_uuid);
    }
    result
}

/// Drains both output pipes into a shared buffer while a flusher thread
/// emits one multi-line chunk every 150ms; the tail is flushed at the end.
fn stream_child(
    app: &tauri::AppHandle,
    project_uuid: &str,
    child: &mut Child,
    cache: &Arc<LogCache>,
) -> Result<(), String> {
    let stdout = child.stdout.take().ok_or_else(|| "无法读取命令输出".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "无法读取命令错误输出".to_string())?;

    let buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let flushing = Arc::new(AtomicBool::new(true));

    let flusher = {
        let app = app.clone();
        let uuid = project_uuid.to_string();
        let buffer = buffer.clone();
        let flushing = flushing.clone();
        let cache = cache.clone();
        std::thread::spawn(move || {
            while flushing.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(150));
                flush_lines(&app, &uuid, &buffer, &cache);
            }
            flush_lines(&app, &uuid, &buffer, &cache);
        })
    };

    let out_thread = {
        let buffer = buffer.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(text) => {
                        if let Ok(mut lines) = buffer.lock() {
                            lines.push(text)
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    };
    let err_thread = {
        let buffer = buffer.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(text) => {
                        if let Ok(mut lines) = buffer.lock() {
                            lines.push(text)
                        }
                    }
                    Err(_) => break,
                }
            }
        })
    };

    let _ = out_thread.join();
    let _ = err_thread.join();
    let status = child
        .wait()
        .map_err(|e| format!("无法等待命令结束：{e}"))?;

    // Stop the flusher; it pushes the remaining buffered lines on exit.
    flushing.store(false, Ordering::SeqCst);
    let _ = flusher.join();

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "命令退出码 {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "未知".to_string())
        ))
    }
}

/// Emits every buffered line as one `\n`-joined chunk event, appending
/// the same chunk to the session's persistent log cache.
fn flush_lines(
    app: &tauri::AppHandle,
    project_uuid: &str,
    buffer: &Mutex<Vec<String>>,
    cache: &LogCache,
) {
    let Ok(mut lines) = buffer.lock() else { return };
    if lines.is_empty() {
        return;
    }
    let chunk = lines.join("\n");
    lines.clear();
    drop(lines);
    cache.write(&chunk);
    emit_log(app, project_uuid, "stdout", &chunk, None);
}

/// Stops a running build by killing its whole process tree (the gradle
/// launcher spawns java children that must not survive). Runs on the
/// async thread pool so the tree kill never blocks the UI.
#[tauri::command]
pub async fn stop_project_build(app: tauri::AppHandle, project_uuid: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let project_uuid = project_uuid.trim().to_string();
        let pid = {
            let registry = app.state::<BuildRegistry>();
            let pids = registry
                .pids
                .lock()
                .map_err(|_| "构建状态已失效".to_string())?;
            pids.get(&project_uuid).copied()
        };
        let Some(pid) = pid else {
            return Err("该项目当前没有正在进行的构建".to_string());
        };
        kill_process_tree(pid)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
pub(crate) fn kill_process_tree(pid: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("无法执行停止命令：{e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "停止构建失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(not(windows))]
pub(crate) fn kill_process_tree(_pid: u32) -> Result<(), String> {
    Err("当前平台不支持停止构建".to_string())
}

// ----------------------------------------------------------------------
// Cross-page synchronization helpers (config-area cascades)
// ----------------------------------------------------------------------

/// Re-points build cards from an old project directory to a new one and
/// keeps the output records in sync; used when the config area renames a
/// project folder (package rename) or moves it (record action).
pub fn update_project_roots(app: &tauri::AppHandle, from: &str, to: &str) {
    if from.is_empty() || to.is_empty() || from == to {
        return;
    }
    if let Ok(mut list) = load_queues(app) {
        let mut changed = false;
        for queue in list.iter_mut() {
            for project in queue.projects.iter_mut() {
                if project.root_path == from {
                    project.root_path = to.to_string();
                    changed = true;
                }
            }
        }
        if changed {
            let _ = save_queues(app, &list);
        }
    }
    crate::outputs::update_record_roots(app, from, to);
}

/// Drops build cards whose project directory was deleted (config-area
/// cascade); the cards would only fail with「项目目录不存在」otherwise.
pub fn drop_projects_by_roots(app: &tauri::AppHandle, roots: &[String]) {
    if roots.is_empty() {
        return;
    }
    let matches = |path: &str| roots.iter().any(|r| !r.is_empty() && path == r);
    let Ok(mut list) = load_queues(app) else {
        return;
    };
    let mut removed: Vec<String> = Vec::new();
    let mut changed = false;
    for queue in list.iter_mut() {
        for project in queue.projects.iter().filter(|p| matches(&p.root_path)) {
            removed.push(project.name.clone());
        }
        let before = queue.projects.len();
        queue.projects.retain(|p| !matches(&p.root_path));
        changed |= queue.projects.len() != before;
    }
    if changed {
        let _ = save_queues(app, &list);
        crate::records::log_operation(
            app,
            "build",
            "delete",
            "级联移除失效构建卡片",
            "配置区已删除其引用的项目目录",
            removed,
        );
    }
}

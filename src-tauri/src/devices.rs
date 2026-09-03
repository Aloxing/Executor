use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::builds::LogCache;

/// Android device USB-debug support for the build page: detect devices
/// through `adb devices -l` and stream `adb logcat` into the build log
/// area (one tab per device). Streaming uses the same coalescing pattern
/// as the build logs so a chatty logcat can never flood the webview.
///
/// Captures come in two flavors:
/// - whole-device: plain `logcat -v time -T 500` (every process);
/// - app-scoped: resolve the app pid (`adb shell pidof -s <package>`) and
///   stream `logcat --pid=<pid>` — waits while the app is not running and
///   automatically re-attaches when the app restarts, so only that app's
///   lines reach the tab.

/// One live logcat capture session.
#[derive(Default)]
struct Capture {
    /// Pid of the adb client while streaming (None between re-attaches).
    pid: Option<u32>,
    /// Set by stop_device_logcat; the capture loop exits on it.
    stop: bool,
}

/// Live logcat capture sessions, keyed by device serial.
#[derive(Default)]
pub struct DeviceRegistry {
    captures: Mutex<HashMap<String, Capture>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AndroidDevice {
    pub serial: String,
    /// Parsed from `model:` (underscores turned into spaces).
    pub model: String,
    pub product: String,
    /// `device` (authorized), `unauthorized` or `offline`.
    pub status: String,
}

/// One streamed logcat chunk of the `device-log` event.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceLogPayload {
    serial: String,
    /// `status` (flow markers), `stdout` (logcat output) or `done`.
    kind: String,
    line: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<bool>,
}

fn emit_log(
    app: &tauri::AppHandle,
    serial: &str,
    kind: &str,
    line: &str,
    success: Option<bool>,
) {
    let _ = app.emit(
        "device-log",
        DeviceLogPayload {
            serial: serial.to_string(),
            kind: kind.to_string(),
            line: line.to_string(),
            success,
        },
    );
}

/// Best-effort adb lookup: ANDROID_HOME / ANDROID_SDK_ROOT, the default
/// Windows SDK location, then plain `adb` resolved through PATH.
fn resolve_adb() -> PathBuf {
    let exe = if cfg!(windows) { "adb.exe" } else { "adb" };
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(root) = std::env::var(var) {
            let candidate = PathBuf::from(root).join("platform-tools").join(exe);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let candidate = PathBuf::from(local)
            .join("Android")
            .join("Sdk")
            .join("platform-tools")
            .join(exe);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("adb")
}

fn adb_command(program: &PathBuf) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: keep console windows from flashing.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Lists USB-debug Android devices via `adb devices -l`. Runs on the
/// async thread pool: the first adb call may boot the adb daemon, which
/// would freeze the main thread (and the whole UI) for seconds.
#[tauri::command]
pub async fn list_android_devices() -> Result<Vec<AndroidDevice>, String> {
    tauri::async_runtime::spawn_blocking(scan_devices)
        .await
        .map_err(|e| e.to_string())?
}

fn scan_devices() -> Result<Vec<AndroidDevice>, String> {
    let adb = resolve_adb();
    let output = adb_command(&adb)
        .args(["devices", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            format!("无法执行 adb（{}），请确认已安装 Android SDK platform-tools：{e}", adb.display())
        })?;
    if !output.status.success() {
        return Err(format!(
            "adb devices 执行失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut devices = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(serial), Some(status)) = (parts.next(), parts.next()) else {
            continue;
        };
        let mut model = String::new();
        let mut product = String::new();
        for kv in parts {
            if let Some(value) = kv.strip_prefix("model:") {
                model = value.replace('_', " ");
            } else if let Some(value) = kv.strip_prefix("product:") {
                product = value.to_string();
            }
        }
        devices.push(AndroidDevice {
            serial: serial.to_string(),
            model,
            product,
            status: status.to_string(),
        });
    }
    Ok(devices)
}

/// Starts streaming one device's logcat; with a non-empty `package_name`
/// only that app's logs are captured (auto-attach / re-attach by pid).
/// The promise stays pending until the capture is stopped.
#[tauri::command]
pub async fn start_device_logcat(
    app: tauri::AppHandle,
    serial: String,
    package_name: Option<String>,
) -> Result<(), String> {
    let serial = serial.trim().to_string();
    if serial.is_empty() {
        return Err("设备序列号为空".to_string());
    }
    let package = package_name.unwrap_or_default().trim().to_string();
    tauri::async_runtime::spawn_blocking(move || run_logcat(&app, &serial, &package))
        .await
        .map_err(|e| e.to_string())?
}

fn run_logcat(app: &tauri::AppHandle, serial: &str, package: &str) -> Result<(), String> {
    // One capture session per device; a second start is rejected.
    {
        let registry = app.state::<DeviceRegistry>();
        let mut captures = registry
            .captures
            .lock()
            .map_err(|_| "设备日志状态已失效".to_string())?;
        if captures.contains_key(serial) {
            return Err("该设备的日志已在抓取中".to_string());
        }
        captures.insert(serial.to_string(), Capture::default());
    }

    // Persistent cache: the full session lands in `build/logs/`.
    let cache = LogCache::new(app, "device", serial);
    let status = |line: &str| {
        cache.write(line);
        emit_log(app, serial, "status", line, None);
    };

    let result = if package.is_empty() {
        status("== 开始抓取整机日志 ==");
        capture_all(app, serial, &cache)
    } else {
        status(&format!("== 开始抓取应用「{package}」日志 =="));
        capture_app(app, serial, package, &cache)
    };

    if let Ok(mut captures) = app.state::<DeviceRegistry>().captures.lock() {
        captures.remove(serial);
    }
    // Tell the frontend the capture ended (manual stops remove the tab
    // before this arrives, so the event is simply ignored there).
    match &result {
        Ok(()) => emit_log(app, serial, "done", "", Some(true)),
        Err(e) => {
            status(&format!("== 设备日志中断：{e} =="));
            emit_log(app, serial, "done", "", Some(false));
        }
    }
    result
}

/// Whole-device capture: plain logcat until stopped or adb dies.
fn capture_all(
    app: &tauri::AppHandle,
    serial: &str,
    cache: &Arc<LogCache>,
) -> Result<(), String> {
    let adb = resolve_adb();
    let mut child = adb_command(&adb)
        // -T 500: backfill the newest 500 lines, then follow live.
        .args(["-s", serial, "logcat", "-v", "time", "-T", "500"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 adb logcat（{}）：{e}", adb.display()))?;
    set_capture_pid(app, serial, Some(child.id()));
    let result = stream_logcat(app, serial, &mut child, cache);
    set_capture_pid(app, serial, None);
    result
}

/// App-scoped capture: wait for the app process, attach logcat to its
/// pid, and re-attach whenever the app restarts — until stopped.
fn capture_app(
    app: &tauri::AppHandle,
    serial: &str,
    package: &str,
    cache: &Arc<LogCache>,
) -> Result<(), String> {
    let adb = resolve_adb();
    let status = |line: &str| {
        cache.write(line);
        emit_log(app, serial, "status", line, None);
    };
    let mut consecutive_failures = 0;
    loop {
        if stop_requested(app, serial) {
            return Ok(());
        }
        let Some(pid) = find_app_pid(&adb, serial, package) else {
            status(&format!(
                "应用「{package}」未在运行，等待启动中（每 2 秒检测）…"
            ));
            // Sleep in small slices so stopping reacts quickly.
            for _ in 0..10 {
                if stop_requested(app, serial) {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            continue;
        };
        status(&format!("== 已附加到应用「{package}」进程（pid {pid}）=="));
        let mut child = adb_command(&adb)
            .args([
                "-s".to_string(),
                serial.to_string(),
                "logcat".to_string(),
                "-v".to_string(),
                "time".to_string(),
                format!("--pid={pid}"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("无法启动 adb logcat（{}）：{e}", adb.display()))?;
        set_capture_pid(app, serial, Some(child.id()));
        let result = stream_logcat(app, serial, &mut child, cache);
        set_capture_pid(app, serial, None);
        // Manual stop kills the child: exit without any failure noise.
        if stop_requested(app, serial) {
            return Ok(());
        }
        if let Err(e) = result {
            // --pid needs Android 7+; bail out instead of spinning when
            // the stream keeps failing right away.
            consecutive_failures += 1;
            if consecutive_failures >= 3 {
                return Err(format!(
                    "按进程抓取连续失败（设备系统需 Android 7.0+）：{e}"
                ));
            }
            status(&format!("日志流异常：{e}，尝试重新附加…"));
            continue;
        }
        consecutive_failures = 0;
        status("== 应用进程已结束，等待重新启动后自动重连… ==");
    }
}

/// `adb shell pidof -s <package>`; None when the app is not running.
fn find_app_pid(adb: &PathBuf, serial: &str, package: &str) -> Option<u32> {
    let output = adb_command(adb)
        .args(["-s", serial, "shell", "pidof", "-s", package])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().ok()
}

fn set_capture_pid(app: &tauri::AppHandle, serial: &str, pid: Option<u32>) {
    if let Ok(mut captures) = app.state::<DeviceRegistry>().captures.lock() {
        if let Some(capture) = captures.get_mut(serial) {
            capture.pid = pid;
        }
    }
}

fn stop_requested(app: &tauri::AppHandle, serial: &str) -> bool {
    app.state::<DeviceRegistry>()
        .captures
        .lock()
        .map(|captures| {
            captures
                .get(serial)
                .map(|capture| capture.stop)
                .unwrap_or(true)
        })
        .unwrap_or(true)
}

/// Stops the logcat capture of one device (kills the adb client tree).
/// Runs on the async thread pool so the tree kill never blocks the UI.
#[tauri::command]
pub async fn stop_device_logcat(app: tauri::AppHandle, serial: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        let pid = {
            let registry = app.state::<DeviceRegistry>();
            let mut captures = registry
                .captures
                .lock()
                .map_err(|_| "设备日志状态已失效".to_string())?;
            let capture = captures
                .get_mut(&serial)
                .ok_or_else(|| "该设备当前没有在抓取日志".to_string())?;
            // Flag first so the re-attach loop never spawns again.
            capture.stop = true;
            capture.pid
        };
        match pid {
            Some(pid) => crate::builds::kill_process_tree(pid),
            // Waiting phase (no live child); the loop exits on the flag.
            None => Ok(()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Drains both pipes into a shared buffer while a flusher thread emits
/// one multi-line chunk every 150ms; the tail is flushed at the end.
fn stream_logcat(
    app: &tauri::AppHandle,
    serial: &str,
    child: &mut Child,
    cache: &Arc<LogCache>,
) -> Result<(), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取设备日志输出".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法读取设备日志错误输出".to_string())?;

    let buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let flushing = Arc::new(AtomicBool::new(true));

    let flusher = {
        let app = app.clone();
        let serial = serial.to_string();
        let buffer = buffer.clone();
        let flushing = flushing.clone();
        let cache = cache.clone();
        std::thread::spawn(move || {
            while flushing.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(150));
                flush_lines(&app, &serial, &buffer, &cache);
            }
            flush_lines(&app, &serial, &buffer, &cache);
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
        .map_err(|e| format!("无法等待 adb logcat 结束：{e}"))?;

    flushing.store(false, Ordering::SeqCst);
    let _ = flusher.join();

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "adb logcat 退出码 {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "未知".to_string())
        ))
    }
}

fn flush_lines(
    app: &tauri::AppHandle,
    serial: &str,
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
    emit_log(app, serial, "stdout", &chunk, None);
}

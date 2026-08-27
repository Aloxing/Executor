//! Code kernel: Rust port of `code_kernel.py` (CodeInjectEngine).
//!
//! Input: a config JSON path + the project root folder. Entries with
//! `write_mode == "code"` generate Java methods from their `scenes`
//! definitions and inject them between `area_name` markers of the target
//! file; every other `write_mode` is skipped.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use regex::Regex;
use serde_json::{Map, Value};

/// One task result, mirroring Python's per-task result dict.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub index: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    pub methods_generated: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<String>,
}

/// Runs the code kernel: loads the config JSON at `config_path` and
/// executes every `code` task against the project at `project_root`.
pub fn run(project_root: &Path, config_path: &Path) -> Result<Vec<TaskResult>, String> {
    if !project_root.exists() {
        return Err(format!("项目路径不存在: {}", project_root.display()));
    }
    if !config_path.exists() {
        return Err(format!("JSON配置文件不存在: {}", config_path.display()));
    }
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("无法读取配置文件 {}: {e}", config_path.display()))?;
    let data: Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析JSON文件失败 - {e}"))?;
    let configs = load_config(data);
    Ok(execute_all(project_root, &configs))
}

/// Extracts the task list: a JSON array is used as-is; an object yields
/// every value that contains a `write_mode` field (insertion order kept).
fn load_config(data: Value) -> Vec<Map<String, Value>> {
    match data {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::Object(map) => Some(map),
                _ => None,
            })
            .collect(),
        Value::Object(map) => map
            .into_iter()
            .filter(|(_, value)| value.get("write_mode").is_some())
            .filter_map(|(_, value)| match value {
                Value::Object(inner) => Some(inner),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Batch entry (mirrors `execute_all`): tasks whose `write_mode` is not
/// `code` are skipped with a reason; the rest are executed in order.
fn execute_all(project_root: &Path, configs: &[Map<String, Value>]) -> Vec<TaskResult> {
    let mut results = Vec::new();
    for (index, config) in configs.iter().enumerate() {
        let write_mode = config
            .get("write_mode")
            .and_then(Value::as_str)
            .unwrap_or("code")
            .to_string();
        if write_mode != "code" {
            results.push(TaskResult {
                index,
                success: true,
                skipped: true,
                reason: Some(format!("write_mode 为 '{write_mode}'，非 'code'，已跳过")),
                file: Some(
                    config
                        .get("file_path")
                        .and_then(Value::as_str)
                        .unwrap_or("未知")
                        .to_string(),
                ),
                ..Default::default()
            });
            continue;
        }
        let mut result = execute(project_root, config);
        result.index = index;
        results.push(result);
    }
    results
}

/// Single task execution (mirrors `execute`).
fn execute(project_root: &Path, config: &Map<String, Value>) -> TaskResult {
    let file_path = str_field(config, "file_path");
    let area_name = str_field(config, "area_name");
    let write_mode = {
        let mode = str_field(config, "write_mode");
        if mode.is_empty() { "code".to_string() } else { mode }
    };
    let do_backup = config.get("backup").and_then(Value::as_bool).unwrap_or(true);

    let generated_code = match generate_code(config) {
        Ok(code) => code,
        Err(err) => {
            return TaskResult {
                success: false,
                error: Some(err),
                ..Default::default()
            }
        }
    };

    // Normalize the JSON relative path and join it onto the project root
    // (like Python's normpath(join(root, rel))).
    let normalized_rel = file_path.replace('\\', "/");
    let full_path = normalize_join(project_root, &normalized_rel);

    if !full_path.exists() {
        return TaskResult {
            success: false,
            error: Some(format!("目标文件不存在: {}", full_path.display())),
            ..Default::default()
        };
    }

    let Ok(raw) = fs::read_to_string(&full_path) else {
        return TaskResult {
            success: false,
            error: Some(format!("无法读取目标文件: {}", full_path.display())),
            ..Default::default()
        };
    };
    // Python reads in text mode: universal newlines on read, os.linesep
    // translation on write. Reproduce both for byte-identical output.
    let original = crate::common::text::normalize_newlines(&raw);

    let backup_path = if do_backup {
        backup_file(&full_path).ok()
    } else {
        None
    };

    let methods_generated = config
        .get("scenes")
        .and_then(Value::as_object)
        .map(Map::len)
        .unwrap_or(0);

    let Some(injected) = inject_code(&original, &area_name, &generated_code, &write_mode) else {
        return TaskResult {
            success: false,
            error: Some(format!("未找到注入区域标记: {area_name}")),
            file: Some(full_path.display().to_string()),
            backup: backup_path,
            ..Default::default()
        };
    };

    if let Err(e) = fs::write(&full_path, crate::common::text::to_platform_newlines(&injected)) {
        return TaskResult {
            success: false,
            error: Some(format!("写入目标文件失败: {e}")),
            file: Some(full_path.display().to_string()),
            backup: backup_path,
            ..Default::default()
        };
    }

    TaskResult {
        success: true,
        skipped: false,
        file: Some(full_path.display().to_string()),
        area: Some(area_name),
        mode: Some(write_mode),
        methods_generated,
        backup: backup_path,
        ..Default::default()
    }
}

fn str_field(map: &Map<String, Value>, field: &str) -> String {
    map.get(field).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Joins a (possibly absolute, possibly `..`-bearing) normalized relative
/// path onto the root, mirroring `os.path.normpath(os.path.join(...))`.
fn normalize_join(root: &Path, rel: &str) -> PathBuf {
    let joined = root.join(rel);
    let mut components = Vec::new();
    for component in joined.components() {
        match component {
            std::path::Component::ParentDir => {
                if components.len() > 1 {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

// ----------------------------------------------------------------------
// Code generation
// ----------------------------------------------------------------------

/// Generates the Java source for all scenes (mirrors `generate_code`).
fn generate_code(config: &Map<String, Value>) -> Result<String, String> {
    let scenes = config
        .get("scenes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let rule_templates = config
        .get("ruleTemplates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let log_tag = {
        let tag = str_field(config, "logTag");
        if tag.is_empty() { "TAG".to_string() } else { tag }
    };

    let mut methods = Vec::new();
    for (scene_name, scene_def) in &scenes {
        let scene_object = match scene_def {
            Value::Object(map) => map,
            _ => continue,
        };
        methods.push(generate_method(scene_name, scene_object, &rule_templates, &log_tag)?);
    }
    Ok(methods.join("\n\n"))
}

fn generate_method(
    scene_name: &str,
    scene_def: &Map<String, Value>,
    rule_templates: &Map<String, Value>,
    log_tag: &str,
) -> Result<String, String> {
    let mut lines = vec![
        format!("    public void {scene_name}() {{"),
        format!("        Log.i({log_tag}, \"{scene_name}: \");"),
    ];
    let body = scene_def.get("body").and_then(Value::as_array);
    for block in body.into_iter().flatten() {
        let Value::Object(block) = block else { continue };
        let block_type = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("direct");
        let statement = match block_type {
            "direct" => {
                let call = block
                    .get("call")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("场景「{scene_name}」的 direct 语句缺少 call"))?;
                build_direct_call(call)
            }
            "ruled" => {
                let rule = block
                    .get("rule")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("场景「{scene_name}」的 ruled 语句缺少 rule"))?;
                let call = block
                    .get("call")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("场景「{scene_name}」的 ruled 语句缺少 call"))?;
                build_ruled_call(rule, call, rule_templates)?
            }
            _ => continue,
        };
        lines.push(format!("        {statement}"));
    }
    lines.push("    }".to_string());
    Ok(lines.join("\n"))
}

fn build_direct_call(call: &Map<String, Value>) -> String {
    let callback = call
        .get("callback")
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = format_args_list(call.get("args").and_then(Value::as_array));
    format!("{callback}({args});")
}

fn build_ruled_call(
    rule: &Map<String, Value>,
    call: &Map<String, Value>,
    rule_templates: &Map<String, Value>,
) -> Result<String, String> {
    let template = rule
        .get("template")
        .and_then(Value::as_str)
        .ok_or_else(|| "ruled 语句缺少 rule.template".to_string())?;
    let template_def = rule_templates
        .get(template)
        .and_then(Value::as_object);
    let obj_class = template_def
        .and_then(|def| def.get("class").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .unwrap_or("advertiseComplianceJob")
        .to_string();
    let method = template_def
        .and_then(|def| def.get("method").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .unwrap_or(template)
        .to_string();
    let rule_args = format_args_list(rule.get("args").and_then(Value::as_array));
    let callback = call
        .get("callback")
        .and_then(Value::as_str)
        .unwrap_or("");
    let cb_ref = format!("this::{callback}");
    let all_args = if rule_args.is_empty() {
        cb_ref
    } else {
        format!("{rule_args}, {cb_ref}")
    };
    Ok(format!("{obj_class}.{method}({all_args});"))
}

/// Python-style argument formatting: strings are quoted, everything else
/// uses Python's `str()` rendering (bools as True/False etc.).
fn format_args_list(args: Option<&Vec<Value>>) -> String {
    args.map(|items| {
        items
            .iter()
            .map(|arg| match arg {
                Value::String(s) => format!("\"{s}\""),
                other => crate::core::android::argument::display(other),
            })
            .collect::<Vec<_>>()
            .join(", ")
    })
    .unwrap_or_default()
}

// ----------------------------------------------------------------------
// Injection: marker matching + same-name dedup
// ----------------------------------------------------------------------

/// Extracts the method names from generated code
/// (`public <type> Name(`), mirroring `_extract_method_names`.
fn extract_method_names(code: &str) -> Vec<String> {
    let Ok(re) = Regex::new(r"public\s+\w+\s+(\w+)\s*\(") else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for caps in re.captures_iter(code) {
        let name = caps[1].to_string();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Removes every method whose name appears in `method_names` from
/// `existing_code`, using brace-depth counting to find the body end
/// (mirrors `_remove_existing_methods`).
fn remove_existing_methods(existing_code: &str, method_names: &[String]) -> String {
    let mut code = existing_code.to_string();
    for name in method_names {
        let sig_pattern = format!(
            r"(?m)^[ \t]*public\s+\w+\s+{}\s*\([^)]*\)\s*\{{",
            regex::escape(name)
        );
        let Ok(sig_re) = Regex::new(&sig_pattern) else {
            continue;
        };
        let Some(sig_match) = sig_re.find(&code) else {
            continue;
        };

        // Brace counting from the signature's '{' finds the body end.
        let mut start_pos = sig_match.start();
        let brace_start = sig_match.end() - 1;
        let bytes = code.as_bytes();
        let mut depth = 0i32;
        let mut pos = brace_start;
        let mut end_pos: Option<usize> = None;
        while pos < bytes.len() {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = Some(pos + 1);
                        break;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        let Some(mut end_pos) = end_pos else {
            continue; // Unbalanced braces: leave the method untouched.
        };

        // Eat the blank lines before the method, keeping one newline.
        while start_pos > 0 && matches!(code.as_bytes()[start_pos - 1], b' ' | b'\t') {
            start_pos -= 1;
        }
        while start_pos > 0 && code.as_bytes()[start_pos - 1] == b'\n' {
            start_pos -= 1;
        }
        start_pos += 1;

        // Eat the blank lines after the method.
        while end_pos < bytes.len() && matches!(bytes[end_pos], b'\n' | b' ') {
            end_pos += 1;
        }

        code = format!("{}{}", &code[..start_pos], &code[end_pos..]);
    }
    code
}

/// Injects `code` into the `area_name` region of `content`
/// (mirrors `_inject_code`). Paired markers take priority; a single
/// marker inserts right below the marker line.
fn inject_code(content: &str, area_name: &str, code: &str, write_mode: &str) -> Option<String> {
    let escaped = regex::escape(area_name);
    let new_method_names = extract_method_names(code);

    let paired_pattern = format!(r"(?s)({escaped}[^\n]*\n)(.*?)(\n[^\n]*{escaped})");
    if let Ok(paired_re) = Regex::new(&paired_pattern) {
        if let Some(caps) = paired_re.captures(content) {
            let whole = caps.get(0).unwrap();
            let start_marker = caps[1].to_string();
            let mut old_code = caps[2].to_string();
            let end_marker = caps[3].to_string();

            // Core step: drop same-named methods from the old block.
            if !new_method_names.is_empty() {
                old_code = remove_existing_methods(&old_code, &new_method_names);
            }

            let new_block = if write_mode == "append" {
                let combined = if old_code.trim().is_empty() {
                    format!("\n{code}")
                } else {
                    format!("{}\n\n{code}", old_code.trim_end())
                };
                format!("{start_marker}{combined}{end_marker}")
            } else {
                // "code" (and any other mode) overwrites the region.
                format!("{start_marker}\n{code}\n{end_marker}")
            };
            return Some(format!(
                "{}{}{}",
                &content[..whole.start()],
                new_block,
                &content[whole.end()..]
            ));
        }
    }

    // Fallback: single marker inserts the code below the marker line.
    let single_pattern = format!(r"({escaped}[^\n]*\n)");
    if let Ok(single_re) = Regex::new(&single_pattern) {
        if let Some(m) = single_re.find(content) {
            let insert_pos = m.end();
            return Some(format!(
                "{}\n{code}\n{}",
                &content[..insert_pos],
                &content[insert_pos..]
            ));
        }
    }

    None
}

/// Backs the file up as `<file>.bak_<timestamp>` (mirrors `_backup_file`).
fn backup_file(file_path: &Path) -> Result<String, std::io::Error> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = format!("{}.bak_{timestamp}", file_path.display());
    fs::copy(file_path, &backup_path)?;
    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Dedicated test root under target/: avoids the Windows
    /// `std::env::temp_dir()` NotFound quirk and per-process cleanup races.
    fn test_dir(name: &str) -> PathBuf {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest
            .join("target")
            .join("test_tmp")
            .join(format!("code_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn object(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => unreachable!(),
        }
    }

    #[test]
    fn load_config_object_and_array_roots() {
        let from_object = load_config(json!({
            "task_a": { "write_mode": "code", "file_path": "A.java" },
            "not_a_task": { "foo": 1 },
            "task_b": { "write_mode": "argument" }
        }));
        assert_eq!(from_object.len(), 2);
        let from_array = load_config(json!([
            { "write_mode": "code" },
            "not-an-object"
        ]));
        assert_eq!(from_array.len(), 1);
    }

    /// Generation matches the Python reference: direct + ruled calls,
    /// argument quoting and the `this::callback` method reference.
    #[test]
    fn generate_code_reference_shape() {
        let config = object(json!({
            "write_mode": "code",
            "logTag": "AdsSDK",
            "scenes": {
                "VideoAd": {
                    "body": [
                        { "type": "direct",
                          "call": { "callback": "tryShowVideo",
                                    "args": ["MoyvGameHelper", "OnVideoAdClosed", ""] } }
                    ]
                },
                "VictoryGameLevelAD": {
                    "body": [
                        { "type": "ruled",
                          "rule": { "template": "secondsLimit", "args": ["native", 30] },
                          "call": { "callback": "tryShowNative", "args": [] } }
                    ]
                }
            },
            "ruleTemplates": {
                "secondsLimit": { "class": "advertiseComplianceJob", "method": "secondsLimit" }
            }
        }));
        let code = generate_code(&config).unwrap();
        assert!(code.contains("    public void VideoAd() {\n        Log.i(AdsSDK, \"VideoAd: \");"));
        assert!(code.contains("tryShowVideo(\"MoyvGameHelper\", \"OnVideoAdClosed\", \"\");"));
        assert!(code.contains(
            "advertiseComplianceJob.secondsLimit(\"native\", 30, this::tryShowNative);"
        ));
        // Methods are separated by a blank line.
        assert!(code.contains("}\n\n    public void"));
    }

    /// Exact parity with the Python reference output: dedup removes the
    /// old Foo block and its surrounding blank lines.
    #[test]
    fn paired_marker_overwrite_with_dedup() {
        let content = "class A {\n//---area---\n    public void Foo() {\n        old();\n    }\n\n    public void Keep() {\n    }\n//---area---\n}\n";
        let new_code = "    public void Foo() {\n        fresh();\n    }";
        let out = inject_code(content, "//---area---", new_code, "code").unwrap();
        let expected = "class A {\n//---area---\n\n    public void Foo() {\n        fresh();\n    }\n\n//---area---\n}\n";
        assert_eq!(out, expected);
        assert!(!out.contains("old();"));
    }

    /// CRLF files (the norm in the test project): the region is still
    /// fully overwritten in code mode. Exact Python parity.
    #[test]
    fn paired_marker_crlf_code_mode() {
        let content = "class A {\r\n//---area---\r\n    public void Foo() {\r\n        old();\r\n    }\r\n//---area---\r\n}\r\n";
        let new_code = "    public void Foo() {\n        fresh();\n    }";
        let out = inject_code(content, "//---area---", new_code, "code").unwrap();
        let expected = "class A {\r\n//---area---\r\n\n    public void Foo() {\n        fresh();\n    }\n\n//---area---\r\n}\r\n";
        assert_eq!(out, expected);
    }

    /// Append mode keeps existing code; dedup strips the old same-named
    /// method including the next method's indentation (Python parity).
    #[test]
    fn append_mode_dedup_exact_parity() {
        let content = "class A {\n//---area---\n    public void Foo() {\n        old();\n    }\n\n    public void Keep() {\n    }\n//---area---\n}\n";
        let new_code = "    public void Foo() {\n        fresh();\n    }";
        let out = inject_code(content, "//---area---", new_code, "append").unwrap();
        let expected = "class A {\n//---area---\n public void Keep() {\n    }\n\n    public void Foo() {\n        fresh();\n    }\n//---area---\n}\n";
        assert_eq!(out, expected);
        assert!(!out.contains("old();"));
    }

    #[test]
    fn append_mode_keeps_existing_code() {
        let content = "class A {\n//---area---\n    public void Other() {\n    }\n//---area---\n}\n";
        let new_code = "    public void Fresh() {\n    }";
        let out = inject_code(content, "//---area---", new_code, "append").unwrap();
        assert!(out.contains("public void Other()"));
        assert!(out.contains("public void Fresh()"));
    }

    #[test]
    fn single_marker_fallback() {
        let content = "class A {\n//---area---\n    int x;\n}\n";
        let out = inject_code(content, "//---area---", "CODE", "code").unwrap();
        assert!(out.contains("//---area---\n\nCODE\n"));
        assert!(inject_code("no marker", "//---area---", "CODE", "code").is_none());
    }

    /// End-to-end run on a fixture: success task, missing-file task and
    /// missing-marker task, with backup creation.
    #[test]
    fn kernel_run_on_fixture() {
        let dir = test_dir("fixture");
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src/Target.java"),
            "public class Target {\n//---inject_code_area---\n//---inject_code_area---\n}\n",
        )
        .unwrap();

        let config_path = dir.join("config.json");
        fs::write(
            &config_path,
            json!({
                "ok_task": {
                    "write_mode": "code",
                    "file_path": "src\\Target.java",
                    "area_name": "//---inject_code_area---",
                    "backup": true,
                    "scenes": {
                        "ShowAd": { "body": [
                            { "type": "direct", "call": { "callback": "show", "args": ["id", 30] } }
                        ] }
                    }
                },
                "missing_file": {
                    "write_mode": "code",
                    "file_path": "src/Nope.java",
                    "area_name": "//---inject_code_area---",
                    "scenes": {}
                },
                "missing_marker": {
                    "write_mode": "code",
                    "file_path": "src/Target.java",
                    "area_name": "//---no_such_marker---",
                    "scenes": {}
                },
                "not_code": { "write_mode": "argument", "file_path": "x" }
            })
            .to_string(),
        )
        .unwrap();

        let results = run(&dir, &config_path).unwrap();
        assert_eq!(results.len(), 4);
        // Task order follows the JSON order: ok, missing file, missing
        // marker, then the skipped argument entry.
        assert!(results[0].success && !results[0].skipped);
        assert_eq!(results[0].methods_generated, 1);
        assert!(results[0].backup.as_ref().unwrap().contains(".bak_"));
        assert!(!results[1].success);
        assert!(results[1].error.as_ref().unwrap().contains("目标文件不存在"));
        assert!(!results[2].success);
        assert!(results[2].error.as_ref().unwrap().contains("未找到注入区域标记"));
        assert!(results[3].success && results[3].skipped);

        let injected = fs::read_to_string(dir.join("src/Target.java")).unwrap();
        assert!(injected.contains("public void ShowAd()"));
        assert!(injected.contains("show(\"id\", 30);"));
        // A backup file exists next to the target.
        let backups: Vec<_> = fs::read_dir(dir.join("src"))
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".bak_"))
            .collect();
        assert_eq!(backups.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }
}

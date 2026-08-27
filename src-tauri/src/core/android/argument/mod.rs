//! Argument kernel: Rust port of `argument_kernel.py`.
//!
//! Input: a config JSON path + the project root folder. Only entries with
//! `write_mode == "argument"` participate; everything else is recorded in
//! `skipped`. Values are resolved (`value_override` / `value_prefix` /
//! `value_format`) and either injected into project files by file type or
//! copied when `file_path` is a directory.

mod values;
mod writers;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

pub use values::{apply_format, cast_value, display, kind_of, Kind};

const WRITE_MODE: &str = "argument";

/// One parsed `argument` entry (mirrors Python's `Argument` dataclass).
#[derive(Debug, Clone)]
struct Argument {
    name: String,
    file_path: String,
    file_type: String,
    key_name: String,
    value_override: String,
    value_prefix: String,
    value_format: String,
    value: Value,
    kind: Kind,
}

impl Argument {
    fn from_spec(name: &str, body: &Map<String, Value>) -> Argument {
        let raw = body.get("value").cloned().unwrap_or(Value::String(String::new()));
        let file_path = body
            .get("file_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace("\\\\", "/")
            .replace('\\', "/");
        let file_type = body
            .get("file_type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase()
            .trim_start_matches('.')
            .to_string();
        let key_name = body
            .get("key_name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| name.to_string());
        Argument {
            name: name.to_string(),
            file_path,
            file_type,
            key_name,
            value_override: str_field(body, "value_override"),
            value_prefix: str_field(body, "value_prefix"),
            value_format: str_field(body, "value_format"),
            kind: kind_of(&raw),
            value: raw,
        }
    }

    /// `file_path` is a directory => `value` is the source file to copy.
    fn is_copy(&self) -> bool {
        self.file_type.is_empty() && Path::new(&self.file_path).extension().is_none()
    }
}

fn str_field(body: &Map<String, Value>, field: &str) -> String {
    body.get(field).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Parses the config and resolves each field's final value
/// (mirrors `ArgumentTable`). Iteration order follows the JSON order.
struct ArgumentTable {
    arguments: Vec<(String, Argument)>,
    skipped: Vec<String>,
    cache: HashMap<String, Value>,
    cache_raw: HashMap<String, Value>,
}

impl ArgumentTable {
    fn new(spec: &Map<String, Value>, injected: Option<&Map<String, Value>>) -> ArgumentTable {
        let mut arguments: Vec<(String, Argument)> = Vec::new();
        let mut skipped = Vec::new();
        for (name, body) in spec {
            let Value::Object(body) = body else {
                skipped.push(name.clone());
                continue;
            };
            if body.get("write_mode").and_then(Value::as_str) != Some(WRITE_MODE) {
                // Other write_modes belong to other kernels.
                skipped.push(name.clone());
                continue;
            }
            arguments.push((name.clone(), Argument::from_spec(name, body)));
        }
        if let Some(injected) = injected {
            for (name, value) in injected {
                if let Some((_, arg)) = arguments.iter_mut().find(|(n, _)| n == name) {
                    arg.value = value.clone();
                }
            }
        }
        ArgumentTable {
            arguments,
            skipped,
            cache: HashMap::new(),
            cache_raw: HashMap::new(),
        }
    }

    fn contains(&self, name: &str) -> bool {
        self.arguments.iter().any(|(n, _)| n == name)
    }

    fn get(&self, name: &str) -> Option<&Argument> {
        self.arguments.iter().find(|(n, _)| n == name).map(|(_, arg)| arg)
    }

    /// Resolves a field's final value; `with_prefix = false` returns the
    /// bare value without the field's own `value_prefix`.
    fn resolve(
        &mut self,
        name: &str,
        with_prefix: bool,
        stack: &[String],
    ) -> Result<Value, String> {
        let cache = if with_prefix { &self.cache } else { &self.cache_raw };
        if let Some(cached) = cache.get(name) {
            return Ok(cached.clone());
        }
        if stack.iter().any(|entry| entry == name) {
            let chain = stack
                .iter()
                .cloned()
                .chain(std::iter::once(name.to_string()))
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(format!("value_override 存在循环引用: {chain}"));
        }
        let Some(arg) = self.get(name).cloned() else {
            return Err(format!("未找到 argument 字段: {name}"));
        };

        let mut raw = arg.value.clone();
        if !arg.value_override.is_empty() {
            if !self.contains(&arg.value_override) {
                return Err(format!(
                    "{} 的 value_override 指向不存在的字段: {}",
                    name, arg.value_override
                ));
            }
            // Only the bare value is inherited: the referenced field's
            // value_prefix must not leak into this field.
            let mut child_stack = stack.to_vec();
            child_stack.push(name.to_string());
            raw = self.resolve(&arg.value_override, false, &child_stack)?;
        }

        let mut bare = cast_value(&raw, arg.kind)?;
        if arg.kind == Kind::String {
            let formatted = apply_format(bare.as_str().unwrap_or(""), &arg.value_format);
            bare = Value::String(formatted);
        }
        self.cache_raw.insert(name.to_string(), bare.clone());

        // The prefix is applied only for the field that declares it, and
        // idempotently (existing prefix is not duplicated).
        let mut final_value = bare.clone();
        if arg.kind == Kind::String && !arg.value_prefix.is_empty() {
            let bare_str = bare.as_str().unwrap_or("");
            if !bare_str.starts_with(&arg.value_prefix) {
                final_value = Value::String(format!("{}{}", arg.value_prefix, bare_str));
            }
        }
        self.cache.insert(name.to_string(), final_value.clone());
        Ok(if with_prefix { final_value } else { bare })
    }

    fn resolve_all(&mut self) -> Result<Map<String, Value>, String> {
        let names: Vec<String> = self.arguments.iter().map(|(n, _)| n.clone()).collect();
        let mut resolved = Map::new();
        for name in names {
            let value = self.resolve(&name, true, &[])?;
            resolved.insert(name, value);
        }
        Ok(resolved)
    }
}

// ----------------------------------------------------------------------
// Report
// ----------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrittenEntry {
    pub file: String,
    pub key: String,
    pub argument: String,
    pub value: Value,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopiedEntry {
    pub argument: String,
    pub from: String,
    pub to: String,
}

/// Kernel report, mirroring Python's `report` dict.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentReport {
    pub resolved: Map<String, Value>,
    pub skipped: Vec<String>,
    pub written: Vec<WrittenEntry>,
    pub copied: Vec<CopiedEntry>,
    pub errors: Vec<String>,
}

// ----------------------------------------------------------------------
// Kernel
// ----------------------------------------------------------------------

/// Kernel options (mirrors the `ArgumentKernel` constructor flags).
#[derive(Debug, Clone, Copy)]
pub struct KernelOptions {
    /// Parse and validate only; never touch the disk.
    pub dry_run: bool,
    /// Abort with `Err` on the first error instead of collecting them.
    pub strict: bool,
}

impl Default for KernelOptions {
    fn default() -> Self {
        KernelOptions {
            dry_run: false,
            strict: true,
        }
    }
}

struct Kernel<'a> {
    root: &'a Path,
    options: KernelOptions,
}

/// Runs the argument kernel: reads the config JSON at `config_path`,
/// applies every `argument` entry to the project at `project_root` and
/// returns the report. `values` injects runtime overrides by field name
/// (Python's `--values`).
pub fn run(
    project_root: &Path,
    config_path: &Path,
    values: Option<&Map<String, Value>>,
    options: KernelOptions,
) -> Result<ArgumentReport, String> {
    let spec = load_config(config_path)?;
    let kernel = Kernel {
        root: project_root,
        options,
    };
    kernel.run(&spec, values)
}

fn load_config(config_path: &Path) -> Result<Map<String, Value>, String> {
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("无法读取配置文件 {}: {e}", config_path.display()))?;
    match serde_json::from_str::<Value>(&content) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("配置文件根节点必须是 JSON 对象".to_string()),
        Err(e) => Err(format!("配置文件 JSON 解析失败: {e}")),
    }
}

impl<'a> Kernel<'a> {
    fn run(
        self,
        spec: &Map<String, Value>,
        values: Option<&Map<String, Value>>,
    ) -> Result<ArgumentReport, String> {
        let mut table = ArgumentTable::new(spec, values);
        let resolved = table.resolve_all()?;

        let mut report = ArgumentReport {
            resolved: resolved.clone(),
            skipped: table.skipped.clone(),
            ..Default::default()
        };

        // Group the arguments of the same file so each file is read and
        // written only once; copy entries are handled immediately.
        let mut edits: Vec<(PathBuf, Vec<String>)> = Vec::new();
        for (name, arg) in table.arguments.iter() {
            if arg.file_path.is_empty() {
                continue;
            }
            if arg.is_copy() {
                let source = resolved.get(name).cloned().unwrap_or(Value::Null);
                self.copy(arg, &source, &mut report)?;
            } else {
                let path = self.root.join(&arg.file_path);
                match edits.iter_mut().find(|(p, _)| *p == path) {
                    Some(entry) => entry.1.push(name.clone()),
                    None => edits.push((path, vec![name.clone()])),
                }
            }
        }

        for (path, names) in edits {
            let args: Vec<&Argument> = names
                .iter()
                .filter_map(|name| table.get(name))
                .collect();
            self.write_file(&path, &args, &resolved, &mut report)?;
        }
        Ok(report)
    }

    /// Records an error; in strict mode the run aborts with `Err`.
    fn fail(&self, report: &mut ArgumentReport, message: String) -> Result<(), String> {
        report.errors.push(message.clone());
        if self.options.strict {
            Err(message)
        } else {
            Ok(())
        }
    }

    fn write_file(
        &self,
        path: &Path,
        args: &[&Argument],
        resolved: &Map<String, Value>,
        report: &mut ArgumentReport,
    ) -> Result<(), String> {
        if !path.exists() {
            return self.fail(report, format!("文件不存在: {}", path.display()));
        }
        let file_type = if args[0].file_type.is_empty() {
            path.extension()
                .map(|ext| ext.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        } else {
            args[0].file_type.clone()
        };
        if !matches!(
            file_type.as_str(),
            "xml" | "gradle" | "kts" | "properties" | "java" | "kt" | "cs"
        ) {
            return self.fail(
                report,
                format!("暂不支持的 file_type: {file_type} ({})", path.display()),
            );
        }

        let raw = fs::read_to_string(path)
            .map_err(|e| format!("无法读取文件 {}: {e}", path.display()))?;
        // Python reads in text mode: universal newlines on read,
        // os.linesep translation on write (byte-identical output).
        let original = crate::common::text::normalize_newlines(&raw);
        let mut text = original.clone();
        for arg in args {
            let value = resolved.get(&arg.name).cloned().unwrap_or(Value::Null);
            match writers::write(&file_type, &text, &arg.key_name, &value, arg.kind) {
                Ok(updated) => {
                    text = updated;
                    report.written.push(WrittenEntry {
                        file: path.display().to_string(),
                        key: arg.key_name.clone(),
                        argument: arg.name.clone(),
                        value,
                    });
                }
                Err(err) => self.fail(report, format!("{}: {err}", arg.name))?,
            }
        }

        if text != original && !self.options.dry_run {
            let out = crate::common::text::to_platform_newlines(&text);
            fs::write(path, out).map_err(|e| format!("无法写入文件 {}: {e}", path.display()))?;
        }
        Ok(())
    }

    fn copy(
        &self,
        arg: &Argument,
        source: &Value,
        report: &mut ArgumentReport,
    ) -> Result<(), String> {
        // Python's `if not source:` — any falsy value aborts this entry.
        let falsy = match source {
            Value::Null => true,
            Value::Bool(b) => !b,
            Value::Number(n) => n.as_f64().map_or(false, |f| f == 0.0),
            Value::String(s) => s.is_empty(),
            _ => false,
        };
        if falsy {
            return self.fail(
                report,
                format!("{}: file_path 为目录但 value 为空", arg.name),
            );
        }

        let mut src = PathBuf::from(display(source));
        if src.is_relative() {
            let candidate = self.root.join(&src);
            if candidate.exists() {
                src = candidate;
            }
        }
        if !src.exists() {
            return self.fail(report, format!("{}: 源文件不存在 {}", arg.name, src.display()));
        }

        let target_dir = self.root.join(&arg.file_path);
        let ext = src
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let target = target_dir.join(format!("{}{}", arg.key_name, ext));
        if !self.options.dry_run {
            fs::create_dir_all(&target_dir)
                .map_err(|e| format!("无法创建目录 {}: {e}", target_dir.display()))?;
            fs::copy(&src, &target)
                .map_err(|e| format!("复制文件失败 {}: {e}", src.display()))?;
        }
        report.copied.push(CopiedEntry {
            argument: arg.name.clone(),
            from: src.display().to_string(),
            to: target.display().to_string(),
        });
        Ok(())
    }
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
            .join(format!("argument_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn table_from(spec: Value) -> ArgumentTable {
        let Value::Object(map) = spec else { unreachable!() };
        ArgumentTable::new(&map, None)
    }

    /// value_override takes the referenced field's bare value (without its
    /// prefix); the own value_prefix is applied idempotently.
    #[test]
    fn override_and_prefix_semantics() {
        let mut table = table_from(json!({
            "game_app_id": {
                "write_mode": "argument",
                "value_prefix": "mi_",
                "value": "123456"
            },
            "ads_app_id": {
                "write_mode": "argument",
                "value_override": "game_app_id",
                "value": ""
            },
            "prefixed_override": {
                "write_mode": "argument",
                "value_prefix": "ad_",
                "value_override": "game_app_id",
                "value": ""
            }
        }));
        let resolved = table.resolve_all().unwrap();
        assert_eq!(resolved.get("game_app_id").unwrap(), &json!("mi_123456"));
        // Bare value inherited: no "mi_" prefix leaks in.
        assert_eq!(resolved.get("ads_app_id").unwrap(), &json!("123456"));
        // Own prefix is applied on top of the bare value.
        assert_eq!(resolved.get("prefixed_override").unwrap(), &json!("ad_123456"));
    }

    #[test]
    fn circular_override_is_reported() {
        let mut table = table_from(json!({
            "a": { "write_mode": "argument", "value_override": "b", "value": "" },
            "b": { "write_mode": "argument", "value_override": "a", "value": "" }
        }));
        let err = table.resolve_all().unwrap_err();
        assert!(err.contains("循环引用"));
    }

    #[test]
    fn unknown_write_modes_are_skipped() {
        let mut table = table_from(json!({
            "arg_one": { "write_mode": "argument", "value": "x" },
            "code_task": { "write_mode": "code", "file_path": "a.java" },
            "plain": "not-an-object"
        }));
        let resolved = table.resolve_all().unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(table.skipped, vec!["code_task".to_string(), "plain".to_string()]);
    }

    /// Full run on a small fixture tree: injection + file copy + dry-run.
    #[test]
    fn kernel_run_on_fixture() {
        let dir = test_dir("fixture");
        fs::create_dir_all(dir.join("res/values")).unwrap();
        fs::create_dir_all(dir.join("cfg")).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(
            dir.join("res/values/strings.xml"),
            "<resources><string name=\"app_name\">old</string></resources>",
        )
        .unwrap();
        fs::write(dir.join("cfg/Cfg.java"),
            "public class Cfg {\n    public static final String Name = \"\";\n    public static final boolean Show = false;\n}\n")
            .unwrap();
        fs::write(dir.join("assets/source.png"), b"png-bytes").unwrap();

        let spec_path = dir.join("config.json");
        fs::write(
            &spec_path,
            json!({
                "app_name": {
                    "write_mode": "argument",
                    "file_path": "res\\values\\strings.xml",
                    "file_type": "xml",
                    "key_name": "app_name",
                    "value": "新名称"
                },
                "cfg_name": {
                    "write_mode": "argument",
                    "file_path": "cfg/Cfg.java",
                    "file_type": "java",
                    "key_name": "Name",
                    "value_override": "app_name",
                    "value": ""
                },
                "cfg_show": {
                    "write_mode": "argument",
                    "file_path": "cfg/Cfg.java",
                    "file_type": "java",
                    "key_name": "Show",
                    "value": true
                },
                "app_image": {
                    "write_mode": "argument",
                    "file_path": "res/drawable",
                    "file_type": "",
                    "key_name": "image",
                    "value": "assets/source.png"
                },
                "some_code": { "write_mode": "code", "file_path": "x.java" }
            })
            .to_string(),
        )
        .unwrap();

        let report = run(&dir, &spec_path, None, KernelOptions::default()).unwrap();
        assert_eq!(report.skipped, vec!["some_code".to_string()]);
        assert!(report.errors.is_empty());
        assert_eq!(report.written.len(), 3);
        assert_eq!(report.copied.len(), 1);
        // Relative source resolved against the project root.
        let copied_target = dir.join("res/drawable/image.png");
        assert_eq!(PathBuf::from(&report.copied[0].to), copied_target);
        assert!(copied_target.exists());

        let xml = fs::read_to_string(dir.join("res/values/strings.xml")).unwrap();
        assert!(xml.contains(">新名称</string>"));
        let java = fs::read_to_string(dir.join("cfg/Cfg.java")).unwrap();
        assert!(java.contains("public static final String Name = \"新名称\";"));
        assert!(java.contains("public static final boolean Show = true;"));

        // dry_run: no disk changes.
        fs::write(
            dir.join("res/values/strings.xml"),
            "<resources><string name=\"app_name\">old</string></resources>",
        )
        .unwrap();
        let dry = KernelOptions {
            dry_run: true,
            strict: true,
        };
        let report = run(&dir, &spec_path, None, dry).unwrap();
        assert_eq!(report.written.len(), 3);
        let xml = fs::read_to_string(dir.join("res/values/strings.xml")).unwrap();
        assert!(xml.contains(">old</string>"));

        let _ = fs::remove_dir_all(&dir);
    }
}

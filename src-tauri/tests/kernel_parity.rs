//! Parity tests against the reference Python kernels.
//!
//! Runs the Rust argument/code kernels on a copy of
//! `E:\neihe_dev\test_project` with `E:\neihe_dev\test.json` and compares
//! the results byte-for-byte against the output produced by running
//! `argument_kernel.py` / `code_kernel.py` on an identical copy
//! (`E:\neihe_dev\_ref_py`). The tests skip silently when the reference
//! assets are not present.

use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

/// Copies `src` into `dst`, skipping the large native binary folders that
/// the kernels never touch (same exclusion set as the reference copy).
fn copy_project_filtered(src: &Path, dst: &Path) {
    if dst.exists() {
        fs::remove_dir_all(dst).unwrap();
    }
    copy_filtered(src, dst);
}

fn copy_filtered(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if entry.file_type().unwrap().is_dir() {
            if matches!(
                name.as_str(),
                "jnilibs" | "jnistaticlibs" | "symbols" | ".gradle" | ".idea"
            ) {
                continue;
            }
            copy_filtered(&entry.path(), &dst.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
        }
    }
}

fn parity_assets() -> Option<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let base = Path::new("E:/neihe_dev");
    let test_json = base.join("test.json");
    let test_project = base.join("test_project");
    let python_ref = base.join("_ref_py");
    if test_json.is_file() && test_project.is_dir() && python_ref.is_dir() {
        Some((test_json, test_project, python_ref))
    } else {
        None
    }
}

fn parity_tmp(name: &str) -> std::path::PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target").join("test_tmp").join(name)
}

/// Files the Python argument kernel modifies; each must come out of the
/// Rust kernel byte-identical to the Python reference copy.
const ARGUMENT_TOUCHED: &[&str] = &[
    "launcher/src/main/res/values/strings.xml",
    "launcher/build.gradle",
    "unityLibrary/src/main/AndroidManifest.xml",
    "unityLibrary/src/main/java/com/unity3d/player/common/config/AdsConfig.java",
    "unityLibrary/src/main/res/drawable/icon.jpg",
    "unityLibrary/src/main/res/drawable/image.png",
];

#[test]
fn argument_kernel_matches_python_reference() {
    let Some((test_json, test_project, python_ref)) = parity_assets() else {
        eprintln!("skip: E:/neihe_dev reference assets not present");
        return;
    };

    let work = parity_tmp("parity_argument");
    copy_project_filtered(&test_project, &work);

    // Loose mode so a full report is produced even on partial failure.
    let options = executor_lib::core::android::argument::KernelOptions {
        dry_run: false,
        strict: false,
    };
    let report =
        executor_lib::core::android::argument::run(&work, test_json.as_path(), None, options).unwrap();

    // Report shape from the Python reference run:
    // 21 argument entries resolved, 1 skipped, 19 written, 2 copied, no errors.
    assert_eq!(report.resolved.len(), 21);
    assert_eq!(report.skipped, vec!["app_code_inject".to_string()]);
    assert_eq!(report.written.len(), 19);
    assert_eq!(report.copied.len(), 2);
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);

    // Spot-checks of the resolved values against the filled-in test data
    // (override takes the bare value, own prefix applies, dates format).
    let resolved = &report.resolved;
    assert_eq!(resolved.get("xiaomi_game_app_id").unwrap(), &serde_json::json!("mi_2882303761520123456"));
    assert_eq!(resolved.get("xiaomi_ads_app_id").unwrap(), &serde_json::json!("2882303761520123456"));
    assert_eq!(resolved.get("xiaomi_ads_app_name").unwrap(), &serde_json::json!("魔界大冒险"));
    assert_eq!(resolved.get("app_version_code").unwrap(), &serde_json::json!(3));
    assert_eq!(resolved.get("app_show_privacy").unwrap(), &serde_json::json!(true));
    assert_eq!(resolved.get("xiaomi_ads_date_limit").unwrap(), &serde_json::json!("2026-06-01 19:00:00"));
    assert_eq!(resolved.get("xiaomi_ads_banner_id").unwrap(), &serde_json::json!("b0dad3317de042b0a6c588155dd83610"));

    // Byte-for-byte equality with the Python-generated files.
    for rel in ARGUMENT_TOUCHED {
        let rust_file = work.join(rel);
        let py_file = python_ref.join(rel);
        assert!(rust_file.exists(), "rust run missing {rel}");
        assert!(py_file.exists(), "python reference missing {rel}");
        let rust_bytes = fs::read(&rust_file).unwrap();
        let py_bytes = fs::read(&py_file).unwrap();
        assert_eq!(rust_bytes, py_bytes, "content differs: {rel}");
    }

    // Untouched files must also stay identical: compare against the
    // pristine test_project (the reference copy may have been injected by
    // the Python code kernel).
    let untouched = "unityLibrary/src/main/java/com/unity3d/player/interfaces/activity/AdsUnityPlayerActivity.java";
    assert_eq!(
        fs::read(work.join(untouched)).unwrap(),
        fs::read(test_project.join(untouched)).unwrap(),
        "untouched file differs: {untouched}"
    );
}

#[test]
fn code_kernel_matches_python_reference() {
    let Some((test_json, test_project, python_ref)) = parity_assets() else {
        eprintln!("skip: E:/neihe_dev reference assets not present");
        return;
    };

    let work = parity_tmp("parity_code");
    copy_project_filtered(&test_project, &work);

    let results = executor_lib::core::android::code::run(&work, test_json.as_path()).unwrap();

    // Python reference: 22 entries -> 21 skipped (write_mode != "code")
    // and 1 successful injection between the paired markers at the end of
    // the target java file.
    assert_eq!(results.len(), 22);
    let skipped = results.iter().filter(|r| r.skipped).count();
    assert_eq!(skipped, 21);
    let executed: Vec<_> = results.iter().filter(|r| !r.skipped).collect();
    assert_eq!(executed.len(), 1);
    assert!(
        executed[0].success,
        "injection failed: {:?}",
        executed[0].error
    );
    assert_eq!(executed[0].methods_generated, 9);
    // Backup is disabled via "backup": false in the config: no .bak file.
    assert!(executed[0].backup.is_none(), "unexpected backup: {:?}", executed[0].backup);

    // The injected file must be byte-identical to the Python reference
    // copy (both kernels ran the same injection).
    let target = "unityLibrary/src/main/java/com/unity3d/player/interfaces/activity/AdsUnityPlayerActivity.java";
    assert_eq!(
        fs::read(work.join(target)).unwrap(),
        fs::read(python_ref.join(target)).unwrap()
    );
}

/// Runs the Rust code kernel against a target that actually carries the
/// injection markers, then diffs the result against the same operation
/// performed with the Python engine.
#[test]
fn code_kernel_injection_roundtrip_with_python() {
    let Some((test_json, _test_project, _python_ref)) = parity_assets() else {
        eprintln!("skip: E:/neihe_dev reference assets not present");
        return;
    };

    let target_rel = "unityLibrary/src/main/java/com/unity3d/player/interfaces/activity/AdsUnityPlayerActivity.java";
    let marker = "//---inject_code_area---";

    // Rust side: add a paired marker region and run the kernel.
    let rust_dir = parity_tmp("parity_roundtrip_rust");
    fs::create_dir_all(rust_dir.join("unityLibrary/src/main/java/com/unity3d/player/interfaces/activity")).unwrap();
    let original = fs::read_to_string(Path::new("E:/neihe_dev/test_project").join(target_rel)).unwrap();
    let with_marker = original.replace(
        "public class AdsUnityPlayerActivity extends BaseUnityPlayerActivity {",
        &format!("public class AdsUnityPlayerActivity extends BaseUnityPlayerActivity {{\n{marker}\n{marker}"),
    );
    let rust_target = rust_dir.join(target_rel);
    fs::write(&rust_target, &with_marker).unwrap();
    let results = executor_lib::core::android::code::run(&rust_dir, test_json.as_path()).unwrap();
    let executed: Vec<_> = results.iter().filter(|r| !r.skipped).collect();
    assert_eq!(executed.len(), 1);
    assert!(executed[0].success, "rust injection failed: {:?}", executed[0].error);
    assert_eq!(executed[0].methods_generated, 9);

    // Python side: identical fixture, run through code_kernel.py.
    let py_dir = parity_tmp("parity_roundtrip_py");
    fs::create_dir_all(py_dir.join("unityLibrary/src/main/java/com/unity3d/player/interfaces/activity")).unwrap();
    let py_target = py_dir.join(target_rel);
    fs::write(&py_target, &with_marker).unwrap();
    let status = std::process::Command::new("python")
        .arg("E:/neihe_dev/code_kernel.py")
        .arg(&py_dir)
        .arg("E:/neihe_dev/test.json")
        .stdout(std::process::Stdio::null())
        .status()
        .expect("python not available");
    assert!(status.success());

    // Byte-for-byte equality of the injected files.
    let rust_out = fs::read(&rust_target).unwrap();
    let py_out = fs::read(&py_target).unwrap();
    if rust_out != py_out {
        // Dump both sides to ease inspection of the first divergence.
        let dump = parity_tmp("parity_roundtrip_dump");
        fs::create_dir_all(&dump).unwrap();
        fs::write(dump.join("rust_out.java"), &rust_out).unwrap();
        fs::write(dump.join("py_out.java"), &py_out).unwrap();
        let common = rust_out.len().min(py_out.len());
        let first_diff = (0..common)
            .find(|i| rust_out[*i] != py_out[*i])
            .unwrap_or(common);
        panic!(
            "injected file differs from Python output at byte {} (rust {} bytes, py {} bytes), dumped to {}",
            first_diff,
            rust_out.len(),
            py_out.len(),
            dump.display()
        );
    }
}

/// Helper for tests that need the spec as a map (currently unused but kept
/// for future value-injection parity checks).
#[allow(dead_code)]
fn load_spec_map(test_json: &Path) -> Map<String, Value> {
    match serde_json::from_str::<Value>(&fs::read_to_string(test_json).unwrap()).unwrap() {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

//! Multi-round acceptance test: repeatedly modify the config JSON (argument
//! values + code scenes), re-run both kernels and verify:
//!
//! 1. values/code are updated in place on every round (no duplicates);
//! 2. file copy overwrites same-name targets (a corrupted target is
//!    restored to the source bytes);
//! 3. other files in the target directory are never touched
//!    (sentinel file + the pre-existing extensionless leftovers stay
//!    byte-identical).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

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

fn work_dir() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("target").join("test_tmp").join("multi_round");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Applies one round of argument-value edits on top of the base config.
fn round_config(base: &Value, name: &str, version: &str, code: i64, banner: &str, date: &str, orientation: &str) -> Value {
    let mut config = base.clone();
    config["app_name"]["value"] = json!(name);
    config["app_version"]["value"] = json!(version);
    config["app_version_code"]["value"] = json!(code);
    config["xiaomi_ads_banner_id"]["value"] = json!(banner);
    config["xiaomi_ads_date_limit"]["value"] = json!(date);
    config["app_screen_orientation"]["value"] = json!(orientation);
    config
}

fn write_config(dir: &Path, round: &str, config: &Value) -> PathBuf {
    let path = dir.join(format!("config_{round}.json"));
    fs::write(&path, config.to_string()).unwrap();
    path
}

fn gradle(work: &Path) -> String {
    fs::read_to_string(work.join("launcher/build.gradle")).unwrap()
}

fn ads_config(work: &Path) -> String {
    fs::read_to_string(work.join(
        "unityLibrary/src/main/java/com/unity3d/player/common/config/AdsConfig.java",
    ))
    .unwrap()
}

fn injected_file(work: &Path) -> String {
    fs::read_to_string(work.join(
        "unityLibrary/src/main/java/com/unity3d/player/interfaces/activity/AdsUnityPlayerActivity.java",
    ))
    .unwrap()
}

#[test]
fn multi_round_edit_overwrite_and_untouched_files() {
    let base_assets = Path::new("E:/neihe_dev");
    let test_json = base_assets.join("test.json");
    let test_project = base_assets.join("test_project");
    if !test_json.is_file() || !test_project.is_dir() {
        eprintln!("skip: E:/neihe_dev assets not present");
        return;
    }
    let base: Value = serde_json::from_str(&fs::read_to_string(&test_json).unwrap()).unwrap();

    let icon_source = PathBuf::from(base["app_icon"]["value"].as_str().unwrap());
    let image_source = PathBuf::from(base["app_image"]["value"].as_str().unwrap());
    if !icon_source.is_file() || !image_source.is_file() {
        eprintln!("skip: copy source files not present");
        return;
    }

    let work = work_dir();
    copy_filtered(&test_project, &work);

    let drawable = work.join("unityLibrary/src/main/res/drawable");
    let icon_target = drawable.join("icon.jpg");
    let image_target = drawable.join("image.png");

    // Sentinels: a brand-new file plus the pre-existing extensionless
    // leftovers — none of them shares a name with a copy target.
    let sentinel = drawable.join("keep_me.txt");
    fs::write(&sentinel, "sentinel-do-not-touch").unwrap();
    let stray_names: Vec<String> = fs::read_dir(&drawable)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n != "icon.jpg" && n != "image.png")
        .collect();
    let stray_bytes: Vec<(String, Vec<u8>)> = stray_names
        .iter()
        .map(|n| (n.clone(), fs::read(drawable.join(n)).unwrap()))
        .collect();

    // Corrupt the same-name targets: only an overwriting copy restores them.
    fs::write(&icon_target, b"corrupted-icon").unwrap();
    fs::write(&image_target, b"corrupted-image").unwrap();

    let options = executor_lib::core::android::argument::KernelOptions::default();

    // ---------------- round 1 ------------------------------------------
    let r1 = round_config(&base, "深渊远征", "1.0.4", 4, "round1-banner-id", "2026-07-01 12:00:00", "portrait");
    let r1_path = write_config(&work, "r1", &r1);
    let report = executor_lib::core::android::argument::run(&work, &r1_path, None, options).unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let g = gradle(&work);
    assert!(g.contains("versionName '1.0.4'"));
    assert!(g.contains("versionCode 4"));
    assert!(ads_config(&work).contains("BannerID = \"round1-banner-id\""));
    assert!(ads_config(&work).contains("AppName = \"深渊远征\""));

    // Copy overwrites the corrupted same-name files with the source bytes.
    assert_eq!(fs::read(&icon_target).unwrap(), fs::read(&icon_source).unwrap());
    assert_eq!(fs::read(&image_target).unwrap(), fs::read(&image_source).unwrap());

    // Code round 1: scenes injected between the markers.
    let results = executor_lib::core::android::code::run(&work, &r1_path).unwrap();
    assert!(results.iter().filter(|r| !r.skipped).all(|r| r.success));
    let java = injected_file(&work);
    assert!(java.contains("public void VictoryGameLevelAD() {"));
    assert!(java.contains("advertiseComplianceJob.secondsLimit(\"native\", 30, this::tryShowNative);"));

    // ---------------- round 2: different values + changed scenes --------
    let mut r2 = round_config(&base, "星海征途", "1.0.5", 5, "round2-banner-id", "2026-08-01 08:30:00", "landscape");
    // Code change: existing scene gets new rule args, plus a brand-new scene.
    r2["app_code_inject"]["scenes"]["VictoryGameLevelAD"]["body"][0]["rule"]["args"] =
        json!(["banner", 45]);
    r2["app_code_inject"]["scenes"]["RoundTwoScene"] = json!({
        "body": [
            { "type": "direct", "call": { "callback": "roundTwoCall", "args": ["tag", 2] } }
        ]
    });
    let r2_path = write_config(&work, "r2", &r2);
    let report = executor_lib::core::android::argument::run(&work, &r2_path, None, options).unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let g = gradle(&work);
    assert!(g.contains("versionName '1.0.5'"), "round1 value must be overwritten");
    assert!(!g.contains("versionName '1.0.4'"));
    assert!(g.contains("versionCode 5"));
    let cfg = ads_config(&work);
    assert!(cfg.contains("BannerID = \"round2-banner-id\""));
    assert!(!cfg.contains("round1-banner-id"));
    assert!(cfg.contains("AppName = \"星海征途\""));
    assert!(cfg.contains("DateLimit = \"2026-08-01 08:30:00\""));

    // Same-name targets overwritten again (still the source bytes).
    assert_eq!(fs::read(&icon_target).unwrap(), fs::read(&icon_source).unwrap());
    assert_eq!(fs::read(&image_target).unwrap(), fs::read(&image_source).unwrap());

    let results = executor_lib::core::android::code::run(&work, &r2_path).unwrap();
    assert!(results.iter().filter(|r| !r.skipped).all(|r| r.success));
    let java = injected_file(&work);
    // The VictoryGameLevelAD block now uses the round-2 args.
    let victory_block = java
        .split("public void VictoryGameLevelAD() {")
        .nth(1)
        .and_then(|rest| rest.split("    }").next())
        .unwrap_or("");
    assert!(victory_block.contains("secondsLimit(\"banner\", 45, this::tryShowNative);"));
    assert!(!victory_block.contains("\"native\", 30"));
    // New scene present; no duplicated method definitions.
    assert!(java.contains("public void RoundTwoScene() {"));
    assert!(java.contains("roundTwoCall(\"tag\", 2);"));
    assert_eq!(java.matches("public void VictoryGameLevelAD() {").count(), 1);
    assert_eq!(java.matches("public void RoundTwoScene() {").count(), 1);
    assert_eq!(java.matches("//---inject_code_area---").count(), 2);

    // ---------------- untouched files -----------------------------------
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "sentinel-do-not-touch");
    for (name, bytes) in &stray_bytes {
        assert_eq!(
            &fs::read(drawable.join(name)).unwrap(),
            bytes,
            "differently-named file was modified: {name}"
        );
    }
    // No unexpected entries appeared in the drawable directory.
    let after_names: Vec<String> = fs::read_dir(&drawable)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let mut expected: Vec<String> = stray_names.clone();
    expected.push("icon.jpg".into());
    expected.push("image.png".into());
    expected.sort();
    let mut after_sorted = after_names;
    after_sorted.sort();
    assert_eq!(after_sorted, expected, "directory entries changed unexpectedly");
}

//! Runs both kernels directly on a target project (same input shape as the
//! Python CLIs: config JSON path + project folder).
//!
//! Usage:
//!   cargo run --example run_kernels -- <config.json> <project_root> [--dry-run] [--loose]

use std::path::Path;
use std::process::ExitCode;

use executor_lib::core::android::{argument, code};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let positional: Vec<&str> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();
    if positional.len() != 2 {
        eprintln!("用法: cargo run --example run_kernels -- <config.json> <项目根目录> [--dry-run] [--loose]");
        return ExitCode::FAILURE;
    }
    let config = Path::new(positional[0]);
    let project = Path::new(positional[1]);
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let loose = args.iter().any(|a| a == "--loose");

    // -- argument kernel --------------------------------------------
    println!("== argument 内核: {} -> {}", config.display(), project.display());
    let options = argument::KernelOptions {
        dry_run,
        strict: !loose,
    };
    match argument::run(project, config, None, options) {
        Ok(report) => {
            println!("解析字段: {} 个", report.resolved.len());
            for (name, value) in &report.resolved {
                println!("  {name} = {value}");
            }
            println!("跳过字段: {:?}", report.skipped);
            println!("写入 {} 处:", report.written.len());
            for entry in &report.written {
                println!("  [{}] {} = {}", entry.file, entry.key, entry.value);
            }
            println!("复制 {} 个文件:", report.copied.len());
            for entry in &report.copied {
                println!("  {} -> {}", entry.from, entry.to);
            }
            if !report.errors.is_empty() {
                println!("错误 {} 条:", report.errors.len());
                for error in &report.errors {
                    println!("  {error}");
                }
            }
        }
        Err(e) => {
            eprintln!("[失败] {e}");
            return ExitCode::FAILURE;
        }
    }

    // -- code kernel ---------------------------------------------------
    println!("\n== code 内核: {} -> {}", config.display(), project.display());
    match code::run(project, config) {
        Ok(results) => {
            let mut success = 0usize;
            let mut fail = 0usize;
            for result in &results {
                if result.success {
                    if result.skipped {
                        println!(
                            "[跳过] {} - {}",
                            result.file.as_deref().unwrap_or("未知"),
                            result.reason.as_deref().unwrap_or("")
                        );
                    } else {
                        println!(
                            "[成功] {} (区域: {}, 生成方法: {} 个, 备份: {})",
                            result.file.as_deref().unwrap_or("未知"),
                            result.area.as_deref().unwrap_or("N/A"),
                            result.methods_generated,
                            result.backup.as_deref().unwrap_or("无")
                        );
                    }
                    success += 1;
                } else {
                    println!(
                        "[失败] {} - 原因: {}",
                        result.file.as_deref().unwrap_or("Unknown"),
                        result.error.as_deref().unwrap_or("")
                    );
                    fail += 1;
                }
            }
            println!("------------------------------");
            println!("处理完成. 成功: {success}, 失败: {fail}");
            if fail > 0 {
                return ExitCode::FAILURE;
            }
        }
        Err(e) => {
            eprintln!("[失败] {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

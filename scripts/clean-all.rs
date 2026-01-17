#!/usr/bin/env cargo-script
---
[dependencies]
anyhow = "1"
walkdir = "2"
glob = "0.3"
---

use anyhow::Result;
use std::{env, fs, path::Path, process::Command};
use walkdir::WalkDir;

fn remove_file(path: &Path, cleaned: &mut Vec<String>, label: &str) {
    if path.exists() {
        if path.is_file() {
            if let Err(e) = fs::remove_file(path) {
                eprintln!("Failed to remove {}: {}", path.display(), e);
            } else {
                println!("   → Removing {}", path.display());
                cleaned.push(label.to_string());
            }
        } else if path.is_dir() {
            if let Err(e) = fs::remove_dir_all(path) {
                eprintln!("Failed to remove {}: {}", path.display(), e);
            } else {
                println!("   → Removing {}", path.display());
                cleaned.push(label.to_string());
            }
        }
    }
}

fn remove_temp_files(cleaned: &mut Vec<String>) -> usize {
    let patterns = ["*.tmp", "*.bak", "*.orig", "*.log", "*.swp", "*.swo"];
    let mut count = 0;

    for pattern in patterns {
        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if glob::Pattern::new(pattern).unwrap().matches(name) {
                        if fs::remove_file(entry.path()).is_ok() {
                            println!("   → Removing {}", entry.path().display());
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    let mut ds_count = 0;
    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if entry.file_name() == ".DS_Store" {
                if fs::remove_file(entry.path()).is_ok() {
                    ds_count += 1;
                }
            }
        }
    }
    if ds_count > 0 {
        println!("   → Removing .DS_Store files ({})", ds_count);
        count += ds_count;
    }

    count
}

fn main() -> Result<()> {
    println!("🧹 Cleaning all generated files and build artifacts...\n");

    let project_root = env::current_dir()?;

    let mut cleaned_items: Vec<String> = vec![];

    println!("🦀 Cleaning Rust build artifacts...");
    remove_file(Path::new(".cargo"), &mut cleaned_items, "Rust .cargo/");
    remove_file(Path::new("target"), &mut cleaned_items, "Rust target/");
    remove_file(Path::new("Cargo.lock"), &mut cleaned_items, "Cargo.lock");

    println!("\n🤖 Cleaning Android platform artifacts...");
    remove_file(
        Path::new("android/app/src/main/kotlin/uniffi/"),
        &mut cleaned_items,
        "Android: uniffi bindings",
    );
    remove_file(
        Path::new("android/app/src/main/jniLibs"),
        &mut cleaned_items,
        "Android: jniLibs/",
    );
    remove_file(
        Path::new("android/build"),
        &mut cleaned_items,
        "Android: build/",
    );
    remove_file(
        Path::new("android/app/build"),
        &mut cleaned_items,
        "Android: app/build/",
    );
    remove_file(
        Path::new("android/.gradle"),
        &mut cleaned_items,
        "Android: .gradle/",
    );
    remove_file(
        Path::new("android/.kotlin"),
        &mut cleaned_items,
        "Android: .kotlin/",
    );
    remove_file(
        Path::new("android/.konan"),
        &mut cleaned_items,
        "Android: .konan/",
    );
    remove_file(
        Path::new("android/.cxx"),
        &mut cleaned_items,
        "Android: .cxx/",
    );

    for entry in WalkDir::new("android/app/src/jvmMain")
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ["so", "dylib", "dll"].contains(&ext.to_str().unwrap_or("")) {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
    }
    cleaned_items.push("Android: JVM native libs".to_string());

    println!("\n📚 Cleaning generated documentation...");
    remove_file(Path::new("docs/lib"), &mut cleaned_items, "Docs: lib/");

    println!("\n🗑️  Cleaning temporary files...");
    let temp_count = remove_temp_files(&mut cleaned_items);
    if temp_count > 0 {
        cleaned_items.push(format!("{} temporary files", temp_count));
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("✨ Cleanup Complete!");
    println!("════════════════════════════════════════════════════════════\n");

    if cleaned_items.is_empty() {
        println!("✓ No generated files found - workspace was already clean");
    } else {
        println!("Cleaned {} categories:", cleaned_items.len());
        for item in cleaned_items {
            println!("  ✓ {}", item);
        }
    }

    println!("\nTo rebuild everything, run:");
    println!("  ./scripts/build-all.sh\n");

    Ok(())
}

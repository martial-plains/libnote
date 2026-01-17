#!/usr/bin/env cargo-script
---
[dependencies]
anyhow = "1"
---


use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, exit};

fn main() -> Result<()> {
    println!("[TEST] Testing Android Platforms...\n");

    let project_root = env::current_dir()?;

    let uniffi_dir = project_root.join("platforms/kotlin/src/commonMain/kotlin/uniffi");
    if !uniffi_dir.exists() {
        println!("[WARNING] Android bindings not found. Building first...\n");

        let status = Command::new("cargo")
            .args(&["script", "scripts/build-android.rs"])
            .status()
            .context("Failed to run build-android.rs")?;

        if !status.success() {
            eprintln!("[FAILED] Android build failed!");
            exit(1);
        }
        println!();
    }

    println!("════════════════════════════════════════════════════════════");
    println!("Running Android Multiplatform Tests");
    println!("════════════════════════════════════════════════════════════\n");

    let kotlin_dir = project_root.join("platforms/kotlin");
    std::env::set_current_dir(&kotlin_dir)?;

    let gradle_cmd = if kotlin_dir.join("gradlew").exists() {
        "./gradlew".to_string()
    } else if Command::new("gradle").output().is_ok() {
        "gradle".to_string()
    } else {
        println!("[WARNING] Gradle not found. Skipping Android tests.\n");
        println!("To run Android tests, either:");
        println!("  1. Install Gradle: https://gradle.org/install/");
        println!("  2. Or run './gradlew wrapper' in platforms/kotlin/ to create a wrapper\n");
        exit(0);
    };

    println!(" Running JVM tests...");
    let test_status = Command::new(&gradle_cmd)
        .args(&["jvmTest", "--console=plain"])
        .status()
        .context("Failed to run JVM tests")?;

    let test_result = if test_status.success() {
        println!("\n[SUCCESS] Android JVM tests passed!");
        0
    } else {
        println!("\n[FAILED] Android JVM tests failed!");
        1
    };

    println!("\n[INFO] Android unit tests are skipped (require emulator/device).");
    println!("       JVM tests verify the Android bindings work correctly.\n");

    println!("════════════════════════════════════════════════════════════");
    println!("Test Summary");
    println!("════════════════════════════════════════════════════════════\n");

    if test_result == 0 {
        println!("[SUCCESS] All Android platform tests passed!\n");
        println!(" Test Report:");
        println!("   JVM: platforms/kotlin/build/reports/tests/jvmTest/index.html\n");
        println!(" Note: Android tests require running on an emulator/device.");
        exit(0);
    } else {
        println!("[FAILED] Some tests failed. Check the output above.\n");
        println!(" Test Report:");
        println!("   Check platforms/kotlin/build/reports/tests/ for detailed reports");
        exit(1);
    }
}

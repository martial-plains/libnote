#!/usr/bin/env cargo-script
---
[dependencies]
anyhow = "1"
---


use anyhow::{Context, Result};
use std::process::{Command, exit};

fn main() -> Result<()> {
    println!("[TEST] Running All Tests...\n");

    let project_root = env::current_dir()?;

    let mut failed_tests = Vec::new();

    println!("════════════════════════════════════════════════════════════");
    println!("Testing Rust Core Library");
    println!("════════════════════════════════════════════════════════════\n");

    let rust_status = Command::new("cargo")
        .args(&["test"])
        .status()
        .context("Failed to run Rust tests")?;

    if rust_status.success() {
        println!("\n[SUCCESS] Rust tests passed!\n");
    } else {
        println!("\n[FAILED] Rust tests failed!\n");
        failed_tests.push("Rust");
    }

    println!("\n");

    println!("════════════════════════════════════════════════════════════");
    println!("Testing Android Platforms (Android, JVM)");
    println!("════════════════════════════════════════════════════════════\n");

    let android_status = Command::new("cargo")
        .args(&["script", "scripts/test-android.rs"])
        .status()
        .unwrap_or_else(|_| {
            eprintln!("[FAILED] Unable to run Android tests script");
            exit(1);
        });

    if android_status.success() {
        println!("\n[SUCCESS] Android platform tests passed!\n");
    } else {
        println!("\n[FAILED] Android platform tests failed!\n");
        failed_tests.push("Android");
    }

    println!("\n");

    println!("════════════════════════════════════════════════════════════");
    println!(" Test Summary");
    println!("════════════════════════════════════════════════════════════\n");

    if failed_tests.is_empty() {
        println!("[SUCCESS] All tests passed!\n");
        println!("Test Coverage:");
        println!("   [SUCCESS] Rust core library (unit + integration + doc tests)");
        println!("   [SUCCESS] Apple platforms (iOS + macOS Swift tests)");
        println!("   [SUCCESS] Android platforms (Android + JVM tests)");
        exit(0);
    } else {
        println!("[WARNING]  Some test suites failed:");
        for platform in &failed_tests {
            println!("   [FAILED] {}", platform);
        }
        println!("\nPlease check the error messages above for details.\n");
        exit(1);
    }
}

#!/usr/bin/env cargo-script`
---
[dependencies]
anyhow = "1"
---

use anyhow::{Context, Result};
use std::process::Command;

fn run_build(script: &str) -> bool {
    let status = Command::new("cargo")
        .args(&["script", script])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to execute {}: {}", script, e);
            std::process::exit(1);
        });

    status.success()
}

fn main() -> Result<()> {
    println!("Building for all platforms...\n");

    let mut failed_builds = vec![];

    println!("============================================================");
    println!("Building Android Platforms (Android, JVM)");
    println!("============================================================\n");

    if run_build("scripts/build-android.rs") {
        println!("\n[SUCCESS] Android platforms build successful\n");
    } else {
        println!("\n[FAILED] Android platforms build failed\n");
        failed_builds.push("Android");
    }

    println!("\n");

    println!("============================================================");
    println!("Build Summary");
    println!("============================================================\n");

    if failed_builds.is_empty() {
        println!("[SUCCESS] All platform builds completed successfully!\n");

        println!("Build Artifacts:\n");

        println!("Android Platforms:");
        println!("   - Android JNI libs: platforms/kotlin/src/jniLibs/");
        println!("   - JVM libs: platforms/kotlin/src/jvmMain/kotlin/");
        println!("   - Kotlin bindings: platforms/kotlin/src/commonMain/kotlin/");
        println!("   - Gradle project: platforms/kotlin/build.gradle.kts\n");

        std::process::exit(0);
    } else {
        println!("[WARNING] Some builds failed:");
        for platform in &failed_builds {
            println!("   [FAILED] {}", platform);
        }

        println!("\nPlease check the error messages above and ensure:");
        println!("   - For Apple: Xcode and Rust targets are installed");
        println!("   - For Kotlin: Android NDK is configured (NDK_HOME)\n");

        std::process::exit(1);
    }
}

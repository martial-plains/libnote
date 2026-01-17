#!/usr/bin/env cargo-script
---
[dependencies]
which = { version = "8.0.0" }
---

use std::env;
use std::path::Path;
use std::process::{Command, exit};

/// Portable command detection using the `which` crate
fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Helper function to run a command and check success
fn run_command(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn main() {
    println!("==> Setup checks for dot-dash");

    if !command_exists("rustup") {
        eprintln!("ERROR: rustup not found in PATH");
        exit(1);
    }

    if !command_exists("cargo") {
        eprintln!("ERROR: cargo not found in PATH");
        exit(1);
    }

    println!("-> Ensuring required Rust targets are installed");

    let rust_targets = vec![
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
        "i686-linux-android",
    ];

    #[cfg(target_os = "macos")]
    rust_targets.extend(["aarch64-apple-darwin", "x86_64-apple-darwin"]);

    for target in rust_targets.iter() {
        let is_installed = Command::new("rustup")
            .arg("target")
            .arg("list")
            .arg("--installed")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(*target))
            .unwrap_or(false);

        if is_installed {
            println!("  Target already installed: {}", target);
        } else {
            println!("  Installing target: {}", target);
            if let Err(_) = Command::new("rustup")
                .arg("target")
                .arg("add")
                .arg(*target)
                .output()
            {
                eprintln!("  Failed to install target: {}", target);
            }
        }
    }

    println!("-> Checking Android NDK (if Android builds will be used)");
    if let Some(ndk_home) = env::var_os("NDK_HOME") {
        let ndk_path = Path::new(&ndk_home);
        if !ndk_path.exists() {
            eprintln!(
                "ERROR: NDK_HOME is set but directory not found: {}",
                ndk_path.display()
            );
            exit(1);
        } else {
            println!("  Using NDK_HOME: {}", ndk_path.display());
        }
    } else {
        println!("  Warning: NDK_HOME is not set. Android builds may fail.");
    }

    println!("-> Checking uniffi-bindgen availability");
    if run_command(
        "cargo",
        &["run", "--bin", "uniffi-bindgen", "--", "--version"],
    ) {
        println!("  uniffi-bindgen available via cargo run");
    } else {
        println!("  Note: uniffi-bindgen not invokable via cargo run -- this may be fine.");
    }

    println!("-> Checking basic tools");

    for cmd in ["git"].iter() {
        if !command_exists(cmd) {
            eprintln!("ERROR: required command not found: {}", cmd);
            exit(1);
        }
    }

    #[cfg(unix)]
    for cmd in ["sed", "awk", "uname"].iter() {
        if !command_exists(cmd) {
            eprintln!("ERROR: required command not found: {}", cmd);
            exit(1);
        }
    }

    println!("Setup checks complete.");
}

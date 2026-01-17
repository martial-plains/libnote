#!/usr/bin/env cargo-script
---
[dependencies]
anyhow = "1"
dirs = "4"
glob = "0.3"
---

use anyhow::{Result, anyhow};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    println!("🛠️  Building Android library for Android & JVM...");

    let project_root = env::current_dir()?;

    let ndk_home = find_ndk_home()?;
    if let Some(ref ndk) = ndk_home {
        println!("[SUCCESS] NDK_HOME: {}", ndk.display());
    } else {
        println!("[WARNING] NDK_HOME not set – skipping Android build");
    }

    let platform = env::consts::OS;
    let arch = env::consts::ARCH;

    if let Some(ndk) = ndk_home {
        build_android(&ndk)?;
    }

    build_jvm(platform, arch)?;

    generate_android_bindings(platform)?;

    println!("\n✅ Android build complete!");
    Ok(())
}

/// Find the Android NDK directory safely and portably.
/// Search order:
/// 1. NDK_HOME
/// 2. ANDROID_NDK_HOME
/// 3. ANDROID_HOME/SDK/ndk
/// 4. ANDROID_SDK_ROOT/ndk
/// 5. android/local.properties sdk.dir/ndk
pub fn find_ndk_home() -> Result<Option<PathBuf>> {
    // 1. NDK_HOME (recommended override)
    if let Ok(ndk_home) = env::var("NDK_HOME") {
        let path = PathBuf::from(&ndk_home);
        if path.exists() {
            return Ok(Some(path));
        }
    }

    // 2. ANDROID_NDK_HOME
    if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        let path = PathBuf::from(&ndk_home);
        if path.exists() {
            return Ok(Some(path));
        }
    }

    // 3. ANDROID_HOME (official Android variable)
    if let Ok(android_home) = env::var("ANDROID_HOME") {
        if let Some(ndk) = search_ndk_in_sdk(Path::new(&android_home))? {
            return Ok(Some(ndk));
        }
    }

    // 4. ANDROID_SDK_ROOT (alternate official variable)
    if let Ok(sdk_root) = env::var("ANDROID_SDK_ROOT") {
        if let Some(ndk) = search_ndk_in_sdk(Path::new(&sdk_root))? {
            return Ok(Some(ndk));
        }
    }

    // 5. android/local.properties file
    if let Ok(sdk_root) = fs::read_to_string("android/local.properties") {
        for line in sdk_root.lines() {
            if line.starts_with("sdk.dir=") {
                let sdk_path = line.trim_start_matches("sdk.dir=");
                if let Some(ndk) = search_ndk_in_sdk(Path::new(sdk_path))? {
                    return Ok(Some(ndk));
                }
            }
        }
    }

    // Nothing found
    Ok(None)
}

/// Search inside `sdk/ndk/*` for highest version NDK.
fn search_ndk_in_sdk(sdk_path: &Path) -> Result<Option<PathBuf>> {
    let ndk_dir = sdk_path.join("ndk");
    if !ndk_dir.exists() {
        return Ok(None);
    }

    let mut versions = vec![];

    for entry in fs::read_dir(&ndk_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            versions.push(path);
        }
    }

    if versions.is_empty() {
        return Ok(None);
    }

    // Pick newest NDK version folder by name
    versions.sort_by(|a, b| b.file_name().unwrap().cmp(a.file_name().unwrap()));

    Ok(Some(versions[0].clone()))
}

fn build_android(ndk_home: &Path) -> Result<()> {
    println!("\n📱 Building for Android...");

    let host_os = match env::consts::OS {
        "macos" => "darwin-x86_64",
        "linux" => "linux-x86_64",
        "windows" => "windows-x86_64",
        _ => "unknown",
    };
    let toolchain = ndk_home.join("toolchains/llvm/prebuilt").join(host_os);
    if !toolchain.exists() {
        println!(
            "[WARNING] NDK toolchain not found at: {}",
            toolchain.display()
        );
        return Ok(());
    }
    println!("[SUCCESS] NDK toolchain verified");

    let targets = [
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "i686-linux-android",
        "x86_64-linux-android",
    ];
    for target in &targets {
        let _ = Command::new("rustup")
            .args(&["target", "add", target])
            .status();
    }

    let cargo_config_dir = Path::new(".cargo");
    fs::create_dir_all(cargo_config_dir)?;
    let config_toml = cargo_config_dir.join("config.toml");
    let toolchain_unix = to_unix_path(&toolchain);
    let clang_aarch64 = linker_name("aarch64-linux-android21-clang");
    let clang_armv7 = linker_name("armv7a-linux-androideabi21-clang");
    let clang_i686 = linker_name("i686-linux-android21-clang");
    let clang_x86_64 = linker_name("x86_64-linux-android21-clang");
    let config_content = format!(
        r#"[target.aarch64-linux-android]
ar = "{tc}/bin/llvm-ar"
linker = "{tc}/bin/{clang_aarch64}"
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384",]

[target.armv7-linux-androideabi]
ar = "{tc}/bin/llvm-ar"
linker = "{tc}/bin/{clang_armv7}"
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384",]

[target.i686-linux-android]
ar = "{tc}/bin/llvm-ar"
linker = "{tc}/bin/{clang_i686}"
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384",]

[target.x86_64-linux-android]
ar = "{tc}/bin/llvm-ar"
linker = "{tc}/bin/{clang_x86_64}"
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384",]
"#,
        tc = toolchain_unix,
        clang_aarch64 = clang_aarch64,
        clang_armv7 = clang_armv7,
        clang_i686 = clang_i686,
        clang_x86_64 = clang_x86_64,
    );

    fs::write(config_toml, config_content)?;

    let arch_dirs = [
        ("aarch64-linux-android", "arm64-v8a"),
        ("armv7-linux-androideabi", "armeabi-v7a"),
        ("i686-linux-android", "x86"),
        ("x86_64-linux-android", "x86_64"),
    ];
    for (target, arch) in &arch_dirs {
        println!(" Building {}...", arch);
        run_command("cargo", &["build", "--release", "--target", target])?;
    }

    for (target, arch) in &arch_dirs {
        let src = Path::new("target")
            .join(target)
            .join("release")
            .join("libnote.so");
        let dest = Path::new("android/app/src/main/jniLibs").join(arch);
        fs::create_dir_all(&dest)?;
        fs::copy(&src, dest.join("libnote.so"))?;
    }

    println!("[SUCCESS] Android build complete!");
    Ok(())
}

fn build_jvm(platform: &str, arch: &str) -> Result<()> {
    println!("\n☕ Building for JVM...");
    let dest_dir = Path::new("android/app/src/main/jniLibs");
    fs::create_dir_all(dest_dir)?;

    let is_repo = env::var("IS_REPO").is_ok();

    // Determine target file paths
    let (target_path, dest_file_name) = match platform {
        "macos" => {
            println!(" Building for macOS Apple Silicon...");
            run_command(
                "cargo",
                &["build", "--release", "--target", "aarch64-apple-darwin"],
            )?;
            (
                Path::new("target/aarch64-apple-darwin/release/libnote.dylib"),
                "libnote.dylib",
            )
        }
        "linux" => {
            println!(" Building for Linux x86_64...");
            run_command("cargo", &["build", "--release"])?;
            (Path::new("target/release/libnote.so"), "libnote.so")
        }
        "windows" => {
            println!(" Building for Windows x86_64...");
            run_command("cargo", &["build", "--release"])?;
            (Path::new("target/release/note.dll"), "note.dll")
        }
        _ => {
            println!("[WARNING] Unsupported platform for JVM: {}", platform);
            return Ok(()); // Exit early if unsupported
        }
    };

    if false {
        fs::copy(target_path, dest_dir.join(dest_file_name))?;
    }

    Ok(())
}

fn generate_android_bindings(platform: &str) -> Result<()> {
    println!("\n🔧 Generating Android bindings...");
    let out_dir = Path::new("android/app/src/main/kotlin");
    fs::create_dir_all(out_dir)?;

    let lib_path = match platform {
        "macos" => "target/aarch64-apple-darwin/release/libnote.dylib",
        "linux" => "target/release/libnote.so",
        "windows" => "target/release/note.dll",
        _ => return Ok(()),
    };

    run_command(
        "cargo",
        &[
            "run",
            "--bin",
            "uniffi-bindgen",
            "--",
            "generate",
            "--library",
            lib_path,
            "--language",
            "kotlin",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ],
    )?;
    Ok(())
}

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd).args(args).spawn()?.wait()?;
    if !status.success() {
        return Err(anyhow!("Command {} failed", cmd));
    }
    Ok(())
}

fn to_unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn linker_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.cmd")
    } else {
        base.to_string()
    }
}

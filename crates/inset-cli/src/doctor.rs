//! `cargo inset doctor`: what this machine can build for, and the command that fixes each gap.

use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::android;
use crate::tools;

pub fn report() -> Result<()> {
    let here = Path::new(".");
    let targets = tools::installed_targets(here).unwrap_or_default();

    section("Rust");
    match tools::which("rustup") {
        Some(_) => {
            let toolchain = tools::output(
                Command::new("rustup").args(["show", "active-toolchain"]),
                "rustup show",
            )
            .unwrap_or_default();
            ok(&format!("toolchain: {}", toolchain.trim()));
        }
        None => gap("rustup is not on PATH", "install from https://rustup.rs"),
    }

    section("Web");
    target(&targets, "wasm32-unknown-unknown");
    match tools::locked_wasm_bindgen_version(here) {
        Ok(version) => {
            let cached = tools::cache_dir()
                .join(format!("wasm-bindgen-{version}"))
                .join(tools::exe_name("wasm-bindgen"))
                .is_file();
            if cached || tools::which("wasm-bindgen").is_some() {
                ok(&format!("wasm-bindgen {version}"));
            } else {
                note(&format!(
                    "wasm-bindgen {version} downloads on the first web build"
                ));
            }
        }
        Err(_) => note("no Cargo.lock here; wasm-bindgen is checked inside a project"),
    }
    match tools::which("wasm-opt") {
        Some(_) => ok("wasm-opt"),
        None => note("wasm-opt: downloads on the first release web build"),
    }

    section("iOS");
    if cfg!(target_os = "macos") {
        match tools::output(Command::new("xcode-select").arg("-p"), "xcode-select") {
            Ok(path) => ok(&format!("Xcode: {}", path.trim())),
            Err(_) => gap("Xcode command line tools", "xcode-select --install"),
        }
        target(&targets, "aarch64-apple-ios-sim");
        target(&targets, "aarch64-apple-ios");
        match tools::output(
            Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]),
            "security",
        ) {
            Ok(listing) => {
                let names: Vec<&str> = listing
                    .lines()
                    .filter(|l| l.contains("Apple Development"))
                    .filter_map(|l| l.split('"').nth(1))
                    .collect();
                if names.is_empty() {
                    note(
                        "no Apple Development certificate: simulator only; sign in to Xcode (Settings > Accounts) for a device",
                    );
                }
                for name in names {
                    ok(&format!("device runs sign with {name}"));
                }
            }
            Err(_) => note("could not query the keychain"),
        }
    } else {
        note("iOS needs macOS");
    }

    section("macOS distribution");
    env_present("APPLE_SIGNING_IDENTITY", "signs .app and .dmg");
    env_present(
        "APPLE_ID",
        "notarizes with APPLE_PASSWORD and APPLE_TEAM_ID",
    );

    section("Android");
    match android::sdk() {
        Ok(sdk) => {
            ok(&format!("SDK: {}", sdk.root.display()));
            ok(&format!(
                "build-tools {}",
                sdk.build_tools
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
            ok(&format!("platform API {}", sdk.platform.level));
            match &sdk.ndk {
                Some(ndk) => ok(&format!("NDK: {}", ndk.display())),
                None => gap(
                    "NDK: links the app for the device",
                    "Android Studio > Settings > SDK Manager > SDK Tools > NDK, or set ANDROID_NDK_HOME",
                ),
            }
            match &sdk.java {
                Some(java) => ok(&format!("Java: {}", java.home.display())),
                None => gap(
                    "Java: runs apksigner",
                    "install Android Studio, whose runtime is used, or set JAVA_HOME",
                ),
            }
            target(&targets, "aarch64-linux-android");
            if sdk.emulator.is_some() {
                let avds = android::emulators().unwrap_or_default();
                if avds.is_empty() {
                    note(
                        "emulator installed, no virtual device: create one in Android Studio > Device Manager",
                    );
                } else {
                    ok(&format!("emulator: {}", avds.join(", ")));
                }
            } else {
                note("no emulator: `run -d android` needs a connected device");
            }
        }
        Err(error) => gap(
            &format!("{error:#}"),
            "https://developer.android.com/studio",
        ),
    }
    Ok(())
}

fn target(installed: &[String], triple: &str) {
    if installed.iter().any(|t| t == triple) {
        ok(&format!("target {triple}"));
    } else {
        gap(
            &format!("target {triple}"),
            &format!("rustup target add {triple}"),
        );
    }
}

fn env_present(name: &str, effect: &str) {
    if std::env::var_os(name).is_some() {
        ok(&format!("{name} set: {effect}"));
    } else {
        note(&format!("{name} unset: {effect} when set"));
    }
}

fn section(title: &str) {
    println!("\n{title}");
}

fn ok(text: &str) {
    println!("  [ok] {text}");
}

fn gap(text: &str, fix: &str) {
    println!("  [--] {text}\n       fix: {fix}");
}

fn note(text: &str) {
    println!("  [..] {text}");
}

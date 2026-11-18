//! The layering guard: a framework crate does not switch on the target. What differs between a
//! phone, a desktop and a browser is the platform's, which answers at run time; a `cfg` on the
//! target in framework code is a platform check in the wrong layer.
//!
//! The crates this scans are the framework's, every `crates/waft-*` but `waft-embedder-*` and the
//! host, tool and macro crates; test code is read out, since a test may be about one target. The
//! few Flutter-shaped exceptions are named below with why they are not a platform in framework
//! code.

use std::fs;
use std::path::{Path, PathBuf};

/// Crates that are not the framework: the platforms themselves (`waft-embedder-*`), the app entry
/// and tools (`waft`, `waft-cli`, `waft-single-instance`), a test fixture crate, and a proc macro
/// that writes the per-target entry points.
const HOST_AND_TOOL_CRATES: &[&str] = &[
    "waft",
    "waft-cli",
    "waft-single-instance",
    "waft-macros",
    "waft-winui-test-support",
];

/// The lines where a target switch is Flutter's own, not a platform check in the framework: a
/// file and the text its line holds. Each names one switch, so a new one in the same file is
/// still caught.
const FLUTTER_TARGET_CONSTANTS: &[(&str, &str)] = &[
    // `kIsWeb` and `kIsWasm`, the constants Flutter's foundation defines by the compilation
    // target (`bool.fromEnvironment('dart.library.js_util')`).
    (
        "crates/waft-foundation/src/constants.rs",
        "pub const K_IS_WEB: bool = cfg!(",
    ),
    (
        "crates/waft-foundation/src/constants.rs",
        "pub const K_IS_WASM: bool = cfg!(",
    ),
    // Flutter's debug-only assert on the input actions `Platform.isIOS` and `Platform.isAndroid`
    // accept: dart:io's platform, which is the OS the process runs on, not the app's chosen
    // `TargetPlatform` (Flutter's test binding says Android while the process runs on macOS).
    (
        "crates/waft-services/src/text_input.rs",
        "if cfg!(target_os = \"ios\") {",
    ),
    (
        "crates/waft-services/src/text_input.rs",
        "} else if cfg!(target_os = \"android\") {",
    ),
];

#[test]
fn no_framework_crate_switches_on_the_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates folder")
        .parent()
        .expect("the repository root")
        .to_owned();
    let mut hits = Vec::new();
    for crate_dir in crate_dirs(&root.join("crates")) {
        let name = crate_dir
            .file_name()
            .expect("a crate folder's name")
            .to_string_lossy()
            .into_owned();
        if name.starts_with("waft-embedder-") || HOST_AND_TOOL_CRATES.contains(&name.as_str()) {
            continue;
        }
        let src = crate_dir.join("src");
        for file in rust_files(&src) {
            let relative = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .to_string_lossy()
                .into_owned();
            scan(&file, &relative, &mut hits);
        }
    }
    assert!(
        hits.is_empty(),
        "framework crates switch on the target; move each to the platform that answers it:\n{}",
        hits.join("\n")
    );
}

fn crate_dirs(crates: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(crates)
        .expect("the crates folder")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn rust_files(src: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![src.to_owned()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Adds each line of `file` outside a test module that switches on the target and is not one of
/// Flutter's own.
fn scan(file: &Path, relative: &str, hits: &mut Vec<String>) {
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    let mut depth = 0i32;
    // The brace depth at which a `#[cfg(test)] mod` began, and what is left for it to close.
    let mut test_module: Option<i32> = None;
    let mut pending_test = false;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("#[cfg(all(test,") {
            pending_test = true;
        } else if pending_test && !trimmed.starts_with("#[") {
            // The item a `cfg(test)` is on: a module is read out to its end, anything else alone.
            if trimmed.starts_with("mod ") || trimmed.contains(" mod ") {
                test_module = Some(depth);
            }
            pending_test = false;
        }
        let in_test = test_module.is_some() || pending_test;
        if !in_test && has_target_switch(trimmed) && !is_flutter_constant(relative, trimmed) {
            hits.push(format!("{}:{}: {}", file.display(), number + 1, trimmed));
        }
        depth += braces(line);
        if let Some(start) = test_module
            && depth <= start
        {
            test_module = None;
        }
    }
}

fn is_flutter_constant(relative: &str, line: &str) -> bool {
    FLUTTER_TARGET_CONSTANTS
        .iter()
        .any(|&(file, text)| file == relative && line.starts_with(text))
}

fn has_target_switch(line: &str) -> bool {
    let conditional = line.contains("cfg(") || line.contains("cfg!(") || line.contains("cfg_attr(");
    conditional
        && ["target_os", "target_family", "target_arch", "target_vendor"]
            .iter()
            .any(|target| line.contains(target))
}

fn braces(line: &str) -> i32 {
    let code = line.split("//").next().unwrap_or(line);
    code.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

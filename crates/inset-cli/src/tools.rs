//! External tools: locating them, fetching the ones cargo does not ship, running them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// binaryen release used when `wasm-opt` is not on PATH.
const BINARYEN_VERSION: &str = "version_123";

/// Runs a command whose output is for the user; fails on a non-zero exit.
pub fn run(command: &mut Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("could not start {what}"))?;
    if !status.success() {
        bail!("{what} failed ({status})");
    }
    Ok(())
}

/// Runs a command and returns its stdout; stderr goes into the error.
pub fn output(command: &mut Command, what: &str) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("could not start {what}"))?;
    if !output.status.success() {
        bail!(
            "{what} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe_name(name)))
        .find(|candidate| candidate.is_file())
}

pub fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

/// Where downloaded tools live: `$INSET_CACHE_DIR`, else the user's cache directory.
pub fn cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("INSET_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("inset");
    }
    if cfg!(windows)
        && let Some(dir) = std::env::var_os("LOCALAPPDATA")
    {
        return PathBuf::from(dir).join("inset");
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".cache").join("inset")
}

/// Rust targets rustup has installed for the toolchain a project directory selects.
pub fn installed_targets(dir: &Path) -> Result<Vec<String>> {
    if which("rustup").is_none() {
        return Ok(Vec::new());
    }
    let listing = output(
        Command::new("rustup")
            .current_dir(dir)
            .args(["target", "list", "--installed"]),
        "rustup target list",
    )?;
    Ok(listing.lines().map(|line| line.trim().to_owned()).collect())
}

pub fn ensure_rust_target(dir: &Path, triple: &str) -> Result<()> {
    if which("rustup").is_none() {
        return Ok(());
    }
    if !installed_targets(dir)?.iter().any(|t| t == triple) {
        bail!("Rust target {triple} is not installed; run: rustup target add {triple}");
    }
    Ok(())
}

/// The `wasm-bindgen` CLI at exactly the version the app's Cargo.lock resolved,
/// since its output format is only stable within one version.
pub fn wasm_bindgen(version: &str) -> Result<PathBuf> {
    if let Some(on_path) = which("wasm-bindgen")
        && let Ok(reported) = output(Command::new(&on_path).arg("--version"), "wasm-bindgen")
        && reported.trim() == format!("wasm-bindgen {version}")
    {
        return Ok(on_path);
    }
    let dir = cache_dir().join(format!("wasm-bindgen-{version}"));
    let exe = dir.join(exe_name("wasm-bindgen"));
    if exe.is_file() {
        return Ok(exe);
    }
    let triple = wasm_bindgen_triple()?;
    let url = format!(
        "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/{version}/wasm-bindgen-{version}-{triple}.tar.gz"
    );
    eprintln!("downloading wasm-bindgen {version}");
    let extracted = download_tar_gz(&url, &dir).with_context(|| {
        format!(
            "could not fetch wasm-bindgen {version}; install it with: cargo install wasm-bindgen-cli --version {version}"
        )
    })?;
    let found = find_file(&extracted, &exe_name("wasm-bindgen"))
        .context("the wasm-bindgen archive had no wasm-bindgen binary")?;
    fs::rename(&found, &exe)?;
    Ok(exe)
}

fn wasm_bindgen_triple() -> Result<&'static str> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => bail!("no prebuilt wasm-bindgen for {os}/{arch}"),
    })
}

/// `wasm-opt` from PATH, the cache, or a binaryen release download. `None` when
/// nothing works, so a release build proceeds unoptimized with a note.
pub fn wasm_opt() -> Option<PathBuf> {
    if let Some(on_path) = which("wasm-opt") {
        return Some(on_path);
    }
    let dir = cache_dir().join(format!("binaryen-{BINARYEN_VERSION}"));
    let exe = dir.join(exe_name("wasm-opt"));
    if exe.is_file() {
        return Some(exe);
    }
    let arch = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "arm64-macos",
        ("macos", "x86_64") => "x86_64-macos",
        ("linux", "x86_64") => "x86_64-linux",
        ("linux", "aarch64") => "aarch64-linux",
        ("windows", "x86_64") => "x86_64-windows",
        _ => return None,
    };
    let url = format!(
        "https://github.com/WebAssembly/binaryen/releases/download/{BINARYEN_VERSION}/binaryen-{BINARYEN_VERSION}-{arch}.tar.gz"
    );
    eprintln!("downloading binaryen {BINARYEN_VERSION} for wasm-opt");
    let extracted = download_tar_gz(&url, &dir).ok()?;
    let bin_dir = find_file(&extracted, &exe_name("wasm-opt"))?
        .parent()?
        .to_path_buf();
    // binaryen's binaries load lib/ next to bin/, so the layout stays as shipped.
    let layout = bin_dir.parent()?;
    for entry in fs::read_dir(layout).ok()?.flatten() {
        let _ = fs::rename(entry.path(), dir.join(entry.file_name()));
    }
    let exe = dir.join("bin").join(exe_name("wasm-opt"));
    exe.is_file().then_some(exe)
}

/// Downloads and unpacks an archive; returns the directory holding its contents.
fn download_tar_gz(url: &str, dir: &Path) -> Result<PathBuf> {
    let extracted = dir.join("download");
    if extracted.exists() {
        fs::remove_dir_all(&extracted)?;
    }
    fs::create_dir_all(&extracted)?;
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let reader = response.into_body().into_reader();
    tar::Archive::new(flate2::read::GzDecoder::new(reader))
        .unpack(&extracted)
        .with_context(|| format!("unpacking {url}"))?;
    Ok(extracted)
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|f| f == name) {
            return Some(path);
        }
    }
    None
}

/// The `wasm-bindgen` version the workspace's Cargo.lock resolved.
pub fn locked_wasm_bindgen_version(workspace_root: &Path) -> Result<String> {
    let lock = fs::read_to_string(workspace_root.join("Cargo.lock"))
        .context("no Cargo.lock; run `cargo build` once")?;
    locked_version(&lock, "wasm-bindgen")
        .context("wasm-bindgen is not in Cargo.lock; the web host needs it")
}

fn locked_version(lock: &str, package: &str) -> Option<String> {
    let header = format!("name = \"{package}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == header {
            let version = lines.next()?.trim().strip_prefix("version = \"")?;
            return Some(version.trim_end_matches('"').to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_version_reads_the_exact_package() {
        let lock = "[[package]]\nname = \"wasm-bindgen-futures\"\nversion = \"0.4.50\"\n\n[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.128\"\n";
        assert_eq!(
            locked_version(lock, "wasm-bindgen").as_deref(),
            Some("0.2.128")
        );
        assert_eq!(locked_version(lock, "wasm-opt"), None);
    }
}

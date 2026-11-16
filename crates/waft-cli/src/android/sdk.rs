//! Where the Android tools are: the SDK, its newest build tools and platform, the NDK, and the Java
//! that runs `javac`, `d8`, `apksigner` and `keytool`, each found as Android Studio finds it or
//! where the environment names it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::tools;

/// The SDK as this machine has it: the tools an `.apk` is put together with, and where the
/// NDK and Java are when it has them.
pub struct Sdk {
    pub root: PathBuf,
    /// The newest `build-tools/<version>`.
    pub build_tools: PathBuf,
    /// The newest `platforms/android-<level>`, whose `android.jar` names the framework's
    /// resources the manifest refers to.
    pub platform: Platform,
    pub adb: PathBuf,
    pub emulator: Option<PathBuf>,
    pub ndk: Option<PathBuf>,
    pub java: Option<Java>,
}

pub struct Platform {
    pub level: u32,
    pub jar: PathBuf,
}

/// A Java development kit: `javac` for Waft's activity, and the runtime of `d8`, `apksigner`
/// and `keytool`, which are Java programs.
pub struct Java {
    pub home: PathBuf,
}

impl Java {
    pub(super) fn bin(&self, tool: &str) -> Command {
        Command::new(self.home.join("bin").join(tools::exe_name(tool)))
    }
}

/// The SDK, or why there is none.
pub fn sdk() -> Result<Sdk> {
    let root = sdk_root().context(
        "no Android SDK: install one with Android Studio, or set ANDROID_HOME to its directory",
    )?;
    let build_tools = newest_dir(&root.join("build-tools"), version_key).with_context(|| {
        format!(
            "no build-tools under {}; install one with the SDK Manager",
            root.display()
        )
    })?;
    let platform_dir = newest_dir(&root.join("platforms"), |name| {
        name.strip_prefix("android-").and_then(version_key)
    })
    .with_context(|| {
        format!(
            "no platform under {}; install one with the SDK Manager",
            root.display()
        )
    })?;
    let level = platform_dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("android-"))
        .and_then(|version| version.split('.').next())
        .and_then(|major| major.parse().ok())
        .context("platform directory has no API level")?;
    Ok(Sdk {
        adb: root.join("platform-tools").join(tools::exe_name("adb")),
        emulator: Some(root.join("emulator").join(tools::exe_name("emulator")))
            .filter(|p| p.is_file()),
        ndk: ndk_root(&root),
        java: java(),
        build_tools,
        platform: Platform {
            level,
            jar: platform_dir.join("android.jar"),
        },
        root,
    })
}

/// `ANDROID_HOME`, `ANDROID_SDK_ROOT`, or where Android Studio installs the SDK.
pub fn sdk_root() -> Option<PathBuf> {
    for name in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(dir) = std::env::var_os(name).map(PathBuf::from)
            && dir.is_dir()
        {
            return Some(dir);
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    let default = match std::env::consts::OS {
        "macos" => home.join("Library/Android/sdk"),
        "windows" => std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or(home)
            .join("Android/Sdk"),
        _ => home.join("Android/Sdk"),
    };
    default.is_dir().then_some(default)
}

/// `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, or the newest NDK the SDK Manager installed.
fn ndk_root(sdk: &Path) -> Option<PathBuf> {
    for name in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"] {
        if let Some(dir) = std::env::var_os(name).map(PathBuf::from)
            && dir.is_dir()
        {
            return Some(dir);
        }
    }
    newest_dir(&sdk.join("ndk"), version_key)
        .or_else(|| Some(sdk.join("ndk-bundle")).filter(|dir| dir.is_dir()))
}

/// `JAVA_HOME`, the runtime Android Studio bundles, or a `java` on PATH that runs.
pub(super) fn java() -> Option<Java> {
    if let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from)
        && home.is_dir()
    {
        return Some(Java { home });
    }
    let bundled = match std::env::consts::OS {
        "macos" => vec![PathBuf::from(
            "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
        )],
        "windows" => vec![PathBuf::from(
            r"C:\Program Files\Android\Android Studio\jbr",
        )],
        _ => vec![
            PathBuf::from("/opt/android-studio/jbr"),
            PathBuf::from("/usr/local/android-studio/jbr"),
        ],
    };
    if let Some(home) = bundled.into_iter().find(|home| home.join("bin").is_dir()) {
        return Some(Java { home });
    }
    // macOS ships a `java` stub that fails when no runtime is installed; only a `java`
    // that answers counts.
    let on_path = tools::which("java")?;
    let works = Command::new(&on_path)
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success());
    let home = on_path.parent()?.parent()?.to_path_buf();
    works.then_some(Java { home })
}

/// The newest of a directory's children by the version its name carries.
fn newest_dir(parent: &Path, key: impl Fn(&str) -> Option<Vec<u32>>) -> Option<PathBuf> {
    fs::read_dir(parent)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name();
            let version = key(name.to_str()?)?;
            Some((version, entry.path()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, path)| path)
}

/// `36.0.0` → `[36, 0, 0]`; `None` for a name that is not a version.
fn version_key(name: &str) -> Option<Vec<u32>> {
    let parts: Vec<u32> = name
        .split(['.', '-'])
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    (!parts.is_empty()).then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_numerically() {
        assert_eq!(version_key("36.0.0"), Some(vec![36, 0, 0]));
        assert!(version_key("28.2.13676358") > version_key("9.0.0"));
        assert_eq!(version_key("latest"), None);
    }
}

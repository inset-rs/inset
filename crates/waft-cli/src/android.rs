//! Android: the `.apk` Gradle would produce, assembled from a cargo library.
//!
//! No Gradle project and no Java of the app's own. The Java is Waft's five classes (under
//! `android/rs/waft/` in this crate). The activity, `rs.waft.WaftActivity`, is the platform's
//! `NativeActivity`, which loads the app's shared library and calls its `android_main`, with the
//! results of the activities the app starts, the intents that reach it while it runs and the
//! answers to the permissions it asks for handed on to its native code, as the document pickers,
//! being opened and notifications need. The file provider, `rs.waft.WaftFileProvider`, shares a
//! file of the app's with the app that opens it. The notification receiver,
//! `rs.waft.WaftNotificationReceiver`, builds and shows the app's notifications, and shows one
//! when its alarm goes off, whether or not the app runs. The text input, `rs.waft.WaftTextInput`,
//! is the view the activity adds for the system's keyboards to edit text through. The window
//! insets listener, `rs.waft.WaftWindowInsets`, tells the app what the system's bars and the
//! keyboard cover of its window. So the package is that library, the classes compiled to
//! `classes.dex`, a manifest, and icons, put together with the SDK's own tools: `javac` and `d8`
//! compile the classes, `aapt2` links the manifest and resources, `zipalign` aligns, `apksigner`
//! signs with the debug key, and `adb` installs and launches it on an emulator or a device. The
//! library in the package is a stripped copy (`strip_option` says of what); the library as built
//! stays under `target/` to symbolise a stack trace against.
//!
//! Everything but the NDK's linker and Java is found under the SDK. The NDK is where the
//! linker and the C library for the target live; Java compiles the classes and runs `d8`,
//! `apksigner` and `keytool`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::devices::Target;

use crate::cargo::{self, Artifact, Build};
use crate::icons;
use crate::project::{FileAssociation, Profile, Project};
use crate::tools;

/// Android 12, the least an app runs on: Waft's Android host links the surface-control calls
/// that place a composition layer by position, scale and crop (`waft-embedder-android`'s
/// `surface_control.rs`), which are API 31's, so its library neither links for nor loads on an
/// earlier version. It is also every app's default.
const LEAST_MIN_SDK: u32 = 31;
/// The level an app targets unless it names another: the last before the system lays every app
/// out edge to edge. A later one lays a Waft app out the same: it draws its window's whole
/// surface at every level, and the host is told what the bars cover of it as the window's insets
/// (`waft-embedder-android`'s `window_insets.rs`).
const DEFAULT_TARGET_SDK: u32 = 34;
/// Waft's activity: the platform's `NativeActivity`, which loads a native library and calls
/// `android_main`, handing activity results on to it.
const WAFT_ACTIVITY: &str = "rs.waft.WaftActivity";
/// Waft's file provider, which shares a file of the app's with the app that opens it.
const WAFT_FILE_PROVIDER: &str = "rs.waft.WaftFileProvider";
/// Waft's notification receiver, which builds the app's notifications and hears their alarms.
const WAFT_NOTIFICATION_RECEIVER: &str = "rs.waft.WaftNotificationReceiver";
/// What every app may do for its notifications: post them, which Android 13 asks the user for, and
/// show one at its exact time, which Android 14 leaves to the user to grant in the app's settings.
const PERMISSIONS: [&str; 2] = [
    "android.permission.POST_NOTIFICATIONS",
    "android.permission.SCHEDULE_EXACT_ALARM",
];
/// The Java classes every app's package compiles, by their paths under the sources' root.
const JAVA_SOURCES: [(&str, &str); 5] = [
    (
        "rs/waft/WaftActivity.java",
        include_str!("../android/rs/waft/WaftActivity.java"),
    ),
    (
        "rs/waft/WaftFileProvider.java",
        include_str!("../android/rs/waft/WaftFileProvider.java"),
    ),
    (
        "rs/waft/WaftNotificationReceiver.java",
        include_str!("../android/rs/waft/WaftNotificationReceiver.java"),
    ),
    (
        "rs/waft/WaftTextInput.java",
        include_str!("../android/rs/waft/WaftTextInput.java"),
    ),
    (
        "rs/waft/WaftWindowInsets.java",
        include_str!("../android/rs/waft/WaftWindowInsets.java"),
    ),
];
/// The schemes of the links whose viewers the app may ask about (`<queries>`): from Android 11 the
/// system answers an app only about the intents its manifest names. The application's meta-data
/// lists them for the app's native code under [`QUERIED_SCHEMES_KEY`], so it knows which answers
/// are the system's whole answer.
const QUERIED_SCHEMES: [&str; 3] = ["http", "https", "mailto"];
const QUERIED_SCHEMES_KEY: &str = "rs.waft.queried_schemes";
/// The Java release the classes are compiled for, which `d8` takes and lowers to `min_sdk`.
const JAVA_RELEASE: &str = "11";
/// What the app says it is, for a system that asks. An app that says nothing is
/// `CATEGORY_UNDEFINED`, and the makers' own optimisers take an uncategorised app that
/// draws with the GPU for a game they do not know and hold it at 60 Hz whatever the panel
/// can do. Saying anything at all stops that.
const DEFAULT_CATEGORY: &str = "productivity";
/// Every configuration change the activity absorbs rather than being recreated for, which
/// is every one Android names. The activity must survive them all: winit allows one event
/// loop per process, and a recreated activity calls `android_main` a second time in the
/// same process, where a second loop cannot be made.
const CONFIG_CHANGES: &str = "colorMode|density|fontScale|grammaticalGender|keyboard|keyboardHidden|layoutDirection|locale|mcc|mnc|navigation|orientation|screenLayout|screenSize|smallestScreenSize|touchscreen|uiMode";
/// How long an emulator gets to boot.
const BOOT_TIMEOUT: Duration = Duration::from_secs(240);

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
    fn bin(&self, tool: &str) -> Command {
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
fn java() -> Option<Java> {
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

/// The processor a device runs, as Android names it, and the Rust target for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abi {
    Arm64,
    X86_64,
    Arm,
    X86,
}

impl Abi {
    /// `ro.product.cpu.abi` as a device reports it.
    pub fn from_product(abi: &str) -> Option<Abi> {
        Some(match abi.trim() {
            "arm64-v8a" => Abi::Arm64,
            "x86_64" => Abi::X86_64,
            "armeabi-v7a" => Abi::Arm,
            "x86" => Abi::X86,
            _ => return None,
        })
    }

    /// The `lib/<abi>` directory of the package.
    pub fn dir_name(self) -> &'static str {
        match self {
            Abi::Arm64 => "arm64-v8a",
            Abi::X86_64 => "x86_64",
            Abi::Arm => "armeabi-v7a",
            Abi::X86 => "x86",
        }
    }

    pub fn triple(self) -> &'static str {
        match self {
            Abi::Arm64 => "aarch64-linux-android",
            Abi::X86_64 => "x86_64-linux-android",
            Abi::Arm => "armv7-linux-androideabi",
            Abi::X86 => "i686-linux-android",
        }
    }

    /// The NDK's clang for the target, which is named by the target and the API level it
    /// links against; only the 32-bit ARM name differs from the Rust triple.
    fn clang_name(self, min_sdk: u32) -> String {
        let target = match self {
            Abi::Arm => "armv7a-linux-androideabi",
            other => other.triple(),
        };
        format!("{target}{min_sdk}-clang")
    }
}

pub struct Apk {
    pub path: PathBuf,
    /// The package name the system knows the app by.
    pub package: String,
}

/// Builds the library for `abi` and puts the `.apk` together around it.
pub fn build(project: &Project, profile: Profile, abi: Abi) -> Result<Apk> {
    let library = project.library.as_deref().context(
        "the Android activity loads a library: add `crate-type = [\"cdylib\", \"rlib\"]` under [lib] in Cargo.toml",
    )?;
    let sdk = sdk()?;
    let ndk = sdk.ndk.as_deref().context(
        "no Android NDK: install one with Android Studio (SDK Manager > SDK Tools > NDK), or set ANDROID_NDK_HOME",
    )?;
    let java = sdk
        .java
        .as_ref()
        .context("no Java for javac and apksigner: install Android Studio, or set JAVA_HOME")?;
    let min_sdk = min_sdk(project.android.min_sdk)?;
    tools::ensure_rust_target(&project.root, abi.triple())?;
    let shared_library = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: Some(abi.triple()),
        artifact: Artifact::Library(library),
        env: linker_env(ndk, abi, min_sdk)?,
    })?;

    let out = project.out_dir("android", profile);
    let stage = out.join("apk");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    let package = project.android_package();
    let has_icon = write_icons(project, &stage.join("res"))?;
    fs::write(
        stage.join("AndroidManifest.xml"),
        manifest(&Manifest {
            package: &package,
            label: &project.name,
            library,
            version_code: project
                .android
                .version_code
                .unwrap_or_else(|| version_code(&project.version)),
            version_name: &project.version,
            category: project
                .android
                .category
                .as_deref()
                .unwrap_or(DEFAULT_CATEGORY),
            min_sdk,
            target_sdk: project.android.target_sdk.unwrap_or(DEFAULT_TARGET_SDK),
            debuggable: !profile.is_release(),
            has_icon,
            file_associations: &project.file_associations,
            permissions: &project.android.permissions,
        }),
    )?;
    let linked = stage.join("unaligned.apk");
    link(&sdk, &stage, &linked, has_icon)?;
    let classes = compile_classes(&sdk, java, &stage, min_sdk)?;
    add_to_package(&linked, "classes.dex", &classes)?;
    let packaged = strip(
        ndk,
        &shared_library,
        &stage.join("stripped.so"),
        strip_option(profile),
    )?;
    add_to_package(
        &linked,
        &format!("lib/{}/lib{library}.so", abi.dir_name()),
        &packaged,
    )?;
    let aligned = stage.join("aligned.apk");
    tools::run(
        Command::new(sdk.build_tools.join(tools::exe_name("zipalign")))
            .args(["-f", "-P", "16", "4"])
            .arg(&linked)
            .arg(&aligned),
        "zipalign",
    )?;
    let apk = out.join(format!("{}.apk", project.package));
    sign(&sdk, java, &aligned, &apk)?;
    if profile.is_release() {
        eprintln!(
            "warning: signed with the debug key, whose password is public. \
             A store will not take this apk."
        );
    }
    Ok(Apk { path: apk, package })
}

/// The `minSdkVersion` an app is built for: the one it asks for, or Android 12, refusing one below
/// Android 12, which Waft's host does not run on.
fn min_sdk(asked: Option<u32>) -> Result<u32> {
    let min_sdk = asked.unwrap_or(LEAST_MIN_SDK);
    if min_sdk < LEAST_MIN_SDK {
        bail!(
            "min_sdk = {min_sdk} is below {LEAST_MIN_SDK}, Android 12, the least Waft's Android host runs on: \
             it places composition layers with surface-control calls Android 12 brought"
        );
    }
    Ok(min_sdk)
}

/// What `llvm-strip` removes from the library the package carries. The library as built keeps
/// everything under `target/`, so a stack trace from either build can be symbolised against it
/// with `ndk-stack` or `llvm-addr2line`, which is what Gradle does.
///
/// A release package drops the symbols too: they are for reading a crash report, not for the
/// device, and nothing on it loads them. A debug package drops only the debug information, most
/// of a debug library and read by nothing on the device, and keeps the symbol table and the
/// unwind tables, so a panic's backtrace still names its functions.
fn strip_option(profile: Profile) -> &'static str {
    match profile {
        Profile::Release => "--strip-all",
        Profile::Debug => "--strip-debug",
    }
}

/// A copy of `library` at `to`, stripped with `option`, for the package.
fn strip(ndk: &Path, library: &Path, to: &Path, option: &str) -> Result<PathBuf> {
    let strip = toolchain(ndk).join(tools::exe_name("llvm-strip"));
    if !strip.is_file() {
        // Shipping the library as built wastes the download but runs the same; say so and go on.
        eprintln!("note: no llvm-strip in the NDK; the library ships as built");
        return Ok(library.to_path_buf());
    }
    fs::copy(library, to)?;
    tools::run(Command::new(strip).arg(option).arg(to), "llvm-strip")?;
    Ok(to.to_path_buf())
}

/// The NDK's toolchain binaries for this host.
fn toolchain(ndk: &Path) -> PathBuf {
    let host = match std::env::consts::OS {
        "macos" => "darwin-x86_64",
        "windows" => "windows-x86_64",
        _ => "linux-x86_64",
    };
    ndk.join("toolchains/llvm/prebuilt").join(host).join("bin")
}

/// The environment that points cargo, and any C a dependency compiles, at the NDK's
/// toolchain for the target.
fn linker_env(ndk: &Path, abi: Abi, min_sdk: u32) -> Result<Vec<(&'static str, String)>> {
    let bin = toolchain(ndk);
    let clang = bin.join(tools::exe_name(&abi.clang_name(min_sdk)));
    if !clang.is_file() {
        bail!(
            "the NDK at {} has no {} for API {min_sdk}; the NDK's toolchain may be older than the API, or for another host",
            ndk.display(),
            clang.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    let clang = clang.to_string_lossy().into_owned();
    let ar = bin
        .join(tools::exe_name("llvm-ar"))
        .to_string_lossy()
        .into_owned();
    let (linker, cc, cxx, ar_key) = match abi {
        Abi::Arm64 => (
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            "CC_aarch64_linux_android",
            "CXX_aarch64_linux_android",
            "AR_aarch64_linux_android",
        ),
        Abi::X86_64 => (
            "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
            "CC_x86_64_linux_android",
            "CXX_x86_64_linux_android",
            "AR_x86_64_linux_android",
        ),
        Abi::Arm => (
            "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
            "CC_armv7_linux_androideabi",
            "CXX_armv7_linux_androideabi",
            "AR_armv7_linux_androideabi",
        ),
        Abi::X86 => (
            "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
            "CC_i686_linux_android",
            "CXX_i686_linux_android",
            "AR_i686_linux_android",
        ),
    };
    Ok(vec![
        (linker, clang.clone()),
        (cc, clang.clone()),
        (cxx, format!("{clang}++")),
        (ar_key, ar),
    ])
}

struct Manifest<'a> {
    package: &'a str,
    label: &'a str,
    /// The `cdylib`'s name, which the activity loads as `lib<name>.so`.
    library: &'a str,
    version_code: u32,
    version_name: &'a str,
    min_sdk: u32,
    target_sdk: u32,
    debuggable: bool,
    has_icon: bool,
    category: &'a str,
    /// The kinds of file the system offers the app for.
    file_associations: &'a [FileAssociation],
    /// The permissions the app asks for besides [`PERMISSIONS`].
    permissions: &'a [String],
}

/// The manifest around Waft's activity. `configChanges` keeps the activity, and so the
/// app's state, through a rotation or a switch to dark mode, which winit reports as a
/// resize; `adjustResize`, which Flutter's template sets too, has the system leave the window in
/// place under the keyboard, which the host hears of as the window's insets. Waft's file provider is declared at `<package>.waft.files`, not
/// exported, granting each URI it shares; its notification receiver, not exported, hears only the
/// app's own alarms; `<queries>` names viewing the links of [`QUERIED_SCHEMES`]; the app asks for
/// [`PERMISSIONS`] and its own; the activity is offered for the files of the app's associations
/// ([`file_intent_filters`]).
fn manifest(manifest: &Manifest) -> String {
    let icon = if manifest.has_icon {
        r#" android:icon="@mipmap/ic_launcher""#
    } else {
        ""
    };
    let debuggable = if manifest.debuggable {
        r#" android:debuggable="true""#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package}"
    android:versionCode="{version_code}"
    android:versionName="{version_name}">
  <uses-sdk android:minSdkVersion="{min_sdk}" android:targetSdkVersion="{target_sdk}" />
{permissions}  <application
      android:label="{label}"{icon}{debuggable}
      android:appCategory="{category}"
      android:hasCode="true"
      android:extractNativeLibs="false"
      android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
    <activity
        android:name="{activity}"
        android:exported="true"
        android:launchMode="singleTask"
        android:configChanges="{config_changes}"
        android:windowSoftInputMode="adjustResize">
      <meta-data android:name="android.app.lib_name" android:value="{library}" />
      <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
      </intent-filter>
{file_intent_filters}    </activity>
    <provider
        android:name="{provider}"
        android:authorities="{package}.waft.files"
        android:exported="false"
        android:grantUriPermissions="true" />
    <receiver android:name="{receiver}" android:exported="false" />
    <meta-data android:name="{queried_schemes_key}" android:value="{queried_schemes}" />
  </application>
  <queries>
{queries}  </queries>
</manifest>
"#,
        config_changes = CONFIG_CHANGES,
        package = manifest.package,
        version_code = manifest.version_code,
        version_name = xml_escape(manifest.version_name),
        min_sdk = manifest.min_sdk,
        target_sdk = manifest.target_sdk,
        label = xml_escape(manifest.label),
        category = manifest.category,
        activity = WAFT_ACTIVITY,
        library = manifest.library,
        provider = WAFT_FILE_PROVIDER,
        receiver = WAFT_NOTIFICATION_RECEIVER,
        permissions = uses_permissions(manifest.permissions),
        queried_schemes_key = QUERIED_SCHEMES_KEY,
        queried_schemes = QUERIED_SCHEMES.join(","),
        queries = queried_intents(),
        file_intent_filters = file_intent_filters(manifest.file_associations),
    )
}

/// The `<intent-filter>`s by which the system offers the activity to view a file of one of
/// `associations`, as a `content:` or a `file:` URI: a type's MIME type, where it has one; its
/// extensions at the end of the URI's path, of any type; and, for a type the app defines, any file
/// of `application/octet-stream`, once for all of them.
///
/// Android has no way for an app to teach the system a new extension, so a file of a type the app
/// defines reaches it as `application/octet-stream`, and the Files app's Downloads view may name
/// a download by a number, with no extension to match: only the untyped filter offers the app for
/// it, which offers the app for every untyped file too.
fn file_intent_filters(associations: &[FileAssociation]) -> String {
    let mut filters = String::new();
    for association in associations {
        if let Some(mime_type) = &association.mime_type {
            filters += &view_filter(&format!(
                r#"<data android:mimeType="{}" />"#,
                xml_escape(mime_type)
            ));
        }
        filters += &view_filter(&extension_data(&association.extensions));
    }
    if associations.iter().any(|association| association.owner) {
        filters += &view_filter(r#"<data android:mimeType="application/octet-stream" />"#);
    }
    filters
}

/// The data of a filter that matches a path ending in one of `extensions`, of any type: a path is
/// matched only where the filter names a host.
fn extension_data(extensions: &[String]) -> String {
    let suffixes = extensions.iter().map(|extension| {
        format!(
            r#"<data android:pathSuffix=".{}" />"#,
            xml_escape(extension)
        )
    });
    [
        r#"<data android:host="*" />"#.to_owned(),
        r#"<data android:mimeType="*/*" />"#.to_owned(),
    ]
    .into_iter()
    .chain(suffixes)
    .collect::<Vec<_>>()
    .join("\n        ")
}

/// An `<intent-filter>` for viewing a `content:` or a `file:` URI that `data` matches further.
fn view_filter(data: &str) -> String {
    format!(
        r#"      <intent-filter>
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <data android:scheme="content" />
        <data android:scheme="file" />
        {data}
      </intent-filter>
"#
    )
}

/// One `<uses-permission>` for each of [`PERMISSIONS`], then for each of `the_apps` not among
/// them, once each.
fn uses_permissions(the_apps: &[String]) -> String {
    let mut named: Vec<&str> = PERMISSIONS.to_vec();
    for permission in the_apps {
        if !named.contains(&permission.as_str()) {
            named.push(permission);
        }
    }
    named
        .into_iter()
        .map(|permission| {
            format!(
                "  <uses-permission android:name=\"{}\" />\n",
                xml_escape(permission)
            )
        })
        .collect()
}

/// One `<intent>` for viewing a link of each of [`QUERIED_SCHEMES`].
fn queried_intents() -> String {
    QUERIED_SCHEMES
        .iter()
        .map(|scheme| {
            format!(
                r#"    <intent>
      <action android:name="android.intent.action.VIEW" />
      <data android:scheme="{scheme}" />
    </intent>
"#
            )
        })
        .collect()
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A `versionCode` that orders like the package version: `1.2.3` → 10203, with the minor
/// and patch numbers each given two digits.
fn version_code(version: &str) -> u32 {
    let mut parts = version
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0).min(99);
    let patch = parts.next().unwrap_or(0).min(99);
    major
        .saturating_mul(10_000)
        .saturating_add(minor.saturating_mul(100))
        .saturating_add(patch)
        .max(1)
}

/// The launcher icon at each density, from the app's one PNG. Answers whether there is one.
fn write_icons(project: &Project, res: &Path) -> Result<bool> {
    let Some(icon) = &project.icon else {
        return Ok(false);
    };
    let source = icons::load(icon)?;
    for (density, size) in [
        ("mdpi", 48),
        ("hdpi", 72),
        ("xhdpi", 96),
        ("xxhdpi", 144),
        ("xxxhdpi", 192),
    ] {
        let dir = res.join(format!("mipmap-{density}"));
        fs::create_dir_all(&dir)?;
        icons::write_resized(&source, size, &dir.join("ic_launcher.png"))?;
    }
    Ok(true)
}

/// `aapt2`: the resources compiled, then linked with the manifest into an unsigned package.
fn link(sdk: &Sdk, stage: &Path, out: &Path, has_resources: bool) -> Result<()> {
    let aapt2 = sdk.build_tools.join(tools::exe_name("aapt2"));
    let compiled = stage.join("res.zip");
    if has_resources {
        tools::run(
            Command::new(&aapt2)
                .args(["compile", "--dir"])
                .arg(stage.join("res"))
                .arg("-o")
                .arg(&compiled),
            "aapt2 compile",
        )?;
    }
    let mut command = Command::new(&aapt2);
    command
        .arg("link")
        .arg("-o")
        .arg(out)
        .arg("--manifest")
        .arg(stage.join("AndroidManifest.xml"))
        .arg("-I")
        .arg(&sdk.platform.jar);
    if has_resources {
        command.arg(&compiled);
    }
    tools::run(&mut command, "aapt2 link")
}

/// `javac` and `d8`: Waft's classes, compiled against the platform's `android.jar` into the
/// `classes.dex` the package carries.
fn compile_classes(sdk: &Sdk, java: &Java, stage: &Path, min_sdk: u32) -> Result<PathBuf> {
    let sources = stage.join("java");
    fs::create_dir_all(sources.join("rs/waft"))?;
    let mut source_paths = Vec::new();
    for (path, source) in JAVA_SOURCES {
        let written = sources.join(path);
        fs::write(&written, source)?;
        source_paths.push(written);
    }
    let classes = stage.join("classes");
    tools::run(
        java.bin("javac")
            .args(["--release", JAVA_RELEASE, "-Xlint:-options", "-classpath"])
            .arg(&sdk.platform.jar)
            .arg("-d")
            .arg(&classes)
            .args(&source_paths),
        "javac",
    )?;
    let class_files = class_files(&classes.join("rs/waft"))?;
    let dex = stage.join("dex");
    fs::create_dir_all(&dex)?;
    tools::run(
        java.bin("java")
            .arg("-cp")
            .arg(sdk.build_tools.join("lib/d8.jar"))
            .args(["com.android.tools.r8.D8", "--release", "--min-api"])
            .arg(min_sdk.to_string())
            .arg("--lib")
            .arg(&sdk.platform.jar)
            .arg("--output")
            .arg(&dex)
            .args(class_files),
        "d8",
    )?;
    Ok(dex.join("classes.dex"))
}

/// The classes `javac` wrote into `dir`, a nested class's file beside its outer class's.
fn class_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "class")
        {
            files.push(path);
        }
    }
    Ok(files)
}

/// Adds `file` to the package as `entry`, stored rather than deflated: the system loads a
/// shared library from the package in place, page-aligned by `zipalign`, and maps the dex.
fn add_to_package(apk: &Path, entry: &str, file: &Path) -> Result<()> {
    let package = fs::OpenOptions::new().read(true).write(true).open(apk)?;
    let mut writer = zip::ZipWriter::new_append(package).context("reading the linked package")?;
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file(entry, options)?;
    writer.write_all(&fs::read(file)?)?;
    writer.finish()?;
    Ok(())
}

/// `apksigner` with the debug key, the one Android Studio and Gradle sign debug builds
/// with, made here when the machine has none yet.
fn sign(sdk: &Sdk, java: &Java, aligned: &Path, out: &Path) -> Result<()> {
    let keystore = debug_keystore(java)?;
    tools::run(
        java.bin("java")
            .arg("-jar")
            .arg(sdk.build_tools.join("lib/apksigner.jar"))
            .args([
                "sign",
                "--ks-pass",
                "pass:android",
                "--ks-key-alias",
                "androiddebugkey",
                "--key-pass",
                "pass:android",
                "--ks",
            ])
            .arg(&keystore)
            .arg("--out")
            .arg(out)
            .arg(aligned),
        "apksigner",
    )
}

/// `~/.android/debug.keystore`, as Android Studio writes it.
fn debug_keystore(java: &Java) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let dir = home.join(".android");
    let keystore = dir.join("debug.keystore");
    if keystore.is_file() {
        return Ok(keystore);
    }
    fs::create_dir_all(&dir)?;
    println!("creating the debug keystore at {}", keystore.display());
    tools::run(
        java.bin("keytool")
            .args(["-genkeypair", "-keystore"])
            .arg(&keystore)
            .args([
                "-storepass",
                "android",
                "-alias",
                "androiddebugkey",
                "-keypass",
                "android",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
            ]),
        "keytool",
    )?;
    Ok(keystore)
}

/// A device `adb` can reach, running or booting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub model: String,
    /// `device`, `offline`, `unauthorized`, as `adb` reports it.
    pub state: String,
    /// The emulator's AVD, for a device that is one.
    pub avd: Option<String>,
}

/// The emulators this machine has: AVD names.
pub fn emulators() -> Result<Vec<String>> {
    let Ok(sdk) = sdk() else {
        return Ok(Vec::new());
    };
    let Some(emulator) = sdk.emulator else {
        return Ok(Vec::new());
    };
    let listing = tools::output(
        Command::new(emulator).arg("-list-avds"),
        "emulator -list-avds",
    )?;
    Ok(listing
        .lines()
        .map(str::trim)
        // The emulator prints its own notices among the names.
        .filter(|line| !line.is_empty() && !line.starts_with("INFO") && !line.contains(' '))
        .map(str::to_owned)
        .collect())
}

/// The devices `adb` sees, emulators named by their AVD.
pub fn devices() -> Result<Vec<Device>> {
    let Ok(sdk) = sdk() else {
        return Ok(Vec::new());
    };
    if !sdk.adb.is_file() {
        return Ok(Vec::new());
    }
    let listing = tools::output(
        Command::new(&sdk.adb).args(["devices", "-l"]),
        "adb devices",
    )?;
    let mut devices = parse_devices(&listing);
    for device in &mut devices {
        if device.serial.starts_with("emulator-") {
            device.avd = avd_name(&sdk.adb, &device.serial);
        }
    }
    Ok(devices)
}

/// `emulator-5554   device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 …`.
fn parse_devices(listing: &str) -> Vec<Device> {
    listing
        .lines()
        .skip_while(|line| !line.starts_with("List of devices"))
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?;
            let state = fields.next()?;
            let model = fields
                .find_map(|field| field.strip_prefix("model:"))
                .unwrap_or(serial)
                .to_owned();
            Some(Device {
                serial: serial.to_owned(),
                model,
                state: state.to_owned(),
                avd: None,
            })
        })
        .collect()
}

/// The AVD an emulator runs, from its console: the name, then `OK`.
fn avd_name(adb: &Path, serial: &str) -> Option<String> {
    let answer = tools::output(
        Command::new(adb).args(["-s", serial, "emu", "avd", "name"]),
        "adb emu avd name",
    )
    .ok()?;
    answer
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "OK")
        .map(str::to_owned)
}

/// Starts the emulator for `avd` and waits for it to boot; answers the device it is.
pub fn boot_emulator(avd: &str) -> Result<Device> {
    let sdk = sdk()?;
    let emulator = sdk
        .emulator
        .as_ref()
        .context("no emulator under the SDK; install it with the SDK Manager")?;
    println!("booting {avd}");
    Command::new(emulator)
        .args(["-avd", avd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("could not start the emulator")?;
    let started = Instant::now();
    while started.elapsed() < BOOT_TIMEOUT {
        std::thread::sleep(Duration::from_secs(2));
        let Some(device) = devices()?
            .into_iter()
            .find(|device| device.avd.as_deref() == Some(avd) && device.state == "device")
        else {
            continue;
        };
        if booted(&sdk.adb, &device.serial) {
            return Ok(device);
        }
    }
    bail!(
        "{avd} did not boot within {} seconds",
        BOOT_TIMEOUT.as_secs()
    )
}

fn booted(adb: &Path, serial: &str) -> bool {
    tools::output(
        Command::new(adb).args(["-s", serial, "shell", "getprop", "sys.boot_completed"]),
        "adb getprop",
    )
    .is_ok_and(|answer| answer.trim() == "1")
}

/// The processor a device runs, for the library to build.
pub fn device_abi(serial: &str) -> Result<Abi> {
    let sdk = sdk()?;
    let product = tools::output(
        Command::new(&sdk.adb).args(["-s", serial, "shell", "getprop", "ro.product.cpu.abi"]),
        "adb getprop",
    )?;
    Abi::from_product(&product).with_context(|| format!("unknown device ABI `{}`", product.trim()))
}

/// Installs and launches the package, then follows its log until interrupted.
pub fn run(apk: &Apk, device: &Device) -> Result<()> {
    let sdk = sdk()?;
    let adb = |args: &[&str]| {
        let mut command = Command::new(&sdk.adb);
        command.args(["-s", &device.serial]).args(args);
        command
    };
    // Stopped before it is replaced: installing over a running app has the system relaunch
    // the old activity, and the launch below then makes a second one in that same process,
    // which is one more than winit's single event loop can serve.
    let _ = adb(&["shell", "am", "force-stop", &apk.package]).output();
    tools::run(adb(&["install", "-r", "-t"]).arg(&apk.path), "adb install")?;
    println!("launching {} on {}", apk.package, device.model);
    let component = format!("{}/{WAFT_ACTIVITY}", apk.package);
    tools::run(
        &mut adb(&["shell", "am", "start", "-W", "-n", &component]),
        "adb shell am start",
    )?;
    let Some(pid) = process_id(&sdk.adb, &device.serial, &apk.package) else {
        bail!("{} exited before its log could be followed", apk.package);
    };
    println!("press ctrl-c to stop");
    tools::run(
        &mut adb(&["logcat", "-v", "time", "--pid", &pid]),
        "adb logcat",
    )
}

/// The app's process id once it is up; `None` when it never appears.
fn process_id(adb: &Path, serial: &str, package: &str) -> Option<String> {
    for _ in 0..20 {
        if let Ok(pid) = tools::output(
            Command::new(adb).args(["-s", serial, "shell", "pidof", package]),
            "adb pidof",
        ) && !pid.trim().is_empty()
        {
            return Some(pid.trim().to_owned());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
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

    #[test]
    fn an_app_runs_on_android_12_or_later() {
        assert_eq!(min_sdk(None).unwrap(), 31);
        assert_eq!(min_sdk(Some(33)).unwrap(), 33);
        assert!(
            min_sdk(Some(26)).is_err(),
            "the host's library does not load before API 31"
        );
    }

    #[test]
    fn only_a_release_package_drops_the_symbols() {
        assert_eq!(strip_option(Profile::Release), "--strip-all");
        assert_eq!(
            strip_option(Profile::Debug),
            "--strip-debug",
            "a debug backtrace names its functions from the symbol table"
        );
    }

    #[test]
    fn version_code_orders_like_the_package_version() {
        assert_eq!(version_code("0.1.0"), 100);
        assert_eq!(version_code("1.2.3"), 10203);
        assert!(version_code("1.10.0") > version_code("1.9.9"));
        assert_eq!(version_code("bogus"), 1, "never zero");
    }

    #[test]
    fn manifest_names_waft_activity_and_its_library() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My <App>",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: true,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
        });
        assert!(manifest.contains(r#"package="com.example.my_app""#));
        assert!(manifest.contains(r#"android:name="rs.waft.WaftActivity""#));
        assert!(manifest.contains(r#"android:hasCode="true""#));
        assert!(manifest.contains(r#"android:value="my_app""#));
        assert!(manifest.contains(r#"android:label="My &lt;App&gt;""#));
        assert!(manifest.contains(r#"android:debuggable="true""#));
        assert!(!manifest.contains("ic_launcher"));
        assert!(manifest.contains(r#"android:targetSdkVersion="34""#));
        assert!(
            manifest.contains(r#"android:appCategory="productivity""#),
            "an uncategorised app is taken for a game and held at 60 Hz"
        );
        for change in [
            "orientation",
            "screenSize",
            "uiMode",
            "fontScale",
            "density",
        ] {
            assert!(
                manifest.contains(change),
                "the activity must absorb {change} rather than be recreated for it"
            );
        }
    }

    #[test]
    fn manifest_declares_the_file_provider_and_the_queried_links() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
        });
        assert!(manifest.contains(r#"android:name="rs.waft.WaftFileProvider""#));
        assert!(manifest.contains(r#"android:authorities="com.example.my_app.waft.files""#));
        assert!(
            manifest.contains(r#"android:exported="false""#),
            "the provider shares only the URIs it grants"
        );
        assert!(manifest.contains(r#"android:grantUriPermissions="true""#));
        assert!(manifest.contains(r#"<data android:scheme="mailto" />"#));
        assert!(manifest.contains(
            r#"<meta-data android:name="rs.waft.queried_schemes" android:value="http,https,mailto" />"#
        ));
    }

    #[test]
    fn an_apps_own_permissions_follow_wafts_once_each() {
        let own = [
            "android.permission.INTERNET".to_owned(),
            "android.permission.POST_NOTIFICATIONS".to_owned(),
            "android.permission.INTERNET".to_owned(),
        ];
        let asked = uses_permissions(&own);
        assert_eq!(
            asked.matches("android.permission.INTERNET").count(),
            1,
            "{asked}"
        );
        assert_eq!(
            asked
                .matches("android.permission.POST_NOTIFICATIONS")
                .count(),
            1,
            "{asked}"
        );
        assert!(
            asked.find("POST_NOTIFICATIONS") < asked.find("INTERNET"),
            "Waft's own come first: {asked}"
        );
    }

    #[test]
    fn manifest_declares_the_notification_receiver_and_its_permissions() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
        });
        assert!(manifest.contains(
            r#"<receiver android:name="rs.waft.WaftNotificationReceiver" android:exported="false" />"#
        ));
        assert!(manifest.contains(
            r#"<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />"#
        ));
        assert!(manifest.contains(
            r#"<uses-permission android:name="android.permission.SCHEDULE_EXACT_ALARM" />"#
        ));
        assert!(
            manifest.find("<uses-permission") < manifest.find("<application"),
            "permissions belong to the manifest, before the application"
        );
    }

    fn association(extensions: &[&str], mime_type: Option<&str>, owner: bool) -> FileAssociation {
        FileAssociation {
            extensions: extensions
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
            name: "A document".into(),
            owner,
            role: Default::default(),
            identifier: None,
            mime_type: mime_type.map(str::to_owned),
        }
    }

    #[test]
    fn a_typed_association_is_viewed_by_its_mime_type_and_its_extensions() {
        let filters = file_intent_filters(&[association(
            &["md", "markdown"],
            Some("text/markdown"),
            false,
        )]);
        assert_eq!(filters.matches("<intent-filter>").count(), 2);
        assert!(filters.contains(r#"<action android:name="android.intent.action.VIEW" />"#));
        assert!(filters.contains(r#"<category android:name="android.intent.category.DEFAULT" />"#));
        assert!(filters.contains(r#"<data android:scheme="content" />"#));
        assert!(filters.contains(r#"<data android:scheme="file" />"#));
        assert!(filters.contains(r#"<data android:mimeType="text/markdown" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".md" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".markdown" />"#));
        assert!(
            filters.contains(r#"<data android:host="*" />"#),
            "a path is matched only where the filter names a host"
        );
        assert!(
            !filters.contains("application/octet-stream"),
            "a type the system knows needs no untyped filter"
        );
    }

    #[test]
    fn an_owned_type_is_also_offered_for_untyped_files_once() {
        let filters = file_intent_filters(&[
            association(&["wapk"], None, true),
            association(&["wapp"], None, true),
        ]);
        assert!(filters.contains(r#"<data android:pathSuffix=".wapk" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".wapp" />"#));
        assert_eq!(
            filters
                .matches(r#"<data android:mimeType="application/octet-stream" />"#)
                .count(),
            1
        );
    }

    #[test]
    fn the_activity_carries_the_file_filters() {
        let associations = [association(&["wapk"], None, true)];
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &associations,
            permissions: &[],
        });
        let activity =
            &manifest[manifest.find("<activity").unwrap()..manifest.find("</activity>").unwrap()];
        assert!(activity.contains(r#"<data android:pathSuffix=".wapk" />"#));
        assert!(activity.contains("application/octet-stream"));
    }

    #[test]
    fn abis_map_to_rust_targets_and_ndk_clangs() {
        assert_eq!(Abi::from_product("arm64-v8a\n"), Some(Abi::Arm64));
        assert_eq!(Abi::Arm64.triple(), "aarch64-linux-android");
        assert_eq!(Abi::Arm64.clang_name(24), "aarch64-linux-android24-clang");
        assert_eq!(Abi::Arm.clang_name(24), "armv7a-linux-androideabi24-clang");
        assert_eq!(Abi::from_product("mips"), None);
    }

    #[test]
    fn adb_listing_yields_serial_state_and_model() {
        let listing = "List of devices attached\nemulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 device:emu64a transport_id:1\nR5CX1234ABC            unauthorized transport_id:2\n\n";
        let devices = parse_devices(listing);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].model, "sdk_gphone64_arm64");
        assert_eq!(devices[0].state, "device");
        assert_eq!(devices[1].state, "unauthorized");
        assert_eq!(
            devices[1].model, "R5CX1234ABC",
            "no model: the serial stands in"
        );
    }
}

/// Everything Android that `run -d` can name: the emulators the SDK knows, running or
/// not, and the devices `adb` reaches that are not one of them.
pub fn listed_devices() -> Result<Vec<crate::devices::Device>> {
    let attached = devices()?;
    let mut listed: Vec<crate::devices::Device> = emulators()?
        .into_iter()
        .map(|avd| {
            let running = attached
                .iter()
                .any(|device| device.avd.as_deref() == Some(&avd));
            crate::devices::Device {
                name: avd.replace('_', " "),
                id: avd,
                platform: "android emulator",
                state: if running {
                    "running".into()
                } else {
                    String::new()
                },
            }
        })
        .collect();
    listed.extend(
        attached
            .into_iter()
            .filter(|device| device.avd.is_none())
            .map(|device| crate::devices::Device {
                id: device.serial,
                name: device.model,
                platform: "android device",
                state: device.state,
            }),
    );
    Ok(listed)
}

/// `android` is a device `adb` reaches, else the first emulator to boot; a name is an
/// emulator's AVD, running or not, or a device's serial.
pub fn target_for(wanted: &str) -> Result<Option<Target>> {
    let running = devices()?;
    let ready = |device: &Device| device.state == "device";
    if wanted == "android" {
        if let Some(device) = running.iter().find(|device| ready(device)) {
            return Ok(Some(Target::AndroidDevice(device.clone())));
        }
        if let Some(avd) = emulators()?.into_iter().next() {
            return Ok(Some(Target::AndroidEmulator(avd)));
        }
        // An attached device that cannot be used is the likeliest thing the person meant.
        if let Some(device) = running.first() {
            bail!("{}", not_ready(device));
        }
        return Ok(None);
    }
    if let Some(device) = running
        .iter()
        .find(|d| d.serial.eq_ignore_ascii_case(wanted) || d.avd.as_deref() == Some(wanted))
    {
        // Named rather than picked, so say why it cannot be used instead of building first
        // and failing at the install.
        if !ready(device) {
            bail!("{}", not_ready(device));
        }
        return Ok(Some(Target::AndroidDevice(device.clone())));
    }
    Ok(emulators()?
        .into_iter()
        .find(|avd| avd.eq_ignore_ascii_case(wanted))
        .map(Target::AndroidEmulator))
}

/// Why a device `adb` lists cannot be built for, and what to do about it.
fn not_ready(device: &Device) -> String {
    let what = match device.state.as_str() {
        "unauthorized" => {
            "has not allowed USB debugging from this computer yet: unlock it and accept the prompt"
        }
        "offline" => "is offline: unplug it and plug it in again, or turn USB debugging off and on",
        other => return format!("{} is {other}", device.serial),
    };
    format!("{} {what}", device.model)
}

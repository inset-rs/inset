//! Android: the `.apk` Gradle would produce, assembled from a cargo library.
//!
//! No Gradle project and no Java of the app's own. The Java is Waft's eight classes (under
//! `android/rs/waft/` in this crate). The activity, `rs.waft.WaftActivity`, is the platform's
//! `NativeActivity`, which loads the app's shared library and calls into Waft's host in it, with the
//! results of the activities the app starts, the intents that reach it while it runs and the
//! answers to the permissions it asks for handed on to its native code, as the document pickers,
//! being opened and notifications need. The document activity, `rs.waft.WaftDocumentActivity`, is
//! Waft's activity in a task of its own, a card in Recents, for each view the app asks for, and
//! `rs.waft.WaftTask` describes a task's card and brings a task forward. The file provider, `rs.waft.WaftFileProvider`, shares a
//! file of the app's with the app that opens it. The notification receiver,
//! `rs.waft.WaftNotificationReceiver`, builds and shows the app's notifications, and shows one
//! when its alarm goes off, whether or not the app runs. The text input, `rs.waft.WaftTextInput`,
//! is the view the activity adds for the system's keyboards to edit text through. The window
//! insets listener, `rs.waft.WaftWindowInsets`, tells the app what the system's bars and the
//! keyboard cover of its window. The shortcuts, `rs.waft.WaftShortcuts`, make and read the app's
//! shortcuts on the home screen and its icon's menu. So the package is that library, the classes compiled to
//! `classes.dex`, a manifest, and icons, put together with the SDK's own tools: `javac` and `d8`
//! compile the classes, `aapt2` links the manifest and resources (`packaging.rs`, `manifest.rs`,
//! `launcher_icons.rs`), `zipalign` aligns, `apksigner` signs it (`signing.rs`), and `adb` installs
//! and launches it on an emulator or a device (`devices.rs`). The
//! library in the package is a stripped copy (`strip_option` says of what); the library as built
//! stays under `target/` to symbolise a stack trace against.
//!
//! The Android libraries the app depends on (`library.rs`) are merged in as Gradle merges them:
//! their classes compiled into the dex with Waft's, their manifests into Waft's
//! (`manifest_merger.rs`), their resources linked with the app's, beneath them, and an R class
//! generated for each, their native libraries for the package's processor
//! (`native_libraries.rs`) and their JARs' Java resources (`java_resources.rs`) packaged.
//!
//! Everything but the NDK's linker and Java is found under the SDK (`sdk.rs`). The NDK is where the
//! linker and the C library for the target live; Java compiles the classes and runs `d8`,
//! `apksigner` and `keytool`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

mod abi;
mod devices;
mod java_resources;
mod launcher_icons;
mod library;
mod manifest;
mod manifest_merger;
mod native_libraries;
mod packaging;
mod sdk;
mod signing;
mod xml;

pub use devices::{Device, default_target, emulators, ensure_ready, listed_devices};
pub use sdk::sdk;

use library::AndroidLibrary;
use manifest_merger::LibraryManifest;
use signing::Signing;

use crate::cargo::{self, Artifact, Build};
use crate::project::{Profile, Project};
use crate::tools;
use abi::{Abi, linker_env, toolchain};
use devices::{boot_emulator, device_abi, run};
use launcher_icons::write_icons;
use manifest::{Manifest, manifest, merge_manifests, version_code};
use packaging::{add_to_package, compile_classes, link};

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
/// What the app says it is, for a system that asks. An app that says nothing is
/// `CATEGORY_UNDEFINED`, and the makers' own optimisers take an uncategorised app that
/// draws with the GPU for a game they do not know and hold it at 60 Hz whatever the panel
/// can do. Saying anything at all stops that.
const DEFAULT_CATEGORY: &str = "productivity";
struct Apk {
    pub path: PathBuf,
    /// The package name the system knows the app by.
    pub package: String,
}

/// `build apk`: the `.apk` for 64-bit ARM devices and emulators.
pub fn build_apk(project: &Project, profile: Profile) -> Result<PathBuf> {
    Ok(build(project, profile, Abi::Arm64)?.path)
}

/// `run` on the emulator of `avd`, booted first.
pub fn run_on_emulator(project: &Project, profile: Profile, avd: &str) -> Result<()> {
    let device = boot_emulator(avd)?;
    run_on_device(project, profile, &device)
}

/// `run` on `device`: the library for its own processor, packaged, installed and launched.
pub fn run_on_device(project: &Project, profile: Profile, device: &Device) -> Result<()> {
    let abi = device_abi(&device.serial)?;
    let apk = build(project, profile, abi)?;
    run(&apk, device)
}

/// Builds the library for `abi` and puts the `.apk` together around it.
fn build(project: &Project, profile: Profile, abi: Abi) -> Result<Apk> {
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
    let out = project.out_dir("android", profile);
    let stage = out.join("apk");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    // Unpacked before the app is built, so a library that cannot be merged is refused at once.
    let libraries = library::unpack_all(
        &project.root,
        &project.android.dependencies,
        &stage.join("libraries"),
        sdk.platform.level,
    )?;
    let library_file = format!("lib{library}.so");
    let native_libraries = native_libraries::for_abi(&libraries, abi.dir_name(), &library_file)?;
    let java_resources = java_resources::gather(&libraries, &stage.join("java-resources"))?;
    tools::ensure_rust_target(&project.root, abi.triple())?;
    let shared_library = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: Some(abi.triple()),
        artifact: Artifact::Library(library),
        env: linker_env(ndk, abi, min_sdk)?,
        link_inputs: Vec::new(),
    })?;

    let package = project.android_package();
    let has_icon = write_icons(project, &stage.join("res"))?;
    let wafts_manifest = manifest(&Manifest {
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
        devices: &project.usage_descriptions,
    });
    fs::write(
        stage.join("AndroidManifest.xml"),
        merge_manifests(&wafts_manifest, &libraries)?,
    )?;
    let linked = stage.join("unaligned.apk");
    link(&sdk, &stage, &linked, has_icon, &libraries)?;
    let packaged = strip(
        ndk,
        &shared_library,
        &stage.join("stripped.so"),
        strip_option(profile),
    )?;
    let mut entries = compile_classes(&sdk, java, &stage, min_sdk, &libraries)?;
    entries.push((format!("lib/{}/{library_file}", abi.dir_name()), packaged));
    entries.extend(native_libraries);
    entries.extend(
        java_resources
            .into_iter()
            .map(|resource| (resource.path, resource.file)),
    );
    add_to_package(&linked, &entries)?;
    let aligned = stage.join("aligned.apk");
    tools::run(
        Command::new(sdk.build_tools.join(tools::exe_name("zipalign")))
            .args(["-f", "-P", "16", "4"])
            .arg(&linked)
            .arg(&aligned),
        "zipalign",
    )?;
    let apk = out.join(format!("{}.apk", project.package));
    Signing::for_build(project, profile).sign(&sdk, java, &aligned, &apk)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

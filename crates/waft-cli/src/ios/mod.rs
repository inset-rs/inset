//! iOS: the `.app` folder Xcode would produce, assembled from a cargo binary.
//!
//! No Xcode project. The binary, an `Info.plist`, icons, and resources go into a
//! folder; `codesign` signs it (ad hoc for the simulator, a development identity
//! and profile for a device); `simctl` or `devicectl` installs and launches it.

mod devices;
mod profiles;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use plist::{Dictionary, Value};

pub use devices::{Simulator, connected_iphones, listed_devices, pick_simulator, simulators};

use crate::apple::{self, SigningIdentity, SigningSettings};
use crate::cargo::{self, Artifact, Build};
use crate::icons;
use crate::project::{Profile, Project};
use crate::tools;

const DEFAULT_MINIMUM_VERSION: &str = "15.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Simulator,
    Device,
}

impl Destination {
    fn triple(self) -> &'static str {
        match self {
            Destination::Simulator if cfg!(target_arch = "aarch64") => "aarch64-apple-ios-sim",
            Destination::Simulator => "x86_64-apple-ios",
            Destination::Device => "aarch64-apple-ios",
        }
    }

    fn dir_name(self) -> &'static str {
        match self {
            Destination::Simulator => "ios-simulator",
            Destination::Device => "ios",
        }
    }

    fn platform_name(self) -> &'static str {
        match self {
            Destination::Simulator => "iphonesimulator",
            Destination::Device => "iphoneos",
        }
    }

    fn supported_platform(self) -> &'static str {
        match self {
            Destination::Simulator => "iPhoneSimulator",
            Destination::Device => "iPhoneOS",
        }
    }
}

pub struct App {
    pub path: PathBuf,
    pub bundle_id: String,
}

pub fn build(project: &Project, profile: Profile, destination: Destination) -> Result<App> {
    if !cfg!(target_os = "macos") {
        bail!("iOS builds need macOS with the Xcode command line tools");
    }
    let minimum_version = project
        .ios
        .minimum_version
        .clone()
        .unwrap_or_else(|| DEFAULT_MINIMUM_VERSION.to_owned());
    tools::ensure_rust_target(&project.root, destination.triple())?;
    let executable = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: Some(destination.triple()),
        artifact: Artifact::Executable(&project.bin),
        env: vec![("IPHONEOS_DEPLOYMENT_TARGET", minimum_version.clone())],
        link_inputs: Vec::new(),
    })?;

    let out = project.out_dir(destination.dir_name(), profile);
    let app = out.join(format!("{}.app", project.name));
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    fs::create_dir_all(&app)?;
    fs::copy(&executable, app.join(&project.bin))?;
    let icon_names = write_icons(project, &app)?;
    plist::to_file_xml(
        app.join("Info.plist"),
        &info_plist(project, destination, &minimum_version, &icon_names),
    )?;
    for (source, relative) in project.resource_files()? {
        let dest = app.join(relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
    }
    match destination {
        Destination::Simulator => sign_ad_hoc(&app)?,
        Destination::Device => sign_for_device(project, &app, &out)?,
    }
    Ok(App {
        path: app,
        bundle_id: project.identifier.clone(),
    })
}

/// `Payload/<name>.app` zipped, the shape TestFlight and device installers take.
pub fn ipa(app: &App, out: &Path) -> Result<PathBuf> {
    let payload = out.join("Payload");
    if payload.exists() {
        fs::remove_dir_all(&payload)?;
    }
    fs::create_dir_all(&payload)?;
    let name = app.path.file_name().context("app folder has no name")?;
    tools::run(
        Command::new("cp")
            .arg("-R")
            .arg(&app.path)
            .arg(payload.join(name)),
        "copying the app into Payload",
    )?;
    let stem = app.path.file_stem().context("app folder has no name")?;
    let ipa = out.join(format!("{}.ipa", stem.to_string_lossy()));
    if ipa.exists() {
        fs::remove_file(&ipa)?;
    }
    tools::run(
        Command::new("ditto")
            .current_dir(out)
            .args(["-c", "-k", "--sequesterRsrc", "--keepParent", "Payload"])
            .arg(&ipa),
        "ditto",
    )?;
    fs::remove_dir_all(&payload)?;
    Ok(ipa)
}

fn info_plist(
    project: &Project,
    destination: Destination,
    minimum_version: &str,
    icon_names: &[String],
) -> Value {
    let mut info = apple::info_plist(project);
    let mut put = |key: &str, value: Value| {
        info.insert(key.to_owned(), value);
    };
    let text = |s: &str| Value::String(s.to_owned());
    put(
        "CFBundleSupportedPlatforms",
        Value::Array(vec![text(destination.supported_platform())]),
    );
    put("DTPlatformName", text(destination.platform_name()));
    put("DTSDKName", text(destination.platform_name()));
    put("LSRequiresIPhoneOS", Value::Boolean(true));
    put("MinimumOSVersion", text(minimum_version));
    put(
        "UIDeviceFamily",
        Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
    );
    // Without a launch screen entry iOS runs the app letterboxed at a legacy size.
    put("UILaunchScreen", Value::Dictionary(Dictionary::new()));
    // UIKit's scene lifecycle, which iOS 27 requires of an app built with its SDK: each window
    // scene's delegate is Waft's host's, which the host registers before UIKit starts.
    put("UIApplicationSceneManifest", scene_manifest());
    // A ProMotion screen's full rate for the host's display link, which an iPhone caps at 60 Hz
    // otherwise.
    put("CADisableMinimumFrameDurationOnPhone", Value::Boolean(true));
    put(
        "UISupportedInterfaceOrientations",
        Value::Array(
            [
                "UIInterfaceOrientationPortrait",
                "UIInterfaceOrientationLandscapeLeft",
                "UIInterfaceOrientationLandscapeRight",
                "UIInterfaceOrientationPortraitUpsideDown",
            ]
            .iter()
            .map(|s| text(s))
            .collect(),
        ),
    );
    if destination == Destination::Device {
        put(
            "UIRequiredDeviceCapabilities",
            Value::Array(vec![text("arm64")]),
        );
    }
    if !icon_names.is_empty() {
        let mut primary = Dictionary::new();
        primary.insert(
            "CFBundleIconFiles".to_owned(),
            Value::Array(icon_names.iter().map(|n| text(n)).collect()),
        );
        let mut icons = Dictionary::new();
        icons.insert("CFBundlePrimaryIcon".to_owned(), Value::Dictionary(primary));
        put("CFBundleIcons", Value::Dictionary(icons.clone()));
        put("CFBundleIcons~ipad", Value::Dictionary(icons));
    }
    Value::Dictionary(info)
}

/// One kind of scene, the app's window, whose delegate is `WaftSceneDelegate`, the iOS host's; one
/// scene at a time.
fn scene_manifest() -> Value {
    let text = |s: &str| Value::String(s.to_owned());
    let mut window = Dictionary::new();
    window.insert(
        "UISceneConfigurationName".to_owned(),
        text("Default Configuration"),
    );
    window.insert(
        "UISceneDelegateClassName".to_owned(),
        text("WaftSceneDelegate"),
    );
    let mut configurations = Dictionary::new();
    configurations.insert(
        "UIWindowSceneSessionRoleApplication".to_owned(),
        Value::Array(vec![Value::Dictionary(window)]),
    );
    let mut manifest = Dictionary::new();
    manifest.insert(
        "UIApplicationSupportsMultipleScenes".to_owned(),
        Value::Boolean(false),
    );
    manifest.insert(
        "UISceneConfigurations".to_owned(),
        Value::Dictionary(configurations),
    );
    Value::Dictionary(manifest)
}

/// Legacy icon files, which need no asset catalog. Returns the base names for `CFBundleIconFiles`.
fn write_icons(project: &Project, app: &Path) -> Result<Vec<String>> {
    let Some(icon) = &project.icon else {
        return Ok(Vec::new());
    };
    let source = icons::load(icon)?;
    for (file, size) in [
        ("AppIcon60x60@2x.png", 120),
        ("AppIcon60x60@3x.png", 180),
        ("AppIcon76x76@2x~ipad.png", 152),
        ("AppIcon83.5x83.5@2x~ipad.png", 167),
    ] {
        icons::write_resized(&source, size, &app.join(file))?;
    }
    Ok(vec![
        "AppIcon60x60".to_owned(),
        "AppIcon76x76".to_owned(),
        "AppIcon83.5x83.5".to_owned(),
    ])
}

fn sign_ad_hoc(app: &Path) -> Result<()> {
    apple::sign(
        app,
        &SigningSettings {
            identity: &SigningIdentity::AdHoc,
            hardened_runtime: false,
            entitlements: None,
            timestamp: false,
        },
    )
}

fn sign_for_device(project: &Project, app: &Path, out: &Path) -> Result<()> {
    let identity = apple::development_identity(project.ios.team.as_deref())?;
    let profile = profiles::provisioning_profile(&project.identifier, &identity.team)?;
    fs::copy(&profile.path, app.join("embedded.mobileprovision"))?;
    let entitlements = out.join("entitlements.plist");
    plist::to_file_xml(
        &entitlements,
        &entitlements_plist(&project.identifier, &identity.team),
    )?;
    println!("signing with {} ({})", identity.name, profile.name);
    apple::sign(
        app,
        &SigningSettings {
            identity: &SigningIdentity::Keychain(identity),
            hardened_runtime: false,
            entitlements: Some(&entitlements),
            timestamp: false,
        },
    )
}

fn entitlements_plist(bundle_id: &str, team: &str) -> Value {
    let mut entitlements = Dictionary::new();
    entitlements.insert(
        "application-identifier".to_owned(),
        Value::String(format!("{team}.{bundle_id}")),
    );
    entitlements.insert(
        "com.apple.developer.team-identifier".to_owned(),
        Value::String(team.to_owned()),
    );
    entitlements.insert("get-task-allow".to_owned(), Value::Boolean(true));
    Value::Dictionary(entitlements)
}

pub fn run_simulator(app: &App, simulator: &Simulator) -> Result<()> {
    let udid = simulator.udid.as_str();
    if simulator.state != "Booted" {
        println!("booting {} ({})", simulator.name, simulator.runtime);
        let boot = Command::new("xcrun")
            .args(["simctl", "boot", udid])
            .output()?;
        let stderr = String::from_utf8_lossy(&boot.stderr);
        if !boot.status.success() && !stderr.contains("Booted") {
            bail!("simctl boot failed: {}", stderr.trim());
        }
    }
    tools::run(
        Command::new("open").args(["-a", "Simulator"]),
        "open Simulator",
    )?;
    tools::run(
        Command::new("xcrun").args(["simctl", "bootstatus", udid, "-b"]),
        "simctl bootstatus",
    )?;
    tools::run(
        Command::new("xcrun")
            .args(["simctl", "install", udid])
            .arg(&app.path),
        "simctl install",
    )?;
    let _ = Command::new("xcrun")
        .args(["simctl", "terminate", udid, &app.bundle_id])
        .output();
    println!("launching {} on {}", app.bundle_id, simulator.name);
    tools::run(
        Command::new("xcrun").args(["simctl", "launch", "--console-pty", udid, &app.bundle_id]),
        "simctl launch",
    )
}

pub fn run_device(app: &App, device: &str) -> Result<()> {
    tools::run(
        Command::new("xcrun")
            .args(["devicectl", "device", "install", "app", "--device", device])
            .arg(&app.path),
        "devicectl install",
    )?;
    println!("launching {} on {device}", app.bundle_id);
    tools::run(
        Command::new("xcrun").args([
            "devicectl",
            "device",
            "process",
            "launch",
            "--console",
            "--device",
            device,
            &app.bundle_id,
        ]),
        "devicectl launch",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_plist_names_the_executable_and_platform() {
        let project = test_project();
        let plist = info_plist(&project, Destination::Simulator, "15.0", &[]);
        let dict = plist.as_dictionary().unwrap();
        assert_eq!(
            dict.get("CFBundleExecutable").unwrap().as_string(),
            Some("myapp")
        );
        assert_eq!(
            dict.get("DTPlatformName").unwrap().as_string(),
            Some("iphonesimulator")
        );
        assert!(dict.get("UILaunchScreen").is_some());
        assert!(dict.get("CFBundleIcons").is_none());
        assert!(dict.get("UIRequiredDeviceCapabilities").is_none());
        assert!(dict.get("NSCameraUsageDescription").is_none());
    }

    #[test]
    fn info_plist_adopts_the_scene_lifecycle_with_the_hosts_delegate() {
        let project = test_project();
        let plist = info_plist(&project, Destination::Simulator, "15.0", &[]);
        let dict = plist.as_dictionary().unwrap();
        let manifest = dict
            .get("UIApplicationSceneManifest")
            .and_then(Value::as_dictionary)
            .expect("a scene manifest");
        assert_eq!(
            manifest
                .get("UIApplicationSupportsMultipleScenes")
                .and_then(Value::as_boolean),
            Some(false)
        );
        let window = manifest
            .get("UISceneConfigurations")
            .and_then(Value::as_dictionary)
            .and_then(|configurations| configurations.get("UIWindowSceneSessionRoleApplication"))
            .and_then(Value::as_array)
            .and_then(|configurations| configurations.first())
            .and_then(Value::as_dictionary)
            .expect("the window scene's configuration");
        assert_eq!(
            window
                .get("UISceneDelegateClassName")
                .and_then(Value::as_string),
            Some("WaftSceneDelegate")
        );
        assert_eq!(
            dict.get("CADisableMinimumFrameDurationOnPhone")
                .and_then(Value::as_boolean),
            Some(true)
        );
    }

    #[test]
    fn info_plist_says_why_the_app_uses_the_camera() {
        let mut project = test_project();
        project.usage_descriptions.camera = Some("Scans the code on a ticket.".into());
        let plist = info_plist(&project, Destination::Device, "15.0", &[]);
        let dict = plist.as_dictionary().unwrap();
        assert_eq!(
            dict.get("NSCameraUsageDescription").unwrap().as_string(),
            Some("Scans the code on a ticket.")
        );
    }

    fn test_project() -> Project {
        Project {
            root: PathBuf::from("/tmp/x"),
            workspace_root: PathBuf::from("/tmp/x"),
            target_dir: PathBuf::from("/tmp/x/target"),
            package: "myapp".into(),
            bin: "myapp".into(),
            library: Some("myapp".into()),
            version: "0.1.0".into(),
            description: None,
            homepage: None,
            authors: Vec::new(),
            name: "My App".into(),
            identifier: "com.example.myapp".into(),
            icon: None,
            resources: Vec::new(),
            file_associations: Vec::new(),
            usage_descriptions: Default::default(),
            macos: Default::default(),
            ios: Default::default(),
            android: Default::default(),
        }
    }
}

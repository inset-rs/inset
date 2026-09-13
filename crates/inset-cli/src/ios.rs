//! iOS: the `.app` folder Xcode would produce, assembled from a cargo binary.
//!
//! No Xcode project. The binary, an `Info.plist`, icons, and resources go into a
//! folder; `codesign` signs it (ad hoc for the simulator, a development identity
//! and profile for a device); `simctl` or `devicectl` installs and launches it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result, bail};
use plist::{Dictionary, Value};

use crate::cargo::{self, Build, Kind};
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
    let artifacts = cargo::build(&Build {
        project,
        profile,
        triple: Some(destination.triple()),
        kind: Kind::Bin,
        env: vec![("IPHONEOS_DEPLOYMENT_TARGET", minimum_version.clone())],
    })?;
    let executable = artifacts
        .executable
        .context("cargo produced no executable")?;

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
    let mut info = Dictionary::new();
    let mut put = |key: &str, value: Value| {
        info.insert(key.to_owned(), value);
    };
    let text = |s: &str| Value::String(s.to_owned());
    put("CFBundleDevelopmentRegion", text("en"));
    put("CFBundleExecutable", text(&project.bin));
    put("CFBundleIdentifier", text(&project.identifier));
    put("CFBundleInfoDictionaryVersion", text("6.0"));
    put("CFBundleName", text(&project.name));
    put("CFBundleDisplayName", text(&project.name));
    put("CFBundlePackageType", text("APPL"));
    put("CFBundleShortVersionString", text(&project.version));
    put("CFBundleVersion", text(&project.version));
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
    tools::run(
        Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(app),
        "codesign",
    )
}

fn sign_for_device(project: &Project, app: &Path, out: &Path) -> Result<()> {
    let identity = signing_identity(project.ios.team.as_deref())?;
    let profile = provisioning_profile(&project.identifier, &identity.team)?;
    fs::copy(&profile.path, app.join("embedded.mobileprovision"))?;
    let entitlements = out.join("entitlements.plist");
    plist::to_file_xml(
        &entitlements,
        &entitlements_plist(&project.identifier, &identity.team),
    )?;
    tools::run(
        Command::new("codesign")
            .args([
                "--force",
                "--timestamp=none",
                "--generate-entitlement-der",
                "--sign",
            ])
            .arg(&identity.hash)
            .arg("--entitlements")
            .arg(&entitlements)
            .arg(app),
        "codesign",
    )?;
    println!("signed with {} ({})", identity.name, profile.name);
    Ok(())
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

struct Identity {
    hash: String,
    name: String,
    team: String,
}

/// The development certificate in the login keychain, from `security find-identity`.
fn signing_identity(team: Option<&str>) -> Result<Identity> {
    let listing = tools::output(
        Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]),
        "security find-identity",
    )?;
    let identities: Vec<Identity> = listing.lines().filter_map(parse_identity).collect();
    let development = identities
        .iter()
        .filter(|i| {
            i.name.starts_with("Apple Development") || i.name.starts_with("iPhone Developer")
        })
        .find(|i| team.is_none_or(|team| i.team == team));
    match development {
        Some(identity) => Ok(Identity {
            hash: identity.hash.clone(),
            name: identity.name.clone(),
            team: identity.team.clone(),
        }),
        None => bail!(
            "no Apple Development certificate{}; sign in to Xcode once (Settings > Accounts) to create one",
            team.map(|t| format!(" for team {t}")).unwrap_or_default()
        ),
    }
}

/// `  1) 0123…ABCD "Apple Development: Jane Doe (TEAMID)"`.
fn parse_identity(line: &str) -> Option<Identity> {
    let (hash, rest) = line.trim().split_once(") ")?.1.split_once(' ')?;
    let name = rest.trim().trim_matches('"').to_owned();
    let team = name.rsplit_once('(')?.1.trim_end_matches(')').to_owned();
    Some(Identity {
        hash: hash.to_owned(),
        name,
        team,
    })
}

struct Profile_ {
    path: PathBuf,
    name: String,
}

/// A provisioning profile Xcode has stored that covers the bundle id for the team.
fn provisioning_profile(bundle_id: &str, team: &str) -> Result<Profile_> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let dirs = [
        home.join("Library/Developer/Xcode/UserData/Provisioning Profiles"),
        home.join("Library/MobileDevice/Provisioning Profiles"),
    ];
    let mut wildcard = None;
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "mobileprovision") {
                continue;
            }
            let Some(decoded) = decode_profile(&path) else {
                continue;
            };
            if !decoded.teams.iter().any(|t| t == team) || decoded.expired() {
                continue;
            }
            match decoded.covers(bundle_id) {
                Some(true) => {
                    return Ok(Profile_ {
                        path,
                        name: decoded.name,
                    });
                }
                Some(false) => {
                    wildcard = Some(Profile_ {
                        path,
                        name: decoded.name,
                    })
                }
                None => {}
            }
        }
    }
    wildcard.with_context(|| {
        format!(
            "no provisioning profile for {bundle_id} (team {team}); run the app once from Xcode with automatic signing, or download one from developer.apple.com"
        )
    })
}

struct DecodedProfile {
    name: String,
    teams: Vec<String>,
    /// `TEAM.bundle` or `TEAM.*`.
    app_id: String,
    expiration: Option<SystemTime>,
}

impl DecodedProfile {
    fn expired(&self) -> bool {
        self.expiration.is_some_and(|when| when < SystemTime::now())
    }

    /// `Some(true)` for an exact match, `Some(false)` for a wildcard match, `None` otherwise.
    fn covers(&self, bundle_id: &str) -> Option<bool> {
        let (_, pattern) = self.app_id.split_once('.')?;
        if pattern == bundle_id {
            Some(true)
        } else if let Some(prefix) = pattern.strip_suffix('*')
            && bundle_id.starts_with(prefix)
        {
            Some(false)
        } else {
            None
        }
    }
}

fn decode_profile(path: &Path) -> Option<DecodedProfile> {
    let xml = Command::new("security")
        .args(["cms", "-D", "-i"])
        .arg(path)
        .output()
        .ok()?;
    let value: Value = plist::from_bytes(&xml.stdout).ok()?;
    let dict = value.as_dictionary()?;
    let entitlements = dict.get("Entitlements")?.as_dictionary()?;
    Some(DecodedProfile {
        name: dict.get("Name")?.as_string()?.to_owned(),
        teams: dict
            .get("TeamIdentifier")?
            .as_array()?
            .iter()
            .filter_map(|t| t.as_string().map(str::to_owned))
            .collect(),
        app_id: entitlements
            .get("application-identifier")?
            .as_string()?
            .to_owned(),
        expiration: dict
            .get("ExpirationDate")
            .and_then(Value::as_date)
            .map(SystemTime::from),
    })
}

#[derive(Clone, Debug)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    pub state: String,
    /// `iOS 18.6`.
    pub runtime: String,
}

/// Available iOS simulators, newest runtime first.
pub fn simulators() -> Result<Vec<Simulator>> {
    if !cfg!(target_os = "macos") || tools::which("xcrun").is_none() {
        return Ok(Vec::new());
    }
    let json = tools::output(
        Command::new("xcrun").args(["simctl", "list", "-j", "devices", "available"]),
        "xcrun simctl list",
    )?;
    Ok(parse_simulators(&json))
}

fn parse_simulators(json: &str) -> Vec<Simulator> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(devices) = value.get("devices").and_then(|d| d.as_object()) else {
        return Vec::new();
    };
    let mut runtimes: Vec<(Vec<u32>, String, &serde_json::Value)> = devices
        .iter()
        .filter_map(|(runtime, list)| {
            let (family, version) = runtime.rsplit_once('.')?.1.split_once('-')?;
            if family != "iOS" {
                return None;
            }
            let numbers: Vec<u32> = version.split('-').filter_map(|n| n.parse().ok()).collect();
            let label = format!("iOS {}", version.replace('-', "."));
            Some((numbers, label, list))
        })
        .collect();
    runtimes.sort_by(|a, b| b.0.cmp(&a.0));
    runtimes
        .into_iter()
        .flat_map(|(_, runtime, list)| {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(move |device| {
                    Some(Simulator {
                        udid: device.get("udid")?.as_str()?.to_owned(),
                        name: device.get("name")?.as_str()?.to_owned(),
                        state: device.get("state")?.as_str()?.to_owned(),
                        runtime: runtime.clone(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The simulator `-d` names, or the booted one, or the newest iPhone.
pub fn pick_simulator<'a>(
    simulators: &'a [Simulator],
    wanted: Option<&str>,
) -> Option<&'a Simulator> {
    match wanted {
        Some(wanted) => {
            let lower = wanted.to_lowercase();
            simulators
                .iter()
                .find(|s| s.udid.eq_ignore_ascii_case(wanted))
                .or_else(|| simulators.iter().find(|s| s.name.to_lowercase() == lower))
                .or_else(|| {
                    simulators
                        .iter()
                        .find(|s| s.name.to_lowercase().starts_with(&lower))
                })
        }
        None => simulators
            .iter()
            .find(|s| s.state == "Booted")
            .or_else(|| simulators.iter().find(|s| s.name.starts_with("iPhone")))
            .or_else(|| simulators.first()),
    }
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
    fn identity_line_yields_hash_name_and_team() {
        let identity =
            parse_identity(r#"  1) ABCDEF0123456789ABCDEF0123456789ABCDEF01 "Apple Development: Jane Doe (TEAM12345)""#)
                .unwrap();
        assert_eq!(identity.hash, "ABCDEF0123456789ABCDEF0123456789ABCDEF01");
        assert_eq!(identity.name, "Apple Development: Jane Doe (TEAM12345)");
        assert_eq!(identity.team, "TEAM12345");
    }

    #[test]
    fn profile_coverage_distinguishes_exact_and_wildcard() {
        let profile = DecodedProfile {
            name: "p".into(),
            teams: vec!["T".into()],
            app_id: "T.com.example.*".into(),
            expiration: None,
        };
        assert_eq!(profile.covers("com.example.app"), Some(false));
        assert_eq!(profile.covers("org.other.app"), None);
        let exact = DecodedProfile {
            app_id: "T.com.example.app".into(),
            ..profile
        };
        assert_eq!(exact.covers("com.example.app"), Some(true));
    }

    #[test]
    fn simulators_come_newest_runtime_first_and_ios_only() {
        let json = r#"{"devices":{
            "com.apple.CoreSimulator.SimRuntime.iOS-17-5":[{"name":"iPhone 15","udid":"A","state":"Shutdown"}],
            "com.apple.CoreSimulator.SimRuntime.tvOS-18-0":[{"name":"Apple TV","udid":"T","state":"Shutdown"}],
            "com.apple.CoreSimulator.SimRuntime.iOS-18-6":[{"name":"iPhone 16","udid":"B","state":"Booted"}]}}"#;
        let list = parse_simulators(json);
        assert_eq!(
            list.iter().map(|s| s.udid.as_str()).collect::<Vec<_>>(),
            ["B", "A"]
        );
        assert_eq!(list[0].runtime, "iOS 18.6");
        assert_eq!(pick_simulator(&list, None).unwrap().udid, "B");
        assert_eq!(pick_simulator(&list, Some("iphone 15")).unwrap().udid, "A");
        assert_eq!(pick_simulator(&list, Some("iPhone")).unwrap().udid, "B");
    }

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
    }

    fn test_project() -> Project {
        Project {
            root: PathBuf::from("/tmp/x"),
            workspace_root: PathBuf::from("/tmp/x"),
            target_dir: PathBuf::from("/tmp/x/target"),
            package: "myapp".into(),
            bin: "myapp".into(),
            has_cdylib: true,
            version: "0.1.0".into(),
            description: None,
            homepage: None,
            authors: Vec::new(),
            name: "My App".into(),
            identifier: "com.example.myapp".into(),
            icon: None,
            resources: Vec::new(),
            macos: Default::default(),
            ios: Default::default(),
        }
    }
}

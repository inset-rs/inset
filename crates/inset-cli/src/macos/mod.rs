//! macOS: the `.app`, and the disk image around it, put together and signed here as the iOS app
//! and the Android package are, and notarized when the environment holds the credentials.
//!
//! The bundle holds the app's executable in `Contents/MacOS`, its `Info.plist`, its icon and
//! resources in `Contents/Resources`, and the helpers and libraries it carries (`carried.rs`). It
//! is signed with `APPLE_SIGNING_IDENTITY`'s identity, else the keychain's Apple Development
//! certificate, so that permission grants such as Accessibility survive a rebuild, else ad hoc;
//! with an identity of the keychain's, under the hardened runtime. It is signed inside out: each
//! helper and library first, with no entitlements of its own, then the bundle, with the app's own
//! and the hardened runtime's leave to open each device it declares a usage description for. A build for distribution (`build macos`, `build dmg`) is
//! notarized when the environment holds the credentials (`notarization`); `run` never is.

mod disk_image;
mod notarization;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use plist::{Dictionary, Value};

use crate::apple::{self, SigningIdentity, SigningSettings};
use crate::cargo::{self, Artifact, Build};
use crate::carried;
use crate::file_associations;
use crate::icons;
use crate::project::{Profile, Project};
use crate::tools;
use crate::usage_descriptions;

use notarization::Notarization;

/// The name of the app's icon in `Contents/Resources`.
const ICON_FILE: &str = "AppIcon.icns";

/// Builds the `.app` for distribution: signed, and notarized when the environment holds the
/// credentials.
pub fn build_app(project: &Project, profile: Profile) -> Result<PathBuf> {
    let signing = Signing::for_distribution(project)?;
    distributed_app(project, profile, &signing)
}

/// Builds the `.app` for distribution and the disk image around it, both signed, and both
/// notarized when the environment holds the credentials, as Apple advises for a disk image.
pub fn build_dmg(project: &Project, profile: Profile) -> Result<PathBuf> {
    let signing = Signing::for_distribution(project)?;
    let app = distributed_app(project, profile, &signing)?;
    let out = project.out_dir("dmg", profile);
    fs::create_dir_all(&out)?;
    let dmg = disk_image::make(project, &app, &out)?;
    if let Some(settings) = signing.disk_image_settings() {
        apple::sign(&dmg, &settings)?;
    }
    if let Some(notarization) = &signing.notarization {
        notarization.notarize_disk_image(&dmg)?;
    }
    Ok(dmg)
}

/// Builds the `.app` and runs its executable in place, the way Flutter's macOS runner does: the
/// process carries the bundle's identifier, icon, and `Info.plist`, and its stdio is the
/// terminal.
pub fn run(project: &Project, profile: Profile, args: &[String]) -> Result<()> {
    let signing = Signing::for_development(project)?;
    let app = make_app(project, profile, &signing)?;
    let executable = app.join("Contents/MacOS").join(&project.bin);
    let status = Command::new(&executable)
        .args(args)
        .status()
        .with_context(|| format!("could not start {}", executable.display()))?;
    if !status.success() {
        bail!("{} exited with {status}", project.name);
    }
    Ok(())
}

/// Who signs the bundle, and who notarizes it.
struct Signing {
    identity: SigningIdentity,
    /// `None` for a development build, and where the environment holds no credentials.
    notarization: Option<Notarization>,
}

impl Signing {
    /// For a build that runs here: no notarization.
    fn for_development(project: &Project) -> Result<Signing> {
        Ok(Signing {
            identity: signing_identity(project)?,
            notarization: None,
        })
    }

    /// For a build that is handed out: notarized when the environment holds the credentials,
    /// which only an identity named by `APPLE_SIGNING_IDENTITY`, a Developer ID, can be.
    fn for_distribution(project: &Project) -> Result<Signing> {
        let notarization = Notarization::from_env()?;
        if notarization.is_some() && !named_identity_is_set() {
            bail!(
                "notarizing needs a Developer ID: set APPLE_SIGNING_IDENTITY to the one to sign with"
            );
        }
        Ok(Signing {
            identity: signing_identity(project)?,
            notarization,
        })
    }

    /// How a disk image is signed: with the bundle's identity, when it is the keychain's; an
    /// image is not signed ad hoc.
    fn disk_image_settings(&self) -> Option<SigningSettings<'_>> {
        matches!(self.identity, SigningIdentity::Keychain(_)).then(|| SigningSettings {
            identity: &self.identity,
            hardened_runtime: false,
            entitlements: None,
            timestamp: self.notarization.is_some(),
        })
    }

    /// How the bundle is signed, with `entitlements`, and each helper and library in it, with
    /// none.
    fn settings<'a>(&'a self, entitlements: Option<&'a Path>) -> SigningSettings<'a> {
        SigningSettings {
            identity: &self.identity,
            hardened_runtime: matches!(self.identity, SigningIdentity::Keychain(_)),
            entitlements,
            timestamp: self.notarization.is_some(),
        }
    }
}

/// `APPLE_SIGNING_IDENTITY`'s identity, `-` meaning ad hoc; else the keychain's Apple
/// Development certificate; else ad hoc.
fn signing_identity(project: &Project) -> Result<SigningIdentity> {
    let identity = match std::env::var("APPLE_SIGNING_IDENTITY") {
        Ok(named) if named == "-" => SigningIdentity::AdHoc,
        Ok(named) => SigningIdentity::Keychain(apple::keychain_identity(&named)?),
        Err(_) => apple::development_identity(project.macos.team.as_deref())
            .map_or(SigningIdentity::AdHoc, SigningIdentity::Keychain),
    };
    match &identity {
        SigningIdentity::Keychain(identity) => println!("signing with {}", identity.name),
        SigningIdentity::AdHoc => println!("signing ad hoc"),
    }
    Ok(identity)
}

fn named_identity_is_set() -> bool {
    std::env::var("APPLE_SIGNING_IDENTITY").is_ok_and(|named| named != "-")
}

/// The `.app` for distribution, notarized and stapled when `signing` holds credentials.
fn distributed_app(project: &Project, profile: Profile, signing: &Signing) -> Result<PathBuf> {
    let app = make_app(project, profile, signing)?;
    if let Some(notarization) = &signing.notarization {
        notarization.notarize_app(&app)?;
    }
    Ok(app)
}

/// The `.app` in `target/inset/macos/<profile>`, signed as `signing` says.
fn make_app(project: &Project, profile: Profile, signing: &Signing) -> Result<PathBuf> {
    if !cfg!(target_os = "macos") {
        bail!("macOS builds need macOS");
    }
    let executable = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: None,
        artifact: Artifact::Executable(&project.bin),
        env: Vec::new(),
    })?;
    let carried = carried::build(project, profile)?;
    let out = project.out_dir("macos", profile);
    let app = out.join(format!("{}.app", project.name));
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    let contents = app.join("Contents");
    let resources = contents.join("Resources");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(&resources)?;
    fs::copy(&executable, contents.join("MacOS").join(&project.bin))?;
    let icon = write_icon(project, &resources, &out)?;
    plist::to_file_xml(
        contents.join("Info.plist"),
        &Value::Dictionary(info_plist(project, icon)),
    )?;
    copy_resources(project, &resources)?;
    for placed in carried::put_in(&contents, &carried)? {
        apple::sign(&placed, &signing.settings(None))?;
    }
    let entitlements = write_entitlements(project, &out)?;
    apple::sign(&app, &signing.settings(entitlements.as_deref()))?;
    Ok(app)
}

/// The bundle's `Info.plist`: what every Apple bundle has, and what a Mac app adds: its icon,
/// the system it needs, its high-resolution drawing, whether it stays out of the Dock, and the
/// kinds of file it opens.
fn info_plist(project: &Project, icon: Option<&str>) -> Dictionary {
    let mut info = apple::info_plist(project);
    if let Some(icon) = icon {
        info.insert("CFBundleIconFile".into(), icon.into());
    }
    if let Some(version) = &project.macos.minimum_system_version {
        info.insert("LSMinimumSystemVersion".into(), version.clone().into());
    }
    info.insert("NSHighResolutionCapable".into(), true.into());
    if project.macos.background_app {
        info.insert("LSUIElement".into(), true.into());
    }
    if !project.file_associations.is_empty() {
        info.extend(file_associations::macos_info_plist(
            &project.file_associations,
            &project.identifier,
        ));
    }
    info
}

/// The app's icon as an `.icns` in `resources`, made by `iconutil` from every size of the app's
/// one PNG an `.iconset` holds, in a folder of `work`; `None` for an app without an icon.
fn write_icon(project: &Project, resources: &Path, work: &Path) -> Result<Option<&'static str>> {
    let Some(icon) = &project.icon else {
        return Ok(None);
    };
    let source = icons::load(icon)?;
    let iconset = work.join("AppIcon.iconset");
    if iconset.exists() {
        fs::remove_dir_all(&iconset)?;
    }
    fs::create_dir_all(&iconset)?;
    for (points, scale) in [16, 32, 128, 256, 512]
        .into_iter()
        .flat_map(|points| [(points, 1), (points, 2)])
    {
        let suffix = if scale == 2 { "@2x" } else { "" };
        let file = iconset.join(format!("icon_{points}x{points}{suffix}.png"));
        icons::write_resized(&source, points * scale, &file)?;
    }
    tools::run(
        Command::new("iconutil")
            .args(["--convert", "icns", "--output"])
            .arg(resources.join(ICON_FILE))
            .arg(&iconset),
        "iconutil",
    )?;
    Ok(Some(ICON_FILE))
}

/// Every resource file, at its path in the package, under `resources`.
fn copy_resources(project: &Project, resources: &Path) -> Result<()> {
    for (source, relative) in project.resource_files()? {
        let dest = resources.join(relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
    }
    Ok(())
}

/// The entitlements the bundle is signed with, in a file in `out`: the app's own, and for each
/// device the app declares a usage description for, the hardened runtime's leave to open it,
/// unless the app's own file says otherwise. `None` when there are none.
fn write_entitlements(project: &Project, out: &Path) -> Result<Option<PathBuf>> {
    let devices = usage_descriptions::macos_entitlements(&project.usage_descriptions);
    let own = project.macos.entitlements.clone();
    if devices.is_empty() {
        return Ok(own);
    }
    let mut entitlements = match &own {
        Some(path) => plist::Value::from_file(path)
            .with_context(|| format!("could not read the entitlements {}", path.display()))?
            .into_dictionary()
            .with_context(|| format!("the entitlements {} are not a dictionary", path.display()))?,
        None => Dictionary::new(),
    };
    for (key, value) in devices {
        if !entitlements.contains_key(&key) {
            entitlements.insert(key, value);
        }
    }
    let path = out.join("entitlements.plist");
    plist::to_file_xml(&path, &entitlements).context("could not write the entitlements")?;
    Ok(Some(path))
}

//! Desktop bundles and installers, through cargo-packager.
//!
//! The app's file associations go to cargo-packager for every format, and the macOS
//! bundle's `Info.plist` also gets what `file_associations` writes beyond them and the usage
//! descriptions, whose devices its entitlements allow. A macOS bundle that carries helpers or
//! libraries is put together and signed in two steps (`carried`): cargo-packager makes it
//! unsigned, and the CLI adds what it carries and signs it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_packager::config::{Binary, MacOsConfig, Resource};
use cargo_packager::{Config, PackageFormat};
use plist::Dictionary;

use crate::cargo::{self, Artifact, Build};
use crate::carried;
use crate::file_associations;
use crate::icons;
use crate::ios;
use crate::project::{Profile, Project};
use crate::usage_descriptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The executable and resources in a folder; no installer.
    Folder,
    App,
    Dmg,
    Msi,
    Nsis,
    Deb,
    AppImage,
}

impl Format {
    fn dir_name(self) -> &'static str {
        match self {
            Format::Folder => std::env::consts::OS,
            Format::App => "macos",
            Format::Dmg => "dmg",
            Format::Msi => "msi",
            Format::Nsis => "nsis",
            Format::Deb => "deb",
            Format::AppImage => "appimage",
        }
    }

    fn package_format(self) -> Option<PackageFormat> {
        match self {
            Format::Folder => None,
            Format::App => Some(PackageFormat::App),
            Format::Dmg => Some(PackageFormat::Dmg),
            Format::Msi => Some(PackageFormat::Wix),
            Format::Nsis => Some(PackageFormat::Nsis),
            Format::Deb => Some(PackageFormat::Deb),
            Format::AppImage => Some(PackageFormat::AppImage),
        }
    }
}

pub fn build(project: &Project, profile: Profile, format: Format) -> Result<Vec<PathBuf>> {
    if format == Format::Dmg && carried::any(project) {
        bail!(
            "a disk image of an app that carries helpers or libraries is not built yet: \
             cargo-packager makes it from a bundle it signs before they are in"
        );
    }
    let executable = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: None,
        artifact: Artifact::Executable(&project.bin),
        env: Vec::new(),
    })?;
    let out = project.out_dir(format.dir_name(), profile);
    fs::create_dir_all(&out)?;
    let Some(package_format) = format.package_format() else {
        return folder(project, &executable, &out);
    };
    let macos = macos_config(project, &out, package_format)?;
    match package_format {
        PackageFormat::App if carried::any(project) => {
            app_carrying(project, profile, &executable, &out, macos)
        }
        _ => package(project, &executable, &out, package_format, macos),
    }
}

/// The `.app` with the helpers and libraries it carries: cargo-packager makes it unsigned, since
/// it would sign the bundle before they are in, then they are put in and the bundle is signed
/// inside out with the identity and entitlements cargo-packager would have used.
fn app_carrying(
    project: &Project,
    profile: Profile,
    executable: &Path,
    out: &Path,
    mut macos: MacOsConfig,
) -> Result<Vec<PathBuf>> {
    let carried = carried::build(project, profile)?;
    let identity = macos.signing_identity.take();
    let entitlements = macos.entitlements.clone().map(PathBuf::from);
    let outputs = package(project, executable, out, PackageFormat::App, macos)?;
    let app = app_bundle(&outputs)?;
    carried::put_in(app, &carried, identity.as_deref(), entitlements.as_deref())?;
    Ok(outputs)
}

fn folder(project: &Project, executable: &Path, out: &Path) -> Result<Vec<PathBuf>> {
    let dir = out.join(&project.name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    let name = executable
        .file_name()
        .context("executable has no file name")?;
    fs::copy(executable, dir.join(name))?;
    for (source, relative) in project.resource_files()? {
        let dest = dir.join(relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
    }
    Ok(vec![dir])
}

fn package(
    project: &Project,
    executable: &Path,
    out: &Path,
    format: PackageFormat,
    macos: MacOsConfig,
) -> Result<Vec<PathBuf>> {
    let icons = icon_set(project, &out.join("icons"))?;
    let resources = project
        .resource_files()?
        .into_iter()
        .map(|(source, relative)| Resource::Mapped {
            src: source.to_string_lossy().into_owned(),
            target: relative,
        })
        .collect();
    let mut binary = Binary::new(executable.to_path_buf());
    binary.main = true;
    let mut config = Config::default();
    config.product_name = project.name.clone();
    config.version = project.version.clone();
    config.identifier = Some(project.identifier.clone());
    config.binaries = vec![binary];
    config.out_dir = out.to_path_buf();
    config.formats = Some(vec![format]);
    config.icons = (!icons.is_empty()).then_some(icons);
    config.resources = Some(resources);
    config.file_associations = (!project.file_associations.is_empty())
        .then(|| file_associations::packager_associations(&project.file_associations));
    config.description = project.description.clone();
    config.homepage = project.homepage.clone();
    config.authors = (!project.authors.is_empty()).then(|| project.authors.clone());
    config.macos = Some(macos);
    let outputs = cargo_packager::package(&config).context("packaging failed")?;
    Ok(outputs
        .into_iter()
        .flat_map(|output| output.paths)
        .collect())
}

/// How cargo-packager makes and signs a macOS bundle, for `format`: the app's own settings, and
/// for a bundle or a disk image also its `Info.plist` additions and its entitlements.
fn macos_config(project: &Project, out: &Path, format: PackageFormat) -> Result<MacOsConfig> {
    let mut macos = MacOsConfig::default();
    macos.minimum_system_version = project.macos.minimum_system_version.clone();
    macos.background_app = project.macos.background_app;
    macos.signing_identity = macos_signing_identity(project);
    if matches!(format, PackageFormat::App | PackageFormat::Dmg) {
        macos.info_plist_path = macos_info_plist_additions(project, out)?;
        macos.entitlements = macos_entitlements(project, out)?
            .map(|entitlements| entitlements.to_string_lossy().into_owned());
    }
    Ok(macos)
}

/// The `Info.plist` keys cargo-packager does not write, in a file beside the bundle that it
/// merges over its own: the file associations' and the usage descriptions'. `None` when the app
/// has neither.
fn macos_info_plist_additions(project: &Project, out: &Path) -> Result<Option<PathBuf>> {
    let mut info = Dictionary::new();
    if !project.file_associations.is_empty() {
        info = file_associations::macos_info_plist(&project.file_associations, &project.identifier);
    }
    info.extend(usage_descriptions::info_plist(&project.usage_descriptions));
    if info.is_empty() {
        return Ok(None);
    }
    let path = out.join("info-additions.plist");
    plist::to_file_xml(&path, &info)
        .context("could not write the Info.plist keys cargo-packager leaves out")?;
    Ok(Some(path))
}

/// The entitlements the bundle is signed with: the app's own, and for each device the app
/// declares a usage description for, the hardened runtime's leave to open it, unless the app's
/// own file says otherwise. `None` when there are none.
fn macos_entitlements(project: &Project, out: &Path) -> Result<Option<PathBuf>> {
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

/// `APPLE_SIGNING_IDENTITY` for distribution, else the keychain's development
/// certificate so the bundle keeps one identity across rebuilds: macOS ties
/// permission grants such as Accessibility to the signature, and an ad-hoc
/// signature changes with every build.
fn macos_signing_identity(project: &Project) -> Option<String> {
    if let Ok(identity) = std::env::var("APPLE_SIGNING_IDENTITY") {
        return Some(identity);
    }
    if !cfg!(target_os = "macos") {
        return None;
    }
    match ios::signing_identity(project.macos.team.as_deref()) {
        Ok(identity) => {
            println!("signing with {}", identity.name);
            Some(identity.hash)
        }
        Err(_) => None,
    }
}

/// Builds the `.app` and runs its executable in place, the way Flutter's macOS
/// runner does: the process carries the bundle's identifier, icon, and
/// `Info.plist`, and its stdio is the terminal.
pub fn run_bundle(project: &Project, profile: Profile, args: &[String]) -> Result<()> {
    let outputs = build(project, profile, Format::App)?;
    let executable = app_bundle(&outputs)?
        .join("Contents/MacOS")
        .join(&project.bin);
    let status = std::process::Command::new(&executable)
        .args(args)
        .status()
        .with_context(|| format!("could not start {}", executable.display()))?;
    if !status.success() {
        anyhow::bail!("{} exited with {status}", project.name);
    }
    Ok(())
}

/// The `.app` among what cargo-packager made.
fn app_bundle(outputs: &[PathBuf]) -> Result<&Path> {
    outputs
        .iter()
        .find(|path| path.extension().is_some_and(|e| e == "app"))
        .map(PathBuf::as_path)
        .context("packaging produced no .app")
}

/// PNGs at every size the icns and ico builders want, from the app's one icon.
fn icon_set(project: &Project, dir: &Path) -> Result<Vec<String>> {
    let Some(icon) = &project.icon else {
        return Ok(Vec::new());
    };
    let source = icons::load(icon)?;
    fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    // icns has no 1024 px slot at 1x; that size is the 512 slot at 2x, which
    // cargo-packager reads from the `@2x` name.
    for (size, file) in [
        (16, "16x16.png"),
        (32, "32x32.png"),
        (128, "128x128.png"),
        (256, "256x256.png"),
        (512, "512x512.png"),
        (1024, "512x512@2x.png"),
    ] {
        let path = dir.join(file);
        icons::write_resized(&source, size, &path)?;
        paths.push(path.to_string_lossy().into_owned());
    }
    Ok(paths)
}

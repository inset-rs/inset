//! Windows and Linux bundles and installers, through cargo-packager; macOS's are the CLI's own
//! (`macos/`).
//!
//! The app's file associations go to cargo-packager for every format. A Linux package's desktop
//! entry and the types the app owns are the CLI's own (`freedesktop`), which the package carries
//! among its files in place of cargo-packager's entry.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_packager::config::{AppImageConfig, Binary, DebianConfig, LinuxConfig, Resource};
use cargo_packager::{Config, PackageFormat};

use crate::cargo::{self, Artifact, Build};
use crate::file_associations;
use crate::freedesktop;
use crate::icons;
use crate::project::{Profile, Project};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The executable and resources in a folder; no installer.
    Folder,
    Msi,
    Nsis,
    Deb,
    AppImage,
}

impl Format {
    fn dir_name(self) -> &'static str {
        match self {
            Format::Folder => std::env::consts::OS,
            Format::Msi => "msi",
            Format::Nsis => "nsis",
            Format::Deb => "deb",
            Format::AppImage => "appimage",
        }
    }

    fn package_format(self) -> Option<PackageFormat> {
        match self {
            Format::Folder => None,
            Format::Msi => Some(PackageFormat::Wix),
            Format::Nsis => Some(PackageFormat::Nsis),
            Format::Deb => Some(PackageFormat::Deb),
            Format::AppImage => Some(PackageFormat::AppImage),
        }
    }
}

pub fn build(project: &Project, profile: Profile, format: Format) -> Result<Vec<PathBuf>> {
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
    match format.package_format() {
        None => folder(project, &executable, &out),
        Some(package_format) => package(project, &executable, &out, package_format),
    }
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
    if let Some(entry_name) = linux_entry_name(project, format) {
        add_freedesktop_files(&mut config, project, &entry_name, out)?;
    }
    let outputs = cargo_packager::package(&config).context("packaging failed")?;
    Ok(outputs
        .into_iter()
        .flat_map(|output| output.paths)
        .collect())
}

/// What a Linux package's desktop entry is named: after the app's identifier in a deb, which the
/// desktop matches the app's windows to; after the executable in an AppImage, whose tool links the
/// entry by that name. `None` for a package of another system.
fn linux_entry_name(project: &Project, format: PackageFormat) -> Option<String> {
    match format {
        PackageFormat::Deb => Some(freedesktop::deb_entry_name(project)),
        PackageFormat::AppImage => Some(format!("{}.desktop", project.bin)),
        _ => None,
    }
}

/// Has the package carry the app's own desktop entry, named `entry_name`, and its types, in place
/// of cargo-packager's entry.
fn add_freedesktop_files(
    config: &mut Config,
    project: &Project,
    entry_name: &str,
    out: &Path,
) -> Result<()> {
    let staged = freedesktop::staging_dir(out);
    freedesktop::write(project, entry_name, &staged)?;
    let files = [(staged.to_string_lossy().into_owned(), "/".to_owned())];
    // The app's entry is Waft's, named `<identifier>.desktop` in a deb and naming the windows'
    // class, so the desktop matches a window to it; cargo-packager's would be a second launcher.
    let mut linux = LinuxConfig::default();
    linux.generate_desktop_entry = false;
    config.linux = Some(linux);
    config.deb = Some(DebianConfig::new().files(files.clone()));
    config.appimage = Some(AppImageConfig::new().files(files));
    Ok(())
}

/// PNGs at every size the ico and Linux icon builders want, from the app's one icon.
fn icon_set(project: &Project, dir: &Path) -> Result<Vec<String>> {
    let Some(icon) = &project.icon else {
        return Ok(Vec::new());
    };
    let source = icons::load(icon)?;
    fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    for (size, file) in [
        (16, "16x16.png"),
        (32, "32x32.png"),
        (128, "128x128.png"),
        (256, "256x256.png"),
        (512, "512x512.png"),
    ] {
        let path = dir.join(file);
        icons::write_resized(&source, size, &path)?;
        paths.push(path.to_string_lossy().into_owned());
    }
    Ok(paths)
}

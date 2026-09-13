//! Desktop bundles and installers, through cargo-packager.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_packager::config::{Binary, MacOsConfig, Resource};
use cargo_packager::{Config, PackageFormat};

use crate::cargo::{self, Build, Kind};
use crate::icons;
use crate::project::{Profile, Project};

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
    let artifacts = cargo::build(&Build {
        project,
        profile,
        triple: None,
        kind: Kind::Bin,
        env: Vec::new(),
    })?;
    let executable = artifacts
        .executable
        .context("cargo produced no executable")?;
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
    let mut macos = MacOsConfig::default();
    macos.minimum_system_version = project.macos.minimum_system_version.clone();
    macos.signing_identity = std::env::var("APPLE_SIGNING_IDENTITY").ok();
    let mut config = Config::default();
    config.product_name = project.name.clone();
    config.version = project.version.clone();
    config.identifier = Some(project.identifier.clone());
    config.binaries = vec![binary];
    config.out_dir = out.to_path_buf();
    config.formats = Some(vec![format]);
    config.icons = (!icons.is_empty()).then_some(icons);
    config.resources = Some(resources);
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

/// PNGs at every size the icns and ico builders want, from the app's one icon.
fn icon_set(project: &Project, dir: &Path) -> Result<Vec<String>> {
    let Some(icon) = &project.icon else {
        return Ok(Vec::new());
    };
    let source = icons::load(icon)?;
    fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    for size in [16, 32, 128, 256, 512, 1024] {
        let path = dir.join(format!("{size}x{size}.png"));
        icons::write_resized(&source, size, &path)?;
        paths.push(path.to_string_lossy().into_owned());
    }
    Ok(paths)
}

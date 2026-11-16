//! What Windows and Linux share: the app's executable built for this machine, run here, put in a
//! folder with its resources, or packaged by cargo-packager, to whose configuration each system
//! adds its own (`windows/`, `linux/`). macOS's bundle is the CLI's own (`macos/`).
//!
//! The app's file associations go to cargo-packager for every package.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cargo_packager::config::{Binary, Resource};
use cargo_packager::{Config, PackageFormat};

use crate::cargo::{self, Artifact, Build};
use crate::file_associations;
use crate::icons;
use crate::project::{Profile, Project};

/// The app's executable for this machine, linked with `link_inputs`, built under `out`.
pub fn build_executable(
    project: &Project,
    profile: Profile,
    out: &Path,
    link_inputs: Vec<PathBuf>,
) -> Result<PathBuf> {
    fs::create_dir_all(out)?;
    cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: None,
        artifact: Artifact::Executable(&project.bin),
        env: Vec::new(),
        link_inputs,
    })
}

/// Runs `executable` here with `args`; the app's stdio is the terminal.
pub fn run_executable(project: &Project, executable: &Path, args: &[String]) -> Result<()> {
    let status = Command::new(executable)
        .args(args)
        .status()
        .with_context(|| format!("could not start {}", executable.display()))?;
    if !status.success() {
        bail!("{} exited with {status}", project.name);
    }
    Ok(())
}

/// The executable and the app's resources in a folder named after the app, in `out`.
pub fn folder(project: &Project, executable: &Path, out: &Path) -> Result<Vec<PathBuf>> {
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

/// The `format` package cargo-packager makes in `out` around `executable`, with the app's icons,
/// resources and file associations, and what `system` adds to its configuration.
pub fn package(
    project: &Project,
    executable: &Path,
    out: &Path,
    format: PackageFormat,
    system: impl FnOnce(&mut Config) -> Result<()>,
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
    system(&mut config)?;
    let outputs = cargo_packager::package(&config).context("packaging failed")?;
    Ok(outputs
        .into_iter()
        .flat_map(|output| output.paths)
        .collect())
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

//! Windows: the app's executable, carrying the app's icon as a resource (`resources.rs`), linked
//! in as it is built, since cargo-packager puts it only on the installer; in a folder, or in an MSI
//! or NSIS installer that cargo-packager makes, signed as the metadata says (`signing.rs`). `run`
//! builds the same executable, so it carries the icon too.

mod resources;
mod signing;

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_packager::PackageFormat;

use crate::icons;
use crate::packager;
use crate::project::{Profile, Project};
use signing::WindowsSigning;

/// What `build` makes for Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The executable and resources in a folder; no installer.
    Folder,
    Msi,
    Nsis,
}

impl Format {
    fn dir_name(self) -> &'static str {
        match self {
            Format::Folder => "windows",
            Format::Msi => "msi",
            Format::Nsis => "nsis",
        }
    }
}

pub fn build(project: &Project, profile: Profile, format: Format) -> Result<Vec<PathBuf>> {
    let out = project.out_dir(format.dir_name(), profile);
    let executable = executable(project, profile, &out)?;
    let package_format = match format {
        Format::Folder => return packager::folder(project, &executable, &out),
        Format::Msi => PackageFormat::Wix,
        Format::Nsis => PackageFormat::Nsis,
    };
    let signing = WindowsSigning::of(&project.windows)?;
    packager::package(project, &executable, &out, package_format, |config| {
        config.windows = signing.packager_config(project.windows.timestamp_url.as_deref());
        Ok(())
    })
}

/// The executable `build windows` makes, run here with `args`.
pub fn run(project: &Project, profile: Profile, args: &[String]) -> Result<()> {
    let out = project.out_dir(Format::Folder.dir_name(), profile);
    let executable = executable(project, profile, &out)?;
    packager::run_executable(project, &executable, args)
}

/// The app's executable, with the resources it carries written under `out`.
fn executable(project: &Project, profile: Profile, out: &Path) -> Result<PathBuf> {
    fs::create_dir_all(out)?;
    let resources = executable_resources(project, out)?;
    packager::build_executable(project, profile, out, resources.into_iter().collect())
}

/// The resources the executable carries, written under `out`: on Windows its icon, which the
/// shortcuts, the taskbar and the title bar show; none elsewhere, or for an app without an icon.
fn executable_resources(project: &Project, out: &Path) -> Result<Option<PathBuf>> {
    let Some(icon) = project
        .icon
        .as_ref()
        .filter(|_| cfg!(target_os = "windows"))
    else {
        return Ok(None);
    };
    let resources = resources::icon_resources(&icons::load(icon)?);
    write_resource_file(out, &resources).map(Some)
}

/// Writes a compiled resource file under `dir`, named after a hash of its content.
///
/// Cargo tracks the arguments it gives rustc, not the files they name: a file of the same name
/// with another icon in it would leave the executable with the old one until its crate compiled
/// for another reason. The hash may change with the Rust release the CLI is built with, which
/// costs one more link.
fn write_resource_file(dir: &Path, resources: &[u8]) -> Result<PathBuf> {
    let mut hasher = DefaultHasher::new();
    resources.hash(&mut hasher);
    let path = dir.join(format!("resources-{:016x}.res", hasher.finish()));
    fs::write(&path, resources).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn a_changed_icon_is_linked_from_a_file_of_another_name() {
        let dir = std::env::temp_dir().join(format!("waft-resources-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let written = |colour| {
            let icon = RgbaImage::from_pixel(32, 32, Rgba(colour));
            write_resource_file(&dir, &resources::icon_resources(&icon)).unwrap()
        };
        let blue = written([10, 132, 255, 255]);
        assert_eq!(written([10, 132, 255, 255]), blue, "the same icon");
        assert_ne!(written([255, 59, 48, 255]), blue);
        fs::remove_dir_all(&dir).unwrap();
    }
}

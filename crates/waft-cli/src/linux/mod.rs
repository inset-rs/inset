//! Linux: the app's executable in a folder, or in a deb or an AppImage that cargo-packager makes.
//! A package's desktop entry and the types the app owns are the CLI's own (`freedesktop.rs`),
//! which the package carries among its files in place of cargo-packager's entry.

mod freedesktop;

use std::path::{Path, PathBuf};

use anyhow::Result;
use cargo_packager::config::{AppImageConfig, DebianConfig, LinuxConfig};
use cargo_packager::{Config, PackageFormat};

use crate::packager;
use crate::project::{Profile, Project};

/// What `build` makes for Linux.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The executable and resources in a folder; no package.
    Folder,
    Deb,
    AppImage,
}

impl Format {
    fn dir_name(self) -> &'static str {
        match self {
            Format::Folder => "linux",
            Format::Deb => "deb",
            Format::AppImage => "appimage",
        }
    }
}

pub fn build(project: &Project, profile: Profile, format: Format) -> Result<Vec<PathBuf>> {
    let out = project.out_dir(format.dir_name(), profile);
    let executable = packager::build_executable(project, profile, &out, Vec::new())?;
    let package_format = match format {
        Format::Folder => return packager::folder(project, &executable, &out),
        Format::Deb => PackageFormat::Deb,
        Format::AppImage => PackageFormat::AppImage,
    };
    let entry_name = entry_name(project, format);
    packager::package(project, &executable, &out, package_format, |config| {
        add_freedesktop_files(config, project, &entry_name, &out)
    })
}

/// The executable `build linux` makes, run here with `args`.
pub fn run(project: &Project, profile: Profile, args: &[String]) -> Result<()> {
    let out = project.out_dir(Format::Folder.dir_name(), profile);
    let executable = packager::build_executable(project, profile, &out, Vec::new())?;
    packager::run_executable(project, &executable, args)
}

/// What a package's desktop entry is named: after the app's identifier in a deb, which the
/// desktop matches the app's windows to; after the executable in an AppImage, whose tool links the
/// entry by that name.
fn entry_name(project: &Project, format: Format) -> String {
    match format {
        Format::AppImage => format!("{}.desktop", project.bin),
        Format::Deb | Format::Folder => freedesktop::deb_entry_name(project),
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

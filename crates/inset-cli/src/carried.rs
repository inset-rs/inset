//! What a macOS bundle carries beside the app's executable: helper executables and libraries,
//! other targets of the app's workspace, built with the app.
//!
//! A helper goes in `Contents/Helpers`, as Chrome keeps its app shims' loader in its framework's
//! Helpers, and a library in `Contents/Frameworks`. The bundle signs each before itself
//! (`macos/`): a signature seals what is in the bundle when it is made.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_metadata::{Metadata, TargetKind};

use crate::cargo::{self, Artifact, Build};
use crate::project::{Helper, Library, MacosMetadata, Profile, Project};

/// A target the bundle carries, built: its file, and where it goes under `Contents`.
pub struct Carried {
    file: PathBuf,
    place: PathBuf,
}

/// Fails unless every helper names a binary target, and every library a `cdylib` target, of a
/// package of the app's workspace.
pub fn check(macos: &MacosMetadata, metadata: &Metadata) -> Result<()> {
    for helper in &macos.helpers {
        if !has_target(metadata, &helper.package, &helper.bin, TargetKind::Bin) {
            bail!(
                "helper `{}`: the workspace has no package `{}` with a binary of that name",
                helper.bin,
                helper.package
            );
        }
    }
    for library in &macos.libraries {
        if !has_target(metadata, &library.package, &library.lib, TargetKind::CDyLib) {
            bail!(
                "library `{}`: the workspace has no package `{}` whose `cdylib` has that name",
                library.lib,
                library.package
            );
        }
    }
    Ok(())
}

/// Builds every helper and library the app's bundle carries, as the app's executable is built:
/// with the same profile, for this machine.
pub fn build(project: &Project, profile: Profile) -> Result<Vec<Carried>> {
    let helpers = project
        .macos
        .helpers
        .iter()
        .map(|helper| build_helper(project, profile, helper));
    let libraries = project
        .macos
        .libraries
        .iter()
        .map(|library| build_library(project, profile, library));
    helpers.chain(libraries).collect()
}

/// Puts `carried` in the bundle's `contents`, each where it goes; answers where each went.
pub fn put_in(contents: &Path, carried: &[Carried]) -> Result<Vec<PathBuf>> {
    carried
        .iter()
        .map(|target| {
            let dest = contents.join(&target.place);
            fs::create_dir_all(dest.parent().context("a carried file has a folder")?)?;
            fs::copy(&target.file, &dest).with_context(|| {
                format!("could not copy {} into the bundle", target.file.display())
            })?;
            Ok(dest)
        })
        .collect()
}

fn build_helper(project: &Project, profile: Profile, helper: &Helper) -> Result<Carried> {
    let file = cargo::build(&Build {
        project,
        package: &helper.package,
        profile,
        triple: None,
        artifact: Artifact::Executable(&helper.bin),
        env: Vec::new(),
    })?;
    Ok(Carried {
        file,
        place: helper_place(&helper.bin),
    })
}

fn build_library(project: &Project, profile: Profile, library: &Library) -> Result<Carried> {
    let file = cargo::build(&Build {
        project,
        package: &library.package,
        profile,
        triple: None,
        artifact: Artifact::Library(&library.lib),
        env: Vec::new(),
    })?;
    Ok(Carried {
        file,
        place: library_place(&library.lib),
    })
}

/// Where helper `bin` goes under `Contents`.
fn helper_place(bin: &str) -> PathBuf {
    Path::new("Helpers").join(bin)
}

/// Where library `lib` goes under `Contents`: the name cargo gives a `cdylib` on macOS.
fn library_place(lib: &str) -> PathBuf {
    Path::new("Frameworks").join(format!("lib{lib}.dylib"))
}

/// Whether a package `package` of the workspace has a target `name` of `kind`.
fn has_target(metadata: &Metadata, package: &str, name: &str, kind: TargetKind) -> bool {
    metadata
        .packages
        .iter()
        .filter(|candidate| metadata.workspace_members.contains(&candidate.id))
        .filter(|candidate| candidate.name.as_str() == package)
        .flat_map(|candidate| &candidate.targets)
        .any(|target| target.name == name && target.kind.contains(&kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_goes_in_helpers_and_a_library_in_frameworks() {
        assert_eq!(
            helper_place("waos-program-loader"),
            Path::new("Helpers/waos-program-loader")
        );
        assert_eq!(
            library_place("waos_window"),
            Path::new("Frameworks/libwaos_window.dylib")
        );
    }
}

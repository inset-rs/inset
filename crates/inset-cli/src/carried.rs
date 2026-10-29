//! What a macOS bundle carries beside the app's executable: helper executables and libraries,
//! other targets of the app's workspace, built with the app.
//!
//! A helper goes in `Contents/Helpers`, as Chrome keeps its app shims' loader in its framework's
//! Helpers, and a library in `Contents/Frameworks`. cargo-packager has no place for a helper,
//! and signs a bundle as it makes it, before anything can be added to it, so the CLI puts both in
//! and signs the bundle itself, inside out: each carried file with the app's identity and the
//! hardened runtime, and no entitlements of its own, then the bundle with the app's entitlements.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cargo_metadata::{Metadata, TargetKind};

use crate::cargo::{self, Artifact, Build};
use crate::project::{Helper, Library, MacosMetadata, Profile, Project};
use crate::tools;

/// A target the bundle carries, built: its file, and where it goes under `Contents`.
pub struct Carried {
    file: PathBuf,
    place: PathBuf,
}

/// Whether the app's bundle carries anything beside its executable.
pub fn any(project: &Project) -> bool {
    !project.macos.helpers.is_empty() || !project.macos.libraries.is_empty()
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

/// Puts `carried` in the bundle `app` and, with an `identity`, signs it inside out: each carried
/// file with the hardened runtime and no entitlements, then the bundle with `entitlements`.
/// Without one nothing is signed, and each file keeps the ad hoc signature the linker gave it, as
/// the app's executable does.
pub fn put_in(
    app: &Path,
    carried: &[Carried],
    identity: Option<&str>,
    entitlements: Option<&Path>,
) -> Result<()> {
    let contents = app.join("Contents");
    for target in carried {
        let dest = contents.join(&target.place);
        fs::create_dir_all(dest.parent().context("a carried file has a folder")?)?;
        fs::copy(&target.file, &dest)
            .with_context(|| format!("could not copy {} into the bundle", target.file.display()))?;
        if let Some(identity) = identity {
            sign(&dest, identity, None)?;
        }
    }
    if let Some(identity) = identity {
        sign(app, identity, entitlements)?;
    }
    Ok(())
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

/// Signs `path` with `identity` and the hardened runtime, with `entitlements` or none, as
/// cargo-packager signs what it puts in a bundle.
fn sign(path: &Path, identity: &str, entitlements: Option<&Path>) -> Result<()> {
    let mut command = Command::new("codesign");
    command.args([
        "--force",
        "--sign",
        identity,
        "--options",
        "runtime",
        "--timestamp",
    ]);
    if let Some(entitlements) = entitlements {
        command.arg("--entitlements").arg(entitlements);
    }
    tools::run(command.arg(path), "codesign")
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

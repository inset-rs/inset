//! The app being built: cargo's view of the package plus `[package.metadata.inset]`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_metadata::{MetadataCommand, Package, TargetKind};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn dir_name(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }

    pub fn is_release(self) -> bool {
        self == Profile::Release
    }
}

/// `[package.metadata.inset]` as written in Cargo.toml. Every key is optional.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Metadata {
    /// Reverse-DNS bundle identifier, `com.example.myapp`.
    pub identifier: Option<String>,
    /// Display name. Defaults to the package name with the first letter upper-cased.
    pub name: Option<String>,
    /// One square PNG, 1024 px. Every platform's icon sizes derive from it.
    pub icon: Option<PathBuf>,
    /// Globs relative to the package root, copied into the bundle at the same relative path.
    pub resources: Vec<String>,
    pub macos: MacosMetadata,
    pub ios: IosMetadata,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct MacosMetadata {
    pub minimum_system_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct IosMetadata {
    /// Apple team id. Narrows the signing identity and profile search when a machine has several.
    pub team: Option<String>,
    /// `MinimumOSVersion`. Defaults to 15.0.
    pub minimum_version: Option<String>,
}

pub struct Project {
    /// Directory holding the package's Cargo.toml.
    pub root: PathBuf,
    pub workspace_root: PathBuf,
    pub target_dir: PathBuf,
    /// Cargo package name.
    pub package: String,
    /// Name of the binary target.
    pub bin: String,
    pub has_cdylib: bool,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub authors: Vec<String>,
    pub name: String,
    pub identifier: String,
    pub icon: Option<PathBuf>,
    pub resources: Vec<String>,
    pub macos: MacosMetadata,
    pub ios: IosMetadata,
}

impl Project {
    pub fn load(package: Option<&str>) -> Result<Project> {
        let metadata = MetadataCommand::new()
            .exec()
            .context("cargo metadata failed; run inside a cargo project")?;
        let package = match package {
            Some(name) => metadata
                .packages
                .iter()
                .find(|p| p.name.as_str() == name && metadata.workspace_members.contains(&p.id))
                .with_context(|| format!("no workspace package named `{name}`"))?,
            None => metadata
                .root_package()
                .context("no root package here; pass `-p <package>` from the workspace root")?,
        };
        let inset: Metadata = match package.metadata.get("inset") {
            Some(value) => serde_json::from_value(value.clone())
                .context("[package.metadata.inset] in Cargo.toml has an unexpected shape")?,
            None => Metadata::default(),
        };
        let bin = package
            .targets
            .iter()
            .find(|t| t.kind.contains(&TargetKind::Bin))
            .map(|t| t.name.clone())
            .with_context(|| format!("package `{}` has no binary target", package.name))?;
        let has_cdylib = package
            .targets
            .iter()
            .any(|t| t.kind.contains(&TargetKind::CDyLib));
        let root = manifest_dir(package).to_path_buf();
        let identifier = match inset.identifier {
            Some(identifier) => identifier,
            None => {
                let identifier = format!("com.example.{}", package.name.replace('_', "-"));
                eprintln!(
                    "note: no `identifier` under [package.metadata.inset]; using {identifier}"
                );
                identifier
            }
        };
        let icon = inset
            .icon
            .map(|icon| root.join(icon))
            .or_else(|| Some(root.join("assets/icon.png")).filter(|p| p.exists()));
        if let Some(icon) = &icon
            && !icon.exists()
        {
            bail!("icon {} does not exist", icon.display());
        }
        Ok(Project {
            workspace_root: metadata.workspace_root.clone().into_std_path_buf(),
            target_dir: metadata.target_directory.clone().into_std_path_buf(),
            package: package.name.to_string(),
            bin,
            has_cdylib,
            version: package.version.to_string(),
            description: package.description.clone(),
            homepage: package.homepage.clone(),
            authors: package.authors.clone(),
            name: inset.name.unwrap_or_else(|| display_name(&package.name)),
            identifier,
            icon,
            resources: inset.resources,
            macos: inset.macos,
            ios: inset.ios,
            root,
        })
    }

    /// `target/inset/<target>/<profile>`.
    pub fn out_dir(&self, target: &str, profile: Profile) -> PathBuf {
        self.target_dir
            .join("inset")
            .join(target)
            .join(profile.dir_name())
    }

    /// Every file the resource globs match, as (absolute source, path inside the bundle).
    pub fn resource_files(&self) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut files = Vec::new();
        for pattern in &self.resources {
            let absolute = self.root.join(pattern);
            let matches = glob::glob(&absolute.to_string_lossy())
                .with_context(|| format!("bad resource glob `{pattern}`"))?;
            for entry in matches {
                let source = entry?;
                if source.is_file() {
                    let relative = source.strip_prefix(&self.root)?.to_path_buf();
                    files.push((source, relative));
                }
            }
        }
        Ok(files)
    }
}

fn manifest_dir(package: &Package) -> &Path {
    package
        .manifest_path
        .parent()
        .map(|p| p.as_std_path())
        .unwrap_or_else(|| Path::new("."))
}

/// `my-app` → `My app`.
pub fn display_name(package: &str) -> String {
    let spaced = package.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_spaces_and_capitalizes() {
        assert_eq!(display_name("my-app"), "My app");
        assert_eq!(display_name("counter"), "Counter");
    }

    #[test]
    fn metadata_defaults_when_keys_are_missing() {
        let metadata: Metadata =
            serde_json::from_value(serde_json::json!({ "name": "X" })).unwrap();
        assert_eq!(metadata.name.as_deref(), Some("X"));
        assert!(metadata.identifier.is_none());
        assert!(metadata.resources.is_empty());
        assert!(metadata.ios.team.is_none());
    }
}

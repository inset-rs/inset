//! The app being built: cargo's view of the package plus `[package.metadata.inset]`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_metadata::{MetadataCommand, Package, TargetKind};
use serde::Deserialize;

use crate::file_associations;

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
    /// The kinds of file the app opens, which the system then offers it for.
    pub file_associations: Vec<FileAssociation>,
    pub macos: MacosMetadata,
    pub ios: IosMetadata,
    pub android: AndroidMetadata,
}

/// A kind of file the app opens: one `[[package.metadata.inset.file_associations]]` table.
#[derive(Clone, Debug, Deserialize)]
pub struct FileAssociation {
    /// File name extensions, without the dot.
    pub extensions: Vec<String>,
    /// What the system calls a file of this type: the Finder's Kind, Explorer's Type.
    pub name: String,
    /// Whether the app defines this type. An owner declares the type to the system and is
    /// the app the system opens such a file with first; otherwise the type is someone
    /// else's, and the app is one of those that open it.
    #[serde(default)]
    pub owner: bool,
    /// What the app does with such a file.
    #[serde(default)]
    pub role: FileRole,
    /// The type's identifier on macOS, `org.webassembly.wasm`. An owner's defaults to the
    /// app's identifier and the first extension; a type the app only opens has none unless
    /// given, and is then matched by its extensions.
    pub identifier: Option<String>,
    /// The type's MIME type, `application/wasm`. Linux associates files by it, so there a
    /// type without one is not associated; macOS adds it to the type's declaration.
    pub mime_type: Option<String>,
}

/// What an app does with a file of a type it opens: macOS's `CFBundleTypeRole`.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    /// Reads and saves it.
    Editor,
    /// Reads it only.
    #[default]
    Viewer,
    /// Runs it, as a shell runs a script.
    Shell,
    /// Claims no use of it, for a type declared only so the system knows it.
    None,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct MacosMetadata {
    pub minimum_system_version: Option<String>,
    /// `LSUIElement`: no Dock tile and no menu bar, for agents that live in a panel.
    pub background_app: bool,
    /// Apple team id. Narrows the signing identity when a machine has several.
    pub team: Option<String>,
    /// An `.entitlements` plist, relative to the package root, that the bundle is signed with:
    /// what the hardened runtime otherwise denies, such as memory a JIT makes executable.
    pub entitlements: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct IosMetadata {
    /// Apple team id. Narrows the signing identity and profile search when a machine has several.
    pub team: Option<String>,
    /// `MinimumOSVersion`. Defaults to 15.0.
    pub minimum_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AndroidMetadata {
    /// `minSdkVersion`. Defaults to 26, Android 8.0.
    pub min_sdk: Option<u32>,
    /// `targetSdkVersion`. Defaults to 34, the last level before the system stops fitting the
    /// window to its bars, which is how the host learns the safe area.
    pub target_sdk: Option<u32>,
    /// `versionCode`. Defaults to one derived from the package version.
    pub version_code: Option<u32>,
    /// `android:appCategory`: `accessibility`, `audio`, `game`, `image`, `maps`, `news`,
    /// `productivity`, `social` or `video`. Defaults to `productivity`.
    pub category: Option<String>,
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
    /// Name of the `cdylib` target, which the browser and Android load; `None` when the
    /// package has none.
    pub library: Option<String>,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub authors: Vec<String>,
    pub name: String,
    pub identifier: String,
    pub icon: Option<PathBuf>,
    pub resources: Vec<String>,
    pub file_associations: Vec<FileAssociation>,
    pub macos: MacosMetadata,
    pub ios: IosMetadata,
    pub android: AndroidMetadata,
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
        let library = package
            .targets
            .iter()
            .find(|t| t.kind.contains(&TargetKind::CDyLib))
            .map(|t| t.name.clone());
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
        let icon = match inset.icon {
            Some(icon) => Some(existing_file(&root, icon, "icon")?),
            None => Some(root.join("assets/icon.png")).filter(|p| p.exists()),
        };
        file_associations::check(&inset.file_associations)?;
        let mut macos = inset.macos;
        macos.entitlements = macos
            .entitlements
            .map(|entitlements| existing_file(&root, entitlements, "entitlements"))
            .transpose()?;
        Ok(Project {
            workspace_root: metadata.workspace_root.clone().into_std_path_buf(),
            target_dir: metadata.target_directory.clone().into_std_path_buf(),
            package: package.name.to_string(),
            bin,
            library,
            version: package.version.to_string(),
            description: package.description.clone(),
            homepage: package.homepage.clone(),
            authors: package.authors.clone(),
            name: inset.name.unwrap_or_else(|| display_name(&package.name)),
            identifier,
            icon,
            resources: inset.resources,
            file_associations: inset.file_associations,
            macos,
            ios: inset.ios,
            android: inset.android,
            root,
        })
    }

    /// The identifier as an Android package name, which is a Java package name: the
    /// reverse-DNS segments with what Java does not allow in a name replaced by `_`, and a
    /// segment that starts with a digit led by one.
    pub fn android_package(&self) -> String {
        android_package(&self.identifier)
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

/// `relative` under the package `root`, which must exist; `key` names it in the error.
fn existing_file(root: &Path, relative: PathBuf, key: &str) -> Result<PathBuf> {
    let path = root.join(relative);
    if !path.exists() {
        bail!("{key} {} does not exist", path.display());
    }
    Ok(path)
}

fn android_package(identifier: &str) -> String {
    let segments: Vec<String> = identifier
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut name: String = segment
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            if name.starts_with(|c: char| c.is_ascii_digit()) {
                name.insert(0, '_');
            }
            name
        })
        .collect();
    match segments.len() {
        0 => "app.main".to_owned(),
        1 => format!("app.{}", segments[0]),
        _ => segments.join("."),
    }
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
    fn android_package_is_a_java_package_name() {
        assert_eq!(android_package("com.example.my-app"), "com.example.my_app");
        assert_eq!(
            android_package("rs.inset.cupertino-gallery"),
            "rs.inset.cupertino_gallery"
        );
        assert_eq!(android_package("io.3d.viewer"), "io._3d.viewer");
        assert_eq!(android_package("myapp"), "app.myapp");
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

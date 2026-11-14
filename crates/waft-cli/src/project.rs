//! The app being built: cargo's view of the package plus `[package.metadata.waft]`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_metadata::{MetadataCommand, Package, TargetKind};
use serde::Deserialize;

use crate::carried;
use crate::file_associations;
use crate::usage_descriptions;

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

/// `[package.metadata.waft]` as written in Cargo.toml. Every key is optional.
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
    /// Why the app uses each device the system asks the user about first.
    pub usage_descriptions: UsageDescriptions,
    pub macos: MacosMetadata,
    pub ios: IosMetadata,
    pub android: AndroidMetadata,
}

/// A kind of file the app opens: one `[[package.metadata.waft.file_associations]]` table.
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
    /// type without one is not associated; macOS adds it to the type's declaration; Android
    /// offers the app for files of it, beside those its extensions end.
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

/// `[package.metadata.waft.usage_descriptions]`: why the app uses each device the system asks
/// the user about before an app may open it, in the sentence the system's prompt shows. macOS and
/// iOS end an app that opens one of these without its description.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct UsageDescriptions {
    /// `NSCameraUsageDescription`.
    pub camera: Option<String>,
    /// `NSMicrophoneUsageDescription`.
    pub microphone: Option<String>,
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
    /// Executables of the app's workspace the bundle carries in `Contents/Helpers`, built with
    /// the app.
    pub helpers: Vec<Helper>,
    /// Libraries of the app's workspace the bundle carries in `Contents/Frameworks`, built with
    /// the app.
    pub libraries: Vec<Library>,
}

/// A binary target of a package of the app's workspace, which the macOS bundle carries.
#[derive(Clone, Debug, Deserialize)]
pub struct Helper {
    pub package: String,
    pub bin: String,
}

/// A `cdylib` target of a package of the app's workspace, which the macOS bundle carries: `lib`
/// is the target's name, as `[lib] name` gives it.
#[derive(Clone, Debug, Deserialize)]
pub struct Library {
    pub package: String,
    pub lib: String,
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
    /// `minSdkVersion`. Defaults to 31, Android 12, the least Waft's Android host runs on, and
    /// refused below it.
    pub min_sdk: Option<u32>,
    /// `targetSdkVersion`. Defaults to 34, the last level before the system lays every app out
    /// edge to edge; a later one lays a Waft app out the same, as it draws its whole window and is
    /// told what the bars cover at every level.
    pub target_sdk: Option<u32>,
    /// `versionCode`. Defaults to one derived from the package version.
    pub version_code: Option<u32>,
    /// `android:appCategory`: `accessibility`, `audio`, `game`, `image`, `maps`, `news`,
    /// `productivity`, `social` or `video`. Defaults to `productivity`.
    pub category: Option<String>,
    /// The permissions the app asks for besides those Waft's own classes need, each by its full
    /// name, `android.permission.INTERNET`: one `<uses-permission>` each. Android refuses what a
    /// manifest does not name, so an app that opens a socket without `INTERNET` gets "operation
    /// not permitted".
    pub permissions: Vec<String>,
    /// The launcher icon as Android draws it since API 26, `<adaptive-icon>`: two layers the
    /// launcher cuts to its own shape. Without it, the app's one `icon` is the launcher icon, which
    /// a launcher shrinks into its shape.
    pub adaptive_icon: Option<AdaptiveIcon>,
    /// Android libraries merged into the package, as Gradle merges a module's dependencies: each an
    /// `.aar` or a `.jar`, relative to the package root, in priority order, the first winning a
    /// resource both define. An app lists the libraries each of these needs too.
    pub dependencies: Vec<PathBuf>,
}

/// Android's adaptive launcher icon: a foreground over a background, each 108 dp square, which the
/// launcher masks to its shape (a circle, a rounded square) and may move apart as it animates; what
/// lies in the middle 72 dp is always shown, and up to 18 dp on each side may be cut.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveIcon {
    /// `<foreground>`: a square PNG with transparency, relative to the package root, its whole
    /// side the layer's 108 dp: 432 px is its size at the largest density.
    pub foreground: PathBuf,
    /// `<background>`: an opaque colour, `#rrggbb`, or a square PNG relative to the package root,
    /// as the foreground is.
    pub background: AdaptiveIconBackground,
}

/// What an adaptive icon's background layer is.
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub enum AdaptiveIconBackground {
    /// `#rrggbb`.
    Color(String),
    Image(PathBuf),
}

impl TryFrom<String> for AdaptiveIconBackground {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some(hex) = value.strip_prefix('#') else {
            return Ok(AdaptiveIconBackground::Image(PathBuf::from(value)));
        };
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(AdaptiveIconBackground::Color(value))
        } else {
            Err(format!(
                "adaptive_icon's background `{value}` is neither `#rrggbb` nor a path to a PNG"
            ))
        }
    }
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
    pub usage_descriptions: UsageDescriptions,
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
        let waft: Metadata = match package.metadata.get("waft") {
            Some(value) => serde_json::from_value(value.clone())
                .context("[package.metadata.waft] in Cargo.toml has an unexpected shape")?,
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
        let identifier = match waft.identifier {
            Some(identifier) => identifier,
            None => {
                let identifier = format!("com.example.{}", package.name.replace('_', "-"));
                eprintln!(
                    "note: no `identifier` under [package.metadata.waft]; using {identifier}"
                );
                identifier
            }
        };
        let icon = match waft.icon {
            Some(icon) => Some(existing_file(&root, icon, "icon")?),
            None => Some(root.join("assets/icon.png")).filter(|p| p.exists()),
        };
        file_associations::check(&waft.file_associations)?;
        usage_descriptions::check(&waft.usage_descriptions)?;
        carried::check(&waft.macos, &metadata)?;
        let mut macos = waft.macos;
        macos.entitlements = macos
            .entitlements
            .map(|entitlements| existing_file(&root, entitlements, "entitlements"))
            .transpose()?;
        let mut android = waft.android;
        android.adaptive_icon = android
            .adaptive_icon
            .map(|icon| icon.under(&root))
            .transpose()?;
        android.dependencies = android
            .dependencies
            .into_iter()
            .map(|library| existing_file(&root, library, "the Android library"))
            .collect::<Result<_>>()?;
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
            name: waft.name.unwrap_or_else(|| display_name(&package.name)),
            identifier,
            icon,
            resources: waft.resources,
            file_associations: waft.file_associations,
            usage_descriptions: waft.usage_descriptions,
            macos,
            ios: waft.ios,
            android,
            root,
        })
    }

    /// The identifier as an Android package name, which is a Java package name: the
    /// reverse-DNS segments with what Java does not allow in a name replaced by `_`, and a
    /// segment that starts with a digit led by one.
    pub fn android_package(&self) -> String {
        android_package(&self.identifier)
    }

    /// `target/waft/<target>/<profile>`.
    pub fn out_dir(&self, target: &str, profile: Profile) -> PathBuf {
        self.target_dir
            .join("waft")
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

impl AdaptiveIcon {
    /// The icon with its images' paths under the package `root`, each of which must exist.
    fn under(self, root: &Path) -> Result<AdaptiveIcon> {
        let background =
            match self.background {
                AdaptiveIconBackground::Image(image) => AdaptiveIconBackground::Image(
                    existing_file(root, image, "adaptive_icon's background")?,
                ),
                color => color,
            };
        Ok(AdaptiveIcon {
            foreground: existing_file(root, self.foreground, "adaptive_icon's foreground")?,
            background,
        })
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
    fn an_adaptive_icons_background_is_a_colour_or_an_image() {
        let background = |value: &str| AdaptiveIconBackground::try_from(value.to_owned());
        assert!(
            matches!(background("#1f5fae"), Ok(AdaptiveIconBackground::Color(color)) if color == "#1f5fae")
        );
        assert!(
            matches!(background("assets/back.png"), Ok(AdaptiveIconBackground::Image(path)) if path == Path::new("assets/back.png"))
        );
        assert!(background("#fff").is_err(), "only #rrggbb");
        assert!(background("#12345g").is_err());
    }

    #[test]
    fn display_name_spaces_and_capitalizes() {
        assert_eq!(display_name("my-app"), "My app");
        assert_eq!(display_name("counter"), "Counter");
    }

    #[test]
    fn android_package_is_a_java_package_name() {
        assert_eq!(android_package("com.example.my-app"), "com.example.my_app");
        assert_eq!(
            android_package("rs.waft.cupertino-gallery"),
            "rs.waft.cupertino_gallery"
        );
        assert_eq!(android_package("io.3d.viewer"), "io._3d.viewer");
        assert_eq!(android_package("myapp"), "app.myapp");
    }

    #[test]
    fn a_macos_bundle_names_the_helpers_and_libraries_it_carries() {
        let metadata: Metadata = serde_json::from_value(serde_json::json!({
            "macos": {
                "helpers": [{ "package": "waos-window", "bin": "waos-program-loader" }],
                "libraries": [{ "package": "waos-window", "lib": "waos_window" }],
            }
        }))
        .unwrap();
        let helper = &metadata.macos.helpers[0];
        assert_eq!(
            (helper.package.as_str(), helper.bin.as_str()),
            ("waos-window", "waos-program-loader")
        );
        let library = &metadata.macos.libraries[0];
        assert_eq!(
            (library.package.as_str(), library.lib.as_str()),
            ("waos-window", "waos_window")
        );
    }

    #[test]
    fn metadata_defaults_when_keys_are_missing() {
        let metadata: Metadata =
            serde_json::from_value(serde_json::json!({ "name": "X" })).unwrap();
        assert_eq!(metadata.name.as_deref(), Some("X"));
        assert!(metadata.identifier.is_none());
        assert!(metadata.resources.is_empty());
        assert!(metadata.ios.team.is_none());
        assert!(metadata.macos.helpers.is_empty() && metadata.macos.libraries.is_empty());
        assert!(metadata.android.dependencies.is_empty());
    }
}

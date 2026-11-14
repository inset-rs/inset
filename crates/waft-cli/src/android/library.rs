//! An Android library the app depends on, an `.aar` or a `.jar`, unpacked and sorted into what the
//! package build merges: classes, a manifest, resources and native libraries.
//!
//! An AAR is laid out as "Create an Android library" on developer.android.com describes it. Every
//! entry is merged, ignored for a stated reason, or refused: the build must never leave out of the
//! package something a library brings without saying so ([`AarEntry`]). A JAR brings classes, and
//! the Java resources beside them, alone.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::xml::{self, Element};

/// One library, unpacked.
pub struct AndroidLibrary {
    /// The library's path as the app lists it, relative to the package root, which every
    /// refusal names.
    pub name: String,
    /// The JARs whose classes, and whose Java resources, the package carries.
    pub class_jars: Vec<PathBuf>,
    /// An AAR's manifest, with the package it names.
    pub manifest: Option<LibraryManifest>,
    /// An AAR's `res/`.
    pub resources: Option<PathBuf>,
    /// An AAR's `jni/<abi>/lib<name>.so`.
    pub native_libraries: Vec<NativeLibrary>,
}

pub struct LibraryManifest {
    pub element: Element,
    /// The `package` it names: its R class's, and the one its relative class names are under.
    pub package: String,
}

pub struct NativeLibrary {
    /// `arm64-v8a`, as `jni/` names the folder.
    pub abi: String,
    /// `libfoo.so`.
    pub file_name: String,
    pub path: PathBuf,
}

/// The version of the AAR format, and of its metadata file, this reads: what Android's Gradle
/// plugin writes and checks for.
const AAR_FORMAT_MAJOR: u32 = 1;
const AAR_METADATA: &str = "META-INF/com/android/build/gradle/aar-metadata.properties";

/// Each of `dependencies`, paths under the package `root`, unpacked under `into`. `compile_sdk` is
/// the platform the package is linked against, which an AAR may require a least of.
pub fn unpack_all(
    root: &Path,
    dependencies: &[PathBuf],
    into: &Path,
    compile_sdk: u32,
) -> Result<Vec<AndroidLibrary>> {
    let mut libraries: Vec<AndroidLibrary> = Vec::new();
    for (index, dependency) in dependencies.iter().enumerate() {
        let name = dependency
            .strip_prefix(root)
            .unwrap_or(dependency)
            .display()
            .to_string();
        if libraries.iter().any(|library| library.name == name) {
            bail!("{name} is listed twice under [package.metadata.waft.android] dependencies");
        }
        let library = unpack(
            &name,
            dependency,
            &into.join(index.to_string()),
            compile_sdk,
        )
        .with_context(|| format!("{name}: the Android library cannot be merged"))?;
        libraries.push(library);
    }
    refuse_shared_packages(&libraries)?;
    Ok(libraries)
}

/// Refuses two AARs of one package: their R classes and relative names would be one.
fn refuse_shared_packages(libraries: &[AndroidLibrary]) -> Result<()> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for library in libraries {
        let Some(manifest) = &library.manifest else {
            continue;
        };
        if let Some(first) = seen.insert(&manifest.package, &library.name) {
            bail!(
                "{} and {first} are both of the package {}: two libraries' R classes would be one class",
                library.name,
                manifest.package
            );
        }
    }
    Ok(())
}

fn unpack(name: &str, path: &Path, into: &Path, compile_sdk: u32) -> Result<AndroidLibrary> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("jar") => Ok(AndroidLibrary {
            name: name.to_owned(),
            class_jars: vec![path.to_path_buf()],
            manifest: None,
            resources: None,
            native_libraries: Vec::new(),
        }),
        Some("aar") => unpack_aar(name, path, into, compile_sdk),
        _ => bail!("an Android library is an .aar or a .jar"),
    }
}

/// Where an entry of an AAR goes.
#[derive(Debug, PartialEq, Eq)]
enum AarEntry {
    Manifest,
    /// `classes.jar` or `libs/<name>.jar`.
    ClassJar,
    /// Under `res/`.
    Resource,
    /// `jni/<abi>/<name>.so`.
    NativeLibrary {
        abi: String,
        file_name: String,
    },
    /// The plugin's metadata, which says what the library needs of the build.
    Metadata,
    /// Read by nothing in a package built without Gradle, for the reason its arm of [`classify`]
    /// gives.
    Ignored,
    /// Part of the AAR format that is not merged yet, or not part of it at all.
    Refused(&'static str),
}

/// Where the AAR entry `name` goes.
fn classify(name: &str) -> AarEntry {
    let parts: Vec<&str> = name.split('/').collect();
    match parts.as_slice() {
        ["AndroidManifest.xml"] => AarEntry::Manifest,
        ["classes.jar"] => AarEntry::ClassJar,
        ["libs", jar] if jar.ends_with(".jar") => AarEntry::ClassJar,
        ["res", ..] => AarEntry::Resource,
        ["jni", abi, file_name] if file_name.ends_with(".so") => AarEntry::NativeLibrary {
            abi: (*abi).to_owned(),
            file_name: (*file_name).to_owned(),
        },
        _ if name == AAR_METADATA => AarEntry::Metadata,
        // The R class is generated from the linked resources.
        ["R.txt"] | ["public.txt"] => AarEntry::Ignored,
        // Nothing shrinks the package's classes.
        ["proguard.txt"] => AarEntry::Ignored,
        // Lint reads them.
        ["lint.jar"] | ["annotations.zip"] => AarEntry::Ignored,
        // The app compiles no Java against them.
        ["api.jar"] | ["aidl", ..] => AarEntry::Ignored,
        // The app builds no C or C++ against it, and Gradle packages none of it unless one does.
        ["prefab", ..] => AarEntry::Ignored,
        // A profile only speeds the app's start, and only Gradle's packaging compiles one.
        ["baseline-prof.txt"] => AarEntry::Ignored,
        ["assets", ..] => AarEntry::Refused("a library's assets are not merged yet"),
        ["navigation.json"] => AarEntry::Refused("navigation graphs are not merged yet"),
        _ => AarEntry::Refused("it is not part of the AAR format"),
    }
}

fn unpack_aar(name: &str, path: &Path, into: &Path, compile_sdk: u32) -> Result<AndroidLibrary> {
    let file = fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("it is not a ZIP archive")?;
    let mut library = AndroidLibrary {
        name: name.to_owned(),
        class_jars: Vec::new(),
        manifest: None,
        resources: None,
        native_libraries: Vec::new(),
    };
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry
            .enclosed_name()
            .with_context(|| format!("its entry {} leaves the archive", entry.name()))?
            .to_string_lossy()
            .replace('\\', "/");
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        let unpacked = into.join(&entry_name);
        match classify(&entry_name) {
            AarEntry::Manifest => {
                let text = String::from_utf8(contents).context("its manifest is not UTF-8")?;
                library.manifest = Some(read_manifest(&text)?);
                continue;
            }
            AarEntry::Metadata => {
                check_metadata(&String::from_utf8_lossy(&contents), compile_sdk)?;
                continue;
            }
            AarEntry::Ignored => continue,
            AarEntry::Refused(why) => bail!("it holds {entry_name}, which is refused: {why}"),
            AarEntry::ClassJar => library.class_jars.push(unpacked.clone()),
            AarEntry::Resource => library.resources = Some(into.join("res")),
            AarEntry::NativeLibrary { abi, file_name } => {
                library.native_libraries.push(NativeLibrary {
                    abi,
                    file_name,
                    path: unpacked.clone(),
                })
            }
        }
        fs::create_dir_all(unpacked.parent().unwrap_or(into))?;
        fs::write(&unpacked, contents)?;
    }
    if library.manifest.is_none() {
        bail!("it has no AndroidManifest.xml, the one entry every AAR has");
    }
    library.class_jars.sort();
    Ok(library)
}

fn read_manifest(text: &str) -> Result<LibraryManifest> {
    let element = xml::parse(text).context("its manifest does not parse")?;
    let package = super::manifest_merger::package(&element)?.to_owned();
    Ok(LibraryManifest { element, package })
}

/// Refuses an AAR whose metadata asks for what this build does not do, as Android's Gradle plugin
/// checks it: a newer AAR format, a later platform to compile against than `compile_sdk`, or
/// core library desugaring.
fn check_metadata(properties: &str, compile_sdk: u32) -> Result<()> {
    let properties: HashMap<&str, &str> = properties
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect();
    for key in ["aarFormatVersion", "aarMetadataVersion"] {
        let major = properties
            .get(key)
            .and_then(|version| version.split('.').next())
            .and_then(|major| major.parse::<u32>().ok());
        if major.is_some_and(|major| major > AAR_FORMAT_MAJOR) {
            bail!(
                "its {key} is {}, newer than the {AAR_FORMAT_MAJOR}.x this reads",
                properties[key]
            );
        }
    }
    if let Some(least) = properties
        .get("minCompileSdk")
        .and_then(|level| level.parse::<u32>().ok())
        && least > compile_sdk
    {
        bail!(
            "it needs the package compiled against API {least} or later, and the newest platform the SDK has is API {compile_sdk}: install a later one with the SDK Manager"
        );
    }
    if properties.get("coreLibraryDesugaringEnabled") == Some(&"true") {
        bail!("it needs core library desugaring, which this build does not do");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    const MANIFEST: &str = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.example.lib" />"#;

    fn aar(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join("lib.aar");
        let mut writer = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for (name, contents) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("waft-library-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn an_aars_entries_are_merged_ignored_or_refused() {
        assert_eq!(classify("AndroidManifest.xml"), AarEntry::Manifest);
        assert_eq!(classify("libs/okio.jar"), AarEntry::ClassJar);
        assert_eq!(classify("res/xml/config.xml"), AarEntry::Resource);
        assert_eq!(
            classify("jni/arm64-v8a/libfoo.so"),
            AarEntry::NativeLibrary {
                abi: "arm64-v8a".into(),
                file_name: "libfoo.so".into()
            }
        );
        assert_eq!(classify(AAR_METADATA), AarEntry::Metadata);
        assert!(classify("proguard.txt") == AarEntry::Ignored);
        assert!(classify("R.txt") == AarEntry::Ignored);
        assert!(matches!(classify("assets/a.bin"), AarEntry::Refused(_)));
        assert!(matches!(
            classify("jni/arm64-v8a/notes.txt"),
            AarEntry::Refused(_)
        ));
        assert!(matches!(classify("surprise.dat"), AarEntry::Refused(_)));
    }

    #[test]
    fn an_aar_unpacks_into_its_parts() {
        let dir = scratch("parts");
        let path = aar(
            &dir,
            &[
                ("AndroidManifest.xml", MANIFEST.as_bytes()),
                ("classes.jar", b"jar"),
                ("res/values/strings.xml", b"<resources />"),
                ("jni/arm64-v8a/libfoo.so", b"elf"),
                ("proguard.txt", b"-keep class *"),
            ],
        );
        let library = unpack("android/lib.aar", &path, &dir.join("out"), 36).unwrap();
        assert_eq!(library.manifest.unwrap().package, "com.example.lib");
        assert_eq!(library.class_jars, [dir.join("out/classes.jar")]);
        assert_eq!(library.resources, Some(dir.join("out/res")));
        assert!(dir.join("out/res/values/strings.xml").is_file());
        assert_eq!(library.native_libraries[0].abi, "arm64-v8a");
        assert_eq!(library.native_libraries[0].file_name, "libfoo.so");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_aar_with_assets_or_no_manifest_is_refused() {
        let dir = scratch("refused");
        let assets = aar(
            &dir,
            &[
                ("AndroidManifest.xml", MANIFEST.as_bytes()),
                ("assets/data.bin", b"x"),
            ],
        );
        let refused = unpack("android/lib.aar", &assets, &dir.join("out"), 36)
            .err()
            .unwrap();
        assert!(refused.to_string().contains("assets/data.bin"), "{refused}");
        let bare = aar(&dir, &[("classes.jar", b"jar")]);
        assert!(
            unpack("android/lib.aar", &bare, &dir.join("out2"), 36)
                .err()
                .unwrap()
                .to_string()
                .contains("AndroidManifest.xml")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_aars_metadata_is_checked_as_gradle_checks_it() {
        assert!(
            check_metadata(
                "aarFormatVersion=1.0\naarMetadataVersion=1.0\nminCompileSdk=34\n",
                36
            )
            .is_ok()
        );
        assert!(
            check_metadata("minCompileSdk=37\n", 36)
                .unwrap_err()
                .to_string()
                .contains("API 37")
        );
        assert!(check_metadata("coreLibraryDesugaringEnabled=true\n", 36).is_err());
        assert!(check_metadata("aarFormatVersion=2.0\n", 36).is_err());
    }

    #[test]
    fn two_libraries_of_one_package_or_one_listed_twice_are_refused() {
        let dir = scratch("shared");
        let first = aar(
            &dir.join("a"),
            &[("AndroidManifest.xml", MANIFEST.as_bytes())],
        );
        let second = aar(
            &dir.join("b"),
            &[("AndroidManifest.xml", MANIFEST.as_bytes())],
        );
        let shared = unpack_all(&dir, &[first.clone(), second], &dir.join("out"), 36);
        assert!(
            shared
                .err()
                .unwrap()
                .to_string()
                .contains("com.example.lib")
        );
        let twice = unpack_all(&dir, &[first.clone(), first], &dir.join("out2"), 36);
        assert!(twice.err().unwrap().to_string().contains("listed twice"));
        fs::remove_dir_all(&dir).unwrap();
    }
}

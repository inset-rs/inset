//! The Java resources of the libraries' JARs, the files beside their classes, gathered for the
//! package's root as Android's Gradle plugin merges them: what `Class.getResource` and
//! `ServiceLoader` read on the device.
//!
//! The plugin's default exclusions and merges apply ([`EXCLUDED`] and [`CONCATENATED`], from its
//! `PackagingOptionsUtils.kt`): an excluded path is left out, the JARs' files at a concatenated path
//! are joined into one, and any other path two JARs both have is refused, as the plugin refuses it
//! ("2 files found with path"), as is a path the package itself holds.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use glob::{MatchOptions, Pattern};

use super::library::AndroidLibrary;

/// The paths a JAR's Java resources are left out at by default, relative to its root.
const EXCLUDED: [&str; 32] = [
    "META-INF/LICENSE",
    "META-INF/LICENSE.txt",
    "META-INF/MANIFEST.MF",
    "META-INF/**/MANIFEST.MF",
    "META-INF/NOTICE",
    "META-INF/NOTICE.txt",
    "META-INF/*.DSA",
    "META-INF/*.EC",
    "META-INF/*.SF",
    "META-INF/*.RSA",
    "META-INF/maven/**",
    "META-INF/proguard/*",
    "META-INF/com.android.tools/**",
    "NOTICE",
    "NOTICE.txt",
    "LICENSE.txt",
    "LICENSE",
    "**/.svn/**",
    "**/CVS/**",
    "**/SCCS/**",
    "**/.*/**",
    "**/.*",
    "**/*~",
    "**/thumbs.db",
    "**/picasa.ini",
    "**/about.html",
    "**/package.html",
    "**/overview.html",
    "**/protobuf.meta",
    "**/_*",
    "**/_*/**",
    "**/*.kotlin_metadata",
];

/// The paths whose files from every JAR are joined into one by default.
const CONCATENATED: [&str; 2] = ["META-INF/services/**", "jacoco-agent.properties"];

/// The paths the package itself writes, which no Java resource may take.
const PACKAGE_PATHS: [&str; 6] = [
    "AndroidManifest.xml",
    "resources.arsc",
    "classes*.dex",
    "res/**",
    "lib/**",
    "assets/**",
];

/// One file for the package's root.
pub struct JavaResource {
    /// Its path in the package, `/`-separated, as in the JAR.
    pub path: String,
    /// Where it is written out, under the folder [`gather`] was given.
    pub file: PathBuf,
}

/// The Java resources of every library's JARs, written under `into`.
pub fn gather(libraries: &[AndroidLibrary], into: &Path) -> Result<Vec<JavaResource>> {
    let excluded = patterns(&EXCLUDED);
    let concatenated = patterns(&CONCATENATED);
    let reserved = patterns(&PACKAGE_PATHS);
    // A path's contents so far, with the library each part came from.
    let mut gathered: Vec<(String, Vec<u8>, &str)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for library in libraries {
        for jar in &library.class_jars {
            for (path, contents) in resources_of(jar)
                .with_context(|| format!("{}: reading {}", library.name, jar.display()))?
            {
                if matches_any(&excluded, &path) {
                    continue;
                }
                if matches_any(&reserved, &path) {
                    bail!(
                        "{}: its Java resource {path} would take a path the package writes",
                        library.name
                    );
                }
                match index.get(&path) {
                    None => {
                        index.insert(path.clone(), gathered.len());
                        gathered.push((path, contents, &library.name));
                    }
                    Some(&at) if matches_any(&concatenated, &path) => {
                        gathered[at].1.extend_from_slice(&contents)
                    }
                    Some(&at) => bail!(
                        "2 files found with path '{path}', from {} and {}: Java resources at one path are refused",
                        gathered[at].2,
                        library.name
                    ),
                }
            }
        }
    }
    gathered
        .into_iter()
        .map(|(path, contents, _)| {
            let file = into.join(&path);
            fs::create_dir_all(file.parent().unwrap_or(into))?;
            fs::write(&file, contents)?;
            Ok(JavaResource { path, file })
        })
        .collect()
}

/// The files of `jar` that are not classes, by their paths in it.
fn resources_of(jar: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut archive = zip::ZipArchive::new(fs::File::open(jar)?)?;
    let mut resources = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() || entry.name().ends_with(".class") {
            continue;
        }
        let path = entry
            .enclosed_name()
            .with_context(|| format!("the entry {} leaves the archive", entry.name()))?
            .to_string_lossy()
            .replace('\\', "/");
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        resources.push((path, contents));
    }
    Ok(resources)
}

fn patterns(globs: &[&str]) -> Vec<Pattern> {
    globs
        .iter()
        .map(|glob| Pattern::new(glob).expect("the packaging patterns are valid globs"))
        .collect()
}

/// Whether `path` is one of `patterns`, a `*` matching within one folder.
fn matches_any(patterns: &[Pattern], path: &str) -> bool {
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    patterns
        .iter()
        .any(|pattern| pattern.matches_with(path, options))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn jar(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let mut writer = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for (entry, contents) in entries {
            writer
                .start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    fn library(name: &str, jar: PathBuf) -> AndroidLibrary {
        AndroidLibrary {
            name: name.to_owned(),
            class_jars: vec![jar],
            manifest: None,
            resources: None,
            native_libraries: Vec::new(),
        }
    }

    fn scratch(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("waft-java-resources-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn gradles_default_exclusions_apply() {
        let excluded = patterns(&EXCLUDED);
        for path in [
            "META-INF/MANIFEST.MF",
            "META-INF/versions/9/OSGI-INF/MANIFEST.MF",
            "META-INF/maven/org.example/lib/pom.xml",
            "LICENSE",
            "a/.hidden",
            "a/_private",
            "kotlin/x.kotlin_metadata",
        ] {
            assert!(matches_any(&excluded, path), "{path} is excluded");
        }
        for path in [
            "META-INF/kotlin-stdlib.kotlin_module",
            "kotlin/kotlin.kotlin_builtins",
            "META-INF/services/java.nio.file.spi.FileSystemProvider",
            "com/example/answer.txt",
        ] {
            assert!(!matches_any(&excluded, path), "{path} is packaged");
        }
    }

    #[test]
    fn resources_are_gathered_services_joined_and_a_clash_refused() {
        let dir = scratch("gather");
        let first = library(
            "a.jar",
            jar(
                &dir,
                "a.jar",
                &[
                    ("com/a/A.class", b"class"),
                    ("com/a/answer.txt", b"42"),
                    ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0"),
                    ("META-INF/services/x.Provider", b"com.a.P\n"),
                ],
            ),
        );
        let second = library(
            "b.jar",
            jar(
                &dir,
                "b.jar",
                &[("META-INF/services/x.Provider", b"com.b.P\n")],
            ),
        );
        let resources = gather(&[first, second], &dir.join("out")).unwrap();
        let paths: Vec<_> = resources
            .iter()
            .map(|resource| resource.path.as_str())
            .collect();
        assert_eq!(paths, ["com/a/answer.txt", "META-INF/services/x.Provider"]);
        assert_eq!(
            fs::read_to_string(&resources[1].file).unwrap(),
            "com.a.P\ncom.b.P\n"
        );

        let clash = library("c.jar", jar(&dir, "c.jar", &[("com/a/answer.txt", b"41")]));
        let again = library("a.jar", dir.join("a.jar"));
        let refused = gather(&[again, clash], &dir.join("out2"))
            .err()
            .unwrap()
            .to_string();
        assert!(refused.contains("com/a/answer.txt"), "{refused}");
        assert!(
            refused.contains("a.jar") && refused.contains("c.jar"),
            "{refused}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_resource_at_one_of_the_packages_paths_is_refused() {
        let dir = scratch("reserved");
        let taking = library("d.jar", jar(&dir, "d.jar", &[("classes2.dex", b"dex")]));
        assert!(gather(&[taking], &dir.join("out")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}

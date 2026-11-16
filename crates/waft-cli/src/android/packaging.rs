//! Putting the `.apk` together with the SDK's own tools: `aapt2` compiles and links the resources
//! with the manifest, `javac` and `d8` compile Waft's classes and the libraries' into the dex files,
//! and the library and the rest are added to the package.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::AndroidLibrary;
use super::sdk::{Java, Sdk};

use crate::tools;

/// The Java classes every app's package compiles, by their paths under the sources' root.
const JAVA_SOURCES: [(&str, &str); 8] = [
    (
        "rs/waft/WaftActivity.java",
        include_str!("../../android/rs/waft/WaftActivity.java"),
    ),
    (
        "rs/waft/WaftDocumentActivity.java",
        include_str!("../../android/rs/waft/WaftDocumentActivity.java"),
    ),
    (
        "rs/waft/WaftTask.java",
        include_str!("../../android/rs/waft/WaftTask.java"),
    ),
    (
        "rs/waft/WaftFileProvider.java",
        include_str!("../../android/rs/waft/WaftFileProvider.java"),
    ),
    (
        "rs/waft/WaftNotificationReceiver.java",
        include_str!("../../android/rs/waft/WaftNotificationReceiver.java"),
    ),
    (
        "rs/waft/WaftTextInput.java",
        include_str!("../../android/rs/waft/WaftTextInput.java"),
    ),
    (
        "rs/waft/WaftWindowInsets.java",
        include_str!("../../android/rs/waft/WaftWindowInsets.java"),
    ),
    (
        "rs/waft/WaftShortcuts.java",
        include_str!("../../android/rs/waft/WaftShortcuts.java"),
    ),
];

/// The Java release the classes are compiled for, which `d8` takes and lowers to `min_sdk`.
const JAVA_RELEASE: &str = "11";

/// `aapt2`: the app's resources and each library's compiled, then linked with the manifest into an
/// unsigned package, with the R class of each library's package written under `r/` for its
/// classes to read their resources by.
///
/// Where two define one resource the app's wins, then the library listed first, as Gradle merges
/// them: the least of them is linked first and each one above it laid over it.
pub(super) fn link(
    sdk: &Sdk,
    stage: &Path,
    out: &Path,
    has_resources: bool,
    libraries: &[AndroidLibrary],
) -> Result<()> {
    let aapt2 = sdk.build_tools.join(tools::exe_name("aapt2"));
    let mut compiled = Vec::new();
    for (index, library) in libraries.iter().enumerate().rev() {
        if let Some(resources) = &library.resources {
            let archive = stage.join(format!("res-{index}.zip"));
            compile_resources(&aapt2, resources, &archive)
                .with_context(|| format!("{}: its resources do not compile", library.name))?;
            compiled.push(archive);
        }
    }
    if has_resources {
        let archive = stage.join("res.zip");
        compile_resources(&aapt2, &stage.join("res"), &archive)?;
        compiled.push(archive);
    }
    let mut command = Command::new(&aapt2);
    command
        .arg("link")
        .arg("-o")
        .arg(out)
        .arg("--manifest")
        .arg(stage.join("AndroidManifest.xml"))
        .arg("-I")
        .arg(&sdk.platform.jar);
    if let Some((least, overlays)) = compiled.split_first() {
        command.arg(least);
        for overlay in overlays {
            command.arg("-R").arg(overlay);
        }
        if !overlays.is_empty() {
            command.arg("--auto-add-overlay");
        }
    }
    let packages = library_packages(libraries);
    if !packages.is_empty() {
        command
            .arg("--java")
            .arg(stage.join("r"))
            .arg("--extra-packages")
            .arg(packages.join(":"));
    }
    tools::run(&mut command, "aapt2 link")
}

fn compile_resources(aapt2: &Path, res: &Path, archive: &Path) -> Result<()> {
    tools::run(
        Command::new(aapt2)
            .args(["compile", "--dir"])
            .arg(res)
            .arg("-o")
            .arg(archive),
        "aapt2 compile",
    )
}

/// The packages of the AARs among `libraries`, whose R classes the link writes.
fn library_packages(libraries: &[AndroidLibrary]) -> Vec<&str> {
    libraries
        .iter()
        .filter_map(|library| library.manifest.as_ref())
        .map(|manifest| manifest.package.as_str())
        .collect()
}

/// `javac` and `d8`: Waft's classes and the R classes the link wrote, compiled against the
/// platform's `android.jar` with the libraries' JARs on the class path, then dexed with those
/// JARs; answers the package's dex files, `classes.dex` and, past the 65,536 methods one file
/// holds, `classes2.dex` and on, as package entries.
pub(super) fn compile_classes(
    sdk: &Sdk,
    java: &Java,
    stage: &Path,
    min_sdk: u32,
    libraries: &[AndroidLibrary],
) -> Result<Vec<(String, PathBuf)>> {
    let sources = stage.join("java");
    fs::create_dir_all(sources.join("rs/waft"))?;
    let mut source_paths = Vec::new();
    for (path, source) in JAVA_SOURCES {
        let written = sources.join(path);
        fs::write(&written, source)?;
        source_paths.push(written);
    }
    let generated = stage.join("r");
    if generated.is_dir() {
        source_paths.extend(files_with_extension(&generated, "java")?);
    }
    let class_jars: Vec<&Path> = libraries
        .iter()
        .flat_map(|library| &library.class_jars)
        .map(PathBuf::as_path)
        .collect();
    let class_path = std::env::join_paths(
        std::iter::once(sdk.platform.jar.as_path()).chain(class_jars.iter().copied()),
    )?;
    let classes = stage.join("classes");
    tools::run(
        java.bin("javac")
            .args(["--release", JAVA_RELEASE, "-Xlint:-options", "-classpath"])
            .arg(&class_path)
            .arg("-d")
            .arg(&classes)
            .args(&source_paths),
        "javac",
    )?;
    let dex = stage.join("dex");
    fs::create_dir_all(&dex)?;
    tools::run(
        java.bin("java")
            .arg("-cp")
            .arg(sdk.build_tools.join("lib/d8.jar"))
            .args(["com.android.tools.r8.D8", "--release", "--min-api"])
            .arg(min_sdk.to_string())
            .arg("--lib")
            .arg(&sdk.platform.jar)
            .arg("--output")
            .arg(&dex)
            .args(files_with_extension(&classes, "class")?)
            .args(class_jars),
        "d8",
    )?;
    dex_files(&dex)
}

/// The files under `dir`, at any depth, whose names end in `.<extension>`: a nested class's file
/// is beside its outer class's.
fn files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(files_with_extension(&path, extension)?);
        } else if path.extension().is_some_and(|found| found == extension) {
            files.push(path);
        }
    }
    Ok(files)
}

/// The dex files `d8` wrote into `dir`, by their names in the package.
fn dex_files(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.starts_with("classes") && name.ends_with(".dex") {
            files.push((name, path));
        }
    }
    files.sort();
    Ok(files)
}

/// Adds each file to the package under its entry's name, stored rather than deflated: the system
/// loads a shared library from the package in place, page-aligned by `zipalign`, and maps the dex.
pub(super) fn add_to_package(apk: &Path, entries: &[(String, PathBuf)]) -> Result<()> {
    let package = fs::OpenOptions::new().read(true).write(true).open(apk)?;
    let mut writer = zip::ZipWriter::new_append(package).context("reading the linked package")?;
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (entry, file) in entries {
        writer.start_file(entry.as_str(), options)?;
        writer.write_all(&fs::read(file)?)?;
    }
    writer.finish()?;
    Ok(())
}

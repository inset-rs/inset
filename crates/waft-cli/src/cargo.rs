//! Runs cargo and finds what it produced.

use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use cargo_metadata::Message;

use crate::project::{Profile, Project};

/// What a build is for: the target cargo is asked to build, by its name, and the one file
/// taken back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Artifact<'a> {
    /// A binary target: the one the desktops and iOS run, or a helper an app carries.
    Executable(&'a str),
    /// A `cdylib` target as the target's loader takes it: `.so`, `.dylib` or `.dll`.
    Library(&'a str),
    /// The `.wasm` a `cdylib` target compiles to on a wasm target.
    Wasm(&'a str),
}

impl Artifact<'_> {
    fn target_name(&self) -> &str {
        match self {
            Artifact::Executable(name) | Artifact::Library(name) | Artifact::Wasm(name) => name,
        }
    }

    /// The extensions this artifact's file can have, empty for the binary, which cargo
    /// names outright.
    fn extensions(&self) -> &'static [&'static str] {
        match self {
            Artifact::Executable(_) => &[],
            Artifact::Library(_) => &["so", "dylib", "dll"],
            Artifact::Wasm(_) => &["wasm"],
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Artifact::Executable(_) => "an executable",
            Artifact::Library(_) => "a library",
            Artifact::Wasm(_) => "a .wasm",
        }
    }
}

pub struct Build<'a> {
    /// The app, in whose workspace cargo runs.
    pub project: &'a Project,
    /// The package the artifact is a target of: the app's own, or another of its workspace.
    pub package: &'a str,
    pub profile: Profile,
    pub triple: Option<&'a str>,
    pub artifact: Artifact<'a>,
    /// Environment for rustc and the linker, such as a deployment target.
    pub env: Vec<(&'a str, String)>,
    /// Files the linker takes into the artifact itself, as a Windows executable's compiled
    /// resources. They go to rustc for the artifact's own crate alone, through `cargo rustc`, so
    /// no dependency is built again for them. Cargo links again when an input's path changes, not
    /// its content: a file whose content changes takes a new name.
    pub link_inputs: Vec<PathBuf>,
}

/// `cargo build` with JSON messages, so the artifact's path comes from cargo instead of
/// being guessed from the target directory layout; `cargo rustc` where the artifact links inputs
/// of its own.
pub fn build(build: &Build) -> Result<PathBuf> {
    let mut command = Command::new("cargo");
    let subcommand = if build.link_inputs.is_empty() {
        "build"
    } else {
        "rustc"
    };
    command
        .current_dir(&build.project.root)
        .arg(subcommand)
        .arg("--message-format=json-render-diagnostics")
        .args(["-p", build.package]);
    if build.profile.is_release() {
        command.arg("--release");
    }
    if let Some(triple) = build.triple {
        command.args(["--target", triple]);
    }
    match build.artifact {
        Artifact::Executable(name) => {
            command.args(["--bin", name]);
        }
        Artifact::Library(_) | Artifact::Wasm(_) => {
            command.arg("--lib");
        }
    }
    for (key, value) in &build.env {
        command.env(key, value);
    }
    if !build.link_inputs.is_empty() {
        command.arg("--");
        for input in &build.link_inputs {
            let mut argument = std::ffi::OsString::from("-Clink-arg=");
            argument.push(input);
            command.arg(argument);
        }
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("cargo is not on PATH")?;
    let stdout = child.stdout.take().expect("piped stdout");
    let mut found = None;
    for message in Message::parse_stream(BufReader::new(stdout)) {
        if let Message::CompilerArtifact(artifact) = message? {
            if let Some(path) = wanted_file(build, &artifact) {
                found = Some(path);
            }
        }
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("cargo build failed");
    }
    found.with_context(|| format!("cargo produced {}", build.artifact.describe()))
}

/// The file this build asked for, out of what cargo says one compilation produced.
///
/// The target's name is checked as well as the extension: a dependency that is a proc
/// macro compiles to a `.dylib` or `.so` of its own, and cargo reports it the same way.
fn wanted_file(build: &Build, artifact: &cargo_metadata::Artifact) -> Option<PathBuf> {
    if artifact.target.name != build.artifact.target_name() {
        return None;
    }
    if let Artifact::Executable(_) = build.artifact {
        return artifact
            .executable
            .as_ref()
            .map(|path| path.clone().into_std_path_buf());
    }
    artifact
        .filenames
        .iter()
        .find(|file| {
            file.extension()
                .is_some_and(|extension| build.artifact.extensions().contains(&extension))
        })
        .map(|file| file.clone().into_std_path_buf())
}

//! Runs cargo and finds what it produced.

use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use cargo_metadata::Message;

use crate::project::{Profile, Project};

/// What a build is for: the target cargo is asked to build, and the one file taken back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Artifact {
    /// The binary the desktops and iOS run.
    Executable,
    /// The `cdylib` as the target's loader takes it: `.so`, `.dylib` or `.dll`.
    Library,
    /// The `.wasm` a `cdylib` compiles to on a wasm target.
    Wasm,
}

impl Artifact {
    /// The extensions this artifact's file can have, empty for the binary, which cargo
    /// names outright.
    fn extensions(self) -> &'static [&'static str] {
        match self {
            Artifact::Executable => &[],
            Artifact::Library => &["so", "dylib", "dll"],
            Artifact::Wasm => &["wasm"],
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Artifact::Executable => "an executable",
            Artifact::Library => "a library",
            Artifact::Wasm => "a .wasm",
        }
    }
}

pub struct Build<'a> {
    pub project: &'a Project,
    pub profile: Profile,
    pub triple: Option<&'a str>,
    pub artifact: Artifact,
    /// Environment for rustc and the linker, such as a deployment target.
    pub env: Vec<(&'a str, String)>,
}

/// `cargo build` with JSON messages, so the artifact's path comes from cargo instead of
/// being guessed from the target directory layout.
pub fn build(build: &Build) -> Result<PathBuf> {
    let mut command = Command::new("cargo");
    command
        .current_dir(&build.project.root)
        .arg("build")
        .arg("--message-format=json-render-diagnostics")
        .args(["-p", &build.project.package]);
    if build.profile.is_release() {
        command.arg("--release");
    }
    if let Some(triple) = build.triple {
        command.args(["--target", triple]);
    }
    match build.artifact {
        Artifact::Executable => {
            command.args(["--bin", &build.project.bin]);
        }
        Artifact::Library | Artifact::Wasm => {
            command.arg("--lib");
        }
    }
    for (key, value) in &build.env {
        command.env(key, value);
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
    let wanted_name = match build.artifact {
        Artifact::Executable => &build.project.bin,
        Artifact::Library | Artifact::Wasm => build.project.library.as_ref()?,
    };
    if &artifact.target.name != wanted_name {
        return None;
    }
    if build.artifact == Artifact::Executable {
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

/// `cargo run` for the host desktop; the app's stdio is the terminal.
pub fn run(project: &Project, profile: Profile, args: &[String]) -> Result<()> {
    let mut command = Command::new("cargo");
    command.current_dir(&project.root).arg("run").args([
        "-p",
        &project.package,
        "--bin",
        &project.bin,
    ]);
    if profile.is_release() {
        command.arg("--release");
    }
    if !args.is_empty() {
        command.arg("--").args(args);
    }
    let status = command.status().context("cargo is not on PATH")?;
    if !status.success() {
        bail!("cargo run failed");
    }
    Ok(())
}

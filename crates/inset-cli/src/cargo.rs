//! Runs cargo and finds what it produced.

use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use cargo_metadata::Message;

use crate::project::{Profile, Project};

pub enum Kind {
    Bin,
    Lib,
}

pub struct Build<'a> {
    pub project: &'a Project,
    pub profile: Profile,
    pub triple: Option<&'a str>,
    pub kind: Kind,
    /// Environment for rustc and the linker, such as a deployment target.
    pub env: Vec<(&'a str, String)>,
}

pub struct Artifacts {
    pub executable: Option<PathBuf>,
    pub wasm: Option<PathBuf>,
}

/// `cargo build` with JSON messages, so the artifact paths come from cargo
/// instead of being guessed from the target directory layout.
pub fn build(build: &Build) -> Result<Artifacts> {
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
    match build.kind {
        Kind::Bin => {
            command.args(["--bin", &build.project.bin]);
        }
        Kind::Lib => {
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
    let mut artifacts = Artifacts {
        executable: None,
        wasm: None,
    };
    for message in Message::parse_stream(BufReader::new(stdout)) {
        if let Message::CompilerArtifact(artifact) = message? {
            if let Some(executable) = artifact.executable
                && artifact.target.name == build.project.bin
            {
                artifacts.executable = Some(executable.into_std_path_buf());
            }
            if let Some(wasm) = artifact
                .filenames
                .iter()
                .find(|f| f.extension() == Some("wasm"))
            {
                artifacts.wasm = Some(wasm.clone().into_std_path_buf());
            }
        }
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("cargo build failed");
    }
    Ok(artifacts)
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

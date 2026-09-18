//! `cargo inset build wasm`: the app as a wasm component for a WASI host.
//!
//! The library target is built for `wasm32-wasip2`, which rustc links straight into a
//! component, and copied out under its package name. Which host runs it — wapk, or any other
//! with the same imports — is the app's business: the component imports what the app's
//! embedder crate asks for, and this tool adds nothing to it. There is no page, no glue and no
//! packaging, hence a bare `.wasm`; a container format would wrap this file.

use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::cargo::{self, Artifact, Build};
use crate::project::{Profile, Project};
use crate::tools;

const WASM_TARGET: &str = "wasm32-wasip2";

pub fn build(project: &Project, profile: Profile) -> Result<PathBuf> {
    if project.library.is_none() {
        bail!(
            "a wasm component is built from a library: add `crate-type = [\"cdylib\", \"rlib\"]` under [lib] in Cargo.toml"
        );
    }
    tools::ensure_rust_target(&project.root, WASM_TARGET)?;
    let wasm = cargo::build(&Build {
        project,
        profile,
        triple: Some(WASM_TARGET),
        artifact: Artifact::Wasm,
        env: Vec::new(),
    })?;

    let out = project.out_dir("wasm", profile);
    fs::create_dir_all(&out)?;
    let component = out.join(format!("{}.wasm", project.package));
    fs::copy(&wasm, &component)?;
    Ok(component)
}

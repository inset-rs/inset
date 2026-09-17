//! The static web folder: the app's wasm module, its JS glue, and a page with one canvas.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::cargo::{self, Build, Kind};
use crate::project::{Profile, Project};
use crate::tools;

const WASM_TARGET: &str = "wasm32-unknown-unknown";

pub fn build(project: &Project, profile: Profile) -> Result<PathBuf> {
    if !project.has_cdylib {
        bail!(
            "the web host loads a library: add `crate-type = [\"cdylib\", \"rlib\"]` under [lib] in Cargo.toml"
        );
    }
    tools::ensure_rust_target(&project.root, WASM_TARGET)?;
    let artifacts = cargo::build(&Build {
        project,
        profile,
        triple: Some(WASM_TARGET),
        kind: Kind::Lib,
        env: Vec::new(),
    })?;
    let wasm = artifacts.wasm.context("cargo produced no .wasm")?;

    let out = project.out_dir("web", profile);
    if out.exists() {
        fs::remove_dir_all(&out)?;
    }
    fs::create_dir_all(&out)?;

    let version = tools::locked_wasm_bindgen_version(&project.workspace_root)?;
    let bindgen = tools::wasm_bindgen(&version)?;
    tools::run(
        Command::new(bindgen)
            .args([
                "--target",
                "web",
                "--no-typescript",
                "--out-name",
                "app",
                "--out-dir",
            ])
            .arg(&out)
            .arg(&wasm),
        "wasm-bindgen",
    )?;

    let module = out.join("app_bg.wasm");
    if profile.is_release() {
        optimize(&module)?;
    }

    let page = project.root.join("index.html");
    if page.exists() {
        fs::copy(&page, out.join("index.html"))?;
    } else {
        fs::write(out.join("index.html"), index_html(&project.name))?;
    }

    for (source, relative) in project.resource_files()? {
        let dest = out.join(&relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &dest)?;
    }

    if profile.is_release() {
        compress(&module)?;
        compress(&out.join("app.js"))?;
    }
    Ok(out)
}

fn optimize(module: &Path) -> Result<()> {
    let Some(wasm_opt) = tools::wasm_opt() else {
        eprintln!("note: wasm-opt not found, module left unoptimized (brew install binaryen)");
        return Ok(());
    };
    tools::run(
        Command::new(wasm_opt)
            .args([
                "-Os",
                "--enable-bulk-memory",
                "--enable-nontrapping-float-to-int",
                "-o",
            ])
            .arg(module)
            .arg(module),
        "wasm-opt",
    )
}

/// Writes `<file>.br` beside the file for hosts that serve pre-compressed assets.
fn compress(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    let mut target = path.as_os_str().to_owned();
    target.push(".br");
    let file = fs::File::create(&target)?;
    let mut writer = brotli::CompressorWriter::new(file, 1 << 16, 11, 22);
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

/// The page an app gets when it ships no `index.html` of its own: it instantiates the module
/// and calls the `main` that `#[inset::main]` exports. A page of the app's own must do the
/// same.
pub fn index_html(title: &str) -> String {
    let title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <title>{title}</title>
    <style>
      html,
      body {{
        margin: 0;
        height: 100%;
        overflow: hidden;
        background: #000;
      }}
      canvas {{
        display: block;
        width: 100%;
        height: 100%;
        touch-action: none;
      }}
    </style>
  </head>
  <body>
    <canvas id="inset"></canvas>
    <script type="module">
      import init from "./app.js";
      const wasm = await init();
      wasm.main();
    </script>
  </body>
</html>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_escapes_the_title_and_imports_the_glue() {
        let page = index_html("A & B");
        assert!(page.contains("<title>A &amp; B</title>"));
        assert!(page.contains(r#"import init from "./app.js""#));
        assert!(page.contains("wasm.main();"));
        assert!(page.contains(r#"<canvas id="inset">"#));
    }
}

//! The web: the static folder of the app's wasm module, its JS glue, and a page with one canvas,
//! which `run` serves here and opens in a browser (`serve.rs`).

mod serve;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::cargo::{self, Artifact, Build};
use crate::project::{Profile, Project};
use crate::tools;

pub use serve::Browser;

const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// The folder built, served here and opened in `browser`.
pub fn run(project: &Project, profile: Profile, browser: Browser) -> Result<()> {
    let out = build(project, profile)?;
    serve::serve_and_open(out, browser)
}

pub fn build(project: &Project, profile: Profile) -> Result<PathBuf> {
    let library = project.library.as_deref().context(
        "the web host loads a library: add `crate-type = [\"cdylib\", \"rlib\"]` under [lib] in Cargo.toml",
    )?;
    tools::ensure_rust_target(&project.root, WASM_TARGET)?;
    let wasm = cargo::build(&Build {
        project,
        package: &project.package,
        profile,
        triple: Some(WASM_TARGET),
        artifact: Artifact::Wasm(library),
        env: if profile.is_release() {
            vec![
                ("CARGO_PROFILE_RELEASE_LTO", "thin".to_owned()),
                ("CARGO_PROFILE_RELEASE_OPT_LEVEL", "s".to_owned()),
                ("CARGO_PROFILE_RELEASE_STRIP", "symbols".to_owned()),
            ]
        } else {
            Vec::new()
        },
        link_inputs: Vec::new(),
    })?;

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
                "--strip-debug",
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

/// The page an app gets when it ships no `index.html` of its own: it streams the module with a
/// progress indicator, instantiates it, and calls the `main` that `#[waft::main]` exports.
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
        background: #faf9f6;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      }}
      @media (prefers-color-scheme: dark) {{
        html,
        body {{
          background: #141413;
        }}
      }}
      canvas {{
        display: block;
        width: 100%;
        height: 100%;
        touch-action: none;
        outline: none;
      }}
      #waft-loader {{
        position: fixed;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 10px;
        pointer-events: none;
        transition: opacity 160ms ease;
      }}
      #waft-loader.done {{
        opacity: 0;
      }}
      #waft-track {{
        width: 160px;
        height: 3px;
        border-radius: 999px;
        background: rgba(128, 128, 128, 0.2);
        overflow: hidden;
      }}
      #waft-bar {{
        width: 12%;
        height: 100%;
        border-radius: 999px;
        background: #c85a28;
        transition: width 120ms ease;
      }}
      #waft-status {{
        font-size: 11.5px;
        color: rgba(128, 128, 128, 0.85);
        font-variant-numeric: tabular-nums;
      }}
    </style>
  </head>
  <body>
    <canvas id="waft"></canvas>
    <div id="waft-loader">
      <div id="waft-track"><div id="waft-bar"></div></div>
      <div id="waft-status">Loading WebGPU module…</div>
    </div>
    <script type="module">
      import init from "./app.js";
      const bar = document.getElementById("waft-bar");
      const status = document.getElementById("waft-status");
      const loader = document.getElementById("waft-loader");

      async function loadWasm() {{
        const res = await fetch("./app_bg.wasm");
        const total = Number(res.headers.get("content-length")) || 0;
        if (!res.body || !total) {{
          return await res.arrayBuffer();
        }}
        const reader = res.body.getReader();
        const chunks = [];
        let loaded = 0;
        while (true) {{
          const {{ done, value }} = await reader.read();
          if (done) break;
          chunks.push(value);
          loaded += value.byteLength;
          const pct = Math.min(100, Math.round((loaded / total) * 100));
          if (bar) bar.style.width = pct + "%";
          if (status) status.textContent = "Loading " + pct + "%";
        }}
        const bytes = new Uint8Array(loaded);
        let offset = 0;
        for (const chunk of chunks) {{
          bytes.set(chunk, offset);
          offset += chunk.byteLength;
        }}
        return bytes.buffer;
      }}

      const buffer = await loadWasm();
      if (status) status.textContent = "Starting WebGPU…";
      const wasm = await init({{ module_or_path: buffer }});
      wasm.main();
      if (loader) {{
        loader.classList.add("done");
        setTimeout(() => loader.remove(), 200);
      }}
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
        assert!(page.contains(r#"<canvas id="waft">"#));
        assert!(page.contains(r#"id="waft-loader""#));
    }
}

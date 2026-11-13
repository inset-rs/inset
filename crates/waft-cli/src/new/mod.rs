//! `cargo waft new`: a crate in the shape every host can load, starting from one widget kit.
//!
//! One file per template beside this one; each supplies the kit dependency and
//! `src/lib.rs`. Everything else about the crate is the same.

mod cupertino;
mod winui;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;

use crate::icons;
use crate::project::display_name;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Template {
    /// iOS-styled widgets, for phone apps.
    #[default]
    Cupertino,
    /// WinUI controls and Fluent styling, for desktop apps.
    Winui,
}

/// What a template contributes: the kit's Cargo.toml lines and the library source.
struct Kit {
    /// Dependency lines; `{waft}` stands for the framework's version requirement.
    dependencies: &'static str,
    lib_rs: &'static str,
}

impl Template {
    fn kit(self) -> &'static Kit {
        match self {
            Template::Cupertino => &cupertino::KIT,
            Template::Winui => &winui::KIT,
        }
    }
}

pub fn create(name: &str, template: Template, parent: Option<&Path>) -> Result<()> {
    validate(name)?;
    let dir = parent.unwrap_or(Path::new(".")).join(name);
    if dir.exists() {
        bail!("{} already exists", dir.display());
    }
    let kit = template.kit();
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("assets"))?;
    let display = display_name(name);
    fs::write(dir.join("Cargo.toml"), cargo_toml(name, &display, kit))?;
    fs::write(dir.join("src/lib.rs"), kit.lib_rs)?;
    fs::write(dir.join("src/main.rs"), main_rs(name))?;
    fs::write(dir.join("rust-toolchain.toml"), TOOLCHAIN)?;
    fs::write(dir.join(".gitignore"), "/target\n")?;
    icons::placeholder()
        .save(dir.join("assets/icon.png"))
        .context("writing the placeholder icon")?;
    println!("created {}", dir.display());
    println!("  cd {name}");
    println!("  cargo waft run              # this desktop");
    println!("  cargo waft run -d ios       # a simulator");
    println!("  cargo waft run -d android   # an emulator");
    println!("  cargo waft run -d web       # a browser");
    Ok(())
}

fn validate(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    if !valid {
        bail!(
            "`{name}` is not a package name: lowercase letters, digits, `-` and `_`, starting with a letter"
        );
    }
    Ok(())
}

fn cargo_toml(name: &str, display: &str, kit: &Kit) -> String {
    let waft = waft_version();
    let dependencies = kit.dependencies.replace("{waft}", &waft);
    let identifier = format!("com.example.{}", name.replace('_', "-"));
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

# The app is the library; the browser and Android load it as one.
[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
waft = "{waft}"
{dependencies}
[package.metadata.waft]
identifier = "{identifier}"
name = "{display}"
icon = "assets/icon.png"

# A few percent off the web download at the cost of a slower release build.
[profile.release]
lto = "fat"
codegen-units = 1
"#
    )
}

/// The `waft` crate line: this tool's own major.minor.
fn waft_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) if major != "0" => format!("{major}.{minor}"),
        (Some(_), Some(_)) => version
            .rsplit_once('.')
            .map(|(v, _)| v.to_owned())
            .unwrap_or_else(|| version.to_owned()),
        _ => version.to_owned(),
    }
}

/// The binary: the library's `main`, as a GUI program on Windows, which opens no console window;
/// the entry `#[waft::main]` emits sends what the app writes to the terminal it was run from.
/// Only a binary's own source can say its subsystem, so it is here, not in the macro.
fn main_rs(name: &str) -> String {
    let krate = name.replace('-', "_");
    format!("{GUI_SUBSYSTEM}\nfn main() {{\n    {krate}::main();\n}}\n")
}

/// The attribute that makes the binary a GUI program on Windows, with its reason; ignored
/// elsewhere.
const GUI_SUBSYSTEM: &str = "// A GUI program on Windows: no console window opens with it, and what it writes goes to the\n// terminal it was run from.\n#![windows_subsystem = \"windows\"]\n";

const TOOLCHAIN: &str = "[toolchain]\nchannel = \"nightly\"\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_cargo_rules() {
        assert!(validate("my-app").is_ok());
        assert!(validate("My App").is_err());
        assert!(validate("1st").is_err());
    }

    #[test]
    fn manifest_names_the_lib_bin_and_kit() {
        let manifest = cargo_toml("my-app", "My app", Template::Cupertino.kit());
        assert!(manifest.contains("crate-type = [\"cdylib\", \"rlib\"]"));
        assert!(manifest.contains("identifier = \"com.example.my-app\""));
        assert!(manifest.contains("name = \"My app\""));
        assert!(manifest.contains("[profile.release]"));
        assert!(manifest.contains("waft-cupertino = \""));
        assert!(!manifest.contains("{waft}"));
        let winui = cargo_toml("my-app", "My app", Template::Winui.kit());
        assert!(winui.contains("waft-winui = \""));
    }

    #[test]
    fn each_template_has_an_entry_point() {
        for template in [Template::Cupertino, Template::Winui] {
            assert!(template.kit().lib_rs.contains("#[waft::main"));
        }
    }

    #[test]
    fn binary_calls_the_library_main_as_a_gui_program() {
        let main = main_rs("my-app");
        assert!(
            main.contains("#![windows_subsystem = \"windows\"]\n"),
            "{main}"
        );
        assert!(
            main.ends_with("fn main() {\n    my_app::main();\n}\n"),
            "{main}"
        );
    }
}

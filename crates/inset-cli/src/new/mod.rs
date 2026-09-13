//! `cargo inset new`: a crate in the shape every host can load, starting from one widget kit.
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
    /// Dependency lines; `{inset}` stands for the framework's version requirement.
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
    println!("  cargo inset run              # this desktop");
    println!("  cargo inset run -d ios       # a simulator");
    println!("  cargo inset run -d web       # a browser");
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
    let inset = inset_version();
    let dependencies = kit.dependencies.replace("{inset}", &inset);
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
inset = "{inset}"
{dependencies}
[package.metadata.inset]
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

/// The `inset` crate line: this tool's own major.minor.
fn inset_version() -> String {
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

fn main_rs(name: &str) -> String {
    let krate = name.replace('-', "_");
    format!("fn main() {{\n    {krate}::main();\n}}\n")
}

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
        assert!(manifest.contains("inset-cupertino = \""));
        assert!(!manifest.contains("{inset}"));
        let winui = cargo_toml("my-app", "My app", Template::Winui.kit());
        assert!(winui.contains("inset-winui = \""));
    }

    #[test]
    fn each_template_has_an_entry_point() {
        for template in [Template::Cupertino, Template::Winui] {
            assert!(template.kit().lib_rs.contains("#[inset::main"));
        }
    }

    #[test]
    fn binary_calls_the_library_main() {
        assert_eq!(main_rs("my-app"), "fn main() {\n    my_app::main();\n}\n");
    }
}

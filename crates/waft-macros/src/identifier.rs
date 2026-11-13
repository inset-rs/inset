//! The app's identifier, read from its manifest as the crate is compiled: `identifier` under
//! `[package.metadata.waft]`, or, without one, `com.example.<package>`, as `cargo waft` names the
//! app in every bundle and package it makes. It is the app's Linux app id.
//!
//! The invariant: the identifier is the CLI's, rule for rule, so a window carries the app id the
//! desktop file `cargo waft` installed is named after; a change to the CLI's default changes this
//! one. The expression carries the manifest as a dependency of the crate, so editing it rebuilds
//! the crate with the identifier it gives.

use proc_macro2::{Span, TokenStream};
use quote::quote;

/// The app's identifier as a string expression, the manifest read for it a dependency of the
/// crate being compiled.
pub fn expression() -> TokenStream {
    match read() {
        Ok(identifier) => quote! {
            {
                const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
                #identifier
            }
        },
        Err(error) => syn::Error::new(Span::call_site(), error).to_compile_error(),
    }
}

/// The identifier the manifest of the crate being compiled gives.
fn read() -> Result<String, String> {
    let directory = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "the app's identifier is read from its Cargo.toml: build it with cargo")?;
    let path = std::path::Path::new(&directory).join("Cargo.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    identifier_in(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// The identifier `manifest` gives: its own, or the default for its package's name, with `_`,
/// which an identifier does not hold, written `-`.
fn identifier_in(manifest: &str) -> Result<String, String> {
    let manifest: toml::Table = manifest.parse().map_err(|error| format!("{error}"))?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or("no [package]")?;
    let given = package
        .get("metadata")
        .and_then(|metadata| metadata.get("waft"))
        .and_then(|waft| waft.get("identifier"));
    match given {
        Some(identifier) => identifier
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "`identifier` under [package.metadata.waft] is not a string".into()),
        None => {
            let name = package
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or("[package] has no name")?;
            Ok(format!("com.example.{}", name.replace('_', "-")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::identifier_in;

    #[test]
    fn the_identifier_is_the_one_the_metadata_gives() {
        let manifest = r#"
            [package]
            name = "waos-app"

            [package.metadata.waft]
            identifier = "dev.waos.app"
        "#;
        assert_eq!(identifier_in(manifest).unwrap(), "dev.waos.app");
    }

    #[test]
    fn without_one_it_is_the_clis_default_for_the_package() {
        let manifest = r#"
            [package]
            name = "cupertino_gallery"
        "#;
        assert_eq!(
            identifier_in(manifest).unwrap(),
            "com.example.cupertino-gallery"
        );
    }
}

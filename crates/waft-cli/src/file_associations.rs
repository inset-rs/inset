//! What the app tells each system about the files it opens: cargo-packager's file
//! associations for every desktop package, and for the macOS bundle the `Info.plist` keys
//! cargo-packager leaves out.
//!
//! cargo-packager writes a macOS document type that names its extensions and nothing else.
//! macOS also wants the type itself declared, exported by the app that defines it and
//! imported by an app that only opens it, and a document type that names that declaration
//! and ranks the app's claim on it. The fragment [`macos_info_plist`] makes replaces
//! cargo-packager's `CFBundleDocumentTypes` with such types and adds the declarations; keep
//! the two describing the same types.

use anyhow::{Result, bail};
use cargo_packager::config::{self as packager, BundleTypeRole};
use plist::{Dictionary, Value};

use crate::project::{FileAssociation, FileRole};

/// Refuses a file association no system would match a file to: one without extensions, or with an
/// extension written with its dot.
pub fn check(associations: &[FileAssociation]) -> Result<()> {
    for association in associations {
        if association.extensions.is_empty() {
            bail!("file association `{}` has no extensions", association.name);
        }
        if let Some(extension) = association
            .extensions
            .iter()
            .find(|extension| extension.is_empty() || extension.starts_with('.'))
        {
            bail!(
                "file association `{}`: `{extension}` is not an extension; write it without the dot",
                association.name
            );
        }
    }
    Ok(())
}

/// cargo-packager's associations: the macOS document types, the extensions the Windows
/// installers register, and the MIME types of the Linux desktop entry.
pub fn packager_associations(associations: &[FileAssociation]) -> Vec<packager::FileAssociation> {
    associations
        .iter()
        .map(|association| {
            let packaged = packager::FileAssociation::new(association.extensions.iter().cloned())
                .name(association.name.clone())
                .description(association.name.clone())
                .role(bundle_type_role(association.role));
            match &association.mime_type {
                Some(mime_type) => packaged.mime_type(mime_type.clone()),
                None => packaged,
            }
        })
        .collect()
}

/// The `Info.plist` keys for these types that cargo-packager does not write: document types
/// that name each type's declaration and rank the app's claim, the declarations of the types
/// the app owns, and those of the types it opens that have an identifier.
pub fn macos_info_plist(associations: &[FileAssociation], app_identifier: &str) -> Dictionary {
    let mut info = Dictionary::new();
    info.insert(
        "CFBundleDocumentTypes".into(),
        Value::Array(
            associations
                .iter()
                .map(|association| document_type(association, app_identifier))
                .collect(),
        ),
    );
    let declarations = |owned: bool| -> Vec<Value> {
        associations
            .iter()
            .filter(|association| association.owner == owned)
            .filter_map(|association| {
                let identifier = type_identifier(association, app_identifier)?;
                Some(type_declaration(association, identifier))
            })
            .collect()
    };
    let exported = declarations(true);
    if !exported.is_empty() {
        info.insert("UTExportedTypeDeclarations".into(), Value::Array(exported));
    }
    let imported = declarations(false);
    if !imported.is_empty() {
        info.insert("UTImportedTypeDeclarations".into(), Value::Array(imported));
    }
    info
}

/// One `CFBundleDocumentTypes` entry: the type by its identifier where it has one, which
/// Launch Services then prefers to the extensions, else by its extensions. The owner of a
/// type ranks first for it; an app that only opens it is a default, so a file with no owning
/// app installed still opens in this one.
fn document_type(association: &FileAssociation, app_identifier: &str) -> Value {
    let mut entry = Dictionary::new();
    entry.insert("CFBundleTypeName".into(), association.name.clone().into());
    entry.insert(
        "CFBundleTypeRole".into(),
        bundle_type_role(association.role).to_string().into(),
    );
    let rank = if association.owner {
        "Owner"
    } else {
        "Default"
    };
    entry.insert("LSHandlerRank".into(), rank.into());
    match type_identifier(association, app_identifier) {
        Some(identifier) => {
            entry.insert(
                "LSItemContentTypes".into(),
                Value::Array(vec![identifier.into()]),
            );
        }
        None => {
            entry.insert(
                "CFBundleTypeExtensions".into(),
                strings(&association.extensions),
            );
        }
    }
    Value::Dictionary(entry)
}

/// A `UTExportedTypeDeclarations` or `UTImportedTypeDeclarations` entry: the type's
/// identifier, description and tags, as a kind of data.
fn type_declaration(association: &FileAssociation, identifier: String) -> Value {
    let mut tags = Dictionary::new();
    tags.insert(
        "public.filename-extension".into(),
        strings(&association.extensions),
    );
    if let Some(mime_type) = &association.mime_type {
        tags.insert("public.mime-type".into(), mime_type.clone().into());
    }
    let mut declaration = Dictionary::new();
    declaration.insert("UTTypeIdentifier".into(), identifier.into());
    declaration.insert("UTTypeDescription".into(), association.name.clone().into());
    declaration.insert(
        "UTTypeConformsTo".into(),
        Value::Array(vec!["public.data".into()]),
    );
    declaration.insert("UTTypeTagSpecification".into(), Value::Dictionary(tags));
    Value::Dictionary(declaration)
}

/// The type's identifier on macOS: the one given, or for a type the app owns, the app's
/// identifier followed by the first extension.
fn type_identifier(association: &FileAssociation, app_identifier: &str) -> Option<String> {
    association.identifier.clone().or_else(|| {
        association.owner.then(|| {
            format!(
                "{app_identifier}.{}",
                identifier_part(&association.extensions[0])
            )
        })
    })
}

/// `text` with what a type identifier may not hold, anything but ASCII letters, digits, `-`
/// and `.`, replaced by `-`.
fn identifier_part(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn bundle_type_role(role: FileRole) -> BundleTypeRole {
    match role {
        FileRole::Editor => BundleTypeRole::Editor,
        FileRole::Viewer => BundleTypeRole::Viewer,
        FileRole::Shell => BundleTypeRole::Shell,
        FileRole::None => BundleTypeRole::None,
    }
}

fn strings(items: &[String]) -> Value {
    Value::Array(items.iter().map(|item| item.clone().into()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_associations() -> Vec<FileAssociation> {
        serde_json::from_value(serde_json::json!([
            { "extensions": ["wapk"], "name": "WaOS program", "owner": true },
            {
                "extensions": ["wasm"],
                "name": "WebAssembly module",
                "identifier": "org.webassembly.wasm",
                "mime_type": "application/wasm",
            },
            { "extensions": ["txt"], "name": "Text", "role": "editor" },
        ]))
        .unwrap()
    }

    fn entries(info: &Dictionary, key: &str) -> Vec<Dictionary> {
        info[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry.as_dictionary().unwrap().clone())
            .collect()
    }

    fn string<'a>(entry: &'a Dictionary, key: &str) -> &'a str {
        entry[key].as_string().unwrap()
    }

    #[test]
    fn an_owned_type_is_exported_under_the_apps_identifier_and_ranks_the_app_its_owner() {
        let info = macos_info_plist(&sample_associations(), "dev.wasdk.waos");
        let exported = entries(&info, "UTExportedTypeDeclarations");
        assert_eq!(exported.len(), 1);
        assert_eq!(
            string(&exported[0], "UTTypeIdentifier"),
            "dev.wasdk.waos.wapk"
        );
        assert_eq!(string(&exported[0], "UTTypeDescription"), "WaOS program");
        let tags = exported[0]["UTTypeTagSpecification"]
            .as_dictionary()
            .unwrap();
        assert_eq!(
            tags["public.filename-extension"],
            Value::Array(vec!["wapk".into()])
        );
        let document = &entries(&info, "CFBundleDocumentTypes")[0];
        assert_eq!(string(document, "LSHandlerRank"), "Owner");
        assert_eq!(string(document, "CFBundleTypeRole"), "Viewer");
        assert_eq!(
            document["LSItemContentTypes"],
            Value::Array(vec!["dev.wasdk.waos.wapk".into()])
        );
    }

    #[test]
    fn a_type_the_app_only_opens_is_imported_when_it_has_an_identifier() {
        let info = macos_info_plist(&sample_associations(), "dev.wasdk.waos");
        let imported = entries(&info, "UTImportedTypeDeclarations");
        assert_eq!(imported.len(), 1);
        assert_eq!(
            string(&imported[0], "UTTypeIdentifier"),
            "org.webassembly.wasm"
        );
        let tags = imported[0]["UTTypeTagSpecification"]
            .as_dictionary()
            .unwrap();
        assert_eq!(string(tags, "public.mime-type"), "application/wasm");
        let document = &entries(&info, "CFBundleDocumentTypes")[1];
        assert_eq!(string(document, "LSHandlerRank"), "Default");
    }

    #[test]
    fn a_type_the_app_only_opens_without_an_identifier_is_matched_by_its_extensions() {
        let info = macos_info_plist(&sample_associations(), "dev.wasdk.waos");
        let document = &entries(&info, "CFBundleDocumentTypes")[2];
        assert_eq!(
            document["CFBundleTypeExtensions"],
            Value::Array(vec!["txt".into()])
        );
        assert!(!document.contains_key("LSItemContentTypes"));
        assert_eq!(string(document, "CFBundleTypeRole"), "Editor");
    }

    #[test]
    fn every_association_reaches_cargo_packager_for_the_other_desktops() {
        let packaged = packager_associations(&sample_associations());
        assert_eq!(packaged.len(), 3);
        assert_eq!(packaged[1].extensions, vec!["wasm".to_owned()]);
        assert_eq!(packaged[1].mime_type.as_deref(), Some("application/wasm"));
        assert_eq!(
            packaged[1].description.as_deref(),
            Some("WebAssembly module")
        );
        assert_eq!(packaged[0].role, BundleTypeRole::Viewer);
    }

    #[test]
    fn an_extension_with_its_dot_is_refused() {
        let associations: Vec<FileAssociation> = serde_json::from_value(serde_json::json!([
            { "extensions": [".wapk"], "name": "WaOS program" },
        ]))
        .unwrap();
        assert!(check(&associations).is_err());
    }
}

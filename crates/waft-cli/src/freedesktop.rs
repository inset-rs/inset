//! What a Linux package tells the desktop about the app, as freedesktop.org's specifications have
//! it: its desktop entry, which the launcher and the dock show it by and which names the types of
//! file it opens, and the shared-mime-info of the types it owns, which teaches the system each
//! type's name and the file names it matches.
//!
//! The invariant: the entry's `StartupWMClass` is the app's identifier, the name Waft gives each
//! of its windows (the Wayland app id, the X11 class), and a deb's entry is named after it too, so
//! the desktop matches a window to its app; `waft-macros` gives the windows that name by the same
//! rule as `project` does here.
//!
//! cargo-packager writes neither: its entry is named after the executable with no window class,
//! and it declares no type. A package lists both among its files, and the databases built from
//! them are the system's to update: Debian's shared-mime-info and desktop-file-utils do it in the
//! triggers they hold on `/usr/share/mime/packages` and `/usr/share/applications`, run whenever a
//! package installs or removes a file there.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::project::{FileAssociation, Project};

/// The folder a desktop entry is installed in, under the package's root.
const APPLICATIONS: &str = "usr/share/applications";

/// The folder a package's shared-mime-info is installed in, under the package's root.
const MIME_PACKAGES: &str = "usr/share/mime/packages";

/// Where a package's freedesktop files are staged in `out`, the package's output folder, before
/// cargo-packager copies them in.
pub fn staging_dir(out: &Path) -> PathBuf {
    out.join("freedesktop")
}

/// Writes, under `root` at the paths they are installed at, the app's desktop entry, named
/// `entry_name`, and the shared-mime-info of the types it owns where it owns any.
pub fn write(project: &Project, entry_name: &str, root: &Path) -> Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    write_file(
        &root.join(APPLICATIONS).join(entry_name),
        &desktop_entry(project),
    )?;
    if let Some(mime_info) = mime_info(&project.file_associations) {
        let name = format!("{}.xml", project.identifier);
        write_file(&root.join(MIME_PACKAGES).join(name), &mime_info)?;
    }
    for association in owned_without_mime_type(&project.file_associations) {
        eprintln!(
            "note: file association `{}` is the app's own but has no `mime_type`: Linux does not associate it",
            association.name
        );
    }
    Ok(())
}

/// The name a deb's desktop entry is installed under: the app's identifier, as the Desktop Entry
/// specification has an entry's id for an app named in reverse DNS.
pub fn deb_entry_name(project: &Project) -> String {
    format!("{}.desktop", project.identifier)
}

fn write_file(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

/// The app's desktop entry: its name, description and icon, the executable started with the
/// files it is to open where it opens any, its windows' class, and the MIME types it opens.
fn desktop_entry(project: &Project) -> String {
    let mut entry = String::from("[Desktop Entry]\nType=Application\n");
    entry += &format!("Name={}\n", entry_string(&project.name));
    if let Some(description) = &project.description {
        entry += &format!("Comment={}\n", entry_string(description));
    }
    let program = exec_argument(&project.bin);
    let exec = if project.file_associations.is_empty() {
        program
    } else {
        format!("{program} %F")
    };
    entry += &format!("Exec={exec}\n");
    // cargo-packager installs the icons under the executable's name.
    entry += &format!("Icon={}\n", entry_string(&project.bin));
    entry += "Terminal=false\n";
    entry += &format!("StartupWMClass={}\n", entry_string(&project.identifier));
    let mime_types: Vec<&str> = project
        .file_associations
        .iter()
        .filter_map(|association| association.mime_type.as_deref())
        .collect();
    if !mime_types.is_empty() {
        let listed: String = mime_types
            .iter()
            .map(|mime_type| format!("{};", entry_string(mime_type)))
            .collect();
        entry += &format!("MimeType={listed}\n");
    }
    entry
}

/// `text` as a desktop entry's string value: a backslash, a line break, a tab or a carriage return
/// escaped, and a leading space written `\s`.
fn entry_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r");
    match escaped.strip_prefix(' ') {
        Some(rest) => format!("\\s{rest}"),
        None => escaped,
    }
}

/// `program` as an argument of `Exec`: as it is, or quoted where it holds a character the
/// specification reserves, with `"`, `` ` ``, `$` and `\` escaped inside the quotes and every
/// backslash then escaped again, as any string value's is.
fn exec_argument(program: &str) -> String {
    const RESERVED: &str = " \t\n\"'\\><~|&;$*?#()`";
    if !program.chars().any(|c| RESERVED.contains(c)) {
        return program.to_owned();
    }
    let mut quoted = String::from("\"");
    for c in program.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    entry_string(&quoted)
}

/// The shared-mime-info declaring each type the app owns that has a MIME type: its name and the
/// file names it matches. `None` where the app owns no such type.
fn mime_info(associations: &[FileAssociation]) -> Option<String> {
    let declared: Vec<String> = associations
        .iter()
        .filter(|association| association.owner)
        .filter_map(|association| {
            let mime_type = association.mime_type.as_deref()?;
            Some(mime_type_element(association, mime_type))
        })
        .collect();
    if declared.is_empty() {
        return None;
    }
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n\
         {}</mime-info>\n",
        declared.concat()
    ))
}

/// One `<mime-type>`: the type, what the file manager calls a file of it, and a glob per
/// extension.
fn mime_type_element(association: &FileAssociation, mime_type: &str) -> String {
    let globs: String = association
        .extensions
        .iter()
        .map(|extension| format!("    <glob pattern=\"*.{}\"/>\n", xml_text(extension)))
        .collect();
    format!(
        "  <mime-type type=\"{}\">\n    <comment>{}</comment>\n{globs}  </mime-type>\n",
        xml_text(mime_type),
        xml_text(&association.name),
    )
}

/// The types the app owns that the system cannot be told of, having no MIME type to name them by.
fn owned_without_mime_type(
    associations: &[FileAssociation],
) -> impl Iterator<Item = &FileAssociation> {
    associations
        .iter()
        .filter(|association| association.owner && association.mime_type.is_none())
}

/// `text` as XML character data or an attribute's value.
fn xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn associations() -> Vec<FileAssociation> {
        serde_json::from_value(serde_json::json!([
            {
                "extensions": ["wapk"],
                "name": "WaOS program",
                "owner": true,
                "mime_type": "application/x-wapk",
            },
            { "extensions": ["mine"], "name": "Mine & only", "owner": true },
            { "extensions": ["md"], "name": "Markdown", "mime_type": "text/markdown" },
        ]))
        .unwrap()
    }

    #[test]
    fn only_the_owned_types_with_a_mime_type_are_declared() {
        let info = mime_info(&associations()).unwrap();
        assert!(info.contains("<mime-type type=\"application/x-wapk\">"));
        assert!(info.contains("<comment>WaOS program</comment>"));
        assert!(info.contains("<glob pattern=\"*.wapk\"/>"));
        assert!(!info.contains("Mine"));
        assert!(!info.contains("text/markdown"));
    }

    #[test]
    fn an_app_that_owns_no_declared_type_has_no_mime_info() {
        assert!(mime_info(&associations()[1..]).is_none());
    }

    fn project() -> Project {
        Project {
            root: PathBuf::from("/tmp/x"),
            workspace_root: PathBuf::from("/tmp/x"),
            target_dir: PathBuf::from("/tmp/x/target"),
            package: "waos-app".into(),
            bin: "waos".into(),
            library: None,
            version: "0.1.0".into(),
            description: Some("Runs programs.".into()),
            homepage: None,
            authors: Vec::new(),
            name: "WaOS".into(),
            identifier: "dev.waos.app".into(),
            icon: None,
            resources: Vec::new(),
            file_associations: associations(),
            usage_descriptions: Default::default(),
            macos: Default::default(),
            ios: Default::default(),
            android: Default::default(),
        }
    }

    #[test]
    fn the_entry_names_the_windows_class_and_every_mime_type_it_opens() {
        let entry = desktop_entry(&project());
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\n"));
        assert!(entry.contains("\nName=WaOS\n"));
        assert!(entry.contains("\nComment=Runs programs.\n"));
        assert!(entry.contains("\nExec=waos %F\n"));
        assert!(entry.contains("\nIcon=waos\n"));
        assert!(entry.contains("\nStartupWMClass=dev.waos.app\n"));
        assert!(entry.contains("\nMimeType=application/x-wapk;text/markdown;\n"));
        assert_eq!(deb_entry_name(&project()), "dev.waos.app.desktop");
    }

    #[test]
    fn an_app_that_opens_no_file_is_started_with_none() {
        let mut project = project();
        project.file_associations.clear();
        let entry = desktop_entry(&project);
        assert!(entry.contains("\nExec=waos\n"));
        assert!(!entry.contains("MimeType"));
    }

    #[test]
    fn a_program_with_a_reserved_character_is_quoted_and_escaped() {
        assert_eq!(exec_argument("waos"), "waos");
        assert_eq!(exec_argument("my app"), "\"my app\"");
        assert_eq!(exec_argument("a$b"), "\"a\\\\$b\"");
    }

    #[test]
    fn a_value_keeps_its_leading_space_and_its_backslashes() {
        assert_eq!(entry_string(" WaOS\\1"), "\\sWaOS\\\\1");
    }
}

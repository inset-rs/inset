//! Opening, and being opened. Not a dart:ui file: Flutter reaches the first through its
//! `url_launcher` package, one call taking a URI, and the second through each platform's plugins.
//!
//! Opening is a call for each kind, as Electron's `shell` and Tauri's opener have them: a link in
//! the app that handles its scheme, a file in the app the system opens its kind with, and a file
//! shown in the system's file manager ([`Opener`]). A file is no `file://` link here: phones share
//! a file through the system (Android's content provider, iOS's interaction controller), and only
//! a file can be revealed. Whether some app opens a link is asked where the system answers it
//! ([`Opener::can_open_link`]); Apple and Google limit the question to the schemes an app
//! declared, since asking about many tells which apps the user has.
//!
//! Being opened: the system asks the app to open files and links, at launch or while it runs, as
//! one [`OpenRequest`] reaching the app through `EmbedderClient::open_requested`. A file is a
//! [`PickedFile`], as a file dialog answers one: a path where the system gives one, and the
//! system's handle where it gives none.

use std::ffi::OsString;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use crate::PickedFile;

/// The system's way to open links and files in other apps.
pub trait Opener {
    /// Opens `link` in the app that handles its scheme: a browser for `https`, the mail app for
    /// `mailto`.
    fn open_link(&self, link: &str) -> OpenFuture;

    /// Opens the file at `path` in the app the system opens its kind with.
    fn open_file(&self, path: &Path) -> OpenFuture;

    /// Shows the file at `path` in the system's file manager, selected in its folder.
    fn reveal_file(&self, path: &Path) -> OpenFuture;

    /// Whether some app on this system opens `link`; `None` where the system will not say, as
    /// for a scheme an app did not declare on iOS and Android, and anywhere on Linux and the web.
    fn can_open_link(&self, link: &str) -> Option<bool>;
}

/// What opening answers, once the system has handed the link or the file on.
pub type OpenFuture = Pin<Box<dyn Future<Output = Result<(), OpenError>>>>;

/// Why a link or a file did not open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// No app on this system opens it.
    NoApp,
    /// The system failed to open it.
    Failed(String),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::NoApp => f.write_str("no app opens it"),
            OpenError::Failed(reason) => write!(f, "it did not open: {reason}"),
        }
    }
}

impl std::error::Error for OpenError {}

/// What the system asks the app to open, at launch or while it runs.
#[derive(Debug, Default)]
pub struct OpenRequest {
    pub files: Vec<PickedFile>,
    pub links: Vec<String>,
}

impl OpenRequest {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.links.is_empty()
    }

    /// A launch's arguments as what they ask to open, as Windows and Linux start an app with its
    /// files and links on its command line: an argument with a URI scheme of two letters or more
    /// (`https:`, `mailto:`) is a link, a `file:` URI is the file it names, and any other argument
    /// is a file, made absolute against `current_dir`. An argument starting with `-` is an option
    /// of the app's own, and left out.
    pub fn from_arguments(
        arguments: impl IntoIterator<Item = OsString>,
        current_dir: &Path,
    ) -> OpenRequest {
        let mut request = OpenRequest::default();
        for argument in arguments {
            match argument.to_str() {
                Some(text) if text.starts_with('-') => {}
                Some(text) if text.starts_with("file:") => {
                    request
                        .files
                        .extend(file_uri_path(text).map(PickedFile::Path));
                }
                Some(text) if has_scheme(text) => request.links.push(text.to_owned()),
                _ => request
                    .files
                    .push(PickedFile::Path(current_dir.join(argument))),
            }
        }
        request
    }
}

/// Whether `text` starts with a URI scheme of two letters or more, so a Windows drive (`C:`) is
/// none.
fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    scheme.len() >= 2
        && scheme.starts_with(|first: char| first.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '+' | '-' | '.'))
}

/// The path a `file:` URI names on this system, its percent-encoding decoded; `None` for another
/// URI, one on another host, or one that does not decode.
pub fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = match rest.find('/')? {
        0 => rest,
        host_end if &rest[..host_end] == "localhost" => &rest[host_end..],
        _ => return None,
    };
    let mut bytes = Vec::with_capacity(path.len());
    let mut encoded = path.bytes();
    while let Some(byte) = encoded.next() {
        if byte == b'%' {
            let high = encoded.next().and_then(hex_digit)?;
            let low = encoded.next().and_then(hex_digit)?;
            bytes.push(high << 4 | low);
        } else {
            bytes.push(byte);
        }
    }
    path_from_bytes(bytes)
}

fn hex_digit(digit: u8) -> Option<u8> {
    char::from(digit).to_digit(16).map(|value| value as u8)
}

/// A decoded URI path as this system's path: its bytes on Unix; on Windows UTF-8, with the slash
/// before a drive dropped and the slashes turned around.
#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: Vec<u8>) -> Option<PathBuf> {
    let text = String::from_utf8(bytes).ok()?;
    let has_drive = text.len() > 2
        && text.as_bytes()[0] == b'/'
        && text.as_bytes()[1].is_ascii_alphabetic()
        && text.as_bytes()[2] == b':';
    let text = if has_drive { &text[1..] } else { &text[..] };
    Some(PathBuf::from(text.replace('/', "\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(request: &OpenRequest) -> Vec<PathBuf> {
        request
            .files
            .iter()
            .filter_map(|file| match file {
                PickedFile::Path(path) => Some(path.clone()),
                PickedFile::Handle(_) | PickedFile::Stream(_) => None,
            })
            .collect()
    }

    #[test]
    fn arguments_are_links_files_or_the_apps_own_options() {
        let request = OpenRequest::from_arguments(
            [
                "--verbose",
                "notes.txt",
                "https://example.org/a",
                "mailto:?subject=notes",
                "/tmp/b.md",
            ]
            .map(OsString::from),
            Path::new("/home/me"),
        );
        assert_eq!(
            request.links,
            ["https://example.org/a", "mailto:?subject=notes"]
        );
        assert_eq!(
            paths(&request),
            [
                PathBuf::from("/home/me/notes.txt"),
                PathBuf::from("/tmp/b.md")
            ]
        );
        assert!(OpenRequest::from_arguments([], Path::new("/")).is_empty());
    }

    #[test]
    fn a_drive_is_no_scheme() {
        assert!(!has_scheme(r"C:\Users\me\a.txt"));
        assert!(has_scheme("web+inset:open"));
        assert!(!has_scheme("notes.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn a_file_uri_names_its_path_decoded() {
        assert_eq!(
            file_uri_path("file:///home/me/My%20Notes/caf%C3%A9.txt"),
            Some(PathBuf::from("/home/me/My Notes/café.txt"))
        );
        assert_eq!(
            file_uri_path("file://localhost/tmp/a"),
            Some(PathBuf::from("/tmp/a"))
        );
        assert_eq!(file_uri_path("file://elsewhere/tmp/a"), None);
        assert_eq!(file_uri_path("https://example.org/a"), None);
        assert_eq!(file_uri_path("file:///bad%2"), None);
        let request =
            OpenRequest::from_arguments([OsString::from("file:///tmp/c%20d.txt")], Path::new("/"));
        assert_eq!(paths(&request), [PathBuf::from("/tmp/c d.txt")]);
    }
}

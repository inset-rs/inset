//! Linux: D-Bus activation's way, as GTK's and Qt's single-instance apps have it. The first
//! instance owns the app's id as its name on the session bus and serves
//! `org.freedesktop.Application` at the path the id names; a later launch, finding the name owned,
//! calls `Open` with its files as `file://` URIs and its links, or `Activate` with nothing to
//! open, and quits. The object is served before the name is owned, so a later launch never calls
//! an instance that cannot answer yet.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{OpenRequest, PickedFile};
use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::interface;
use zbus::names::WellKnownName;
use zbus::zvariant::{OwnedValue, Value};

use crate::{Arrivals, FirstInstance, SingleInstance, this_launch};

const APPLICATION: &str = "org.freedesktop.Application";
/// The longest a call to the bus may take.
const PATIENCE: Duration = Duration::from_secs(3);

pub(crate) fn claim(id: &str) -> io::Result<SingleInstance> {
    let failed = |error: zbus::Error| io::Error::other(error.to_string());
    let name = WellKnownName::try_from(id).map_err(|error| failed(error.into()))?;
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(failed)?
        .method_timeout(PATIENCE)
        .build()
        .map_err(failed)?;
    let path = object_path(id);
    let arrivals = Arc::new(Arrivals::default());
    connection
        .object_server()
        .at(
            path.as_str(),
            Application {
                arrivals: Arc::clone(&arrivals),
            },
        )
        .map_err(failed)?;
    let reply = DBusProxy::new(&connection)
        .map_err(failed)?
        .request_name(name, RequestNameFlags::DoNotQueue.into())
        .map_err(|error| failed(error.into()))?;
    match reply {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(
            SingleInstance::First(FirstInstance::new(arrivals, Box::new(connection))),
        ),
        RequestNameReply::Exists | RequestNameReply::InQueue => {
            hand_over(&connection, id, &path).map_err(failed)?;
            Ok(SingleInstance::Second)
        }
    }
}

/// The path D-Bus activation serves an app's object at: its id with the dots as slashes and the
/// dashes as underscores.
fn object_path(id: &str) -> String {
    format!("/{}", id.replace('.', "/").replace('-', "_"))
}

/// Hands this launch's files and links to the first instance, or asks it to show itself.
fn hand_over(connection: &Connection, id: &str, path: &str) -> zbus::Result<()> {
    let (arguments, current_dir) = this_launch();
    let uris = uris(OpenRequest::from_arguments(arguments, &current_dir));
    let platform_data: HashMap<&str, Value<'_>> = HashMap::new();
    if uris.is_empty() {
        connection.call_method(
            Some(id),
            path,
            Some(APPLICATION),
            "Activate",
            &(platform_data,),
        )?;
    } else {
        connection.call_method(
            Some(id),
            path,
            Some(APPLICATION),
            "Open",
            &(uris, platform_data),
        )?;
    }
    Ok(())
}

/// A request's files as `file://` URIs, then its links.
fn uris(request: OpenRequest) -> Vec<String> {
    let files = request.files.into_iter().filter_map(|file| match file {
        PickedFile::Path(path) => Some(file_uri(&path)),
        PickedFile::Handle(_) | PickedFile::Stream(_) => None,
    });
    files.chain(request.links).collect()
}

/// `path` as a `file://` URI, every byte but the unreserved ones and the slashes encoded.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// The first instance's `org.freedesktop.Application`.
struct Application {
    arrivals: Arc<Arrivals>,
}

#[interface(name = "org.freedesktop.Application")]
impl Application {
    /// A later launch with nothing to open: the app shows itself again.
    fn activate(&self, _platform_data: HashMap<String, OwnedValue>) {
        self.arrivals.arrive(Vec::new(), PathBuf::from("/"));
    }

    fn open(&self, uris: Vec<String>, _platform_data: HashMap<String, OwnedValue>) {
        let arguments = uris.into_iter().map(OsString::from).collect();
        self.arrivals.arrive(arguments, PathBuf::from("/"));
    }

    /// The app has no actions of its own here: an action shows it again.
    fn activate_action(
        &self,
        _action_name: String,
        _parameter: Vec<OwnedValue>,
        _platform_data: HashMap<String, OwnedValue>,
    ) {
        self.arrivals.arrive(Vec::new(), PathBuf::from("/"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_names_its_object_and_a_file_its_uri() {
        assert_eq!(object_path("com.example.my-notes"), "/com/example/my_notes");
        assert_eq!(
            file_uri(Path::new("/home/me/My Notes/café.txt")),
            "file:///home/me/My%20Notes/caf%C3%A9.txt"
        );
        let request = OpenRequest::from_arguments(
            [OsString::from(file_uri(Path::new("/tmp/a b")))],
            Path::new("/"),
        );
        assert!(
            matches!(&request.files[..], [PickedFile::Path(path)] if path == Path::new("/tmp/a b"))
        );
    }
}

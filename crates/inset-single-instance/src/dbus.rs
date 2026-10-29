//! Linux: the session bus. The first instance owns the app's id as its name on the bus and serves
//! `rs.inset.SingleInstance` at the path the id names, as D-Bus activated apps serve
//! `org.freedesktop.Application` there; a later launch, finding the name owned, calls its `Launch`
//! with its arguments and folder, each as the bytes the system gave, and quits. The object is
//! served before the name is owned, so a later launch never calls an instance that cannot answer
//! yet.
//!
//! `org.freedesktop.Application`'s `Open` takes URIs, which would make every argument a file: a
//! launch's arguments are the app's to read, so they go through an interface of Inset's own.

use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::interface;
use zbus::names::WellKnownName;

use crate::{Arrivals, FirstInstance, SingleInstance, this_launch};

const SINGLE_INSTANCE: &str = "rs.inset.SingleInstance";
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
            Instance {
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

/// Hands this launch's arguments and folder to the first instance.
fn hand_over(connection: &Connection, id: &str, path: &str) -> zbus::Result<()> {
    let (arguments, current_dir) = this_launch();
    let arguments: Vec<&[u8]> = arguments
        .iter()
        .map(|argument| argument.as_bytes())
        .collect();
    connection.call_method(
        Some(id),
        path,
        Some(SINGLE_INSTANCE),
        "Launch",
        &(arguments, current_dir.as_os_str().as_bytes()),
    )?;
    Ok(())
}

/// The first instance's `rs.inset.SingleInstance`.
struct Instance {
    arrivals: Arc<Arrivals>,
}

#[interface(name = "rs.inset.SingleInstance")]
impl Instance {
    /// A later launch, with its arguments and the folder it was started in.
    fn launch(&self, arguments: Vec<Vec<u8>>, current_dir: Vec<u8>) {
        let arguments = arguments.into_iter().map(OsString::from_vec).collect();
        self.arrivals
            .arrive(arguments, PathBuf::from(OsString::from_vec(current_dir)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_names_its_object() {
        assert_eq!(object_path("com.example.my-notes"), "/com/example/my_notes");
    }

    #[test]
    #[ignore = "needs a session bus of its own: dbus-run-session -- cargo test -- --ignored"]
    fn a_later_launch_hands_its_arguments_over_unread() {
        let id = "rs.inset.SingleInstanceTest";
        let Ok(SingleInstance::First(first)) = claim(id) else {
            panic!("the first claim is the first instance");
        };
        let connection = zbus::blocking::connection::Builder::session()
            .and_then(|builder| builder.build())
            .expect("a second connection");
        let arguments: Vec<&[u8]> = vec![b"phone", b"--verbose", b"caf\xC3\xA9 notes.txt"];
        connection
            .call_method(
                Some(id),
                object_path(id).as_str(),
                Some(SINGLE_INSTANCE),
                "Launch",
                &(arguments, b"/home/me".as_slice()),
            )
            .expect("the first instance answers");
        let mut launches = first.launches();
        let launch = crate::tests::now(launches.next_launch()).expect("the launch arrived");
        assert_eq!(
            launch.arguments,
            ["phone", "--verbose", "café notes.txt"].map(OsString::from)
        );
        assert_eq!(launch.current_dir, PathBuf::from("/home/me"));
    }
}

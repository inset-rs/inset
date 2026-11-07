//! The session bus, as the desktop's services are reached on it: a connection whose calls wait no
//! longer than [`PATIENCE`], and whether a service is there to call, running or one the bus starts
//! when asked.

use std::time::Duration;

use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::BusName;

/// The longest a call to the bus may take.
pub(super) const PATIENCE: Duration = Duration::from_secs(5);

/// A connection of its own to the session bus.
pub(super) fn connect() -> Option<Connection> {
    zbus::blocking::connection::Builder::session()
        .ok()?
        .method_timeout(PATIENCE)
        .build()
        .ok()
}

/// Whether the session bus has the service `name`, running or one it starts when asked.
pub(super) fn has_service(name: &str) -> bool {
    let Some(connection) = connect() else {
        return false;
    };
    let Ok(bus) = DBusProxy::new(&connection) else {
        return false;
    };
    let Ok(service) = BusName::try_from(name) else {
        return false;
    };
    bus.name_has_owner(service).unwrap_or(false)
        || bus
            .list_activatable_names()
            .is_ok_and(|names| names.iter().any(|activatable| activatable.as_str() == name))
}

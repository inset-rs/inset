//! The desktop portal (`org.freedesktop.portal.Desktop`), which the desktop answers for an app on
//! GNOME, KDE and in a Flatpak alike: whether the session bus has it, and a request's round, the
//! call made with a token and the `Response` the portal sends at the request's own path once the
//! user is done, awaited on the asking thread. The response is listened for before the request is
//! made, so it cannot come before the listening.
//!
//! A request the app stops waiting for is closed through [`Closer`], `org.freedesktop.portal.
//! Request.Close` at the request's path, from whatever thread the app drops its future on.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use zbus::blocking::Connection;
use zbus::blocking::Proxy;
use zbus::export::serde::Serialize;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{DynamicType, OwnedValue, Value};

use super::session_bus;

pub(super) const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const REQUEST: &str = "org.freedesktop.portal.Request";

/// Whether the session bus has the desktop portal, running or one it starts when asked; a call to
/// it waits no longer than the bus's patience, and the wait for the user's answer has no limit.
pub(super) fn is_on_the_bus() -> bool {
    session_bus::has_service(PORTAL)
}

/// What the portal answered: `0` done, `1` cancelled by the user, `2` ended otherwise; and its
/// results.
pub(super) struct Response {
    pub(super) code: u32,
    pub(super) results: HashMap<String, OwnedValue>,
}

/// A token each request is told by.
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

/// Calls `method` of the portal's `interface` with the body `body` makes of `options`, the
/// request's token added to them, and waits for the portal's response. `close` closes the request
/// while the portal waits for its user.
pub(super) fn request<B>(
    interface: &str,
    method: &str,
    mut options: HashMap<&'static str, OwnedValue>,
    body: impl FnOnce(HashMap<&'static str, OwnedValue>) -> B,
    close: &Closer,
) -> zbus::Result<Response>
where
    B: Serialize + DynamicType,
{
    let connection = session_bus::connect()
        .ok_or_else(|| zbus::Error::Failure("the session bus could not be reached".to_owned()))?;
    let token = format!("waft{}", NEXT_TOKEN.fetch_add(1, Ordering::Relaxed));
    let sender = connection
        .unique_name()
        .map(|name| name.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let path = format!("{PORTAL_PATH}/request/{sender}/{token}");
    let request: Proxy<'_> = zbus::blocking::proxy::Builder::new(&connection)
        .destination(PORTAL)?
        .path(path.clone())?
        .interface(REQUEST)?
        .cache_properties(CacheProperties::No)
        .build()?;
    let mut responses = request.receive_signal("Response")?;
    options.insert("handle_token", owned(token));
    connection.call_method(
        Some(PORTAL),
        PORTAL_PATH,
        Some(interface),
        method,
        &body(options),
    )?;
    close.made(&connection, &path);
    let response = responses
        .next()
        .ok_or_else(|| zbus::Error::Failure("the portal closed without an answer".to_owned()))?;
    close.done();
    let (code, results) = response.body().deserialize()?;
    Ok(Response { code, results })
}

/// The way a request the app stops waiting for is closed: `close` ends it while the portal still
/// waits, from any thread, once the request is made; one closed before it is made closes as soon
/// as it is.
#[derive(Clone)]
pub(super) struct Closer(Arc<Mutex<Closing>>);

/// What a [`Closer`] knows of its request.
enum Closing {
    /// Not made yet.
    Waiting,
    /// Not made yet, and closed as soon as it is.
    ClosingEarly,
    /// Made, over the connection whose call closes it, at its path.
    Made(Connection, String),
    /// Answered, or never made: nothing left to close.
    Done,
}

impl Closer {
    /// A closer for a request not made yet.
    pub(super) fn new() -> Closer {
        Closer(Arc::new(Mutex::new(Closing::Waiting)))
    }

    /// Notes the request at `path` made over `connection`, closing it at once if the app asked
    /// for that before it was made.
    fn made(&self, connection: &Connection, path: &str) {
        let close_early = {
            let mut closing = locked(&self.0);
            match &*closing {
                Closing::Waiting => {
                    *closing = Closing::Made(connection.clone(), path.to_owned());
                    false
                }
                Closing::ClosingEarly => {
                    *closing = Closing::Done;
                    true
                }
                Closing::Made(..) | Closing::Done => false,
            }
        };
        if close_early {
            let _ = close_request(connection, path);
        }
    }

    /// Ends the request unless it has answered already, or as soon as it is made; from any
    /// thread.
    pub(super) fn close(&self) {
        let made = {
            let mut closing = locked(&self.0);
            match &mut *closing {
                Closing::Waiting => {
                    *closing = Closing::ClosingEarly;
                    None
                }
                Closing::Made(connection, path) => {
                    let made = (connection.clone(), std::mem::take(path));
                    *closing = Closing::Done;
                    Some(made)
                }
                Closing::ClosingEarly | Closing::Done => None,
            }
        };
        if let Some((connection, path)) = made {
            let _ = close_request(&connection, &path);
        }
    }

    /// Notes the request answered: nothing is left to close.
    fn done(&self) {
        *locked(&self.0) = Closing::Done;
    }
}

/// Asks the portal to close the request at `path`, as its Cancel button does.
fn close_request(connection: &Connection, path: &str) -> zbus::Result<()> {
    let request: Proxy<'_> = zbus::blocking::proxy::Builder::new(connection)
        .destination(PORTAL)?
        .path(path)?
        .interface(REQUEST)?
        .cache_properties(CacheProperties::No)
        .build()?;
    request.call_method("Close", &())?;
    Ok(())
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `value` as an owned D-Bus value.
pub(super) fn owned<'v>(value: impl Into<Value<'v>>) -> OwnedValue {
    value
        .into()
        .try_into_owned()
        .expect("a value without file descriptors")
}

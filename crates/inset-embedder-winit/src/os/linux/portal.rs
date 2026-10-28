//! The desktop portal (`org.freedesktop.portal.Desktop`), which the desktop answers for an app on
//! GNOME, KDE and in a Flatpak alike: whether the session bus has it, and a request's round, the
//! call made with a token and the `Response` the portal sends at the request's own path once the
//! user is done, awaited on the asking thread. The response is listened for before the request is
//! made, so it cannot come before the listening.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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
/// request's token added to them, and waits for the portal's response.
pub(super) fn request<B>(
    interface: &str,
    method: &str,
    mut options: HashMap<&'static str, OwnedValue>,
    body: impl FnOnce(HashMap<&'static str, OwnedValue>) -> B,
) -> zbus::Result<Response>
where
    B: Serialize + DynamicType,
{
    let connection = session_bus::connect()
        .ok_or_else(|| zbus::Error::Failure("the session bus could not be reached".to_owned()))?;
    let token = format!("inset{}", NEXT_TOKEN.fetch_add(1, Ordering::Relaxed));
    let sender = connection
        .unique_name()
        .map(|name| name.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let request: Proxy<'_> = zbus::blocking::proxy::Builder::new(&connection)
        .destination(PORTAL)?
        .path(format!("{PORTAL_PATH}/request/{sender}/{token}"))?
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
    let response = responses
        .next()
        .ok_or_else(|| zbus::Error::Failure("the portal closed without an answer".to_owned()))?;
    let (code, results) = response.body().deserialize()?;
    Ok(Response { code, results })
}

/// `value` as an owned D-Bus value.
pub(super) fn owned<'v>(value: impl Into<Value<'v>>) -> OwnedValue {
    value
        .into()
        .try_into_owned()
        .expect("a value without file descriptors")
}

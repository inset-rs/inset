//! Clicks on the app's notifications, from wherever the system reports them, a thread of its own
//! included, to the main loop, which hands each to the client
//! (`EmbedderClient::notification_activated`) as a request of the system's. The system's
//! notifications themselves are each system's (`os/*/notifications`).
//!
//! A notification the app posts as another app, which Windows lets a process do
//! ([`Windows::notifications_of`](crate::Windows::notifications_of)), is that app's: its clicks
//! carry the app's identity, and go to the handler the app gave with it instead of the client.

use std::sync::Arc;

use waft_embedder::NotificationActivation;

use crate::main_loop::{MainLoopEvent, MainLoopProxy};
use crate::system_requests::SystemRequest;

/// Where a system's notifications send the user's clicks, from any thread, with the identity of the
/// app they were posted as; none for the app's own.
#[derive(Clone)]
pub(crate) struct NotificationActivations {
    send: Arc<dyn Fn(Option<String>, NotificationActivation) + Send + Sync>,
    /// The app the notifications that send here were posted as; `None` for the app's own.
    identity: Option<String>,
}

impl NotificationActivations {
    /// Clicks sent to the main loop through `proxy`.
    pub(crate) fn new(proxy: MainLoopProxy) -> NotificationActivations {
        NotificationActivations {
            send: Arc::new(move |identity, activation| {
                proxy.send(MainLoopEvent::SystemRequest(
                    SystemRequest::NotificationActivated {
                        identity,
                        activation,
                    },
                ));
            }),
            identity: None,
        }
    }

    /// Clicks handed to `send` with the identity they carry, as a test hears them.
    #[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
    pub(crate) fn to(
        send: impl Fn(Option<String>, NotificationActivation) + Send + Sync + 'static,
    ) -> NotificationActivations {
        NotificationActivations {
            send: Arc::new(send),
            identity: None,
        }
    }

    /// The clicks on notifications posted as the app `identity`, sent to the same place.
    #[cfg_attr(
        not(target_os = "windows"),
        allow(dead_code, reason = "only Windows posts as another app")
    )]
    pub(crate) fn of(&self, identity: &str) -> NotificationActivations {
        NotificationActivations {
            send: Arc::clone(&self.send),
            identity: Some(identity.to_owned()),
        }
    }

    #[cfg_attr(
        not(any(target_os = "macos", target_os = "windows", target_os = "linux")),
        allow(dead_code, reason = "a system without notifications reports no click")
    )]
    pub(crate) fn send(&self, activation: NotificationActivation) {
        (self.send)(self.identity.clone(), activation);
    }
}

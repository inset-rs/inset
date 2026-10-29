//! Clicks on the app's notifications, from wherever the system reports them, a thread of its own
//! included, to the main loop, which hands each to the client
//! (`EmbedderClient::notification_activated`) as a request of the system's. The system's
//! notifications themselves are each system's (`os/*/notifications`).

use std::sync::Arc;

use inset_embedder::NotificationActivation;

use crate::main_loop::{MainLoopEvent, MainLoopProxy};
use crate::system_requests::SystemRequest;

/// Where a system's notifications send the user's clicks, from any thread.
#[derive(Clone)]
pub(crate) struct NotificationActivations(Arc<dyn Fn(NotificationActivation) + Send + Sync>);

impl NotificationActivations {
    /// Clicks sent to the main loop through `proxy`.
    pub(crate) fn new(proxy: MainLoopProxy) -> NotificationActivations {
        NotificationActivations(Arc::new(move |activation| {
            proxy.send(MainLoopEvent::SystemRequest(
                SystemRequest::NotificationActivated(activation),
            ));
        }))
    }

    /// Clicks handed to `send`, as a test hears them.
    #[cfg(test)]
    pub(crate) fn to(
        send: impl Fn(NotificationActivation) + Send + Sync + 'static,
    ) -> NotificationActivations {
        NotificationActivations(Arc::new(send))
    }

    #[cfg_attr(
        not(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "windows",
            target_os = "linux"
        )),
        allow(dead_code, reason = "a system without notifications reports no click")
    )]
    pub(crate) fn send(&self, activation: NotificationActivation) {
        (self.0)(activation);
    }
}

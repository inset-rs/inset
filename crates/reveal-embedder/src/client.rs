//! Flutter counterpart: `dart:ui` `hooks.dart`.
//!
//! Dart's hooks are isolate-global functions the engine calls. Here they are
//! methods on the embedder client so a host crate need not name `App`.

use crate::{Frame, ViewId};

/// Complete frames and typed view lifecycle notifications.
///
/// The platform updates its view registry before sending a lifecycle method,
/// so the client can immediately query the affected view.
pub trait EmbedderClient {
    fn frame(&mut self, frame: Frame);
    fn view_added(&mut self, id: ViewId);
    fn view_metrics_changed(&mut self, id: ViewId);
    fn view_removed(&mut self, id: ViewId);
}

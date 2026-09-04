//! Flutter counterpart: `dart:ui` `hooks.dart`.
//!
//! Dart's hooks are isolate-global functions the engine calls. Here they are
//! methods on the embedder client so a host crate need not name `App`.

use std::time::Duration;

use crate::{Frame, PointerDataPacket, ViewId};

/// Complete frames and typed view lifecycle notifications.
///
/// The platform updates its view registry before sending a lifecycle method,
/// so the client can immediately query the affected view.
pub trait EmbedderClient {
    fn frame(&mut self, frame: Frame);
    fn view_added(&mut self, id: ViewId);
    fn view_metrics_changed(&mut self, id: ViewId);
    fn view_removed(&mut self, id: ViewId);

    /// Flutter `PlatformDispatcher.onPointerDataPacket`.
    fn pointer_data_packet(&mut self, packet: PointerDataPacket);

    /// A deadline requested through `Platform::wake_at` has passed.
    ///
    /// `elapsed` is the platform clock, the same one `Frame::elapsed` reports. Dart's
    /// event loop runs due `Timer`s itself; here the client advances its own clock.
    fn wake(&mut self, elapsed: Duration);
}

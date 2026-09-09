//! Flutter counterpart: `dart:ui` `hooks.dart`.
//!
//! Dart's hooks are isolate-global functions the engine calls. Here they are
//! methods on the embedder client so a host crate need not name `App`.

use std::time::Duration;

use crate::{
    Frame, KeyData, PointerDataPacket, TextEditingValue, TextInputAction, ViewFocusEvent, ViewId,
};

/// Complete frames and typed view lifecycle notifications.
///
/// The platform updates its view registry before sending a lifecycle method,
/// so the client can immediately query the affected view.
pub trait EmbedderClient {
    fn frame(&mut self, frame: Frame);
    fn view_added(&mut self, id: ViewId);
    fn view_metrics_changed(&mut self, id: ViewId);
    fn view_removed(&mut self, id: ViewId);

    /// Flutter `PlatformDispatcher.onViewFocusChange`.
    fn view_focus_changed(&mut self, event: ViewFocusEvent) {
        let _ = event;
    }

    /// Flutter `PlatformDispatcher.onPointerDataPacket`.
    fn pointer_data_packet(&mut self, packet: PointerDataPacket);

    /// Flutter `PlatformDispatcher.onKeyData`.
    ///
    /// Returns whether the framework handled the event; a host that shares the
    /// keyboard with other native components should not propagate a handled one.
    fn key_data(&mut self, data: KeyData) -> bool;

    /// Flutter `PlatformDispatcher.onPlatformBrightnessChanged`: the host's light or dark
    /// preference changed, and `Platform::platform_brightness` already answers the new one.
    fn platform_brightness_changed(&mut self);

    /// Flutter `PlatformDispatcher.onLocaleChanged`: the host's locale list changed, and
    /// `Platform::locales` already answers the new one.
    fn locales_changed(&mut self);

    /// A deadline requested through `Platform::wake_at` has passed.
    ///
    /// `elapsed` is the platform clock, the same one `Frame::elapsed` reports. Dart's
    /// event loop runs due `Timer`s itself; here the client advances its own clock.
    fn wake(&mut self, elapsed: Duration);

    /// Flutter `TextInputClient.updateEditingValue`. The default drops it.
    fn text_input_editing_value(&mut self, view: ViewId, value: TextEditingValue) {
        let _ = (view, value);
    }

    /// Flutter `TextInputClient.performAction`. The default drops it.
    fn text_input_action(&mut self, view: ViewId, action: TextInputAction) {
        let _ = (view, action);
    }

    /// Flutter `TextInputClient.connectionClosed`. The default drops it.
    fn text_input_closed(&mut self, view: ViewId) {
        let _ = view;
    }

    /// Flutter `SystemContextMenuClient.handleSystemHide`. The default drops it.
    fn system_context_menu_hidden(&mut self) {}

    /// Flutter `SystemContextMenuClient.handleCustomContextMenuAction`. The default drops it.
    fn custom_context_menu_action(&mut self, callback_id: &str) {
        let _ = callback_id;
    }
}

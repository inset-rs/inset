//! Host capabilities the framework calls at runtime.
//!
//! Unlike Dart's isolate-global `PlatformDispatcher`, this object is supplied
//! by the embedder and held by `App`. Incoming host events travel through
//! `EmbedderClient`, so this interface contains requests and state only.

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::mouse_cursor::SystemMouseCursorKind;
use crate::{View, ViewId};

pub type PlatformRef = Rc<dyn Platform>;
pub type ViewRef = Rc<dyn View>;

/// Describes the contrast of a theme or color palette.
///
/// Flutter counterpart: `Brightness` (`dart:ui` `window.dart`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brightness {
    /// The color is dark and will require a light text color to achieve readable
    /// contrast.
    ///
    /// For example, the color might be dark grey, requiring white text.
    Dark,

    /// The color is light and will require a dark text color to achieve readable
    /// contrast.
    ///
    /// For example, the color might be bright white, requiring black text.
    Light,
}

/// The platform that user interaction should adapt to target.
///
/// Flutter counterpart: `TargetPlatform` (`foundation/platform.dart`). Flutter
/// reads the current one from a library global that consults `dart:io`. Here it
/// is a plain value type, and the App-owned [`Platform`] reports which one it
/// is, so two Apps in one process can differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetPlatform {
    /// Android: <https://www.android.com/>
    Android,
    /// Fuchsia: <https://fuchsia.dev/fuchsia-src/concepts>
    Fuchsia,
    /// iOS: <https://www.apple.com/ios/>
    IOS,
    /// Linux: <https://www.linux.org>
    Linux,
    /// macOS: <https://www.apple.com/macos>
    MacOS,
    /// Windows: <https://www.windows.com>
    Windows,
}

/// The long-lived host object held by the application.
///
/// Implementations must queue requests and return. They must not synchronously
/// re-enter the [`EmbedderClient`](crate::EmbedderClient) while `App` is active.
pub trait Platform: 'static {
    /// Which host this is, for behaviour that follows platform convention
    /// (Flutter `defaultTargetPlatform`).
    ///
    /// Required rather than defaulted: an embedder must say what it is, and a
    /// default would let one silently claim the wrong conventions.
    fn target_platform(&self) -> TargetPlatform;

    /// The platform's light/dark preference (Flutter
    /// `PlatformDispatcher.platformBrightness`).
    ///
    /// Defaults to [`Brightness::Light`], matching Flutter's view configuration
    /// default. A live host that can see the OS theme overrides this.
    fn platform_brightness(&self) -> Brightness {
        Brightness::Light
    }

    /// Requests one isolate frame at the host's next appropriate opportunity.
    fn request_frame(&self);

    /// The host clock used for frame and timer timestamps.
    fn now(&self) -> Instant;

    /// Asks the host to wake the application at `deadline`.
    fn wake_at(&self, deadline: Instant);

    /// Current host-provided views.
    fn views(&self) -> Vec<ViewRef>;

    /// Looks up a current host-provided view.
    fn view(&self, id: ViewId) -> Option<ViewRef>;

    /// The stable implicit view, when this embedding provides one.
    fn implicit_view(&self) -> Option<ViewRef>;

    /// Shows a system cursor for a pointing device (Flutter's `activateSystemCursor`
    /// message on `SystemChannels.mouseCursor`).
    ///
    /// Defaults to nothing: a host without a system cursor ignores the request.
    fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
        let _ = (device, kind);
    }
}

/// Platform for hand-pumped tests.
pub struct InertPlatform;

impl Platform for InertPlatform {
    /// Android, which is what Flutter's own test binding defaults
    /// `debugDefaultTargetPlatform` to: a bare App has no host to speak of,
    /// and a deterministic answer beats the machine's.
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::Android
    }

    fn request_frame(&self) {}

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wake_at(&self, _deadline: Instant) {}

    fn views(&self) -> Vec<ViewRef> {
        Vec::new()
    }

    fn view(&self, _id: ViewId) -> Option<ViewRef> {
        None
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        None
    }
}

/// One complete engine frame delivered to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::{Brightness, InertPlatform, Platform, TargetPlatform};

    #[test]
    fn inert_platform_is_android() {
        assert_eq!(InertPlatform.target_platform(), TargetPlatform::Android);
    }

    #[test]
    fn inert_platform_brightness_is_light() {
        assert_eq!(InertPlatform.platform_brightness(), Brightness::Light);
    }
}

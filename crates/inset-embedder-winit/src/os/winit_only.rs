//! A system winit covers by itself, the BSDs among them: nothing to add, and nothing to ask, so
//! every answer is the trait's own but the clipboard's, arboard's plain text.

use inset_embedder::TargetPlatform;

use crate::clipboard::{ChangeProbe, TextClipboard};

use super::absent::{
    NoDisplayLink, NoTrackpadTouchObserver, NoVisibilityNotices, NoWindowSurfaces,
};
use super::{
    ApplicationControl, ApplicationLifecycle, EventLoopSetup, FramePresentation, ImageDecoding,
    NativeComposition, NativeMenus, OperatingSystem, PointerQueries, Screens, SystemChanges,
    SystemClipboard, SystemDrags, SystemFileDialogs, SystemMedia, SystemNotifications,
    SystemOpener, SystemRequests, TrackpadTouches, WindowChrome, WindowGeometry, WindowMotion,
    WindowVisibility, WorkQueue,
};

/// Any system without a module of its own. A type only: never made, its answers are
/// associated functions.
pub(crate) enum WinitOnly {}

impl OperatingSystem for WinitOnly {
    /// Every system without a module of its own that Flutter names, and Linux for the rest,
    /// which are Linux's relatives.
    const TARGET_PLATFORM: TargetPlatform = if cfg!(target_os = "fuchsia") {
        TargetPlatform::Fuchsia
    } else {
        TargetPlatform::Linux
    };
}

/// arboard's plain text, as winit has no clipboard (`clipboard/text.rs`), its changes told by the
/// text itself, as arboard has no cheaper probe there; a copy that expires is cleared by the main
/// loop.
impl SystemClipboard for WinitOnly {
    type Clipboard = TextClipboard;

    fn clipboard(_look: Box<dyn Fn()>) -> TextClipboard {
        TextClipboard::new(ChangeProbe::Text)
    }
}

impl FramePresentation for WinitOnly {
    type DisplayLink = NoDisplayLink;
}

impl NativeComposition for WinitOnly {
    type WindowSurfaces = NoWindowSurfaces;
}

impl WindowVisibility for WinitOnly {
    type Notices = NoVisibilityNotices;
}

impl TrackpadTouches for WinitOnly {
    type Observer = NoTrackpadTouchObserver;
}

impl EventLoopSetup for WinitOnly {}
impl WindowGeometry for WinitOnly {}
impl SystemMedia for WinitOnly {}
impl SystemDrags for WinitOnly {}
impl WindowMotion for WinitOnly {}
impl WindowChrome for WinitOnly {}
impl Screens for WinitOnly {}
impl PointerQueries for WinitOnly {}
impl NativeMenus for WinitOnly {}
impl ImageDecoding for WinitOnly {}
impl WorkQueue for WinitOnly {}
impl SystemFileDialogs for WinitOnly {}
impl SystemOpener for WinitOnly {}
impl SystemNotifications for WinitOnly {}
impl SystemRequests for WinitOnly {}
impl SystemChanges for WinitOnly {}
impl ApplicationControl for WinitOnly {}
impl ApplicationLifecycle for WinitOnly {}

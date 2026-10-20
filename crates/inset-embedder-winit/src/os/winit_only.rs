//! A system winit covers by itself, Windows and Linux among them: nothing to add, and
//! nothing to ask, so every answer is the trait's own.

use inset_embedder::TargetPlatform;

use super::absent::{NoDisplayLink, NoVisibilityNotices, NoWindowSurfaces};
use super::{
    ApplicationControl, ApplicationLifecycle, FramePresentation, ImageDecoding, NativeComposition,
    NativeMenus, OperatingSystem, PointerQueries, Screens, SystemChanges, SystemRequests,
    WindowChrome, WindowGeometry, WindowMotion, WindowVisibility, WorkQueue,
};

/// Any system without a module of its own. A type only: never made, its answers are
/// associated functions.
pub(crate) enum WinitOnly {}

impl OperatingSystem for WinitOnly {
    /// Every system without a module of its own that Flutter names, and Linux for the rest.
    const TARGET_PLATFORM: TargetPlatform = if cfg!(target_os = "windows") {
        TargetPlatform::Windows
    } else if cfg!(target_os = "fuchsia") {
        TargetPlatform::Fuchsia
    } else {
        TargetPlatform::Linux
    };
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

impl WindowGeometry for WinitOnly {}
impl WindowMotion for WinitOnly {}
impl WindowChrome for WinitOnly {}
impl Screens for WinitOnly {}
impl PointerQueries for WinitOnly {}
impl NativeMenus for WinitOnly {}
impl ImageDecoding for WinitOnly {}
impl WorkQueue for WinitOnly {}
impl SystemRequests for WinitOnly {}
impl SystemChanges for WinitOnly {}
impl ApplicationControl for WinitOnly {}
impl ApplicationLifecycle for WinitOnly {}

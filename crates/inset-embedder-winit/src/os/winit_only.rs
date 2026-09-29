//! A system winit covers by itself, Windows and Linux among them: nothing to add, and
//! nothing to ask, so every answer is the trait's own.

use super::absent::{NoDisplayLink, NoWindowSurfaces};
use super::{
    FramePresentation, NativeComposition, NativeMenus, OperatingSystem, PointerState, WindowChrome,
    WindowGeometry, WorkQueue,
};

/// Any system without a module of its own. A type only: never made, its answers are
/// associated functions.
pub(crate) enum WinitOnly {}

impl FramePresentation for WinitOnly {
    type DisplayLink = NoDisplayLink;
}

impl NativeComposition for WinitOnly {
    type WindowSurfaces = NoWindowSurfaces;
}

impl WindowGeometry for WinitOnly {}
impl WindowChrome for WinitOnly {}
impl PointerState for WinitOnly {}
impl NativeMenus for WinitOnly {}
impl WorkQueue for WinitOnly {}
impl OperatingSystem for WinitOnly {}

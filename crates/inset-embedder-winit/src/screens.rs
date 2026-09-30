//! The screens an app's windows are placed on, as [`WinitPlatform::screens`] reports them, for
//! an app that places a window against something on the screen: its Dock icon, an edge.
//! Flutter's framework has no API for them (a `Display` has a size, not a place), so they are
//! this host's own, beside `Platform`.
//!
//! [`WinitPlatform::screens`]: crate::WinitPlatform::screens

use inset_embedder::Rect;

/// A screen, in the coordinates of window positions: logical points from the top-left corner
/// of the main screen, y growing down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    /// All of the screen.
    pub frame: Rect,
    /// What the menu bar and the Dock leave free. On macOS the Dock's band is left out only
    /// while the Dock keeps its space, and not while it hides itself.
    pub visible_frame: Rect,
}

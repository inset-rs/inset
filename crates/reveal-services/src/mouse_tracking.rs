//! Flutter counterpart: `services/mouse_tracking.dart`.

use std::fmt;

use reveal_gestures::{PointerEnterEventListener, PointerExitEventListener};

use crate::mouse_cursor::{MouseCursor, MouseCursorRef};

/// The annotation object used to annotate regions that are interested in mouse
/// movements.
///
/// To use an annotation, return this object as a `HitTestTarget` in a hit test.
/// Typically this is done using a `RenderMouseRegion`.
///
/// The `MouseTracker` uses this class as a label to filter the hit test results.
/// Only hit test targets that have this annotation as their target will receive
/// mouse events.
///
/// Dart's constructor takes named optional arguments; write
/// `MouseTrackerAnnotation { on_enter: Some(..), ..Default::default() }`.
pub struct MouseTrackerAnnotation {
    /// Triggered when a mouse pointer, with or without buttons pressed, has
    /// entered the region and `valid_for_mouse_tracker` is true.
    ///
    /// This callback is triggered when the pointer has started to be contained by
    /// the region, either due to a pointer event, or due to the movement or
    /// disappearance of the region. This method is always matched by a later
    /// `on_exit`.
    pub on_enter: Option<PointerEnterEventListener>,

    /// Triggered when a mouse pointer, with or without buttons pressed, has
    /// exited the region and `valid_for_mouse_tracker` is true.
    ///
    /// This callback is triggered when the pointer has stopped being contained
    /// by the region, either due to a pointer event, or due to the movement or
    /// disappearance of the region. This method always matches an earlier
    /// `on_enter`.
    pub on_exit: Option<PointerExitEventListener>,

    /// The mouse cursor for mouse pointers that are hovering over the region.
    ///
    /// When a mouse enters the region, its cursor will be changed to the `cursor`.
    /// When the mouse leaves the region, the cursor will be set by the region
    /// found at the new location.
    ///
    /// Defaults to `MouseCursor.defer`, deferring the choice of cursor to the next
    /// region behind it in hit-test order.
    pub cursor: MouseCursorRef,

    /// Whether this is included when a `MouseTracker` collects the list of
    /// annotations.
    ///
    /// If [`valid_for_mouse_tracker`](Self::valid_for_mouse_tracker) is false, this
    /// object is excluded from the current annotation list even if it's included
    /// in the hit test, affecting mouse-related behavior such as enter events,
    /// exit events, and mouse cursors. The [`valid_for_mouse_tracker`] does not
    /// affect hit testing.
    ///
    /// Defaults to true.
    ///
    /// [`valid_for_mouse_tracker`]: Self::valid_for_mouse_tracker
    pub valid_for_mouse_tracker: bool,
}

impl Default for MouseTrackerAnnotation {
    fn default() -> MouseTrackerAnnotation {
        MouseTrackerAnnotation {
            on_enter: None,
            on_exit: None,
            cursor: <dyn MouseCursor>::defer(),
            valid_for_mouse_tracker: true,
        }
    }
}

impl fmt::Debug for MouseTrackerAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut callbacks = Vec::new();
        if self.on_enter.is_some() {
            callbacks.push("enter");
        }
        if self.on_exit.is_some() {
            callbacks.push("exit");
        }
        f.debug_struct("MouseTrackerAnnotation")
            .field("callbacks", &callbacks)
            .field("cursor", &self.cursor.debug_description())
            .field("valid_for_mouse_tracker", &self.valid_for_mouse_tracker)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_defers_the_cursor_and_is_valid() {
        let annotation = MouseTrackerAnnotation::default();
        assert!(annotation.on_enter.is_none());
        assert!(annotation.on_exit.is_none());
        assert!(*annotation.cursor == *<dyn MouseCursor>::defer());
        assert!(annotation.valid_for_mouse_tracker);
    }
}

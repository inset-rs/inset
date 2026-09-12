//! Flutter counterpart: `widgets/desktop_text_selection_toolbar_layout_delegate.dart`.

use std::any::Any;
use std::fmt::{self, Debug};

use inset_embedder::{Offset, Size};
use inset_rendering::{BoxConstraints, SingleChildLayoutDelegate};

/// Positions the toolbar at [`anchor`](Self::anchor) if it fits, otherwise moves it so that it
/// just fits fully on-screen.
///
/// See also:
///
///   * `desktopTextSelectionControls`, which uses this to position itself.
///   * `cupertinoDesktopTextSelectionControls`, which uses this to position itself.
///   * [`crate::TextSelectionToolbarLayoutDelegate`], which does a similar layout for the
///     mobile text selection toolbars.
#[derive(Debug)]
pub struct DesktopTextSelectionToolbarLayoutDelegate {
    /// The point at which to render the menu, if possible.
    ///
    /// Should be provided in local coordinates.
    pub anchor: Offset,
}

impl DesktopTextSelectionToolbarLayoutDelegate {
    /// Creates an instance of [`DesktopTextSelectionToolbarLayoutDelegate`].
    pub fn new(anchor: Offset) -> DesktopTextSelectionToolbarLayoutDelegate {
        DesktopTextSelectionToolbarLayoutDelegate { anchor }
    }
}

impl SingleChildLayoutDelegate for DesktopTextSelectionToolbarLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let overhang = Offset::new(
            self.anchor.dx() + child_size.width() - size.width(),
            self.anchor.dy() + child_size.height() - size.height(),
        );
        Offset::new(
            if overhang.dx() > 0.0 {
                self.anchor.dx() - overhang.dx()
            } else {
                self.anchor.dx()
            },
            if overhang.dy() > 0.0 {
                self.anchor.dy() - overhang.dy()
            } else {
                self.anchor.dy()
            },
        )
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<DesktopTextSelectionToolbarLayoutDelegate>()
            .is_none_or(|old| self.anchor != old.anchor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Display for DesktopTextSelectionToolbarLayoutDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DesktopTextSelectionToolbarLayoutDelegate")
    }
}

//! Flutter counterpart: `widgets/text_selection_toolbar_layout_delegate.dart`.

use std::any::Any;
use std::fmt::{self, Debug};

use reveal_embedder::{Offset, Size};
use reveal_rendering::{BoxConstraints, SingleChildLayoutDelegate};

/// A [`SingleChildLayoutDelegate`] for use with `CustomSingleChildLayout` that positions its
/// child above [`anchor_above`](Self::anchor_above) if it fits, or otherwise below
/// [`anchor_below`](Self::anchor_below).
///
/// Primarily intended for use with toolbars or context menus.
///
/// See also:
///
///   * `TextSelectionToolbar`, which uses this to position itself.
///   * `CupertinoTextSelectionToolbar`, which also uses this to position itself.
#[derive(Debug)]
pub struct TextSelectionToolbarLayoutDelegate {
    /// The location that the toolbar should attempt to position itself at.
    ///
    /// Should be provided in local coordinates.
    pub anchor_above: Offset,
    /// The fallback position that should be used if [`anchor_above`](Self::anchor_above)
    /// doesn't work.
    ///
    /// Should be provided in local coordinates.
    pub anchor_below: Offset,
    /// Whether or not the child should be considered to fit above
    /// [`anchor_above`](Self::anchor_above).
    ///
    /// Typically used to force the child to be drawn at `anchor_above` even when it doesn't
    /// fit, such as when the Material `TextSelectionToolbar` draws an open overflow menu.
    ///
    /// If not provided, it will be calculated.
    pub fits_above: Option<bool>,
}

impl TextSelectionToolbarLayoutDelegate {
    /// Creates an instance of [`TextSelectionToolbarLayoutDelegate`].
    pub fn new(anchor_above: Offset, anchor_below: Offset) -> TextSelectionToolbarLayoutDelegate {
        TextSelectionToolbarLayoutDelegate {
            anchor_above,
            anchor_below,
            fits_above: None,
        }
    }

    /// Dart `TextSelectionToolbarLayoutDelegate(fitsAbove:)`.
    pub fn fits_above(mut self, fits_above: bool) -> TextSelectionToolbarLayoutDelegate {
        self.fits_above = Some(fits_above);
        self
    }

    /// Return the distance from zero that centers `width` as closely as possible to `position`
    /// from zero while fitting between zero and `max`.
    pub fn center_on(position: f64, width: f64, max: f64) -> f64 {
        if position - width / 2.0 < 0.0 {
            return 0.0;
        }
        if position + width / 2.0 > max {
            return max - width;
        }
        position - width / 2.0
    }
}

impl SingleChildLayoutDelegate for TextSelectionToolbarLayoutDelegate {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints.loosen()
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let fits_above = self
            .fits_above
            .unwrap_or(self.anchor_above.dy() >= child_size.height());
        let anchor = if fits_above {
            self.anchor_above
        } else {
            self.anchor_below
        };
        Offset::new(
            Self::center_on(anchor.dx(), child_size.width(), size.width()),
            if fits_above {
                (self.anchor_above.dy() - child_size.height()).max(0.0)
            } else {
                anchor.dy()
            },
        )
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<TextSelectionToolbarLayoutDelegate>()
            .is_none_or(|old| {
                self.anchor_above != old.anchor_above
                    || self.anchor_below != old.anchor_below
                    || self.fits_above != old.fits_above
            })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Display for TextSelectionToolbarLayoutDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextSelectionToolbarLayoutDelegate")
    }
}

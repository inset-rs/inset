//! Flutter counterpart: `rendering/editable.dart` (`TextSelectionPoint`).
//!
//! `RenderEditable` and `VerticalCaretMovementRun` wait; see PORTING.md.

use std::fmt;

use reveal_embedder::{Offset, TextDirection};

/// Represents the coordinates of the point in a selection, and the text
/// direction at that point, relative to top left of the `RenderEditable` that
/// holds the selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionPoint {
    /// Coordinates of the lower left or lower right corner of the selection,
    /// relative to the top left of the `RenderEditable` object.
    pub point: Offset,
    /// Direction of the text at this edge of the selection.
    pub direction: Option<TextDirection>,
}

impl TextSelectionPoint {
    /// Creates a description of a point in a text selection.
    pub const fn new(point: Offset, direction: Option<TextDirection>) -> TextSelectionPoint {
        TextSelectionPoint { point, direction }
    }
}

impl fmt::Display for TextSelectionPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.direction {
            Some(TextDirection::Ltr) => write!(f, "{:?}-ltr", self.point),
            Some(TextDirection::Rtl) => write!(f, "{:?}-rtl", self.point),
            None => write!(f, "{:?}", self.point),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_direction() {
        let point = Offset::new(1.0, 2.0);
        assert_eq!(
            TextSelectionPoint::new(point, Some(TextDirection::Ltr)).to_string(),
            format!("{point:?}-ltr")
        );
        assert_eq!(
            TextSelectionPoint::new(point, None).to_string(),
            format!("{point:?}")
        );
    }
}

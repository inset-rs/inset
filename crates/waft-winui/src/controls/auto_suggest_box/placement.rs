//! Where `SuggestionsPopup` opens and how big `SuggestionsContainer` may be:
//! `MaximizeSuggestionAreaWithoutInputPane`, `UpdateSuggestionListSize` and
//! `UpdateSuggestionListPosition` of AutoSuggestBox_Partial.cpp, and the delegate that places the
//! container where they say.

use std::any::Any;
use waft_embedder::{Offset, Rect, Size};
use waft_rendering::{BoxConstraints, SingleChildLayoutDelegate};

/// `SuggestionListPosition`: the side of the text box the list opens on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SuggestionListPosition {
    /// Under the text box, in the items' order.
    Below,
    /// Over the text box, in the items' reverse order, so that the first item is next to the box.
    Above,
}

/// What the three source functions decide, in the root overlay's coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SuggestionListLayout {
    /// `m_suggestionListPosition`.
    pub(super) position: SuggestionListPosition,

    /// The popup's left edge: the box's, as the source leaves `HorizontalOffset` at zero.
    pub(super) left: f64,

    /// `SuggestionsContainer.Width`: the box's `ActualWidth`.
    pub(super) width: f64,

    /// `SuggestionsContainer.MaxHeight`.
    pub(super) max_height: f64,

    /// The text box's `ContentElement`, which the popup aligns to: its top for a list above, its
    /// bottom for a list below.
    pub(super) content_element: Rect,
}

/// Lays the list out beside `text_box`, the whole TextBox with its header and description,
/// whose `ContentElement` is `content_element`, in a window of `bounds`, for the box's
/// `MaxSuggestionListHeight`.
pub(super) fn arrange(
    text_box: Rect,
    content_element: Rect,
    bounds: Size,
    max_suggestion_list_height: f64,
) -> SuggestionListLayout {
    // `MaximizeSuggestionAreaWithoutInputPane`: the side with more room, which is all the list
    // may take.
    let delta_top = text_box.top;
    let delta_bottom = bounds.height() - text_box.bottom;
    let (position, available) = if delta_top < delta_bottom {
        (SuggestionListPosition::Below, delta_bottom.abs())
    } else {
        (SuggestionListPosition::Above, delta_top)
    };
    let available = available.max(0.0);
    // `UpdateSuggestionListSize`: a negative `MaxSuggestionListHeight` asks for all the room.
    let max_height = if (available > 0.0 && max_suggestion_list_height > available)
        || max_suggestion_list_height < 0.0
    {
        available
    } else {
        max_suggestion_list_height
    };
    SuggestionListLayout {
        position,
        left: text_box.left,
        width: text_box.width(),
        max_height,
        content_element,
    }
}

/// Places `SuggestionsContainer` as `UpdateSuggestionListPosition` translates `SuggestionsPopup`:
/// its top at `ContentElement`'s bottom, or its bottom at `ContentElement`'s top.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SuggestionsPosition(pub(super) SuggestionListLayout);

impl SingleChildLayoutDelegate for SuggestionsPosition {
    fn get_constraints_for_child(&self, _constraints: BoxConstraints) -> BoxConstraints {
        let layout = self.0;
        BoxConstraints::new()
            .min_width(layout.width)
            .max_width(layout.width)
            .max_height(layout.max_height)
    }

    fn get_position_for_child(&self, _size: Size, child_size: Size) -> Offset {
        let layout = self.0;
        let top = match layout.position {
            SuggestionListPosition::Below => layout.content_element.bottom,
            SuggestionListPosition::Above => layout.content_element.top - child_size.height(),
        };
        Offset::new(layout.left, top)
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<Self>()
            .is_none_or(|old| old != self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(text_box: Rect) -> Rect {
        Rect::from_ltrb(
            text_box.left + 1.0,
            text_box.top + 1.0,
            text_box.right - 1.0,
            text_box.bottom - 1.0,
        )
    }

    #[test]
    fn the_list_opens_on_the_side_with_more_room_and_takes_at_most_that_room() {
        let window = Size::new(800.0, 600.0);
        let high = Rect::from_ltwh(10.0, 100.0, 300.0, 32.0);
        let below = arrange(high, content(high), window, f64::INFINITY);
        assert_eq!(below.position, SuggestionListPosition::Below);
        assert_eq!(below.max_height, 600.0 - 132.0);
        assert_eq!((below.left, below.width), (10.0, 300.0));

        let low = Rect::from_ltwh(10.0, 500.0, 300.0, 32.0);
        let above = arrange(low, content(low), window, 200.0);
        assert_eq!(above.position, SuggestionListPosition::Above);
        assert_eq!(above.max_height, 200.0);
    }

    #[test]
    fn a_negative_max_height_asks_for_all_the_room() {
        let window = Size::new(800.0, 600.0);
        let text_box = Rect::from_ltwh(0.0, 0.0, 300.0, 32.0);
        let layout = arrange(text_box, content(text_box), window, -1.0);
        assert_eq!(layout.max_height, 568.0);
    }
}

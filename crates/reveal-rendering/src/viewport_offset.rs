//! Flutter counterpart: `rendering/viewport_offset.dart` (`ScrollDirection`).
//!
//! `ViewportOffset` waits on the first viewport.

/// The direction of a scroll, relative to the positive scroll offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    /// No scrolling is underway.
    Idle,

    /// Scrolling is happening in the negative scroll offset direction.
    Forward,

    /// Scrolling is happening in the positive scroll offset direction.
    Reverse,
}

/// Returns the opposite of the given [`ScrollDirection`].
///
/// [`ScrollDirection::Idle`] is unchanged.
pub fn flip_scroll_direction(direction: ScrollDirection) -> ScrollDirection {
    match direction {
        ScrollDirection::Idle => ScrollDirection::Idle,
        ScrollDirection::Forward => ScrollDirection::Reverse,
        ScrollDirection::Reverse => ScrollDirection::Forward,
    }
}

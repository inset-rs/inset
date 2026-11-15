//! Flutter `services/predictive_back_event.dart`.

use std::fmt::{self, Debug};

use waft_embedder::Offset;

/// Enum representing the edge from which a swipe starts in a back gesture.
///
/// This is used in [`PredictiveBackEvent`] to indicate the starting edge of the swipe gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwipeEdge {
    /// Indicates that the swipe gesture starts from the left edge of the screen.
    Left,

    /// Indicates that the swipe gesture starts from the right edge of the screen.
    Right,
}

/// Object used to report back gesture progress in Android.
///
/// Holds information about the touch event, swipe direction, and the animation progress that
/// predictive back animations should follow.
#[derive(Clone, Copy, PartialEq)]
pub struct PredictiveBackEvent {
    /// The global position of the touch point as an `Offset`, or `None` if the event is
    /// triggered by a button press.
    ///
    /// This represents the touch location that initiates or interacts with the back gesture.
    /// When `None`, it indicates the gesture was not started by a touch event, such as a back
    /// button press in devices with hardware buttons.
    pub touch_offset: Option<Offset>,

    /// Returns a value between 0.0 and 1.0 representing how far along the back gesture is.
    ///
    /// This value is driven by the horizontal location of the touch point, and should be used
    /// as the fraction to seek the predictive back animation with. Specifically,
    ///
    /// - The progress is 0.0 when the touch is at the starting edge of the screen (left or
    ///   right), and the animation should seek to its start state.
    /// - The progress is approximately 1.0 when the touch is at the opposite side of the
    ///   screen, and the animation should seek to its end state. Exact end value may vary
    ///   depending on screen size.
    ///
    /// When the gesture is canceled, the progress value continues to update, animating back to
    /// 0.0 until the cancellation animation completes.
    ///
    /// In-between locations are linearly interpolated based on horizontal distance from the
    /// starting edge and smooth clamped to 1.0 when the distance exceeds a system-wide
    /// threshold.
    pub progress: f64,

    /// The screen edge from which the swipe gesture starts.
    pub swipe_edge: SwipeEdge,
}

impl PredictiveBackEvent {
    /// Creates a new [`PredictiveBackEvent`] instance, from the values a host reports; Dart's
    /// `PredictiveBackEvent.fromMap` reads the same three from a platform channel's map.
    ///
    /// `progress` must be between 0.0 and 1.0.
    pub fn new(touch_offset: Option<Offset>, progress: f64, swipe_edge: SwipeEdge) -> Self {
        debug_assert!((0.0..=1.0).contains(&progress));
        PredictiveBackEvent {
            touch_offset,
            progress,
            swipe_edge,
        }
    }

    /// Indicates if the event was triggered by a system back button press.
    ///
    /// Returns false for a predictive back gesture.
    pub fn is_button_event(&self) -> bool {
        // The Android documentation for BackEvent
        // (https://developer.android.com/reference/android/window/BackEvent#getTouchX())
        // says that getTouchX and getTouchY should return NaN when the system
        // back button is pressed, but in practice it seems to return 0.0, hence
        // the check for Offset.zero here. This was tested directly in the engine
        // on Android emulator running API 34.
        self.touch_offset.is_none()
            || (self.progress == 0.0 && self.touch_offset == Some(Offset::ZERO))
    }
}

impl Debug for PredictiveBackEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PredictiveBackEvent{{touchOffset: {:?}, progress: {:?}, swipeEdge: {:?}}}",
            self.touch_offset, self.progress, self.swipe_edge
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // predictive_back_event_test.dart builds each event with `fromMap`; the host hands the same
    // values to `new`.

    #[test]
    fn can_be_created_with_valid_values_swipe_edge_left() {
        let event = PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 0.0, SwipeEdge::Left);
        assert_eq!(event.swipe_edge, SwipeEdge::Left);
        assert!(!event.is_button_event());
    }

    #[test]
    fn can_be_created_with_valid_values_swipe_edge_right() {
        let event = PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 0.0, SwipeEdge::Right);
        assert_eq!(event.swipe_edge, SwipeEdge::Right);
        assert!(!event.is_button_event());
    }

    #[test]
    fn a_button_event_at_the_zero_position() {
        let event = PredictiveBackEvent::new(Some(Offset::ZERO), 0.0, SwipeEdge::Right);
        assert!(event.is_button_event());
    }

    #[test]
    fn a_button_event_with_no_position() {
        let event = PredictiveBackEvent::new(None, 0.0, SwipeEdge::Right);
        assert!(event.is_button_event());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn creating_it_panics_when_given_invalid_progress() {
        PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 2.0, SwipeEdge::Right);
    }

    #[test]
    fn events_created_with_the_same_values_are_equal() {
        let event_a = PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 0.0, SwipeEdge::Left);
        let event_b = PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 0.0, SwipeEdge::Left);
        assert_eq!(event_a, event_b);
        assert_eq!(format!("{event_a:?}"), format!("{event_b:?}"));
    }

    #[test]
    fn events_created_with_different_values_differ() {
        let event_a = PredictiveBackEvent::new(Some(Offset::new(0.0, 100.0)), 0.0, SwipeEdge::Left);
        let event_b = PredictiveBackEvent::new(Some(Offset::new(1.0, 100.0)), 0.0, SwipeEdge::Left);
        assert_ne!(event_a, event_b);
        assert_ne!(format!("{event_a:?}"), format!("{event_b:?}"));
    }
}

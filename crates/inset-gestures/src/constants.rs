//! Flutter counterpart: `gestures/constants.dart`.

use std::time::Duration;

// Modeled after Android's ViewConfiguration:
// https://github.com/android/platform_frameworks_base/blob/main/core/java/android/view/ViewConfiguration.java

/// The time that must elapse before a tap gesture sends onTapDown, if there's
/// any doubt that the gesture is a tap.
pub const K_PRESS_TIMEOUT: Duration = Duration::from_millis(100);

/// Maximum length of time between a tap down and a tap up for the gesture to be
/// considered a tap. (Currently not honored by the TapGestureRecognizer.)
pub const K_HOVER_TAP_TIMEOUT: Duration = Duration::from_millis(150);

/// Maximum distance between the down and up pointers for a tap. (Currently not
/// honored by the `TapGestureRecognizer`; `PrimaryPointerGestureRecognizer`,
/// which TapGestureRecognizer inherits from, uses [`K_TOUCH_SLOP`].)
pub const K_HOVER_TAP_SLOP: f64 = 20.0; // Logical pixels

/// The time before a long press gesture attempts to win.
pub const K_LONG_PRESS_TIMEOUT: Duration = Duration::from_millis(500);

/// The maximum time from the start of the first tap to the start of the second
/// tap in a double-tap gesture.
pub const K_DOUBLE_TAP_TIMEOUT: Duration = Duration::from_millis(300);

/// The minimum time from the end of the first tap to the start of the second
/// tap in a double-tap gesture.
pub const K_DOUBLE_TAP_MIN_TIME: Duration = Duration::from_millis(40);

/// The distance a touch has to travel for the framework to be confident that
/// the gesture is a scroll gesture, or, inversely, the maximum distance that a
/// touch can travel before the framework becomes confident that it is not a
/// tap.
///
/// A total delta less than or equal to [`K_TOUCH_SLOP`] is not considered to be a
/// drag, whereas if the delta is greater than [`K_TOUCH_SLOP`] it is considered to
/// be a drag.
pub const K_TOUCH_SLOP: f64 = 18.0; // Logical pixels

/// The maximum distance that the first touch in a double-tap gesture can travel
/// before deciding that it is not part of a double-tap gesture.
/// DoubleTapGestureRecognizer also restricts the second touch to this distance.
pub const K_DOUBLE_TAP_TOUCH_SLOP: f64 = K_TOUCH_SLOP; // Logical pixels

/// Distance between the initial position of the first touch and the start
/// position of a potential second touch for the second touch to be considered
/// the second touch of a double-tap gesture.
pub const K_DOUBLE_TAP_SLOP: f64 = 100.0; // Logical pixels

/// The time for which zoom controls (e.g. in a map interface) are to be
/// displayed on the screen, from the moment they were last requested.
pub const K_ZOOM_CONTROLS_TIMEOUT: Duration = Duration::from_millis(3000);

/// The distance a touch has to travel for the framework to be confident that
/// the gesture is a paging gesture. (Currently not used, because paging uses a
/// regular drag gesture, which uses kTouchSlop.)
pub const K_PAGING_TOUCH_SLOP: f64 = K_TOUCH_SLOP * 2.0; // Logical pixels

/// The distance a touch has to travel for the framework to be confident that
/// the gesture is a panning gesture.
pub const K_PAN_SLOP: f64 = K_TOUCH_SLOP * 2.0; // Logical pixels

/// The distance a touch has to travel for the framework to be confident that
/// the gesture is a scale gesture.
pub const K_SCALE_SLOP: f64 = K_TOUCH_SLOP; // Logical pixels

/// The margin around a dialog, popup menu, or other window-like widget inside
/// which we do not consider a tap to dismiss the widget. (Not currently used.)
pub const K_WINDOW_TOUCH_SLOP: f64 = 16.0; // Logical pixels

/// The minimum velocity for a touch to consider that touch to trigger a fling
/// gesture.
pub const K_MIN_FLING_VELOCITY: f64 = 50.0; // Logical pixels / second

/// Drag gesture fling velocities are clipped to this value.
pub const K_MAX_FLING_VELOCITY: f64 = 8000.0; // Logical pixels / second

/// The maximum time from the start of the first tap to the start of the second
/// tap in a jump-tap gesture.
pub const K_JUMP_TAP_TIMEOUT: Duration = Duration::from_millis(500);

/// Like [`K_TOUCH_SLOP`], but for more precise pointers like mice and trackpads.
pub const K_PRECISE_POINTER_HIT_SLOP: f64 = 1.0; // Logical pixels

/// Like [`K_PAN_SLOP`], but for more precise pointers like mice and trackpads.
pub const K_PRECISE_POINTER_PAN_SLOP: f64 = K_PRECISE_POINTER_HIT_SLOP * 2.0; // Logical pixels

/// Like [`K_SCALE_SLOP`], but for more precise pointers like mice and trackpads.
pub const K_PRECISE_POINTER_SCALE_SLOP: f64 = K_PRECISE_POINTER_HIT_SLOP; // Logical pixels

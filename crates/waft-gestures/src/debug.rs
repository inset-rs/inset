//! Flutter counterpart: `gestures/debug.dart`.

use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_PRINT_HIT_TEST_RESULTS: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_MOUSE_HOVER_EVENTS: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_GESTURE_ARENA_DIAGNOSTICS: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_RECOGNIZER_CALLBACKS_TRACE: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_RESAMPLING_MARGIN: AtomicBool = AtomicBool::new(false);

/// Whether to print the results of each hit test to the console.
///
/// When this is set, in debug mode, any time a hit test is triggered by the
/// `GestureBinding` the results are dumped to the console.
///
/// This has no effect in release builds.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_print_hit_test_results`].
pub fn debug_print_hit_test_results() -> bool {
    DEBUG_PRINT_HIT_TEST_RESULTS.load(Ordering::Relaxed)
}

/// Sets [`debug_print_hit_test_results`].
pub fn set_debug_print_hit_test_results(value: bool) {
    DEBUG_PRINT_HIT_TEST_RESULTS.store(value, Ordering::Relaxed);
}

/// Whether to print the details of each mouse hover event to the console.
///
/// When this is set, in debug mode, any time a mouse hover event is triggered
/// by the `GestureBinding`, the results are dumped to the console.
///
/// This has no effect in release builds, and only applies to mouse hover
/// events.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_print_mouse_hover_events`].
pub fn debug_print_mouse_hover_events() -> bool {
    DEBUG_PRINT_MOUSE_HOVER_EVENTS.load(Ordering::Relaxed)
}

/// Sets [`debug_print_mouse_hover_events`].
pub fn set_debug_print_mouse_hover_events(value: bool) {
    DEBUG_PRINT_MOUSE_HOVER_EVENTS.store(value, Ordering::Relaxed);
}

/// Prints information about gesture recognizers and gesture arenas.
///
/// This flag only has an effect in debug mode.
///
/// See also:
///
///  * [`crate::GestureArenaManager`], the class that manages gesture arenas.
///  * [`debug_print_recognizer_callbacks_trace`], for debugging issues with
///    gesture recognizers.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_print_gesture_arena_diagnostics`].
pub fn debug_print_gesture_arena_diagnostics() -> bool {
    DEBUG_PRINT_GESTURE_ARENA_DIAGNOSTICS.load(Ordering::Relaxed)
}

/// Sets [`debug_print_gesture_arena_diagnostics`].
pub fn set_debug_print_gesture_arena_diagnostics(value: bool) {
    DEBUG_PRINT_GESTURE_ARENA_DIAGNOSTICS.store(value, Ordering::Relaxed);
}

/// Logs a message every time a gesture recognizer callback is invoked.
///
/// This flag only has an effect in debug mode.
///
/// This is specifically used by `GestureRecognizer.invokeCallback`. Gesture
/// recognizers that do not use this method to invoke callbacks may not honor
/// the [`debug_print_recognizer_callbacks_trace`] flag.
///
/// See also:
///
///  * [`debug_print_gesture_arena_diagnostics`], for debugging issues with gesture
///    arenas.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_print_recognizer_callbacks_trace`].
pub fn debug_print_recognizer_callbacks_trace() -> bool {
    DEBUG_PRINT_RECOGNIZER_CALLBACKS_TRACE.load(Ordering::Relaxed)
}

/// Sets [`debug_print_recognizer_callbacks_trace`].
pub fn set_debug_print_recognizer_callbacks_trace(value: bool) {
    DEBUG_PRINT_RECOGNIZER_CALLBACKS_TRACE.store(value, Ordering::Relaxed);
}

/// Whether to print the resampling margin to the console.
///
/// When this is set, in debug mode, any time resampling is triggered by the
/// `GestureBinding` the resampling margin is dumped to the console. The
/// resampling margin is the delta between the time of the last received
/// touch event and the current sample time. Positive value indicates that
/// resampling is effective and the resampling offset can potentially be
/// reduced for improved latency. Negative value indicates that resampling
/// is failing and resampling offset needs to be increased for smooth
/// touch event processing.
///
/// This has no effect in release builds.
///
/// In Flutter this is a library-level `bool`. Assignment is
/// [`set_debug_print_resampling_margin`].
pub fn debug_print_resampling_margin() -> bool {
    DEBUG_PRINT_RESAMPLING_MARGIN.load(Ordering::Relaxed)
}

/// Sets [`debug_print_resampling_margin`].
pub fn set_debug_print_resampling_margin(value: bool) {
    DEBUG_PRINT_RESAMPLING_MARGIN.store(value, Ordering::Relaxed);
}

/// Returns true if none of the gestures library debug variables have been changed.
///
/// This function is used by the test framework to ensure that debug variables
/// haven't been inadvertently changed.
///
/// See [the gestures library](https://api.flutter.dev/flutter/gestures/gestures-library.html) for a complete
/// list.
pub fn debug_assert_all_gestures_vars_unset(reason: &str) -> bool {
    debug_assert!(
        !(debug_print_hit_test_results()
            || debug_print_gesture_arena_diagnostics()
            || debug_print_recognizer_callbacks_trace()
            || debug_print_resampling_margin()),
        "{reason}"
    );
    true
}

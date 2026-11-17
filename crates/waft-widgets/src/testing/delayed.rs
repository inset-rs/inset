//! Dart's `Future.delayed` for a test: a [`Task`] that resolves `value` after a timer.
//!
//! A case that gives a widget a value after a delay builds it here instead of chaining a
//! timer and a completer by hand, so the delay is the tester's clock and a `pump_by` the only
//! thing that lets it pass.

use std::time::Duration;

use waft_foundation::{App, Completer, Listener, Task, Timer};

/// Dart's `Future<T>.delayed(delay, () => value)`: `value` once `delay` has passed on `app`'s
/// clock.
pub fn delayed<T: Clone + 'static>(app: &mut App, delay: Duration, value: T) -> Task<T> {
    let completer: Completer<T> = Completer::new();
    let future = completer.future();
    Timer::new(
        app,
        delay,
        Listener::new(move |app| completer.complete(app, value.clone())),
    );
    app.spawn(async move |_cx| future.await)
}

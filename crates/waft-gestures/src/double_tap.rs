//! Flutter counterpart: `gestures/multitap.dart`'s `DoubleTapGestureRecognizer`.
//!
//! The recognizer keeps each in-progress tap in `trackers`, holds the first tap's arena once
//! that tap ends, and resolves both entries when a second tap lands within
//! [`K_DOUBLE_TAP_SLOP`] and after [`K_DOUBLE_TAP_MIN_TIME`]. A move beyond
//! [`K_DOUBLE_TAP_TOUCH_SLOP`] during a tap, or a rejected arena, cancels the sequence.
//!
//! Unlike tap and long press, this recognizer owns its trackers: the four states live in
//! `first_tap` and `trackers`, as in Dart, rather than in the one-sequence bags.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use waft_embedder::{Matrix4, Offset, PointerDeviceKind};
use waft_foundation::{App, Handle, Listener, Ref, Timer};

use crate::arena::{GestureArenaEntry, GestureDisposition};
use crate::binding::GestureBinding;
use crate::constants::{
    K_DOUBLE_TAP_MIN_TIME, K_DOUBLE_TAP_SLOP, K_DOUBLE_TAP_TIMEOUT, K_DOUBLE_TAP_TOUCH_SLOP,
};
use crate::events::{K_PRIMARY_BUTTON, PointerDownEvent, PointerEvent};
use crate::gesture_settings::DeviceGestureSettings;
use crate::pointer_router::PointerRoute;
use crate::recognizer::{
    GestureRecognizer, GestureRecognizerData, RecognizerLeaf, RecognizerLeafData,
};
use crate::tap::{GestureTapCancelCallback, GestureTapDownCallback, TapDownDetails};

/// Signature for when the user has tapped the screen at the same location twice in quick
/// succession.
///
/// See also:
///
///  * `GestureDetector.onDoubleTap`, which matches this signature.
pub type GestureDoubleTapCallback = Listener;

/// One pointer's tap, tracked from its down event until it ends or is rejected.
struct TapTracker {
    pointer: i64,
    entry: GestureArenaEntry,
    initial_global_position: Offset,
    initial_buttons: i64,
    /// Whether [`K_DOUBLE_TAP_MIN_TIME`] has passed since the down event, as Dart's
    /// `_CountdownZoned` records.
    min_time_elapsed: Rc<Cell<bool>>,
    /// The timer that sets [`min_time_elapsed`](Self::min_time_elapsed); kept so the countdown
    /// lives as long as its tracker.
    _min_time_timer: Timer,
    /// Whether the pointer route is registered; Dart's `_isTrackingPointer`.
    tracking: bool,
    _gesture_settings: Option<DeviceGestureSettings>,
}

impl TapTracker {
    fn is_within_global_tolerance(&self, position: Offset, tolerance: f64) -> bool {
        (position - self.initial_global_position).distance() <= tolerance
    }

    fn has_elapsed_min_time(&self) -> bool {
        self.min_time_elapsed.get()
    }

    fn has_same_button(&self, buttons: i64) -> bool {
        buttons == self.initial_buttons
    }
}

/// Recognizes when the user has tapped the screen at the same location twice in quick
/// succession.
///
/// It competes on pointer events when it has a non-null callback, and is a no-op otherwise.
pub struct DoubleTapGestureRecognizer {
    recognizer: GestureRecognizerData,

    on_double_tap_down: Option<GestureTapDownCallback>,
    on_double_tap: Option<GestureDoubleTapCallback>,
    on_double_tap_cancel: Option<GestureTapCancelCallback>,

    double_tap_timer: Option<Timer>,
    first_tap: Option<TapTracker>,
    trackers: HashMap<i64, TapTracker>,
}

impl DoubleTapGestureRecognizer {
    /// Creates a gesture recognizer for double taps.
    pub fn new(app: &mut App) -> Ref<DoubleTapGestureRecognizer> {
        let mut recognizer = GestureRecognizerData::new();
        // Accept only a primary button, as Dart's constructor does.
        recognizer.set_allowed_buttons_filter(Rc::new(|buttons| buttons == K_PRIMARY_BUTTON));
        app.create(DoubleTapGestureRecognizer {
            recognizer,
            on_double_tap_down: None,
            on_double_tap: None,
            on_double_tap_cancel: None,
            double_tap_timer: None,
            first_tap: None,
            trackers: HashMap::new(),
        })
    }

    /// Sets the kind of devices that are allowed to be recognized; `None` tracks and recognizes
    /// events from all device kinds.
    pub fn supported_devices(
        self: Ref<Self>,
        app: &mut App,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Ref<DoubleTapGestureRecognizer> {
        app.get_mut(&self).recognizer.supported_devices = Some(devices.into_iter().collect());
        self
    }

    /// Sets the kind of devices that are allowed to be recognized; `None` tracks and recognizes
    /// events from all device kinds.
    pub fn set_supported_devices(
        self: Handle<Self>,
        app: &mut App,
        devices: Option<HashSet<PointerDeviceKind>>,
    ) {
        app.get_mut(self).recognizer.supported_devices = devices;
    }

    /// Called when interaction starts. Limits the buttons this recognizer accepts.
    pub fn allowed_buttons_filter(
        self: Handle<Self>,
        app: &mut App,
        filter: impl Fn(i64) -> bool + 'static,
    ) -> Handle<DoubleTapGestureRecognizer> {
        app.get_mut(self).recognizer.allowed_buttons_filter = Rc::new(filter);
        self
    }

    /// Sets [`allowed_buttons_filter`], which limits the buttons this recognizer accepts.
    pub fn set_allowed_buttons_filter(
        self: Handle<Self>,
        app: &mut App,
        filter: impl Fn(i64) -> bool + 'static,
    ) {
        app.get_mut(self).recognizer.allowed_buttons_filter = Rc::new(filter);
    }

    /// Optional device specific configuration that takes precedence over framework defaults.
    pub fn gesture_settings(self: Handle<Self>, app: &App) -> Option<DeviceGestureSettings> {
        app.get(self).recognizer.gesture_settings
    }

    /// Sets [`gesture_settings`](Self::gesture_settings).
    pub fn set_gesture_settings(
        self: Handle<Self>,
        app: &mut App,
        settings: Option<DeviceGestureSettings>,
    ) {
        app.get_mut(self).recognizer.gesture_settings = settings;
    }

    /// Sets [`on_double_tap_down`](Self::on_double_tap_down).
    pub fn set_on_double_tap_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDownCallback>,
    ) {
        app.get_mut(self).on_double_tap_down = callback;
    }

    /// Sets [`on_double_tap`](Self::on_double_tap).
    pub fn set_on_double_tap(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureDoubleTapCallback>,
    ) {
        app.get_mut(self).on_double_tap = callback;
    }

    /// Sets [`on_double_tap_cancel`](Self::on_double_tap_cancel).
    pub fn set_on_double_tap_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapCancelCallback>,
    ) {
        app.get_mut(self).on_double_tap_cancel = callback;
    }

    /// A pointer has contacted the screen with a primary button at the same location twice in
    /// quick succession, which might be the start of a double tap.
    ///
    /// This triggers immediately after the down event of the second tap.
    ///
    /// If this recognizer doesn't win the arena, [`on_double_tap_cancel`]
    /// (Self::on_double_tap_cancel) is called next. Otherwise, [`on_double_tap`]
    /// (Self::on_double_tap) is called next.
    pub fn on_double_tap_down(self: Handle<Self>, app: &App) -> Option<GestureTapDownCallback> {
        app.get(self).on_double_tap_down.clone()
    }

    /// Called when the user has tapped the screen with a primary button at the same location
    /// twice in quick succession.
    ///
    /// This triggers when the pointer stops contacting the device after the second tap.
    pub fn on_double_tap(self: Handle<Self>, app: &App) -> Option<GestureDoubleTapCallback> {
        app.get(self).on_double_tap.clone()
    }

    /// A pointer that previously triggered [`on_double_tap_down`]
    /// (Self::on_double_tap_down) will not end up causing a double tap.
    ///
    /// This triggers once the gesture loses the arena if [`on_double_tap_down`]
    /// (Self::on_double_tap_down) has previously been triggered.
    ///
    /// If this recognizer wins the arena, [`on_double_tap`](Self::on_double_tap) is called
    /// instead.
    pub fn on_double_tap_cancel(self: Handle<Self>, app: &App) -> Option<GestureTapCancelCallback> {
        app.get(self).on_double_tap_cancel.clone()
    }

    fn has_callbacks(self: Handle<Self>, app: &App) -> bool {
        let recognizer = app.get(self);
        recognizer.on_double_tap_down.is_some()
            || recognizer.on_double_tap.is_some()
            || recognizer.on_double_tap_cancel.is_some()
    }

    fn invoke_callback<T>(
        self: Handle<Self>,
        app: &mut App,
        name: &str,
        callback: impl FnOnce(&mut App) -> T,
    ) -> Option<T> {
        GestureRecognizer::invoke_callback(self, app, name, callback)
    }

    /// Tracks `event` as a new tap, adding it to the arena and the router.
    fn track_tap(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        self.stop_double_tap_timer(app);
        let entry = GestureBinding::instance(app)
            .gesture_arena(app)
            .add(app, event.pointer, self);
        let min_time_elapsed = Rc::new(Cell::new(false));
        let min_time_timer = {
            let elapsed = Rc::clone(&min_time_elapsed);
            Timer::new(
                app,
                K_DOUBLE_TAP_MIN_TIME,
                Listener::new(move |_app| elapsed.set(true)),
            )
        };
        let transform: Option<Matrix4> = event.transform;
        let pointer = event.pointer;
        let tracker = TapTracker {
            pointer,
            entry,
            initial_global_position: event.position,
            initial_buttons: event.buttons,
            min_time_elapsed,
            _min_time_timer: min_time_timer,
            tracking: false,
            _gesture_settings: app.get(self).recognizer.gesture_settings(),
        };
        app.get_mut(self).trackers.insert(pointer, tracker);
        self.start_tracking_pointer(app, pointer, transform);
    }

    /// Registers the pointer's route so its moves and up reach this recognizer.
    fn start_tracking_pointer(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        GestureBinding::instance(app).pointer_router(app).add_route(
            app,
            pointer,
            PointerRoute::handle_method(self, Self::handle_event_route),
            transform,
        );
        if let Some(tracker) = app.get_mut(self).trackers.get_mut(&pointer) {
            tracker.tracking = true;
        }
    }

    /// Removes the pointer's route; Dart's `_freezeTracker`.
    fn freeze_tracker(self: Handle<Self>, app: &mut App, tracker: &mut TapTracker) {
        if tracker.tracking {
            tracker.tracking = false;
            GestureBinding::instance(app)
                .pointer_router(app)
                .remove_route(
                    app,
                    tracker.pointer,
                    &PointerRoute::handle_method(self, Self::handle_event_route),
                );
        }
    }

    /// Registers the end of the first tap: the sequence waits for a second one.
    fn register_first_tap(self: Handle<Self>, app: &mut App, mut tracker: TapTracker) {
        self.start_double_tap_timer(app);
        GestureBinding::instance(app)
            .gesture_arena(app)
            .hold(app, tracker.pointer);
        self.freeze_tracker(app, &mut tracker);
        self.clear_trackers(app);
        app.get_mut(self).first_tap = Some(tracker);
    }

    /// Registers the end of the second tap: both entries are accepted and the double tap fires.
    fn register_second_tap(self: Handle<Self>, app: &mut App, mut tracker: TapTracker) {
        let Some(first_entry) = app
            .get(self)
            .first_tap
            .as_ref()
            .map(|first| first.entry.clone())
        else {
            return;
        };
        first_entry.resolve(app, GestureDisposition::Accepted);
        tracker.entry.resolve(app, GestureDisposition::Accepted);
        self.freeze_tracker(app, &mut tracker);
        self.check_up(app);
        self.reset(app);
    }

    /// Rejects one tracker and, when it was the first tap, the whole sequence.
    fn reject_tracker(self: Handle<Self>, app: &mut App, mut tracker: TapTracker) {
        app.get_mut(self).trackers.remove(&tracker.pointer);
        tracker.entry.resolve(app, GestureDisposition::Rejected);
        self.freeze_tracker(app, &mut tracker);
        let first_pointer = app.get(self).first_tap.as_ref().map(|first| first.pointer);
        if let Some(first_pointer) = first_pointer {
            if tracker.pointer == first_pointer {
                self.reset(app);
            } else {
                self.check_cancel(app);
                if app.get(self).trackers.is_empty() {
                    self.reset(app);
                }
            }
        }
    }

    /// Rejects every outstanding tracker.
    fn clear_trackers(self: Handle<Self>, app: &mut App) {
        let trackers = std::mem::take(&mut app.get_mut(self).trackers);
        for tracker in trackers.into_values() {
            self.reject_tracker(app, tracker);
        }
        debug_assert!(app.get(self).trackers.is_empty());
    }

    /// Resets the recognizer to waiting for a first tap.
    fn reset(self: Handle<Self>, app: &mut App) {
        self.stop_double_tap_timer(app);
        if let Some(first) = app.get_mut(self).first_tap.take() {
            if !app.get(self).trackers.is_empty() {
                self.check_cancel(app);
            }
            let pointer = first.pointer;
            self.reject_tracker(app, first);
            GestureBinding::instance(app)
                .gesture_arena(app)
                .release(app, pointer);
        }
        self.clear_trackers(app);
    }

    fn start_double_tap_timer(self: Handle<Self>, app: &mut App) {
        if app.get(self).double_tap_timer.is_none() {
            let timer = Timer::new(
                app,
                K_DOUBLE_TAP_TIMEOUT,
                Listener::handle_method(self, Self::reset),
            );
            app.get_mut(self).double_tap_timer = Some(timer);
        }
    }

    fn stop_double_tap_timer(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).double_tap_timer.take() {
            timer.cancel(app);
        }
    }

    fn check_up(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = app.get(self).on_double_tap.clone() {
            self.invoke_callback(app, "onDoubleTap", |app| callback.call(app));
        }
    }

    fn check_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = app.get(self).on_double_tap_cancel.clone() {
            self.invoke_callback(app, "onDoubleTapCancel", |app| callback.call(app));
        }
    }
}

impl RecognizerLeafData for DoubleTapGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl RecognizerLeaf for DoubleTapGestureRecognizer {
    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        if app.get(self).first_tap.is_none() && !self.has_callbacks(app) {
            return false;
        }
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        let first = app.get(self).first_tap.as_ref().map(|first| {
            (
                first.initial_global_position,
                first.has_elapsed_min_time(),
                first.has_same_button(event.buttons),
            )
        });
        if let Some((position, elapsed, same_button)) = first {
            if (event.position - position).distance() > K_DOUBLE_TAP_SLOP {
                // Ignore out-of-bounds second taps.
                return;
            }
            if !elapsed || !same_button {
                // Restart when the second tap is too close to the first, or the buttons
                // mismatch.
                self.reset(app);
                self.track_tap(app, event);
                return;
            }
            if let Some(callback) = app.get(self).on_double_tap_down.clone() {
                let details = TapDownDetails::new(
                    event.position,
                    Some(event.local_position()),
                    Some(GestureRecognizer::kind_for_pointer(
                        self,
                        app,
                        event.pointer,
                    )),
                );
                self.invoke_callback(app, "onDoubleTapDown", |app| callback(app, details));
            }
        }
        self.track_tap(app, event);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        let pointer = event.pointer();
        let Some(tracker) = app.get_mut(self).trackers.remove(&pointer) else {
            return;
        };
        match &event {
            PointerEvent::Up(_) => {
                if app.get(self).first_tap.is_none() {
                    self.register_first_tap(app, tracker);
                } else {
                    self.register_second_tap(app, tracker);
                }
            }
            PointerEvent::Move(move_event) => {
                if tracker.is_within_global_tolerance(move_event.position, K_DOUBLE_TAP_TOUCH_SLOP)
                {
                    app.get_mut(self).trackers.insert(pointer, tracker);
                } else {
                    self.reject_tracker(app, tracker);
                }
            }
            PointerEvent::Cancel(_) => self.reject_tracker(app, tracker),
            _ => {
                app.get_mut(self).trackers.insert(pointer, tracker);
            }
        }
    }

    fn accept_gesture(self: Handle<Self>, _app: &mut App, _pointer: i64) {}

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, _event: &PointerDownEvent) {
        self.reset(app);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        let tracker = app.get_mut(self).trackers.remove(&pointer);
        if let Some(tracker) = tracker {
            self.reject_tracker(app, tracker);
            return;
        }
        let is_first = app
            .get(self)
            .first_tap
            .as_ref()
            .is_some_and(|first| first.pointer == pointer);
        if is_first {
            self.reset(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.reset(app);
        GestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "double tap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;
    use waft_foundation::{AppCell, HandleId};

    use crate::arena::GestureArenaMember;
    use crate::binding::GestureBinding;
    use crate::events::{K_SECONDARY_BUTTON, PointerMoveEvent, PointerUpEvent};
    use crate::tap::TapGestureRecognizer;

    fn down(pointer: i64, position: Offset) -> PointerDownEvent {
        PointerDownEvent {
            pointer,
            position,
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        }
    }

    fn secondary_down(pointer: i64, position: Offset) -> PointerDownEvent {
        PointerDownEvent {
            pointer,
            position,
            buttons: K_SECONDARY_BUTTON,
            ..PointerDownEvent::default()
        }
    }

    fn up(pointer: i64, position: Offset) -> PointerUpEvent {
        PointerUpEvent {
            pointer,
            position,
            ..PointerUpEvent::default()
        }
    }

    fn move_event(pointer: i64, position: Offset) -> PointerMoveEvent {
        PointerMoveEvent {
            pointer,
            position,
            down: true,
            buttons: K_PRIMARY_BUTTON,
            ..PointerMoveEvent::default()
        }
    }

    fn close_arena(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .close(app, pointer);
    }

    fn route(app: &mut App, event: PointerEvent) {
        GestureBinding::instance(app)
            .pointer_router(app)
            .route(app, event);
        app.drain_microtasks();
    }

    fn sweep(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .sweep(app, pointer);
    }

    /// The global and local position of the second tap's down event, as
    /// `onDoubleTapDown` saw them.
    type RecordedDetails = Rc<RefCell<Option<(Offset, Offset)>>>;

    fn set_handlers(
        gesture: Handle<DoubleTapGestureRecognizer>,
        app: &mut App,
        log: &Rc<RefCell<Vec<&'static str>>>,
        details: &RecordedDetails,
    ) {
        let down_log = Rc::clone(log);
        let down_details = Rc::clone(details);
        gesture.set_on_double_tap_down(
            app,
            Some(Rc::new(move |_app, value| {
                assert!(down_details.borrow().is_none());
                down_log.borrow_mut().push("down");
                *down_details.borrow_mut() = Some((value.global_position, value.local_position));
            })),
        );
        let tap_log = Rc::clone(log);
        gesture.set_on_double_tap(
            app,
            Some(Listener::new(move |_app| {
                tap_log.borrow_mut().push("double_tap");
            })),
        );
        let cancel_log = Rc::clone(log);
        gesture.set_on_double_tap_cancel(
            app,
            Some(Listener::new(move |_app| {
                cancel_log.borrow_mut().push("cancel");
            })),
        );
    }

    struct TestGestureArenaMember {
        accepted: bool,
        rejected: bool,
    }

    impl TestGestureArenaMember {
        fn new(app: &mut App) -> Ref<TestGestureArenaMember> {
            app.create(TestGestureArenaMember {
                accepted: false,
                rejected: false,
            })
        }

        fn accepted(self: Handle<Self>, app: &App) -> bool {
            app.get(self).accepted
        }
    }

    impl GestureArenaMember for Handle<TestGestureArenaMember> {
        fn accept_gesture(&self, app: &mut App, _pointer: i64) {
            app.get_mut(*self).accepted = true;
        }

        fn reject_gesture(&self, app: &mut App, _pointer: i64) {
            app.get_mut(*self).rejected = true;
        }

        fn member_id(&self) -> HandleId {
            self.id()
        }
    }

    #[test]
    fn should_recognize_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        let up2 = up(2, Offset::new(13.0, 11.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down2.position);
        assert_eq!(recorded.1, down2.local_position());
        assert_eq!(*log.borrow(), ["down"]);

        route(&mut app, PointerEvent::Down(down2));
        assert_eq!(*log.borrow(), ["down"]);

        route(&mut app, PointerEvent::Up(up2));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 2);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_recognize_double_tap_with_secondary_button() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        gesture.set_allowed_buttons_filter(&mut app, |buttons| buttons == K_SECONDARY_BUTTON);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        let up6 = up(6, Offset::new(11.0, 9.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up6));
        sweep(&mut app, 6);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down7 = secondary_down(7, Offset::new(10.0, 10.0));
        let up7 = up(7, Offset::new(11.0, 9.0));
        gesture.add_pointer(&mut app, down7.clone());
        close_arena(&mut app, 7);
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down7.position);
        assert_eq!(recorded.1, down7.local_position());

        route(&mut app, PointerEvent::Down(down7));
        assert_eq!(*log.borrow(), ["down"]);

        route(&mut app, PointerEvent::Up(up7));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 7);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn inter_tap_distance_cancels_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        let down3 = down(3, Offset::new(130.0, 130.0));
        gesture.add_pointer(&mut app, down3.clone());
        close_arena(&mut app, 3);
        route(&mut app, PointerEvent::Down(down3));
        route(&mut app, PointerEvent::Up(up(3, Offset::new(131.0, 129.0))));
        sweep(&mut app, 3);

        assert!(!log.borrow().contains(&"double_tap"));
        assert!(details.borrow().is_none());
        assert!(!log.borrow().contains(&"cancel"));

        gesture.dispose(&mut app);
    }

    #[test]
    fn intra_tap_distance_cancels_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down4 = down(4, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down4.clone());
        close_arena(&mut app, 4);
        route(&mut app, PointerEvent::Down(down4));
        route(
            &mut app,
            PointerEvent::Move(move_event(4, Offset::new(25.0, 25.0))),
        );
        route(&mut app, PointerEvent::Up(up(4, Offset::new(25.0, 25.0))));
        sweep(&mut app, 4);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1);
        close_arena(&mut app, 1);
        route(
            &mut app,
            PointerEvent::Down(down(2, Offset::new(12.0, 12.0))),
        );
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        assert!(!log.borrow().contains(&"double_tap"));
        assert!(details.borrow().is_none());
        assert!(!log.borrow().contains(&"cancel"));

        gesture.dispose(&mut app);
    }

    #[test]
    fn inter_tap_delay_cancels_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(Duration::from_millis(5000));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(!log.borrow().contains(&"double_tap"));
        assert!(details.borrow().is_none());
        assert!(!log.borrow().contains(&"cancel"));

        gesture.dispose(&mut app);
    }

    #[test]
    fn inter_tap_delay_resets_double_tap_allowing_third_tap_to_be_a_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(Duration::from_millis(5000));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down5 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down5.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down5.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down5.position);
        assert_eq!(recorded.1, down5.local_position());
        assert!(!log.borrow().contains(&"double_tap"));

        route(&mut app, PointerEvent::Up(up(5, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 5);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn intra_tap_delay_does_not_cancel_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        drop(app);
        cell.elapse(Duration::from_millis(1000));
        let mut app = cell.borrow_mut();
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);
        assert!(details.borrow().is_none());

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down2.position);
        assert_eq!(recorded.1, down2.local_position());
        assert!(!log.borrow().contains(&"double_tap"));

        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 2);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_not_recognize_two_overlapping_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1.clone()));

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2);
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down1));

        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_recognize_one_tap_of_group_followed_by_second_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1.clone()));

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2);
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down1.clone()));

        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down1.position);
        assert_eq!(recorded.1, down1.local_position());
        assert!(!log.borrow().contains(&"double_tap"));

        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 1);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_cancel_on_arena_reject_during_first_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        let member = TestGestureArenaMember::new(&mut app);
        let entry = GestureBinding::instance(&mut app).gesture_arena(&app).add(
            &mut app,
            1,
            member.handle(),
        );
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(member.accepted(&app));
        sweep(&mut app, 1);

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_cancel_on_arena_reject_between_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        let member = TestGestureArenaMember::new(&mut app);
        let entry = GestureBinding::instance(&mut app).gesture_arena(&app).add(
            &mut app,
            1,
            member.handle(),
        );
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(member.accepted(&app));

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_cancel_on_arena_reject_during_last_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        let member = TestGestureArenaMember::new(&mut app);
        let entry = GestureBinding::instance(&mut app).gesture_arena(&app).add(
            &mut app,
            1,
            member.handle(),
        );
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down2.position);
        assert_eq!(recorded.1, down2.local_position());
        assert!(!log.borrow().contains(&"cancel"));

        entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(member.accepted(&app));
        assert!(log.borrow().contains(&"cancel"));

        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);
        assert!(!log.borrow().contains(&"double_tap"));

        gesture.dispose(&mut app);
    }

    #[test]
    fn passive_gesture_should_trigger_on_double_tap_cancel() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        let member = TestGestureArenaMember::new(&mut app);
        GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .add(&mut app, 1, member.handle());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        assert!(!member.accepted(&app));
        drop(app);
        cell.elapse(Duration::from_millis(5000));
        let mut app = cell.borrow_mut();
        assert!(member.accepted(&app));

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn should_not_recognize_two_over_rapid_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(Duration::from_millis(10));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn over_rapid_taps_reset_double_tap_allowing_third_tap_to_be_a_double_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(Duration::from_millis(10));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);
        assert!(details.borrow().is_none());

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down5 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down5.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down5.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down5.position);
        assert_eq!(recorded.1, down5.local_position());
        assert!(!log.borrow().contains(&"double_tap"));

        route(&mut app, PointerEvent::Up(up(5, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "double_tap"]);
        sweep(&mut app, 5);
        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn button_change_should_interrupt_existing_sequence() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let interval = Duration::from_millis(100);
        assert!(interval * 2 < K_DOUBLE_TAP_TIMEOUT);
        assert!(interval > K_DOUBLE_TAP_MIN_TIME);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up(6, Offset::new(11.0, 9.0))));
        sweep(&mut app, 6);

        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();
        assert!(log.borrow().is_empty());

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn button_change_with_allowed_buttons_filter_should_interrupt_existing_sequence() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let interval = Duration::from_millis(100);
        assert!(interval * 2 < K_DOUBLE_TAP_TIMEOUT);
        assert!(interval > K_DOUBLE_TAP_MIN_TIME);

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();

        // The secondary down is ignored by the primary filter, but it still resets
        // the sequence.
        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up(6, Offset::new(11.0, 9.0))));
        sweep(&mut app, 6);

        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();
        assert!(log.borrow().is_empty());

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert!(log.borrow().is_empty());

        gesture.dispose(&mut app);
    }

    #[test]
    fn button_change_should_start_a_valid_sequence() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let details = Rc::new(RefCell::new(None));
        set_handlers(gesture.handle(), &mut app, &log, &details);

        let interval = Duration::from_millis(100);
        assert!(interval * 2 < K_DOUBLE_TAP_TIMEOUT);
        assert!(interval > K_DOUBLE_TAP_MIN_TIME);

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up(6, Offset::new(11.0, 9.0))));
        sweep(&mut app, 6);

        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();

        let down1 = down(1, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);

        assert!(log.borrow().is_empty());
        drop(app);
        cell.elapse(interval);
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        gesture.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2.clone()));
        let recorded = (*details.borrow()).expect("onDoubleTapDown fired");
        assert_eq!(recorded.0, down2.position);
        assert_eq!(recorded.1, down2.local_position());
        route(&mut app, PointerEvent::Up(up(2, Offset::new(13.0, 11.0))));
        sweep(&mut app, 2);

        assert_eq!(*log.borrow(), ["down", "double_tap"]);

        gesture.dispose(&mut app);
    }

    #[test]
    fn a_secondary_double_tap_should_not_trigger_primary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(RefCell::new(Vec::new()));
        let flag = Rc::clone(&recognized);
        gesture.set_on_double_tap(
            &mut app,
            Some(Listener::new(move |_app| flag.borrow_mut().push("primary"))),
        );

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up(6, Offset::new(11.0, 9.0))));
        sweep(&mut app, 6);

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down7 = secondary_down(7, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down7.clone());
        close_arena(&mut app, 7);
        route(&mut app, PointerEvent::Down(down7));
        route(&mut app, PointerEvent::Up(up(7, Offset::new(11.0, 9.0))));
        assert!(recognized.borrow().is_empty());

        gesture.dispose(&mut app);
        recognized.borrow_mut().clear();
    }

    #[test]
    fn buttons_filter_should_cancel_invalid_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = DoubleTapGestureRecognizer::new(&mut app);
        gesture.set_allowed_buttons_filter(&mut app, |_buttons| false);
        let recognized = Rc::new(RefCell::new(Vec::new()));
        let flag = Rc::clone(&recognized);
        gesture.set_on_double_tap(
            &mut app,
            Some(Listener::new(move |_app| flag.borrow_mut().push("primary"))),
        );

        let down7 = down(7, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down7.clone());
        close_arena(&mut app, 7);
        route(&mut app, PointerEvent::Down(down7));
        route(&mut app, PointerEvent::Up(up(7, Offset::new(11.0, 9.0))));
        sweep(&mut app, 7);

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        route(&mut app, PointerEvent::Up(up(6, Offset::new(11.0, 9.0))));
        assert!(recognized.borrow().is_empty());

        gesture.dispose(&mut app);
        recognized.borrow_mut().clear();
    }

    #[test]
    fn unfinished_double_tap_does_not_prevent_competing_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let double_tap = DoubleTapGestureRecognizer::new(&mut app);
        double_tap.set_on_double_tap(&mut app, Some(Listener::new(|_app| {})));
        let tap = TapGestureRecognizer::new(&mut app);
        let tap_count = Rc::new(Cell::new(0usize));
        let count = Rc::clone(&tap_count);
        tap.set_on_tap(
            &mut app,
            Some(Listener::new(move |_app| count.set(count.get() + 1))),
        );

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        double_tap.add_pointer(&mut app, down1.clone());
        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);

        drop(app);
        cell.elapse(Duration::from_millis(100));
        let mut app = cell.borrow_mut();

        let down2 = down(2, Offset::new(12.0, 12.0));
        let up2 = up(2, Offset::new(13.0, 11.0));
        tap.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2.clone()));
        route(&mut app, PointerEvent::Move(move_event(2, down2.position)));
        route(&mut app, PointerEvent::Up(up2));
        assert_eq!(tap_count.get(), 1);
        sweep(&mut app, 2);

        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(tap_count.get(), 1);

        tap.dispose(&mut app);
        double_tap.dispose(&mut app);
    }

    #[test]
    fn a_primary_double_tap_does_not_form_competition_with_a_secondary_tap_recognizer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let double_tap = DoubleTapGestureRecognizer::new(&mut app);
        double_tap.set_on_double_tap(&mut app, Some(Listener::new(|_app| {})));
        let tap_secondary = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(RefCell::new(Vec::new()));
        let flag = Rc::clone(&recognized);
        tap_secondary.set_on_secondary_tap_down(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                flag.borrow_mut().push("tapSecondary");
            })),
        );

        let down6 = secondary_down(6, Offset::new(10.0, 10.0));
        double_tap.add_pointer(&mut app, down6.clone());
        tap_secondary.add_pointer(&mut app, down6.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down6));
        assert_eq!(*recognized.borrow(), ["tapSecondary"]);

        tap_secondary.dispose(&mut app);
        double_tap.dispose(&mut app);
        recognized.borrow_mut().clear();
    }

    #[test]
    fn a_primary_double_tap_forms_competition_with_a_primary_tap_recognizer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let double_tap = DoubleTapGestureRecognizer::new(&mut app);
        double_tap.set_on_double_tap(&mut app, Some(Listener::new(|_app| {})));
        let tap_primary = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(RefCell::new(Vec::new()));
        let flag = Rc::clone(&recognized);
        tap_primary.set_on_tap_down(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                flag.borrow_mut().push("tapPrimary");
            })),
        );

        let down1 = down(1, Offset::new(10.0, 10.0));
        double_tap.add_pointer(&mut app, down1.clone());
        tap_primary.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        assert!(recognized.borrow().is_empty());

        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(*recognized.borrow(), ["tapPrimary"]);

        tap_primary.dispose(&mut app);
        double_tap.dispose(&mut app);
        recognized.borrow_mut().clear();
    }
}

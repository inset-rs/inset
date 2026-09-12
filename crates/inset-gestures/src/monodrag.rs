//! Flutter counterpart: `gestures/monodrag.dart`.
//!
//! Leaf objects. `DragGestureRecognizer`'s fields are the [`DragData`] bag and
//! its bodies are the [`DragGestureRecognizer`] trait, blanket-implemented for
//! every drag leaf; `super` reads `DragGestureRecognizer::handle_event(self, ..)`
//! at Dart's position. The bags above it and their `super` namespaces are in
//! `recognizer.rs`.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use inset_embedder::{Matrix4, Offset, PointerDeviceKind};
use inset_foundation::{App, Handle, Listener};
use inset_scheduler::SchedulerBinding;

use crate::arena::GestureDisposition;
use crate::constants::{K_MAX_FLING_VELOCITY, K_MIN_FLING_VELOCITY};
use crate::drag_details::{
    DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, GestureDragDownCallback,
    GestureDragStartCallback, GestureDragUpdateCallback,
};
use crate::events::{
    K_PRIMARY_BUTTON, PointerDownEvent, PointerEvent, PointerPanZoomStartEvent, compute_hit_slop,
    compute_pan_slop, transform_delta_via_positions,
};
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    DragStartBehavior, GestureRecognizer, GestureRecognizerData, MultitouchDragStrategy,
    OffsetPair, OneSequenceData, OneSequenceGestureRecognizer, OneSequenceLeafData, RecognizerLeaf,
    RecognizerLeafData,
};
use crate::team::GestureArenaTeam;
use crate::velocity::{Velocity, VelocityEstimate};
use crate::velocity_tracker::{AnyVelocityTracker, VelocityTracker};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragState {
    Ready,
    Possible,
    Accepted,
}

/// Signature for when a pointer that was previously in contact with the screen
/// and moving is no longer in contact with the screen.
///
/// The velocity at which the pointer was moving when it stopped contacting
/// the screen is available in the `details`.
///
/// Used by [`DragGestureRecognizer::set_on_end`].
pub type GestureDragEndCallback = inset_foundation::ValueChanged<DragEndDetails>;

/// Signature for when the pointer that previously triggered a
/// [`GestureDragDownCallback`] did not complete.
///
/// Used by [`DragGestureRecognizer::set_on_cancel`].
pub type GestureDragCancelCallback = Listener;

/// Signature for a function that builds a [`VelocityTracker`].
///
/// Used by [`DragGestureRecognizer::set_velocity_tracker_builder`].
pub type GestureVelocityTrackerBuilder = Rc<dyn Fn(&PointerEvent) -> Box<dyn AnyVelocityTracker>>;

fn default_builder(event: &PointerEvent) -> Box<dyn AnyVelocityTracker> {
    Box::new(VelocityTracker::with_kind(event.kind()))
}

/// Accept the input if, and only if, [`K_PRIMARY_BUTTON`] is pressed.
fn default_button_accept_behavior(buttons: i64) -> bool {
    buttons == K_PRIMARY_BUTTON
}

/// Dart's `num.sign`: `-1.0`, `0.0` or `1.0`. `f64::signum` answers `±1.0` for
/// `±0.0`, which would move `global_distance_moved` on a zero primary delta.
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// Field bag for Dart's `DragGestureRecognizer`.
pub struct DragData {
    drag_start_behavior: DragStartBehavior,
    multitouch_drag_strategy: MultitouchDragStrategy,
    on_down: Option<GestureDragDownCallback>,
    on_start: Option<GestureDragStartCallback>,
    on_update: Option<GestureDragUpdateCallback>,
    on_end: Option<GestureDragEndCallback>,
    on_cancel: Option<GestureDragCancelCallback>,
    min_fling_distance: Option<f64>,
    min_fling_velocity: Option<f64>,
    max_fling_velocity: Option<f64>,
    only_accept_drag_on_threshold: bool,
    velocity_tracker_builder: GestureVelocityTrackerBuilder,
    state: DragState,
    /// Dart declares this `late`; `add_pointer` writes it on the `Ready`
    /// transition, before any read.
    initial_position: OffsetPair,
    pending_drag_offset: OffsetPair,
    last_position: OffsetPair,
    last_pending_event_timestamp: Option<Duration>,
    /// The buttons sent by `PointerDownEvent`. If a `PointerMoveEvent` comes
    /// with a different set of buttons, the gesture is canceled.
    initial_buttons: Option<i64>,
    last_transform: Option<Matrix4>,
    global_distance_moved: f64,
    has_drag_threshold_been_met: bool,
    velocity_trackers: HashMap<i64, Box<dyn AnyVelocityTracker>>,
    /// The move delta of each pointer before the next frame. The key is the
    /// pointer ID. It is cleared whenever a new batch of pointer events is
    /// detected.
    move_delta_before_frame: IndexMap<i64, Offset>,
    /// The timestamp of all events of the current frame. On an event with a
    /// different timestamp, the event is considered a new batch.
    frame_time_stamp: Option<Duration>,
    last_updated_delta_for_pan: Offset,
    accepted_active_pointers: Vec<i64>,
    /// This value is used when the multitouch strategy is
    /// [`MultitouchDragStrategy::LatestPointer`]; it keeps track of the last
    /// accepted pointer. If this active pointer leaves up, it will be set to the
    /// first accepted pointer.
    active_pointer: Option<i64>,
}

impl DragData {
    /// Initializes the drag recognizer: no callbacks, `DragStartBehavior::Start`, and the
    /// default velocity tracker.
    pub fn new() -> DragData {
        DragData {
            drag_start_behavior: DragStartBehavior::Start,
            multitouch_drag_strategy: MultitouchDragStrategy::LatestPointer,
            on_down: None,
            on_start: None,
            on_update: None,
            on_end: None,
            on_cancel: None,
            min_fling_distance: None,
            min_fling_velocity: None,
            max_fling_velocity: None,
            only_accept_drag_on_threshold: false,
            velocity_tracker_builder: Rc::new(default_builder),
            state: DragState::Ready,
            initial_position: OffsetPair::ZERO,
            pending_drag_offset: OffsetPair::ZERO,
            last_position: OffsetPair::ZERO,
            last_pending_event_timestamp: None,
            initial_buttons: None,
            last_transform: None,
            global_distance_moved: 0.0,
            has_drag_threshold_been_met: false,
            velocity_trackers: HashMap::new(),
            move_delta_before_frame: IndexMap::new(),
            frame_time_stamp: None,
            last_updated_delta_for_pan: Offset::ZERO,
            accepted_active_pointers: Vec::new(),
            active_pointer: None,
        }
    }
}

impl Default for DragData {
    fn default() -> DragData {
        DragData::new()
    }
}

/// The `DragGestureRecognizer` field bag on every drag leaf.
pub trait DragLeafData: OneSequenceLeafData {
    /// The leaf's [`DragGestureRecognizer`] bag.
    fn drag(&self) -> &DragData;

    /// The leaf's [`DragGestureRecognizer`] bag, mutably.
    fn drag_mut(&mut self) -> &mut DragData;
}

/// The axis (horizontal or vertical) corresponding to the primary drag
/// direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragDirection {
    /// The primary drag direction is horizontal.
    Horizontal,
    /// The primary drag direction is vertical.
    Vertical,
}

/// Virtuals `DragGestureRecognizer`'s bodies call on its leaves.
pub trait DragLeaf: RecognizerLeaf + DragLeafData {
    /// Determines if a gesture is a fling or not based on velocity.
    ///
    /// A fling calls its gesture end callback with a velocity, allowing the
    /// provider of the callback to respond by carrying the gesture forward with
    /// inertia, for example.
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool;

    /// Determines if a gesture is a fling or not, and if so its effective
    /// velocity.
    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails>;

    /// Returns the effective delta that should be considered for the incoming
    /// `delta`.
    ///
    /// The delta received by an event might contain both the x and y components
    /// greater than zero, and a one-axis drag recognizer only cares about one of
    /// them.
    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset;

    /// Returns the value for the primary axis from the given `value`.
    ///
    /// Returns `None` if the recognizer does not have a primary axis.
    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64>;

    /// The axis corresponding to the primary drag direction.
    ///
    /// [`PanGestureRecognizer`] returns `None`.
    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        let _ = app;
        None
    }

    /// Whether [`DragGestureRecognizer::global_distance_moved`] is big enough to
    /// accept the gesture.
    ///
    /// If this method returns `true`, it means this recognizer should declare
    /// win in the gesture arena.
    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool;
}

/// Recognizes movement.
///
/// In contrast to `MultiDragGestureRecognizer`, a [`DragGestureRecognizer`]
/// recognizes a single gesture sequence for all the pointers it watches, which
/// means that the recognizer has at most one drag sequence active at any given
/// time regardless of how many pointers are in contact with the screen.
///
/// [`DragGestureRecognizer`] is not intended to be used directly. Instead,
/// consider using one of its subclasses to recognize specific types for drag
/// gestures.
///
/// [`DragGestureRecognizer`] competes on pointer events only when it has at
/// least one non-none callback. If it has no callbacks, it is a no-op.
///
/// See also:
///
///  * [`HorizontalDragGestureRecognizer`], for left and right drags.
///  * [`VerticalDragGestureRecognizer`], for up and down drags.
///  * [`PanGestureRecognizer`], for drags that are not locked to a single axis.
pub trait DragGestureRecognizer: Sized + 'static {
    /// Configure the behavior of offsets passed to
    /// [`set_on_start`](Self::set_on_start).
    ///
    /// If set to [`DragStartBehavior::Start`], the start callback will be called
    /// with the position of the pointer at the time this gesture recognizer won
    /// the arena. If [`DragStartBehavior::Down`], it will be called with the
    /// position of the first detected down event for the pointer. When there are
    /// no other gestures competing with this gesture in the arena, there's no
    /// difference in behavior between the two settings.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    fn drag_start_behavior(self: Handle<Self>, app: &App) -> DragStartBehavior;

    /// Sets [`drag_start_behavior`](Self::drag_start_behavior).
    fn set_drag_start_behavior(self: Handle<Self>, app: &mut App, value: DragStartBehavior);

    /// Configure the multi-finger drag strategy on multi-touch devices.
    ///
    /// By default, the strategy is [`MultitouchDragStrategy::LatestPointer`].
    fn multitouch_drag_strategy(self: Handle<Self>, app: &App) -> MultitouchDragStrategy;

    /// Sets [`multitouch_drag_strategy`](Self::multitouch_drag_strategy).
    fn set_multitouch_drag_strategy(
        self: Handle<Self>,
        app: &mut App,
        value: MultitouchDragStrategy,
    );

    /// A pointer has contacted the screen with a primary button and might begin
    /// to move.
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`DragDownDetails`] object.
    fn set_on_down(self: Handle<Self>, app: &mut App, callback: Option<GestureDragDownCallback>);

    /// A pointer has contacted the screen with a primary button and has begun to
    /// move.
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`DragStartDetails`] object. The
    /// [`drag_start_behavior`](Self::drag_start_behavior) determines this
    /// position.
    fn set_on_start(self: Handle<Self>, app: &mut App, callback: Option<GestureDragStartCallback>);

    /// A pointer that is in contact with the screen with a primary button and
    /// moving has moved again.
    ///
    /// The distance traveled by the pointer since the last update is provided in
    /// the callback's `details` argument, which is a [`DragUpdateDetails`]
    /// object.
    ///
    /// If this gesture recognizer recognizes movement on a single axis (a
    /// [`VerticalDragGestureRecognizer`] or [`HorizontalDragGestureRecognizer`]),
    /// then `details` will reflect movement only on that axis and its
    /// [`DragUpdateDetails::primary_delta`] will be non-none. If this gesture
    /// recognizer recognizes movement in all directions (a
    /// [`PanGestureRecognizer`]), then `details` will reflect movement on both
    /// axes and its [`DragUpdateDetails::primary_delta`] will be none.
    fn set_on_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureDragUpdateCallback>,
    );

    /// A pointer that was previously in contact with the screen with a primary
    /// button and moving is no longer in contact with the screen and was moving
    /// at a specific velocity when it stopped contacting the screen.
    ///
    /// The velocity is provided in the callback's `details` argument, which is a
    /// [`DragEndDetails`] object.
    fn set_on_end(self: Handle<Self>, app: &mut App, callback: Option<GestureDragEndCallback>);

    /// The pointer that previously triggered [`set_on_down`](Self::set_on_down)
    /// did not complete.
    fn set_on_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureDragCancelCallback>,
    );

    /// The minimum distance an input pointer drag must have moved to be
    /// considered a fling gesture.
    ///
    /// This value is typically compared with the distance traveled along the
    /// scrolling axis. If none then
    /// [`K_TOUCH_SLOP`](crate::K_TOUCH_SLOP) is used.
    fn min_fling_distance(self: Handle<Self>, app: &App) -> Option<f64>;

    /// Sets [`min_fling_distance`](Self::min_fling_distance).
    fn set_min_fling_distance(self: Handle<Self>, app: &mut App, value: Option<f64>);

    /// The minimum velocity for an input pointer drag to be considered fling.
    ///
    /// This value is typically compared with the magnitude of fling gesture's
    /// velocity along the scrolling axis. If none then
    /// [`K_MIN_FLING_VELOCITY`] is used.
    fn min_fling_velocity(self: Handle<Self>, app: &App) -> Option<f64>;

    /// Sets [`min_fling_velocity`](Self::min_fling_velocity).
    fn set_min_fling_velocity(self: Handle<Self>, app: &mut App, value: Option<f64>);

    /// Fling velocity magnitudes will be clamped to this value.
    ///
    /// If none then [`K_MAX_FLING_VELOCITY`] is used.
    fn max_fling_velocity(self: Handle<Self>, app: &App) -> Option<f64>;

    /// Sets [`max_fling_velocity`](Self::max_fling_velocity).
    fn set_max_fling_velocity(self: Handle<Self>, app: &mut App, value: Option<f64>);

    /// Whether the drag threshold should be met before dispatching any drag
    /// callbacks.
    ///
    /// The drag threshold is met when the global distance traveled by a pointer
    /// has exceeded the defined threshold on the relevant axis, i.e. y-axis for
    /// the [`VerticalDragGestureRecognizer`], x-axis for the
    /// [`HorizontalDragGestureRecognizer`], and the entire plane for
    /// [`PanGestureRecognizer`].
    ///
    /// If true, the drag callbacks will only be dispatched when this recognizer
    /// has won the arena and the drag threshold has been met.
    ///
    /// If false, the drag callbacks will be dispatched immediately when this
    /// recognizer has won the arena.
    ///
    /// This value defaults to false.
    fn only_accept_drag_on_threshold(self: Handle<Self>, app: &App) -> bool;

    /// Sets [`only_accept_drag_on_threshold`](Self::only_accept_drag_on_threshold).
    fn set_only_accept_drag_on_threshold(self: Handle<Self>, app: &mut App, value: bool);

    /// Determines the type of velocity estimation method to use for a potential
    /// drag gesture, when a new pointer is added.
    ///
    /// To estimate the velocity of a gesture, a [`DragGestureRecognizer`] calls
    /// the builder when it starts to track a new pointer in
    /// [`add_allowed_pointer`](Self::add_allowed_pointer), and adds subsequent
    /// updates on the pointer to the resulting velocity tracker, until the
    /// gesture recognizer stops tracking the pointer.
    ///
    /// If left unspecified the default builder creates a new [`VelocityTracker`]
    /// for every pointer added.
    fn velocity_tracker_builder(self: Handle<Self>, app: &App) -> GestureVelocityTrackerBuilder;

    /// Sets [`velocity_tracker_builder`](Self::velocity_tracker_builder).
    fn set_velocity_tracker_builder(
        self: Handle<Self>,
        app: &mut App,
        builder: GestureVelocityTrackerBuilder,
    );

    /// The local and global offsets of the last pointer event received.
    ///
    /// It is used to create the [`DragEndDetails`], which provides information
    /// about the end of a drag gesture.
    fn last_position(self: Handle<Self>, app: &App) -> OffsetPair;

    /// When asserts are enabled, returns the last tracked pending event
    /// timestamp for this recognizer. Otherwise, returns none.
    ///
    /// This getter is intended for use in framework unit tests. Applications
    /// must not depend on its value.
    fn debug_last_pending_event_timestamp(self: Handle<Self>, app: &App) -> Option<Duration>;

    /// Distance moved in the global coordinate space of the screen in drag
    /// direction.
    ///
    /// If drag is only allowed along a defined axis, this value may be negative
    /// to differentiate the direction of the drag.
    fn global_distance_moved(self: Handle<Self>, app: &App) -> f64;

    /// Determines if a gesture is a fling or not based on velocity.
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool;

    /// Determines if a gesture is a fling or not, and if so its effective
    /// velocity.
    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails>;

    /// Whether [`global_distance_moved`](Self::global_distance_moved) is big
    /// enough to accept the gesture.
    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool;

    /// Checks whether or not a pointer is allowed to be tracked by this
    /// recognizer.
    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool;

    /// Registers a new pointer that is allowed by this recognizer.
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent);

    /// Registers a new pointer pan/zoom that is allowed by this recognizer.
    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    );

    /// Called when a pointer event is routed to this recognizer.
    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent);

    /// Called when this member wins the arena for the given pointer id.
    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64);

    /// Called when this member loses the arena for the given pointer id.
    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64);

    /// Called when the number of pointers this recognizer is tracking changes
    /// from one to zero.
    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64);

    /// Releases any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App);
}

impl<L: DragLeaf> DragGestureRecognizer for L {
    fn drag_start_behavior(self: Handle<Self>, app: &App) -> DragStartBehavior {
        app.get(self).drag().drag_start_behavior
    }

    fn set_drag_start_behavior(self: Handle<Self>, app: &mut App, value: DragStartBehavior) {
        app.get_mut(self).drag_mut().drag_start_behavior = value;
    }

    fn multitouch_drag_strategy(self: Handle<Self>, app: &App) -> MultitouchDragStrategy {
        app.get(self).drag().multitouch_drag_strategy
    }

    fn set_multitouch_drag_strategy(
        self: Handle<Self>,
        app: &mut App,
        value: MultitouchDragStrategy,
    ) {
        app.get_mut(self).drag_mut().multitouch_drag_strategy = value;
    }

    fn set_on_down(self: Handle<Self>, app: &mut App, callback: Option<GestureDragDownCallback>) {
        app.get_mut(self).drag_mut().on_down = callback;
    }

    fn set_on_start(self: Handle<Self>, app: &mut App, callback: Option<GestureDragStartCallback>) {
        app.get_mut(self).drag_mut().on_start = callback;
    }

    fn set_on_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureDragUpdateCallback>,
    ) {
        app.get_mut(self).drag_mut().on_update = callback;
    }

    fn set_on_end(self: Handle<Self>, app: &mut App, callback: Option<GestureDragEndCallback>) {
        app.get_mut(self).drag_mut().on_end = callback;
    }

    fn set_on_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureDragCancelCallback>,
    ) {
        app.get_mut(self).drag_mut().on_cancel = callback;
    }

    fn min_fling_distance(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).drag().min_fling_distance
    }

    fn set_min_fling_distance(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        app.get_mut(self).drag_mut().min_fling_distance = value;
    }

    fn min_fling_velocity(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).drag().min_fling_velocity
    }

    fn set_min_fling_velocity(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        app.get_mut(self).drag_mut().min_fling_velocity = value;
    }

    fn max_fling_velocity(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).drag().max_fling_velocity
    }

    fn set_max_fling_velocity(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        app.get_mut(self).drag_mut().max_fling_velocity = value;
    }

    fn only_accept_drag_on_threshold(self: Handle<Self>, app: &App) -> bool {
        app.get(self).drag().only_accept_drag_on_threshold
    }

    fn set_only_accept_drag_on_threshold(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).drag_mut().only_accept_drag_on_threshold = value;
    }

    fn velocity_tracker_builder(self: Handle<Self>, app: &App) -> GestureVelocityTrackerBuilder {
        Rc::clone(&app.get(self).drag().velocity_tracker_builder)
    }

    fn set_velocity_tracker_builder(
        self: Handle<Self>,
        app: &mut App,
        builder: GestureVelocityTrackerBuilder,
    ) {
        app.get_mut(self).drag_mut().velocity_tracker_builder = builder;
    }

    fn last_position(self: Handle<Self>, app: &App) -> OffsetPair {
        app.get(self).drag().last_position
    }

    fn debug_last_pending_event_timestamp(self: Handle<Self>, app: &App) -> Option<Duration> {
        let mut last_pending_event_timestamp = None;
        if cfg!(debug_assertions) {
            last_pending_event_timestamp = app.get(self).drag().last_pending_event_timestamp;
        }
        last_pending_event_timestamp
    }

    fn global_distance_moved(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).drag().global_distance_moved
    }

    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        DragLeaf::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        DragLeaf::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        DragLeaf::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        let recognizer = app.get(self).drag();
        if recognizer.initial_buttons.is_none() {
            if recognizer.on_down.is_none()
                && recognizer.on_start.is_none()
                && recognizer.on_update.is_none()
                && recognizer.on_end.is_none()
                && recognizer.on_cancel.is_none()
            {
                return false;
            }
        } else if Some(event.buttons) != recognizer.initial_buttons {
            // There can be multiple drags simultaneously. Their effects are combined.
            return false;
        }
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        OneSequenceGestureRecognizer::add_allowed_pointer(self, app, &event);
        if app.get(self).drag().state == DragState::Ready {
            app.get_mut(self).drag_mut().initial_buttons = Some(event.buttons);
        }
        add_pointer(self, app, &PointerEvent::Down(event));
    }

    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    ) {
        self.start_tracking_pointer(app, event.pointer, event.transform);
        if app.get(self).drag().state == DragState::Ready {
            app.get_mut(self).drag_mut().initial_buttons = Some(K_PRIMARY_BUTTON);
        }
        add_pointer(self, app, &PointerEvent::PanZoomStart(event));
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        debug_assert!(app.get(self).drag().state != DragState::Ready);
        if !event.synthesized()
            && matches!(
                event,
                PointerEvent::Down(_)
                    | PointerEvent::Move(_)
                    | PointerEvent::PanZoomStart(_)
                    | PointerEvent::PanZoomUpdate(_)
            )
        {
            let position = match &event {
                PointerEvent::PanZoomStart(_) => Offset::ZERO,
                PointerEvent::PanZoomUpdate(pan_zoom) => pan_zoom.pan,
                _ => event.local_position(),
            };
            let time_stamp = event.time_stamp();
            app.get_mut(self)
                .drag_mut()
                .velocity_trackers
                .get_mut(&event.pointer())
                .unwrap()
                .add_position(time_stamp, position);
        }
        if let PointerEvent::Move(moved) = &event
            && Some(moved.buttons) != app.get(self).drag().initial_buttons
        {
            give_up_pointer(self, app, moved.pointer);
            return;
        }
        if matches!(
            event,
            PointerEvent::Move(_) | PointerEvent::PanZoomUpdate(_)
        ) && should_track_move_event(self, app, event.pointer())
        {
            let (delta, local_delta, position, local_position) = match &event {
                PointerEvent::Move(moved) => (
                    moved.delta,
                    moved.local_delta(),
                    moved.position,
                    moved.local_position(),
                ),
                PointerEvent::PanZoomUpdate(pan_zoom) => (
                    pan_zoom.pan_delta,
                    pan_zoom.local_pan_delta(),
                    pan_zoom.position + pan_zoom.pan,
                    pan_zoom.local_position() + pan_zoom.local_pan(),
                ),
                _ => unreachable!(),
            };
            app.get_mut(self).drag_mut().last_position = OffsetPair::new(local_position, position);
            let resolved_delta =
                resolve_local_delta_for_multitouch(self, app, event.pointer(), local_delta);
            match app.get(self).drag().state {
                DragState::Ready | DragState::Possible => {
                    let pending = app.get(self).drag().pending_drag_offset;
                    app.get_mut(self).drag_mut().pending_drag_offset =
                        pending + OffsetPair::new(local_delta, delta);
                    app.get_mut(self).drag_mut().last_pending_event_timestamp =
                        Some(event.time_stamp());
                    app.get_mut(self).drag_mut().last_transform = event.transform();
                    let moved_locally = DragLeaf::get_delta_for_details(self, app, local_delta);
                    let local_to_global_transform =
                        event.transform().and_then(|transform| transform.invert());
                    let primary = DragLeaf::get_primary_value_from_offset(self, app, moved_locally);
                    app.get_mut(self).drag_mut().global_distance_moved +=
                        transform_delta_via_positions(
                            local_position,
                            None,
                            moved_locally,
                            local_to_global_transform,
                        )
                        .distance()
                            * sign(primary.unwrap_or(1.0));
                    let touch_slop = app
                        .get(self)
                        .recognizer()
                        .gesture_settings
                        .and_then(|settings| settings.touch_slop);
                    if DragLeaf::has_sufficient_global_distance_to_accept(
                        self,
                        app,
                        event.kind(),
                        touch_slop,
                    ) {
                        app.get_mut(self).drag_mut().has_drag_threshold_been_met = true;
                        if app
                            .get(self)
                            .drag()
                            .accepted_active_pointers
                            .contains(&event.pointer())
                        {
                            check_drag(self, app, event.pointer());
                        } else {
                            self.resolve(app, GestureDisposition::Accepted);
                        }
                    }
                }
                DragState::Accepted => {
                    let delta_for_details =
                        DragLeaf::get_delta_for_details(self, app, resolved_delta);
                    let primary_delta =
                        DragLeaf::get_primary_value_from_offset(self, app, resolved_delta);
                    check_update(
                        self,
                        app,
                        Some(event.time_stamp()),
                        delta_for_details,
                        primary_delta,
                        position,
                        Some(local_position),
                        event.pointer(),
                    );
                }
            }
            record_move_delta_for_multitouch(self, app, event.pointer(), local_delta);
        }
        if matches!(
            event,
            PointerEvent::Up(_) | PointerEvent::Cancel(_) | PointerEvent::PanZoomEnd(_)
        ) {
            give_up_pointer(self, app, event.pointer());
        }
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        debug_assert!(
            !app.get(self)
                .drag()
                .accepted_active_pointers
                .contains(&pointer)
        );
        app.get_mut(self)
            .drag_mut()
            .accepted_active_pointers
            .push(pointer);
        app.get_mut(self).drag_mut().active_pointer = Some(pointer);
        let drag = app.get(self).drag();
        if !drag.only_accept_drag_on_threshold || drag.has_drag_threshold_been_met {
            check_drag(self, app, pointer);
        }
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        give_up_pointer(self, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        debug_assert!(app.get(self).drag().state != DragState::Ready);
        match app.get(self).drag().state {
            DragState::Ready => {}
            DragState::Possible => {
                self.resolve(app, GestureDisposition::Rejected);
                check_cancel(self, app);
            }
            DragState::Accepted => check_end(self, app, pointer),
        }
        let drag = app.get_mut(self).drag_mut();
        drag.has_drag_threshold_been_met = false;
        drag.velocity_trackers.clear();
        drag.initial_buttons = None;
        drag.state = DragState::Ready;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).drag_mut().velocity_trackers.clear();
        OneSequenceGestureRecognizer::dispose(self, app);
    }
}

/// Dart's private `DragGestureRecognizer._addPointer`.
fn add_pointer<L: DragLeaf>(this: Handle<L>, app: &mut App, event: &PointerEvent) {
    let builder = Rc::clone(&app.get(this).drag().velocity_tracker_builder);
    let tracker = builder(event);
    app.get_mut(this)
        .drag_mut()
        .velocity_trackers
        .insert(event.pointer(), tracker);
    match app.get(this).drag().state {
        DragState::Ready => {
            let initial_position = OffsetPair::new(event.local_position(), event.position());
            let drag = app.get_mut(this).drag_mut();
            drag.state = DragState::Possible;
            drag.initial_position = initial_position;
            drag.last_position = initial_position;
            drag.pending_drag_offset = OffsetPair::ZERO;
            drag.global_distance_moved = 0.0;
            drag.last_pending_event_timestamp = Some(event.time_stamp());
            drag.last_transform = event.transform();
            check_down(this, app);
        }
        DragState::Possible => {}
        DragState::Accepted => this.resolve(app, GestureDisposition::Accepted),
    }
}

fn should_track_move_event<L: DragLeaf>(this: Handle<L>, app: &App, pointer: i64) -> bool {
    let drag = app.get(this).drag();
    match drag.multitouch_drag_strategy {
        MultitouchDragStrategy::SumAllPointers
        | MultitouchDragStrategy::AverageBoundaryPointers => true,
        MultitouchDragStrategy::LatestPointer => {
            drag.active_pointer.is_none() || Some(pointer) == drag.active_pointer
        }
    }
}

fn record_move_delta_for_multitouch<L: DragLeaf>(
    this: Handle<L>,
    app: &mut App,
    pointer: i64,
    local_delta: Offset,
) {
    if app.get(this).drag().multitouch_drag_strategy
        != MultitouchDragStrategy::AverageBoundaryPointers
    {
        debug_assert!(app.get(this).drag().frame_time_stamp.is_none());
        debug_assert!(app.get(this).drag().move_delta_before_frame.is_empty());
        return;
    }

    if cfg!(debug_assertions) {
        let frame_time_stamp = app.get(this).drag().frame_time_stamp;
        debug_assert_eq!(
            frame_time_stamp,
            Some(SchedulerBinding::current_system_frame_time_stamp(app))
        );
    }

    if app.get(this).drag().state != DragState::Accepted || local_delta == Offset::ZERO {
        return;
    }

    let entry = app
        .get_mut(this)
        .drag_mut()
        .move_delta_before_frame
        .entry(pointer)
        .or_insert(Offset::ZERO);
    *entry = *entry + local_delta;
}

fn get_sum_delta<L: DragLeaf>(
    this: Handle<L>,
    app: &App,
    pointer: i64,
    positive: bool,
    axis: DragDirection,
) -> f64 {
    let Some(offset) = app.get(this).drag().move_delta_before_frame.get(&pointer) else {
        return 0.0;
    };
    if positive {
        if axis == DragDirection::Vertical {
            offset.dy().max(0.0)
        } else {
            offset.dx().max(0.0)
        }
    } else if axis == DragDirection::Vertical {
        offset.dy().min(0.0)
    } else {
        offset.dx().min(0.0)
    }
}

fn get_max_sum_delta_pointer<L: DragLeaf>(
    this: Handle<L>,
    app: &App,
    positive: bool,
    axis: DragDirection,
) -> Option<i64> {
    if app.get(this).drag().move_delta_before_frame.is_empty() {
        return None;
    }

    let pointers: Vec<i64> = app
        .get(this)
        .drag()
        .move_delta_before_frame
        .keys()
        .copied()
        .collect();
    let mut ret: Option<i64> = None;
    let mut max = 0.0;
    for pointer in pointers {
        let sum = get_sum_delta(this, app, pointer, positive, axis);
        match ret {
            None => {
                ret = Some(pointer);
                max = sum;
            }
            Some(_) => {
                if positive {
                    if sum > max {
                        ret = Some(pointer);
                        max = sum;
                    }
                } else if sum < max {
                    ret = Some(pointer);
                    max = sum;
                }
            }
        }
    }
    debug_assert!(ret.is_some());
    ret
}

fn resolve_local_delta_for_multitouch<L: DragLeaf>(
    this: Handle<L>,
    app: &mut App,
    pointer: i64,
    local_delta: Offset,
) -> Offset {
    if app.get(this).drag().multitouch_drag_strategy
        != MultitouchDragStrategy::AverageBoundaryPointers
    {
        if app.get(this).drag().frame_time_stamp.is_some() {
            let drag = app.get_mut(this).drag_mut();
            drag.move_delta_before_frame.clear();
            drag.frame_time_stamp = None;
            drag.last_updated_delta_for_pan = Offset::ZERO;
        }
        return local_delta;
    }

    let current_system_frame_time_stamp = SchedulerBinding::current_system_frame_time_stamp(app);
    if app.get(this).drag().frame_time_stamp != Some(current_system_frame_time_stamp) {
        let drag = app.get_mut(this).drag_mut();
        drag.move_delta_before_frame.clear();
        drag.last_updated_delta_for_pan = Offset::ZERO;
        drag.frame_time_stamp = Some(current_system_frame_time_stamp);
    }

    let axis = DragLeaf::get_primary_drag_axis(this, app);

    if app.get(this).drag().state != DragState::Accepted
        || local_delta == Offset::ZERO
        || (app.get(this).drag().move_delta_before_frame.is_empty() && axis.is_some())
    {
        return local_delta;
    }

    let (dx, dy);
    if axis == Some(DragDirection::Horizontal) {
        dx = resolve_delta(this, app, pointer, DragDirection::Horizontal, local_delta);
        debug_assert!(dx.abs() <= local_delta.dx().abs());
        dy = 0.0;
    } else if axis == Some(DragDirection::Vertical) {
        dx = 0.0;
        dy = resolve_delta(this, app, pointer, DragDirection::Vertical, local_delta);
        debug_assert!(dy.abs() <= local_delta.dy().abs());
    } else {
        let average_x =
            resolve_delta_for_pan_gesture(this, app, DragDirection::Horizontal, local_delta);
        let average_y =
            resolve_delta_for_pan_gesture(this, app, DragDirection::Vertical, local_delta);
        let updated_delta =
            Offset::new(average_x, average_y) - app.get(this).drag().last_updated_delta_for_pan;
        app.get_mut(this).drag_mut().last_updated_delta_for_pan = Offset::new(average_x, average_y);
        dx = updated_delta.dx();
        dy = updated_delta.dy();
    }

    Offset::new(dx, dy)
}

fn resolve_delta<L: DragLeaf>(
    this: Handle<L>,
    app: &App,
    pointer: i64,
    axis: DragDirection,
    local_delta: Offset,
) -> f64 {
    let positive = if axis == DragDirection::Horizontal {
        local_delta.dx() > 0.0
    } else {
        local_delta.dy() > 0.0
    };
    let delta = if axis == DragDirection::Horizontal {
        local_delta.dx()
    } else {
        local_delta.dy()
    };
    let max_sum_delta_pointer = get_max_sum_delta_pointer(this, app, positive, axis);
    debug_assert!(max_sum_delta_pointer.is_some());

    if max_sum_delta_pointer == Some(pointer) {
        delta
    } else {
        let max_sum_delta =
            get_sum_delta(this, app, max_sum_delta_pointer.unwrap(), positive, axis);
        let cur_pointer_sum_delta = get_sum_delta(this, app, pointer, positive, axis);
        if positive {
            if cur_pointer_sum_delta + delta > max_sum_delta {
                cur_pointer_sum_delta + delta - max_sum_delta
            } else {
                0.0
            }
        } else if cur_pointer_sum_delta + delta < max_sum_delta {
            cur_pointer_sum_delta + delta - max_sum_delta
        } else {
            0.0
        }
    }
}

fn resolve_delta_for_pan_gesture<L: DragLeaf>(
    this: Handle<L>,
    app: &App,
    axis: DragDirection,
    local_delta: Offset,
) -> f64 {
    let delta = if axis == DragDirection::Horizontal {
        local_delta.dx()
    } else {
        local_delta.dy()
    };
    let pointer_count = app.get(this).drag().accepted_active_pointers.len();
    debug_assert!(pointer_count >= 1);

    let mut sum = delta;
    for offset in app.get(this).drag().move_delta_before_frame.values() {
        if axis == DragDirection::Horizontal {
            sum += offset.dx();
        } else {
            sum += offset.dy();
        }
    }
    sum / pointer_count as f64
}

fn give_up_pointer<L: DragLeaf>(this: Handle<L>, app: &mut App, pointer: i64) {
    OneSequenceGestureRecognizer::stop_tracking_pointer(this, app, pointer);
    // If we never accepted the pointer, we reject it since we are no longer
    // interested in winning the gesture arena for it.
    let accepted = app
        .get(this)
        .drag()
        .accepted_active_pointers
        .iter()
        .position(|&p| p == pointer);
    match accepted {
        Some(index) => {
            app.get_mut(this)
                .drag_mut()
                .accepted_active_pointers
                .remove(index);
        }
        None => {
            OneSequenceGestureRecognizer::resolve_pointer(
                this,
                app,
                pointer,
                GestureDisposition::Rejected,
            );
        }
    }

    let drag = app.get_mut(this).drag_mut();
    drag.move_delta_before_frame.shift_remove(&pointer);
    if drag.active_pointer == Some(pointer) {
        drag.active_pointer = drag.accepted_active_pointers.first().copied();
    }
}

fn check_down<L: DragLeaf>(this: Handle<L>, app: &mut App) {
    if let Some(callback) = app.get(this).drag().on_down.clone() {
        let initial_position = app.get(this).drag().initial_position;
        let details = DragDownDetails::new(initial_position.global, Some(initial_position.local));
        GestureRecognizer::invoke_callback(this, app, "onDown", |app| callback(app, details));
    }
}

fn check_drag<L: DragLeaf>(this: Handle<L>, app: &mut App, pointer: i64) {
    if app.get(this).drag().state == DragState::Accepted {
        return;
    }
    app.get_mut(this).drag_mut().state = DragState::Accepted;
    let delta = app.get(this).drag().pending_drag_offset;
    let timestamp = app.get(this).drag().last_pending_event_timestamp;
    let transform = app.get(this).drag().last_transform;
    let local_update_delta = match app.get(this).drag().drag_start_behavior {
        DragStartBehavior::Start => {
            let initial_position = app.get(this).drag().initial_position;
            app.get_mut(this).drag_mut().initial_position = initial_position + delta;
            Offset::ZERO
        }
        DragStartBehavior::Down => DragLeaf::get_delta_for_details(this, app, delta.local),
    };
    let drag = app.get_mut(this).drag_mut();
    drag.pending_drag_offset = OffsetPair::ZERO;
    drag.last_pending_event_timestamp = None;
    drag.last_transform = None;
    check_start(this, app, timestamp, pointer);
    if local_update_delta != Offset::ZERO && app.get(this).drag().on_update.is_some() {
        let local_to_global = transform.and_then(|transform| transform.invert());
        let corrected_local_position =
            app.get(this).drag().initial_position.local + local_update_delta;
        let global_update_delta = transform_delta_via_positions(
            corrected_local_position,
            None,
            local_update_delta,
            local_to_global,
        );
        let update_delta = OffsetPair::new(local_update_delta, global_update_delta);
        // Only adds delta for down behaviour.
        let corrected_position = app.get(this).drag().initial_position + update_delta;
        let primary_delta = DragLeaf::get_primary_value_from_offset(this, app, local_update_delta);
        check_update(
            this,
            app,
            timestamp,
            local_update_delta,
            primary_delta,
            corrected_position.global,
            Some(corrected_position.local),
            pointer,
        );
    }
    // This accept_gesture might have been called only for one pointer, instead
    // of all pointers. Resolve all pointers to `accepted`. This won't cause
    // infinite recursion because an accepted pointer won't be accepted again.
    this.resolve(app, GestureDisposition::Accepted);
}

fn check_start<L: DragLeaf>(
    this: Handle<L>,
    app: &mut App,
    timestamp: Option<Duration>,
    pointer: i64,
) {
    if let Some(callback) = app.get(this).drag().on_start.clone() {
        let initial_position = app.get(this).drag().initial_position;
        let details = DragStartDetails::new(
            initial_position.global,
            Some(initial_position.local),
            timestamp,
            Some(GestureRecognizer::kind_for_pointer(this, app, pointer)),
        );
        GestureRecognizer::invoke_callback(this, app, "onStart", |app| callback(app, details));
    }
}

#[allow(clippy::too_many_arguments)]
fn check_update<L: DragLeaf>(
    this: Handle<L>,
    app: &mut App,
    source_time_stamp: Option<Duration>,
    delta: Offset,
    primary_delta: Option<f64>,
    global_position: Offset,
    local_position: Option<Offset>,
    pointer: i64,
) {
    if let Some(callback) = app.get(this).drag().on_update.clone() {
        let details = DragUpdateDetails::new(
            global_position,
            local_position,
            source_time_stamp,
            delta,
            primary_delta,
            Some(GestureRecognizer::kind_for_pointer(this, app, pointer)),
        );
        GestureRecognizer::invoke_callback(this, app, "onUpdate", |app| callback(app, details));
    }
}

fn check_end<L: DragLeaf>(this: Handle<L>, app: &mut App, pointer: i64) {
    let Some(callback) = app.get(this).drag().on_end.clone() else {
        return;
    };

    let (estimate, kind) = {
        let tracker = &app.get(this).drag().velocity_trackers[&pointer];
        (tracker.get_velocity_estimate(), tracker.kind())
    };

    let details = estimate
        .and_then(|estimate| DragLeaf::consider_fling(this, app, &estimate, kind))
        .unwrap_or_else(|| {
            let last_position = app.get(this).drag().last_position;
            DragEndDetails::new(
                last_position.global,
                Some(last_position.local),
                Velocity::ZERO,
                Some(0.0),
            )
        });

    GestureRecognizer::invoke_callback(this, app, "onEnd", |app| callback(app, details));
}

fn check_cancel<L: DragLeaf>(this: Handle<L>, app: &mut App) {
    if let Some(callback) = app.get(this).drag().on_cancel.clone() {
        GestureRecognizer::invoke_callback(this, app, "onCancel", |app| callback.call(app));
    }
}

/// The members every drag leaf inherits verbatim: the constructor, the
/// superclass field accessors, and the `RecognizerLeaf` overrides that forward
/// to [`DragGestureRecognizer`].
macro_rules! drag_gesture_recognizer_leaf {
    ($name:ident, $description:literal) => {
        impl $name {
            /// Create a gesture recognizer.
            pub fn new(app: &mut App) -> Handle<$name> {
                let mut recognizer = GestureRecognizerData::new();
                recognizer.allowed_buttons_filter = Rc::new(default_button_accept_behavior);
                app.create($name {
                    recognizer,
                    one_sequence: OneSequenceData::new(),
                    drag: DragData::new(),
                })
            }

            /// The kind of devices that are allowed to be recognized.
            pub fn supported_devices(
                self: Handle<Self>,
                app: &mut App,
                devices: impl IntoIterator<Item = PointerDeviceKind>,
            ) -> Handle<$name> {
                app.get_mut(self).recognizer.supported_devices =
                    Some(devices.into_iter().collect());
                self
            }

            /// Sets the kind of devices that are allowed to be recognized; `None`
            /// tracks and recognizes events from all device kinds.
            pub fn set_supported_devices(
                self: Handle<Self>,
                app: &mut App,
                devices: Option<std::collections::HashSet<PointerDeviceKind>>,
            ) {
                app.get_mut(self).recognizer.supported_devices = devices;
            }

            /// Called when interaction starts. Limits buttons this recognizer
            /// accepts.
            pub fn allowed_buttons_filter(
                self: Handle<Self>,
                app: &mut App,
                filter: impl Fn(i64) -> bool + 'static,
            ) -> Handle<$name> {
                app.get_mut(self).recognizer.allowed_buttons_filter = Rc::new(filter);
                self
            }

            /// Optional device specific configuration that takes precedence over
            /// framework defaults.
            pub fn gesture_settings(
                self: Handle<Self>,
                app: &App,
            ) -> Option<DeviceGestureSettings> {
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

            /// The team that this recognizer belongs to, if any.
            pub fn team(self: Handle<Self>, app: &App) -> Option<Handle<GestureArenaTeam>> {
                app.get(self).one_sequence.team
            }

            /// The [`team`](Self::team) can only be set once.
            pub fn set_team(self: Handle<Self>, app: &mut App, value: Handle<GestureArenaTeam>) {
                debug_assert!(app.get(self).one_sequence.entries.is_empty());
                debug_assert!(app.get(self).one_sequence.tracked_pointers.is_empty());
                debug_assert!(app.get(self).one_sequence.team.is_none());
                app.get_mut(self).one_sequence.team = Some(value);
            }

            /// Registers a new pointer that might be relevant to this gesture
            /// detector.
            pub fn add_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                GestureRecognizer::add_pointer(self, app, event);
            }

            /// Registers a new pointer pan/zoom that might be relevant to this
            /// gesture detector.
            pub fn add_pointer_pan_zoom(
                self: Handle<Self>,
                app: &mut App,
                event: PointerPanZoomStartEvent,
            ) {
                GestureRecognizer::add_pointer_pan_zoom(self, app, event);
            }

            /// Returns a very short pretty description of the gesture that the
            /// recognizer looks for.
            pub fn debug_description(self: Handle<Self>) -> &'static str {
                <Self as RecognizerLeaf>::debug_description(self)
            }
        }

        impl RecognizerLeafData for $name {
            fn recognizer(&self) -> &GestureRecognizerData {
                &self.recognizer
            }
            fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
                &mut self.recognizer
            }
        }

        impl OneSequenceLeafData for $name {
            fn one_sequence(&self) -> &OneSequenceData {
                &self.one_sequence
            }
            fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
                &mut self.one_sequence
            }
        }

        impl DragLeafData for $name {
            fn drag(&self) -> &DragData {
                &self.drag
            }
            fn drag_mut(&mut self) -> &mut DragData {
                &mut self.drag
            }
        }

        impl RecognizerLeaf for $name {
            fn handle_non_allowed_pointer(
                self: Handle<Self>,
                app: &mut App,
                _event: &PointerDownEvent,
            ) {
                OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
            }

            fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
                DragGestureRecognizer::is_pointer_allowed(self, app, event)
            }

            fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                DragGestureRecognizer::add_allowed_pointer(self, app, event);
            }

            fn add_allowed_pointer_pan_zoom(
                self: Handle<Self>,
                app: &mut App,
                event: PointerPanZoomStartEvent,
            ) {
                DragGestureRecognizer::add_allowed_pointer_pan_zoom(self, app, event);
            }

            fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
                DragGestureRecognizer::handle_event(self, app, event);
            }

            fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::accept_gesture(self, app, pointer);
            }

            fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::reject_gesture(self, app, pointer);
            }

            fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                DragGestureRecognizer::dispose(self, app);
            }

            fn debug_description(self: Handle<Self>) -> &'static str {
                $description
            }
        }
    };
}

/// Recognizes movement in the vertical direction.
///
/// Used for vertical scrolling.
///
/// See also:
///
///  * [`HorizontalDragGestureRecognizer`], for a similar recognizer but for
///    horizontal movement.
pub struct VerticalDragGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
}

drag_gesture_recognizer_leaf!(VerticalDragGestureRecognizer, "vertical drag");

/// The bodies of [`VerticalDragGestureRecognizer`], which its subclasses inherit.
pub struct VerticalDragGestureRecognizerBase;

impl VerticalDragGestureRecognizerBase {
    /// See [`DragLeaf::is_fling_gesture`].
    pub fn is_fling_gesture<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        let min_velocity = app
            .get(this)
            .drag()
            .min_fling_velocity
            .unwrap_or(K_MIN_FLING_VELOCITY);
        let min_distance =
            app.get(this).drag().min_fling_distance.unwrap_or_else(|| {
                compute_hit_slop(kind, app.get(this).recognizer().gesture_settings)
            });
        estimate.pixels_per_second.dy().abs() > min_velocity
            && estimate.offset.dy().abs() > min_distance
    }

    /// See [`DragLeaf::consider_fling`].
    pub fn consider_fling<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        if !VerticalDragGestureRecognizerBase::is_fling_gesture(this, app, estimate, kind) {
            return None;
        }
        let max_velocity = app
            .get(this)
            .drag()
            .max_fling_velocity
            .unwrap_or(K_MAX_FLING_VELOCITY);
        let dy = estimate
            .pixels_per_second
            .dy()
            .clamp(-max_velocity, max_velocity);
        let last_position = app.get(this).drag().last_position;
        Some(DragEndDetails::new(
            last_position.global,
            Some(last_position.local),
            Velocity::new(Offset::new(0.0, dy)),
            Some(dy),
        ))
    }

    /// See [`DragLeaf::has_sufficient_global_distance_to_accept`].
    pub fn has_sufficient_global_distance_to_accept<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        _device_touch_slop: Option<f64>,
    ) -> bool {
        app.get(this).drag().global_distance_moved.abs()
            > compute_hit_slop(
                pointer_device_kind,
                app.get(this).recognizer().gesture_settings,
            )
    }

    /// See [`DragLeaf::get_delta_for_details`].
    pub fn get_delta_for_details<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
        delta: Offset,
    ) -> Offset {
        Offset::new(0.0, delta.dy())
    }

    /// See [`DragLeaf::get_primary_value_from_offset`].
    pub fn get_primary_value_from_offset<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
        value: Offset,
    ) -> Option<f64> {
        Some(value.dy())
    }

    /// See [`DragLeaf::get_primary_drag_axis`].
    pub fn get_primary_drag_axis<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
    ) -> Option<DragDirection> {
        Some(DragDirection::Vertical)
    }
}

impl DragLeaf for VerticalDragGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        VerticalDragGestureRecognizerBase::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        VerticalDragGestureRecognizerBase::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        VerticalDragGestureRecognizerBase::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset {
        VerticalDragGestureRecognizerBase::get_delta_for_details(self, app, delta)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64> {
        VerticalDragGestureRecognizerBase::get_primary_value_from_offset(self, app, value)
    }

    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        VerticalDragGestureRecognizerBase::get_primary_drag_axis(self, app)
    }
}

/// Recognizes movement in the horizontal direction.
///
/// Used for horizontal scrolling.
///
/// See also:
///
///  * [`VerticalDragGestureRecognizer`], for a similar recognizer but for
///    vertical movement.
pub struct HorizontalDragGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
}

drag_gesture_recognizer_leaf!(HorizontalDragGestureRecognizer, "horizontal drag");

/// The bodies of [`HorizontalDragGestureRecognizer`], which its subclasses inherit.
pub struct HorizontalDragGestureRecognizerBase;

impl HorizontalDragGestureRecognizerBase {
    /// See [`DragLeaf::is_fling_gesture`].
    pub fn is_fling_gesture<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        let min_velocity = app
            .get(this)
            .drag()
            .min_fling_velocity
            .unwrap_or(K_MIN_FLING_VELOCITY);
        let min_distance =
            app.get(this).drag().min_fling_distance.unwrap_or_else(|| {
                compute_hit_slop(kind, app.get(this).recognizer().gesture_settings)
            });
        estimate.pixels_per_second.dx().abs() > min_velocity
            && estimate.offset.dx().abs() > min_distance
    }

    /// See [`DragLeaf::consider_fling`].
    pub fn consider_fling<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        if !HorizontalDragGestureRecognizerBase::is_fling_gesture(this, app, estimate, kind) {
            return None;
        }
        let max_velocity = app
            .get(this)
            .drag()
            .max_fling_velocity
            .unwrap_or(K_MAX_FLING_VELOCITY);
        let dx = estimate
            .pixels_per_second
            .dx()
            .clamp(-max_velocity, max_velocity);
        let last_position = app.get(this).drag().last_position;
        Some(DragEndDetails::new(
            last_position.global,
            Some(last_position.local),
            Velocity::new(Offset::new(dx, 0.0)),
            Some(dx),
        ))
    }

    /// See [`DragLeaf::has_sufficient_global_distance_to_accept`].
    pub fn has_sufficient_global_distance_to_accept<R: DragLeaf>(
        this: Handle<R>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        _device_touch_slop: Option<f64>,
    ) -> bool {
        app.get(this).drag().global_distance_moved.abs()
            > compute_hit_slop(
                pointer_device_kind,
                app.get(this).recognizer().gesture_settings,
            )
    }

    /// See [`DragLeaf::get_delta_for_details`].
    pub fn get_delta_for_details<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
        delta: Offset,
    ) -> Offset {
        Offset::new(delta.dx(), 0.0)
    }

    /// See [`DragLeaf::get_primary_value_from_offset`].
    pub fn get_primary_value_from_offset<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
        value: Offset,
    ) -> Option<f64> {
        Some(value.dx())
    }

    /// See [`DragLeaf::get_primary_drag_axis`].
    pub fn get_primary_drag_axis<R: DragLeaf>(
        _this: Handle<R>,
        _app: &App,
    ) -> Option<DragDirection> {
        Some(DragDirection::Horizontal)
    }
}

impl DragLeaf for HorizontalDragGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        HorizontalDragGestureRecognizerBase::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        HorizontalDragGestureRecognizerBase::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        HorizontalDragGestureRecognizerBase::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset {
        HorizontalDragGestureRecognizerBase::get_delta_for_details(self, app, delta)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64> {
        HorizontalDragGestureRecognizerBase::get_primary_value_from_offset(self, app, value)
    }

    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        HorizontalDragGestureRecognizerBase::get_primary_drag_axis(self, app)
    }
}

/// Recognizes movement both horizontally and vertically.
pub struct PanGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
}

drag_gesture_recognizer_leaf!(PanGestureRecognizer, "pan");

impl DragLeaf for PanGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        let min_velocity = app
            .get(self)
            .drag
            .min_fling_velocity
            .unwrap_or(K_MIN_FLING_VELOCITY);
        let min_distance =
            app.get(self).drag.min_fling_distance.unwrap_or_else(|| {
                compute_hit_slop(kind, app.get(self).recognizer.gesture_settings)
            });
        estimate.pixels_per_second.distance_squared() > min_velocity * min_velocity
            && estimate.offset.distance_squared() > min_distance * min_distance
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        if !DragLeaf::is_fling_gesture(self, app, estimate, kind) {
            return None;
        }
        let velocity = Velocity::new(estimate.pixels_per_second).clamp_magnitude(
            app.get(self)
                .drag
                .min_fling_velocity
                .unwrap_or(K_MIN_FLING_VELOCITY),
            app.get(self)
                .drag
                .max_fling_velocity
                .unwrap_or(K_MAX_FLING_VELOCITY),
        );
        let last_position = app.get(self).drag.last_position;
        Some(DragEndDetails::new(
            last_position.global,
            Some(last_position.local),
            velocity,
            None,
        ))
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        _device_touch_slop: Option<f64>,
    ) -> bool {
        app.get(self).drag.global_distance_moved.abs()
            > compute_pan_slop(
                pointer_device_kind,
                app.get(self).recognizer.gesture_settings,
            )
    }

    fn get_delta_for_details(self: Handle<Self>, _app: &App, delta: Offset) -> Offset {
        delta
    }

    fn get_primary_value_from_offset(
        self: Handle<Self>,
        _app: &App,
        _value: Offset,
    ) -> Option<f64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::binding::GestureBinding;
    use crate::constants::K_MAX_FLING_VELOCITY;
    use crate::events::{
        PointerMoveEvent, PointerPanZoomEndEvent, PointerPanZoomUpdateEvent, PointerUpEvent,
    };
    use crate::tap::TapGestureRecognizer;

    /// Flutter's `TestPointer`: keeps the last location so a move carries the
    /// delta the recognizer reads.
    struct TestPointer {
        pointer: i64,
        kind: PointerDeviceKind,
        location: Offset,
        pan: Offset,
    }

    impl TestPointer {
        fn new(pointer: i64) -> TestPointer {
            TestPointer::with_kind(pointer, PointerDeviceKind::Touch)
        }

        fn with_kind(pointer: i64, kind: PointerDeviceKind) -> TestPointer {
            TestPointer {
                pointer,
                kind,
                location: Offset::ZERO,
                pan: Offset::ZERO,
            }
        }

        fn down(&mut self, position: Offset, time_stamp: Duration) -> PointerDownEvent {
            self.location = position;
            PointerDownEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                time_stamp,
                buttons: K_PRIMARY_BUTTON,
                ..PointerDownEvent::default()
            }
        }

        fn move_to(&mut self, position: Offset, time_stamp: Duration) -> PointerMoveEvent {
            let delta = position - self.location;
            self.location = position;
            PointerMoveEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                delta,
                time_stamp,
                down: true,
                buttons: K_PRIMARY_BUTTON,
                ..PointerMoveEvent::default()
            }
        }

        fn up(&mut self, time_stamp: Duration) -> PointerUpEvent {
            PointerUpEvent {
                pointer: self.pointer,
                kind: self.kind,
                position: self.location,
                time_stamp,
                ..PointerUpEvent::default()
            }
        }

        fn pan_zoom_start(&mut self, position: Offset) -> PointerPanZoomStartEvent {
            self.location = position;
            self.pan = Offset::ZERO;
            PointerPanZoomStartEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                ..PointerPanZoomStartEvent::default()
            }
        }

        fn pan_zoom_update(&mut self, position: Offset, pan: Offset) -> PointerPanZoomUpdateEvent {
            let pan_delta = pan - self.pan;
            self.location = position;
            self.pan = pan;
            PointerPanZoomUpdateEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                pan,
                pan_delta,
                ..PointerPanZoomUpdateEvent::default()
            }
        }

        fn pan_zoom_end(&mut self) -> PointerPanZoomEndEvent {
            PointerPanZoomEndEvent {
                pointer: self.pointer,
                kind: self.kind,
                position: self.location,
                ..PointerPanZoomEndEvent::default()
            }
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

    fn flag() -> (Rc<Cell<bool>>, Rc<Cell<bool>>) {
        let value = Rc::new(Cell::new(false));
        (Rc::clone(&value), value)
    }

    #[test]
    fn should_recognize_pan() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let pan = PanGestureRecognizer::new(&mut app);
        let tap = TapGestureRecognizer::new(&mut app);

        let (did_start_pan, start_flag) = flag();
        pan.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| start_flag.set(true))),
        );
        let updated_scroll_delta = Rc::new(Cell::new(None));
        let delta_sink = Rc::clone(&updated_scroll_delta);
        pan.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, details: DragUpdateDetails| {
                delta_sink.set(Some(details.delta))
            })),
        );
        let (did_end_pan, end_flag) = flag();
        pan.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| end_flag.set(true))),
        );
        let (did_tap, tap_flag) = flag();
        tap.set_on_tap(
            &mut app,
            Some(Listener::new(move |_app| tap_flag.set(true))),
        );

        let mut pointer = TestPointer::new(5);
        let down = pointer.down(Offset::new(10.0, 10.0), Duration::ZERO);
        pan.add_pointer(&mut app, down.clone());
        tap.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 5);
        assert!(!did_start_pan.get());
        assert!(updated_scroll_delta.get().is_none());
        assert!(!did_end_pan.get());
        assert!(!did_tap.get());

        route(&mut app, PointerEvent::Down(down));
        assert!(!did_start_pan.get());

        // The tap gives up when the pointer passes kTouchSlop (18.0), leaving the
        // pan alone in the arena.
        let moved = pointer.move_to(Offset::new(20.0, 20.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        assert!(!did_start_pan.get(), "14 < 18");

        let moved = pointer.move_to(Offset::new(20.0, 30.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        assert!(did_start_pan.get(), "22 > 18");
        did_start_pan.set(false);
        assert!(!did_end_pan.get());
        assert!(!did_tap.get());

        let moved = pointer.move_to(Offset::new(20.0, 25.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        assert!(!did_start_pan.get());
        assert_eq!(updated_scroll_delta.get(), Some(Offset::new(0.0, -5.0)));
        updated_scroll_delta.set(None);
        assert!(!did_end_pan.get());

        let up = pointer.up(Duration::ZERO);
        route(&mut app, PointerEvent::Up(up));
        assert!(updated_scroll_delta.get().is_none());
        assert!(did_end_pan.get());
        assert!(!did_tap.get());

        DragGestureRecognizer::dispose(pan, &mut app);
        tap.dispose(&mut app);
    }

    #[test]
    fn should_report_most_recent_point_to_on_start_by_default() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let drag = HorizontalDragGestureRecognizer::new(&mut app);
        let competing_drag = VerticalDragGestureRecognizer::new(&mut app);
        competing_drag.set_on_start(&mut app, Some(Rc::new(|_app, _details| {})));

        let position_at_on_start = Rc::new(Cell::new(None));
        let start_sink = Rc::clone(&position_at_on_start);
        drag.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, details: DragStartDetails| {
                start_sink.set(Some(details.global_position))
            })),
        );
        let update_offset = Rc::new(Cell::new(None));
        let update_sink = Rc::clone(&update_offset);
        drag.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, details: DragUpdateDetails| {
                update_sink.set(Some(details.global_position))
            })),
        );

        let mut pointer = TestPointer::new(5);
        let down = pointer.down(Offset::new(10.0, 10.0), Duration::ZERO);
        drag.add_pointer(&mut app, down.clone());
        competing_drag.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down));

        let moved = pointer.move_to(Offset::new(30.0, 0.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));

        assert_eq!(position_at_on_start.get(), Some(Offset::new(30.0, 0.0)));
        assert_eq!(update_offset.get(), None);

        DragGestureRecognizer::dispose(drag, &mut app);
        DragGestureRecognizer::dispose(competing_drag, &mut app);
    }

    #[test]
    fn should_recognize_drag_with_the_down_start_behavior() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let drag = HorizontalDragGestureRecognizer::new(&mut app);
        drag.set_drag_start_behavior(&mut app, DragStartBehavior::Down);

        let (did_start_drag, start_flag) = flag();
        drag.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| start_flag.set(true))),
        );
        let updated_delta = Rc::new(Cell::new(None));
        let delta_sink = Rc::clone(&updated_delta);
        drag.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, details: DragUpdateDetails| {
                delta_sink.set(details.primary_delta)
            })),
        );
        let (did_end_drag, end_flag) = flag();
        drag.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| end_flag.set(true))),
        );

        let mut pointer = TestPointer::new(5);
        let down = pointer.down(Offset::new(10.0, 10.0), Duration::ZERO);
        drag.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 5);
        assert!(!did_start_drag.get());
        assert!(updated_delta.get().is_none());
        assert!(!did_end_drag.get());

        route(&mut app, PointerEvent::Down(down));
        assert!(did_start_drag.get());
        assert!(updated_delta.get().is_none());
        assert!(!did_end_drag.get());

        let moved = pointer.move_to(Offset::new(20.0, 25.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        did_start_drag.set(false);
        assert_eq!(updated_delta.get(), Some(10.0));
        updated_delta.set(None);

        let moved = pointer.move_to(Offset::new(20.0, 25.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        assert!(!did_start_drag.get());
        assert_eq!(updated_delta.get(), Some(0.0));
        updated_delta.set(None);
        assert!(!did_end_drag.get());

        let up = pointer.up(Duration::ZERO);
        route(&mut app, PointerEvent::Up(up));
        assert!(!did_start_drag.get());
        assert!(updated_delta.get().is_none());
        assert!(did_end_drag.get());

        DragGestureRecognizer::dispose(drag, &mut app);
    }

    #[test]
    fn the_vertical_recognizer_wins_a_vertical_move_and_the_horizontal_one_cancels() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let vertical = VerticalDragGestureRecognizer::new(&mut app);
        let horizontal = HorizontalDragGestureRecognizer::new(&mut app);

        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let vertical_log = Rc::clone(&log);
        vertical.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                vertical_log.borrow_mut().push("vertical start")
            })),
        );
        let horizontal_log = Rc::clone(&log);
        horizontal.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                horizontal_log.borrow_mut().push("horizontal start")
            })),
        );
        let cancel_log = Rc::clone(&log);
        horizontal.set_on_cancel(
            &mut app,
            Some(Listener::new(move |_app| {
                cancel_log.borrow_mut().push("horizontal cancel")
            })),
        );

        let mut pointer = TestPointer::new(5);
        let down = pointer.down(Offset::new(10.0, 10.0), Duration::ZERO);
        vertical.add_pointer(&mut app, down.clone());
        horizontal.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down));
        assert!(log.borrow().is_empty());

        let moved = pointer.move_to(Offset::new(10.0, 50.0), Duration::ZERO);
        route(&mut app, PointerEvent::Move(moved));
        // The arena rejects the losers before it accepts the winner.
        assert_eq!(*log.borrow(), vec!["horizontal cancel", "vertical start"]);

        DragGestureRecognizer::dispose(vertical, &mut app);
        DragGestureRecognizer::dispose(horizontal, &mut app);
    }

    #[test]
    fn clamp_max_velocity() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let drag = HorizontalDragGestureRecognizer::new(&mut app);
        drag.set_drag_start_behavior(&mut app, DragStartBehavior::Down);

        let velocity = Rc::new(Cell::new(Velocity::ZERO));
        let primary_velocity = Rc::new(Cell::new(None));
        let velocity_sink = Rc::clone(&velocity);
        let primary_sink = Rc::clone(&primary_velocity);
        drag.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, details: DragEndDetails| {
                velocity_sink.set(details.velocity);
                primary_sink.set(details.primary_velocity);
            })),
        );

        let mut pointer = TestPointer::new(5);
        let down = pointer.down(Offset::new(10.0, 25.0), Duration::from_millis(10));
        drag.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down));
        for step in 0..11 {
            let moved = pointer.move_to(
                Offset::new(20.0 + 10.0 * step as f64, 25.0),
                Duration::from_millis(10 + step),
            );
            route(&mut app, PointerEvent::Move(moved));
        }
        let up = pointer.up(Duration::from_millis(20));
        route(&mut app, PointerEvent::Up(up));

        let dx = velocity.get().pixels_per_second.dx();
        assert!(
            (0.99 * K_MAX_FLING_VELOCITY..=K_MAX_FLING_VELOCITY).contains(&dx),
            "clamped to at most kMaxFlingVelocity, got {dx}"
        );
        assert!(velocity.get().pixels_per_second.dy().abs() < 1e-9);
        assert_eq!(primary_velocity.get(), Some(dx));

        DragGestureRecognizer::dispose(drag, &mut app);
    }

    fn drag_callbacks_on_a_drag_that_never_moves(
        only_accept_drag_on_threshold: bool,
    ) -> Vec<&'static str> {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let vertical_drag = VerticalDragGestureRecognizer::new(&mut app);
        vertical_drag.set_only_accept_drag_on_threshold(&mut app, only_accept_drag_on_threshold);
        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let start_log = Rc::clone(&log);
        vertical_drag.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                start_log.borrow_mut().push("onStart")
            })),
        );
        let update_log = Rc::clone(&log);
        vertical_drag.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                update_log.borrow_mut().push("onUpdate")
            })),
        );
        let end_log = Rc::clone(&log);
        vertical_drag.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                end_log.borrow_mut().push("onEnd")
            })),
        );

        let mut pointer = TestPointer::new(6);
        let down = pointer.down(Offset::new(10.0, 10.0), Duration::ZERO);
        vertical_drag.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 6);
        route(&mut app, PointerEvent::Down(down));
        let up = pointer.up(Duration::ZERO);
        route(&mut app, PointerEvent::Up(up));
        DragGestureRecognizer::dispose(vertical_drag, &mut app);
        log.borrow().clone()
    }

    #[test]
    fn only_accept_drag_on_threshold_suppresses_callbacks_until_the_threshold_is_met() {
        assert!(drag_callbacks_on_a_drag_that_never_moves(true).is_empty());
    }

    #[test]
    fn without_only_accept_drag_on_threshold_winning_the_arena_is_enough() {
        assert_eq!(
            drag_callbacks_on_a_drag_that_never_moves(false),
            vec!["onStart", "onEnd"]
        );
    }

    #[test]
    fn should_recognize_pan_gestures_from_platform() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let pan = PanGestureRecognizer::new(&mut app);
        // A competing recognizer, so the gesture is not immediately claimed.
        let competing_pan = PanGestureRecognizer::new(&mut app);
        competing_pan.set_on_start(&mut app, Some(Rc::new(|_app, _details| {})));

        let (did_start_pan, start_flag) = flag();
        pan.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| start_flag.set(true))),
        );
        let updated_scroll_delta = Rc::new(Cell::new(None));
        let delta_sink = Rc::clone(&updated_scroll_delta);
        pan.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, details: DragUpdateDetails| {
                delta_sink.set(Some(details.delta))
            })),
        );
        let (did_end_pan, end_flag) = flag();
        pan.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| end_flag.set(true))),
        );

        let mut pointer = TestPointer::with_kind(2, PointerDeviceKind::Trackpad);
        let start = pointer.pan_zoom_start(Offset::new(10.0, 10.0));
        pan.add_pointer_pan_zoom(&mut app, start.clone());
        competing_pan.add_pointer_pan_zoom(&mut app, start.clone());
        close_arena(&mut app, 2);
        assert!(!did_start_pan.get());

        route(&mut app, PointerEvent::PanZoomStart(start));
        assert!(!did_start_pan.get());

        // The gesture is claimed when the distance reaches kPanSlop (36.0).
        let update = pointer.pan_zoom_update(Offset::new(10.0, 10.0), Offset::new(20.0, 20.0));
        route(&mut app, PointerEvent::PanZoomUpdate(update));
        assert!(!did_start_pan.get(), "28 < 36");

        let update = pointer.pan_zoom_update(Offset::new(10.0, 10.0), Offset::new(30.0, 30.0));
        route(&mut app, PointerEvent::PanZoomUpdate(update));
        assert!(did_start_pan.get(), "42 > 36");
        did_start_pan.set(false);
        assert!(!did_end_pan.get());

        let update = pointer.pan_zoom_update(Offset::new(10.0, 10.0), Offset::new(30.0, 25.0));
        route(&mut app, PointerEvent::PanZoomUpdate(update));
        assert!(!did_start_pan.get());
        assert_eq!(updated_scroll_delta.get(), Some(Offset::new(0.0, -5.0)));
        updated_scroll_delta.set(None);
        assert!(!did_end_pan.get());

        let end = pointer.pan_zoom_end();
        route(&mut app, PointerEvent::PanZoomEnd(end));
        assert!(updated_scroll_delta.get().is_none());
        assert!(did_end_pan.get());

        DragGestureRecognizer::dispose(pan, &mut app);
        DragGestureRecognizer::dispose(competing_pan, &mut app);
    }

    #[test]
    fn a_recognizer_with_no_callbacks_does_not_compete() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let drag = VerticalDragGestureRecognizer::new(&mut app);
        let down = PointerDownEvent {
            pointer: 5,
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        };
        assert!(!DragGestureRecognizer::is_pointer_allowed(
            drag, &app, &down
        ));
        drag.set_on_start(&mut app, Some(Rc::new(|_app, _details| {})));
        assert!(DragGestureRecognizer::is_pointer_allowed(drag, &app, &down));
        DragGestureRecognizer::dispose(drag, &mut app);
    }

    #[test]
    fn debug_descriptions_name_the_axis() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let vertical = VerticalDragGestureRecognizer::new(&mut app);
        let horizontal = HorizontalDragGestureRecognizer::new(&mut app);
        let pan = PanGestureRecognizer::new(&mut app);
        assert_eq!(vertical.debug_description(), "vertical drag");
        assert_eq!(horizontal.debug_description(), "horizontal drag");
        assert_eq!(pan.debug_description(), "pan");
        DragGestureRecognizer::dispose(vertical, &mut app);
        DragGestureRecognizer::dispose(horizontal, &mut app);
        DragGestureRecognizer::dispose(pan, &mut app);
    }
}

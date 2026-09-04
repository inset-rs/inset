//! Flutter counterpart: `gestures/recognizer.dart`.
//!
//! Superclass field bags and `super` namespaces. Leaves implement
//! [`RecognizerLeaf`] so a superclass body calls the leaf, not a sibling. A
//! `super` call is a namespace fn taking the leaf's `Handle` first.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Matrix4, Offset, PointerDeviceKind};
use reveal_foundation::{App, Handle, HandleId, Listener, Timer};

use crate::arena::{GestureArenaEntry, GestureArenaMember, GestureDisposition};
use crate::binding::GestureBinding;
use crate::constants::K_TOUCH_SLOP;
use crate::debug::{debug_print_gesture_arena_diagnostics, debug_print_recognizer_callbacks_trace};
use crate::events::{PointerDownEvent, PointerEvent};
use crate::gesture_settings::DeviceGestureSettings;
use crate::pointer_router::PointerRoute;
use crate::team::GestureArenaTeam;

/// Signature for a recognizer's `allowed_buttons_filter`.
///
/// Used to filter the input buttons of incoming pointer events.
/// The parameter `buttons` comes from `PointerEvent.buttons`.
pub type AllowedButtonsFilter = Rc<dyn Fn(i64) -> bool>;

/// `-1` is used as a sentinel value to indicate no touch slop was specified.
pub(crate) const UNSET_TOUCH_SLOP: f64 = -1.0;

/// The default value for `allowedButtonsFilter`.
/// Accept any input.
pub(crate) fn default_button_accept_behavior(_buttons: i64) -> bool {
    true
}

/// Configuration of offset passed to `DragStartDetails`.
///
/// See also:
///
///  * `DragGestureRecognizer.dragStartBehavior`, which gives an example for the
///    different behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragStartBehavior {
    /// Set the initial offset at the position where the first down event was
    /// detected.
    Down,

    /// Set the initial position at the position where this gesture recognizer
    /// won the arena.
    Start,
}

/// Configuration of multi-finger drag strategy on multi-touch devices.
///
/// When dragging with only one finger, there's no difference in behavior
/// between all the settings.
///
/// Used by `DragGestureRecognizer.multitouchDragStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultitouchDragStrategy {
    /// Only the latest active pointer is tracked by the recognizer.
    ///
    /// If the tracked pointer is released, the first accepted of the remaining active
    /// pointers will continue to be tracked.
    ///
    /// This is the behavior typically seen on Android.
    LatestPointer,

    /// All active pointers will be tracked, and the result is computed from
    /// the boundary pointers.
    ///
    /// The scrolling offset is determined by the maximum deltas of both directions.
    ///
    /// This is the behavior typically seen on iOS.
    AverageBoundaryPointers,

    /// All active pointers will be tracked together. The scrolling offset
    /// is the sum of the offsets of all active pointers.
    SumAllPointers,
}

/// The possible states of a `PrimaryPointerGestureRecognizer`.
///
/// The recognizer advances from [`Ready`](Self::Ready) to [`Possible`](Self::Possible) when it starts tracking a
/// primary pointer. Where it advances from there depends on how the gesture is
/// resolved for that pointer:
///
///  * If the primary pointer is resolved by the gesture winning the arena, the
///    recognizer stays in the [`Possible`](Self::Possible) state as long as it continues to track
///    a pointer.
///  * If the primary pointer is resolved by the gesture being rejected and
///    losing the arena, the recognizer's state advances to [`Defunct`](Self::Defunct).
///
/// Once the recognizer has stopped tracking any remaining pointers, the
/// recognizer returns to [`Ready`](Self::Ready).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureRecognizerState {
    /// The recognizer is ready to start recognizing a gesture.
    Ready,

    /// The sequence of pointer events seen thus far is consistent with the
    /// gesture the recognizer is attempting to recognize but the gesture has not
    /// been accepted definitively.
    Possible,

    /// Further pointer events cannot cause this recognizer to recognize the
    /// gesture until the recognizer returns to the [`Ready`](Self::Ready) state (typically when
    /// all the pointers the recognizer is tracking are removed from the screen).
    Defunct,
}

/// Data associated with a pointer event.
///
/// This is stored by `GestureRecognizer`s on a per-pointer basis.
#[derive(Clone, Copy)]
pub(crate) struct RecognizerEventData {
    pub kind: PointerDeviceKind,
    pub buttons: i64,
}

/// Field bag for Dart's `GestureRecognizer`.
pub(crate) struct GestureRecognizerData {
    pub gesture_settings: Option<DeviceGestureSettings>,
    pub supported_devices: Option<HashSet<PointerDeviceKind>>,
    pub allowed_buttons_filter: AllowedButtonsFilter,
    pub pointer_to_event_data: HashMap<i64, RecognizerEventData>,
}

impl GestureRecognizerData {
    pub(crate) fn new() -> GestureRecognizerData {
        GestureRecognizerData {
            gesture_settings: None,
            supported_devices: None,
            allowed_buttons_filter: Rc::new(default_button_accept_behavior),
            pointer_to_event_data: HashMap::new(),
        }
    }
}

/// Field bag for Dart's `OneSequenceGestureRecognizer`.
pub(crate) struct OneSequenceData {
    pub entries: HashMap<i64, GestureArenaEntry>,
    pub tracked_pointers: HashSet<i64>,
    pub team: Option<Handle<GestureArenaTeam>>,
}

impl OneSequenceData {
    pub(crate) fn new() -> OneSequenceData {
        OneSequenceData {
            entries: HashMap::new(),
            tracked_pointers: HashSet::new(),
            team: None,
        }
    }
}

/// Field bag for Dart's `PrimaryPointerGestureRecognizer`.
pub(crate) struct PrimaryPointerData {
    pub deadline: Option<Duration>,
    pub pre_accept_slop_tolerance: Option<f64>,
    pub post_accept_slop_tolerance: Option<f64>,
    pub state: GestureRecognizerState,
    pub primary_pointer: Option<i64>,
    pub initial_position: Option<OffsetPair>,
    pub gesture_accepted: bool,
    pub timer: Option<Timer>,
    pub deadline_event: Option<crate::events::PointerDownEvent>,
}

impl PrimaryPointerData {
    pub(crate) fn new(
        deadline: Option<Duration>,
        pre_accept_slop_tolerance: Option<f64>,
        post_accept_slop_tolerance: Option<f64>,
    ) -> PrimaryPointerData {
        debug_assert!(
            pre_accept_slop_tolerance == Some(UNSET_TOUCH_SLOP)
                || pre_accept_slop_tolerance.is_none()
                || pre_accept_slop_tolerance.is_some_and(|value| value >= 0.0),
            "The preAcceptSlopTolerance must be unspecified, positive, or null"
        );
        debug_assert!(
            post_accept_slop_tolerance == Some(UNSET_TOUCH_SLOP)
                || post_accept_slop_tolerance.is_none()
                || post_accept_slop_tolerance.is_some_and(|value| value >= 0.0),
            "The postAcceptSlopTolerance must be unspecified, positive, or null"
        );
        PrimaryPointerData {
            deadline,
            pre_accept_slop_tolerance,
            post_accept_slop_tolerance,
            state: GestureRecognizerState::Ready,
            primary_pointer: None,
            initial_position: None,
            gesture_accepted: false,
            timer: None,
            deadline_event: None,
        }
    }
}

/// A container for a [`local`](Self::local) and [`global`](Self::global) [`Offset`] pair.
///
/// Usually, the [`global`](Self::global) [`Offset`] is in the coordinate space of the screen
/// after conversion to logical pixels and the [`local`](Self::local) offset is the same
/// [`Offset`], but transformed to a local coordinate space.
#[derive(Clone, Copy, Debug)]
pub struct OffsetPair {
    /// The [`Offset`] in the local coordinate space.
    pub local: Offset,

    /// The [`Offset`] in the global coordinate space after conversion to logical
    /// pixels.
    pub global: Offset,
}

impl OffsetPair {
    /// A [`OffsetPair`] where both [`Offset`]s are [`Offset::ZERO`].
    pub const ZERO: OffsetPair = OffsetPair {
        local: Offset::ZERO,
        global: Offset::ZERO,
    };

    /// Creates a [`OffsetPair`] combining a [`local`](Self::local) and [`global`](Self::global) [`Offset`].
    pub const fn new(local: Offset, global: Offset) -> OffsetPair {
        OffsetPair { local, global }
    }

    /// Creates a [`OffsetPair`] from [`PointerEvent::local_position`] and
    /// [`PointerEvent::position`].
    pub fn from_event_position(event: &PointerEvent) -> OffsetPair {
        OffsetPair {
            local: event.local_position(),
            global: event.position(),
        }
    }

    /// Creates a [`OffsetPair`] from [`PointerEvent::local_delta`] and
    /// [`PointerEvent::delta`].
    pub fn from_event_delta(event: &PointerEvent) -> OffsetPair {
        OffsetPair {
            local: event.local_delta(),
            global: event.delta(),
        }
    }
}

impl std::ops::Add for OffsetPair {
    type Output = OffsetPair;

    fn add(self, other: OffsetPair) -> OffsetPair {
        OffsetPair {
            local: self.local + other.local,
            global: self.global + other.global,
        }
    }
}

impl std::ops::Sub for OffsetPair {
    type Output = OffsetPair;

    fn sub(self, other: OffsetPair) -> OffsetPair {
        OffsetPair {
            local: self.local - other.local,
            global: self.global - other.global,
        }
    }
}

impl fmt::Display for OffsetPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OffsetPair(local: {:?}, global: {:?})",
            self.local, self.global
        )
    }
}

/// Superclass fields on every `OneSequence` / `PrimaryPointer` leaf.
pub(crate) trait RecognizerLeafData {
    fn recognizer(&self) -> &GestureRecognizerData;
    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData;
    fn one_sequence(&self) -> &OneSequenceData;
    fn one_sequence_mut(&mut self) -> &mut OneSequenceData;
    fn primary(&self) -> &PrimaryPointerData;
    fn primary_mut(&mut self) -> &mut PrimaryPointerData;
}

/// Virtuals a superclass body calls on the leaf. Defaults match
/// `PrimaryPointerGestureRecognizer` (the next class after `OneSequence`).
///
/// A leaf competes in the arena as its `Handle`: see the [`GestureArenaMember`]
/// impl below.
pub(crate) trait RecognizerLeaf: RecognizerLeafData + Sized + 'static {
    /// The route registered per tracked pointer. A distinct fn item per leaf, so
    /// the `TypeId` matches on remove.
    fn handle_event_route(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        self.handle_event(app, event);
    }

    /// The deadline timer's callback. A distinct fn item per leaf.
    fn deadline_fired(self: Handle<Self>, app: &mut App) {
        self.did_exceed_deadline_with_event(app);
    }

    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        PrimaryPointerGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        _app: &mut App,
        _event: crate::events::PointerPanZoomStartEvent,
    ) {
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        PrimaryPointerGestureRecognizer::handle_non_allowed_pointer(self, app, event);
    }

    fn handle_non_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        _app: &mut App,
        _event: &crate::events::PointerPanZoomStartEvent,
    ) {
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn is_pointer_pan_zoom_allowed(
        self: Handle<Self>,
        app: &App,
        event: &crate::events::PointerPanZoomStartEvent,
    ) -> bool {
        GestureRecognizer::is_pointer_pan_zoom_allowed(self, app, event)
    }

    fn start_tracking_pointer(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        OneSequenceGestureRecognizer::start_tracking_pointer(self, app, pointer, transform);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        PrimaryPointerGestureRecognizer::handle_event(self, app, event);
    }

    fn handle_primary_pointer(self: Handle<Self>, app: &mut App, event: PointerEvent);

    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            app.get(self).primary().deadline.is_none(),
            "must override didExceedDeadline if deadline is set"
        );
    }

    fn did_exceed_deadline_with_event(self: Handle<Self>, app: &mut App) {
        PrimaryPointerGestureRecognizer::did_exceed_deadline_with_event(self, app);
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::accept_gesture(self, app, pointer);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::reject_gesture(self, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
    }

    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition) {
        OneSequenceGestureRecognizer::resolve(self, app, disposition);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        PrimaryPointerGestureRecognizer::dispose(self, app);
    }
}

/// Dart's `GestureRecognizer extends GestureArenaMember`: every leaf competes in
/// the arena as its `Handle`, and its identity is the handle.
impl<R: RecognizerLeaf> GestureArenaMember for Handle<R> {
    fn accept_gesture(&self, app: &mut App, pointer: i64) {
        R::accept_gesture(*self, app, pointer);
    }

    fn reject_gesture(&self, app: &mut App, pointer: i64) {
        R::reject_gesture(*self, app, pointer);
    }

    fn member_id(&self) -> HandleId {
        self.id()
    }
}

pub(crate) struct GestureRecognizer;

impl GestureRecognizer {
    pub(crate) fn add_pointer_pan_zoom<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: crate::events::PointerPanZoomStartEvent,
    ) {
        app.get_mut(this)
            .recognizer_mut()
            .pointer_to_event_data
            .insert(
                event.pointer,
                RecognizerEventData {
                    kind: event.kind,
                    buttons: event.buttons,
                },
            );
        if this.is_pointer_pan_zoom_allowed(app, &event) {
            this.add_allowed_pointer_pan_zoom(app, event);
        } else {
            this.handle_non_allowed_pointer_pan_zoom(app, &event);
        }
    }

    pub(crate) fn add_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: PointerDownEvent,
    ) {
        app.get_mut(this)
            .recognizer_mut()
            .pointer_to_event_data
            .insert(
                event.pointer,
                RecognizerEventData {
                    kind: event.kind,
                    buttons: event.buttons,
                },
            );
        if this.is_pointer_allowed(app, &event) {
            this.add_allowed_pointer(app, event);
        } else {
            this.handle_non_allowed_pointer(app, &event);
        }
    }

    pub(crate) fn is_pointer_allowed<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        event: &PointerDownEvent,
    ) -> bool {
        let devices = app.get(this).recognizer().supported_devices.clone();
        let filter = Rc::clone(&app.get(this).recognizer().allowed_buttons_filter);
        (devices.is_none()
            || devices
                .as_ref()
                .is_some_and(|set| set.contains(&event.kind)))
            && filter(event.buttons)
    }

    pub(crate) fn is_pointer_pan_zoom_allowed<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        event: &crate::events::PointerPanZoomStartEvent,
    ) -> bool {
        let devices = app.get(this).recognizer().supported_devices.clone();
        devices.is_none()
            || devices
                .as_ref()
                .is_some_and(|set| set.contains(&event.kind))
    }

    pub(crate) fn kind_for_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        pointer: i64,
    ) -> PointerDeviceKind {
        let data = &app.get(this).recognizer().pointer_to_event_data;
        debug_assert!(data.contains_key(&pointer));
        data[&pointer].kind
    }

    #[allow(dead_code)]
    pub(crate) fn buttons_for_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        pointer: i64,
    ) -> i64 {
        let data = &app.get(this).recognizer().pointer_to_event_data;
        debug_assert!(data.contains_key(&pointer));
        data[&pointer].buttons
    }

    pub(crate) fn dispose<R: RecognizerLeaf>(_this: Handle<R>, _app: &mut App) {}

    pub(crate) fn invoke_callback<R: RecognizerLeaf, T>(
        this: Handle<R>,
        app: &mut App,
        name: &str,
        callback: impl FnOnce(&mut App) -> T,
    ) -> Option<T> {
        if cfg!(debug_assertions) && debug_print_recognizer_callbacks_trace() {
            let prefix = if debug_print_gesture_arena_diagnostics() {
                format!("{}❙ ", " ".repeat(19))
            } else {
                String::new()
            };
            eprintln!("{prefix}{this:?} calling {name} callback.");
        }
        Some(callback(app))
    }
}

pub(crate) struct OneSequenceGestureRecognizer;

impl OneSequenceGestureRecognizer {
    pub(crate) fn add_allowed_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: &PointerDownEvent,
    ) {
        this.start_tracking_pointer(app, event.pointer, event.transform);
    }

    pub(crate) fn handle_non_allowed_pointer<R: RecognizerLeaf>(this: Handle<R>, app: &mut App) {
        this.resolve(app, GestureDisposition::Rejected);
    }

    pub(crate) fn resolve<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        disposition: GestureDisposition,
    ) {
        let local_entries: Vec<GestureArenaEntry> = app
            .get(this)
            .one_sequence()
            .entries
            .values()
            .cloned()
            .collect();
        app.get_mut(this).one_sequence_mut().entries.clear();
        for entry in local_entries {
            entry.resolve(app, disposition);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        pointer: i64,
        disposition: GestureDisposition,
    ) {
        let entry = app
            .get_mut(this)
            .one_sequence_mut()
            .entries
            .remove(&pointer);
        if let Some(entry) = entry {
            entry.resolve(app, disposition);
        }
    }

    pub(crate) fn dispose<R: RecognizerLeaf>(this: Handle<R>, app: &mut App) {
        this.resolve(app, GestureDisposition::Rejected);
        let pointers: Vec<i64> = app
            .get(this)
            .one_sequence()
            .tracked_pointers
            .iter()
            .copied()
            .collect();
        let router = GestureBinding::instance(app).pointer_router(app);
        for pointer in pointers {
            router.remove_route(
                app,
                pointer,
                &PointerRoute::handle_method(this, R::handle_event_route),
            );
        }
        app.get_mut(this)
            .one_sequence_mut()
            .tracked_pointers
            .clear();
        debug_assert!(app.get(this).one_sequence().entries.is_empty());
        GestureRecognizer::dispose(this, app);
    }

    pub(crate) fn add_pointer_to_arena<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        pointer: i64,
    ) -> GestureArenaEntry {
        let team = app.get(this).one_sequence().team;
        if let Some(team) = team {
            team.add(app, pointer, this)
        } else {
            GestureBinding::instance(app)
                .gesture_arena(app)
                .add(app, pointer, this)
        }
    }

    pub(crate) fn start_tracking_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        let router = GestureBinding::instance(app).pointer_router(app);
        router.add_route(
            app,
            pointer,
            PointerRoute::handle_method(this, R::handle_event_route),
            transform,
        );
        app.get_mut(this)
            .one_sequence_mut()
            .tracked_pointers
            .insert(pointer);
        let entry = Self::add_pointer_to_arena(this, app, pointer);
        app.get_mut(this)
            .one_sequence_mut()
            .entries
            .insert(pointer, entry);
    }

    pub(crate) fn stop_tracking_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        pointer: i64,
    ) {
        if app
            .get(this)
            .one_sequence()
            .tracked_pointers
            .contains(&pointer)
        {
            let router = GestureBinding::instance(app).pointer_router(app);
            router.remove_route(
                app,
                pointer,
                &PointerRoute::handle_method(this, R::handle_event_route),
            );
            app.get_mut(this)
                .one_sequence_mut()
                .tracked_pointers
                .remove(&pointer);
            if app.get(this).one_sequence().tracked_pointers.is_empty() {
                this.did_stop_tracking_last_pointer(app, pointer);
            }
        }
    }

    pub(crate) fn stop_tracking_if_pointer_no_longer_down<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: &PointerEvent,
    ) {
        if matches!(
            event,
            PointerEvent::Up(_) | PointerEvent::Cancel(_) | PointerEvent::PanZoomEnd(_)
        ) {
            Self::stop_tracking_pointer(this, app, event.pointer());
        }
    }
}

pub(crate) struct PrimaryPointerGestureRecognizer;

impl PrimaryPointerGestureRecognizer {
    pub(crate) fn add_allowed_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: PointerDownEvent,
    ) {
        OneSequenceGestureRecognizer::add_allowed_pointer(this, app, &event);
        if app.get(this).primary().state == GestureRecognizerState::Ready {
            app.get_mut(this).primary_mut().state = GestureRecognizerState::Possible;
            app.get_mut(this).primary_mut().primary_pointer = Some(event.pointer);
            app.get_mut(this).primary_mut().initial_position =
                Some(OffsetPair::new(event.local_position(), event.position));
            let deadline = app.get(this).primary().deadline;
            if let Some(deadline) = deadline {
                app.get_mut(this).primary_mut().deadline_event = Some(event);
                let timer = Timer::new(
                    app,
                    deadline,
                    Listener::handle_method(this, R::deadline_fired),
                );
                app.get_mut(this).primary_mut().timer = Some(timer);
            }
        }
    }

    pub(crate) fn handle_non_allowed_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        _event: &PointerDownEvent,
    ) {
        if !app.get(this).primary().gesture_accepted {
            OneSequenceGestureRecognizer::handle_non_allowed_pointer(this, app);
        }
    }

    pub(crate) fn handle_event<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        event: PointerEvent,
    ) {
        debug_assert!(app.get(this).primary().state != GestureRecognizerState::Ready);
        let primary = app.get(this).primary().primary_pointer;
        if app.get(this).primary().state == GestureRecognizerState::Possible
            && Some(event.pointer()) == primary
        {
            let gesture_accepted = app.get(this).primary().gesture_accepted;
            let pre = Self::pre_accept_slop_tolerance(this, app);
            let post = Self::post_accept_slop_tolerance(this, app);
            let distance = Self::global_distance(this, app, &event);
            let is_pre_accept_slop_past_tolerance =
                !gesture_accepted && pre.is_some() && distance > pre.unwrap();
            let is_post_accept_slop_past_tolerance =
                gesture_accepted && post.is_some() && distance > post.unwrap();
            if matches!(event, PointerEvent::Move(_))
                && (is_pre_accept_slop_past_tolerance || is_post_accept_slop_past_tolerance)
            {
                this.resolve(app, GestureDisposition::Rejected);
                OneSequenceGestureRecognizer::stop_tracking_pointer(this, app, primary.unwrap());
            } else {
                this.handle_primary_pointer(app, event.clone());
            }
        }
        OneSequenceGestureRecognizer::stop_tracking_if_pointer_no_longer_down(this, app, &event);
    }

    pub(crate) fn did_exceed_deadline_with_event<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
    ) {
        this.did_exceed_deadline(app);
    }

    pub(crate) fn accept_gesture<R: RecognizerLeaf>(this: Handle<R>, app: &mut App, pointer: i64) {
        if Some(pointer) == app.get(this).primary().primary_pointer {
            Self::stop_timer(this, app);
            app.get_mut(this).primary_mut().gesture_accepted = true;
        }
    }

    pub(crate) fn reject_gesture<R: RecognizerLeaf>(this: Handle<R>, app: &mut App, pointer: i64) {
        if Some(pointer) == app.get(this).primary().primary_pointer
            && app.get(this).primary().state == GestureRecognizerState::Possible
        {
            Self::stop_timer(this, app);
            app.get_mut(this).primary_mut().state = GestureRecognizerState::Defunct;
        }
    }

    pub(crate) fn did_stop_tracking_last_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &mut App,
        _pointer: i64,
    ) {
        debug_assert!(app.get(this).primary().state != GestureRecognizerState::Ready);
        Self::stop_timer(this, app);
        app.get_mut(this).primary_mut().state = GestureRecognizerState::Ready;
        app.get_mut(this).primary_mut().initial_position = None;
        app.get_mut(this).primary_mut().gesture_accepted = false;
    }

    pub(crate) fn dispose<R: RecognizerLeaf>(this: Handle<R>, app: &mut App) {
        Self::stop_timer(this, app);
        OneSequenceGestureRecognizer::dispose(this, app);
    }

    pub(crate) fn stop_timer<R: RecognizerLeaf>(this: Handle<R>, app: &mut App) {
        if let Some(timer) = app.get_mut(this).primary_mut().timer.take() {
            timer.cancel(app);
        }
    }

    pub(crate) fn pre_accept_slop_tolerance<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
    ) -> Option<f64> {
        let stored = app.get(this).primary().pre_accept_slop_tolerance;
        if stored == Some(UNSET_TOUCH_SLOP) {
            Some(Self::default_touch_slop(this, app))
        } else {
            stored
        }
    }

    pub(crate) fn post_accept_slop_tolerance<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
    ) -> Option<f64> {
        let stored = app.get(this).primary().post_accept_slop_tolerance;
        if stored == Some(UNSET_TOUCH_SLOP) {
            Some(Self::default_touch_slop(this, app))
        } else {
            stored
        }
    }

    pub(crate) fn default_touch_slop<R: RecognizerLeaf>(this: Handle<R>, app: &App) -> f64 {
        app.get(this)
            .recognizer()
            .gesture_settings
            .and_then(|settings| settings.touch_slop)
            .unwrap_or(K_TOUCH_SLOP)
    }

    pub(crate) fn global_distance<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        event: &PointerEvent,
    ) -> f64 {
        let initial = app.get(this).primary().initial_position.unwrap();
        (event.position() - initial.global).distance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::Offset;

    use crate::events::{PointerDownEvent, PointerEvent};

    #[test]
    fn from_event_position_and_delta_and_arithmetic() {
        let event = PointerEvent::Down(PointerDownEvent {
            position: Offset::new(10.0, 20.0),
            delta: Offset::new(1.0, 2.0),
            ..PointerDownEvent::default()
        });
        let position = OffsetPair::from_event_position(&event);
        assert_eq!(position.global, Offset::new(10.0, 20.0));
        assert_eq!(position.local, Offset::new(10.0, 20.0));
        let delta = OffsetPair::from_event_delta(&event);
        assert_eq!(delta.global, Offset::new(1.0, 2.0));
        let sum = position + delta;
        assert_eq!(sum.global, Offset::new(11.0, 22.0));
        assert_eq!((sum - delta).global, position.global);
        assert_eq!(OffsetPair::ZERO.global, Offset::ZERO);
    }
}

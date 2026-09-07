//! Flutter counterpart: `gestures/recognizer.dart`.
//!
//! Superclass field bags and `super` namespaces. Leaves implement
//! [`RecognizerLeaf`] so a superclass body calls the leaf, not a sibling. A
//! `super` call is a namespace fn taking the leaf's `Handle` first.
//!
//! Dart's `GestureRecognizer` used as a type (a field, a `Map<Type,
//! GestureRecognizer>` value) is the erased [`AnyGestureRecognizer`], minted by
//! [`GestureRecognizerLeaf::as_recognizer`].

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Matrix4, Offset, PointerDeviceKind};
use reveal_foundation::{App, Handle, HandleId, Listener, Timer};

use crate::arena::{GestureArenaEntry, GestureArenaMember, GestureDisposition};
use crate::binding::GestureBinding;
use crate::constants::K_TOUCH_SLOP;
use crate::debug::{debug_print_gesture_arena_diagnostics, debug_print_recognizer_callbacks_trace};
use crate::events::{PointerDownEvent, PointerEvent, PointerPanZoomStartEvent};
use crate::gesture_settings::DeviceGestureSettings;
use crate::pointer_router::PointerRoute;
use crate::team::GestureArenaTeam;

/// Signature for a recognizer's `allowed_buttons_filter`.
///
/// Used to filter the input buttons of incoming pointer events.
/// The parameter `buttons` comes from `PointerEvent.buttons`.
pub type AllowedButtonsFilter = Rc<dyn Fn(i64) -> bool>;

/// `-1` is used as a sentinel value to indicate no touch slop was specified.
///
/// Pass it to [`PrimaryPointerData::new`] where Dart omits `preAcceptSlopTolerance`
/// or `postAcceptSlopTolerance`.
pub const UNSET_TOUCH_SLOP: f64 = -1.0;

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

/// Field bag for Dart's `GestureRecognizer`, the base class that all gesture
/// recognizers inherit from.
///
/// A leaf holds one and hands it out through [`RecognizerLeafData::recognizer`].
pub struct GestureRecognizerData {
    pub(crate) gesture_settings: Option<DeviceGestureSettings>,
    pub(crate) supported_devices: Option<HashSet<PointerDeviceKind>>,
    pub(crate) allowed_buttons_filter: AllowedButtonsFilter,
    pub(crate) pointer_to_event_data: HashMap<i64, RecognizerEventData>,
}

impl GestureRecognizerData {
    /// Initializes the gesture recognizer: every device kind is recognized, and
    /// every input button is accepted.
    pub fn new() -> GestureRecognizerData {
        GestureRecognizerData {
            gesture_settings: None,
            supported_devices: None,
            allowed_buttons_filter: Rc::new(default_button_accept_behavior),
            pointer_to_event_data: HashMap::new(),
        }
    }
}

impl GestureRecognizerData {
    /// Optional device specific configuration that takes precedence over framework defaults.
    pub fn gesture_settings(&self) -> Option<DeviceGestureSettings> {
        self.gesture_settings
    }

    /// Sets [`gesture_settings`](Self::gesture_settings).
    pub fn set_gesture_settings(&mut self, settings: Option<DeviceGestureSettings>) {
        self.gesture_settings = settings;
    }

    /// Called when interaction starts; limits the buttons this recognizer accepts.
    pub fn allowed_buttons_filter(&self) -> &AllowedButtonsFilter {
        &self.allowed_buttons_filter
    }

    /// Sets [`allowed_buttons_filter`](Self::allowed_buttons_filter).
    pub fn set_allowed_buttons_filter(&mut self, filter: AllowedButtonsFilter) {
        self.allowed_buttons_filter = filter;
    }
}

impl Default for GestureRecognizerData {
    fn default() -> GestureRecognizerData {
        GestureRecognizerData::new()
    }
}

/// Field bag for Dart's `OneSequenceGestureRecognizer`, the base class for
/// gesture recognizers that can only recognize one gesture at a time.
///
/// A leaf holds one and hands it out through [`RecognizerLeafData::one_sequence`].
pub struct OneSequenceData {
    pub(crate) entries: HashMap<i64, GestureArenaEntry>,
    pub(crate) tracked_pointers: HashSet<i64>,
    pub(crate) team: Option<Handle<GestureArenaTeam>>,
}

impl OneSequenceData {
    /// Initializes the object: no arena entries, no tracked pointers, no team.
    pub fn new() -> OneSequenceData {
        OneSequenceData {
            entries: HashMap::new(),
            tracked_pointers: HashSet::new(),
            team: None,
        }
    }
}

impl Default for OneSequenceData {
    fn default() -> OneSequenceData {
        OneSequenceData::new()
    }
}

/// Field bag for Dart's `PrimaryPointerGestureRecognizer`, the base class for
/// gesture recognizers that track a single primary pointer.
///
/// A leaf holds one and hands it out through [`PrimaryPointerLeafData::primary`].
pub struct PrimaryPointerData {
    pub(crate) deadline: Option<Duration>,
    pub(crate) pre_accept_slop_tolerance: Option<f64>,
    pub(crate) post_accept_slop_tolerance: Option<f64>,
    pub(crate) state: GestureRecognizerState,
    pub(crate) primary_pointer: Option<i64>,
    pub(crate) initial_position: Option<OffsetPair>,
    pub(crate) gesture_accepted: bool,
    pub(crate) timer: Option<Timer>,
    pub(crate) deadline_event: Option<crate::events::PointerDownEvent>,
}

impl PrimaryPointerData {
    /// Initializes the `deadline` field during construction of a leaf.
    ///
    /// If `deadline` is non-null, the recognizer calls
    /// [`PrimaryPointerLeaf::did_exceed_deadline`] after that amount of time has
    /// elapsed since starting to track the primary pointer.
    ///
    /// A slop tolerance of [`UNSET_TOUCH_SLOP`] is Dart's omitted argument: the
    /// tolerance then follows `gesture_settings.touch_slop`, falling back to
    /// [`K_TOUCH_SLOP`](crate::K_TOUCH_SLOP). `None` lets the gesture drift any
    /// distance.
    pub fn new(
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

/// Superclass fields on every `GestureRecognizer` leaf.
pub trait RecognizerLeafData {
    /// The leaf's [`GestureRecognizer`] bag.
    fn recognizer(&self) -> &GestureRecognizerData;

    /// The leaf's [`GestureRecognizer`] bag, mutably.
    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData;
}

/// The extra field bag carried by `OneSequenceGestureRecognizer` leaves.
pub trait OneSequenceLeafData: RecognizerLeafData {
    /// The leaf's [`OneSequenceGestureRecognizer`] bag.
    fn one_sequence(&self) -> &OneSequenceData;

    /// The leaf's [`OneSequenceGestureRecognizer`] bag, mutably.
    fn one_sequence_mut(&mut self) -> &mut OneSequenceData;
}

/// The extra field bag a [`PrimaryPointerGestureRecognizer`] leaf carries.
pub trait PrimaryPointerLeafData: OneSequenceLeafData {
    /// The leaf's [`PrimaryPointerGestureRecognizer`] bag.
    fn primary(&self) -> &PrimaryPointerData;

    /// The leaf's [`PrimaryPointerGestureRecognizer`] bag, mutably.
    fn primary_mut(&mut self) -> &mut PrimaryPointerData;
}

/// Virtuals a superclass body calls on the leaf. Defaults match `GestureRecognizer`;
/// leaves forward inherited overrides to their immediate superclass.
///
/// A leaf competes in the arena as its `Handle`: see the [`GestureArenaMember`]
/// impl below.
pub trait RecognizerLeaf: RecognizerLeafData + Sized + 'static {
    /// The route registered per tracked pointer. A distinct fn item per leaf, so
    /// the `TypeId` matches on remove.
    fn handle_event_route(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        self.handle_event(app, event);
    }

    /// Registers a new pointer that might be relevant to this gesture recognizer.
    ///
    /// This is called for each and all pointers being added. A leaf that only
    /// cares about the pointers it accepts overrides
    /// [`add_allowed_pointer`](Self::add_allowed_pointer) instead.
    fn add_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        GestureRecognizer::add_pointer(self, app, event);
    }

    /// Registers a new pointer pan/zoom that might be relevant to this gesture
    /// recognizer.
    fn add_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: crate::events::PointerPanZoomStartEvent,
    ) {
        GestureRecognizer::add_pointer_pan_zoom(self, app, event);
    }

    /// Registers a new pointer that's been checked to be allowed by this gesture
    /// recognizer.
    ///
    /// Override this instead of [`add_pointer`](Self::add_pointer), which is
    /// called for each and all pointers being added.
    fn add_allowed_pointer(self: Handle<Self>, _app: &mut App, _event: PointerDownEvent) {}

    /// Registers a new pointer pan/zoom that's been checked to be allowed by this
    /// gesture recognizer.
    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        _app: &mut App,
        _event: crate::events::PointerPanZoomStartEvent,
    ) {
    }

    /// Handles a pointer being added that's not allowed by this recognizer.
    fn handle_non_allowed_pointer(self: Handle<Self>, _app: &mut App, _event: &PointerDownEvent) {}

    /// Handles a pointer pan/zoom being added that's not allowed by this
    /// recognizer.
    fn handle_non_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        _app: &mut App,
        _event: &crate::events::PointerPanZoomStartEvent,
    ) {
    }

    /// Checks whether or not a pointer is allowed to be tracked by this
    /// recognizer.
    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    /// Checks whether or not a pointer pan/zoom is allowed to be tracked by this
    /// recognizer.
    fn is_pointer_pan_zoom_allowed(
        self: Handle<Self>,
        app: &App,
        event: &crate::events::PointerPanZoomStartEvent,
    ) -> bool {
        GestureRecognizer::is_pointer_pan_zoom_allowed(self, app, event)
    }

    /// Causes events related to the given pointer ID to be routed to this
    /// recognizer.
    fn start_tracking_pointer(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) where
        Self: OneSequenceLeafData,
    {
        OneSequenceGestureRecognizer::start_tracking_pointer(self, app, pointer, transform);
    }

    /// Called when a pointer event is routed to this recognizer.
    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent);

    /// Called when this member wins the arena for the given pointer id.
    fn accept_gesture(self: Handle<Self>, _app: &mut App, _pointer: i64) {}

    /// Called when this member loses the arena for the given pointer id.
    fn reject_gesture(self: Handle<Self>, _app: &mut App, _pointer: i64) {}

    /// Called when the number of pointers this recognizer is tracking changes
    /// from one to zero.
    fn did_stop_tracking_last_pointer(self: Handle<Self>, _app: &mut App, _pointer: i64) {}

    /// Resolves this recognizer's participation in each gesture arena with the
    /// given disposition.
    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition)
    where
        Self: OneSequenceLeafData,
    {
        OneSequenceGestureRecognizer::resolve(self, app, disposition);
    }

    /// Releases any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App) {
        GestureRecognizer::dispose(self, app);
    }

    /// Returns a very short pretty description of the gesture that the
    /// recognizer looks for.
    fn debug_description(self: Handle<Self>) -> &'static str;
}

/// Virtuals [`PrimaryPointerGestureRecognizer`]'s bodies call on its leaves.
pub trait PrimaryPointerLeaf: RecognizerLeaf + PrimaryPointerLeafData {
    /// The deadline timer's callback. A distinct fn item per leaf.
    fn deadline_fired(self: Handle<Self>, app: &mut App) {
        self.did_exceed_deadline_with_event(app);
    }

    /// Override to provide behavior for the primary pointer when the gesture is
    /// still possible.
    fn handle_primary_pointer(self: Handle<Self>, app: &mut App, event: PointerEvent);

    /// Override to be notified when the deadline is exceeded.
    ///
    /// You must override this method or [`did_exceed_deadline_with_event`](Self::did_exceed_deadline_with_event)
    /// if you supply a deadline. An override must _not_ call this body.
    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            app.get(self).primary().deadline.is_none(),
            "must override didExceedDeadline if deadline is set"
        );
    }

    /// Same as [`did_exceed_deadline`](Self::did_exceed_deadline), for the down
    /// event that initiated the gesture.
    ///
    /// You must override this method or [`did_exceed_deadline`](Self::did_exceed_deadline)
    /// if you supply a deadline. An override must _not_ call this body.
    fn did_exceed_deadline_with_event(self: Handle<Self>, app: &mut App) {
        PrimaryPointerGestureRecognizer::did_exceed_deadline_with_event(self, app);
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

/// A concrete recognizer: the bound Dart writes as `T extends GestureRecognizer`.
///
/// Every leaf ([`TapGestureRecognizer`](crate::TapGestureRecognizer),
/// [`LongPressGestureRecognizer`](crate::LongPressGestureRecognizer)) implements it. It hands
/// out the erased [`AnyGestureRecognizer`] that a field Dart types as `GestureRecognizer`
/// holds.
pub trait GestureRecognizerLeaf: Sized + 'static {
    /// This recognizer as the erased [`AnyGestureRecognizer`] — what to pass where a Dart
    /// API takes a `GestureRecognizer`.
    ///
    /// The object is untouched; this mints an erased second handle to it, so concrete
    /// members stay reachable through the typed one.
    fn as_recognizer(self: Handle<Self>) -> AnyGestureRecognizer;
}

impl<R: RecognizerLeaf> GestureRecognizerLeaf for R {
    fn as_recognizer(self: Handle<Self>) -> AnyGestureRecognizer {
        AnyGestureRecognizer {
            id: self.id(),
            vtable: const { &GestureRecognizerVTable::of::<R>() },
        }
    }
}

/// The vtable of an erased [`AnyGestureRecognizer`]: one `&'static` table per leaf type,
/// built by [`GestureRecognizerVTable::of`]. Copied out of the handle before a virtual call,
/// so the slot is not borrowed across it.
pub(crate) struct GestureRecognizerVTable {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
    add_pointer: fn(&mut App, HandleId, PointerDownEvent),
    add_pointer_pan_zoom: fn(&mut App, HandleId, PointerPanZoomStartEvent),
    is_pointer_allowed: fn(&App, HandleId, &PointerDownEvent) -> bool,
    is_pointer_pan_zoom_allowed: fn(&App, HandleId, &PointerPanZoomStartEvent) -> bool,
    dispose: fn(&mut App, HandleId),
    set_gesture_settings: fn(&mut App, HandleId, Option<DeviceGestureSettings>),
    debug_description: fn(HandleId) -> &'static str,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<R: 'static>(id: HandleId) -> Handle<R> {
    Handle::from_id(id)
}

impl GestureRecognizerVTable {
    /// The table for one leaf type.
    const fn of<R: RecognizerLeaf>() -> GestureRecognizerVTable {
        GestureRecognizerVTable {
            type_id: TypeId::of::<R>,
            type_name: std::any::type_name::<R>,
            add_pointer: |app, id, event| R::add_pointer(resolve::<R>(id), app, event),
            add_pointer_pan_zoom: |app, id, event| {
                R::add_pointer_pan_zoom(resolve::<R>(id), app, event)
            },
            is_pointer_allowed: |app, id, event| R::is_pointer_allowed(resolve(id), app, event),
            is_pointer_pan_zoom_allowed: |app, id, event| {
                R::is_pointer_pan_zoom_allowed(resolve(id), app, event)
            },
            dispose: |app, id| R::dispose(resolve(id), app),
            set_gesture_settings: |app, id, value| {
                app.get_mut(resolve::<R>(id))
                    .recognizer_mut()
                    .set_gesture_settings(value)
            },
            debug_description: |id| R::debug_description(resolve(id)),
        }
    }
}

/// Erased `GestureRecognizer`: one identity and a static vtable, the fat pointer rustc
/// cannot build for an arena id. No lease.
///
/// This is what a field Dart types as `GestureRecognizer` becomes. The leaf stays in the
/// [`App`] under its own type; [`downcast`](Self::downcast) gets the typed handle back, and
/// [`type_id`](Self::type_id) is Dart's `runtimeType`.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyGestureRecognizer {
    id: HandleId,
    vtable: &'static GestureRecognizerVTable,
}

impl PartialEq for AnyGestureRecognizer {
    fn eq(&self, other: &AnyGestureRecognizer) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyGestureRecognizer {}

impl Hash for AnyGestureRecognizer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for AnyGestureRecognizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyGestureRecognizer {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `runtimeType`: the leaf type this handle was minted from.
    pub fn type_id(self) -> TypeId {
        (self.vtable.type_id)()
    }

    /// Dart's `recognizer as T`: the typed handle when this recognizer is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// Optional device-specific configuration for device gestures.
    pub fn set_gesture_settings(self, app: &mut App, value: Option<DeviceGestureSettings>) {
        (self.vtable.set_gesture_settings)(app, self.id, value);
    }

    /// Registers a new pointer that might be relevant to this gesture detector.
    ///
    /// The owner of this gesture recognizer calls `add_pointer` with the
    /// [`PointerDownEvent`] of each pointer that should be considered for this gesture.
    pub fn add_pointer(self, app: &mut App, event: &PointerDownEvent) {
        (self.vtable.add_pointer)(app, self.id, event.clone());
    }

    /// Registers a new pointer pan/zoom that might be relevant to this gesture detector.
    pub fn add_pointer_pan_zoom(self, app: &mut App, event: &PointerPanZoomStartEvent) {
        (self.vtable.add_pointer_pan_zoom)(app, self.id, event.clone());
    }

    /// Checks whether or not a pointer is allowed to be tracked by this recognizer.
    pub fn is_pointer_allowed(self, app: &App, event: &PointerDownEvent) -> bool {
        (self.vtable.is_pointer_allowed)(app, self.id, event)
    }

    /// Checks whether or not a pointer pan/zoom is allowed to be tracked by this recognizer.
    pub fn is_pointer_pan_zoom_allowed(self, app: &App, event: &PointerPanZoomStartEvent) -> bool {
        (self.vtable.is_pointer_pan_zoom_allowed)(app, self.id, event)
    }

    /// Releases any resources used by the object and frees its arena slot. Every handle to it
    /// is stale afterwards.
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// Returns a very short pretty description of the gesture that the recognizer looks for.
    pub fn debug_description(self) -> &'static str {
        (self.vtable.debug_description)(self.id)
    }
}

/// Dart's `GestureRecognizer`: the base class that all gesture recognizers
/// inherit from.
///
/// Its bodies are associated fns taking the leaf's `Handle`; a leaf's override
/// calls one where Dart writes `super`. Its fields are [`GestureRecognizerData`].
pub struct GestureRecognizer;

impl GestureRecognizer {
    /// Registers a new pointer pan/zoom that might be relevant to this gesture
    /// detector.
    ///
    /// A pointer pan/zoom is a stream of events that conveys data covering pan,
    /// zoom, and rotate data from a multi-finger trackpad gesture.
    ///
    /// This is called for each and all pointers being added. In most cases, a leaf
    /// overrides [`RecognizerLeaf::add_allowed_pointer_pan_zoom`] instead.
    pub fn add_pointer_pan_zoom<R: RecognizerLeaf>(
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

    /// Registers a new pointer that might be relevant to this gesture detector.
    ///
    /// The owner of this gesture recognizer calls it with the [`PointerDownEvent`]
    /// of each pointer that should be considered for this gesture.
    ///
    /// This is called for each and all pointers being added. In most cases, a leaf
    /// overrides [`RecognizerLeaf::add_allowed_pointer`] instead.
    pub fn add_pointer<R: RecognizerLeaf>(this: Handle<R>, app: &mut App, event: PointerDownEvent) {
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

    /// Checks whether or not a pointer is allowed to be tracked by this
    /// recognizer.
    pub fn is_pointer_allowed<R: RecognizerLeaf>(
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

    /// Checks whether or not a pointer pan/zoom is allowed to be tracked by this
    /// recognizer.
    pub fn is_pointer_pan_zoom_allowed<R: RecognizerLeaf>(
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

    /// For a given pointer ID, returns the device kind associated with it.
    ///
    /// The pointer ID is expected to be a valid one, i.e. an event was received
    /// with that pointer ID.
    pub fn kind_for_pointer<R: RecognizerLeaf>(
        this: Handle<R>,
        app: &App,
        pointer: i64,
    ) -> PointerDeviceKind {
        let data = &app.get(this).recognizer().pointer_to_event_data;
        debug_assert!(data.contains_key(&pointer));
        data[&pointer].kind
    }

    /// For a given pointer ID, returns the buttons associated with it.
    ///
    /// The pointer ID is expected to be valid, meaning an event related to it has
    /// been received.
    pub fn buttons_for_pointer<R: RecognizerLeaf>(this: Handle<R>, app: &App, pointer: i64) -> i64 {
        let data = &app.get(this).recognizer().pointer_to_event_data;
        debug_assert!(data.contains_key(&pointer));
        data[&pointer].buttons
    }

    /// Releases any resources used by the object.
    ///
    /// Dart's body is a diagnostics assert and leaves the object to the collector; the arena
    /// has no collector, so the slot goes too.
    pub fn dispose<R: RecognizerLeaf>(this: Handle<R>, app: &mut App) {
        app.destroy(this);
    }

    /// Invoke a callback provided by the application.
    ///
    /// The `name` argument is ignored except when
    /// [`debug_print_recognizer_callbacks_trace`](crate::debug_print_recognizer_callbacks_trace)
    /// is true.
    pub fn invoke_callback<R: RecognizerLeaf, T>(
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

/// Dart's `OneSequenceGestureRecognizer`: the base class for gesture recognizers
/// that can only recognize one gesture at a time.
///
/// Its bodies are associated fns taking the leaf's `Handle`; a leaf's override
/// calls one where Dart writes `super`. Its fields are [`OneSequenceData`].
pub struct OneSequenceGestureRecognizer;

impl OneSequenceGestureRecognizer {
    /// Starts tracking the pointer of the given down event.
    pub fn add_allowed_pointer<R: RecognizerLeaf + OneSequenceLeafData>(
        this: Handle<R>,
        app: &mut App,
        event: &PointerDownEvent,
    ) {
        this.start_tracking_pointer(app, event.pointer, event.transform);
    }

    /// Rejects the gesture in every arena this recognizer takes part in.
    pub fn handle_non_allowed_pointer<R: RecognizerLeaf + OneSequenceLeafData>(
        this: Handle<R>,
        app: &mut App,
    ) {
        this.resolve(app, GestureDisposition::Rejected);
    }

    /// Resolves this recognizer's participation in each gesture arena with the
    /// given disposition.
    pub fn resolve<R: RecognizerLeaf + OneSequenceLeafData>(
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

    /// Resolves this recognizer's participation in the given gesture arena with
    /// the given disposition.
    pub fn resolve_pointer<R: RecognizerLeaf + OneSequenceLeafData>(
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

    /// Rejects every arena this recognizer takes part in, drops its routes, then
    /// runs [`GestureRecognizer::dispose`].
    pub fn dispose<R: RecognizerLeaf + OneSequenceLeafData>(this: Handle<R>, app: &mut App) {
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

    pub(crate) fn add_pointer_to_arena<R: RecognizerLeaf + OneSequenceLeafData>(
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

    /// Causes events related to the given pointer ID to be routed to this
    /// recognizer, and adds it (or its team) to that pointer's gesture arena.
    ///
    /// The pointer events are transformed according to `transform` and then
    /// delivered to [`RecognizerLeaf::handle_event`]. Use
    /// [`stop_tracking_pointer`](Self::stop_tracking_pointer) to remove the route.
    pub fn start_tracking_pointer<R: RecognizerLeaf + OneSequenceLeafData>(
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

    /// Stops events related to the given pointer ID from being routed to this
    /// recognizer.
    ///
    /// If this reduces the number of tracked pointers to zero, it calls
    /// [`RecognizerLeaf::did_stop_tracking_last_pointer`] synchronously.
    pub fn stop_tracking_pointer<R: RecognizerLeaf + OneSequenceLeafData>(
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

    /// Stops tracking the pointer associated with the given event if the event is
    /// a [`PointerUpEvent`](crate::PointerUpEvent), a
    /// [`PointerCancelEvent`](crate::PointerCancelEvent) or a
    /// [`PointerPanZoomEndEvent`](crate::PointerPanZoomEndEvent).
    pub fn stop_tracking_if_pointer_no_longer_down<R: RecognizerLeaf + OneSequenceLeafData>(
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

/// Dart's `PrimaryPointerGestureRecognizer`: the base class for gesture
/// recognizers that track a single primary pointer.
///
/// Gestures based on it stop tracking the gesture if the primary pointer travels
/// beyond [`pre_accept_slop_tolerance`](Self::pre_accept_slop_tolerance) or
/// [`post_accept_slop_tolerance`](Self::post_accept_slop_tolerance) pixels from
/// the original contact point. If the pre-accept tolerance was breached before
/// the gesture was accepted in the gesture arena, the gesture is rejected.
///
/// Its bodies are associated fns taking the leaf's `Handle`; a leaf's override
/// calls one where Dart writes `super`. Its fields are [`PrimaryPointerData`].
pub struct PrimaryPointerGestureRecognizer;

impl PrimaryPointerGestureRecognizer {
    /// Starts tracking the pointer, and if the recognizer is
    /// [`Ready`](GestureRecognizerState::Ready), makes it the primary pointer and
    /// arms the deadline timer.
    pub fn add_allowed_pointer<R: PrimaryPointerLeaf>(
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

    /// Rejects the gesture unless it has already been accepted.
    pub fn handle_non_allowed_pointer<R: PrimaryPointerLeaf>(
        this: Handle<R>,
        app: &mut App,
        _event: &PointerDownEvent,
    ) {
        if !app.get(this).primary().gesture_accepted {
            OneSequenceGestureRecognizer::handle_non_allowed_pointer(this, app);
        }
    }

    /// Rejects the gesture once the primary pointer drifts past the slop
    /// tolerance, and otherwise hands the event to
    /// [`PrimaryPointerLeaf::handle_primary_pointer`].
    pub fn handle_event<R: PrimaryPointerLeaf>(
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

    /// Calls [`PrimaryPointerLeaf::did_exceed_deadline`].
    pub fn did_exceed_deadline_with_event<R: PrimaryPointerLeaf>(this: Handle<R>, app: &mut App) {
        this.did_exceed_deadline(app);
    }

    /// Stops the deadline timer and marks the gesture accepted when `pointer` is
    /// the primary pointer.
    pub fn accept_gesture<R: PrimaryPointerLeaf>(this: Handle<R>, app: &mut App, pointer: i64) {
        if Some(pointer) == app.get(this).primary().primary_pointer {
            Self::stop_timer(this, app);
            app.get_mut(this).primary_mut().gesture_accepted = true;
        }
    }

    /// Stops the deadline timer and makes the recognizer
    /// [`Defunct`](GestureRecognizerState::Defunct) when `pointer` is the primary
    /// pointer of a still-possible gesture.
    pub fn reject_gesture<R: PrimaryPointerLeaf>(this: Handle<R>, app: &mut App, pointer: i64) {
        if Some(pointer) == app.get(this).primary().primary_pointer
            && app.get(this).primary().state == GestureRecognizerState::Possible
        {
            Self::stop_timer(this, app);
            app.get_mut(this).primary_mut().state = GestureRecognizerState::Defunct;
        }
    }

    /// Returns the recognizer to the [`Ready`](GestureRecognizerState::Ready)
    /// state.
    pub fn did_stop_tracking_last_pointer<R: PrimaryPointerLeaf>(
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

    /// Stops the deadline timer, then runs
    /// [`OneSequenceGestureRecognizer::dispose`].
    pub fn dispose<R: PrimaryPointerLeaf>(this: Handle<R>, app: &mut App) {
        Self::stop_timer(this, app);
        OneSequenceGestureRecognizer::dispose(this, app);
    }

    /// Cancels the deadline timer, if one is running.
    pub fn stop_timer<R: PrimaryPointerLeaf>(this: Handle<R>, app: &mut App) {
        if let Some(timer) = app.get_mut(this).primary_mut().timer.take() {
            timer.cancel(app);
        }
    }

    /// The maximum distance in logical pixels the gesture is allowed to drift
    /// from the initial touch down position before the gesture is accepted.
    ///
    /// `None` means the gesture can drift for any distance.
    pub fn pre_accept_slop_tolerance<R: PrimaryPointerLeaf>(
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

    /// The maximum distance in logical pixels the gesture is allowed to drift
    /// after the gesture has been accepted.
    ///
    /// `None` means the gesture can drift for any distance.
    pub fn post_accept_slop_tolerance<R: PrimaryPointerLeaf>(
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

    /// `gesture_settings.touch_slop`, falling back to
    /// [`K_TOUCH_SLOP`](crate::K_TOUCH_SLOP).
    pub fn default_touch_slop<R: PrimaryPointerLeaf>(this: Handle<R>, app: &App) -> f64 {
        app.get(this)
            .recognizer()
            .gesture_settings
            .and_then(|settings| settings.touch_slop)
            .unwrap_or(K_TOUCH_SLOP)
    }

    /// How far the event is from the primary pointer's initial global position.
    pub fn global_distance<R: PrimaryPointerLeaf>(
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
    use reveal_foundation::AppCell;
    use std::cell::Cell;

    use super::*;
    use reveal_embedder::Offset;

    use crate::events::{PointerDownEvent, PointerEvent};
    use crate::long_press::LongPressGestureRecognizer;
    use crate::tap::TapGestureRecognizer;

    #[test]
    fn an_erased_recognizer_keeps_its_identity_and_type() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap = TapGestureRecognizer::new(&mut app);
        let long_press = LongPressGestureRecognizer::new(&mut app);
        let erased = tap.as_recognizer();
        assert_eq!(erased, tap.as_recognizer());
        assert_ne!(erased, long_press.as_recognizer());
        assert_eq!(erased.id(), tap.id());
        assert_eq!(erased.type_id(), TypeId::of::<TapGestureRecognizer>());
        assert_eq!(erased.downcast::<TapGestureRecognizer>(&app), Some(tap));
        assert_eq!(erased.downcast::<LongPressGestureRecognizer>(&app), None);
        assert_eq!(erased.debug_description(), "tap");
        assert_eq!(long_press.as_recognizer().debug_description(), "long press");
        assert!(format!("{erased:?}").contains("TapGestureRecognizer"));
    }

    #[test]
    fn an_erased_recognizer_adds_pointers_and_dispose_frees_the_slot() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, Some(Listener::new(move |_app| flag.set(true))));
        let erased = tap.as_recognizer();
        let down = PointerDownEvent {
            pointer: 1,
            ..PointerDownEvent::default()
        };
        assert!(erased.is_pointer_allowed(&app, &down));

        erased.add_pointer(&mut app, &down);
        let binding = GestureBinding::instance(&mut app);
        binding.gesture_arena(&app).close(&mut app, 1);
        app.drain_microtasks();
        binding.pointer_router(&app).route(
            &mut app,
            PointerEvent::Up(crate::events::PointerUpEvent {
                pointer: 1,
                ..crate::events::PointerUpEvent::default()
            }),
        );
        assert!(recognized.get());

        erased.dispose(&mut app);
        assert!(!app.contains(erased.id()));
    }

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

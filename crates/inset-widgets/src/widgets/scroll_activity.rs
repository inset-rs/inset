//! Flutter counterpart: `widgets/scroll_activity.dart`.

use std::rc::Rc;
use std::time::Duration;

use inset_animation::{AnimationBehavior, AnimationController, Curve};
use inset_embedder::PointerDeviceKind;
use inset_foundation::{
    App, Completer, CompleterFuture, Handle, HandleId, ListenableObject, Listener,
    PRECISION_ERROR_TOLERANCE,
};
use inset_gestures::{DragEndDetails, DragObject, DragStartDetails, DragUpdateDetails};
use inset_painting::{AxisDirection, axis_direction_is_reversed};
use inset_physics::Simulation;
use inset_scheduler::TickerProvider;

use crate::framework::{BuildContext, Notification};
use crate::widgets::scroll_metrics::ScrollMetrics;
use crate::widgets::scroll_notification::{
    OverscrollNotification, ScrollEndNotification, ScrollStartNotification,
    ScrollUpdateNotification,
};

/// A backend for a [`ScrollActivity`].
///
/// Used by implementors of [`ScrollActivity`] to manipulate the scroll view that
/// they are acting upon.
///
/// See also:
///
///  * [`ScrollActivity`], which uses this trait as its delegate.
///  * `ScrollPositionWithSingleContext`, the main implementation of this interface.
pub trait ScrollActivityDelegate: Sized + 'static {
    /// The direction in which the scroll view scrolls.
    fn axis_direction(self: Handle<Self>, app: &App) -> AxisDirection;

    /// Update the scroll position to the given pixel value.
    ///
    /// Returns the overscroll, if any. See `ScrollPosition::set_pixels` for more
    /// information.
    fn set_pixels(self: Handle<Self>, app: &mut App, pixels: f64) -> f64;

    /// Updates the scroll position by the given amount.
    ///
    /// Appropriate for when the user is directly manipulating the scroll
    /// position, for example by dragging the scroll view. Typically applies
    /// [`crate::ScrollPhysics::apply_physics_to_user_offset`] and other transformations that
    /// are appropriate for user-driving scrolling.
    fn apply_user_offset(self: Handle<Self>, app: &mut App, delta: f64);

    /// Terminate the current activity and start an idle activity.
    fn go_idle(self: Handle<Self>, app: &mut App);

    /// Terminate the current activity and start a ballistic activity with the
    /// given velocity.
    fn go_ballistic(self: Handle<Self>, app: &mut App, velocity: f64);

    /// This delegate as the erased [`AnyScrollActivityDelegate`] — what to pass where a Dart
    /// API takes a `ScrollActivityDelegate`.
    fn as_scroll_activity_delegate(self: Handle<Self>) -> AnyScrollActivityDelegate {
        AnyScrollActivityDelegate {
            id: self.id(),
            vtable: const { &ScrollActivityDelegateVTable::of::<Self>() },
        }
    }
}

/// The vtable of an erased [`AnyScrollActivityDelegate`].
struct ScrollActivityDelegateVTable {
    axis_direction: fn(&App, HandleId) -> AxisDirection,
    set_pixels: fn(&mut App, HandleId, f64) -> f64,
    apply_user_offset: fn(&mut App, HandleId, f64),
    go_idle: fn(&mut App, HandleId),
    go_ballistic: fn(&mut App, HandleId, f64),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl ScrollActivityDelegateVTable {
    /// The table for one concrete delegate type.
    const fn of<D: ScrollActivityDelegate>() -> ScrollActivityDelegateVTable {
        ScrollActivityDelegateVTable {
            axis_direction: |app, id| D::axis_direction(resolve(id), app),
            set_pixels: |app, id, pixels| D::set_pixels(resolve(id), app, pixels),
            apply_user_offset: |app, id, delta| D::apply_user_offset(resolve(id), app, delta),
            go_idle: |app, id| D::go_idle(resolve(id), app),
            go_ballistic: |app, id, velocity| D::go_ballistic(resolve(id), app, velocity),
        }
    }
}

/// Erased [`ScrollActivityDelegate`]: one identity and a static vtable.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyScrollActivityDelegate {
    id: HandleId,
    vtable: &'static ScrollActivityDelegateVTable,
}

impl AnyScrollActivityDelegate {
    /// See [`ScrollActivityDelegate::axis_direction`].
    pub fn axis_direction(self, app: &App) -> AxisDirection {
        (self.vtable.axis_direction)(app, self.id)
    }

    /// See [`ScrollActivityDelegate::set_pixels`].
    pub fn set_pixels(self, app: &mut App, pixels: f64) -> f64 {
        (self.vtable.set_pixels)(app, self.id, pixels)
    }

    /// See [`ScrollActivityDelegate::apply_user_offset`].
    pub fn apply_user_offset(self, app: &mut App, delta: f64) {
        (self.vtable.apply_user_offset)(app, self.id, delta);
    }

    /// See [`ScrollActivityDelegate::go_idle`].
    pub fn go_idle(self, app: &mut App) {
        (self.vtable.go_idle)(app, self.id);
    }

    /// See [`ScrollActivityDelegate::go_ballistic`].
    pub fn go_ballistic(self, app: &mut App, velocity: f64) {
        (self.vtable.go_ballistic)(app, self.id, velocity);
    }
}

impl PartialEq for AnyScrollActivityDelegate {
    fn eq(&self, other: &AnyScrollActivityDelegate) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyScrollActivityDelegate {}

/// The fields of Dart's `ScrollActivity` base class, held under the field `scroll_activity`.
pub struct ScrollActivityData {
    delegate: AnyScrollActivityDelegate,
    is_disposed: bool,
}

impl ScrollActivityData {
    /// The bag of an activity that will actuate `delegate`.
    pub fn new(delegate: AnyScrollActivityDelegate) -> ScrollActivityData {
        ScrollActivityData {
            delegate,
            is_disposed: false,
        }
    }
}

/// The accessors [`ScrollActivity`] asks for, for a struct whose bag is the field
/// `scroll_activity`.
#[macro_export]
macro_rules! scroll_activity_accessors {
    () => {
        fn scroll_activity_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::ScrollActivityData {
            &app.get(self).scroll_activity
        }

        fn scroll_activity_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::ScrollActivityData {
            &mut app.get_mut(self).scroll_activity
        }
    };
}

/// Base trait for scrolling activities like dragging and flinging.
///
/// The shared bodies live on [`ScrollActivityBase`], which an override calls where Dart
/// writes `super.…`.
///
/// See also:
///
///  * `ScrollPosition`, which uses [`ScrollActivity`] objects to manage the
///    `ScrollPosition` of a `Scrollable`.
pub trait ScrollActivity: Sized + 'static {
    /// Dart's `ScrollActivity` fields, held under the field `scroll_activity`
    /// ([`scroll_activity_accessors!`](crate::scroll_activity_accessors)).
    fn scroll_activity_data(self: Handle<Self>, app: &App) -> &ScrollActivityData;

    /// See [`scroll_activity_data`](Self::scroll_activity_data).
    fn scroll_activity_data_mut(self: Handle<Self>, app: &mut App) -> &mut ScrollActivityData;

    /// The delegate that this activity will use to actuate the scroll view.
    fn delegate(self: Handle<Self>, app: &App) -> AnyScrollActivityDelegate {
        self.scroll_activity_data(app).delegate
    }

    /// Whether [`dispose`](Self::dispose) has run; Dart's `_isDisposed`.
    fn is_disposed(self: Handle<Self>, app: &App) -> bool {
        self.scroll_activity_data(app).is_disposed
    }

    /// Updates the activity's link to the [`ScrollActivityDelegate`].
    ///
    /// This should only be called when an activity is being moved from a defunct
    /// (or about-to-be defunct) [`ScrollActivityDelegate`] object to a new one.
    fn update_delegate(self: Handle<Self>, app: &mut App, value: AnyScrollActivityDelegate) {
        debug_assert!(self.delegate(app) != value);
        self.scroll_activity_data_mut(app).delegate = value;
    }

    /// Called by the [`ScrollActivityDelegate`] when it has changed type (for
    /// example, when changing from an Android-style scroll position to an
    /// iOS-style scroll position). If this activity can differ between the two
    /// modes, then it should tell the position to restart that activity
    /// appropriately.
    ///
    /// For example, [`BallisticScrollActivity`]'s implementation calls
    /// [`ScrollActivityDelegate::go_ballistic`].
    fn reset_activity(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Dispatch a [`ScrollStartNotification`] with the given metrics.
    fn dispatch_scroll_start_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: Option<BuildContext>,
    ) {
        ScrollActivityBase::dispatch_scroll_start_notification(app, metrics, context);
    }

    /// Dispatch a [`ScrollUpdateNotification`] with the given metrics and scroll delta.
    fn dispatch_scroll_update_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        scroll_delta: f64,
    ) {
        ScrollActivityBase::dispatch_scroll_update_notification(
            app,
            metrics,
            context,
            scroll_delta,
        );
    }

    /// Dispatch an [`OverscrollNotification`] with the given metrics and overscroll.
    fn dispatch_overscroll_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        ScrollActivityBase::dispatch_overscroll_notification(app, metrics, context, overscroll);
    }

    /// Dispatch a [`ScrollEndNotification`] with the given metrics and overscroll.
    fn dispatch_scroll_end_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
    ) {
        ScrollActivityBase::dispatch_scroll_end_notification(app, metrics, context);
    }

    /// Called when the scroll view that is performing this activity changes its metrics.
    fn apply_new_dimensions(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Whether the scroll view should ignore pointer events while performing this
    /// activity.
    ///
    /// See also:
    ///
    ///  * [`is_scrolling`](Self::is_scrolling), which describes whether the activity is
    ///    considered to represent user interaction or not.
    fn should_ignore_pointer(self: Handle<Self>, app: &App) -> bool;

    /// Whether performing this activity constitutes scrolling.
    ///
    /// Used, for example, to determine whether the user scroll
    /// direction (see `ScrollPosition::user_scroll_direction`) is
    /// [`inset_rendering::ScrollDirection::Idle`].
    ///
    /// See also:
    ///
    ///  * [`should_ignore_pointer`](Self::should_ignore_pointer), which controls whether
    ///    pointer events are allowed while the activity is live.
    ///  * [`crate::UserScrollNotification`], which exposes this status.
    fn is_scrolling(self: Handle<Self>, app: &App) -> bool;

    /// If applicable, the velocity at which the scroll offset is currently
    /// independently changing (i.e. without external stimuli such as a dragging
    /// gestures) in logical pixels per second for this activity.
    fn velocity(self: Handle<Self>, app: &mut App) -> f64;

    /// Called when the scroll view stops performing this activity.
    fn dispose(self: Handle<Self>, app: &mut App) {
        ScrollActivityBase::dispose(self, app);
    }

    /// Dart's `toString`, which reads the arena and so cannot be [`std::fmt::Debug`].
    fn describe(self: Handle<Self>, app: &App) -> String {
        let _ = app;
        describe_identity::<Self>(self.id())
    }

    /// Dart's `activity is ScrollHoldController`; `None` — the default — for every activity
    /// but [`HoldScrollActivity`].
    fn as_hold_controller(self: Handle<Self>) -> Option<Rc<dyn ScrollHoldController>> {
        let _ = self;
        None
    }

    /// This activity as the erased [`AnyScrollActivity`] — what to pass where a Dart API takes
    /// a `ScrollActivity`.
    fn as_activity(self: Handle<Self>) -> AnyScrollActivity {
        AnyScrollActivity {
            id: self.id(),
            vtable: const { &ScrollActivityVTable::of::<Self>() },
        }
    }
}

/// Dart's `describeIdentity`: the runtime type and a short identity hash.
fn describe_identity<T: ?Sized>(id: HandleId) -> String {
    let name = std::any::type_name::<T>();
    let short = name.rsplit("::").next().unwrap_or(name);
    format!("{short}#{id:?}")
}

/// The shared bodies of Dart's `ScrollActivity`: call one where Dart writes `super.…`.
pub struct ScrollActivityBase;

impl ScrollActivityBase {
    /// See [`ScrollActivity::dispatch_scroll_start_notification`].
    pub fn dispatch_scroll_start_notification(
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: Option<BuildContext>,
    ) {
        ScrollStartNotification::new(metrics, context).dispatch(app, context);
    }

    /// See [`ScrollActivity::dispatch_scroll_update_notification`].
    pub fn dispatch_scroll_update_notification(
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        scroll_delta: f64,
    ) {
        ScrollUpdateNotification::new(metrics, context)
            .scroll_delta(scroll_delta)
            .dispatch(app, Some(context));
    }

    /// See [`ScrollActivity::dispatch_overscroll_notification`].
    pub fn dispatch_overscroll_notification(
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        OverscrollNotification::new(metrics, context, overscroll).dispatch(app, Some(context));
    }

    /// See [`ScrollActivity::dispatch_scroll_end_notification`].
    pub fn dispatch_scroll_end_notification(
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
    ) {
        ScrollEndNotification::new(metrics, context).dispatch(app, Some(context));
    }

    /// See [`ScrollActivity::dispose`].
    pub fn dispose<A: ScrollActivity>(this: Handle<A>, app: &mut App) {
        this.scroll_activity_data_mut(app).is_disposed = true;
    }
}

/// The dispatch signature of [`ScrollActivity::dispatch_scroll_update_notification`].
type DispatchDeltaFn = fn(&mut App, HandleId, Rc<dyn ScrollMetrics>, BuildContext, f64);

/// The vtable of an erased [`AnyScrollActivity`]: one `&'static` table per concrete
/// [`ScrollActivity`] type.
struct ScrollActivityVTable {
    delegate: fn(&App, HandleId) -> AnyScrollActivityDelegate,
    is_disposed: fn(&App, HandleId) -> bool,
    update_delegate: fn(&mut App, HandleId, AnyScrollActivityDelegate),
    reset_activity: fn(&mut App, HandleId),
    dispatch_scroll_start_notification:
        fn(&mut App, HandleId, Rc<dyn ScrollMetrics>, Option<BuildContext>),
    dispatch_scroll_update_notification: DispatchDeltaFn,
    dispatch_overscroll_notification: DispatchDeltaFn,
    dispatch_scroll_end_notification: fn(&mut App, HandleId, Rc<dyn ScrollMetrics>, BuildContext),
    apply_new_dimensions: fn(&mut App, HandleId),
    should_ignore_pointer: fn(&App, HandleId) -> bool,
    is_scrolling: fn(&App, HandleId) -> bool,
    velocity: fn(&mut App, HandleId) -> f64,
    dispose: fn(&mut App, HandleId),
    describe: fn(&App, HandleId) -> String,
    as_hold_controller: fn(HandleId) -> Option<Rc<dyn ScrollHoldController>>,
}

impl ScrollActivityVTable {
    /// The table for one concrete activity type.
    const fn of<A: ScrollActivity>() -> ScrollActivityVTable {
        ScrollActivityVTable {
            delegate: |app, id| A::delegate(resolve(id), app),
            is_disposed: |app, id| A::is_disposed(resolve(id), app),
            update_delegate: |app, id, value| A::update_delegate(resolve(id), app, value),
            reset_activity: |app, id| A::reset_activity(resolve(id), app),
            dispatch_scroll_start_notification: |app, id, metrics, context| {
                A::dispatch_scroll_start_notification(resolve(id), app, metrics, context);
            },
            dispatch_scroll_update_notification: |app, id, metrics, context, delta| {
                A::dispatch_scroll_update_notification(resolve(id), app, metrics, context, delta);
            },
            dispatch_overscroll_notification: |app, id, metrics, context, overscroll| {
                A::dispatch_overscroll_notification(resolve(id), app, metrics, context, overscroll);
            },
            dispatch_scroll_end_notification: |app, id, metrics, context| {
                A::dispatch_scroll_end_notification(resolve(id), app, metrics, context);
            },
            apply_new_dimensions: |app, id| A::apply_new_dimensions(resolve(id), app),
            should_ignore_pointer: |app, id| A::should_ignore_pointer(resolve(id), app),
            is_scrolling: |app, id| A::is_scrolling(resolve(id), app),
            velocity: |app, id| A::velocity(resolve(id), app),
            dispose: |app, id| A::dispose(resolve(id), app),
            describe: |app, id| A::describe(resolve(id), app),
            as_hold_controller: |id| A::as_hold_controller(resolve(id)),
        }
    }
}

/// Erased [`ScrollActivity`]: what a field or parameter Dart types as `ScrollActivity`
/// becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyScrollActivity {
    id: HandleId,
    vtable: &'static ScrollActivityVTable,
}

impl AnyScrollActivity {
    /// See [`ScrollActivity::delegate`].
    pub fn delegate(self, app: &App) -> AnyScrollActivityDelegate {
        (self.vtable.delegate)(app, self.id)
    }

    /// See [`ScrollActivity::is_disposed`].
    pub fn is_disposed(self, app: &App) -> bool {
        (self.vtable.is_disposed)(app, self.id)
    }

    /// See [`ScrollActivity::update_delegate`].
    pub fn update_delegate(self, app: &mut App, value: AnyScrollActivityDelegate) {
        (self.vtable.update_delegate)(app, self.id, value);
    }

    /// See [`ScrollActivity::reset_activity`].
    pub fn reset_activity(self, app: &mut App) {
        (self.vtable.reset_activity)(app, self.id);
    }

    /// See [`ScrollActivity::dispatch_scroll_start_notification`].
    pub fn dispatch_scroll_start_notification(
        self,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: Option<BuildContext>,
    ) {
        (self.vtable.dispatch_scroll_start_notification)(app, self.id, metrics, context);
    }

    /// See [`ScrollActivity::dispatch_scroll_update_notification`].
    pub fn dispatch_scroll_update_notification(
        self,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        scroll_delta: f64,
    ) {
        (self.vtable.dispatch_scroll_update_notification)(
            app,
            self.id,
            metrics,
            context,
            scroll_delta,
        );
    }

    /// See [`ScrollActivity::dispatch_overscroll_notification`].
    pub fn dispatch_overscroll_notification(
        self,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        (self.vtable.dispatch_overscroll_notification)(app, self.id, metrics, context, overscroll);
    }

    /// See [`ScrollActivity::dispatch_scroll_end_notification`].
    pub fn dispatch_scroll_end_notification(
        self,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
    ) {
        (self.vtable.dispatch_scroll_end_notification)(app, self.id, metrics, context);
    }

    /// See [`ScrollActivity::apply_new_dimensions`].
    pub fn apply_new_dimensions(self, app: &mut App) {
        (self.vtable.apply_new_dimensions)(app, self.id);
    }

    /// See [`ScrollActivity::should_ignore_pointer`].
    pub fn should_ignore_pointer(self, app: &App) -> bool {
        (self.vtable.should_ignore_pointer)(app, self.id)
    }

    /// See [`ScrollActivity::is_scrolling`].
    pub fn is_scrolling(self, app: &App) -> bool {
        (self.vtable.is_scrolling)(app, self.id)
    }

    /// See [`ScrollActivity::velocity`].
    pub fn velocity(self, app: &mut App) -> f64 {
        (self.vtable.velocity)(app, self.id)
    }

    /// See [`ScrollActivity::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// See [`ScrollActivity::describe`].
    pub fn describe(self, app: &App) -> String {
        (self.vtable.describe)(app, self.id)
    }

    /// Dart's `activity is ScrollHoldController`.
    pub fn as_hold_controller(self) -> Option<Rc<dyn ScrollHoldController>> {
        (self.vtable.as_hold_controller)(self.id)
    }
}

impl PartialEq for AnyScrollActivity {
    fn eq(&self, other: &AnyScrollActivity) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyScrollActivity {}

/// A scroll activity that does nothing.
///
/// When a scroll view is not scrolling, it is performing the idle activity.
///
/// If the `Scrollable` changes dimensions, this activity triggers a ballistic
/// activity to restore the view.
pub struct IdleScrollActivity {
    scroll_activity: ScrollActivityData,
}

impl IdleScrollActivity {
    /// Creates a scroll activity that does nothing.
    pub fn new(app: &mut App, delegate: AnyScrollActivityDelegate) -> Handle<IdleScrollActivity> {
        app.create(IdleScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
        })
    }
}

impl ScrollActivity for IdleScrollActivity {
    crate::scroll_activity_accessors!();

    fn apply_new_dimensions(self: Handle<Self>, app: &mut App) {
        self.delegate(app).go_ballistic(app, 0.0);
    }

    fn should_ignore_pointer(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn is_scrolling(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn velocity(self: Handle<Self>, _app: &mut App) -> f64 {
        0.0
    }
}

/// Interface for holding a `Scrollable` stationary.
///
/// An object that implements this interface is returned by
/// `ScrollPosition::hold`. It holds the scrollable stationary until an activity
/// is started or the [`cancel`](Self::cancel) method is called.
///
/// What Dart types as `ScrollHoldController` becomes an `Rc<dyn ScrollHoldController>`; an
/// arena object implements [`ScrollHoldControllerObject`], and `Rc::new(handle)` is its erased
/// form.
pub trait ScrollHoldController {
    /// Release the `Scrollable`, potentially letting it go ballistic if
    /// necessary.
    fn cancel(&self, app: &mut App);
}

/// The object side of [`ScrollHoldController`]: an arena object that holds a scrollable.
/// Implementing it makes `Handle<Self>` a [`ScrollHoldController`].
pub trait ScrollHoldControllerObject: Sized + 'static {
    /// See [`ScrollHoldController::cancel`].
    fn cancel(self: Handle<Self>, app: &mut App);
}

impl<T: ScrollHoldControllerObject> ScrollHoldController for Handle<T> {
    fn cancel(&self, app: &mut App) {
        T::cancel(*self, app);
    }
}

/// A scroll activity that does nothing but can be released to resume
/// normal idle behavior.
///
/// This is used while the user is touching the `Scrollable` but before the
/// touch has become a [`inset_gestures::Drag`].
///
/// For the purposes of `ScrollNotification`s, this activity does not constitute
/// scrolling, and does not prevent the user from interacting with the contents
/// of the `Scrollable` (unlike when a drag has begun or there is a scroll
/// animation underway).
pub struct HoldScrollActivity {
    scroll_activity: ScrollActivityData,
    /// Called when [`ScrollActivity::dispose`] is called.
    pub on_hold_canceled: Option<Listener>,
}

impl HoldScrollActivity {
    /// Creates a scroll activity that does nothing.
    pub fn new(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        on_hold_canceled: Option<Listener>,
    ) -> Handle<HoldScrollActivity> {
        app.create(HoldScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
            on_hold_canceled,
        })
    }
}

impl ScrollActivity for HoldScrollActivity {
    crate::scroll_activity_accessors!();

    fn should_ignore_pointer(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn is_scrolling(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn velocity(self: Handle<Self>, _app: &mut App) -> f64 {
        0.0
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(on_hold_canceled) = app.get(self).on_hold_canceled.clone() {
            on_hold_canceled.call(app);
        }
        ScrollActivityBase::dispose(self, app);
    }

    fn as_hold_controller(self: Handle<Self>) -> Option<Rc<dyn ScrollHoldController>> {
        Some(Rc::new(self))
    }
}

impl ScrollHoldControllerObject for HoldScrollActivity {
    fn cancel(self: Handle<Self>, app: &mut App) {
        self.delegate(app).go_ballistic(app, 0.0);
    }
}

/// The most recently observed drag details; Dart's `dynamic ScrollDragController.lastDetails`.
#[derive(Clone)]
pub enum LastDragDetails {
    /// The drag has started and has not moved yet.
    Start(DragStartDetails),
    /// The drag has moved.
    Update(DragUpdateDetails),
    /// The drag has ended.
    End(DragEndDetails),
}

/// Scrolls a scroll view as the user drags their finger across the screen.
///
/// See also:
///
///  * [`DragScrollActivity`], which is the activity the scroll view performs
///    while a drag is underway.
pub struct ScrollDragController {
    delegate: AnyScrollActivityDelegate,

    /// Called when [`dispose`](Self::dispose) is called.
    pub on_drag_canceled: Option<Listener>,

    /// Velocity that was present from a previous [`ScrollActivity`] when this drag
    /// began.
    pub carried_velocity: Option<f64>,

    /// Amount of pixels in either direction the drag has to move by to start
    /// scroll movement again after each time scrolling came to a stop.
    pub motion_start_distance_threshold: Option<f64>,

    last_non_stationary_timestamp: Option<Duration>,
    retain_momentum: bool,
    /// `None` if already in motion or has no
    /// [`motion_start_distance_threshold`](Self::motion_start_distance_threshold).
    offset_since_last_stop: Option<f64>,
    kind: Option<PointerDeviceKind>,
    last_details: Option<LastDragDetails>,
}

impl ScrollDragController {
    /// Maximum amount of time interval the drag can have consecutive stationary
    /// pointer update events before losing the momentum carried from a previous
    /// scroll activity.
    pub const MOMENTUM_RETAIN_STATIONARY_DURATION_THRESHOLD: Duration = Duration::from_millis(20);

    /// The minimum amount of velocity needed to apply the
    /// [`carried_velocity`](Self::carried_velocity) at the end of a drag. Expressed as a
    /// factor. For example with a [`carried_velocity`](Self::carried_velocity) of 2000, we
    /// will need a velocity of at least 1000 to apply the
    /// [`carried_velocity`](Self::carried_velocity) as well. If the velocity does not meet
    /// the threshold, the [`carried_velocity`](Self::carried_velocity) is lost. Decided by
    /// fair eyeballing with the scroll_overlay platform test.
    pub const MOMENTUM_RETAIN_VELOCITY_THRESHOLD_FACTOR: f64 = 0.5;

    /// Maximum amount of time interval the drag can have consecutive stationary
    /// pointer update events before needing to break the
    /// [`motion_start_distance_threshold`](Self::motion_start_distance_threshold) to start
    /// motion again.
    pub const MOTION_STOPPED_DURATION_THRESHOLD: Duration = Duration::from_millis(50);

    /// The drag distance past which, a
    /// [`motion_start_distance_threshold`](Self::motion_start_distance_threshold) breaking
    /// drag is considered a deliberate fling.
    const BIG_THRESHOLD_BREAK_DISTANCE: f64 = 24.0;

    /// Creates an object that scrolls a scroll view as the user drags their
    /// finger across the screen.
    pub fn new(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        details: DragStartDetails,
        on_drag_canceled: Option<Listener>,
        carried_velocity: Option<f64>,
        motion_start_distance_threshold: Option<f64>,
    ) -> Handle<ScrollDragController> {
        debug_assert!(
            motion_start_distance_threshold.is_none_or(|threshold| threshold > 0.0),
            "motionStartDistanceThreshold must be a positive number or null"
        );
        app.create(ScrollDragController {
            delegate,
            on_drag_canceled,
            carried_velocity,
            motion_start_distance_threshold,
            last_non_stationary_timestamp: details.source_time_stamp,
            retain_momentum: carried_velocity.is_some_and(|velocity| velocity != 0.0),
            offset_since_last_stop: motion_start_distance_threshold.map(|_| 0.0),
            kind: details.kind,
            last_details: Some(LastDragDetails::Start(details)),
        })
    }

    /// The object that will actuate the scroll view as the user drags.
    pub fn delegate(self: Handle<Self>, app: &App) -> AnyScrollActivityDelegate {
        app.get(self).delegate
    }

    /// The type of input device driving the drag; Dart's `_kind`.
    pub fn kind(self: Handle<Self>, app: &App) -> Option<PointerDeviceKind> {
        app.get(self).kind
    }

    /// The most recently observed [`DragStartDetails`], [`DragUpdateDetails`], or
    /// [`DragEndDetails`] object.
    pub fn last_details(self: Handle<Self>, app: &App) -> Option<&LastDragDetails> {
        app.get(self).last_details.as_ref()
    }

    /// Updates the controller's link to the [`ScrollActivityDelegate`].
    ///
    /// This should only be called when a controller is being moved from a defunct
    /// (or about-to-be defunct) [`ScrollActivityDelegate`] object to a new one.
    pub fn update_delegate(self: Handle<Self>, app: &mut App, value: AnyScrollActivityDelegate) {
        debug_assert!(app.get(self).delegate != value);
        app.get_mut(self).delegate = value;
    }

    /// Called by the delegate when it is no longer sending events to this object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).last_details = None;
        if let Some(on_drag_canceled) = app.get(self).on_drag_canceled.clone() {
            on_drag_canceled.call(app);
        }
    }

    /// Dart's `toString`, which reads the arena and so cannot be [`std::fmt::Debug`].
    pub fn describe(self: Handle<Self>) -> String {
        describe_identity::<ScrollDragController>(self.id())
    }

    fn reversed(self: Handle<Self>, app: &App) -> bool {
        axis_direction_is_reversed(self.delegate(app).axis_direction(app))
    }

    /// Determines whether to lose the existing incoming velocity when starting
    /// the drag.
    fn maybe_lose_momentum(
        self: Handle<Self>,
        app: &mut App,
        offset: f64,
        timestamp: Option<Duration>,
    ) {
        let controller = app.get(self);
        if controller.retain_momentum
            && offset == 0.0
            && timestamp.is_none_or(|timestamp| {
                // If drag event has no timestamp, we lose momentum.
                timestamp.saturating_sub(
                    controller
                        .last_non_stationary_timestamp
                        .expect("a retained momentum carries a timestamp"),
                ) > Self::MOMENTUM_RETAIN_STATIONARY_DURATION_THRESHOLD
            })
        {
            // If pointer is stationary for too long, we lose momentum.
            app.get_mut(self).retain_momentum = false;
        }
    }

    /// If a motion start threshold exists, determine whether the threshold needs
    /// to be broken to scroll. Also possibly apply an offset adjustment when
    /// threshold is first broken.
    ///
    /// Returns `0.0` when stationary or within threshold. Returns `offset`
    /// transparently when already in motion.
    fn adjust_for_scroll_start_threshold(
        self: Handle<Self>,
        app: &mut App,
        offset: f64,
        timestamp: Option<Duration>,
    ) -> f64 {
        let Some(timestamp) = timestamp else {
            // If we can't track time, we can't apply thresholds.
            // May be null for proxied drags like via accessibility.
            return offset;
        };
        let controller = app.get(self);
        let threshold = controller.motion_start_distance_threshold;
        if offset == 0.0 {
            if threshold.is_some()
                && controller.offset_since_last_stop.is_none()
                && timestamp.saturating_sub(
                    controller
                        .last_non_stationary_timestamp
                        .expect("a stationary update follows a timestamped one"),
                ) > Self::MOTION_STOPPED_DURATION_THRESHOLD
            {
                // Enforce a new threshold.
                app.get_mut(self).offset_since_last_stop = Some(0.0);
            }
            // Not moving can't break threshold.
            0.0
        } else {
            let Some(offset_since_last_stop) = controller.offset_since_last_stop else {
                // Already in motion or no threshold behavior configured such as for
                // Android. Allow transparent offset transmission.
                return offset;
            };
            let offset_since_last_stop = offset_since_last_stop + offset;
            app.get_mut(self).offset_since_last_stop = Some(offset_since_last_stop);
            let threshold = threshold.expect("an offset since the last stop implies a threshold");
            if offset_since_last_stop.abs() > threshold {
                // Threshold broken.
                app.get_mut(self).offset_since_last_stop = None;
                if offset.abs() > Self::BIG_THRESHOLD_BREAK_DISTANCE {
                    // This is heuristically a very deliberate fling. Leave the motion
                    // unaffected.
                    offset
                } else {
                    // This is a normal speed threshold break.
                    // Ease into the motion when the threshold is initially broken
                    // to avoid a visible jump.
                    (threshold / 3.0).min(offset.abs()) * offset.signum()
                }
            } else {
                0.0
            }
        }
    }
}

impl DragObject for ScrollDragController {
    fn update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        debug_assert!(details.primary_delta.is_some());
        let mut offset = details.primary_delta.expect("a primary delta");
        let source_time_stamp = details.source_time_stamp;
        app.get_mut(self).last_details = Some(LastDragDetails::Update(details));
        if offset != 0.0 {
            app.get_mut(self).last_non_stationary_timestamp = source_time_stamp;
        }
        // By default, iOS platforms carries momentum and has a start threshold
        // (configured in `BouncingScrollPhysics`). The 2 operations below are
        // no-ops on Android.
        self.maybe_lose_momentum(app, offset, source_time_stamp);
        offset = self.adjust_for_scroll_start_threshold(app, offset, source_time_stamp);
        if offset == 0.0 {
            return;
        }
        if self.reversed(app) {
            offset = -offset;
        }
        self.delegate(app).apply_user_offset(app, offset);
    }

    fn end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        debug_assert!(details.primary_velocity.is_some());
        // We negate the velocity here because if the touch is moving downwards,
        // the scroll has to move upwards. It's the same reason that `update`
        // above negates the delta before applying it to the scroll offset.
        let mut velocity = -details.primary_velocity.expect("a primary velocity");
        if self.reversed(app) {
            velocity = -velocity;
        }
        app.get_mut(self).last_details = Some(LastDragDetails::End(details));

        let controller = app.get(self);
        if controller.retain_momentum {
            let carried_velocity = controller
                .carried_velocity
                .expect("a retained momentum carries a velocity");
            // Build momentum only if dragging in the same direction.
            let is_flinging_in_same_direction = velocity.signum() == carried_velocity.signum();
            // Build momentum only if the velocity of the last drag was not
            // substantially lower than the carried momentum.
            let is_velocity_not_substantially_less_than_carried_momentum = velocity.abs()
                > carried_velocity.abs() * Self::MOMENTUM_RETAIN_VELOCITY_THRESHOLD_FACTOR;
            if is_flinging_in_same_direction
                && is_velocity_not_substantially_less_than_carried_momentum
            {
                velocity += carried_velocity;
            }
        }
        self.delegate(app).go_ballistic(app, velocity);
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        self.delegate(app).go_ballistic(app, 0.0);
    }
}

/// The activity a scroll view performs when the user drags their finger
/// across the screen.
///
/// See also:
///
///  * [`ScrollDragController`], which listens to the [`inset_gestures::Drag`] and actually scrolls
///    the scroll view.
pub struct DragScrollActivity {
    scroll_activity: ScrollActivityData,
    controller: Option<Handle<ScrollDragController>>,
}

impl DragScrollActivity {
    /// Creates an activity for when the user drags their finger across the
    /// screen.
    pub fn new(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        controller: Handle<ScrollDragController>,
    ) -> Handle<DragScrollActivity> {
        app.create(DragScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
            controller: Some(controller),
        })
    }

    fn controller(self: Handle<Self>, app: &App) -> Handle<ScrollDragController> {
        app.get(self)
            .controller
            .expect("the drag activity outlives its controller")
    }
}

impl ScrollActivity for DragScrollActivity {
    crate::scroll_activity_accessors!();

    fn dispatch_scroll_start_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: Option<BuildContext>,
    ) {
        let last_details = self.controller(app).last_details(app).cloned();
        let Some(LastDragDetails::Start(drag_details)) = last_details else {
            unreachable!("a drag activity starts with its start details");
        };
        ScrollStartNotification::new(metrics, context)
            .drag_details(drag_details)
            .dispatch(app, context);
    }

    fn dispatch_scroll_update_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        scroll_delta: f64,
    ) {
        let last_details = self.controller(app).last_details(app).cloned();
        let Some(LastDragDetails::Update(drag_details)) = last_details else {
            unreachable!("a drag update carries its update details");
        };
        ScrollUpdateNotification::new(metrics, context)
            .scroll_delta(scroll_delta)
            .drag_details(drag_details)
            .dispatch(app, Some(context));
    }

    fn dispatch_overscroll_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        let last_details = self.controller(app).last_details(app).cloned();
        let Some(LastDragDetails::Update(drag_details)) = last_details else {
            unreachable!("an overscroll follows an update");
        };
        OverscrollNotification::new(metrics, context, overscroll)
            .drag_details(drag_details)
            .dispatch(app, Some(context));
    }

    fn dispatch_scroll_end_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
    ) {
        // We might not have DragEndDetails yet if we're being called from `begin_activity`.
        let last_details = self.controller(app).last_details(app).cloned();
        let mut notification = ScrollEndNotification::new(metrics, context);
        if let Some(LastDragDetails::End(drag_details)) = last_details {
            notification = notification.drag_details(drag_details);
        }
        notification.dispatch(app, Some(context));
    }

    fn should_ignore_pointer(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .controller
            .and_then(|controller| controller.kind(app))
            != Some(PointerDeviceKind::Trackpad)
    }

    fn is_scrolling(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    /// [`DragScrollActivity`] is not independently changing velocity yet
    /// until the drag is ended.
    fn velocity(self: Handle<Self>, _app: &mut App) -> f64 {
        0.0
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).controller = None;
        ScrollActivityBase::dispose(self, app);
    }

    fn describe(self: Handle<Self>, app: &App) -> String {
        let controller = app.get(self).controller;
        let described =
            controller.map_or_else(|| "null".to_string(), ScrollDragController::describe);
        format!(
            "{}({described})",
            describe_identity::<DragScrollActivity>(self.id())
        )
    }
}

/// The activity a scroll view performs after being set into motion.
///
/// For example, a [`BallisticScrollActivity`] is used when the user
/// lifts their finger off the screen after a [`DragScrollActivity`],
/// to continue the scrolling motion starting from the current velocity.
///
/// [`BallisticScrollActivity`] is also used to restore a scroll view to a valid
/// scroll offset when the geometry of the scroll view changes. In these
/// situations, the [`Simulation`] typically starts with a zero velocity.
///
/// The scrolling will be driven by the given [`Simulation`]. If a
/// [`BallisticScrollActivity`] is in progress when the scroll metrics change,
/// then the activity will be replaced with a new ballistic activity starting
/// from the current velocity (see [`crate::ScrollPhysics::create_ballistic_simulation`]).
/// To ensure the user perceives smooth motion across such a change,
/// the simulation should typically be the result
/// of [`crate::ScrollPhysics::create_ballistic_simulation`]
/// for the scroll physics of the scroll view.
///
/// See also:
///
///  * [`DrivenScrollActivity`], which drives a scroll view through
///    a given animation, without resetting to a ballistic simulation
///    when scroll metrics change.
pub struct BallisticScrollActivity {
    scroll_activity: ScrollActivityData,
    controller: Handle<AnimationController>,
    should_ignore_pointer: bool,
}

impl BallisticScrollActivity {
    /// Creates an activity that sets into motion a scroll view.
    ///
    /// The simulation should typically be the result
    /// of [`crate::ScrollPhysics::create_ballistic_simulation`]
    /// for the scroll physics of the scroll view.
    pub fn new(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        simulation: Box<dyn Simulation>,
        vsync: impl TickerProvider,
        should_ignore_pointer: bool,
    ) -> Handle<BallisticScrollActivity> {
        let controller = AnimationController::create_unbounded(
            app,
            0.0,
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        let this = app.create(BallisticScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
            controller,
            should_ignore_pointer,
        });
        controller.add_listener(app, Listener::handle_method(this, Self::tick));
        let done = controller.animate_with(app, simulation);
        // Won't trigger if we dispose the controller before it completes.
        done.when_complete(app, Listener::handle_method(this, Self::end));
        this
    }

    fn tick(self: Handle<Self>, app: &mut App) {
        let value = app.get(self).controller.value(app);
        if !self.apply_move_to(app, value) {
            self.delegate(app).go_idle(app);
        }
    }

    /// Move the position to the given location.
    ///
    /// If the new position was fully applied, returns true. If there was any
    /// overflow, returns false.
    ///
    /// The default implementation calls [`ScrollActivityDelegate::set_pixels`]
    /// and returns true if the overflow was zero.
    pub fn apply_move_to(self: Handle<Self>, app: &mut App, value: f64) -> bool {
        self.delegate(app).set_pixels(app, value).abs() < PRECISION_ERROR_TOLERANCE
    }

    fn end(self: Handle<Self>, app: &mut App) {
        // Check if the activity was disposed before going ballistic because `end` might be
        // called if the controller is disposed just after completion.
        if !self.is_disposed(app) {
            self.delegate(app).go_ballistic(app, 0.0);
        }
    }
}

impl ScrollActivity for BallisticScrollActivity {
    crate::scroll_activity_accessors!();

    fn reset_activity(self: Handle<Self>, app: &mut App) {
        let velocity = self.velocity(app);
        self.delegate(app).go_ballistic(app, velocity);
    }

    fn apply_new_dimensions(self: Handle<Self>, app: &mut App) {
        let velocity = self.velocity(app);
        self.delegate(app).go_ballistic(app, velocity);
    }

    fn dispatch_overscroll_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        let velocity = self.velocity(app);
        OverscrollNotification::new(metrics, context, overscroll)
            .velocity(velocity)
            .dispatch(app, Some(context));
    }

    fn should_ignore_pointer(self: Handle<Self>, app: &App) -> bool {
        app.get(self).should_ignore_pointer
    }

    fn is_scrolling(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn velocity(self: Handle<Self>, app: &mut App) -> f64 {
        app.get(self).controller.velocity(app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).controller.dispose(app);
        ScrollActivityBase::dispose(self, app);
    }

    fn describe(self: Handle<Self>, app: &App) -> String {
        format!(
            "{}({:?})",
            describe_identity::<BallisticScrollActivity>(self.id()),
            app.get(self).controller.id()
        )
    }
}

/// An activity that drives a scroll view through a given animation.
///
/// For example, a [`DrivenScrollActivity`] is used to implement
/// `ScrollController::animate_to`.
///
/// The scrolling will be driven by the given animation parameters
/// or the given [`Simulation`].
///
/// Unlike a [`BallisticScrollActivity`], if a [`DrivenScrollActivity`] is
/// in progress when the scroll metrics change, the activity will continue
/// with its original animation.
///
/// See also:
///
///  * [`BallisticScrollActivity`], which sets into motion a scroll view.
pub struct DrivenScrollActivity {
    scroll_activity: ScrollActivityData,
    completer: Completer<()>,
    controller: Handle<AnimationController>,
}

impl DrivenScrollActivity {
    /// Creates an activity that drives a scroll view through an animation
    /// given by animation parameters.
    pub fn new(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        from: f64,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        vsync: impl TickerProvider,
    ) -> Handle<DrivenScrollActivity> {
        debug_assert!(duration > Duration::ZERO);
        let completer = Completer::new();
        let controller = AnimationController::create_unbounded(
            app,
            from,
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        let this = app.create(DrivenScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
            completer,
            controller,
        });
        controller.add_listener(app, Listener::handle_method(this, Self::tick));
        let done = controller.animate_to(app, to, Some(duration), curve);
        // Won't trigger if we dispose the controller before it completes.
        done.when_complete(app, Listener::handle_method(this, Self::end));
        this
    }

    /// Creates an activity that drives a scroll view through an animation
    /// given by a [`Simulation`].
    pub fn simulation(
        app: &mut App,
        delegate: AnyScrollActivityDelegate,
        simulation: Box<dyn Simulation>,
        vsync: impl TickerProvider,
    ) -> Handle<DrivenScrollActivity> {
        let completer = Completer::new();
        let controller = AnimationController::create_unbounded(
            app,
            0.0,
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        let this = app.create(DrivenScrollActivity {
            scroll_activity: ScrollActivityData::new(delegate),
            completer,
            controller,
        });
        controller.add_listener(app, Listener::handle_method(this, Self::tick));
        let done = controller.animate_with(app, simulation);
        // Won't trigger if we dispose the controller before it completes.
        done.when_complete(app, Listener::handle_method(this, Self::end));
        this
    }

    /// A future that completes when the activity stops.
    ///
    /// For example, this future will complete if the animation reaches the end
    /// or if the user interacts with the scroll view in way that causes the
    /// animation to stop before it reaches the end.
    pub fn done(self: Handle<Self>, app: &App) -> CompleterFuture<()> {
        app.get(self).completer.future()
    }

    fn tick(self: Handle<Self>, app: &mut App) {
        let value = app.get(self).controller.value(app);
        if !self.apply_move_to(app, value) {
            self.delegate(app).go_idle(app);
        }
    }

    /// Move the position to the given location.
    ///
    /// If the new position was fully applied, returns true. If there was any
    /// overflow, returns false.
    ///
    /// The default implementation calls [`ScrollActivityDelegate::set_pixels`]
    /// and returns true if the overflow was zero.
    pub fn apply_move_to(self: Handle<Self>, app: &mut App, value: f64) -> bool {
        self.delegate(app).set_pixels(app, value).abs() < PRECISION_ERROR_TOLERANCE
    }

    fn end(self: Handle<Self>, app: &mut App) {
        // Check if the activity was disposed before going ballistic because `end` might be
        // called if the controller is disposed just after completion.
        if !self.is_disposed(app) {
            let velocity = self.velocity(app);
            self.delegate(app).go_ballistic(app, velocity);
        }
    }
}

impl ScrollActivity for DrivenScrollActivity {
    crate::scroll_activity_accessors!();

    fn dispatch_overscroll_notification(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) {
        let velocity = self.velocity(app);
        OverscrollNotification::new(metrics, context, overscroll)
            .velocity(velocity)
            .dispatch(app, Some(context));
    }

    fn should_ignore_pointer(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn is_scrolling(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn velocity(self: Handle<Self>, app: &mut App) -> f64 {
        app.get(self).controller.velocity(app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let completer = app.get(self).completer.clone();
        completer.complete(app, ());
        app.get(self).controller.dispose(app);
        ScrollActivityBase::dispose(self, app);
    }

    fn describe(self: Handle<Self>, app: &App) -> String {
        format!(
            "{}({:?})",
            describe_identity::<DrivenScrollActivity>(self.id()),
            app.get(self).controller.id()
        )
    }
}

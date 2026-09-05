//! Flutter counterpart: `widgets/scroll_position_with_single_context.dart`.

use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Listener};
use reveal_gestures::{Drag, DragStartDetails};
use reveal_painting::AxisDirection;
use reveal_physics::near_equal;
use reveal_rendering::{ScrollDirection, ViewportOffset};

use crate::widgets::scroll_activity::{
    AnyScrollActivity, BallisticScrollActivity, DragScrollActivity, DrivenScrollActivity,
    HoldScrollActivity, IdleScrollActivity, ScrollActivity, ScrollActivityDelegate,
    ScrollDragController, ScrollHoldController,
};
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_position::{
    AnyScrollPosition, ScrollPosition, ScrollPositionBase, ScrollPositionData,
};

/// The fields of Dart's `ScrollPositionWithSingleContext`, which its subclasses carry.
pub struct ScrollPositionWithSingleContextData {
    /// Velocity from a previous activity temporarily held by [`ScrollPosition::hold`] to
    /// potentially transfer to a next activity.
    held_previous_velocity: f64,
    user_scroll_direction: ScrollDirection,
    current_drag: Option<Handle<ScrollDragController>>,
}

impl ScrollPositionWithSingleContextData {
    /// The bag of a position that has neither an activity nor a drag yet.
    pub fn new() -> ScrollPositionWithSingleContextData {
        ScrollPositionWithSingleContextData {
            held_previous_velocity: 0.0,
            user_scroll_direction: ScrollDirection::Idle,
            current_drag: None,
        }
    }
}

impl Default for ScrollPositionWithSingleContextData {
    fn default() -> ScrollPositionWithSingleContextData {
        ScrollPositionWithSingleContextData::new()
    }
}

/// The accessors [`ScrollPositionWithSingleContextLeaf`] asks for, for a struct whose bag is
/// the field `single_context`.
#[macro_export]
macro_rules! scroll_position_with_single_context_accessors {
    () => {
        fn single_context_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ScrollPositionWithSingleContextData {
            &app.get(self).single_context
        }

        fn single_context_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ScrollPositionWithSingleContextData {
            &mut app.get_mut(self).single_context
        }
    };
}

/// A [`ScrollPosition`] with the field Dart's `ScrollPositionWithSingleContext` declares and
/// the members it overrides.
///
/// The shared bodies are associated functions on [`ScrollPositionWithSingleContext`]; an
/// override calls one where Dart writes `super.…`.
///
/// A leaf writes [`scroll_position_with_single_context_viewport_offset_overrides!`](crate::scroll_position_with_single_context_viewport_offset_overrides),
/// [`scroll_position_with_single_context_overrides!`](crate::scroll_position_with_single_context_overrides)
/// and [`scroll_position_with_single_context_delegate_overrides!`](crate::scroll_position_with_single_context_delegate_overrides)
/// to route the `ViewportOffset`, `ScrollPosition` and `ScrollActivityDelegate` virtuals here.
pub trait ScrollPositionWithSingleContextLeaf: ScrollPosition + ScrollActivityDelegate {
    /// Dart's `ScrollPositionWithSingleContext` fields, held under the field `single_context`
    /// ([`scroll_position_with_single_context_accessors!`](crate::scroll_position_with_single_context_accessors)).
    fn single_context_data(self: Handle<Self>, app: &App) -> &ScrollPositionWithSingleContextData;

    /// See [`single_context_data`](Self::single_context_data).
    fn single_context_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ScrollPositionWithSingleContextData;

    /// Dart's `ScrollPositionWithSingleContext.axisDirection`.
    fn axis_direction(self: Handle<Self>, app: &App) -> AxisDirection {
        ScrollPositionWithSingleContext::axis_direction(self, app)
    }

    /// Dart's `ScrollPositionWithSingleContext.setPixels`.
    fn set_pixels(self: Handle<Self>, app: &mut App, new_pixels: f64) -> f64 {
        ScrollPositionWithSingleContext::set_pixels(self, app, new_pixels)
    }

    /// Dart's `ScrollPositionWithSingleContext.absorb`.
    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionWithSingleContext::absorb(self, app, other);
    }

    /// Dart's `ScrollPositionWithSingleContext.applyNewDimensions`.
    fn apply_new_dimensions(self: Handle<Self>, app: &mut App) {
        ScrollPositionWithSingleContext::apply_new_dimensions(self, app);
    }

    /// Dart's `ScrollPositionWithSingleContext.beginActivity`.
    fn begin_activity(self: Handle<Self>, app: &mut App, new_activity: Option<AnyScrollActivity>) {
        ScrollPositionWithSingleContext::begin_activity(self, app, new_activity);
    }

    /// Dart's `ScrollPositionWithSingleContext.applyUserOffset`.
    fn apply_user_offset(self: Handle<Self>, app: &mut App, delta: f64) {
        ScrollPositionWithSingleContext::apply_user_offset(self, app, delta);
    }

    /// Dart's `ScrollPositionWithSingleContext.goIdle`.
    fn go_idle(self: Handle<Self>, app: &mut App) {
        ScrollPositionWithSingleContext::go_idle(self, app);
    }

    /// Start a physics-driven simulation that settles the [`ViewportOffset::pixels`] position,
    /// starting at a particular velocity.
    ///
    /// This method defers to [`crate::ScrollPhysics::create_ballistic_simulation`], which
    /// typically provides a bounce simulation when the current position is out of
    /// bounds and a friction simulation when the position is in bounds but has a
    /// non-zero velocity.
    ///
    /// The velocity should be in logical pixels per second.
    fn go_ballistic(self: Handle<Self>, app: &mut App, velocity: f64) {
        ScrollPositionWithSingleContext::go_ballistic(self, app, velocity);
    }

    /// Dart's `ScrollPositionWithSingleContext.userScrollDirection`.
    fn user_scroll_direction(self: Handle<Self>, app: &App) -> ScrollDirection {
        self.single_context_data(app).user_scroll_direction
    }

    /// Set [`ViewportOffset::user_scroll_direction`] to the given value.
    ///
    /// If this changes the value, then a [`crate::UserScrollNotification`] is dispatched.
    fn update_user_scroll_direction(self: Handle<Self>, app: &mut App, value: ScrollDirection) {
        if ScrollPositionWithSingleContextLeaf::user_scroll_direction(self, app) == value {
            return;
        }
        self.single_context_data_mut(app).user_scroll_direction = value;
        self.did_update_scroll_direction(app, value);
    }

    /// Dart's `ScrollPositionWithSingleContext.animateTo`.
    fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        ScrollPositionWithSingleContext::animate_to(self, app, to, duration, curve);
    }

    /// Dart's `ScrollPositionWithSingleContext.jumpTo`.
    fn jump_to(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionWithSingleContext::jump_to(self, app, value);
    }

    /// Dart's `ScrollPositionWithSingleContext.pointerScroll`.
    fn pointer_scroll(self: Handle<Self>, app: &mut App, delta: f64) {
        ScrollPositionWithSingleContext::pointer_scroll(self, app, delta);
    }

    /// Dart's deprecated `ScrollPositionWithSingleContext.jumpToWithoutSettling`.
    fn jump_to_without_settling(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionWithSingleContext::jump_to_without_settling(self, app, value);
    }

    /// Dart's `ScrollPositionWithSingleContext.hold`.
    fn hold(
        self: Handle<Self>,
        app: &mut App,
        hold_cancel_callback: Listener,
    ) -> Rc<dyn ScrollHoldController> {
        ScrollPositionWithSingleContext::hold(self, app, hold_cancel_callback)
    }

    /// Dart's `ScrollPositionWithSingleContext.drag`.
    fn drag(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag> {
        ScrollPositionWithSingleContext::drag(self, app, details, drag_cancel_callback)
    }

    /// Dart's `ScrollPositionWithSingleContext.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        ScrollPositionWithSingleContext::dispose(self, app);
    }

    /// Dart's `ScrollPositionWithSingleContext.debugFillDescription`.
    fn debug_fill_description(self: Handle<Self>, app: &App, description: &mut Vec<String>) {
        ScrollPositionWithSingleContext::debug_fill_description(self, app, description);
    }
}

/// The `ViewportOffset` members a [`ScrollPositionWithSingleContextLeaf`] inherits. Write it
/// inside `impl ViewportOffset for MyPosition { .. }`.
#[macro_export]
macro_rules! scroll_position_with_single_context_viewport_offset_overrides {
    () => {
        fn pixels(self: ::reveal_foundation::Handle<Self>, app: &::reveal_foundation::App) -> f64 {
            $crate::ScrollPositionBase::pixels(self, app)
        }

        fn has_pixels(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::ScrollPositionBase::has_pixels(self, app)
        }

        fn apply_viewport_dimension(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            viewport_dimension: f64,
        ) -> bool {
            $crate::ScrollPositionBase::apply_viewport_dimension(self, app, viewport_dimension)
        }

        fn apply_content_dimensions(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            min_scroll_extent: f64,
            max_scroll_extent: f64,
        ) -> bool {
            $crate::ScrollPositionBase::apply_content_dimensions(
                self,
                app,
                min_scroll_extent,
                max_scroll_extent,
            )
        }

        fn correct_by(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            correction: f64,
        ) {
            $crate::ScrollPositionBase::correct_by(self, app, correction);
        }

        fn jump_to(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            value: f64,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::jump_to(self, app, value);
        }

        fn animate_to(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            to: f64,
            duration: ::std::time::Duration,
            curve: ::std::rc::Rc<dyn ::reveal_animation::Curve>,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::animate_to(self, app, to, duration, curve);
        }

        fn move_to(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            to: f64,
            duration: ::std::option::Option<::std::time::Duration>,
            curve: ::std::option::Option<::std::rc::Rc<dyn ::reveal_animation::Curve>>,
            clamp: ::std::option::Option<bool>,
        ) {
            $crate::ScrollPositionBase::move_to(self, app, to, duration, curve, clamp);
        }

        fn user_scroll_direction(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::reveal_rendering::ScrollDirection {
            $crate::ScrollPositionWithSingleContextLeaf::user_scroll_direction(self, app)
        }

        fn allow_implicit_scrolling(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::ScrollPositionBase::allow_implicit_scrolling(self, app)
        }
    };
}

/// The `ScrollPosition` members a [`ScrollPositionWithSingleContextLeaf`] inherits, plus
/// [`scroll_position_accessors!`](crate::scroll_position_accessors). Write it inside
/// `impl ScrollPosition for MyPosition { .. }`.
#[macro_export]
macro_rules! scroll_position_with_single_context_overrides {
    () => {
        $crate::scroll_position_accessors!();

        fn axis_direction(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::reveal_painting::AxisDirection {
            $crate::ScrollPositionWithSingleContextLeaf::axis_direction(self, app)
        }

        fn set_pixels(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            new_pixels: f64,
        ) -> f64 {
            $crate::ScrollPositionWithSingleContextLeaf::set_pixels(self, app, new_pixels)
        }

        fn absorb(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            other: $crate::AnyScrollPosition,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::absorb(self, app, other);
        }

        fn apply_new_dimensions(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::apply_new_dimensions(self, app);
        }

        fn begin_activity(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            new_activity: ::std::option::Option<$crate::AnyScrollActivity>,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::begin_activity(self, app, new_activity);
        }

        fn pointer_scroll(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            delta: f64,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::pointer_scroll(self, app, delta);
        }

        fn jump_to_without_settling(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            value: f64,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::jump_to_without_settling(self, app, value);
        }

        fn hold(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            hold_cancel_callback: ::reveal_foundation::Listener,
        ) -> ::std::rc::Rc<dyn $crate::ScrollHoldController> {
            $crate::ScrollPositionWithSingleContextLeaf::hold(self, app, hold_cancel_callback)
        }

        fn drag(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            details: ::reveal_gestures::DragStartDetails,
            drag_cancel_callback: ::reveal_foundation::Listener,
        ) -> ::std::rc::Rc<dyn ::reveal_gestures::Drag> {
            $crate::ScrollPositionWithSingleContextLeaf::drag(
                self,
                app,
                details,
                drag_cancel_callback,
            )
        }

        fn dispose(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::ScrollPositionWithSingleContextLeaf::dispose(self, app);
        }

        fn debug_fill_description(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
            description: &mut ::std::vec::Vec<::std::string::String>,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::debug_fill_description(
                self,
                app,
                description,
            );
        }
    };
}

/// The `ScrollActivityDelegate` members a [`ScrollPositionWithSingleContextLeaf`] implements.
/// Write it inside `impl ScrollActivityDelegate for MyPosition { .. }`.
#[macro_export]
macro_rules! scroll_position_with_single_context_delegate_overrides {
    () => {
        fn axis_direction(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::reveal_painting::AxisDirection {
            $crate::ScrollPositionWithSingleContextLeaf::axis_direction(self, app)
        }

        fn set_pixels(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            pixels: f64,
        ) -> f64 {
            $crate::ScrollPositionWithSingleContextLeaf::set_pixels(self, app, pixels)
        }

        fn apply_user_offset(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            delta: f64,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::apply_user_offset(self, app, delta);
        }

        fn go_idle(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::ScrollPositionWithSingleContextLeaf::go_idle(self, app);
        }

        fn go_ballistic(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            velocity: f64,
        ) {
            $crate::ScrollPositionWithSingleContextLeaf::go_ballistic(self, app, velocity);
        }
    };
}

/// A scroll position that manages scroll activities for a single
/// [`ScrollContext`].
///
/// This type is a concrete implementor of [`ScrollPosition`] logic that handles a
/// single scroll context, such as a `Scrollable`. An instance of this type
/// manages [`ScrollActivity`] instances, which change what content is visible in
/// the `Scrollable`'s `Viewport`.
///
/// It is also the base class `_DraggableScrollableSheetScrollPosition` and
/// `_CupertinoSheetScrollPosition` extend: its field is the
/// [`ScrollPositionWithSingleContextData`] bag, the members it overrides are
/// [`ScrollPositionWithSingleContextLeaf`]'s, and the shared bodies are the associated
/// functions below.
///
/// See also:
///
///  * [`ScrollPosition`], which defines the underlying model for a position
///    within a `Scrollable` but is agnostic as to how that position is
///    changed.
///  * `ScrollController`, which can manipulate one or more [`ScrollPosition`]s,
///    and which uses [`ScrollPositionWithSingleContext`] as its default type for
///    scroll positions.
pub struct ScrollPositionWithSingleContext {
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
}

impl ScrollPositionWithSingleContext {
    /// Create a [`ScrollPosition`] object that manages its behavior using
    /// [`ScrollActivity`] objects.
    ///
    /// The `initial_pixels` argument can be `None`, but in that case it is
    /// imperative that the value be set, using [`ScrollPosition::correct_pixels`], as soon as
    /// [`ScrollPosition::apply_new_dimensions`] is invoked, before calling
    /// [`ScrollPositionBase::apply_new_dimensions`]. Dart's default is `0.0`.
    ///
    /// If `keep_scroll_offset` is true (Dart's default), the current scroll offset is
    /// saved with `PageStorage` and restored if this scroll position's scrollable
    /// is recreated.
    pub fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        initial_pixels: Option<f64>,
        keep_scroll_offset: bool,
        old_position: Option<AnyScrollPosition>,
        debug_label: Option<String>,
    ) -> Handle<ScrollPositionWithSingleContext> {
        let scroll_position =
            ScrollPositionData::new(app, physics, context, keep_scroll_offset, debug_label);
        let this = app.create(ScrollPositionWithSingleContext {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
        });
        ScrollPositionWithSingleContext::init(this, app, initial_pixels, old_position);
        this
    }

    /// Dart's `ScrollPositionWithSingleContext` constructor body, which runs the
    /// [`ScrollPositionBase::init`] of its superclass first.
    ///
    /// A leaf's constructor calls this right after `App::create`, where Dart's superclass
    /// constructor would run.
    pub fn init<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        initial_pixels: Option<f64>,
        old_position: Option<AnyScrollPosition>,
    ) {
        // If `old_position` is not `None`, the base constructor first calls `absorb`, which
        // may set the pixels and the activity.
        ScrollPositionBase::init(this, app, old_position);
        if !ViewportOffset::has_pixels(this, app)
            && let Some(initial_pixels) = initial_pixels
        {
            this.correct_pixels(app, initial_pixels);
        }
        if this.activity(app).is_none() {
            ScrollActivityDelegate::go_idle(this, app);
        }
        debug_assert!(this.activity(app).is_some());
    }

    /// See [`ScrollPositionWithSingleContextLeaf::axis_direction`].
    pub fn axis_direction<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &App,
    ) -> AxisDirection {
        this.context(app).axis_direction(app)
    }

    /// See [`ScrollPositionWithSingleContextLeaf::set_pixels`].
    pub fn set_pixels<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        new_pixels: f64,
    ) -> f64 {
        debug_assert!(
            this.activity(app)
                .expect("a position has an activity")
                .is_scrolling(app)
        );
        ScrollPositionBase::set_pixels(this, app, new_pixels)
    }

    /// See [`ScrollPositionWithSingleContextLeaf::absorb`].
    pub fn absorb<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        other: AnyScrollPosition,
    ) {
        ScrollPositionBase::absorb(this, app, other);
        let Some(other) = other.downcast::<P>(app) else {
            ScrollActivityDelegate::go_idle(this, app);
            return;
        };
        let delegate = this.as_scroll_activity_delegate();
        this.activity(app)
            .expect("absorb adopts the other activity")
            .update_delegate(app, delegate);
        this.single_context_data_mut(app).user_scroll_direction =
            other.single_context_data(app).user_scroll_direction;
        debug_assert!(this.single_context_data(app).current_drag.is_none());
        if let Some(current_drag) = other.single_context_data(app).current_drag {
            this.single_context_data_mut(app).current_drag = Some(current_drag);
            current_drag.update_delegate(app, delegate);
            other.single_context_data_mut(app).current_drag = None;
        }
    }

    /// See [`ScrollPositionWithSingleContextLeaf::apply_new_dimensions`].
    pub fn apply_new_dimensions<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
    ) {
        ScrollPositionBase::apply_new_dimensions(this, app);
        let physics = this.physics(app);
        let metrics = this.copy_with(app);
        let can_drag = physics.should_accept_user_offset(&metrics);
        this.context(app).set_can_drag(app, can_drag);
    }

    /// See [`ScrollPositionWithSingleContextLeaf::begin_activity`].
    pub fn begin_activity<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        new_activity: Option<AnyScrollActivity>,
    ) {
        this.single_context_data_mut(app).held_previous_velocity = 0.0;
        let Some(new_activity) = new_activity else {
            return;
        };
        debug_assert!(new_activity.delegate(app) == this.as_scroll_activity_delegate());
        ScrollPositionBase::begin_activity(this, app, Some(new_activity));
        if let Some(current_drag) = this.single_context_data(app).current_drag {
            current_drag.dispose(app);
        }
        this.single_context_data_mut(app).current_drag = None;
        if !this
            .activity(app)
            .expect("a position has an activity")
            .is_scrolling(app)
        {
            this.update_user_scroll_direction(app, ScrollDirection::Idle);
        }
    }

    /// See [`ScrollPositionWithSingleContextLeaf::apply_user_offset`].
    pub fn apply_user_offset<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        delta: f64,
    ) {
        let direction = if delta > 0.0 {
            ScrollDirection::Forward
        } else {
            ScrollDirection::Reverse
        };
        this.update_user_scroll_direction(app, direction);
        let physics = this.physics(app);
        let metrics = this.copy_with(app);
        let new_pixels = ViewportOffset::pixels(this, app)
            - physics.apply_physics_to_user_offset(&metrics, delta);
        ScrollPosition::set_pixels(this, app, new_pixels);
    }

    /// See [`ScrollPositionWithSingleContextLeaf::go_idle`].
    pub fn go_idle<P: ScrollPositionWithSingleContextLeaf>(this: Handle<P>, app: &mut App) {
        let activity = IdleScrollActivity::new(app, this.as_scroll_activity_delegate());
        ScrollPosition::begin_activity(this, app, Some(activity.as_activity()));
    }

    /// See [`ScrollPositionWithSingleContextLeaf::go_ballistic`].
    pub fn go_ballistic<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        velocity: f64,
    ) {
        debug_assert!(ViewportOffset::has_pixels(this, app));
        let physics = this.physics(app);
        let metrics = this.copy_with(app);
        match physics.create_ballistic_simulation(&metrics, velocity) {
            Some(simulation) => {
                let vsync = this.context(app).vsync();
                let should_ignore_pointer = this.should_ignore_pointer(app);
                let activity = BallisticScrollActivity::new(
                    app,
                    this.as_scroll_activity_delegate(),
                    simulation,
                    vsync,
                    should_ignore_pointer,
                );
                ScrollPosition::begin_activity(this, app, Some(activity.as_activity()));
            }
            None => ScrollActivityDelegate::go_idle(this, app),
        }
    }

    /// See [`ScrollPositionWithSingleContextLeaf::animate_to`].
    pub fn animate_to<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        let physics = this.physics(app);
        let metrics = this.copy_with(app);
        if near_equal(
            Some(to),
            Some(ViewportOffset::pixels(this, app)),
            physics.tolerance_for(&metrics).distance,
        ) {
            // Skip the animation, go straight to the position as we are already close.
            ViewportOffset::jump_to(this, app, to);
            return;
        }

        let from = ViewportOffset::pixels(this, app);
        let vsync = this.context(app).vsync();
        let activity = DrivenScrollActivity::new(
            app,
            this.as_scroll_activity_delegate(),
            from,
            to,
            duration,
            curve,
            vsync,
        );
        ScrollPosition::begin_activity(this, app, Some(activity.as_activity()));
    }

    /// See [`ScrollPositionWithSingleContextLeaf::jump_to`].
    pub fn jump_to<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        value: f64,
    ) {
        ScrollActivityDelegate::go_idle(this, app);
        if ViewportOffset::pixels(this, app) != value {
            let old_pixels = ViewportOffset::pixels(this, app);
            this.force_pixels(app, value);
            this.did_start_scroll(app);
            let delta = ViewportOffset::pixels(this, app) - old_pixels;
            this.did_update_scroll_position_by(app, delta);
            this.did_end_scroll(app);
        }
        ScrollActivityDelegate::go_ballistic(this, app, 0.0);
    }

    /// See [`ScrollPositionWithSingleContextLeaf::pointer_scroll`].
    pub fn pointer_scroll<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        delta: f64,
    ) {
        if delta == 0.0 {
            ScrollActivityDelegate::go_ballistic(this, app, 0.0);
            return;
        }

        let target_pixels = (ViewportOffset::pixels(this, app) + delta)
            .max(this.min_scroll_extent(app))
            .min(this.max_scroll_extent(app));
        if target_pixels != ViewportOffset::pixels(this, app) {
            ScrollActivityDelegate::go_idle(this, app);
            let direction = if -delta > 0.0 {
                ScrollDirection::Forward
            } else {
                ScrollDirection::Reverse
            };
            this.update_user_scroll_direction(app, direction);
            let old_pixels = ViewportOffset::pixels(this, app);
            // Set the notifier before calling force pixels.
            // This is set to false again after going ballistic below.
            this.is_scrolling_notifier(app).set_value(app, true);
            this.force_pixels(app, target_pixels);
            this.did_start_scroll(app);
            let moved = ViewportOffset::pixels(this, app) - old_pixels;
            this.did_update_scroll_position_by(app, moved);
            this.did_end_scroll(app);
            ScrollActivityDelegate::go_ballistic(this, app, 0.0);
        }
    }

    /// See [`ScrollPositionWithSingleContextLeaf::jump_to_without_settling`].
    pub fn jump_to_without_settling<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        value: f64,
    ) {
        ScrollActivityDelegate::go_idle(this, app);
        if ViewportOffset::pixels(this, app) != value {
            let old_pixels = ViewportOffset::pixels(this, app);
            this.force_pixels(app, value);
            this.did_start_scroll(app);
            let delta = ViewportOffset::pixels(this, app) - old_pixels;
            this.did_update_scroll_position_by(app, delta);
            this.did_end_scroll(app);
        }
    }

    /// See [`ScrollPositionWithSingleContextLeaf::hold`].
    pub fn hold<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        hold_cancel_callback: Listener,
    ) -> Rc<dyn ScrollHoldController> {
        let previous_velocity = this
            .activity(app)
            .expect("a position has an activity")
            .velocity(app);
        let hold_activity = HoldScrollActivity::new(
            app,
            this.as_scroll_activity_delegate(),
            Some(hold_cancel_callback),
        );
        ScrollPosition::begin_activity(this, app, Some(hold_activity.as_activity()));
        this.single_context_data_mut(app).held_previous_velocity = previous_velocity;
        Rc::new(hold_activity)
    }

    /// See [`ScrollPositionWithSingleContextLeaf::drag`].
    pub fn drag<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag> {
        let physics = this.physics(app);
        let held_previous_velocity = this.single_context_data(app).held_previous_velocity;
        let drag = ScrollDragController::new(
            app,
            this.as_scroll_activity_delegate(),
            details,
            Some(drag_cancel_callback),
            Some(physics.carried_momentum(held_previous_velocity)),
            physics.drag_start_distance_motion_threshold(),
        );
        let activity = DragScrollActivity::new(app, this.as_scroll_activity_delegate(), drag);
        ScrollPosition::begin_activity(this, app, Some(activity.as_activity()));
        debug_assert!(this.single_context_data(app).current_drag.is_none());
        this.single_context_data_mut(app).current_drag = Some(drag);
        Rc::new(drag)
    }

    /// See [`ScrollPositionWithSingleContextLeaf::dispose`].
    pub fn dispose<P: ScrollPositionWithSingleContextLeaf>(this: Handle<P>, app: &mut App) {
        if let Some(current_drag) = this.single_context_data(app).current_drag {
            current_drag.dispose(app);
        }
        this.single_context_data_mut(app).current_drag = None;
        ScrollPositionBase::dispose(this, app);
    }

    /// See [`ScrollPositionWithSingleContextLeaf::debug_fill_description`].
    pub fn debug_fill_description<P: ScrollPositionWithSingleContextLeaf>(
        this: Handle<P>,
        app: &App,
        description: &mut Vec<String>,
    ) {
        ScrollPositionBase::debug_fill_description(this, app, description);
        description.push(this.context(app).type_name().to_string());
        description.push(format!("{:?}", this.physics(app)));
        description.push(
            this.activity(app)
                .map_or_else(|| "null".to_string(), |activity| activity.describe(app)),
        );
        description.push(format!(
            "{:?}",
            ViewportOffset::user_scroll_direction(this, app)
        ));
    }
}

impl ChangeNotifier for ScrollPositionWithSingleContext {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for ScrollPositionWithSingleContext {
    crate::scroll_position_with_single_context_viewport_offset_overrides!();
}

impl ScrollPosition for ScrollPositionWithSingleContext {
    crate::scroll_position_with_single_context_overrides!();
}

impl ScrollActivityDelegate for ScrollPositionWithSingleContext {
    crate::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for ScrollPositionWithSingleContext {
    crate::scroll_position_with_single_context_accessors!();
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use reveal_animation::Curves;
    use reveal_embedder::Offset;
    use reveal_foundation::ListenableObject;
    use reveal_gestures::{DragEndDetails, DragUpdateDetails, Velocity};
    use reveal_scheduler::SchedulerBinding;

    use super::*;
    use crate::test_harness::{ScrollHarness, mount_scroll_harness};
    use crate::widgets::scroll_physics::{BouncingScrollPhysics, ClampingScrollPhysics};

    fn pump(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn position(
        app: &mut App,
        harness: &ScrollHarness,
        physics: ScrollPhysicsRef,
    ) -> Handle<ScrollPositionWithSingleContext> {
        ScrollPositionWithSingleContext::new(
            app,
            physics,
            Rc::clone(&harness.scroll_context),
            Some(0.0),
            true,
            None,
            None,
        )
    }

    /// The viewport reporting a 100 pixel window over 400 pixels of scrollable content.
    fn lay_out(app: &mut App, position: Handle<ScrollPositionWithSingleContext>) {
        assert!(position.apply_viewport_dimension(app, 100.0));
        assert!(position.apply_content_dimensions(app, 0.0, 400.0));
        app.drain_microtasks();
    }

    fn drag_by(delta: f64) -> DragUpdateDetails {
        DragUpdateDetails::new(
            Offset::ZERO,
            None,
            Some(Duration::ZERO),
            Offset::new(0.0, delta),
            Some(delta),
            None,
        )
    }

    fn is_scrolling(app: &App, position: Handle<ScrollPositionWithSingleContext>) -> bool {
        *app.get(position.is_scrolling_notifier(app)).value()
    }

    #[test]
    fn applying_dimensions_makes_the_metrics_available_and_move_to_clamps() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        assert!(!position.have_dimensions(&app));
        assert_eq!(ViewportOffset::pixels(position, &app), 0.0);

        lay_out(&mut app, position);

        assert!(position.have_dimensions(&app));
        assert_eq!(position.min_scroll_extent(&app), 0.0);
        assert_eq!(position.max_scroll_extent(&app), 400.0);
        assert_eq!(position.viewport_dimension(&app), 100.0);
        assert_eq!(position.extent_inside(&app), 100.0);
        assert_eq!(position.extent_after(&app), 400.0);
        assert_eq!(app.get(harness.context).can_drag, vec![true]);

        position.move_to(&mut app, 500.0, None, None, None);
        assert_eq!(ViewportOffset::pixels(position, &app), 400.0);
        position.move_to(&mut app, -50.0, None, None, None);
        assert_eq!(ViewportOffset::pixels(position, &app), 0.0);
        assert!(!position.out_of_range(&app));
    }

    #[test]
    fn jump_to_notifies_listeners_and_dispatches_start_update_end() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);
        harness.notifications.borrow_mut().clear();

        let notified = Rc::new(Cell::new(0));
        let counter = notified.clone();
        ListenableObject::add_listener(
            position,
            &mut app,
            Listener::new(move |_app| counter.set(counter.get() + 1)),
        );

        ViewportOffset::jump_to(position, &mut app, 120.0);

        assert_eq!(ViewportOffset::pixels(position, &app), 120.0);
        assert_eq!(notified.get(), 1);
        assert_eq!(
            *harness.notifications.borrow(),
            ["start", "update 120.0", "end"]
        );
        assert_eq!(app.get(harness.context).saved_offsets, vec![120.0]);
        assert!(!is_scrolling(&app, position));
    }

    #[test]
    fn animate_to_drives_the_position_through_the_ticker() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);

        ViewportOffset::animate_to(
            position,
            &mut app,
            200.0,
            Duration::from_millis(100),
            Curves::linear(),
        );
        assert!(is_scrolling(&app, position));

        pump(&mut app, Duration::ZERO);
        assert_eq!(ViewportOffset::pixels(position, &app), 0.0);

        pump(&mut app, Duration::from_millis(50));
        assert!((ViewportOffset::pixels(position, &app) - 100.0).abs() < 0.001);

        pump(&mut app, Duration::from_millis(100));
        assert_eq!(ViewportOffset::pixels(position, &app), 200.0);
    }

    #[test]
    fn animate_to_within_the_tolerance_jumps_instead() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);
        harness.notifications.borrow_mut().clear();

        ViewportOffset::animate_to(
            position,
            &mut app,
            0.000_001,
            Duration::from_millis(100),
            Curves::linear(),
        );

        assert_eq!(ViewportOffset::pixels(position, &app), 0.000_001);
        assert!(!is_scrolling(&app, position));
    }

    #[test]
    fn a_clamping_drag_scrolls_the_position_and_settles_at_once() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);
        harness.notifications.borrow_mut().clear();

        let _hold = ScrollPosition::hold(position, &mut app, Listener::new(|_app| {}));
        let drag = ScrollPosition::drag(
            position,
            &mut app,
            DragStartDetails::default(),
            Listener::new(|_app| {}),
        );

        drag.update(&mut app, drag_by(-30.0));
        assert_eq!(ViewportOffset::pixels(position, &app), 30.0);
        assert_eq!(
            ViewportOffset::user_scroll_direction(position, &app),
            ScrollDirection::Reverse
        );
        assert!(is_scrolling(&app, position));

        drag.end(
            &mut app,
            DragEndDetails::new(Offset::ZERO, None, Velocity::ZERO, Some(0.0)),
        );

        // In range with no velocity, the physics offers no simulation, so the position idles.
        assert_eq!(ViewportOffset::pixels(position, &app), 30.0);
        assert!(!is_scrolling(&app, position));
        assert_eq!(
            ViewportOffset::user_scroll_direction(position, &app),
            ScrollDirection::Idle
        );
        assert_eq!(
            *harness.notifications.borrow(),
            ["start", "user Reverse", "update 30.0", "end", "user Idle"]
        );
    }

    #[test]
    fn a_bouncing_drag_overscrolls_and_springs_back_over_frames() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(BouncingScrollPhysics::new()));
        lay_out(&mut app, position);

        let drag = ScrollPosition::drag(
            position,
            &mut app,
            DragStartDetails::default(),
            Listener::new(|_app| {}),
        );
        drag.update(&mut app, drag_by(-500.0));
        assert!(ViewportOffset::pixels(position, &app) > 400.0);
        assert!(position.out_of_range(&app));

        drag.end(
            &mut app,
            DragEndDetails::new(Offset::ZERO, None, Velocity::ZERO, Some(0.0)),
        );
        // The ballistic activity has not ticked yet.
        assert!(position.out_of_range(&app));
        assert!(is_scrolling(&app, position));

        for frame in 0..200 {
            pump(&mut app, Duration::from_millis(16 * frame));
            if !is_scrolling(&app, position) {
                break;
            }
        }

        assert!(!is_scrolling(&app, position));
        assert!((ViewportOffset::pixels(position, &app) - 400.0).abs() < 1.0);
    }

    #[test]
    fn a_new_position_absorbs_the_old_one() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let old = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, old);
        ViewportOffset::jump_to(old, &mut app, 75.0);

        let new = ScrollPositionWithSingleContext::new(
            &mut app,
            Rc::new(ClampingScrollPhysics::new()),
            Rc::clone(&harness.scroll_context),
            Some(0.0),
            true,
            Some(old.as_scroll_position()),
            None,
        );

        assert_eq!(ViewportOffset::pixels(new, &app), 75.0);
        assert_eq!(new.max_scroll_extent(&app), 400.0);
        assert!(new.activity(&app).is_some());
        assert!(old.activity(&app).is_none());
    }

    #[test]
    fn a_pointer_scroll_moves_the_position_within_the_extents() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);
        harness.notifications.borrow_mut().clear();

        ScrollPosition::pointer_scroll(position, &mut app, 60.0);
        assert_eq!(ViewportOffset::pixels(position, &app), 60.0);
        assert_eq!(
            *harness.notifications.borrow(),
            ["user Reverse", "start", "update 60.0", "end", "user Idle"]
        );

        // Past the maximum extent it stops at the edge.
        ScrollPosition::pointer_scroll(position, &mut app, 1000.0);
        assert_eq!(ViewportOffset::pixels(position, &app), 400.0);
    }

    #[test]
    fn the_description_names_the_context_the_physics_and_the_activity() {
        let mut app = App::new();
        let harness = mount_scroll_harness(&mut app);
        let position = position(&mut app, &harness, Rc::new(ClampingScrollPhysics::new()));
        lay_out(&mut app, position);

        let description = position.describe(&app);
        assert!(description.contains("offset: 0.0"), "{description}");
        assert!(description.contains("range: 0.0..400.0"), "{description}");
        assert!(description.contains("viewport: 100.0"), "{description}");
        assert!(description.contains("TestScrollContext"), "{description}");
        assert!(
            description.contains("ClampingScrollPhysics"),
            "{description}"
        );
        assert!(description.contains("IdleScrollActivity"), "{description}");
    }
}

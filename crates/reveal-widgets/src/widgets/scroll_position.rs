//! Flutter counterpart: `widgets/scroll_position.dart`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_embedder::{Rect, clamp_double};
use reveal_foundation::{
    App, CompleterFuture, Handle, HandleId, Listenable, ListenableObject, Listener,
    PRECISION_ERROR_TOLERANCE, ValueNotifier,
};
use reveal_gestures::{Drag, DragStartDetails};
use reveal_painting::{Axis, AxisDirection, axis_direction_to_axis, transform_rect};
use reveal_physics::{Tolerance, near_equal};
use reveal_rendering::{
    AnyRenderAbstractViewport, AnyRenderObject, AnyViewportOffset, ScrollDirection, ViewportOffset,
};
use reveal_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};

use crate::framework::{BuildContext, Notification};
use crate::widgets::page_storage::PageStorage;
use crate::widgets::scroll_activity::{AnyScrollActivity, ScrollHoldController};
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_metrics::{FixedScrollMetrics, ScrollMetrics};
use crate::widgets::scroll_notification::{
    ScrollUpdateNotification, UserScrollNotification, ViewportNotificationData,
    ViewportNotificationMixin,
};
use crate::widgets::scroll_physics::ScrollPhysicsRef;

/// The policy to use when applying the `alignment` parameter of
/// [`ScrollPosition::ensure_visible`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollPositionAlignmentPolicy {
    /// Use the `alignment` property of [`ScrollPosition::ensure_visible`] to decide
    /// where to align the visible object.
    Explicit,

    /// Find the bottom edge of the scroll container, and scroll the container, if
    /// necessary, to show the bottom of the object.
    ///
    /// For example, find the bottom edge of the scroll container. If the bottom
    /// edge of the item is below the bottom edge of the scroll container, scroll
    /// the item so that the bottom of the item is just visible. If the entire
    /// item is already visible, then do nothing.
    KeepVisibleAtEnd,

    /// Find the top edge of the scroll container, and scroll the container if
    /// necessary to show the top of the object.
    ///
    /// For example, find the top edge of the scroll container. If the top edge of
    /// the item is above the top edge of the scroll container, scroll the item so
    /// that the top of the item is just visible. If the entire item is already
    /// visible, then do nothing.
    KeepVisibleAtStart,
}

/// The fields of Dart's `ScrollPosition` base class, held under the field `scroll_position`.
pub struct ScrollPositionData {
    physics: ScrollPhysicsRef,
    context: Rc<dyn ScrollContext>,
    keep_scroll_offset: bool,
    debug_label: Option<String>,
    min_scroll_extent: Option<f64>,
    max_scroll_extent: Option<f64>,
    implied_velocity: f64,
    pixels: Option<f64>,
    viewport_dimension: Option<f64>,
    have_dimensions: bool,
    did_change_viewport_dimension_or_receive_correction: bool,
    pending_dimensions: bool,
    last_metrics: Option<FixedScrollMetrics>,
    have_scheduled_update_notification: bool,
    last_axis: Option<Axis>,
    is_scrolling_notifier: Handle<ValueNotifier<bool>>,
    activity: Option<AnyScrollActivity>,
}

impl ScrollPositionData {
    /// The bag of a position that has neither dimensions nor an activity yet.
    pub fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        keep_scroll_offset: bool,
        debug_label: Option<String>,
    ) -> ScrollPositionData {
        ScrollPositionData {
            physics,
            context,
            keep_scroll_offset,
            debug_label,
            min_scroll_extent: None,
            max_scroll_extent: None,
            implied_velocity: 0.0,
            pixels: None,
            viewport_dimension: None,
            have_dimensions: false,
            did_change_viewport_dimension_or_receive_correction: true,
            pending_dimensions: false,
            last_metrics: None,
            have_scheduled_update_notification: false,
            last_axis: None,
            is_scrolling_notifier: app.create(ValueNotifier::new(false)),
            activity: None,
        }
    }
}

/// The accessors [`ScrollPosition`] asks for, for a struct whose bag is the field
/// `scroll_position`.
#[macro_export]
macro_rules! scroll_position_accessors {
    () => {
        fn scroll_position_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ScrollPositionData {
            &app.get(self).scroll_position
        }

        fn scroll_position_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ScrollPositionData {
            &mut app.get_mut(self).scroll_position
        }
    };
}

/// Determines which portion of the content is visible in a scroll view.
///
/// The [`pixels`](ViewportOffset::pixels) value determines the scroll offset that the scroll
/// view uses to select which part of its content to display. As the user scrolls the
/// viewport, this value changes, which changes the content that is displayed.
///
/// The [`ScrollPosition`] applies [`physics`](Self::physics) to scrolling, and stores the
/// [`min_scroll_extent`](Self::min_scroll_extent) and
/// [`max_scroll_extent`](Self::max_scroll_extent).
///
/// Scrolling is controlled by the current [`activity`](Self::activity), which is set by
/// [`begin_activity`](Self::begin_activity). [`ScrollPosition`] itself does not start any
/// activities. Instead, implementors, such as `ScrollPositionWithSingleContext`, typically
/// start activities in response to user input or instructions from a `ScrollController`.
///
/// This object is a [`Listenable`] that notifies its listeners when
/// [`pixels`](ViewportOffset::pixels) changes.
///
/// Dart's `ScrollMetrics` mixin members are re-declared here with the arena receiver;
/// [`copy_with`](Self::copy_with) is the [`ScrollMetrics`] snapshot to hand to anything that
/// reads the position's metrics.
///
/// ## Implementing ScrollPosition
///
/// Over time, a `Scrollable` might have many different [`ScrollPosition`]
/// objects. For example, if `Scrollable.physics` changes type, `Scrollable`
/// creates a new [`ScrollPosition`] with the new physics. To transfer state from
/// the old instance to the new instance, implementors override
/// [`absorb`](Self::absorb).
///
/// Implementors also need to call
/// [`did_update_scroll_direction`](Self::did_update_scroll_direction) whenever
/// [`user_scroll_direction`](ViewportOffset::user_scroll_direction) changes values.
///
/// See also:
///
///  * `Scrollable`, which uses a [`ScrollPosition`] to determine which portion of
///    its content to display.
///  * `ScrollController`, which can be used with `ListView`, `GridView` and
///    other scrollable widgets to control a [`ScrollPosition`].
///  * `ScrollPositionWithSingleContext`, which is the most commonly used
///    concrete implementor.
///  * [`ScrollNotification`](crate::ScrollNotification) and `NotificationListener`, which can be used to watch
///    the scroll position without using a `ScrollController`.
pub trait ScrollPosition: ViewportOffset {
    /// Dart's `ScrollPosition` fields, held under the field `scroll_position`
    /// ([`scroll_position_accessors!`](crate::scroll_position_accessors)).
    fn scroll_position_data(self: Handle<Self>, app: &App) -> &ScrollPositionData;

    /// See [`scroll_position_data`](Self::scroll_position_data).
    fn scroll_position_data_mut(self: Handle<Self>, app: &mut App) -> &mut ScrollPositionData;

    /// How the scroll position should respond to user input.
    ///
    /// For example, determines how the widget continues to animate after the
    /// user stops dragging the scroll view.
    fn physics(self: Handle<Self>, app: &App) -> ScrollPhysicsRef {
        self.scroll_position_data(app).physics.clone()
    }

    /// Where the scrolling is taking place.
    ///
    /// Typically implemented by `ScrollableState`.
    fn context(self: Handle<Self>, app: &App) -> Rc<dyn ScrollContext> {
        Rc::clone(&self.scroll_position_data(app).context)
    }

    /// Save the current scroll offset with `PageStorage` and restore it if
    /// this scroll position's scrollable is recreated.
    ///
    /// See also:
    ///
    ///  * `ScrollController::keep_scroll_offset`, which creates scroll positions and
    ///    initializes this property.
    fn keep_scroll_offset(self: Handle<Self>, app: &App) -> bool {
        self.scroll_position_data(app).keep_scroll_offset
    }

    /// A label that is used in the [`describe`](Self::describe) output.
    ///
    /// Intended to aid with identifying scroll position instances in debug
    /// output.
    fn debug_label(self: Handle<Self>, app: &App) -> Option<&str> {
        self.scroll_position_data(app).debug_label.as_deref()
    }

    /// See [`ScrollMetrics::min_scroll_extent`].
    fn min_scroll_extent(self: Handle<Self>, app: &App) -> f64 {
        self.scroll_position_data(app)
            .min_scroll_extent
            .expect("minScrollExtent was read without content dimensions")
    }

    /// See [`ScrollMetrics::max_scroll_extent`].
    fn max_scroll_extent(self: Handle<Self>, app: &App) -> f64 {
        self.scroll_position_data(app)
            .max_scroll_extent
            .expect("maxScrollExtent was read without content dimensions")
    }

    /// See [`ScrollMetrics::has_content_dimensions`].
    fn has_content_dimensions(self: Handle<Self>, app: &App) -> bool {
        let data = self.scroll_position_data(app);
        data.min_scroll_extent.is_some() && data.max_scroll_extent.is_some()
    }

    /// See [`ScrollMetrics::viewport_dimension`].
    fn viewport_dimension(self: Handle<Self>, app: &App) -> f64 {
        self.scroll_position_data(app)
            .viewport_dimension
            .expect("viewportDimension was read before it was available")
    }

    /// See [`ScrollMetrics::has_viewport_dimension`].
    fn has_viewport_dimension(self: Handle<Self>, app: &App) -> bool {
        self.scroll_position_data(app).viewport_dimension.is_some()
    }

    /// The direction in which the scroll view scrolls; Dart's `ScrollMetrics.axisDirection`.
    fn axis_direction(self: Handle<Self>, app: &App) -> AxisDirection;

    /// See [`ScrollMetrics::axis`].
    fn axis(self: Handle<Self>, app: &App) -> Axis {
        axis_direction_to_axis(self.axis_direction(app))
    }

    /// See [`ScrollMetrics::device_pixel_ratio`].
    fn device_pixel_ratio(self: Handle<Self>, app: &App) -> f64 {
        self.context(app).device_pixel_ratio(app)
    }

    /// See [`ScrollMetrics::out_of_range`].
    fn out_of_range(self: Handle<Self>, app: &App) -> bool {
        self.copy_with(app).out_of_range()
    }

    /// See [`ScrollMetrics::at_edge`].
    fn at_edge(self: Handle<Self>, app: &App) -> bool {
        self.copy_with(app).at_edge()
    }

    /// See [`ScrollMetrics::extent_before`].
    fn extent_before(self: Handle<Self>, app: &App) -> f64 {
        self.copy_with(app).extent_before()
    }

    /// See [`ScrollMetrics::extent_inside`].
    fn extent_inside(self: Handle<Self>, app: &App) -> f64 {
        self.copy_with(app).extent_inside()
    }

    /// See [`ScrollMetrics::extent_after`].
    fn extent_after(self: Handle<Self>, app: &App) -> f64 {
        self.copy_with(app).extent_after()
    }

    /// See [`ScrollMetrics::extent_total`].
    fn extent_total(self: Handle<Self>, app: &App) -> f64 {
        self.copy_with(app).extent_total()
    }

    /// A snapshot of this position's [`ScrollMetrics`]; Dart's `ScrollMetrics.copyWith`.
    ///
    /// This is what to hand to anything that reads the metrics — the physics, a
    /// [`ScrollNotification`](crate::ScrollNotification), `ScrollIncrementDetails`.
    fn copy_with(self: Handle<Self>, app: &App) -> FixedScrollMetrics {
        let has_content_dimensions = self.has_content_dimensions(app);
        FixedScrollMetrics::new(
            has_content_dimensions.then(|| self.min_scroll_extent(app)),
            has_content_dimensions.then(|| self.max_scroll_extent(app)),
            self.has_pixels(app).then(|| self.pixels(app)),
            self.has_viewport_dimension(app)
                .then(|| self.viewport_dimension(app)),
            self.axis_direction(app),
            self.device_pixel_ratio(app),
        )
    }

    /// Whether [`viewport_dimension`](Self::viewport_dimension),
    /// [`min_scroll_extent`](Self::min_scroll_extent),
    /// [`max_scroll_extent`](Self::max_scroll_extent), [`out_of_range`](Self::out_of_range),
    /// and [`at_edge`](Self::at_edge) are available.
    ///
    /// Set to true just before the first time
    /// [`apply_new_dimensions`](Self::apply_new_dimensions) is called.
    fn have_dimensions(self: Handle<Self>, app: &App) -> bool {
        self.scroll_position_data(app).have_dimensions
    }

    /// Whether scrollables should absorb pointer events at this position.
    ///
    /// This value relates to the current [`activity`](Self::activity), which determines
    /// if additional touch input should be received by the scroll view or its children.
    /// If the position is overscrolled, as is allowed by
    /// [`crate::BouncingScrollPhysics`], children of the scroll view will receive pointer
    /// events as the scroll view settles back from the overscrolled state.
    fn should_ignore_pointer(self: Handle<Self>, app: &App) -> bool {
        !self.out_of_range(app)
            && self
                .activity(app)
                .is_none_or(|activity| activity.should_ignore_pointer(app))
    }

    /// Take any current applicable state from the given [`ScrollPosition`].
    ///
    /// This method is called by [`ScrollPositionBase::init`] if it is given an `old_position`.
    /// The `other` argument might not have the same runtime type as this object.
    ///
    /// This method can be destructive to the other [`ScrollPosition`]. The other
    /// object must be disposed immediately after this call.
    ///
    /// If the old [`ScrollPosition`] object has a different runtime type than this
    /// one, the [`AnyScrollActivity::reset_activity`] method is invoked on the newly
    /// adopted activity.
    ///
    /// ## Overriding
    ///
    /// Overrides of this method must call [`ScrollPositionBase::absorb`] after setting any
    /// metrics-related or activity-related state, since this method may restart
    /// the activity and scroll activities tend to use those metrics when being
    /// restarted.
    ///
    /// Overrides of this method might need to start an [`crate::IdleScrollActivity`] if
    /// they are unable to absorb the activity from the other [`ScrollPosition`].
    ///
    /// Overrides of this method might also need to update the delegates of
    /// absorbed scroll activities if they use themselves as a
    /// [`crate::ScrollActivityDelegate`].
    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionBase::absorb(self, app, other);
    }

    /// Update the scroll position ([`pixels`](ViewportOffset::pixels)) to a given pixel value.
    ///
    /// This should only be called by the current [`activity`](Self::activity), either during
    /// the transient callback phase or in response to user input.
    ///
    /// Returns the overscroll, if any. If the return value is 0.0, that means
    /// that [`pixels`](ViewportOffset::pixels) now returns the given `new_pixels`. If the
    /// return value is positive, then [`pixels`](ViewportOffset::pixels) is less than the
    /// requested value by the given amount (overscroll past the max extent), and if it is
    /// negative, it is greater than the requested value by the given amount (underscroll past
    /// the min extent).
    ///
    /// The amount of overscroll is computed by
    /// [`apply_boundary_conditions`](Self::apply_boundary_conditions).
    ///
    /// The amount of the change that is applied is reported using
    /// [`did_update_scroll_position_by`](Self::did_update_scroll_position_by).
    /// If there is any overscroll, it is reported using
    /// [`did_overscroll_by`](Self::did_overscroll_by).
    fn set_pixels(self: Handle<Self>, app: &mut App, new_pixels: f64) -> f64 {
        ScrollPositionBase::set_pixels(self, app, new_pixels)
    }

    /// Change the value of [`pixels`](ViewportOffset::pixels) to the new value, without
    /// notifying any customers.
    ///
    /// This is used to adjust the position while doing layout. In particular,
    /// this is typically called as a response to
    /// [`ViewportOffset::apply_viewport_dimension`] or
    /// [`ViewportOffset::apply_content_dimensions`] (in both cases, if this method is called,
    /// those methods should then return false to indicate that the position has been
    /// adjusted).
    ///
    /// Calling this is rarely correct in other contexts. It will not immediately
    /// cause the rendering to change, since it does not notify the widgets or
    /// render objects that might be listening to this object.
    ///
    /// To cause the position to jump or animate to a new value, consider
    /// [`ViewportOffset::jump_to`] or [`ViewportOffset::animate_to`], which will honor the
    /// normal conventions for changing the scroll offset.
    ///
    /// To force the [`pixels`](ViewportOffset::pixels) to a particular value without honoring
    /// the normal conventions for changing the scroll offset, consider
    /// [`force_pixels`](Self::force_pixels).
    fn correct_pixels(self: Handle<Self>, app: &mut App, value: f64) {
        self.scroll_position_data_mut(app).pixels = Some(value);
    }

    /// Change the value of [`pixels`](ViewportOffset::pixels) to the new value, and notify any
    /// customers, but without honoring normal conventions for changing the scroll offset.
    ///
    /// This is used to implement [`ViewportOffset::jump_to`]. It can also be used to adjust
    /// the position when the dimensions of the viewport change. It should only be
    /// used when manually implementing the logic for honoring the relevant
    /// conventions of the class.
    ///
    /// This should not be called during layout (e.g. when setting the initial
    /// scroll offset). Consider [`correct_pixels`](Self::correct_pixels) if you find you need
    /// to adjust the position during layout.
    fn force_pixels(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionBase::force_pixels(self, app, value);
    }

    /// Called whenever scrolling ends, to store the current scroll offset in a
    /// storage mechanism with a lifetime that matches the app's lifetime.
    ///
    /// The stored value will be used by
    /// [`restore_scroll_offset`](Self::restore_scroll_offset) when the [`ScrollPosition`] is
    /// recreated, in the case of the `Scrollable` being disposed then recreated in the same
    /// session.
    ///
    /// The default implementation writes the [`pixels`](ViewportOffset::pixels) using the
    /// nearest [`PageStorage`] found from the [`context`](Self::context)'s
    /// [`ScrollContext::storage_context`] property.
    fn save_scroll_offset(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::save_scroll_offset(self, app);
    }

    /// Called whenever the [`ScrollPosition`] is created, to restore the scroll
    /// offset if possible.
    ///
    /// The value is stored by [`save_scroll_offset`](Self::save_scroll_offset) when the scroll
    /// position changes, so that it can be restored in the case of the `Scrollable` being
    /// disposed then recreated in the same session.
    ///
    /// The default implementation reads the value from the nearest [`PageStorage`]
    /// found from the [`context`](Self::context)'s [`ScrollContext::storage_context`]
    /// property, and sets it using [`correct_pixels`](Self::correct_pixels), if
    /// [`pixels`](ViewportOffset::pixels) is still unset.
    ///
    /// This method is called from [`ScrollPositionBase::init`], so layout has not yet
    /// occurred, and the viewport dimensions aren't yet known when it is called.
    fn restore_scroll_offset(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::restore_scroll_offset(self, app);
    }

    /// Called by [`context`](Self::context) to restore the scroll offset to the provided
    /// value.
    ///
    /// The provided value has previously been provided to the [`context`](Self::context) by
    /// calling [`ScrollContext::save_offset`], e.g. from [`save_offset`](Self::save_offset).
    ///
    /// This method may be called right after the scroll position is created
    /// before layout has occurred. In that case, `initial_restore` is set to true
    /// and the viewport dimensions will not be known yet. If the
    /// [`context`](Self::context) doesn't have any information to restore the scroll offset
    /// this method is not called.
    ///
    /// The method may be called multiple times in the lifecycle of a
    /// [`ScrollPosition`] to restore it to different scroll offsets.
    fn restore_offset(self: Handle<Self>, app: &mut App, offset: f64, initial_restore: bool) {
        ScrollPositionBase::restore_offset(self, app, offset, initial_restore);
    }

    /// Called whenever scrolling ends, to persist the current scroll offset for
    /// state restoration purposes.
    ///
    /// The default implementation stores the current value of
    /// [`pixels`](ViewportOffset::pixels) on the [`context`](Self::context) by calling
    /// [`ScrollContext::save_offset`]. At a later point in time or after the application
    /// restarts, the [`context`](Self::context) may restore the scroll position to the
    /// persisted offset by calling [`restore_offset`](Self::restore_offset).
    fn save_offset(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::save_offset(self, app);
    }

    /// Returns the overscroll by applying the boundary conditions.
    ///
    /// If the given value is in bounds, returns 0.0. Otherwise, returns the
    /// amount of value that cannot be applied to [`pixels`](ViewportOffset::pixels) as a
    /// result of the boundary conditions. If the [`physics`](Self::physics) allow
    /// out-of-bounds scrolling, this method always returns 0.0.
    ///
    /// The default implementation defers to the [`physics`](Self::physics) object's
    /// [`crate::ScrollPhysics::apply_boundary_conditions`].
    fn apply_boundary_conditions(self: Handle<Self>, app: &mut App, value: f64) -> f64 {
        ScrollPositionBase::apply_boundary_conditions(self, app, value)
    }

    /// Verifies that the new content and viewport dimensions are acceptable.
    ///
    /// Called by [`ViewportOffset::apply_content_dimensions`] to determine its return value.
    ///
    /// Should return true if the current scroll offset is correct given
    /// the new content and viewport dimensions.
    ///
    /// Otherwise, should call [`correct_pixels`](Self::correct_pixels) to correct the scroll
    /// offset given the new dimensions, and then return false.
    ///
    /// This is only called when [`have_dimensions`](Self::have_dimensions) is true.
    ///
    /// The default implementation defers to
    /// [`crate::ScrollPhysics::adjust_position_for_new_dimensions`].
    fn correct_for_new_dimensions(
        self: Handle<Self>,
        app: &mut App,
        old_position: &dyn ScrollMetrics,
        new_position: &dyn ScrollMetrics,
    ) -> bool {
        ScrollPositionBase::correct_for_new_dimensions(self, app, old_position, new_position)
    }

    /// Notifies the activity that the dimensions of the underlying viewport or
    /// contents have changed.
    ///
    /// Called after [`ViewportOffset::apply_viewport_dimension`] or
    /// [`ViewportOffset::apply_content_dimensions`] have changed the
    /// [`min_scroll_extent`](Self::min_scroll_extent), the
    /// [`max_scroll_extent`](Self::max_scroll_extent), or the
    /// [`viewport_dimension`](Self::viewport_dimension). When this method is called, it
    /// should be called _after_ any corrections are applied to
    /// [`pixels`](ViewportOffset::pixels) using [`correct_pixels`](Self::correct_pixels), not
    /// before.
    ///
    /// The default implementation informs the [`activity`](Self::activity) of the new
    /// dimensions by calling its [`AnyScrollActivity::apply_new_dimensions`] method.
    ///
    /// An override calls [`ScrollPositionBase::apply_new_dimensions`] where Dart writes
    /// `super.applyNewDimensions()`.
    fn apply_new_dimensions(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::apply_new_dimensions(self, app);
    }

    /// Animates the position such that the given object is as visible as possible
    /// by just scrolling this position.
    ///
    /// The optional `target_render_object` parameter is used to determine which area
    /// of that object should be as visible as possible. If it is `None`, the entire
    /// render object (as defined by its paint bounds) will be as visible as possible. If
    /// it is provided, it must be a descendant of the object.
    ///
    /// See also:
    ///
    ///  * [`ScrollPositionAlignmentPolicy`] for the way in which `alignment` is
    ///    applied, and the way the given `object` is aligned.
    #[allow(clippy::too_many_arguments)]
    fn ensure_visible(
        self: Handle<Self>,
        app: &mut App,
        object: AnyRenderObject,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
        target_render_object: Option<AnyRenderObject>,
    ) -> CompleterFuture<()> {
        ScrollPositionBase::ensure_visible(
            self,
            app,
            object,
            alignment,
            duration,
            curve,
            alignment_policy,
            target_render_object,
        )
    }

    /// This notifier's value is true if a scroll is underway and false if the scroll
    /// position is idle.
    ///
    /// Listeners added by stateful widgets should be removed in the widget's
    /// `State::dispose` method.
    fn is_scrolling_notifier(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<bool>> {
        self.scroll_position_data(app).is_scrolling_notifier
    }

    /// Changes the scrolling position based on a pointer signal from current
    /// value to delta without animation and without checking if new value is in
    /// range, taking min/max scroll extent into account.
    ///
    /// Any active animation is canceled. If the user is currently scrolling, that
    /// action is canceled.
    ///
    /// This method dispatches the start/update/end sequence of scrolling
    /// notifications.
    ///
    /// This method is very similar to [`ViewportOffset::jump_to`], but
    /// [`pointer_scroll`](Self::pointer_scroll) will update the [`ScrollDirection`].
    fn pointer_scroll(self: Handle<Self>, app: &mut App, delta: f64);

    /// Deprecated. Use [`ViewportOffset::jump_to`] or a custom [`ScrollPosition`] instead.
    #[deprecated(
        note = "This method bypasses scroll activity management and can cause inconsistent \
                layouts or scrolling behavior. Use jump_to or a custom ScrollPosition instead."
    )]
    fn jump_to_without_settling(self: Handle<Self>, app: &mut App, value: f64);

    /// Stop the current activity and start a [`crate::HoldScrollActivity`].
    fn hold(
        self: Handle<Self>,
        app: &mut App,
        hold_cancel_callback: Listener,
    ) -> Rc<dyn ScrollHoldController>;

    /// Start a drag activity corresponding to the given [`DragStartDetails`].
    ///
    /// The `drag_cancel_callback` argument will be invoked if the drag is ended
    /// prematurely (e.g. from another activity taking over). See
    /// [`crate::ScrollDragController::on_drag_canceled`] for details.
    fn drag(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag>;

    /// The currently operative [`AnyScrollActivity`].
    ///
    /// If the scroll position is not performing any more specific activity, the
    /// activity will be an [`crate::IdleScrollActivity`]. To determine whether the scroll
    /// position is idle, check the [`is_scrolling_notifier`](Self::is_scrolling_notifier).
    ///
    /// Call [`begin_activity`](Self::begin_activity) to change the current activity.
    fn activity(self: Handle<Self>, app: &App) -> Option<AnyScrollActivity> {
        self.scroll_position_data(app).activity
    }

    /// Change the current [`activity`](Self::activity), disposing of the old one and
    /// sending scroll notifications as necessary.
    ///
    /// If the argument is `None`, this method has no effect. This is convenient for
    /// cases where the new activity is obtained from another method, and that
    /// method might return `None`.
    fn begin_activity(self: Handle<Self>, app: &mut App, new_activity: Option<AnyScrollActivity>) {
        ScrollPositionBase::begin_activity(self, app, new_activity);
    }

    /// Called by [`begin_activity`](Self::begin_activity) to report when an activity has
    /// started.
    fn did_start_scroll(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::did_start_scroll(self, app);
    }

    /// Called by [`set_pixels`](Self::set_pixels) to report a change to the
    /// [`pixels`](ViewportOffset::pixels) position.
    fn did_update_scroll_position_by(self: Handle<Self>, app: &mut App, delta: f64) {
        ScrollPositionBase::did_update_scroll_position_by(self, app, delta);
    }

    /// Called by [`begin_activity`](Self::begin_activity) to report when an activity has
    /// ended.
    ///
    /// This also saves the scroll offset using
    /// [`save_scroll_offset`](Self::save_scroll_offset).
    fn did_end_scroll(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::did_end_scroll(self, app);
    }

    /// Called by [`set_pixels`](Self::set_pixels) to report overscroll when an attempt is made
    /// to change the [`pixels`](ViewportOffset::pixels) position. Overscroll is the amount of
    /// change that was not applied to the [`pixels`](ViewportOffset::pixels) value.
    fn did_overscroll_by(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionBase::did_overscroll_by(self, app, value);
    }

    /// Dispatches a notification that the [`ViewportOffset::user_scroll_direction`] has
    /// changed.
    ///
    /// Implementors should call this function when they change
    /// [`ViewportOffset::user_scroll_direction`].
    fn did_update_scroll_direction(self: Handle<Self>, app: &mut App, direction: ScrollDirection) {
        ScrollPositionBase::did_update_scroll_direction(self, app, direction);
    }

    /// Dispatches a notification that the [`ScrollMetrics`] have changed.
    fn did_update_scroll_metrics(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::did_update_scroll_metrics(self, app);
    }

    /// Provides a heuristic to determine if expensive frame-bound tasks should be
    /// deferred.
    ///
    /// The actual work of this is delegated to the [`physics`](Self::physics) via
    /// [`crate::ScrollPhysics::recommend_deferred_loading`] called with the current
    /// [`activity`](Self::activity)'s [`AnyScrollActivity::velocity`].
    ///
    /// Returning true from this method indicates that the [`crate::ScrollPhysics`]
    /// evaluate the current scroll velocity to be great enough that expensive
    /// operations impacting the UI should be deferred.
    fn recommend_deferred_loading(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> bool {
        ScrollPositionBase::recommend_deferred_loading(self, app, context)
    }

    /// Discards any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App) {
        ScrollPositionBase::dispose(self, app);
    }

    /// Add additional information to the given `description` for use by
    /// [`describe`](Self::describe).
    ///
    /// An override calls [`ScrollPositionBase::debug_fill_description`] where Dart writes
    /// `super.debugFillDescription(description)`.
    fn debug_fill_description(self: Handle<Self>, app: &App, description: &mut Vec<String>) {
        ScrollPositionBase::debug_fill_description(self, app, description);
    }

    /// Dart's `toString`, which reads the arena and so cannot be [`Debug`].
    fn describe(self: Handle<Self>, app: &App) -> String {
        let mut description = Vec::new();
        ScrollPosition::debug_fill_description(self, app, &mut description);
        format!(
            "{}({})",
            describe_identity::<Self>(self.id()),
            description.join(", ")
        )
    }

    /// This position as the erased [`AnyScrollPosition`] — what to pass where a Dart API takes
    /// a `ScrollPosition`.
    fn as_scroll_position(self: Handle<Self>) -> AnyScrollPosition {
        AnyScrollPosition {
            id: self.id(),
            vtable: const { &ScrollPositionVTable::of::<Self>() },
        }
    }
}

/// Dart's `describeIdentity`: the runtime type and a short identity hash.
fn describe_identity<T: ?Sized>(id: HandleId) -> String {
    let name = std::any::type_name::<T>();
    let short = name.rsplit("::").next().unwrap_or(name);
    format!("{short}#{id:?}")
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

/// The shared bodies of Dart's `ScrollPosition`: call one where Dart writes `super.…`.
pub struct ScrollPositionBase;

impl ScrollPositionBase {
    /// Dart's `ScrollPosition` constructor body: absorb the old position, then restore the
    /// saved scroll offset.
    ///
    /// A leaf's constructor calls this right after `App::create`, where Dart's superclass
    /// constructor would run.
    pub fn init<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        old_position: Option<AnyScrollPosition>,
    ) {
        if let Some(old_position) = old_position {
            P::absorb(this, app, old_position);
        }
        if this.keep_scroll_offset(app) {
            P::restore_scroll_offset(this, app);
        }
    }

    /// See [`ViewportOffset::pixels`], which a [`ScrollPosition`] answers from its bag.
    pub fn pixels<P: ScrollPosition>(this: Handle<P>, app: &App) -> f64 {
        this.scroll_position_data(app)
            .pixels
            .expect("pixels was read before it was available")
    }

    /// See [`ViewportOffset::has_pixels`], which a [`ScrollPosition`] answers from its bag.
    pub fn has_pixels<P: ScrollPosition>(this: Handle<P>, app: &App) -> bool {
        this.scroll_position_data(app).pixels.is_some()
    }

    /// See [`ScrollPosition::absorb`].
    pub fn absorb<P: ScrollPosition>(this: Handle<P>, app: &mut App, other: AnyScrollPosition) {
        debug_assert!(Rc::ptr_eq(&other.context(app), &this.context(app)));
        debug_assert!(this.scroll_position_data(app).pixels.is_none());
        if other.has_content_dimensions(app) {
            let (min, max) = (other.min_scroll_extent(app), other.max_scroll_extent(app));
            let data = this.scroll_position_data_mut(app);
            data.min_scroll_extent = Some(min);
            data.max_scroll_extent = Some(max);
        }
        if other.has_pixels(app) {
            let pixels = other.pixels(app);
            this.scroll_position_data_mut(app).pixels = Some(pixels);
        }
        if other.has_viewport_dimension(app) {
            let viewport_dimension = other.viewport_dimension(app);
            this.scroll_position_data_mut(app).viewport_dimension = Some(viewport_dimension);
        }

        debug_assert!(this.activity(app).is_none());
        debug_assert!(other.activity(app).is_some());
        let activity = other
            .activity(app)
            .expect("the old position has an activity");
        this.scroll_position_data_mut(app).activity = Some(activity);
        other.take_activity(app);
        if other.type_id() != TypeId::of::<P>() {
            activity.reset_activity(app);
        }
        let should_ignore_pointer = activity.should_ignore_pointer(app);
        this.context(app)
            .set_ignore_pointer(app, should_ignore_pointer);
        let is_scrolling = activity.is_scrolling(app);
        this.is_scrolling_notifier(app).set_value(app, is_scrolling);
    }

    /// See [`ScrollPosition::set_pixels`].
    pub fn set_pixels<P: ScrollPosition>(this: Handle<P>, app: &mut App, new_pixels: f64) -> f64 {
        debug_assert!(this.has_pixels(app));
        if cfg!(debug_assertions) {
            assert!(
                SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks,
                "A scrollable's position should not change during the build, layout, and paint \
                 phases, otherwise the rendering will be confused."
            );
        }
        let pixels = this.pixels(app);
        if new_pixels != pixels {
            let overscroll = P::apply_boundary_conditions(this, app, new_pixels);
            debug_assert!(
                overscroll.abs() <= (new_pixels - pixels).abs(),
                "applyBoundaryConditions returned invalid overscroll value."
            );
            let old_pixels = pixels;
            this.scroll_position_data_mut(app).pixels = Some(new_pixels - overscroll);
            if this.pixels(app) != old_pixels {
                if this.out_of_range(app) {
                    this.context(app).set_ignore_pointer(app, false);
                }
                this.notify_listeners(app);
                let delta = this.pixels(app) - old_pixels;
                P::did_update_scroll_position_by(this, app, delta);
            }
            if overscroll.abs() > PRECISION_ERROR_TOLERANCE {
                P::did_overscroll_by(this, app, overscroll);
                return overscroll;
            }
        }
        0.0
    }

    /// See [`ViewportOffset::correct_by`], which a [`ScrollPosition`] overrides.
    pub fn correct_by<P: ScrollPosition>(this: Handle<P>, app: &mut App, correction: f64) {
        debug_assert!(
            this.has_pixels(app),
            "An initial pixels value must exist by calling correctPixels on the ScrollPosition"
        );
        let data = this.scroll_position_data_mut(app);
        data.pixels = Some(data.pixels.expect("an initial pixels value") + correction);
        data.did_change_viewport_dimension_or_receive_correction = true;
    }

    /// See [`ScrollPosition::force_pixels`].
    pub fn force_pixels<P: ScrollPosition>(this: Handle<P>, app: &mut App, value: f64) {
        debug_assert!(this.has_pixels(app));
        let implied_velocity = value - this.pixels(app);
        let data = this.scroll_position_data_mut(app);
        data.implied_velocity = implied_velocity;
        data.pixels = Some(value);
        this.notify_listeners(app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::handle_method(this, reset_implied_velocity::<P>),
        );
    }

    /// See [`ScrollPosition::save_scroll_offset`].
    pub fn save_scroll_offset<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        let storage_context = this.context(app).storage_context(app);
        if let Some(bucket) = PageStorage::maybe_of(app, storage_context) {
            let pixels = this.pixels(app);
            bucket.write_state(app, storage_context, Rc::new(pixels), None);
        }
    }

    /// See [`ScrollPosition::restore_scroll_offset`].
    pub fn restore_scroll_offset<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        if !this.has_pixels(app) {
            let storage_context = this.context(app).storage_context(app);
            let value = PageStorage::maybe_of(app, storage_context)
                .and_then(|bucket| bucket.read_state(app, storage_context, None))
                .and_then(|data| data.downcast_ref::<f64>().copied());
            if let Some(value) = value {
                P::correct_pixels(this, app, value);
            }
        }
    }

    /// See [`ScrollPosition::restore_offset`].
    pub fn restore_offset<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        offset: f64,
        initial_restore: bool,
    ) {
        if initial_restore {
            P::correct_pixels(this, app, offset);
        } else {
            P::jump_to(this, app, offset);
        }
    }

    /// See [`ScrollPosition::save_offset`].
    pub fn save_offset<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        debug_assert!(this.has_pixels(app));
        let pixels = this.pixels(app);
        this.context(app).save_offset(app, pixels);
    }

    /// See [`ScrollPosition::apply_boundary_conditions`].
    pub fn apply_boundary_conditions<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        value: f64,
    ) -> f64 {
        let physics = this.physics(app);
        let metrics = this.copy_with(app);
        let result = physics.apply_boundary_conditions(&metrics, value);
        debug_assert!(
            result.abs() <= (value - this.pixels(app)).abs(),
            "{physics:?}.applyBoundaryConditions returned invalid overscroll value. The \
             applyBoundaryConditions method is only supposed to reduce the possible range of \
             movement, not increase it."
        );
        result
    }

    /// See [`ViewportOffset::apply_viewport_dimension`], which a [`ScrollPosition`] overrides.
    pub fn apply_viewport_dimension<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        viewport_dimension: f64,
    ) -> bool {
        let data = this.scroll_position_data_mut(app);
        if data.viewport_dimension != Some(viewport_dimension) {
            data.viewport_dimension = Some(viewport_dimension);
            data.did_change_viewport_dimension_or_receive_correction = true;
            // If this is called, you can rely on `apply_content_dimensions` being called soon
            // afterwards in the same layout phase. So we put all the logic that relies on both
            // values being computed into `apply_content_dimensions`.
        }
        true
    }

    /// Dart's `_isMetricsChanged`.
    fn is_metrics_changed<P: ScrollPosition>(this: Handle<P>, app: &App) -> bool {
        debug_assert!(this.have_dimensions(app));
        let current_metrics = this.copy_with(app);
        let Some(last_metrics) = this.scroll_position_data(app).last_metrics else {
            return true;
        };
        !(current_metrics.extent_before() == last_metrics.extent_before()
            && current_metrics.extent_inside() == last_metrics.extent_inside()
            && current_metrics.extent_after() == last_metrics.extent_after()
            && current_metrics.axis_direction() == last_metrics.axis_direction())
    }

    /// See [`ViewportOffset::apply_content_dimensions`], which a [`ScrollPosition`] overrides.
    pub fn apply_content_dimensions<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool {
        debug_assert!(
            this.have_dimensions(app) == this.scroll_position_data(app).last_metrics.is_some()
        );
        let data = this.scroll_position_data(app);
        let distance = Tolerance::DEFAULT_TOLERANCE.distance;
        if !near_equal(data.min_scroll_extent, Some(min_scroll_extent), distance)
            || !near_equal(data.max_scroll_extent, Some(max_scroll_extent), distance)
            || data.did_change_viewport_dimension_or_receive_correction
            || data.last_axis != Some(this.axis(app))
        {
            debug_assert!(min_scroll_extent <= max_scroll_extent);
            let axis = this.axis(app);
            let data = this.scroll_position_data_mut(app);
            data.min_scroll_extent = Some(min_scroll_extent);
            data.max_scroll_extent = Some(max_scroll_extent);
            data.last_axis = Some(axis);
            let current_metrics = this.have_dimensions(app).then(|| this.copy_with(app));
            let last_metrics = this.scroll_position_data(app).last_metrics;
            let data = this.scroll_position_data_mut(app);
            data.did_change_viewport_dimension_or_receive_correction = false;
            data.pending_dimensions = true;
            if this.have_dimensions(app) {
                let last_metrics = last_metrics.expect("dimensions imply the last metrics");
                let current_metrics = current_metrics.expect("dimensions imply current metrics");
                if !P::correct_for_new_dimensions(this, app, &last_metrics, &current_metrics) {
                    return false;
                }
            }
            this.scroll_position_data_mut(app).have_dimensions = true;
        }
        debug_assert!(this.have_dimensions(app));
        if this.scroll_position_data(app).pending_dimensions {
            P::apply_new_dimensions(this, app);
            this.scroll_position_data_mut(app).pending_dimensions = false;
        }
        debug_assert!(
            !this
                .scroll_position_data(app)
                .did_change_viewport_dimension_or_receive_correction,
            "Use correct_for_new_dimensions() (and return true) to change the scroll offset \
             during apply_content_dimensions()."
        );

        if Self::is_metrics_changed(this, app) {
            // It is too late to send useful notifications, because the potential listeners
            // have, by definition, already been built this frame. To make sure the
            // notification is sent at all, we delay it until after the frame is complete.
            if !this
                .scroll_position_data(app)
                .have_scheduled_update_notification
            {
                app.schedule_microtask(Listener::handle_method(this, P::did_update_scroll_metrics));
                this.scroll_position_data_mut(app)
                    .have_scheduled_update_notification = true;
            }
            let metrics = this.copy_with(app);
            this.scroll_position_data_mut(app).last_metrics = Some(metrics);
        }
        true
    }

    /// See [`ScrollPosition::correct_for_new_dimensions`].
    pub fn correct_for_new_dimensions<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        old_position: &dyn ScrollMetrics,
        new_position: &dyn ScrollMetrics,
    ) -> bool {
        let activity = this.activity(app).expect("a position has an activity");
        let is_scrolling = activity.is_scrolling(app);
        let velocity = activity.velocity(app);
        let physics = this.physics(app);
        let new_pixels = physics.adjust_position_for_new_dimensions(
            old_position,
            new_position,
            is_scrolling,
            velocity,
        );
        if new_pixels != this.pixels(app) {
            P::correct_pixels(this, app, new_pixels);
            return false;
        }
        true
    }

    /// See [`ScrollPosition::apply_new_dimensions`].
    pub fn apply_new_dimensions<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        debug_assert!(this.has_pixels(app));
        debug_assert!(this.scroll_position_data(app).pending_dimensions);
        this.activity(app)
            .expect("a position has an activity")
            .apply_new_dimensions(app);
    }

    /// Dart's `_maybeFlipAlignment`.
    fn maybe_flip_alignment(
        alignment_policy: ScrollPositionAlignmentPolicy,
    ) -> ScrollPositionAlignmentPolicy {
        match alignment_policy {
            // Don't flip when explicit.
            ScrollPositionAlignmentPolicy::Explicit => alignment_policy,
            ScrollPositionAlignmentPolicy::KeepVisibleAtEnd => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart
            }
            ScrollPositionAlignmentPolicy::KeepVisibleAtStart => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
            }
        }
    }

    /// Dart's `_applyAxisDirectionToAlignmentPolicy`.
    fn apply_axis_direction_to_alignment_policy<P: ScrollPosition>(
        this: Handle<P>,
        app: &App,
        alignment_policy: ScrollPositionAlignmentPolicy,
    ) -> ScrollPositionAlignmentPolicy {
        match this.axis_direction(app) {
            // Start and end alignments must account for axis direction.
            // When focus is requested for example, it knows the directionality of the
            // keyboard keys initiating traversal, but not the direction of the `Scrollable`.
            AxisDirection::Up | AxisDirection::Left => Self::maybe_flip_alignment(alignment_policy),
            AxisDirection::Down | AxisDirection::Right => alignment_policy,
        }
    }

    /// See [`ScrollPosition::ensure_visible`].
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_visible<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        object: AnyRenderObject,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
        target_render_object: Option<AnyRenderObject>,
    ) -> CompleterFuture<()> {
        debug_assert!(object.attached(app));
        // If no viewport is found, return.
        let Some(viewport) = AnyRenderAbstractViewport::maybe_of(app, Some(object)) else {
            return CompleterFuture::ready(());
        };

        let mut target_rect: Option<Rect> = None;
        if let Some(target) = target_render_object
            && target != object
        {
            target_rect = Some(transform_rect(
                &target.get_transform_to(app, Some(object)),
                object.paint_bounds(app).intersect(target.paint_bounds(app)),
            ));
        }

        let axis = this.axis(app);
        let (min, max) = (this.min_scroll_extent(app), this.max_scroll_extent(app));
        let target =
            match Self::apply_axis_direction_to_alignment_policy(this, app, alignment_policy) {
                ScrollPositionAlignmentPolicy::Explicit => clamp_double(
                    viewport
                        .get_offset_to_reveal(app, object, alignment, target_rect, Some(axis))
                        .offset,
                    min,
                    max,
                ),
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd => {
                    // Aligns to end.
                    let revealed = clamp_double(
                        viewport
                            .get_offset_to_reveal(app, object, 1.0, target_rect, Some(axis))
                            .offset,
                        min,
                        max,
                    );
                    revealed.max(this.pixels(app))
                }
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart => {
                    // Aligns to start.
                    let revealed = clamp_double(
                        viewport
                            .get_offset_to_reveal(app, object, 0.0, target_rect, Some(axis))
                            .offset,
                        min,
                        max,
                    );
                    revealed.min(this.pixels(app))
                }
            };

        if target == this.pixels(app) {
            return CompleterFuture::ready(());
        }

        if duration.is_zero() {
            P::jump_to(this, app, target);
            return CompleterFuture::ready(());
        }

        P::animate_to(this, app, target, duration, curve)
    }

    /// See [`ViewportOffset::move_to`], which a [`ScrollPosition`] overrides.
    pub fn move_to<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        let clamp = clamp.unwrap_or(true);
        let to = if clamp {
            clamp_double(to, this.min_scroll_extent(app), this.max_scroll_extent(app))
        } else {
            to
        };
        reveal_rendering::ViewportOffsetBase::move_to(this, app, to, duration, curve, Some(clamp))
    }

    /// See [`ViewportOffset::allow_implicit_scrolling`], which a [`ScrollPosition`] overrides.
    pub fn allow_implicit_scrolling<P: ScrollPosition>(this: Handle<P>, app: &App) -> bool {
        this.scroll_position_data(app)
            .physics
            .allow_implicit_scrolling()
    }

    /// See [`ScrollPosition::begin_activity`].
    pub fn begin_activity<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        new_activity: Option<AnyScrollActivity>,
    ) {
        let Some(new_activity) = new_activity else {
            return;
        };
        let was_scrolling;
        let old_ignore_pointer;
        if let Some(activity) = this.activity(app) {
            old_ignore_pointer = activity.should_ignore_pointer(app);
            was_scrolling = activity.is_scrolling(app);
            if was_scrolling && !new_activity.is_scrolling(app) {
                // Notifies and then saves the scroll offset.
                P::did_end_scroll(this, app);
            }
            activity.dispose(app);
        } else {
            old_ignore_pointer = false;
            was_scrolling = false;
        }
        this.scroll_position_data_mut(app).activity = Some(new_activity);
        let should_ignore_pointer = new_activity.should_ignore_pointer(app);
        if old_ignore_pointer != should_ignore_pointer {
            this.context(app)
                .set_ignore_pointer(app, should_ignore_pointer);
        }
        let is_scrolling = new_activity.is_scrolling(app);
        this.is_scrolling_notifier(app).set_value(app, is_scrolling);
        if !was_scrolling && is_scrolling {
            P::did_start_scroll(this, app);
        }
    }

    /// See [`ScrollPosition::did_start_scroll`].
    pub fn did_start_scroll<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
        let context = this.context(app).notification_context(app);
        this.activity(app)
            .expect("a position has an activity")
            .dispatch_scroll_start_notification(app, metrics, context);
    }

    /// See [`ScrollPosition::did_update_scroll_position_by`].
    pub fn did_update_scroll_position_by<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        delta: f64,
    ) {
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
        let context = notification_context(this, app);
        this.activity(app)
            .expect("a position has an activity")
            .dispatch_scroll_update_notification(app, metrics, context, delta);
    }

    /// See [`ScrollPosition::did_end_scroll`].
    pub fn did_end_scroll<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
        let context = notification_context(this, app);
        this.activity(app)
            .expect("a position has an activity")
            .dispatch_scroll_end_notification(app, metrics, context);
        P::save_offset(this, app);
        if this.keep_scroll_offset(app) {
            P::save_scroll_offset(this, app);
        }
    }

    /// See [`ScrollPosition::did_overscroll_by`].
    pub fn did_overscroll_by<P: ScrollPosition>(this: Handle<P>, app: &mut App, value: f64) {
        let activity = this.activity(app).expect("a position has an activity");
        debug_assert!(activity.is_scrolling(app));
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
        let context = notification_context(this, app);
        activity.dispatch_overscroll_notification(app, metrics, context, value);
    }

    /// See [`ScrollPosition::did_update_scroll_direction`].
    pub fn did_update_scroll_direction<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        direction: ScrollDirection,
    ) {
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
        let context = notification_context(this, app);
        UserScrollNotification::new(metrics, context, direction).dispatch(app, Some(context));
    }

    /// See [`ScrollPosition::did_update_scroll_metrics`].
    pub fn did_update_scroll_metrics<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        if cfg!(debug_assertions) {
            assert!(SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks);
        }
        debug_assert!(
            this.scroll_position_data(app)
                .have_scheduled_update_notification
        );
        this.scroll_position_data_mut(app)
            .have_scheduled_update_notification = false;
        if let Some(context) = this.context(app).notification_context(app) {
            let metrics: Rc<dyn ScrollMetrics> = Rc::new(this.copy_with(app));
            ScrollMetricsNotification::new(metrics, context).dispatch(app, Some(context));
        }
    }

    /// See [`ScrollPosition::recommend_deferred_loading`].
    pub fn recommend_deferred_loading<P: ScrollPosition>(
        this: Handle<P>,
        app: &mut App,
        context: BuildContext,
    ) -> bool {
        let activity = this.activity(app).expect("a position has an activity");
        let velocity = activity.velocity(app) + this.scroll_position_data(app).implied_velocity;
        let metrics = this.copy_with(app);
        let physics = this.physics(app);
        physics.recommend_deferred_loading(app, velocity, &metrics, context)
    }

    /// See [`ScrollPosition::dispose`].
    pub fn dispose<P: ScrollPosition>(this: Handle<P>, app: &mut App) {
        // The activity will be `None` if it got absorbed by another `ScrollPosition`.
        if let Some(activity) = this.activity(app) {
            activity.dispose(app);
        }
        this.scroll_position_data_mut(app).activity = None;
        let is_scrolling_notifier = this.is_scrolling_notifier(app);
        app.get_mut(is_scrolling_notifier).dispose();
        app.get_mut(this).change_notifier_data_mut().dispose();
    }

    /// See [`ScrollPosition::debug_fill_description`].
    pub fn debug_fill_description<P: ScrollPosition>(
        this: Handle<P>,
        app: &App,
        description: &mut Vec<String>,
    ) {
        if let Some(debug_label) = this.debug_label(app) {
            description.push(debug_label.to_string());
        }
        ViewportOffset::debug_fill_description(this, app, description);
        let data = this.scroll_position_data(app);
        description.push(format!(
            "range: {}..{}",
            format_optional(data.min_scroll_extent),
            format_optional(data.max_scroll_extent)
        ));
        description.push(format!(
            "viewport: {}",
            format_optional(data.viewport_dimension)
        ));
    }
}

/// Dart's `'${value?.toStringAsFixed(1)}'`, which prints `null` for a missing value.
fn format_optional(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_string(), |value| format!("{value:.1}"))
}

/// Dart's `context.notificationContext!`.
fn notification_context<P: ScrollPosition>(this: Handle<P>, app: &mut App) -> BuildContext {
    this.context(app).notification_context(app).expect(
        "a ScrollPosition dispatches its notifications from the ScrollContext's notification \
         context, which is only available once the Scrollable has built",
    )
}

/// The post-frame callback [`ScrollPositionBase::force_pixels`] schedules; Dart's
/// `'ScrollPosition.resetVelocity'`.
fn reset_implied_velocity<P: ScrollPosition>(
    this: Handle<P>,
    app: &mut App,
    _time_stamp: Duration,
) {
    this.scroll_position_data_mut(app).implied_velocity = 0.0;
}

/// The dispatch signature of [`ViewportOffset::animate_to`].
type AnimateToFn = fn(&mut App, HandleId, f64, Duration, Rc<dyn Curve>) -> CompleterFuture<()>;

/// The dispatch signature of [`ViewportOffset::move_to`].
type MoveToFn = fn(
    &mut App,
    HandleId,
    f64,
    Option<Duration>,
    Option<Rc<dyn Curve>>,
    Option<bool>,
) -> CompleterFuture<()>;

/// The dispatch signature of [`ScrollPosition::ensure_visible`].
type EnsureVisibleFn = fn(
    &mut App,
    HandleId,
    AnyRenderObject,
    f64,
    Duration,
    Rc<dyn Curve>,
    ScrollPositionAlignmentPolicy,
    Option<AnyRenderObject>,
) -> CompleterFuture<()>;

/// The vtable of an erased [`AnyScrollPosition`]: one `&'static` table per concrete
/// [`ScrollPosition`] type.
struct ScrollPositionVTable {
    type_id: fn() -> TypeId,
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    as_viewport_offset: fn(HandleId) -> AnyViewportOffset,
    physics: fn(&App, HandleId) -> ScrollPhysicsRef,
    context: fn(&App, HandleId) -> Rc<dyn ScrollContext>,
    keep_scroll_offset: fn(&App, HandleId) -> bool,
    debug_label: fn(&App, HandleId) -> Option<String>,
    min_scroll_extent: fn(&App, HandleId) -> f64,
    max_scroll_extent: fn(&App, HandleId) -> f64,
    has_content_dimensions: fn(&App, HandleId) -> bool,
    pixels: fn(&App, HandleId) -> f64,
    has_pixels: fn(&App, HandleId) -> bool,
    viewport_dimension: fn(&App, HandleId) -> f64,
    has_viewport_dimension: fn(&App, HandleId) -> bool,
    axis_direction: fn(&App, HandleId) -> AxisDirection,
    device_pixel_ratio: fn(&App, HandleId) -> f64,
    copy_with: fn(&App, HandleId) -> FixedScrollMetrics,
    have_dimensions: fn(&App, HandleId) -> bool,
    should_ignore_pointer: fn(&App, HandleId) -> bool,
    absorb: fn(&mut App, HandleId, AnyScrollPosition),
    take_activity: fn(&mut App, HandleId),
    set_pixels: fn(&mut App, HandleId, f64) -> f64,
    correct_pixels: fn(&mut App, HandleId, f64),
    correct_by: fn(&mut App, HandleId, f64),
    apply_viewport_dimension: fn(&mut App, HandleId, f64) -> bool,
    apply_content_dimensions: fn(&mut App, HandleId, f64, f64) -> bool,
    ensure_visible: EnsureVisibleFn,
    is_scrolling_notifier: fn(&App, HandleId) -> Handle<ValueNotifier<bool>>,
    animate_to: AnimateToFn,
    jump_to: fn(&mut App, HandleId, f64),
    pointer_scroll: fn(&mut App, HandleId, f64),
    move_to: MoveToFn,
    user_scroll_direction: fn(&App, HandleId) -> ScrollDirection,
    allow_implicit_scrolling: fn(&App, HandleId) -> bool,
    jump_to_without_settling: fn(&mut App, HandleId, f64),
    hold: fn(&mut App, HandleId, Listener) -> Rc<dyn ScrollHoldController>,
    drag: fn(&mut App, HandleId, DragStartDetails, Listener) -> Rc<dyn Drag>,
    restore_offset: fn(&mut App, HandleId, f64, bool),
    activity: fn(&App, HandleId) -> Option<AnyScrollActivity>,
    begin_activity: fn(&mut App, HandleId, Option<AnyScrollActivity>),
    did_start_scroll: fn(&mut App, HandleId),
    did_update_scroll_position_by: fn(&mut App, HandleId, f64),
    did_end_scroll: fn(&mut App, HandleId),
    did_overscroll_by: fn(&mut App, HandleId, f64),
    did_update_scroll_direction: fn(&mut App, HandleId, ScrollDirection),
    did_update_scroll_metrics: fn(&mut App, HandleId),
    recommend_deferred_loading: fn(&mut App, HandleId, BuildContext) -> bool,
    dispose: fn(&mut App, HandleId),
    describe: fn(&App, HandleId) -> String,
}

impl ScrollPositionVTable {
    /// The table for one concrete position type.
    #[allow(deprecated)]
    const fn of<P: ScrollPosition>() -> ScrollPositionVTable {
        ScrollPositionVTable {
            type_id: TypeId::of::<P>,
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<P>(id), app, listener);
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<P>(id), app, listener);
            },
            as_viewport_offset: |id| P::as_viewport_offset(resolve(id)),
            physics: |app, id| P::physics(resolve(id), app),
            context: |app, id| P::context(resolve(id), app),
            keep_scroll_offset: |app, id| P::keep_scroll_offset(resolve(id), app),
            debug_label: |app, id| P::debug_label(resolve(id), app).map(str::to_string),
            min_scroll_extent: |app, id| P::min_scroll_extent(resolve(id), app),
            max_scroll_extent: |app, id| P::max_scroll_extent(resolve(id), app),
            has_content_dimensions: |app, id| P::has_content_dimensions(resolve(id), app),
            pixels: |app, id| P::pixels(resolve(id), app),
            has_pixels: |app, id| P::has_pixels(resolve(id), app),
            viewport_dimension: |app, id| P::viewport_dimension(resolve(id), app),
            has_viewport_dimension: |app, id| P::has_viewport_dimension(resolve(id), app),
            axis_direction: |app, id| P::axis_direction(resolve(id), app),
            device_pixel_ratio: |app, id| P::device_pixel_ratio(resolve(id), app),
            copy_with: |app, id| P::copy_with(resolve(id), app),
            have_dimensions: |app, id| P::have_dimensions(resolve(id), app),
            should_ignore_pointer: |app, id| P::should_ignore_pointer(resolve(id), app),
            absorb: |app, id, other| P::absorb(resolve(id), app, other),
            take_activity: |app, id| {
                P::scroll_position_data_mut(resolve::<P>(id), app).activity = None;
            },
            set_pixels: |app, id, pixels| P::set_pixels(resolve(id), app, pixels),
            correct_pixels: |app, id, value| P::correct_pixels(resolve(id), app, value),
            correct_by: |app, id, correction| P::correct_by(resolve(id), app, correction),
            apply_viewport_dimension: |app, id, dimension| {
                P::apply_viewport_dimension(resolve(id), app, dimension)
            },
            apply_content_dimensions: |app, id, min, max| {
                P::apply_content_dimensions(resolve(id), app, min, max)
            },
            ensure_visible: |app, id, object, alignment, duration, curve, policy, target| {
                P::ensure_visible(
                    resolve(id),
                    app,
                    object,
                    alignment,
                    duration,
                    curve,
                    policy,
                    target,
                )
            },
            is_scrolling_notifier: |app, id| P::is_scrolling_notifier(resolve(id), app),
            animate_to: |app, id, to, duration, curve| {
                P::animate_to(resolve(id), app, to, duration, curve)
            },
            jump_to: |app, id, value| P::jump_to(resolve(id), app, value),
            pointer_scroll: |app, id, delta| P::pointer_scroll(resolve(id), app, delta),
            move_to: |app, id, to, duration, curve, clamp| {
                P::move_to(resolve(id), app, to, duration, curve, clamp)
            },
            user_scroll_direction: |app, id| P::user_scroll_direction(resolve(id), app),
            allow_implicit_scrolling: |app, id| P::allow_implicit_scrolling(resolve(id), app),
            jump_to_without_settling: |app, id, value| {
                P::jump_to_without_settling(resolve(id), app, value);
            },
            hold: |app, id, callback| P::hold(resolve(id), app, callback),
            drag: |app, id, details, callback| P::drag(resolve(id), app, details, callback),
            restore_offset: |app, id, offset, initial| {
                P::restore_offset(resolve(id), app, offset, initial);
            },
            activity: |app, id| P::activity(resolve(id), app),
            begin_activity: |app, id, activity| P::begin_activity(resolve(id), app, activity),
            did_start_scroll: |app, id| P::did_start_scroll(resolve(id), app),
            did_update_scroll_position_by: |app, id, delta| {
                P::did_update_scroll_position_by(resolve(id), app, delta);
            },
            did_end_scroll: |app, id| P::did_end_scroll(resolve(id), app),
            did_overscroll_by: |app, id, value| P::did_overscroll_by(resolve(id), app, value),
            did_update_scroll_direction: |app, id, direction| {
                P::did_update_scroll_direction(resolve(id), app, direction);
            },
            did_update_scroll_metrics: |app, id| P::did_update_scroll_metrics(resolve(id), app),
            recommend_deferred_loading: |app, id, context| {
                P::recommend_deferred_loading(resolve(id), app, context)
            },
            dispose: |app, id| P::dispose(resolve(id), app),
            describe: |app, id| P::describe(resolve(id), app),
        }
    }
}

/// Erased [`ScrollPosition`]: what a field or parameter Dart types as `ScrollPosition`
/// becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyScrollPosition {
    id: HandleId,
    vtable: &'static ScrollPositionVTable,
}

impl AnyScrollPosition {
    /// Dart's `position.runtimeType`.
    pub fn type_id(self) -> TypeId {
        (self.vtable.type_id)()
    }

    /// The typed handle, when this position is a `P`; Dart's `position is P`.
    pub fn downcast<P: ScrollPosition>(self, app: &App) -> Option<Handle<P>> {
        app.handle::<P>(self.id)
    }

    /// This position as an erased [`ViewportOffset`].
    pub fn as_viewport_offset(self) -> AnyViewportOffset {
        (self.vtable.as_viewport_offset)(self.id)
    }

    /// See [`ScrollPosition::physics`].
    pub fn physics(self, app: &App) -> ScrollPhysicsRef {
        (self.vtable.physics)(app, self.id)
    }

    /// See [`ScrollPosition::context`].
    pub fn context(self, app: &App) -> Rc<dyn ScrollContext> {
        (self.vtable.context)(app, self.id)
    }

    /// See [`ScrollPosition::keep_scroll_offset`].
    pub fn keep_scroll_offset(self, app: &App) -> bool {
        (self.vtable.keep_scroll_offset)(app, self.id)
    }

    /// See [`ScrollPosition::debug_label`].
    pub fn debug_label(self, app: &App) -> Option<String> {
        (self.vtable.debug_label)(app, self.id)
    }

    /// See [`ScrollMetrics::min_scroll_extent`].
    pub fn min_scroll_extent(self, app: &App) -> f64 {
        (self.vtable.min_scroll_extent)(app, self.id)
    }

    /// See [`ScrollMetrics::max_scroll_extent`].
    pub fn max_scroll_extent(self, app: &App) -> f64 {
        (self.vtable.max_scroll_extent)(app, self.id)
    }

    /// See [`ScrollMetrics::has_content_dimensions`].
    pub fn has_content_dimensions(self, app: &App) -> bool {
        (self.vtable.has_content_dimensions)(app, self.id)
    }

    /// See [`ViewportOffset::pixels`].
    pub fn pixels(self, app: &App) -> f64 {
        (self.vtable.pixels)(app, self.id)
    }

    /// See [`ViewportOffset::has_pixels`].
    pub fn has_pixels(self, app: &App) -> bool {
        (self.vtable.has_pixels)(app, self.id)
    }

    /// See [`ScrollMetrics::viewport_dimension`].
    pub fn viewport_dimension(self, app: &App) -> f64 {
        (self.vtable.viewport_dimension)(app, self.id)
    }

    /// See [`ScrollMetrics::has_viewport_dimension`].
    pub fn has_viewport_dimension(self, app: &App) -> bool {
        (self.vtable.has_viewport_dimension)(app, self.id)
    }

    /// See [`ScrollMetrics::axis_direction`].
    pub fn axis_direction(self, app: &App) -> AxisDirection {
        (self.vtable.axis_direction)(app, self.id)
    }

    /// See [`ScrollMetrics::axis`].
    pub fn axis(self, app: &App) -> Axis {
        axis_direction_to_axis(self.axis_direction(app))
    }

    /// See [`ScrollMetrics::device_pixel_ratio`].
    pub fn device_pixel_ratio(self, app: &App) -> f64 {
        (self.vtable.device_pixel_ratio)(app, self.id)
    }

    /// See [`ScrollPosition::copy_with`].
    pub fn copy_with(self, app: &App) -> FixedScrollMetrics {
        (self.vtable.copy_with)(app, self.id)
    }

    /// See [`ScrollMetrics::out_of_range`].
    pub fn out_of_range(self, app: &App) -> bool {
        self.copy_with(app).out_of_range()
    }

    /// See [`ScrollMetrics::at_edge`].
    pub fn at_edge(self, app: &App) -> bool {
        self.copy_with(app).at_edge()
    }

    /// See [`ScrollMetrics::extent_before`].
    pub fn extent_before(self, app: &App) -> f64 {
        self.copy_with(app).extent_before()
    }

    /// See [`ScrollMetrics::extent_inside`].
    pub fn extent_inside(self, app: &App) -> f64 {
        self.copy_with(app).extent_inside()
    }

    /// See [`ScrollMetrics::extent_after`].
    pub fn extent_after(self, app: &App) -> f64 {
        self.copy_with(app).extent_after()
    }

    /// See [`ScrollMetrics::extent_total`].
    pub fn extent_total(self, app: &App) -> f64 {
        self.copy_with(app).extent_total()
    }

    /// See [`ScrollPosition::have_dimensions`].
    pub fn have_dimensions(self, app: &App) -> bool {
        (self.vtable.have_dimensions)(app, self.id)
    }

    /// See [`ScrollPosition::should_ignore_pointer`].
    pub fn should_ignore_pointer(self, app: &App) -> bool {
        (self.vtable.should_ignore_pointer)(app, self.id)
    }

    /// See [`ScrollPosition::absorb`].
    pub fn absorb(self, app: &mut App, other: AnyScrollPosition) {
        (self.vtable.absorb)(app, self.id, other);
    }

    /// Dart's `other._activity = null` inside `absorb`.
    fn take_activity(self, app: &mut App) {
        (self.vtable.take_activity)(app, self.id);
    }

    /// See [`ScrollPosition::set_pixels`].
    pub fn set_pixels(self, app: &mut App, new_pixels: f64) -> f64 {
        (self.vtable.set_pixels)(app, self.id, new_pixels)
    }

    /// See [`ScrollPosition::correct_pixels`].
    pub fn correct_pixels(self, app: &mut App, value: f64) {
        (self.vtable.correct_pixels)(app, self.id, value);
    }

    /// See [`ViewportOffset::correct_by`].
    pub fn correct_by(self, app: &mut App, correction: f64) {
        (self.vtable.correct_by)(app, self.id, correction);
    }

    /// See [`ViewportOffset::apply_viewport_dimension`].
    pub fn apply_viewport_dimension(self, app: &mut App, viewport_dimension: f64) -> bool {
        (self.vtable.apply_viewport_dimension)(app, self.id, viewport_dimension)
    }

    /// See [`ViewportOffset::apply_content_dimensions`].
    pub fn apply_content_dimensions(
        self,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool {
        (self.vtable.apply_content_dimensions)(app, self.id, min_scroll_extent, max_scroll_extent)
    }

    /// See [`ScrollPosition::ensure_visible`].
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_visible(
        self,
        app: &mut App,
        object: AnyRenderObject,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
        target_render_object: Option<AnyRenderObject>,
    ) -> CompleterFuture<()> {
        (self.vtable.ensure_visible)(
            app,
            self.id,
            object,
            alignment,
            duration,
            curve,
            alignment_policy,
            target_render_object,
        )
    }

    /// See [`ScrollPosition::is_scrolling_notifier`].
    pub fn is_scrolling_notifier(self, app: &App) -> Handle<ValueNotifier<bool>> {
        (self.vtable.is_scrolling_notifier)(app, self.id)
    }

    /// See [`ViewportOffset::animate_to`].
    pub fn animate_to(
        self,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        (self.vtable.animate_to)(app, self.id, to, duration, curve)
    }

    /// See [`ViewportOffset::jump_to`].
    pub fn jump_to(self, app: &mut App, value: f64) {
        (self.vtable.jump_to)(app, self.id, value);
    }

    /// See [`ScrollPosition::pointer_scroll`].
    pub fn pointer_scroll(self, app: &mut App, delta: f64) {
        (self.vtable.pointer_scroll)(app, self.id, delta);
    }

    /// See [`ViewportOffset::move_to`].
    pub fn move_to(
        self,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        (self.vtable.move_to)(app, self.id, to, duration, curve, clamp)
    }

    /// See [`ViewportOffset::user_scroll_direction`].
    pub fn user_scroll_direction(self, app: &App) -> ScrollDirection {
        (self.vtable.user_scroll_direction)(app, self.id)
    }

    /// See [`ViewportOffset::allow_implicit_scrolling`].
    pub fn allow_implicit_scrolling(self, app: &App) -> bool {
        (self.vtable.allow_implicit_scrolling)(app, self.id)
    }

    /// See [`ScrollPosition::jump_to_without_settling`].
    #[deprecated(
        note = "This method bypasses scroll activity management and can cause inconsistent \
                layouts or scrolling behavior. Use jump_to or a custom ScrollPosition instead."
    )]
    pub fn jump_to_without_settling(self, app: &mut App, value: f64) {
        (self.vtable.jump_to_without_settling)(app, self.id, value);
    }

    /// See [`ScrollPosition::hold`].
    pub fn hold(
        self,
        app: &mut App,
        hold_cancel_callback: Listener,
    ) -> Rc<dyn ScrollHoldController> {
        (self.vtable.hold)(app, self.id, hold_cancel_callback)
    }

    /// See [`ScrollPosition::drag`].
    pub fn drag(
        self,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag> {
        (self.vtable.drag)(app, self.id, details, drag_cancel_callback)
    }

    /// See [`ScrollPosition::restore_offset`].
    pub fn restore_offset(self, app: &mut App, offset: f64, initial_restore: bool) {
        (self.vtable.restore_offset)(app, self.id, offset, initial_restore);
    }

    /// See [`ScrollPosition::activity`].
    pub fn activity(self, app: &App) -> Option<AnyScrollActivity> {
        (self.vtable.activity)(app, self.id)
    }

    /// See [`ScrollPosition::begin_activity`].
    pub fn begin_activity(self, app: &mut App, new_activity: Option<AnyScrollActivity>) {
        (self.vtable.begin_activity)(app, self.id, new_activity);
    }

    /// See [`ScrollPosition::did_start_scroll`].
    pub fn did_start_scroll(self, app: &mut App) {
        (self.vtable.did_start_scroll)(app, self.id);
    }

    /// See [`ScrollPosition::did_update_scroll_position_by`].
    pub fn did_update_scroll_position_by(self, app: &mut App, delta: f64) {
        (self.vtable.did_update_scroll_position_by)(app, self.id, delta);
    }

    /// See [`ScrollPosition::did_end_scroll`].
    pub fn did_end_scroll(self, app: &mut App) {
        (self.vtable.did_end_scroll)(app, self.id);
    }

    /// See [`ScrollPosition::did_overscroll_by`].
    pub fn did_overscroll_by(self, app: &mut App, value: f64) {
        (self.vtable.did_overscroll_by)(app, self.id, value);
    }

    /// See [`ScrollPosition::did_update_scroll_direction`].
    pub fn did_update_scroll_direction(self, app: &mut App, direction: ScrollDirection) {
        (self.vtable.did_update_scroll_direction)(app, self.id, direction);
    }

    /// See [`ScrollPosition::did_update_scroll_metrics`].
    pub fn did_update_scroll_metrics(self, app: &mut App) {
        (self.vtable.did_update_scroll_metrics)(app, self.id);
    }

    /// See [`ScrollPosition::recommend_deferred_loading`].
    pub fn recommend_deferred_loading(self, app: &mut App, context: BuildContext) -> bool {
        (self.vtable.recommend_deferred_loading)(app, self.id, context)
    }

    /// See [`ScrollPosition::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// See [`ScrollPosition::describe`].
    pub fn describe(self, app: &App) -> String {
        (self.vtable.describe)(app, self.id)
    }
}

impl PartialEq for AnyScrollPosition {
    fn eq(&self, other: &AnyScrollPosition) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyScrollPosition {}

impl Hash for AnyScrollPosition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyScrollPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyScrollPosition({:?})", self.id)
    }
}

impl Listenable for AnyScrollPosition {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener);
    }
}

/// A notification that a scrollable widget's [`ScrollMetrics`] have changed.
///
/// For example, when the content of a scrollable is altered, making it larger
/// or smaller, this notification will be dispatched. Similarly, if the size
/// of the window or parent changes, the scrollable can notify of these
/// changes in dimensions.
///
/// The above behaviors usually do not trigger [`ScrollNotification`](crate::ScrollNotification) events,
/// so this is useful for listening to [`ScrollMetrics`] changes that are not
/// caused by the user scrolling.
pub struct ScrollMetricsNotification {
    viewport_notification: ViewportNotificationData,

    /// Description of a scrollable widget's [`ScrollMetrics`].
    pub metrics: Rc<dyn ScrollMetrics>,

    /// The build context of the widget that fired this notification.
    ///
    /// This can be used to find the scrollable widget's render objects to
    /// determine the size of the viewport, for instance.
    pub context: BuildContext,
}

impl ScrollMetricsNotification {
    /// Creates a notification that the scrollable widget's [`ScrollMetrics`] have
    /// changed.
    pub fn new(metrics: Rc<dyn ScrollMetrics>, context: BuildContext) -> ScrollMetricsNotification {
        ScrollMetricsNotification {
            viewport_notification: ViewportNotificationData::new(),
            metrics,
            context,
        }
    }

    /// Convert this notification to a [`ScrollNotification`](crate::ScrollNotification).
    ///
    /// This allows it to be used with [`crate::ScrollNotificationPredicate`]s.
    pub fn as_scroll_update(&self) -> ScrollUpdateNotification {
        ScrollUpdateNotification::new(self.metrics.clone(), self.context).with_depth(self.depth())
    }
}

impl Notification for ScrollMetricsNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(self.viewport_notification.depth_cell())
    }
}

impl ViewportNotificationMixin for ScrollMetricsNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl Debug for ScrollMetricsNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        ViewportNotificationMixin::debug_fill_description(self, &mut description);
        description.push(format!("{:?}", self.metrics));
        write!(f, "ScrollMetricsNotification({})", description.join(", "))
    }
}

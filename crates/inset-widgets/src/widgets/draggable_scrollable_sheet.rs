//! Flutter counterpart: `widgets/draggable_scrollable_sheet.dart`.

use std::cell::Cell;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_animation::{Animation, AnimationBehavior, AnimationController, Curve};
use inset_embedder::clamp_double;
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, ListenableObject, Listener,
    ValueListenable, ValueNotifier,
};
use inset_gestures::{Drag, DragStartDetails};
use inset_painting::AlignmentGeometry;
use inset_physics::{Simulation, Tolerance};
use inset_scheduler::{FrameCallback, SchedulerBinding};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, Notification, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::{FractionallySizedBox, SizedBox};
use crate::widgets::inherited_notifier::{InheritedNotifier, InheritedNotifierKind};
use crate::widgets::layout_builder::LayoutBuilder;
use crate::widgets::scroll_activity::AnyScrollActivity;
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_controller::{
    AnyScrollController, ScrollController, ScrollControllerData, ScrollControllerLeaf,
};
use crate::widgets::scroll_notification::{ViewportNotificationData, ViewportNotificationMixin};
use crate::widgets::scroll_physics::{AlwaysScrollableScrollPhysics, ScrollPhysicsRef};
use crate::widgets::scroll_position::{AnyScrollPosition, ScrollPosition, ScrollPositionData};
use crate::widgets::scroll_position_with_single_context::{
    ScrollPositionWithSingleContext, ScrollPositionWithSingleContextData,
    ScrollPositionWithSingleContextLeaf,
};
use crate::widgets::scroll_simulation::ClampingScrollSimulation;
use crate::widgets::value_listenable_builder::ValueListenableBuilder;
use crate::{InheritedWidget, ScrollActivityDelegate};

/// The signature of a method that provides a [`BuildContext`] and
/// `ScrollController` for building a widget that may overflow the draggable
/// [`Axis`](inset_painting::Axis) of the containing area.
///
/// This builder is used when a widget is expected to have a scrollable child
/// widget, and a vertical gesture needs to trigger scrolling and some other behavior
/// during that same gesture.
///
/// For example, users of [`DraggableScrollableSheet`] should apply the scroll controller
/// to a scroll view such as `SingleChildScrollView` or `ListView`, to have the whole sheet
/// be draggable.
pub type ScrollableWidgetBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyScrollController) -> WidgetRef>;

/// Dart's `_DraggableSheetExtent Function()`: the sheet's current extent, which the state
/// replaces on every rebuild.
type GetSheetExtent = Rc<dyn Fn(&App) -> Handle<DraggableSheetExtent>>;

// ---------------------------------------------------------------------------------------------
// DraggableScrollableController

/// Controls a [`DraggableScrollableSheet`].
///
/// Draggable scrollable controllers are typically stored as member variables in
/// `State` objects and are reused in each `State::build`. Controllers can only
/// be used to control one sheet at a time. A controller can be reused with a
/// new sheet if the previous sheet has been disposed.
///
/// The controller's methods cannot be used until after the controller has been
/// passed into a [`DraggableScrollableSheet`] and the sheet has run `init_state`.
///
/// A [`DraggableScrollableController`] is a
/// [`Listenable`](inset_foundation::Listenable). It notifies its listeners whenever an
/// attached sheet changes sizes. It does not notify its listeners when a sheet is first
/// attached or when an attached sheet's parameters change without affecting the sheet's
/// current size. It does not fire when [`pixels`](Self::pixels) changes without
/// [`size`](Self::size) changing. For example, if the constraints provided to an attached
/// sheet change.
pub struct DraggableScrollableController {
    change_notifier: ChangeNotifierData,
    attached_controller: Option<Handle<DraggableScrollableSheetScrollController>>,
    animation_controllers: Vec<Handle<AnimationController>>,
}

impl DraggableScrollableController {
    /// Creates a controller for [`DraggableScrollableSheet`].
    pub fn new(app: &mut App) -> Handle<DraggableScrollableController> {
        app.create(DraggableScrollableController {
            change_notifier: ChangeNotifierData::new(),
            attached_controller: None,
            animation_controllers: Vec::new(),
        })
    }

    /// Get the current size (as a fraction of the parent height) of the attached sheet.
    pub fn size(self: Handle<Self>, app: &App) -> f64 {
        self.assert_attached(app);
        self.extent(app).current_size(app)
    }

    /// Get the current pixel height of the attached sheet.
    pub fn pixels(self: Handle<Self>, app: &App) -> f64 {
        self.assert_attached(app);
        let extent = self.extent(app);
        extent.current_pixels(app)
    }

    /// Convert a sheet's size (fractional value of parent container height) to pixels.
    pub fn size_to_pixels(self: Handle<Self>, app: &App, size: f64) -> f64 {
        self.assert_attached(app);
        self.extent(app).size_to_pixels(app, size)
    }

    /// Whether a [`DraggableScrollableController`] has attached itself to a
    /// [`DraggableScrollableSheet`].
    ///
    /// If this is false, then members that interact with the `ScrollPosition`,
    /// such as [`size_to_pixels`](Self::size_to_pixels), [`size`](Self::size),
    /// [`animate_to`](Self::animate_to), and [`jump_to`](Self::jump_to), must not be called.
    pub fn is_attached(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .attached_controller
            .is_some_and(|controller| controller.has_clients(app))
    }

    /// Convert a sheet's pixel height to size (fractional value of parent container height).
    pub fn pixels_to_size(self: Handle<Self>, app: &App, pixels: f64) -> f64 {
        self.assert_attached(app);
        self.extent(app).pixels_to_size(app, pixels)
    }

    /// Animates the attached sheet from its current size to the given `size`, a
    /// fractional value of the parent container's height.
    ///
    /// Any active sheet animation is canceled. If the sheet's internal scrollable
    /// is currently animating (e.g. responding to a user fling), that animation is
    /// canceled as well.
    ///
    /// An animation will be interrupted whenever the user attempts to scroll
    /// manually, whenever another activity is started, or when the sheet hits its
    /// max or min size (e.g. if you animate to 1 but the max size is .8, the
    /// animation will stop playing when it reaches .8).
    ///
    /// The duration must not be zero. To jump to a particular value without an
    /// animation, use [`jump_to`](Self::jump_to).
    ///
    /// The sheet will not snap after calling [`animate_to`](Self::animate_to) even if
    /// [`DraggableScrollableSheet::snap`] is true. Snapping only occurs after user drags.
    ///
    /// The returned future completes when the animation ends. An interrupted animation is
    /// stopped as canceled, so its future never completes — as Dart's does not.
    pub fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        size: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        self.assert_attached(app);
        debug_assert!((0.0..=1.0).contains(&size));
        debug_assert!(!duration.is_zero());
        let attached = self.attached_controller(app);
        let extent = self.extent(app);
        let position = attached.position(app);
        let vsync = position.context(app).vsync();
        let animation_controller = AnimationController::create_unbounded(
            app,
            extent.current_size(app),
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        app.get_mut(self)
            .animation_controllers
            .push(animation_controller);
        ScrollActivityDelegate::go_idle(position, app);
        // This disables any snapping until the next user interaction with the sheet.
        app.get_mut(extent).has_dragged = false;
        app.get_mut(extent).has_changed = true;
        extent.start_activity(
            app,
            Listener::new(move |app| {
                // Don't stop the controller if it's already finished and may have been
                // disposed.
                if animation_controller.is_animating(app) {
                    animation_controller.stop(app, true);
                }
            }),
        );
        Animation::add_listener(
            animation_controller,
            app,
            Listener::new(move |app: &mut App| {
                let value = animation_controller.value(app);
                let context = position.context(app).notification_context(app);
                extent.update_size(app, value, context.expect("a mounted scrollable"));
            }),
        );
        let target = clamp_double(size, extent.min_size(app), extent.max_size(app));
        let animated = animation_controller.animate_to(app, target, Some(duration), curve);
        CompleterFuture::spawn(app, animated)
    }

    /// Jumps the attached sheet from its current size to the given `size`, a
    /// fractional value of the parent container's height.
    ///
    /// If `size` is outside of the attached sheet's min or max child size,
    /// [`jump_to`](Self::jump_to) will jump the sheet to the nearest valid size instead.
    ///
    /// Any active sheet animation is canceled. If the sheet's inner scrollable
    /// is currently animating (e.g. responding to a user fling), that animation is
    /// canceled as well.
    ///
    /// The sheet will not snap after calling [`jump_to`](Self::jump_to) even if
    /// [`DraggableScrollableSheet::snap`] is true. Snapping only occurs after user drags.
    pub fn jump_to(self: Handle<Self>, app: &mut App, size: f64) {
        self.assert_attached(app);
        debug_assert!((0.0..=1.0).contains(&size));
        let extent = self.extent(app);
        let position = self.attached_controller(app).position(app);
        // Call start activity to interrupt any other playing activities.
        extent.start_activity(app, Listener::new(|_app| {}));
        ScrollActivityDelegate::go_idle(position, app);
        app.get_mut(extent).has_dragged = false;
        app.get_mut(extent).has_changed = true;
        let context = position.context(app).notification_context(app);
        extent.update_size(app, size, context.expect("a mounted scrollable"));
    }

    /// Reset the attached sheet to its initial size (see
    /// [`DraggableScrollableSheet::initial_child_size`]).
    pub fn reset(self: Handle<Self>, app: &mut App) {
        self.assert_attached(app);
        self.attached_controller(app).reset(app);
    }

    /// Discards any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).change_notifier.dispose();
    }

    fn assert_attached(self: Handle<Self>, app: &App) {
        debug_assert!(
            self.is_attached(app),
            "DraggableScrollableController is not attached to a sheet. A \
             DraggableScrollableController must be used in a DraggableScrollableSheet before \
             any of its methods are called."
        );
    }

    fn attached_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<DraggableScrollableSheetScrollController> {
        app.get(self)
            .attached_controller
            .expect("the controller is attached to a sheet")
    }

    fn extent(self: Handle<Self>, app: &App) -> Handle<DraggableSheetExtent> {
        app.get(self.attached_controller(app)).extent
    }

    /// Dart's `notifyListeners` tear-off, which the extent's size is listened to with.
    fn notification_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, DraggableScrollableController::forward_notification)
    }

    fn forward_notification(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }

    fn attach(
        self: Handle<Self>,
        app: &mut App,
        scroll_controller: Handle<DraggableScrollableSheetScrollController>,
    ) {
        debug_assert!(
            app.get(self).attached_controller.is_none(),
            "Draggable scrollable controller is already attached to a sheet."
        );
        app.get_mut(self).attached_controller = Some(scroll_controller);
        let current_size = app.get(app.get(scroll_controller).extent).current_size;
        let listener = self.notification_listener();
        ListenableObject::add_listener(current_size, app, listener);
        app.get_mut(scroll_controller).on_position_detached = Some(Listener::handle_method(
            self,
            DraggableScrollableController::dispose_animation_controllers,
        ));
    }

    fn on_extent_replaced(
        self: Handle<Self>,
        app: &mut App,
        previous_extent: Handle<DraggableSheetExtent>,
    ) {
        // When the extent has been replaced, the old extent is already disposed and
        // the controller will point to a new extent. We have to add our listener to
        // the new extent.
        let extent = self.extent(app);
        let current_size = app.get(extent).current_size;
        let listener = self.notification_listener();
        ListenableObject::add_listener(current_size, app, listener);
        if previous_extent.current_size(app) != extent.current_size(app) {
            // The listener won't fire for a change in size between two extent
            // objects so we have to fire it manually here.
            self.notify_listeners(app);
        }
    }

    fn detach(self: Handle<Self>, app: &mut App, dispose_extent: bool) {
        if let Some(attached_controller) = app.get(self).attached_controller {
            let extent = app.get(attached_controller).extent;
            if dispose_extent {
                extent.dispose(app);
            } else {
                let current_size = app.get(extent).current_size;
                ListenableObject::remove_listener(current_size, app, &self.notification_listener());
            }
        }
        self.dispose_animation_controllers(app);
        app.get_mut(self).attached_controller = None;
    }

    fn dispose_animation_controllers(self: Handle<Self>, app: &mut App) {
        for animation_controller in std::mem::take(&mut app.get_mut(self).animation_controllers) {
            animation_controller.dispose(app);
        }
    }
}

impl ChangeNotifier for DraggableScrollableController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

// ---------------------------------------------------------------------------------------------
// DraggableScrollableSheet

/// A container for a `Scrollable` that responds to drag gestures by resizing
/// the scrollable until a limit is reached, and then scrolling.
///
/// This widget can be dragged along the vertical axis between its
/// [`min_child_size`](Self::min_child_size), which defaults to `0.25` and
/// [`max_child_size`](Self::max_child_size), which defaults to `1.0`. These sizes are
/// percentages of the height of the parent container.
///
/// The widget coordinates resizing and scrolling of the widget returned by
/// builder as the user drags along the horizontal axis.
///
/// The widget will initially be displayed at its
/// [`initial_child_size`](Self::initial_child_size) which defaults to `0.5`, meaning half the
/// height of its parent. Dragging will work between the range of
/// [`min_child_size`](Self::min_child_size) and [`max_child_size`](Self::max_child_size) (as
/// percentages of the parent container's height) as long as the builder creates a widget
/// which uses the provided scroll controller. If the widget created by the
/// [`ScrollableWidgetBuilder`] does not use the provided controller, the sheet will remain at
/// the initial child size.
///
/// By default, the widget will stay at whatever size the user drags it to. To
/// make the widget snap to specific sizes whenever they lift their finger
/// during a drag, set [`snap`](Self::snap) to `true`. The sheet will snap between
/// [`min_child_size`](Self::min_child_size) and [`max_child_size`](Self::max_child_size). Use
/// [`snap_sizes`](Self::snap_sizes) to add more sizes for the sheet to snap between.
///
/// The snapping effect is only applied on user drags. Programmatically
/// manipulating the sheet size via [`DraggableScrollableController::animate_to`] or
/// [`DraggableScrollableController::jump_to`] will ignore [`snap`](Self::snap) and
/// [`snap_sizes`](Self::snap_sizes).
///
/// By default, the widget will expand its non-occupied area to fill available
/// space in the parent. If this is not desired, e.g. because the parent wants
/// to position sheet based on the space it is taking, the [`expand`](Self::expand) property
/// may be set to false.
pub struct DraggableScrollableSheet {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,
    /// The initial fractional value of the parent container's height to use when
    /// displaying the widget.
    ///
    /// Rebuilding the sheet with a new initial child size will only move
    /// the sheet to the new value if the sheet has not yet been dragged since it
    /// was first built or since the last call to [`DraggableScrollableActuator::reset`].
    ///
    /// The default value is `0.5`.
    pub initial_child_size: f64,
    /// The minimum fractional value of the parent container's height to use when
    /// displaying the widget.
    ///
    /// The default value is `0.25`.
    pub min_child_size: f64,
    /// The maximum fractional value of the parent container's height to use when
    /// displaying the widget.
    ///
    /// The default value is `1.0`.
    pub max_child_size: f64,
    /// Whether the widget should expand to fill the available space in its parent
    /// or not.
    ///
    /// In most cases, this should be true. However, in the case of a parent
    /// widget that will position this one based on its desired size (such as a
    /// `Center`), this should be set to false.
    ///
    /// The default value is true.
    pub expand: bool,
    /// Whether the widget should snap between [`snap_sizes`](Self::snap_sizes) when the user
    /// lifts their finger during a drag.
    ///
    /// If the user's finger was still moving when they lifted it, the widget will
    /// snap to the next snap size in the direction of the drag. If their finger was still,
    /// the widget will snap to the nearest snap size.
    ///
    /// Snapping is not applied when the sheet is programmatically moved by
    /// calling [`DraggableScrollableController::animate_to`] or
    /// [`DraggableScrollableController::jump_to`].
    ///
    /// Rebuilding the sheet with snap newly enabled will immediately trigger a
    /// snap unless the sheet has not yet been dragged away from
    /// [`initial_child_size`](Self::initial_child_size) since first being built or since the
    /// last call to [`DraggableScrollableActuator::reset`].
    pub snap: bool,
    /// A list of target sizes that the widget should snap to.
    ///
    /// Snap sizes are fractional values of the parent container's height. They
    /// must be listed in increasing order and be between
    /// [`min_child_size`](Self::min_child_size) and
    /// [`max_child_size`](Self::max_child_size).
    ///
    /// The minimum and maximum child sizes are implicitly included in snap
    /// sizes and do not need to be specified here.
    pub snap_sizes: Option<Vec<f64>>,
    /// Defines a duration for the snap animations.
    ///
    /// If it's not set, then the animation duration is the distance to the snap
    /// target divided by the velocity of the widget.
    pub snap_animation_duration: Option<Duration>,
    /// A controller that can be used to programmatically control this sheet.
    pub controller: Option<Handle<DraggableScrollableController>>,
    /// Whether the sheet, when dragged (or flung) to its minimum size, should
    /// cause its parent sheet to close.
    ///
    /// Set on emitted [`DraggableScrollableNotification`]s. It is up to parent
    /// widgets to properly read and handle this value.
    pub should_close_on_min_extent: bool,
    /// The builder that creates a child to display in this widget, which will
    /// use the provided scroll controller to enable dragging and scrolling
    /// of the contents.
    pub builder: ScrollableWidgetBuilder,
}

impl DraggableScrollableSheet {
    /// Creates a widget that can be dragged and scrolled in a single gesture; Dart's optional
    /// arguments are the setters.
    ///
    /// Dart's defaults: `initial_child_size` 0.5, `min_child_size` 0.25, `max_child_size` 1.0,
    /// `expand` true, `snap` false, `should_close_on_min_extent` true.
    pub fn new(builder: ScrollableWidgetBuilder) -> DraggableScrollableSheet {
        DraggableScrollableSheet {
            key: None,
            initial_child_size: 0.5,
            min_child_size: 0.25,
            max_child_size: 1.0,
            expand: true,
            snap: false,
            snap_sizes: None,
            snap_animation_duration: None,
            controller: None,
            should_close_on_min_extent: true,
            builder,
        }
    }

    /// Dart `DraggableScrollableSheet(key:)`.
    pub fn key(mut self, key: KeyRef) -> DraggableScrollableSheet {
        self.key = Some(key);
        self
    }

    /// Dart `DraggableScrollableSheet(initialChildSize:)`.
    pub fn initial_child_size(mut self, initial_child_size: f64) -> DraggableScrollableSheet {
        self.initial_child_size = initial_child_size;
        self
    }

    /// Dart `DraggableScrollableSheet(minChildSize:)`.
    pub fn min_child_size(mut self, min_child_size: f64) -> DraggableScrollableSheet {
        self.min_child_size = min_child_size;
        self
    }

    /// Dart `DraggableScrollableSheet(maxChildSize:)`.
    pub fn max_child_size(mut self, max_child_size: f64) -> DraggableScrollableSheet {
        self.max_child_size = max_child_size;
        self
    }

    /// Dart `DraggableScrollableSheet(expand:)`.
    pub fn expand(mut self, expand: bool) -> DraggableScrollableSheet {
        self.expand = expand;
        self
    }

    /// Dart `DraggableScrollableSheet(snap:)`.
    pub fn snap(mut self, snap: bool) -> DraggableScrollableSheet {
        self.snap = snap;
        self
    }

    /// Dart `DraggableScrollableSheet(snapSizes:)`.
    pub fn snap_sizes(mut self, snap_sizes: Vec<f64>) -> DraggableScrollableSheet {
        self.snap_sizes = Some(snap_sizes);
        self
    }

    /// Dart `DraggableScrollableSheet(snapAnimationDuration:)`.
    pub fn snap_animation_duration(mut self, duration: Duration) -> DraggableScrollableSheet {
        debug_assert!(!duration.is_zero());
        self.snap_animation_duration = Some(duration);
        self
    }

    /// Dart `DraggableScrollableSheet(controller:)`.
    pub fn controller(
        mut self,
        controller: Handle<DraggableScrollableController>,
    ) -> DraggableScrollableSheet {
        self.controller = Some(controller);
        self
    }

    /// Dart `DraggableScrollableSheet(shouldCloseOnMinExtent:)`.
    pub fn should_close_on_min_extent(
        mut self,
        should_close_on_min_extent: bool,
    ) -> DraggableScrollableSheet {
        self.should_close_on_min_extent = should_close_on_min_extent;
        self
    }
}

impl Debug for DraggableScrollableSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DraggableScrollableSheet")
            .field("initialChildSize", &self.initial_child_size)
            .field("minChildSize", &self.min_child_size)
            .field("maxChildSize", &self.max_child_size)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DraggableScrollableSheet {
    type State = DraggableScrollableSheetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DraggableScrollableSheetState {
        debug_assert!(self.min_child_size >= 0.0);
        debug_assert!(self.max_child_size <= 1.0);
        debug_assert!(self.min_child_size <= self.initial_child_size);
        debug_assert!(self.initial_child_size <= self.max_child_size);
        DraggableScrollableSheetState {
            state: StateData::new(),
            scroll_controller: None,
            extent: None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// DraggableScrollableNotification

/// A [`Notification`] related to the extent, which is the size, and scroll
/// offset, which is the position of the child list, of the
/// [`DraggableScrollableSheet`].
///
/// [`DraggableScrollableSheet`] widgets notify their ancestors when the size of
/// the sheet changes. When the extent of the sheet changes via a drag,
/// this notification bubbles up through the tree, which means a given
/// `NotificationListener` will receive notifications for all descendant
/// [`DraggableScrollableSheet`] widgets. To focus on notifications from the
/// nearest [`DraggableScrollableSheet`] descendant, check that the
/// [`depth`](ViewportNotificationMixin::depth) property of the notification is zero.
///
/// When an extent notification is received by a `NotificationListener`, the
/// listener will already have completed build and layout, and it is therefore
/// too late for that widget to call `State::set_state`. Any attempt to adjust the
/// build or layout based on an extent notification would result in a layout
/// that lagged one frame behind, which is a poor user experience. Extent
/// notifications are used primarily to drive animations.
pub struct DraggableScrollableNotification {
    viewport_notification: ViewportNotificationData,
    /// The current value of the extent, between [`min_extent`](Self::min_extent) and
    /// [`max_extent`](Self::max_extent).
    pub extent: f64,
    /// The minimum value of [`extent`](Self::extent), which is >= 0.
    pub min_extent: f64,
    /// The maximum value of [`extent`](Self::extent).
    pub max_extent: f64,
    /// The initially requested value for [`extent`](Self::extent).
    pub initial_extent: f64,
    /// The build context of the widget that fired this notification.
    ///
    /// This can be used to find the sheet's render objects to determine the size
    /// of the viewport, for instance. A listener can only assume this context
    /// is live when it first gets the notification.
    pub context: BuildContext,
    /// Whether the widget that fired this notification, when dragged (or flung)
    /// to the minimum extent, should cause its parent sheet to close.
    ///
    /// It is up to parent widgets to properly read and handle this value.
    pub should_close_on_min_extent: bool,
}

impl DraggableScrollableNotification {
    /// Creates a notification that the extent of a [`DraggableScrollableSheet`] has
    /// changed; Dart's optional argument is the setter.
    ///
    /// Dart's default: `should_close_on_min_extent` true.
    pub fn new(
        extent: f64,
        min_extent: f64,
        max_extent: f64,
        initial_extent: f64,
        context: BuildContext,
    ) -> DraggableScrollableNotification {
        debug_assert!(0.0 <= min_extent);
        debug_assert!(max_extent <= 1.0);
        debug_assert!(min_extent <= extent);
        debug_assert!(min_extent <= initial_extent);
        debug_assert!(extent <= max_extent);
        debug_assert!(initial_extent <= max_extent);
        DraggableScrollableNotification {
            viewport_notification: ViewportNotificationData::new(),
            extent,
            min_extent,
            max_extent,
            initial_extent,
            context,
            should_close_on_min_extent: true,
        }
    }

    /// Dart `DraggableScrollableNotification(shouldCloseOnMinExtent:)`.
    pub fn should_close_on_min_extent(
        mut self,
        should_close_on_min_extent: bool,
    ) -> DraggableScrollableNotification {
        self.should_close_on_min_extent = should_close_on_min_extent;
        self
    }
}

impl Notification for DraggableScrollableNotification {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(self.viewport_notification.depth_cell())
    }
}

impl ViewportNotificationMixin for DraggableScrollableNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl Debug for DraggableScrollableNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        ViewportNotificationMixin::debug_fill_description(self, &mut description);
        description.push(format!(
            "minExtent: {}, extent: {}, maxExtent: {}, initialExtent: {}",
            self.min_extent, self.extent, self.max_extent, self.initial_extent
        ));
        write!(
            f,
            "DraggableScrollableNotification({})",
            description.join(", ")
        )
    }
}

// ---------------------------------------------------------------------------------------------
// _DraggableSheetExtent

/// Manages state between [`DraggableScrollableSheetState`],
/// [`DraggableScrollableSheetScrollController`], and
/// [`DraggableScrollableSheetScrollPosition`]; Dart's `_DraggableSheetExtent`.
///
/// The state knows the pixels available along the axis the widget wants to
/// scroll, but expects to get a fraction of those pixels to render the sheet.
///
/// The scroll position knows the number of pixels a user wants to move the sheet.
///
/// The [`available_pixels`](Self::available_pixels) may be `f64::INFINITY`.
struct DraggableSheetExtent {
    cancel_activity: Option<Listener>,
    min_size: f64,
    max_size: f64,
    snap: bool,
    snap_sizes: Vec<f64>,
    snap_animation_duration: Option<Duration>,
    initial_size: f64,
    should_close_on_min_extent: bool,
    current_size: Handle<ValueNotifier<f64>>,
    available_pixels: f64,
    // Used to disable snapping until the user has dragged on the sheet.
    has_dragged: bool,
    // Used to determine if the sheet should move to a new initial size when it
    // changes.
    // We need both `has_changed` and `has_dragged` to achieve the following
    // behavior:
    //   1. The sheet should only snap following user drags (as opposed to
    //      programmatic sheet changes). See docs for `animate_to` and `jump_to`.
    //   2. The sheet should move to a new initial child size on rebuild iff the
    //      sheet has not changed, either by drag or programmatic control. See
    //      docs for `initial_child_size`.
    has_changed: bool,
}

impl DraggableSheetExtent {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        min_size: f64,
        max_size: f64,
        snap: bool,
        snap_sizes: Vec<f64>,
        initial_size: f64,
        snap_animation_duration: Option<Duration>,
        current_size: Option<Handle<ValueNotifier<f64>>>,
        has_dragged: Option<bool>,
        has_changed: Option<bool>,
        should_close_on_min_extent: bool,
    ) -> Handle<DraggableSheetExtent> {
        debug_assert!(min_size >= 0.0);
        debug_assert!(max_size <= 1.0);
        debug_assert!(min_size <= initial_size);
        debug_assert!(initial_size <= max_size);
        let current_size =
            current_size.unwrap_or_else(|| app.create(ValueNotifier::new(initial_size)));
        app.create(DraggableSheetExtent {
            cancel_activity: None,
            min_size,
            max_size,
            snap,
            snap_sizes,
            snap_animation_duration,
            initial_size,
            should_close_on_min_extent,
            current_size,
            available_pixels: f64::INFINITY,
            has_dragged: has_dragged.unwrap_or(false),
            has_changed: has_changed.unwrap_or(false),
        })
    }

    /// The lowest fraction of the parent height the sheet may take.
    pub fn min_size(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).min_size
    }

    /// The highest fraction of the parent height the sheet may take.
    pub fn max_size(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).max_size
    }

    fn is_at_min(self: Handle<Self>, app: &App) -> bool {
        app.get(self).min_size >= self.current_size(app)
    }

    fn is_at_max(self: Handle<Self>, app: &App) -> bool {
        app.get(self).max_size <= self.current_size(app)
    }

    /// The fraction of the parent height the sheet currently takes.
    pub fn current_size(self: Handle<Self>, app: &App) -> f64 {
        *app.get(app.get(self).current_size).value()
    }

    /// The pixel height the sheet currently takes.
    pub fn current_pixels(self: Handle<Self>, app: &App) -> f64 {
        self.size_to_pixels(app, self.current_size(app))
    }

    fn pixel_snap_sizes(self: Handle<Self>, app: &App) -> Vec<f64> {
        app.get(self)
            .snap_sizes
            .clone()
            .into_iter()
            .map(|size| self.size_to_pixels(app, size))
            .collect()
    }

    /// Start an activity that affects the sheet and register a cancel callback
    /// that will be called if another activity starts.
    ///
    /// The `on_canceled` callback will get called even if the subsequent activity
    /// started after this one finished, so `on_canceled` must be safe to call at
    /// any time.
    fn start_activity(self: Handle<Self>, app: &mut App, on_canceled: Listener) {
        if let Some(cancel_activity) = app.get(self).cancel_activity.clone() {
            cancel_activity.call(app);
        }
        app.get_mut(self).cancel_activity = Some(on_canceled);
    }

    /// The scroll position gets inputs in terms of pixels, but the size is
    /// expected to be expressed as a number between 0..1.
    ///
    /// This should only be called to respond to a user drag. To update the
    /// size in response to a programmatic call, use [`update_size`](Self::update_size)
    /// directly.
    fn add_pixel_delta(self: Handle<Self>, app: &mut App, delta: f64, context: BuildContext) {
        // Stop any playing sheet animations.
        if let Some(cancel_activity) = app.get_mut(self).cancel_activity.take() {
            cancel_activity.call(app);
        }
        // The user has interacted with the sheet, set `has_dragged` to true so that
        // we'll snap if applicable.
        app.get_mut(self).has_dragged = true;
        app.get_mut(self).has_changed = true;
        if app.get(self).available_pixels == 0.0 {
            return;
        }
        let new_size = self.current_size(app) + self.pixels_to_size(app, delta);
        self.update_size(app, new_size, context);
    }

    /// Set the size to the new value. `new_size` should be a number between
    /// [`min_size`](Self::min_size) and [`max_size`](Self::max_size).
    ///
    /// This can be triggered by a programmatic (e.g. controller triggered) change
    /// or a user drag.
    fn update_size(self: Handle<Self>, app: &mut App, new_size: f64, context: BuildContext) {
        let clamped_size = clamp_double(new_size, app.get(self).min_size, app.get(self).max_size);
        if self.current_size(app) == clamped_size {
            return;
        }
        app.get(self).current_size.set_value(app, clamped_size);
        let extent = app.get(self);
        DraggableScrollableNotification::new(
            self.current_size(app),
            extent.min_size,
            extent.max_size,
            extent.initial_size,
            context,
        )
        .should_close_on_min_extent(extent.should_close_on_min_extent)
        .dispatch(app, Some(context));
    }

    /// Converts a pixel height to a fraction of the parent container's height.
    pub fn pixels_to_size(self: Handle<Self>, app: &App, pixels: f64) -> f64 {
        let extent = app.get(self);
        pixels / extent.available_pixels * extent.max_size
    }

    /// Converts a fraction of the parent container's height to a pixel height.
    pub fn size_to_pixels(self: Handle<Self>, app: &App, size: f64) -> f64 {
        let extent = app.get(self);
        size / extent.max_size * extent.available_pixels
    }

    /// Discards any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let current_size = app.get(self).current_size;
        app.get_mut(current_size).dispose();
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_with(
        self: Handle<Self>,
        app: &mut App,
        min_size: f64,
        max_size: f64,
        snap: bool,
        snap_sizes: Vec<f64>,
        initial_size: f64,
        snap_animation_duration: Option<Duration>,
        should_close_on_min_extent: bool,
    ) -> Handle<DraggableSheetExtent> {
        let has_changed = app.get(self).has_changed;
        let has_dragged = app.get(self).has_dragged;
        // Set the current size to the possibly updated initial size if the sheet
        // hasn't changed yet.
        let size = if has_changed {
            clamp_double(self.current_size(app), min_size, max_size)
        } else {
            initial_size
        };
        let current_size = app.create(ValueNotifier::new(size));
        DraggableSheetExtent::new(
            app,
            min_size,
            max_size,
            snap,
            snap_sizes,
            initial_size,
            snap_animation_duration,
            Some(current_size),
            Some(has_dragged),
            Some(has_changed),
            should_close_on_min_extent,
        )
    }
}

impl Debug for DraggableSheetExtent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DraggableSheetExtent")
            .field("minSize", &self.min_size)
            .field("maxSize", &self.max_size)
            .field("snap", &self.snap)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------------------------
// _DraggableScrollableSheetState

/// Dart's `_DraggableScrollableSheetState`.
pub struct DraggableScrollableSheetState {
    state: StateData<DraggableScrollableSheet>,
    scroll_controller: Option<Handle<DraggableScrollableSheetScrollController>>,
    extent: Option<Handle<DraggableSheetExtent>>,
}

impl DraggableScrollableSheetState {
    fn scroll_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<DraggableScrollableSheetScrollController> {
        app.get(self).scroll_controller.expect("init_state has run")
    }

    fn extent(self: Handle<Self>, app: &App) -> Handle<DraggableSheetExtent> {
        app.get(self).extent.expect("init_state has run")
    }

    fn implied_snap_sizes(self: Handle<Self>, app: &App) -> Vec<f64> {
        let widget = self.widget(app);
        if cfg!(debug_assertions)
            && let Some(snap_sizes) = &widget.snap_sizes
        {
            for (index, snap_size) in snap_sizes.iter().copied().enumerate() {
                debug_assert!(
                    snap_size >= widget.min_child_size && snap_size <= widget.max_child_size,
                    "Invalid snapSize '{snap_size}' at index {index}. Snap sizes must be \
                     between `minChildSize` and `maxChildSize`."
                );
                debug_assert!(
                    index == 0 || snap_size > snap_sizes[index - 1],
                    "Invalid snapSize '{snap_size}' at index {index}. Snap sizes must be in \
                     ascending order."
                );
            }
        }
        // Ensure the snap sizes start and end with the min and max child sizes.
        let Some(snap_sizes) = widget.snap_sizes.as_ref().filter(|sizes| !sizes.is_empty()) else {
            return vec![widget.min_child_size, widget.max_child_size];
        };
        let mut implied = Vec::with_capacity(snap_sizes.len() + 2);
        if snap_sizes[0] != widget.min_child_size {
            implied.push(widget.min_child_size);
        }
        implied.extend_from_slice(snap_sizes);
        if snap_sizes[snap_sizes.len() - 1] != widget.max_child_size {
            implied.push(widget.max_child_size);
        }
        implied
    }

    fn replace_extent(self: Handle<Self>, app: &mut App, old_widget: &DraggableScrollableSheet) {
        let previous_extent = self.extent(app);
        let snap_sizes = self.implied_snap_sizes(app);
        let widget = self.widget(app);
        let (min, max, snap, duration, initial, close_on_min) = (
            widget.min_child_size,
            widget.max_child_size,
            widget.snap,
            widget.snap_animation_duration,
            widget.initial_child_size,
            widget.should_close_on_min_extent,
        );
        let snap_sizes_changed = widget.snap_sizes != old_widget.snap_sizes;
        let snap_changed = widget.snap != old_widget.snap;
        let extent = previous_extent.copy_with(
            app,
            min,
            max,
            snap,
            snap_sizes,
            initial,
            duration,
            close_on_min,
        );
        app.get_mut(self).extent = Some(extent);
        // Modify the existing scroll controller instead of replacing it so that
        // developers listening to the controller do not have to rebuild their listeners.
        let scroll_controller = self.scroll_controller(app);
        app.get_mut(scroll_controller).extent = extent;
        // If an external facing controller was provided, let it know that the
        // extent has been replaced.
        if let Some(controller) = self.widget(app).controller {
            controller.on_extent_replaced(app, previous_extent);
        }
        previous_extent.dispose(app);
        if snap && (snap_changed || snap_sizes_changed) && scroll_controller.has_clients(app) {
            // Trigger a snap in case snap or snapSizes has changed and there is a
            // scroll position currently attached. We put this in a post frame
            // callback so that `build` can update `extent.available_pixels` before
            // this runs — we can't use the previous extent's available pixels as it may
            // have changed when the widget was updated.
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    for position in scroll_controller.positions(app) {
                        let position = position
                            .downcast::<DraggableScrollableSheetScrollPosition>(app)
                            .expect("the controller creates its own positions");
                        ScrollActivityDelegate::go_ballistic(position, app, 0.0);
                    }
                }),
            );
        }
    }
}

impl State for DraggableScrollableSheetState {
    type Widget = DraggableScrollableSheet;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let snap_sizes = self.implied_snap_sizes(app);
        let widget = self.widget(app);
        let (min, max, snap, duration, initial, close_on_min) = (
            widget.min_child_size,
            widget.max_child_size,
            widget.snap,
            widget.snap_animation_duration,
            widget.initial_child_size,
            widget.should_close_on_min_extent,
        );
        let extent = DraggableSheetExtent::new(
            app,
            min,
            max,
            snap,
            snap_sizes,
            initial,
            duration,
            None,
            None,
            None,
            close_on_min,
        );
        app.get_mut(self).extent = Some(extent);
        let scroll_controller = DraggableScrollableSheetScrollController::new(app, extent);
        app.get_mut(self).scroll_controller = Some(scroll_controller);
        if let Some(controller) = self.widget(app).controller {
            controller.attach(app, scroll_controller);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DraggableScrollableSheet) {
        let controller = self.widget(app).controller;
        if controller != old_widget.controller {
            if let Some(old_controller) = old_widget.controller {
                old_controller.detach(app, false);
            }
            if let Some(controller) = controller {
                let scroll_controller = self.scroll_controller(app);
                controller.attach(app, scroll_controller);
            }
        }
        self.replace_extent(app, old_widget);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        if InheritedResetNotifier::should_reset(app, context) {
            self.scroll_controller(app).reset(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        match self.widget(app).controller {
            None => self.extent(app).dispose(app),
            Some(controller) => controller.detach(app, true),
        }
        self.scroll_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let extent = self.extent(app);
        let expand = self.widget(app).expand;
        let max_child_size = self.widget(app).max_child_size;
        let builder = Rc::clone(&self.widget(app).builder);
        let scroll_controller = self.scroll_controller(app);
        let child = builder(app, context, scroll_controller.as_controller());
        let current_size: Rc<dyn ValueListenable<f64>> = Rc::new(app.get(extent).current_size);
        ValueListenableBuilder::<f64>::new(
            current_size,
            move |_app, _context, current_size, child| {
                let current_size = *current_size;
                let child = child.cloned();
                LayoutBuilder::new(move |app, _context, constraints| {
                    app.get_mut(extent).available_pixels =
                        max_child_size * constraints.biggest().height();
                    let mut sheet = FractionallySizedBox::new()
                        .height_factor(current_size)
                        .alignment(AlignmentGeometry::BOTTOM_CENTER);
                    if let Some(child) = &child {
                        sheet = sheet.child(child.clone());
                    }
                    if expand {
                        SizedBox::expand().child(sheet).into_widget()
                    } else {
                        sheet.into_widget()
                    }
                })
                .into_widget()
            },
        )
        .child(child)
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _DraggableScrollableSheetScrollController

/// A `ScrollController` suitable for use in a [`ScrollableWidgetBuilder`] created
/// by a [`DraggableScrollableSheet`]; Dart's `_DraggableScrollableSheetScrollController`.
///
/// If a [`DraggableScrollableSheet`] contains content that exceeds the height
/// of its container, this controller will allow the sheet to both be dragged to
/// fill the container and then scroll the child content.
///
/// See also:
///
///  * [`DraggableScrollableSheetScrollPosition`], which manages the positioning logic for
///    this controller.
///  * `PrimaryScrollController`, which can be used to establish this controller as the
///    primary controller for descendants.
struct DraggableScrollableSheetScrollController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    extent: Handle<DraggableSheetExtent>,
    on_position_detached: Option<Listener>,
}

impl DraggableScrollableSheetScrollController {
    fn new(
        app: &mut App,
        extent: Handle<DraggableSheetExtent>,
    ) -> Handle<DraggableScrollableSheetScrollController> {
        app.create(DraggableScrollableSheetScrollController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(0.0, true, None, None, None),
            extent,
            on_position_detached: None,
        })
    }

    /// Dart's `_DraggableScrollableSheetScrollController.position`, which narrows
    /// `ScrollController.position` to the position this controller creates.
    pub fn position(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<DraggableScrollableSheetScrollPosition> {
        ScrollControllerLeaf::position(self, app)
            .downcast::<DraggableScrollableSheetScrollPosition>(app)
            .expect("the controller creates its own positions")
    }

    fn reset(self: Handle<Self>, app: &mut App) {
        let extent = app.get(self).extent;
        if let Some(cancel_activity) = app.get(extent).cancel_activity.clone() {
            cancel_activity.call(app);
        }
        app.get_mut(extent).has_dragged = false;
        app.get_mut(extent).has_changed = false;
        // `jump_to` can result in trying to replace semantics during build.
        // Just animate really fast.
        // Avoid doing it at all if the offset is already 0.0.
        if self.offset(app) != 0.0 {
            self.animate_to(
                app,
                0.0,
                Duration::from_millis(1),
                inset_animation::Curves::linear(),
            );
        }
        let initial_size = app.get(extent).initial_size;
        let context = self.position(app).context(app).notification_context(app);
        extent.update_size(app, initial_size, context.expect("a mounted scrollable"));
    }
}

impl ChangeNotifier for DraggableScrollableSheetScrollController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for DraggableScrollableSheetScrollController {
    crate::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> AnyScrollPosition {
        let physics = physics.apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
        DraggableScrollableSheetScrollPosition::new(
            app,
            physics,
            context,
            old_position,
            Rc::new(move |app: &App| app.get(self).extent),
        )
        .as_scroll_position()
    }

    fn debug_fill_description(self: Handle<Self>, app: &App, description: &mut Vec<String>) {
        ScrollController::debug_fill_description(self.as_controller(), app, description);
        description.push(format!("extent: {:?}", app.get(self).extent));
    }

    fn detach(self: Handle<Self>, app: &mut App, position: AnyScrollPosition) {
        if let Some(on_position_detached) = app.get(self).on_position_detached.clone() {
            on_position_detached.call(app);
        }
        ScrollController::detach(self.as_controller(), app, position);
    }
}

// ---------------------------------------------------------------------------------------------
// _DraggableScrollableSheetScrollPosition

/// A scroll position that manages scroll activities for
/// [`DraggableScrollableSheetScrollController`]; Dart's
/// `_DraggableScrollableSheetScrollPosition`.
///
/// This is a [`ScrollPositionWithSingleContextLeaf`] whose activities change the
/// sheet's current size or the visible content offset in the `Scrollable`'s `Viewport`.
struct DraggableScrollableSheetScrollPosition {
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
    drag_cancel_callback: Option<Listener>,
    get_extent: GetSheetExtent,
    ballistic_controllers: Vec<Handle<AnimationController>>,
}

impl DraggableScrollableSheetScrollPosition {
    fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
        get_extent: GetSheetExtent,
    ) -> Handle<DraggableScrollableSheetScrollPosition> {
        let scroll_position = ScrollPositionData::new(app, physics, context, true, None);
        let this = app.create(DraggableScrollableSheetScrollPosition {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
            drag_cancel_callback: None,
            get_extent,
            ballistic_controllers: Vec::new(),
        });
        ScrollPositionWithSingleContext::init(this, app, Some(0.0), old_position);
        this
    }

    /// Whether the list scrolls rather than the sheet resizing.
    pub fn list_should_scroll(self: Handle<Self>, app: &App) -> bool {
        inset_rendering::ViewportOffset::pixels(self, app) > 0.0
    }

    /// The sheet's shared extent.
    pub fn extent(self: Handle<Self>, app: &App) -> Handle<DraggableSheetExtent> {
        (Rc::clone(&app.get(self).get_extent))(app)
    }

    // Checks if the sheet's current size is close to a snap size, returning the
    // snap size if so; returns `None` otherwise.
    fn get_current_snap_size(self: Handle<Self>, app: &App) -> Option<f64> {
        let extent = self.extent(app);
        let metrics = self.copy_with(app);
        let tolerance = self.physics(app).tolerance_for(&metrics).distance;
        let current_size = extent.current_size(app);
        app.get(extent)
            .snap_sizes
            .iter()
            .copied()
            .find(|snap_size| {
                (current_size - snap_size).abs() <= extent.pixels_to_size(app, tolerance)
            })
    }

    fn is_at_snap_size(self: Handle<Self>, app: &App) -> bool {
        self.get_current_snap_size(app).is_some()
    }

    fn should_snap(self: Handle<Self>, app: &App) -> bool {
        let extent = self.extent(app);
        app.get(extent).snap && app.get(extent).has_dragged && !self.is_at_snap_size(app)
    }
}

impl ChangeNotifier for DraggableScrollableSheetScrollPosition {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl inset_rendering::ViewportOffset for DraggableScrollableSheetScrollPosition {
    crate::scroll_position_with_single_context_viewport_offset_overrides!();
}

impl ScrollPosition for DraggableScrollableSheetScrollPosition {
    crate::scroll_position_with_single_context_overrides!();
}

impl ScrollActivityDelegate for DraggableScrollableSheetScrollPosition {
    crate::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for DraggableScrollableSheetScrollPosition {
    crate::scroll_position_with_single_context_accessors!();

    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionWithSingleContext::absorb(self, app, other);
        debug_assert!(app.get(self).drag_cancel_callback.is_none());

        let Some(other) = other.downcast::<DraggableScrollableSheetScrollPosition>(app) else {
            return;
        };

        if let Some(drag_cancel_callback) = app.get_mut(other).drag_cancel_callback.take() {
            app.get_mut(self).drag_cancel_callback = Some(drag_cancel_callback);
        }
    }

    fn begin_activity(self: Handle<Self>, app: &mut App, new_activity: Option<AnyScrollActivity>) {
        // Cancel the running ballistic simulations
        for ballistic_controller in app.get(self).ballistic_controllers.clone() {
            ballistic_controller.stop(app, true);
        }
        ScrollPositionWithSingleContext::begin_activity(self, app, new_activity);
    }

    fn apply_user_offset(self: Handle<Self>, app: &mut App, delta: f64) {
        let extent = self.extent(app);
        let is_at_min = extent.is_at_min(app);
        let is_at_max = extent.is_at_max(app);
        if !self.list_should_scroll(app)
            && (!(is_at_min || is_at_max)
                || (is_at_min && delta < 0.0)
                || (is_at_max && delta > 0.0))
        {
            let context = self.context(app).notification_context(app);
            extent.add_pixel_delta(app, -delta, context.expect("a mounted scrollable"));
        } else {
            ScrollPositionWithSingleContext::apply_user_offset(self, app, delta);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for ballistic_controller in std::mem::take(&mut app.get_mut(self).ballistic_controllers) {
            ballistic_controller.dispose(app);
        }
        ScrollPositionWithSingleContext::dispose(self, app);
    }

    fn go_ballistic(self: Handle<Self>, app: &mut App, velocity: f64) {
        let extent = self.extent(app);
        if (velocity == 0.0 && !self.should_snap(app))
            || (velocity < 0.0 && self.list_should_scroll(app))
            || (velocity > 0.0 && extent.is_at_max(app))
        {
            ScrollPositionWithSingleContext::go_ballistic(self, app, velocity);
            return;
        }
        // Scrollable expects that we will dispose of its current drag cancel callback.
        if let Some(drag_cancel_callback) = app.get_mut(self).drag_cancel_callback.take() {
            drag_cancel_callback.call(app);
        }

        let metrics = self.copy_with(app);
        let tolerance = self.physics(app).tolerance_for(&metrics);
        let simulation: Box<dyn Simulation> = if app.get(extent).snap {
            // Snap is enabled, simulate snapping instead of clamping scroll.
            Box::new(SnappingSimulation::new(
                extent.current_pixels(app),
                velocity,
                extent.pixel_snap_sizes(app),
                app.get(extent).snap_animation_duration,
                tolerance,
            ))
        } else {
            // The iOS bouncing simulation just isn't right here - once we delegate
            // the ballistic back to the ScrollView, it will use the right simulation.
            Box::new(ClampingScrollSimulation::with_options(
                // Run the simulation in terms of pixels, not extent.
                extent.current_pixels(app),
                velocity,
                ClampingScrollSimulation::DEFAULT_FRICTION,
                tolerance,
            ))
        };

        let vsync = self.context(app).vsync();
        let ballistic_controller = AnimationController::create_unbounded(
            app,
            0.0,
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        app.get_mut(self)
            .ballistic_controllers
            .push(ballistic_controller);

        let last_position = Rc::new(Cell::new(extent.current_pixels(app)));
        let velocity = Rc::new(Cell::new(velocity));
        let tick = {
            let last_position = Rc::clone(&last_position);
            let velocity = Rc::clone(&velocity);
            move |app: &mut App| {
                let value = ballistic_controller.value(app);
                let delta = value - last_position.get();
                last_position.set(value);
                let context = self.context(app).notification_context(app);
                extent.add_pixel_delta(app, delta, context.expect("a mounted scrollable"));
                if (velocity.get() > 0.0 && extent.is_at_max(app))
                    || (velocity.get() < 0.0 && extent.is_at_min(app))
                {
                    // Make sure we pass along enough velocity to keep scrolling - otherwise
                    // we just "bounce" off the top making it look like the list doesn't
                    // have more to scroll.
                    let controller_velocity = ballistic_controller.velocity(app);
                    let metrics = self.copy_with(app);
                    velocity.set(
                        controller_velocity
                            + self.physics(app).tolerance_for(&metrics).velocity
                                * controller_velocity.signum(),
                    );
                    ScrollPositionWithSingleContext::go_ballistic(self, app, velocity.get());
                    ballistic_controller.stop(app, true);
                } else if Animation::is_completed(ballistic_controller, app) {
                    // Update the extent value after the snap animation completes to
                    // avoid rounding errors that could prevent the sheet from closing when
                    // it reaches minSize.
                    if let Some(snap_size) = self.get_current_snap_size(app) {
                        let context = self.context(app).notification_context(app);
                        extent.update_size(app, snap_size, context.expect("a mounted scrollable"));
                    }
                    ScrollPositionWithSingleContext::go_ballistic(self, app, 0.0);
                }
            }
        };

        Animation::add_listener(ballistic_controller, app, Listener::new(tick));
        let future = ballistic_controller.animate_with(app, simulation);
        future.when_complete_or_cancel(
            app,
            Listener::new(move |app: &mut App| {
                let controllers = &mut app.get_mut(self).ballistic_controllers;
                if let Some(index) = controllers
                    .iter()
                    .position(|controller| *controller == ballistic_controller)
                {
                    controllers.remove(index);
                    ballistic_controller.dispose(app);
                }
            }),
        );
    }

    fn drag(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag> {
        // Save this so we can call it later if we have to `go_ballistic` on our own.
        app.get_mut(self).drag_cancel_callback = Some(drag_cancel_callback.clone());
        ScrollPositionWithSingleContext::drag(self, app, details, drag_cancel_callback)
    }
}

// ---------------------------------------------------------------------------------------------
// DraggableScrollableActuator

/// A widget that can notify a descendent [`DraggableScrollableSheet`] that it
/// should reset its position to the initial state.
///
/// This is just a wrapper on top of [`DraggableScrollableController`]. It is
/// primarily useful for controlling a sheet in a part of the widget tree that
/// the current code does not control (e.g. library code trying to affect a sheet
/// in library users' code). Generally, it's easier to control the sheet
/// directly by creating a controller and passing the controller to the sheet in
/// its constructor (see [`DraggableScrollableSheet::controller`]).
#[derive(Debug)]
pub struct DraggableScrollableActuator {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,
    /// This child's [`DraggableScrollableSheet`] descendant will be reset when the
    /// [`reset`](Self::reset) method is applied to a context that includes it.
    pub child: WidgetRef,
}

impl DraggableScrollableActuator {
    /// Creates a widget that can notify descendent [`DraggableScrollableSheet`]s
    /// to reset to their initial position.
    pub fn new<K>(child: impl IntoWidget<K>) -> DraggableScrollableActuator {
        DraggableScrollableActuator {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DraggableScrollableActuator(key:)`.
    pub fn key(mut self, key: KeyRef) -> DraggableScrollableActuator {
        self.key = Some(key);
        self
    }

    /// Notifies any descendant [`DraggableScrollableSheet`] that it should reset
    /// to its initial position.
    ///
    /// Returns `true` if a [`DraggableScrollableActuator`] is available and
    /// some [`DraggableScrollableSheet`] is listening for updates, `false`
    /// otherwise.
    pub fn reset(app: &mut App, context: BuildContext) -> bool {
        let notifier = context
            .depend_on_inherited_widget_of_exact_type::<InheritedResetNotifier>(app)
            .map(|widget| widget.notifier);
        notifier.is_some_and(|notifier| notifier.send_reset(app))
    }
}

impl StatefulWidget for DraggableScrollableActuator {
    type State = DraggableScrollableActuatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DraggableScrollableActuatorState {
        DraggableScrollableActuatorState {
            state: StateData::new(),
            notifier: None,
        }
    }
}

/// Dart's `_DraggableScrollableActuatorState`.
pub struct DraggableScrollableActuatorState {
    state: StateData<DraggableScrollableActuator>,
    notifier: Option<Handle<ResetNotifier>>,
}

impl State for DraggableScrollableActuatorState {
    type Widget = DraggableScrollableActuator;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).notifier = Some(ResetNotifier::new(app));
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        InheritedResetNotifier {
            notifier: app.get(self).notifier.expect("init_state has run"),
            child: self.widget(app).child.clone(),
        }
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let notifier = app.get(self).notifier.expect("init_state has run");
        notifier.dispose(app);
    }
}

/// A `ChangeNotifier` to use with `_InheritedResetNotifier` to notify
/// descendants that they should reset to initial state; Dart's `_ResetNotifier`.
struct ResetNotifier {
    change_notifier: ChangeNotifierData,
    /// Whether someone called [`send_reset`](Self::send_reset) or not.
    ///
    /// This flag is reset after checking it.
    was_called: bool,
}

impl ResetNotifier {
    fn new(app: &mut App) -> Handle<ResetNotifier> {
        app.create(ResetNotifier {
            change_notifier: ChangeNotifierData::new(),
            was_called: false,
        })
    }

    /// Fires a reset notification to descendants.
    ///
    /// Returns false if there are no listeners.
    fn send_reset(self: Handle<Self>, app: &mut App) -> bool {
        if !app.get(self).change_notifier.has_listeners() {
            return false;
        }
        app.get_mut(self).was_called = true;
        self.notify_listeners(app);
        true
    }

    /// Discards any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for ResetNotifier {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// An [`InheritedNotifier`] that a [`DraggableScrollableSheet`] listens to for an indication
/// that it should reset itself back to [`DraggableScrollableSheet::initial_child_size`];
/// Dart's `_InheritedResetNotifier`.
#[derive(Debug)]
struct InheritedResetNotifier {
    notifier: Handle<ResetNotifier>,
    child: WidgetRef,
}

impl InheritedResetNotifier {
    /// The tree node; both [`InheritedWidget`] and [`InheritedNotifier`] offer an
    /// `IntoWidget` kind, so this widget names the conversion itself.
    fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedNotifierKind>::into_widget(self)
    }

    /// Specifies whether the [`DraggableScrollableSheet`] should reset to its
    /// initial position.
    ///
    /// Returns true if the notifier requested a reset, false otherwise.
    fn should_reset(app: &mut App, context: BuildContext) -> bool {
        let Some(widget) =
            context.depend_on_inherited_widget_of_exact_type::<InheritedResetNotifier>(app)
        else {
            return false;
        };
        let notifier = widget.notifier;
        let was_called = app.get(notifier).was_called;
        app.get_mut(notifier).was_called = false;
        was_called
    }
}

impl InheritedWidget for InheritedResetNotifier {
    crate::inherited_notifier_overrides!();

    fn child(&self) -> &WidgetRef {
        &self.child
    }
}

impl InheritedNotifier for InheritedResetNotifier {
    type Notifier = ResetNotifier;

    fn notifier(&self) -> Option<Handle<ResetNotifier>> {
        Some(self.notifier)
    }
}

// ---------------------------------------------------------------------------------------------
// _SnappingSimulation

/// Dart's `_SnappingSimulation`: settles the sheet on the snap size the drag is headed for.
#[derive(Debug)]
struct SnappingSimulation {
    position: f64,
    velocity: f64,
    pixel_snap_size: f64,
    tolerance: Tolerance,
}

impl SnappingSimulation {
    // A minimum speed to snap at. Used to ensure that the snapping animation
    // does not play too slowly.
    const MINIMUM_SPEED: f64 = 1600.0;

    fn new(
        position: f64,
        initial_velocity: f64,
        pixel_snap_size: Vec<f64>,
        snap_animation_duration: Option<Duration>,
        tolerance: Tolerance,
    ) -> SnappingSimulation {
        let pixel_snap_size = SnappingSimulation::get_snap_size(
            position,
            initial_velocity,
            &pixel_snap_size,
            tolerance,
        );

        let velocity = match snap_animation_duration {
            Some(duration) if duration.as_millis() > 0 => {
                (pixel_snap_size - position) * 1000.0 / duration.as_millis() as f64
            }
            // Check the direction of the target instead of the sign of the velocity because
            // we may snap in the opposite direction of velocity if velocity is very low.
            _ if pixel_snap_size < position => {
                (-SnappingSimulation::MINIMUM_SPEED).min(initial_velocity)
            }
            _ => SnappingSimulation::MINIMUM_SPEED.max(initial_velocity),
        };

        SnappingSimulation {
            position,
            velocity,
            pixel_snap_size,
            tolerance,
        }
    }

    // Find the two closest snap sizes to the position. If the velocity is
    // non-zero, select the size in the velocity's direction. Otherwise,
    // the nearest snap size.
    fn get_snap_size(
        position: f64,
        initial_velocity: f64,
        pixel_snap_sizes: &[f64],
        tolerance: Tolerance,
    ) -> f64 {
        let index_of_next_size = pixel_snap_sizes.iter().position(|size| *size >= position);
        if index_of_next_size == Some(0) {
            return pixel_snap_sizes[0];
        }
        let index_of_next_size =
            index_of_next_size.expect("the position is at or below the largest snap size");
        let next_size = pixel_snap_sizes[index_of_next_size];
        // If already snapped - keep this as target size
        if next_size == position {
            return next_size;
        }
        let previous_size = pixel_snap_sizes[index_of_next_size - 1];
        if initial_velocity.abs() <= tolerance.velocity {
            // If velocity is zero, snap to the nearest snap size with the minimum velocity.
            if position - previous_size < next_size - position {
                return previous_size;
            }
            return next_size;
        }
        // Snap forward or backward depending on current velocity.
        if initial_velocity < 0.0 {
            return pixel_snap_sizes[index_of_next_size - 1];
        }
        pixel_snap_sizes[index_of_next_size]
    }
}

impl Simulation for SnappingSimulation {
    fn x(&self, time: f64) -> f64 {
        let new_position = self.position + self.velocity * time;
        if (self.velocity >= 0.0 && new_position > self.pixel_snap_size)
            || (self.velocity < 0.0 && new_position < self.pixel_snap_size)
        {
            // We're passed the snap size, return it instead.
            return self.pixel_snap_size;
        }
        new_position
    }

    fn dx(&self, time: f64) -> f64 {
        if self.is_done(time) {
            return 0.0;
        }
        self.velocity
    }

    fn is_done(&self, time: f64) -> bool {
        self.x(time) == self.pixel_snap_size
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use inset_animation::Curves;
    use inset_embedder::{Offset, Size, TextDirection};
    use inset_gestures::{DragEndDetails, DragUpdateDetails, Velocity};

    use super::*;
    use crate::test_harness::{VIEW_HEIGHT, VIEW_WIDTH, binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::{Builder, Directionality};
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};
    use crate::widgets::single_child_scroll_view::SingleChildScrollView;

    /// The tree every test mounts: a text direction and a media query, which a `Scrollable`
    /// reads for its gesture settings.
    fn wrap<K>(child: impl IntoWidget<K>) -> WidgetRef {
        let query: WidgetRef = MediaQuery::new(
            MediaQueryData::new().size(Size::new(VIEW_WIDTH, VIEW_HEIGHT)),
            child,
        )
        .into_widget();
        Directionality::new(TextDirection::Ltr, query).into_widget()
    }

    /// A builder that puts a scroll view taller than the view under the sheet's controller.
    fn scrollable_builder() -> ScrollableWidgetBuilder {
        Rc::new(|_app, _context, controller| {
            SingleChildScrollView::new()
                .controller(controller)
                .child(SizedBox::new().height(500.0).width(VIEW_WIDTH))
                .into_widget()
        })
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

    fn position(
        app: &App,
        controller: Handle<DraggableScrollableController>,
    ) -> Handle<DraggableScrollableSheetScrollPosition> {
        controller.attached_controller(app).position(app)
    }

    #[test]
    fn the_sheet_starts_at_the_initial_child_size() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = DraggableScrollableController::new(&mut app);
        drop(app);
        binding_mount(
            &cell,
            wrap(
                DraggableScrollableSheet::new(scrollable_builder())
                    .initial_child_size(0.4)
                    .controller(controller),
            ),
        );
        let app = cell.borrow();

        assert!(controller.is_attached(&app));
        assert_eq!(controller.size(&app), 0.4);
        // The whole view height is available, so the sheet is 40% of it.
        assert_eq!(controller.pixels(&app), VIEW_HEIGHT * 0.4);
    }

    #[test]
    fn a_drag_on_the_scrollable_grows_the_sheet_up_to_the_max_child_size() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = DraggableScrollableController::new(&mut app);
        drop(app);
        binding_mount(
            &cell,
            wrap(
                DraggableScrollableSheet::new(scrollable_builder())
                    .initial_child_size(0.5)
                    .controller(controller),
            ),
        );
        let mut app = cell.borrow_mut();

        let position = position(&app, controller);
        let drag = ScrollPosition::drag(
            position,
            &mut app,
            inset_gestures::DragStartDetails::default(),
            Listener::new(|_app| {}),
        );

        // Dragging up by a fifth of the view grows the sheet by a fifth.
        drag.update(&mut app, drag_by(-VIEW_HEIGHT / 5.0));
        assert!(
            (controller.size(&app) - 0.7).abs() < 1e-9,
            "{}",
            controller.size(&app)
        );
        // The sheet, not the list, took the drag.
        assert_eq!(
            inset_rendering::ViewportOffset::pixels(position, &app),
            0.0
        );

        // Past the maximum it stops at the maximum, and then the list scrolls.
        drag.update(&mut app, drag_by(-VIEW_HEIGHT));
        assert_eq!(controller.size(&app), 1.0);
        drag.update(&mut app, drag_by(-10.0));
        assert_eq!(controller.size(&app), 1.0);
        assert!(inset_rendering::ViewportOffset::pixels(position, &app) > 0.0);

        drag.end(
            &mut app,
            DragEndDetails::new(Offset::ZERO, None, Velocity::ZERO, Some(0.0)),
        );
    }

    #[test]
    fn animate_to_reaches_the_size_and_notifies() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = DraggableScrollableController::new(&mut app);
        drop(app);
        binding_mount(
            &cell,
            wrap(
                DraggableScrollableSheet::new(scrollable_builder())
                    .initial_child_size(0.5)
                    .controller(controller),
            ),
        );
        let mut app = cell.borrow_mut();

        let notified = Rc::new(Cell::new(0));
        let counter = Rc::clone(&notified);
        ListenableObject::add_listener(
            controller,
            &mut app,
            Listener::new(move |_app| counter.set(counter.get() + 1)),
        );

        let done =
            controller.animate_to(&mut app, 0.8, Duration::from_millis(100), Curves::linear());

        binding_pump(&mut app, Duration::ZERO);
        binding_pump(&mut app, Duration::from_millis(50));
        assert!(
            (controller.size(&app) - 0.65).abs() < 0.01,
            "{}",
            controller.size(&app)
        );
        assert!(!done.is_completed());

        binding_pump(&mut app, Duration::from_millis(100));
        assert!(
            (controller.size(&app) - 0.8).abs() < 1e-9,
            "{}",
            controller.size(&app)
        );
        assert!(notified.get() > 0);
        // The frame past the duration ends the animation.
        binding_pump(&mut app, Duration::from_millis(150));
        assert!(
            !done.is_completed(),
            "the continuation after the animation runs at the checkpoint"
        );
        drop(app);
        cell.checkpoint();
        assert!(done.is_completed());
    }

    #[test]
    fn a_snapping_sheet_settles_on_the_nearest_snap_size() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = DraggableScrollableController::new(&mut app);
        drop(app);
        binding_mount(
            &cell,
            wrap(
                DraggableScrollableSheet::new(scrollable_builder())
                    .initial_child_size(0.5)
                    .snap(true)
                    .snap_sizes(vec![0.5])
                    .controller(controller),
            ),
        );
        let mut app = cell.borrow_mut();

        let position = position(&app, controller);
        let drag = ScrollPosition::drag(
            position,
            &mut app,
            inset_gestures::DragStartDetails::default(),
            Listener::new(|_app| {}),
        );
        // A drag up past the fling threshold, released with no velocity.
        drag.update(&mut app, drag_by(-VIEW_HEIGHT * 0.15));
        assert!(
            (controller.size(&app) - 0.65).abs() < 1e-9,
            "{}",
            controller.size(&app)
        );
        drag.end(
            &mut app,
            DragEndDetails::new(Offset::ZERO, None, Velocity::ZERO, Some(0.0)),
        );

        for frame in 0..200 {
            binding_pump(&mut app, Duration::from_millis(16 * frame));
            if (controller.size(&app) - 0.5).abs() < 1e-9 {
                break;
            }
        }
        // The snap sizes are the min, the given 0.5 and the max; 0.65 is nearest 0.5.
        assert_eq!(controller.size(&app), 0.5);
    }

    #[test]
    fn the_actuator_resets_the_sheet_to_its_initial_size() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = DraggableScrollableController::new(&mut app);
        let inner_context: Rc<Cell<Option<BuildContext>>> = Rc::new(Cell::new(None));
        let reported = Rc::clone(&inner_context);
        drop(app);
        binding_mount(
            &cell,
            wrap(DraggableScrollableActuator::new(Builder::new(
                move |_app, context| {
                    reported.set(Some(context));
                    DraggableScrollableSheet::new(scrollable_builder())
                        .initial_child_size(0.5)
                        .controller(controller)
                        .into_widget()
                },
            ))),
        );
        let mut app = cell.borrow_mut();

        controller.jump_to(&mut app, 0.9);
        assert_eq!(controller.size(&app), 0.9);

        let context = inner_context.get().expect("the builder ran");
        assert!(DraggableScrollableActuator::reset(&mut app, context));
        binding_pump(&mut app, Duration::from_millis(16));

        assert_eq!(controller.size(&app), 0.5);
    }
}

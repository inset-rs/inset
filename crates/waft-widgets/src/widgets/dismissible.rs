//! Flutter counterpart: `widgets/dismissible.dart`.
//!
//! A [`Dismissible`] slides its child away along the drag, asks
//! [`confirm_dismiss`](Dismissible::confirm_dismiss) whether to go, then collapses its
//! background to nothing before it calls [`on_dismissed`](Dismissible::on_dismissed). The
//! state keeps itself alive while either animation runs.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStatus, AnimationStatusListener,
    AnyAnimation, CurveTween, Curves, Interval, Tween,
};
use waft_embedder::{Offset, Rect, Size, TextDirection};
use waft_foundation::{App, AsyncApp, Handle, Listenable, Listener, Task, VoidCallback};
use waft_gestures::{
    DragEndDetails, DragStartBehavior, DragStartDetails, DragUpdateDetails, Velocity,
};
use waft_painting::Axis;
use waft_rendering::{CustomClipper, HitTestBehavior};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, GlobalKey, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::automatic_keep_alive::{
    AutomaticKeepAliveClientMixin, AutomaticKeepAliveClientMixinData,
};
use crate::widgets::basic::{ClipRect, Directionality, KeyedSubtree, Positioned, SizedBox, Stack};
use crate::widgets::gesture_detector::GestureDetector;
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};
use crate::widgets::transitions::{SizeTransition, SlideTransition};

/// Dart's `_kResizeTimeCurve`.
fn k_resize_time_curve() -> Interval {
    Interval::new(0.4, 1.0, Curves::ease())
}
const K_MIN_FLING_VELOCITY: f64 = 700.0;
const K_MIN_FLING_VELOCITY_DELTA: f64 = 400.0;
const K_FLING_VELOCITY_SCALE: f64 = 1.0 / 300.0;
const K_DISMISS_THRESHOLD: f64 = 0.4;

/// Signature used by [`Dismissible`] to indicate that it has been dismissed in the given
/// `direction`.
///
/// Used by [`Dismissible::on_dismissed`].
pub type DismissDirectionCallback = Rc<dyn Fn(&mut App, DismissDirection)>;

/// Signature used by [`Dismissible`] to give the application an opportunity to confirm or veto
/// a dismiss gesture.
///
/// Used by [`Dismissible::confirm_dismiss`].
pub type ConfirmDismissCallback = Rc<dyn Fn(&mut App, DismissDirection) -> Task<Option<bool>>>;

/// Signature used by [`Dismissible`] to indicate that the dismissible has been dragged.
///
/// Used by [`Dismissible::on_update`].
pub type DismissUpdateCallback = Rc<dyn Fn(&mut App, DismissUpdateDetails)>;

/// The direction in which a [`Dismissible`] can be dismissed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DismissDirection {
    /// The [`Dismissible`] can be dismissed by dragging either up or down.
    Vertical,

    /// The [`Dismissible`] can be dismissed by dragging either left or right.
    Horizontal,

    /// The [`Dismissible`] can be dismissed by dragging in the reverse of the reading direction
    /// (e.g., from right to left in left-to-right languages).
    EndToStart,

    /// The [`Dismissible`] can be dismissed by dragging in the reading direction (e.g., from
    /// left to right in left-to-right languages).
    StartToEnd,

    /// The [`Dismissible`] can be dismissed by dragging up only.
    Up,

    /// The [`Dismissible`] can be dismissed by dragging down only.
    Down,

    /// The [`Dismissible`] cannot be dismissed by dragging.
    None,
}

/// A widget that can be dismissed by dragging in the indicated [`direction`](Self::direction).
///
/// Dragging or flinging this widget in the [`DismissDirection`] causes the child to slide out
/// of view. Following the slide animation, if [`resize_duration`](Self::resize_duration) is
/// non-`None`, the Dismissible widget animates its height (or width, whichever is perpendicular
/// to the dismiss direction) to zero over the [`resize_duration`](Self::resize_duration).
///
/// Backgrounds can be used to implement the "leave-behind" idiom. If a background is specified
/// it is stacked behind the Dismissible's child and is exposed when the child moves.
///
/// The widget calls the [`on_dismissed`](Self::on_dismissed) callback either after its size has
/// collapsed to zero (if [`resize_duration`](Self::resize_duration) is non-`None`) or
/// immediately after the slide animation (if [`resize_duration`](Self::resize_duration) is
/// `None`). If the Dismissible is a list item, it must have a key that distinguishes it from
/// the other items and its [`on_dismissed`](Self::on_dismissed) callback must remove the item
/// from the list.
pub struct Dismissible {
    /// See [`StatefulWidget::key`].
    pub key: KeyRef,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// A widget that is stacked behind the child. If secondary_background is also specified
    /// then this widget only appears when the child has been dragged down or to the right.
    pub background: Option<WidgetRef>,

    /// A widget that is stacked behind the child and is exposed when the child has been dragged
    /// up or to the left. It may only be specified when background has also been specified.
    pub secondary_background: Option<WidgetRef>,

    /// Gives the app an opportunity to confirm or veto a pending dismissal.
    ///
    /// The widget cannot be dragged again until the returned future resolves.
    ///
    /// If the returned task completes `Some(true)`, then this widget will be dismissed,
    /// otherwise it will be moved back to its original location.
    ///
    /// If the returned task completes to `Some(false)` or `None` the
    /// [`on_resize`](Self::on_resize) and [`on_dismissed`](Self::on_dismissed) callbacks will
    /// not run.
    pub confirm_dismiss: Option<ConfirmDismissCallback>,

    /// Called when the widget changes size (i.e., when contracting before being dismissed).
    pub on_resize: Option<VoidCallback>,

    /// Called when the widget has been dismissed, after finishing resizing.
    pub on_dismissed: Option<DismissDirectionCallback>,

    /// The direction in which the widget can be dismissed.
    pub direction: DismissDirection,

    /// The amount of time the widget will spend contracting before
    /// [`on_dismissed`](Self::on_dismissed) is called.
    ///
    /// If `None`, the widget will not contract and [`on_dismissed`](Self::on_dismissed) will be
    /// called immediately after the widget is dismissed.
    pub resize_duration: Option<Duration>,

    /// The offset threshold the item has to be dragged in order to be considered dismissed.
    ///
    /// Represented as a fraction, e.g. if it is 0.4 (the default), then the item has to be
    /// dragged at least 40% towards one direction to be considered dismissed. Clients can
    /// define different thresholds for each dismiss direction.
    ///
    /// Flinging is treated as being equivalent to dragging almost to 1.0, so flinging can
    /// dismiss an item past any threshold less than 1.0.
    ///
    /// Setting a threshold of 1.0 (or greater) prevents a drag in the given [`DismissDirection`]
    /// even if it would be allowed by the [`direction`](Self::direction) property.
    ///
    /// See also:
    ///
    ///  * [`direction`](Self::direction), which controls the directions in which the items can
    ///    be dismissed.
    pub dismiss_thresholds: HashMap<DismissDirection, f64>,

    /// Defines the duration for card to dismiss or to come back to original position if not
    /// dismissed.
    pub movement_duration: Duration,

    /// Defines the end offset across the main axis after the card is dismissed.
    ///
    /// If non-zero value is given then widget moves in cross direction depending on whether it
    /// is positive or negative.
    pub cross_axis_end_offset: f64,

    /// Determines the way that drag start behavior is handled.
    ///
    /// If set to [`DragStartBehavior::Start`], the drag gesture used to dismiss a dismissible
    /// will begin at the position where the drag gesture won the arena. If set to
    /// [`DragStartBehavior::Down`] it will begin at the position where a down event is first
    /// detected.
    ///
    /// In general, setting this to [`DragStartBehavior::Start`] will make drag animation
    /// smoother and setting it to [`DragStartBehavior::Down`] will make drag behavior feel
    /// slightly more reactive.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    pub drag_start_behavior: DragStartBehavior,

    /// How to behave during hit tests.
    ///
    /// This defaults to [`HitTestBehavior::Opaque`].
    pub behavior: HitTestBehavior,

    /// Called when the dismissible widget has been dragged.
    ///
    /// If [`on_update`](Self::on_update) is not `None`, then it will be invoked for every
    /// pointer event to dispatch the latest state of the drag. For example, this callback can be
    /// used to for example change the color of the background widget depending on whether the
    /// dismiss threshold is currently reached.
    pub on_update: Option<DismissUpdateCallback>,
}

impl Dismissible {
    /// Creates a widget that can be dismissed.
    ///
    /// The `key` argument is required because [`Dismissible`]s are commonly used in lists and
    /// removed from the list when dismissed. Without keys, the default behavior is to sync
    /// widgets based on their index in the list, which means the item after the dismissed item
    /// would be synced with the state of the dismissed item. Using keys causes the widgets to
    /// sync according to their keys and avoids this pitfall.
    pub fn new<K>(key: KeyRef, child: impl IntoWidget<K>) -> Dismissible {
        Dismissible {
            key,
            child: child.into_widget(),
            background: None,
            secondary_background: None,
            confirm_dismiss: None,
            on_resize: None,
            on_dismissed: None,
            direction: DismissDirection::Horizontal,
            resize_duration: Some(Duration::from_millis(300)),
            dismiss_thresholds: HashMap::new(),
            movement_duration: Duration::from_millis(200),
            cross_axis_end_offset: 0.0,
            drag_start_behavior: DragStartBehavior::Start,
            behavior: HitTestBehavior::Opaque,
            on_update: None,
        }
    }

    /// Dart `Dismissible(background:)`.
    pub fn background<K>(mut self, background: impl IntoWidget<K>) -> Dismissible {
        self.background = Some(background.into_widget());
        self
    }

    /// Dart `Dismissible(secondaryBackground:)`.
    pub fn secondary_background<K>(
        mut self,
        secondary_background: impl IntoWidget<K>,
    ) -> Dismissible {
        self.secondary_background = Some(secondary_background.into_widget());
        self
    }

    /// Dart `Dismissible(confirmDismiss:)`.
    pub fn confirm_dismiss(
        mut self,
        confirm_dismiss: impl Fn(&mut App, DismissDirection) -> Task<Option<bool>> + 'static,
    ) -> Dismissible {
        self.confirm_dismiss = Some(Rc::new(confirm_dismiss));
        self
    }

    /// Dart `Dismissible(onResize:)`.
    pub fn on_resize(mut self, on_resize: impl Fn(&mut App) + 'static) -> Dismissible {
        self.on_resize = Some(Rc::new(on_resize));
        self
    }

    /// Dart `Dismissible(onUpdate:)`.
    pub fn on_update(
        mut self,
        on_update: impl Fn(&mut App, DismissUpdateDetails) + 'static,
    ) -> Dismissible {
        self.on_update = Some(Rc::new(on_update));
        self
    }

    /// Dart `Dismissible(onDismissed:)`.
    pub fn on_dismissed(
        mut self,
        on_dismissed: impl Fn(&mut App, DismissDirection) + 'static,
    ) -> Dismissible {
        self.on_dismissed = Some(Rc::new(on_dismissed));
        self
    }

    /// Dart `Dismissible(direction:)`.
    pub fn direction(mut self, direction: DismissDirection) -> Dismissible {
        self.direction = direction;
        self
    }

    /// Dart `Dismissible(resizeDuration:)`.
    pub fn resize_duration(mut self, resize_duration: Option<Duration>) -> Dismissible {
        self.resize_duration = resize_duration;
        self
    }

    /// Dart `Dismissible(dismissThresholds:)`.
    pub fn dismiss_thresholds(
        mut self,
        dismiss_thresholds: HashMap<DismissDirection, f64>,
    ) -> Dismissible {
        self.dismiss_thresholds = dismiss_thresholds;
        self
    }

    /// Dart `Dismissible(movementDuration:)`.
    pub fn movement_duration(mut self, movement_duration: Duration) -> Dismissible {
        self.movement_duration = movement_duration;
        self
    }

    /// Dart `Dismissible(crossAxisEndOffset:)`.
    pub fn cross_axis_end_offset(mut self, cross_axis_end_offset: f64) -> Dismissible {
        self.cross_axis_end_offset = cross_axis_end_offset;
        self
    }

    /// Dart `Dismissible(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> Dismissible {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `Dismissible(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Dismissible {
        self.behavior = behavior;
        self
    }
}

impl Debug for Dismissible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dismissible")
            .field("key", &self.key)
            .field("direction", &self.direction)
            .field("resize_duration", &self.resize_duration)
            .field("movement_duration", &self.movement_duration)
            .field("cross_axis_end_offset", &self.cross_axis_end_offset)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Dismissible {
    type State = DismissibleState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.secondary_background.is_none() || self.background.is_some());
    }

    fn create_state(&self) -> DismissibleState {
        DismissibleState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            automatic_keep_alive_client: AutomaticKeepAliveClientMixinData::new(),
            move_controller: None,
            move_animation: None,
            resize_controller: None,
            resize_animation: None,
            drag_extent: 0.0,
            confirming: false,
            drag_underway: false,
            size_prior_to_collapse: None,
            dismiss_threshold_reached: false,
            content_key: GlobalKey::new(),
        }
    }
}

/// Details for [`DismissUpdateCallback`].
///
/// See also:
///
///   * [`Dismissible::on_update`], which receives this information.
#[derive(Clone, Copy, Debug)]
pub struct DismissUpdateDetails {
    /// The direction that the dismissible is being dragged.
    pub direction: DismissDirection,

    /// Whether the dismiss threshold is currently reached.
    pub reached: bool,

    /// Whether the dismiss threshold was reached the last time this callback was invoked.
    ///
    /// This can be used in conjunction with [`reached`](Self::reached) to catch the moment that
    /// the [`Dismissible`] is dragged across the threshold.
    pub previous_reached: bool,

    /// The offset ratio of the dismissible in its parent container.
    ///
    /// A value of 0.0 represents the normal position and 1.0 means the child is completely
    /// outside its parent.
    ///
    /// This can be used to synchronize other elements to what the dismissible is doing on
    /// screen, e.g. using this value to set the opacity thereby fading dismissible as it's
    /// dragged offscreen.
    pub progress: f64,
}

impl DismissUpdateDetails {
    /// Create a new instance of [`DismissUpdateDetails`].
    pub fn new() -> DismissUpdateDetails {
        DismissUpdateDetails {
            direction: DismissDirection::Horizontal,
            reached: false,
            previous_reached: false,
            progress: 0.0,
        }
    }

    /// Dart `DismissUpdateDetails(direction:)`.
    pub fn direction(mut self, direction: DismissDirection) -> DismissUpdateDetails {
        self.direction = direction;
        self
    }

    /// Dart `DismissUpdateDetails(reached:)`.
    pub fn reached(mut self, reached: bool) -> DismissUpdateDetails {
        self.reached = reached;
        self
    }

    /// Dart `DismissUpdateDetails(previousReached:)`.
    pub fn previous_reached(mut self, previous_reached: bool) -> DismissUpdateDetails {
        self.previous_reached = previous_reached;
        self
    }

    /// Dart `DismissUpdateDetails(progress:)`.
    pub fn progress(mut self, progress: f64) -> DismissUpdateDetails {
        self.progress = progress;
        self
    }
}

impl Default for DismissUpdateDetails {
    fn default() -> DismissUpdateDetails {
        DismissUpdateDetails::new()
    }
}

/// Dart's `_DismissibleClipper`.
struct DismissibleClipper {
    axis: Axis,
    move_animation: AnyAnimation<Offset>,
    reclip: Rc<dyn Listenable>,
}

impl DismissibleClipper {
    fn new(axis: Axis, move_animation: AnyAnimation<Offset>) -> DismissibleClipper {
        DismissibleClipper {
            axis,
            move_animation,
            reclip: Rc::new(move_animation),
        }
    }
}

impl CustomClipper<Rect> for DismissibleClipper {
    fn reclip(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.reclip)
    }

    fn get_clip(&self, app: &App, size: Size) -> Rect {
        match self.axis {
            Axis::Horizontal => {
                let offset = self.move_animation.value(app).dx() * size.width();
                if offset < 0.0 {
                    return Rect::from_ltrb(
                        size.width() + offset,
                        0.0,
                        size.width(),
                        size.height(),
                    );
                }
                Rect::from_ltrb(0.0, 0.0, offset, size.height())
            }
            Axis::Vertical => {
                let offset = self.move_animation.value(app).dy() * size.height();
                if offset < 0.0 {
                    return Rect::from_ltrb(
                        0.0,
                        size.height() + offset,
                        size.width(),
                        size.height(),
                    );
                }
                Rect::from_ltrb(0.0, 0.0, size.width(), offset)
            }
        }
    }

    fn should_reclip(&self, app: &App, old_clipper: &dyn CustomClipper<Rect>) -> bool {
        let old_clipper = old_clipper
            .as_any()
            .downcast_ref::<DismissibleClipper>()
            .expect("compared with a clipper of its own type");
        old_clipper.axis != self.axis
            || old_clipper.move_animation.value(app) != self.move_animation.value(app)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_FlingGestureKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlingGestureKind {
    None,
    Forward,
    Reverse,
}

/// Dart's `double.sign`: -1.0, 0.0 or 1.0, where Rust's `signum` answers 1.0 for zero.
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// Dart's `_DismissibleState`.
pub struct DismissibleState {
    state: StateData<Dismissible>,
    ticker_provider: TickerProviderStateMixinData,
    automatic_keep_alive_client: AutomaticKeepAliveClientMixinData,
    // Dart's `late final`: created in `init_state`, which has the `App`.
    move_controller: Option<Handle<AnimationController>>,
    move_animation: Option<AnyAnimation<Offset>>,
    resize_controller: Option<Handle<AnimationController>>,
    resize_animation: Option<AnyAnimation<f64>>,
    drag_extent: f64,
    confirming: bool,
    drag_underway: bool,
    size_prior_to_collapse: Option<Size>,
    dismiss_threshold_reached: bool,
    content_key: GlobalKey,
}

impl DismissibleState {
    fn move_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .move_controller
            .expect("created in init_state")
    }

    fn move_animation(self: Handle<Self>, app: &App) -> AnyAnimation<Offset> {
        app.get(self).move_animation.expect("created in init_state")
    }

    fn direction_is_x_axis(self: Handle<Self>, app: &App) -> bool {
        let direction = self.widget(app).direction;
        direction == DismissDirection::Horizontal
            || direction == DismissDirection::EndToStart
            || direction == DismissDirection::StartToEnd
    }

    fn extent_to_direction(self: Handle<Self>, app: &mut App, extent: f64) -> DismissDirection {
        if extent == 0.0 {
            return DismissDirection::None;
        }
        if self.direction_is_x_axis(app) {
            let context = self.context(app);
            return match Directionality::of(app, context) {
                TextDirection::Rtl if extent < 0.0 => DismissDirection::StartToEnd,
                TextDirection::Ltr if extent > 0.0 => DismissDirection::StartToEnd,
                TextDirection::Rtl | TextDirection::Ltr => DismissDirection::EndToStart,
            };
        }
        if extent > 0.0 {
            DismissDirection::Down
        } else {
            DismissDirection::Up
        }
    }

    fn dismiss_direction(self: Handle<Self>, app: &mut App) -> DismissDirection {
        let drag_extent = app.get(self).drag_extent;
        self.extent_to_direction(app, drag_extent)
    }

    fn dismiss_threshold(self: Handle<Self>, app: &mut App) -> f64 {
        let direction = self.dismiss_direction(app);
        self.widget(app)
            .dismiss_thresholds
            .get(&direction)
            .copied()
            .unwrap_or(K_DISMISS_THRESHOLD)
    }

    fn overall_drag_axis_extent(self: Handle<Self>, app: &App) -> f64 {
        let size = self
            .context(app)
            .size(app)
            .expect("a dragged dismissible has been laid out");
        if self.direction_is_x_axis(app) {
            size.width()
        } else {
            size.height()
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        if app.get(self).confirming {
            return;
        }
        app.get_mut(self).drag_underway = true;
        let move_controller = self.move_controller(app);
        if move_controller.is_animating(app) {
            let value = move_controller.value(app);
            let overall_drag_axis_extent = self.overall_drag_axis_extent(app);
            let drag_extent = app.get(self).drag_extent;
            app.get_mut(self).drag_extent = value * overall_drag_axis_extent * sign(drag_extent);
            move_controller.stop(app, true);
        } else {
            app.get_mut(self).drag_extent = 0.0;
            move_controller.set_value(app, 0.0);
        }
        self.update_move_animation(app);
        self.set_state(app, |_state| {});
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        let move_controller = self.move_controller(app);
        if !app.get(self).drag_underway || move_controller.is_animating(app) {
            return;
        }

        let delta = details
            .primary_delta
            .expect("a drag along one axis has a primary delta");
        let old_drag_extent = app.get(self).drag_extent;
        let drag_extent = old_drag_extent;
        let new_drag_extent = match self.widget(app).direction {
            DismissDirection::Horizontal | DismissDirection::Vertical => drag_extent + delta,

            DismissDirection::Up => {
                if drag_extent + delta < 0.0 {
                    drag_extent + delta
                } else {
                    drag_extent
                }
            }

            DismissDirection::Down => {
                if drag_extent + delta > 0.0 {
                    drag_extent + delta
                } else {
                    drag_extent
                }
            }

            DismissDirection::EndToStart => {
                let context = self.context(app);
                match Directionality::of(app, context) {
                    TextDirection::Rtl => {
                        if drag_extent + delta > 0.0 {
                            drag_extent + delta
                        } else {
                            drag_extent
                        }
                    }
                    TextDirection::Ltr => {
                        if drag_extent + delta < 0.0 {
                            drag_extent + delta
                        } else {
                            drag_extent
                        }
                    }
                }
            }

            DismissDirection::StartToEnd => {
                let context = self.context(app);
                match Directionality::of(app, context) {
                    TextDirection::Rtl => {
                        if drag_extent + delta < 0.0 {
                            drag_extent + delta
                        } else {
                            drag_extent
                        }
                    }
                    TextDirection::Ltr => {
                        if drag_extent + delta > 0.0 {
                            drag_extent + delta
                        } else {
                            drag_extent
                        }
                    }
                }
            }

            DismissDirection::None => 0.0,
        };
        app.get_mut(self).drag_extent = new_drag_extent;
        if sign(old_drag_extent) != sign(new_drag_extent) {
            self.update_move_animation(app);
            self.set_state(app, |_state| {});
        }
        if !move_controller.is_animating(app) {
            let overall_drag_axis_extent = self.overall_drag_axis_extent(app);
            move_controller.set_value(app, new_drag_extent.abs() / overall_drag_axis_extent);
        }
    }

    fn handle_dismiss_update_value_changed(self: Handle<Self>, app: &mut App) {
        if let Some(on_update) = self.widget(app).on_update.clone() {
            let old_dismiss_threshold_reached = app.get(self).dismiss_threshold_reached;
            let value = self.move_controller(app).value(app);
            let dismiss_threshold = self.dismiss_threshold(app);
            app.get_mut(self).dismiss_threshold_reached = value > dismiss_threshold;
            let details = DismissUpdateDetails::new()
                .direction(self.dismiss_direction(app))
                .reached(app.get(self).dismiss_threshold_reached)
                .previous_reached(old_dismiss_threshold_reached)
                .progress(value);
            on_update(app, details);
        }
    }

    fn update_move_animation(self: Handle<Self>, app: &mut App) {
        let end = sign(app.get(self).drag_extent);
        let cross_axis_end_offset = self.widget(app).cross_axis_end_offset;
        let end_offset = if self.direction_is_x_axis(app) {
            Offset::new(end, cross_axis_end_offset)
        } else {
            Offset::new(cross_axis_end_offset, end)
        };
        let tween = Tween::new(app, Some(Offset::ZERO), Some(end_offset));
        let move_animation = self.move_controller(app).drive(app, tween);
        app.get_mut(self).move_animation = Some(move_animation);
    }

    fn describe_fling_gesture(
        self: Handle<Self>,
        app: &mut App,
        velocity: Velocity,
    ) -> FlingGestureKind {
        if app.get(self).drag_extent == 0.0 {
            // If it was a fling, then it was a fling that was let loose at the exact
            // middle of the range (i.e. when there's no displacement). In that case,
            // we assume that the user meant to fling it back to the center, as
            // opposed to having wanted to drag it out one way, then fling it past the
            // center and into and out the other side.
            return FlingGestureKind::None;
        }
        let vx = velocity.pixels_per_second.dx();
        let vy = velocity.pixels_per_second.dy();
        // Verify that the fling is in the generally right direction and fast enough.
        let fling_direction = if self.direction_is_x_axis(app) {
            if vx.abs() - vy.abs() < K_MIN_FLING_VELOCITY_DELTA || vx.abs() < K_MIN_FLING_VELOCITY {
                return FlingGestureKind::None;
            }
            debug_assert!(vx != 0.0);
            self.extent_to_direction(app, vx)
        } else {
            if vy.abs() - vx.abs() < K_MIN_FLING_VELOCITY_DELTA || vy.abs() < K_MIN_FLING_VELOCITY {
                return FlingGestureKind::None;
            }
            debug_assert!(vy != 0.0);
            self.extent_to_direction(app, vy)
        };
        if fling_direction == self.dismiss_direction(app) {
            return FlingGestureKind::Forward;
        }
        FlingGestureKind::Reverse
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        let move_controller = self.move_controller(app);
        if !app.get(self).drag_underway || move_controller.is_animating(app) {
            return;
        }
        app.get_mut(self).drag_underway = false;
        if move_controller.status(app).is_completed() {
            drop(self.handle_move_completed(app));
            return;
        }
        let fling_velocity = if self.direction_is_x_axis(app) {
            details.velocity.pixels_per_second.dx()
        } else {
            details.velocity.pixels_per_second.dy()
        };
        match self.describe_fling_gesture(app, details.velocity) {
            FlingGestureKind::Forward => {
                debug_assert!(app.get(self).drag_extent != 0.0);
                debug_assert!(!move_controller.status(app).is_dismissed());
                if self.dismiss_threshold(app) >= 1.0 {
                    move_controller.reverse(app, None);
                    return;
                }
                app.get_mut(self).drag_extent = sign(fling_velocity);
                move_controller.fling(
                    app,
                    fling_velocity.abs() * K_FLING_VELOCITY_SCALE,
                    None,
                    None,
                );
            }
            FlingGestureKind::Reverse => {
                debug_assert!(app.get(self).drag_extent != 0.0);
                debug_assert!(!move_controller.status(app).is_dismissed());
                app.get_mut(self).drag_extent = sign(fling_velocity);
                move_controller.fling(
                    app,
                    -fling_velocity.abs() * K_FLING_VELOCITY_SCALE,
                    None,
                    None,
                );
            }
            FlingGestureKind::None => {
                if !move_controller.status(app).is_dismissed() {
                    // we already know it's not completed, we check that above
                    if move_controller.value(app) > self.dismiss_threshold(app) {
                        move_controller.forward(app, None);
                    } else {
                        move_controller.reverse(app, None);
                    }
                }
            }
        }
    }

    fn handle_dismiss_status_changed(
        self: Handle<Self>,
        app: &mut App,
        status: AnimationStatus,
    ) -> Task<()> {
        if status.is_completed() && !app.get(self).drag_underway {
            let move_completed = self.handle_move_completed(app);
            return app.spawn(async move |cx| {
                move_completed.await;
                cx.update(|app| {
                    if self.mounted(app) {
                        self.update_keep_alive(app);
                    }
                });
            });
        }
        if self.mounted(app) {
            self.update_keep_alive(app);
        }
        Task::ready(())
    }

    fn handle_move_completed(self: Handle<Self>, app: &mut App) -> Task<()> {
        if self.dismiss_threshold(app) >= 1.0 {
            self.move_controller(app).reverse(app, None);
            return Task::ready(());
        }
        let confirmation = self.confirm_start_resize_animation(app);
        app.spawn(async move |cx| {
            let result = confirmation.await;
            cx.update(|app| {
                if self.mounted(app) {
                    if result {
                        self.start_resize_animation(app);
                    } else {
                        self.move_controller(app).reverse(app, None);
                    }
                }
            });
        })
    }

    fn confirm_start_resize_animation(self: Handle<Self>, app: &mut App) -> Task<bool> {
        if let Some(confirm_dismiss) = self.widget(app).confirm_dismiss.clone() {
            app.get_mut(self).confirming = true;
            let direction = self.dismiss_direction(app);
            let confirmation = confirm_dismiss(app, direction);
            return app.spawn(async move |cx| {
                let _confirming = ConfirmingGuard {
                    state: self,
                    cx: cx.clone(),
                };
                confirmation.await.unwrap_or(false)
            });
        }
        Task::ready(true)
    }

    fn start_resize_animation(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.move_controller(app).status(app).is_completed());
        debug_assert!(app.get(self).resize_controller.is_none());
        debug_assert!(app.get(self).size_prior_to_collapse.is_none());
        match self.widget(app).resize_duration {
            None => {
                if let Some(on_dismissed) = self.widget(app).on_dismissed.clone() {
                    let direction = self.dismiss_direction(app);
                    on_dismissed(app, direction);
                }
            }
            Some(resize_duration) => {
                let resize_controller = AnimationController::create(
                    app,
                    None,
                    Some(resize_duration),
                    None,
                    0.0,
                    1.0,
                    AnimationBehavior::Normal,
                    self,
                );
                resize_controller.add_listener(
                    app,
                    Listener::handle_method(self, Self::handle_resize_progress_changed),
                );
                resize_controller.add_status_listener(
                    app,
                    AnimationStatusListener::new(move |_status, app| self.update_keep_alive(app)),
                );
                app.get_mut(self).resize_controller = Some(resize_controller);
                resize_controller.forward(app, None);
                let size_prior_to_collapse = self.context(app).size(app);
                let curve_tween = CurveTween::new(app, Rc::new(k_resize_time_curve()));
                let curved = resize_controller.drive(app, curve_tween);
                let tween = Tween::new(app, Some(1.0), Some(0.0));
                let resize_animation = curved.drive(app, tween);
                app.get_mut(self).size_prior_to_collapse = size_prior_to_collapse;
                app.get_mut(self).resize_animation = Some(resize_animation);
                self.set_state(app, |_state| {});
            }
        }
    }

    fn handle_resize_progress_changed(self: Handle<Self>, app: &mut App) {
        let resize_controller = app
            .get(self)
            .resize_controller
            .expect("listening to the resize controller");
        if resize_controller.status(app).is_completed() {
            if let Some(on_dismissed) = self.widget(app).on_dismissed.clone() {
                let direction = self.dismiss_direction(app);
                on_dismissed(app, direction);
            }
        } else if let Some(on_resize) = self.widget(app).on_resize.clone() {
            on_resize(app);
        }
    }
}

/// Dart's `finally { _confirming = false; }` around the wait for
/// [`Dismissible::confirm_dismiss`]: it runs however the wait ends.
struct ConfirmingGuard {
    state: Handle<DismissibleState>,
    cx: AsyncApp,
}

impl Drop for ConfirmingGuard {
    fn drop(&mut self) {
        let state = self.state;
        // A state disposed while it waited has no slot to write to.
        self.cx.update(|app| {
            if state.mounted(app) {
                app.get_mut(state).confirming = false;
            }
        });
    }
}

impl TickerProviderObject for DismissibleState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for DismissibleState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl AutomaticKeepAliveClientMixin for DismissibleState {
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData {
        &app.get(self).automatic_keep_alive_client
    }

    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData {
        &mut app.get_mut(self).automatic_keep_alive_client
    }

    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
        let move_controller = self.move_controller(app);
        Animation::is_animating(move_controller, app)
            || app
                .get(self)
                .resize_controller
                .is_some_and(|resize_controller| Animation::is_animating(resize_controller, app))
    }
}

impl State for DismissibleState {
    type Widget = Dismissible;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let movement_duration = self.widget(app).movement_duration;
        let move_controller = AnimationController::create(
            app,
            None,
            Some(movement_duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).move_controller = Some(move_controller);
        AutomaticKeepAliveClientMixin::init_state(self, app);
        move_controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                drop(self.handle_dismiss_status_changed(app, status));
            }),
        );
        move_controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_dismiss_update_value_changed),
        );
        self.update_move_animation(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.move_controller(app).dispose(app);
        if let Some(resize_controller) = app.get(self).resize_controller {
            resize_controller.dispose(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        AutomaticKeepAliveClientMixin::build(self, app); // See AutomaticKeepAliveClientMixin.

        let direction_is_x_axis = self.direction_is_x_axis(app);
        let mut background = self.widget(app).background.clone();
        if self.widget(app).secondary_background.is_some() {
            let direction = self.dismiss_direction(app);
            if direction == DismissDirection::EndToStart || direction == DismissDirection::Up {
                background = self.widget(app).secondary_background.clone();
            }
        }

        if let Some(resize_animation) = app.get(self).resize_animation {
            // we've been dragged aside, and are now resizing.
            if cfg!(debug_assertions) && resize_animation.status(app) != AnimationStatus::Forward {
                assert!(resize_animation.is_completed(app));
                panic!(
                    "A dismissed Dismissible widget is still part of the tree.\n\
                     Make sure to implement the onDismissed handler and to immediately remove \
                     the Dismissible widget from the application once that handler has fired."
                );
            }

            let size_prior_to_collapse = app
                .get(self)
                .size_prior_to_collapse
                .expect("recorded as the resize started");
            let mut sized = SizedBox::new()
                .width(size_prior_to_collapse.width())
                .height(size_prior_to_collapse.height());
            if let Some(background) = background {
                sized = sized.child(background);
            }
            return SizeTransition::new(resize_animation)
                .axis(if direction_is_x_axis {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                })
                .child(sized)
                .into_widget();
        }

        let move_animation = self.move_animation(app);
        let content_key = app.get(self).content_key.clone();
        let child = self.widget(app).child.clone();
        let mut content = SlideTransition::new(move_animation)
            .child(KeyedSubtree::new(child).key(content_key.into()))
            .into_widget();

        if let Some(background) = background {
            let mut children = Vec::new();
            if !move_animation.is_dismissed(app) {
                children.push(
                    Positioned::fill(
                        ClipRect::new()
                            .clipper(DismissibleClipper::new(
                                if direction_is_x_axis {
                                    Axis::Horizontal
                                } else {
                                    Axis::Vertical
                                },
                                move_animation,
                            ))
                            .child(background),
                    )
                    .into_widget(),
                );
            }
            children.push(content);
            content = Stack::new().children(children).into_widget();
        }

        // If the DismissDirection is none, we do not add drag gestures because the content
        // cannot be dragged.
        if self.widget(app).direction == DismissDirection::None {
            return content;
        }

        // We are not resizing but we may be being dragging in widget.direction.
        let on_drag_start = Rc::new(move |app: &mut App, details: DragStartDetails| {
            self.handle_drag_start(app, details);
        });
        let on_drag_update = Rc::new(move |app: &mut App, details: DragUpdateDetails| {
            self.handle_drag_update(app, details);
        });
        let on_drag_end = Rc::new(move |app: &mut App, details: DragEndDetails| {
            self.handle_drag_end(app, details);
        });
        let widget = self.widget(app);
        let detector = GestureDetector::new()
            .behavior(widget.behavior)
            .drag_start_behavior(widget.drag_start_behavior)
            .child(content);
        if direction_is_x_axis {
            detector
                .on_horizontal_drag_start(on_drag_start)
                .on_horizontal_drag_update(on_drag_update)
                .on_horizontal_drag_end(on_drag_end)
                .into_widget()
        } else {
            detector
                .on_vertical_drag_start(on_drag_start)
                .on_vertical_drag_update(on_drag_update)
                .on_vertical_drag_end(on_drag_end)
                .into_widget()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell, RefMut};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use waft_embedder::{Color, PointerDeviceKind, TargetPlatform};
    use waft_foundation::{AppCell, Completer, ValueKey};
    use waft_gestures::{PointerDownEvent, PointerEvent, PointerMoveEvent, PointerUpEvent};
    use waft_painting::{AxisDirection, EdgeInsetsGeometry};
    use waft_rendering::{AnyRenderObject, RenderBox, RenderClipRect, RenderCustomClip};
    use waft_scheduler::SchedulerBinding;
    use waft_test::{TestPlatform, TestView};

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::{AnyElement, StatelessWidget, downcast_widget};
    use crate::test_harness::{platform_view, pump_every_view};
    use crate::widgets::basic::{AspectRatio, Center, Column, Listener, StatefulBuilder};
    use crate::widgets::container::Container;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};
    use crate::widgets::scroll_physics::{NeverScrollableScrollPhysics, ScrollPhysicsRef};
    use crate::widgets::scroll_view::ListView;

    // ---- flutter_test's `WidgetTester`, as far as these tests use it ----

    const DRAG_SLOP_DEFAULT: f64 = 20.0;

    struct WidgetTester {
        cell: Rc<AppCell>,
        now: Duration,
        last_pointer: i64,
    }

    struct TestGesture {
        pointer: i64,
        location: Offset,
    }

    impl WidgetTester {
        fn new() -> WidgetTester {
            let view = TestView::with_pixel_ratio(2400.0, 1800.0, 3.0);
            let platform = TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_view(Rc::new(view));
            WidgetTester {
                cell: AppCell::with_platform(Rc::new(platform)),
                now: Duration::ZERO,
                last_pointer: 0,
            }
        }

        fn app(&self) -> RefMut<'_, App> {
            self.cell.borrow_mut()
        }

        /// `tester.pumpWidget`: attaches `widget` as the root, then one frame.
        fn pump_widget(&mut self, widget: WidgetRef) {
            {
                let mut app = self.app();
                let view = platform_view(&app);
                let binding = WidgetsBinding::instance(&mut app);
                binding.attach_root_widget(
                    &mut app,
                    crate::view::View::new(view, widget).into_widget(),
                );
            }
            self.pump(Duration::ZERO);
        }

        /// `tester.pump(duration)`: the clock moves on by `duration`, then one frame.
        fn pump(&mut self, duration: Duration) {
            self.cell.elapse(duration);
            self.now += duration;
            {
                let mut app = self.app();
                pump_every_view(&mut app, self.now);
                app.drain_microtasks();
            }
            // What a frame's callbacks left to their `async` continuations runs before the next
            // event, as the microtasks after a Dart frame do.
            self.cell.checkpoint();
        }

        /// `tester.pumpAndSettle()`: frames 100ms apart until none is scheduled.
        fn pump_and_settle(&mut self) {
            let end_time = self.now + Duration::from_secs(10 * 60);
            loop {
                assert!(self.now <= end_time, "pumpAndSettle timed out");
                self.pump(Duration::from_millis(100));
                if !SchedulerBinding::has_scheduled_frame(&mut self.app()) {
                    return;
                }
            }
        }

        fn send_event_to_binding(&self, event: PointerEvent) {
            {
                let mut app = self.app();
                waft_gestures::GestureBinding::instance(&mut app)
                    .handle_pointer_event(&mut app, event);
                app.drain_microtasks();
            }
            self.cell.checkpoint();
        }

        fn start_gesture(&mut self, location: Offset) -> TestGesture {
            self.last_pointer += 1;
            let gesture = TestGesture {
                pointer: self.last_pointer,
                location,
            };
            self.send_event_to_binding(PointerEvent::Down(PointerDownEvent {
                pointer: gesture.pointer,
                position: location,
                kind: PointerDeviceKind::Touch,
                ..PointerDownEvent::default()
            }));
            gesture
        }

        fn move_to(&self, gesture: &mut TestGesture, location: Offset, time_stamp: Duration) {
            let delta = location - gesture.location;
            gesture.location = location;
            self.send_event_to_binding(PointerEvent::Move(PointerMoveEvent {
                pointer: gesture.pointer,
                position: location,
                delta,
                time_stamp,
                ..PointerMoveEvent::default()
            }));
        }

        fn move_by(&self, gesture: &mut TestGesture, offset: Offset) {
            self.move_to(gesture, gesture.location + offset, Duration::ZERO);
        }

        fn up(&self, gesture: TestGesture) {
            self.up_at(gesture, Duration::ZERO);
        }

        fn up_at(&self, gesture: TestGesture, time_stamp: Duration) {
            self.send_event_to_binding(PointerEvent::Up(PointerUpEvent {
                pointer: gesture.pointer,
                position: gesture.location,
                time_stamp,
                ..PointerUpEvent::default()
            }));
        }

        /// `tester.tapAt(location)`.
        fn tap_at(&mut self, location: Offset) {
            let gesture = self.start_gesture(location);
            self.up(gesture);
        }

        /// `tester.drag(find.text(text), offset)`: `dragFrom` its center with flutter_test's
        /// default slops, the move split where it leaves the 20 x 20 slop box.
        fn drag(&mut self, text: &str, offset: Offset) {
            let start = self.get_center(text);
            let touch_slop_x = DRAG_SLOP_DEFAULT;
            let touch_slop_y = DRAG_SLOP_DEFAULT;
            let mut gesture = self.start_gesture(start);
            let x_sign = sign(offset.dx());
            let y_sign = sign(offset.dy());
            let offset_x = offset.dx();
            let offset_y = offset.dy();
            let separate_x = offset.dx().abs() > touch_slop_x && touch_slop_x > 0.0;
            let separate_y = offset.dy().abs() > touch_slop_y && touch_slop_y > 0.0;
            if separate_y || separate_x {
                let offset_slope = offset_y / offset_x;
                let inverse_offset_slope = offset_x / offset_y;
                let slop_slope = touch_slop_y / touch_slop_x;
                let absolute_offset_slope = offset_slope.abs();
                let signed_slop_x = touch_slop_x * x_sign;
                let signed_slop_y = touch_slop_y * y_sign;
                if absolute_offset_slope != slop_slope {
                    if absolute_offset_slope < slop_slope {
                        let diff_y = offset_slope.abs() * touch_slop_x * y_sign;
                        self.move_by(&mut gesture, Offset::new(signed_slop_x, diff_y));
                        if offset_y.abs() <= touch_slop_y {
                            self.move_by(
                                &mut gesture,
                                Offset::new(offset_x - signed_slop_x, offset_y - diff_y),
                            );
                        } else {
                            let diff_y2 = signed_slop_y - diff_y;
                            let diff_x2 = inverse_offset_slope * diff_y2;
                            self.move_by(&mut gesture, Offset::new(diff_x2, diff_y2));
                            self.move_by(
                                &mut gesture,
                                Offset::new(
                                    offset_x - diff_x2 - signed_slop_x,
                                    offset_y - signed_slop_y,
                                ),
                            );
                        }
                    } else {
                        let diff_x = inverse_offset_slope.abs() * touch_slop_y * x_sign;
                        self.move_by(&mut gesture, Offset::new(diff_x, signed_slop_y));
                        if offset_x.abs() <= touch_slop_x {
                            self.move_by(
                                &mut gesture,
                                Offset::new(offset_x - diff_x, offset_y - signed_slop_y),
                            );
                        } else {
                            let diff_x2 = signed_slop_x - diff_x;
                            let diff_y2 = offset_slope * diff_x2;
                            self.move_by(&mut gesture, Offset::new(diff_x2, diff_y2));
                            self.move_by(
                                &mut gesture,
                                Offset::new(
                                    offset_x - signed_slop_x,
                                    offset_y - diff_y2 - signed_slop_y,
                                ),
                            );
                        }
                    }
                } else {
                    self.move_by(&mut gesture, Offset::new(signed_slop_x, signed_slop_y));
                    self.move_by(
                        &mut gesture,
                        Offset::new(offset_x - signed_slop_x, offset_y - signed_slop_y),
                    );
                }
            } else {
                self.move_by(&mut gesture, offset);
            }
            self.up(gesture);
        }

        /// `tester.fling(find.text(text), offset, speed, initialOffset:)`: `flingFrom` its
        /// center, 50 moves over `offset` at `speed` pixels a second, with a pump each time
        /// more than 16ms of them have passed.
        fn fling(&mut self, text: &str, offset: Offset, speed: f64, initial_offset: Offset) {
            const MOVE_COUNT: u32 = 50;
            let start = self.get_center(text);
            let frame_interval = Duration::from_millis(16).as_micros() as f64;
            let initial_offset_delay = Duration::from_secs(1);
            let time_stamp_delta = 1000000.0 * offset.distance() / (MOVE_COUNT as f64 * speed);
            let mut time_stamp: f64 = 0.0;
            let mut last_time_stamp = time_stamp;
            let mut gesture = self.start_gesture(start);
            if initial_offset.distance() > 0.0 {
                self.move_to(
                    &mut gesture,
                    start + initial_offset,
                    micros(time_stamp.round()),
                );
                time_stamp += initial_offset_delay.as_micros() as f64;
                self.pump(initial_offset_delay);
            }
            for i in 0..=MOVE_COUNT {
                let location = start
                    + initial_offset
                    + Offset::lerp(
                        Some(Offset::ZERO),
                        Some(offset),
                        i as f64 / MOVE_COUNT as f64,
                    )
                    .expect("both ends are given");
                self.move_to(&mut gesture, location, micros(time_stamp.round()));
                time_stamp += time_stamp_delta;
                if time_stamp - last_time_stamp > frame_interval {
                    self.pump(micros((time_stamp - last_time_stamp).trunc()));
                    last_time_stamp = time_stamp;
                }
            }
            self.up_at(gesture, micros(time_stamp.round()));
        }

        /// Every mounted element, in tree order.
        fn elements(&self) -> Vec<AnyElement> {
            let mut app = self.app();
            let root = WidgetsBinding::instance(&mut app)
                .root_element(&app)
                .expect("a mounted tree");
            let mut found = Vec::new();
            let mut stack = vec![root];
            while let Some(element) = stack.pop() {
                found.push(element);
                let mut children = Vec::new();
                element.visit_children(&app, &mut |child| children.push(child));
                stack.extend(children.into_iter().rev());
            }
            found
        }

        /// The elements of the [`Label`]s showing `text`: `find.text(text)`.
        fn find_text(&self, text: &str) -> Vec<AnyElement> {
            let elements = self.elements();
            let app = self.app();
            elements
                .into_iter()
                .filter(|element| {
                    downcast_widget::<Label>(&**element.widget(&app))
                        .is_some_and(|label| label.0 == text)
                })
                .collect()
        }

        fn count(&self, text: &str) -> usize {
            self.find_text(text).len()
        }

        /// `tester.renderObject(find.text(text))`.
        fn render_object(&self, text: &str) -> AnyRenderObject {
            let element = *self.find_text(text).first().expect("the text is mounted");
            element
                .find_render_object(&self.app())
                .expect("a label has a box")
        }

        /// The global top left and size of the box showing `text`.
        fn rect_of(&self, text: &str) -> (Offset, Size) {
            let object = self.render_object(text);
            let app = self.app();
            let render_box = object.as_box().expect("a label is a box");
            (
                render_box.local_to_global(&app, Offset::ZERO, None),
                render_box.size(&app),
            )
        }

        fn get_top_left(&self, text: &str) -> Offset {
            self.rect_of(text).0
        }

        fn get_top_right(&self, text: &str) -> Offset {
            let (origin, size) = self.rect_of(text);
            origin + Offset::new(size.width(), 0.0)
        }

        fn get_bottom_left(&self, text: &str) -> Offset {
            let (origin, size) = self.rect_of(text);
            origin + Offset::new(0.0, size.height())
        }

        fn get_center(&self, text: &str) -> Offset {
            let (origin, size) = self.rect_of(text);
            origin + Offset::new(size.width() / 2.0, size.height() / 2.0)
        }
    }

    fn micros(micros: f64) -> Duration {
        Duration::from_micros(micros as u64)
    }

    /// Where `find.text` looks: a `Text` that fills its tight constraints as a `Text` does in
    /// these tests, without the fonts a `Text` needs to lay out.
    #[derive(Debug)]
    struct Label(String);

    impl Label {
        fn new(text: impl Into<String>) -> Label {
            Label(text.into())
        }
    }

    impl StatelessWidget for Label {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::expand().into_widget()
        }
    }

    // ---- the test's own `buildTest` and its globals ----

    const DEFAULT_DISMISS_DIRECTION: DismissDirection = DismissDirection::Horizontal;
    const CROSS_AXIS_END_OFFSET: f64 = 0.5;

    /// The test's results: Dart's top-level variables, reset for each test.
    struct Results {
        reported_dismiss_update_reached: Cell<bool>,
        reported_dismiss_update_previous_reached: Cell<bool>,
        reported_dismiss_update_progress: Cell<f64>,
        reported_dismiss_update_reached_direction: Cell<Option<DismissDirection>>,
        reported_dismiss_direction: Cell<DismissDirection>,
        dismissed_items: RefCell<Vec<i64>>,
    }

    impl Results {
        fn new() -> Rc<Results> {
            Rc::new(Results {
                reported_dismiss_update_reached: Cell::new(false),
                reported_dismiss_update_previous_reached: Cell::new(false),
                reported_dismiss_update_progress: Cell::new(0.0),
                reported_dismiss_update_reached_direction: Cell::new(None),
                reported_dismiss_direction: Cell::new(DEFAULT_DISMISS_DIRECTION),
                dismissed_items: RefCell::new(Vec::new()),
            })
        }

        fn dismissed_items(&self) -> Vec<i64> {
            self.dismissed_items.borrow().clone()
        }
    }

    /// `buildTest`'s named arguments.
    struct BuildTest {
        scroll_direction: Axis,
        dismiss_direction: DismissDirection,
        start_to_end_threshold: Option<f64>,
        text_direction: TextDirection,
        confirm_dismiss: Option<ConfirmDismissCallback>,
        controller: Option<Handle<ScrollController>>,
        scroll_physics: Option<ScrollPhysicsRef>,
        background: Option<&'static str>,
    }

    impl Default for BuildTest {
        fn default() -> BuildTest {
            BuildTest {
                scroll_direction: Axis::Vertical,
                dismiss_direction: DEFAULT_DISMISS_DIRECTION,
                start_to_end_threshold: None,
                text_direction: TextDirection::Ltr,
                confirm_dismiss: None,
                controller: None,
                scroll_physics: None,
                background: None,
            }
        }
    }

    fn build_test(results: &Rc<Results>, options: BuildTest) -> WidgetRef {
        let results = Rc::clone(results);
        let options = Rc::new(options);
        Directionality::new(
            options.text_direction,
            StatefulBuilder::new(move |_app, _context, set_state| {
                let build_dismissible_item = |item: i64| {
                    let results_dismissed = Rc::clone(&results);
                    let results_resize = Rc::clone(&results);
                    let results_update = Rc::clone(&results);
                    let set_state = Rc::clone(&set_state);
                    let mut thresholds = HashMap::new();
                    if let Some(threshold) = options.start_to_end_threshold {
                        thresholds.insert(DismissDirection::StartToEnd, threshold);
                    }
                    let mut dismissible = Dismissible::new(
                        ValueKey::new(item).into(),
                        SizedBox::new()
                            .width(100.0)
                            .height(100.0)
                            .child(Label::new(item.to_string())),
                    )
                    .drag_start_behavior(DragStartBehavior::Down)
                    .direction(options.dismiss_direction)
                    .on_dismissed(move |app, direction| {
                        let results = Rc::clone(&results_dismissed);
                        set_state(app, &mut move |_app| {
                            results.reported_dismiss_direction.set(direction);
                            assert!(!results.dismissed_items.borrow().contains(&item));
                            results.dismissed_items.borrow_mut().push(item);
                        });
                    })
                    .on_resize(move |_app| {
                        assert!(!results_resize.dismissed_items.borrow().contains(&item));
                    })
                    .on_update(move |_app, details| {
                        let results = &results_update;
                        results
                            .reported_dismiss_update_reached_direction
                            .set(Some(details.direction));
                        results.reported_dismiss_update_reached.set(details.reached);
                        results
                            .reported_dismiss_update_previous_reached
                            .set(details.previous_reached);
                        results
                            .reported_dismiss_update_progress
                            .set(details.progress);
                    })
                    .dismiss_thresholds(thresholds)
                    .cross_axis_end_offset(CROSS_AXIS_END_OFFSET);
                    if let Some(confirm_dismiss) = options.confirm_dismiss.clone() {
                        dismissible = dismissible
                            .confirm_dismiss(move |app, direction| confirm_dismiss(app, direction));
                    }
                    if let Some(background) = options.background {
                        dismissible = dismissible.background(Label::new(background));
                    }
                    dismissible.into_widget()
                };

                let dismissed_items = results.dismissed_items();
                let mut list = ListView::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .scroll_direction(options.scroll_direction)
                    .item_extent(100.0)
                    .children(
                        (0..9)
                            .filter(|item| !dismissed_items.contains(item))
                            .map(build_dismissible_item)
                            .collect::<Vec<_>>(),
                    );
                if let Some(physics) = options.scroll_physics.clone() {
                    list = list.physics(physics);
                }
                if let Some(controller) = options.controller {
                    list = list.controller(controller.as_controller());
                }
                Container::new()
                    .padding(EdgeInsetsGeometry::all(10.0))
                    .child(list)
                    .into_widget()
            }),
        )
        .into_widget()
    }

    /// The test's `dismissElement`: a drag from one edge of the text to the other.
    fn dismiss_element(tester: &mut WidgetTester, text: &str, gesture_direction: AxisDirection) {
        let (down_location, up_location) = match gesture_direction {
            AxisDirection::Left => (
                // getTopRight() returns a point that's just beyond itemWidget's right
                // edge and outside the Dismissible event listener's bounds.
                tester.get_top_right(text) + Offset::new(-0.1, 0.0),
                tester.get_top_left(text) + Offset::new(-0.1, 0.0),
            ),
            AxisDirection::Right => (
                // we do the same thing here to keep the test symmetric
                tester.get_top_left(text) + Offset::new(0.1, 0.0),
                tester.get_top_right(text) + Offset::new(0.1, 0.0),
            ),
            AxisDirection::Up => (
                // getBottomLeft() returns a point that's just below itemWidget's bottom
                // edge and outside the Dismissible event listener's bounds.
                tester.get_bottom_left(text) + Offset::new(0.0, -0.1),
                tester.get_top_left(text) + Offset::new(0.0, -0.1),
            ),
            AxisDirection::Down => (
                // again with doing the same here for symmetry
                tester.get_top_left(text) + Offset::new(0.1, 0.0),
                tester.get_bottom_left(text) + Offset::new(0.1, 0.0),
            ),
        };

        let mut gesture = tester.start_gesture(down_location);
        tester.move_to(&mut gesture, up_location, Duration::ZERO);
        tester.up(gesture);
    }

    fn axis_delta(gesture_direction: AxisDirection, amount: f64) -> Offset {
        match gesture_direction {
            AxisDirection::Left => Offset::new(-amount, 0.0),
            AxisDirection::Right => Offset::new(amount, 0.0),
            AxisDirection::Up => Offset::new(0.0, -amount),
            AxisDirection::Down => Offset::new(0.0, amount),
        }
    }

    /// The test's `flingElement`.
    fn fling_element(tester: &mut WidgetTester, text: &str, gesture_direction: AxisDirection) {
        let delta = axis_delta(gesture_direction, 300.0);
        tester.fling(text, delta, 1000.0, Offset::ZERO);
    }

    /// The test's `flingElementFromZero`: a drag one way, then a fling back to where it
    /// started.
    fn fling_element_from_zero(
        tester: &mut WidgetTester,
        text: &str,
        gesture_direction: AxisDirection,
    ) {
        let delta = axis_delta(gesture_direction, 300.0);
        tester.fling(text, delta, 1000.0, delta * -1.0);
    }

    /// The test's `rollbackElement`.
    fn rollback_element(tester: &mut WidgetTester, text: &str, gesture_direction: AxisDirection) {
        let delta = axis_delta(gesture_direction, 30.0);
        tester.fling(text, delta, 1000.0, Offset::ZERO);
    }

    type DismissMethod = fn(&mut WidgetTester, &str, AxisDirection);

    fn dismiss_item(
        tester: &mut WidgetTester,
        item: i64,
        gesture_direction: AxisDirection,
        mechanism: DismissMethod,
    ) {
        let text = item.to_string();
        assert_eq!(tester.count(&text), 1);

        mechanism(tester, &text, gesture_direction);
        tester.pump_and_settle();
    }

    fn drag_item(
        tester: &mut WidgetTester,
        item: i64,
        gesture_direction: AxisDirection,
        amount: f64,
    ) {
        let text = item.to_string();
        assert_eq!(tester.count(&text), 1);

        tester.drag(&text, axis_delta(gesture_direction, amount));
        tester.pump(Duration::ZERO);
    }

    fn check_fling_item_before_movement_end(
        tester: &mut WidgetTester,
        item: i64,
        gesture_direction: AxisDirection,
        mechanism: DismissMethod,
    ) {
        let text = item.to_string();
        assert_eq!(tester.count(&text), 1);

        mechanism(tester, &text, gesture_direction);

        tester.pump(Duration::ZERO); // start the slide
        tester.pump(Duration::from_millis(100));
    }

    fn check_fling_item_after_movement(
        tester: &mut WidgetTester,
        item: i64,
        gesture_direction: AxisDirection,
        mechanism: DismissMethod,
    ) {
        let text = item.to_string();
        assert_eq!(tester.count(&text), 1);

        mechanism(tester, &text, gesture_direction);

        tester.pump(Duration::ZERO); // start the slide
        tester.pump(Duration::from_millis(300));
    }

    /// The test's `Test1215DismissibleWidget`.
    #[derive(Debug)]
    struct Test1215DismissibleWidget(&'static str);

    impl StatelessWidget for Test1215DismissibleWidget {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Dismissible::new(
                ValueKey::new(self.0).into(),
                AspectRatio::new(1.0).child(Label::new(self.0)),
            )
            .drag_start_behavior(DragStartBehavior::Down)
            .into_widget()
        }
    }

    /// A `confirmDismiss` that records the direction it was asked about and answers `value`.
    fn confirm_with(
        value: Option<bool>,
        confirm_dismiss_direction: &Rc<Cell<Option<DismissDirection>>>,
    ) -> ConfirmDismissCallback {
        let confirm_dismiss_direction = Rc::clone(confirm_dismiss_direction);
        Rc::new(move |_app, direction| {
            confirm_dismiss_direction.set(Some(direction));
            Task::ready(value)
        })
    }

    /// A `confirmDismiss` that answers what `completer` completes with.
    fn confirm_by(completer: &Rc<RefCell<Completer<Option<bool>>>>) -> ConfirmDismissCallback {
        let completer = Rc::clone(completer);
        Rc::new(move |app, _direction| {
            let future = completer.borrow().future();
            app.spawn(async move |_cx| future.await)
        })
    }

    use AxisDirection::{Down as AxisDown, Left, Right, Up as AxisUp};

    #[test]
    fn a_horizontal_drag_dismisses_in_a_vertical_list() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, BuildTest::default()));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Right, dismiss_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::StartToEnd
        );

        dismiss_item(&mut tester, 1, Left, dismiss_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::EndToStart
        );
    }

    #[test]
    fn a_horizontal_fling_dismisses_in_a_vertical_list() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, BuildTest::default()));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Right, fling_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::StartToEnd
        );

        dismiss_item(&mut tester, 1, Left, fling_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::EndToStart
        );
    }

    #[test]
    fn a_horizontal_fling_does_not_dismiss_at_zero_offset_but_does_otherwise() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                start_to_end_threshold: Some(0.95),
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Right, fling_element_from_zero);
        assert_eq!(tester.count("0"), 1);
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Left, fling_element_from_zero);
        assert_eq!(tester.count("0"), 1);
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Right, fling_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::StartToEnd
        );

        dismiss_item(&mut tester, 1, Left, fling_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::EndToStart
        );
    }

    #[test]
    fn a_vertical_drag_dismisses_in_a_horizontal_list() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Vertical,
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, AxisUp, dismiss_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::Up
        );

        dismiss_item(&mut tester, 1, AxisDown, dismiss_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::Down
        );
    }

    /// The shape of the four one-direction cases: the wrong way first, which leaves the item,
    /// then the right way, which dismisses it.
    fn check_one_way(
        options: BuildTest,
        mechanism: DismissMethod,
        wrong_way: AxisDirection,
        right_way: AxisDirection,
        also_item_one: bool,
    ) {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, options));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, wrong_way, mechanism);
        assert_eq!(tester.count("0"), 1);
        assert!(results.dismissed_items().is_empty());
        if also_item_one {
            dismiss_item(&mut tester, 1, wrong_way, mechanism);
        }

        dismiss_item(&mut tester, 0, right_way, mechanism);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        if also_item_one {
            dismiss_item(&mut tester, 1, right_way, mechanism);
        }
    }

    #[test]
    fn a_drag_left_dismisses_with_end_to_start_in_ltr() {
        check_one_way(
            BuildTest {
                dismiss_direction: DismissDirection::EndToStart,
                ..BuildTest::default()
            },
            dismiss_element,
            Right,
            Left,
            true,
        );
    }

    #[test]
    fn a_drag_right_dismisses_with_start_to_end_in_ltr() {
        check_one_way(
            BuildTest {
                dismiss_direction: DismissDirection::StartToEnd,
                ..BuildTest::default()
            },
            dismiss_element,
            Left,
            Right,
            false,
        );
    }

    #[test]
    fn a_drag_right_dismisses_with_end_to_start_in_rtl() {
        check_one_way(
            BuildTest {
                text_direction: TextDirection::Rtl,
                dismiss_direction: DismissDirection::EndToStart,
                ..BuildTest::default()
            },
            dismiss_element,
            Left,
            Right,
            false,
        );
    }

    #[test]
    fn a_drag_left_dismisses_with_start_to_end_in_rtl() {
        check_one_way(
            BuildTest {
                text_direction: TextDirection::Rtl,
                dismiss_direction: DismissDirection::StartToEnd,
                ..BuildTest::default()
            },
            dismiss_element,
            Right,
            Left,
            true,
        );
    }

    #[test]
    fn a_fling_left_dismisses_with_end_to_start_in_ltr() {
        // Flutter's case drags rather than flings, as written.
        check_one_way(
            BuildTest {
                dismiss_direction: DismissDirection::EndToStart,
                ..BuildTest::default()
            },
            dismiss_element,
            Right,
            Left,
            true,
        );
    }

    #[test]
    fn a_fling_right_dismisses_with_start_to_end_in_ltr() {
        check_one_way(
            BuildTest {
                dismiss_direction: DismissDirection::StartToEnd,
                ..BuildTest::default()
            },
            fling_element,
            Left,
            Right,
            false,
        );
    }

    #[test]
    fn a_fling_right_dismisses_with_end_to_start_in_rtl() {
        check_one_way(
            BuildTest {
                text_direction: TextDirection::Rtl,
                dismiss_direction: DismissDirection::EndToStart,
                ..BuildTest::default()
            },
            fling_element,
            Left,
            Right,
            false,
        );
    }

    #[test]
    fn a_fling_left_dismisses_with_start_to_end_in_rtl() {
        check_one_way(
            BuildTest {
                text_direction: TextDirection::Rtl,
                dismiss_direction: DismissDirection::StartToEnd,
                ..BuildTest::default()
            },
            fling_element,
            Right,
            Left,
            true,
        );
    }

    #[test]
    fn a_drag_up_dismisses_with_up() {
        check_one_way(
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Up,
                ..BuildTest::default()
            },
            dismiss_element,
            AxisDown,
            AxisUp,
            false,
        );
    }

    #[test]
    fn a_drag_down_dismisses_with_down() {
        check_one_way(
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Down,
                ..BuildTest::default()
            },
            dismiss_element,
            AxisUp,
            AxisDown,
            false,
        );
    }

    #[test]
    fn a_fling_up_dismisses_with_up() {
        check_one_way(
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Up,
                ..BuildTest::default()
            },
            fling_element,
            AxisDown,
            AxisUp,
            false,
        );
    }

    #[test]
    fn a_fling_down_dismisses_with_down() {
        check_one_way(
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Down,
                ..BuildTest::default()
            },
            fling_element,
            AxisUp,
            AxisDown,
            false,
        );
    }

    #[test]
    fn a_drag_left_has_no_effect_with_a_high_dismiss_threshold() {
        check_one_way(
            BuildTest {
                start_to_end_threshold: Some(1.0),
                ..BuildTest::default()
            },
            dismiss_element,
            Right,
            Left,
            false,
        );
    }

    #[test]
    fn a_fling_left_has_no_effect_with_a_high_dismiss_threshold() {
        check_one_way(
            BuildTest {
                start_to_end_threshold: Some(1.0),
                ..BuildTest::default()
            },
            fling_element,
            Right,
            Left,
            false,
        );
    }

    #[test]
    fn drag_moves_across_rebuilds_do_not_assert() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                scroll_direction: Axis::Horizontal,
                dismiss_direction: DismissDirection::Down,
                ..BuildTest::default()
            },
        ));
        let location = tester.get_top_left("0");
        let offset = Offset::new(0.0, 5.0);
        let mut gesture = tester.start_gesture(location);
        for _ in 0..4 {
            tester.move_by(&mut gesture, offset);
            tester.pump_widget(build_test(&results, BuildTest::default()));
        }
        tester.up(gesture);
    }

    #[test]
    fn dismissing_the_bottom_then_the_top_item_does_not_throw() {
        let mut tester = WidgetTester::new();
        tester.pump_widget(
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::new().width(100.0).height(1000.0).child(
                    Column::new().children([
                        Test1215DismissibleWidget("1").into_widget(),
                        Test1215DismissibleWidget("2").into_widget(),
                    ]),
                )),
            )
            .into_widget(),
        );
        assert_eq!(tester.count("1"), 1);
        assert_eq!(tester.count("2"), 1);
        dismiss_element(&mut tester, "2", Right);
        tester.pump(Duration::ZERO); // start the slide away
        tester.pump(Duration::from_secs(1)); // finish the slide away
        assert_eq!(tester.count("1"), 1);
        assert_eq!(tester.count("2"), 0);
        dismiss_element(&mut tester, "1", Right);
        tester.pump(Duration::ZERO); // start the slide away
        // finish the slide away (at which point the child is no longer included in the tree)
        tester.pump(Duration::from_secs(1));
        assert_eq!(tester.count("1"), 0);
        assert_eq!(tester.count("2"), 0);
    }

    #[test]
    fn a_dismissible_collapses_from_its_full_size() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                background: Some("background"),
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        assert_eq!(tester.count("0"), 1);
        dismiss_element(&mut tester, "0", Right);
        tester.pump(Duration::ZERO);

        // The other four have been culled.
        assert_eq!(tester.count("background"), 1);
        let (_, size) = tester.rect_of("background");
        assert_eq!(size.height(), 100.0);
    }

    // ---- not in dismissible_test.dart: `_DismissibleClipper` ----

    #[test]
    fn the_background_is_clipped_to_the_part_the_child_has_uncovered() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                background: Some("background"),
                ..BuildTest::default()
            },
        ));
        let start = tester.get_center("0");
        let mut gesture = tester.start_gesture(start);
        tester.move_by(&mut gesture, Offset::new(-30.0, 0.0));
        tester.pump(Duration::ZERO);

        let background = tester.render_object("background");
        let app = tester.app();
        let mut clip = None;
        let mut ancestor = background.parent(&app);
        while let Some(object) = ancestor {
            if let Some(clip_rect) = object.downcast::<RenderClipRect>(&app) {
                clip = Some(clip_rect);
                break;
            }
            ancestor = object.parent(&app);
        }
        let clip = clip.expect("the background sits in a ClipRect");
        let width = clip.size(&app).width();
        assert_eq!(
            RenderCustomClip::<Rect>::clip(clip, &app),
            Rect::from_ltrb(width - 30.0, 0.0, width, 100.0)
        );
        drop(app);
        tester.up(gesture);
    }

    #[test]
    fn a_fling_item_stays_before_the_movement_duration_completes() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, BuildTest::default()));
        assert!(results.dismissed_items().is_empty());

        check_fling_item_before_movement_end(&mut tester, 0, Left, fling_element);
        assert_eq!(tester.count("0"), 1);

        check_fling_item_before_movement_end(&mut tester, 1, Right, fling_element);
        assert_eq!(tester.count("1"), 1);
    }

    #[test]
    fn a_fling_item_is_gone_after_the_movement_duration() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, BuildTest::default()));
        assert!(results.dismissed_items().is_empty());

        check_fling_item_after_movement(&mut tester, 1, Left, fling_element);
        // The resize starts after the `await` on the confirmation, which runs at the next
        // checkpoint rather than among the frame's microtasks (waft-foundation's `Task`
        // entry), so the child is gone a frame later than in Flutter.
        tester.pump(Duration::ZERO);
        assert_eq!(tester.count("1"), 0);

        check_fling_item_after_movement(&mut tester, 0, Right, fling_element);
        // The resize starts after the `await` on the confirmation, which runs at the next
        // checkpoint rather than among the frame's microtasks (waft-foundation's `Task`
        // entry), so the child is gone a frame later than in Flutter.
        tester.pump(Duration::ZERO);
        assert_eq!(tester.count("0"), 0);
    }

    #[test]
    fn a_horizontal_fling_short_of_the_threshold_rolls_back() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                scroll_direction: Axis::Horizontal,
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        check_fling_item_after_movement(&mut tester, 0, Left, rollback_element);
        assert_eq!(tester.count("0"), 1);
        assert!(results.dismissed_items().is_empty());

        check_fling_item_after_movement(&mut tester, 1, Right, rollback_element);
        assert_eq!(tester.count("1"), 1);
        assert!(results.dismissed_items().is_empty());
    }

    #[test]
    fn a_vertical_fling_short_of_the_threshold_rolls_back() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(&results, BuildTest::default()));
        assert!(results.dismissed_items().is_empty());

        check_fling_item_after_movement(&mut tester, 0, Left, rollback_element);
        assert_eq!(tester.count("0"), 1);
        assert!(results.dismissed_items().is_empty());

        check_fling_item_after_movement(&mut tester, 1, Right, rollback_element);
        assert_eq!(tester.count("1"), 1);
        assert!(results.dismissed_items().is_empty());
    }

    #[test]
    fn confirm_dismiss_answers_true_false_and_none() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let confirm_dismiss_direction = Rc::new(Cell::new(None));
        let build_frame = |value: Option<bool>| {
            build_test(
                &results,
                BuildTest {
                    confirm_dismiss: Some(confirm_with(value, &confirm_dismiss_direction)),
                    ..BuildTest::default()
                },
            )
        };

        // Dismiss is confirmed IFF confirmDismiss() returns true.
        tester.pump_widget(build_frame(Some(true)));
        assert!(results.dismissed_items().is_empty());

        dismiss_item(&mut tester, 0, Right, fling_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::StartToEnd
        );
        assert_eq!(
            confirm_dismiss_direction.get(),
            Some(DismissDirection::StartToEnd)
        );

        dismiss_item(&mut tester, 1, Left, fling_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_direction.get(),
            DismissDirection::EndToStart
        );
        assert_eq!(
            confirm_dismiss_direction.get(),
            Some(DismissDirection::EndToStart)
        );

        for value in [Some(false), None] {
            // Dismiss is not confirmed if confirmDismiss() returns false or null.
            results.dismissed_items.borrow_mut().clear();
            tester.pump_widget(build_frame(value));

            dismiss_item(&mut tester, 0, Right, fling_element);
            assert_eq!(tester.count("0"), 1);
            assert!(results.dismissed_items().is_empty());
            assert_eq!(
                confirm_dismiss_direction.get(),
                Some(DismissDirection::StartToEnd)
            );

            dismiss_item(&mut tester, 1, Left, fling_element);
            assert_eq!(tester.count("1"), 1);
            assert!(results.dismissed_items().is_empty());
            assert_eq!(
                confirm_dismiss_direction.get(),
                Some(DismissDirection::EndToStart)
            );
        }
    }

    #[test]
    fn a_pending_confirm_dismiss_does_not_cause_errors() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let completer = Rc::new(RefCell::new(Completer::new()));
        let build_frame = |completer: &Rc<RefCell<Completer<Option<bool>>>>| {
            *completer.borrow_mut() = Completer::new();
            build_test(
                &results,
                BuildTest {
                    confirm_dismiss: Some(confirm_by(completer)),
                    ..BuildTest::default()
                },
            )
        };

        // false and true for _handleDragEnd - when dragged to the end and released, then false
        // and true for _handleDismissStatusChanged - when fling reaches the end.
        for (mechanism, answer) in [
            (dismiss_element as DismissMethod, false),
            (dismiss_element, true),
            (fling_element, false),
            (fling_element, true),
        ] {
            tester.pump_widget(build_frame(&completer));

            dismiss_item(&mut tester, 0, Right, mechanism);
            assert_eq!(tester.count("0"), 1);
            assert!(results.dismissed_items().is_empty());

            tester.pump_widget(SizedBox::new().into_widget());
            completer.borrow().complete(&mut tester.app(), Some(answer));
            tester.pump(Duration::ZERO);
        }
    }

    #[test]
    fn a_dismissible_cannot_be_dragged_while_confirm_dismiss_is_pending() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let completer = Rc::new(RefCell::new(Completer::new()));
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                confirm_dismiss: Some(confirm_by(&completer)),
                ..BuildTest::default()
            },
        ));

        // Trigger confirmDismiss call.
        dismiss_item(&mut tester, 0, Right, dismiss_element);
        let position = tester.get_top_left("0");

        // Try to move and verify it has not moved.
        let drag_at = Offset::new(100.0, tester.get_top_left("0").dy());
        let mut gesture = tester.start_gesture(drag_at);
        tester.move_to(
            &mut gesture,
            drag_at + Offset::new(100.0, 0.0),
            Duration::ZERO,
        );
        tester.up(gesture);
        tester.pump(Duration::ZERO);
        assert_eq!(tester.get_top_left("0"), position);
    }

    #[test]
    fn an_item_dragged_to_the_end_comes_back_when_confirm_dismiss_answers_false() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let completer = Rc::new(RefCell::new(Completer::new()));
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                confirm_dismiss: Some(confirm_by(&completer)),
                ..BuildTest::default()
            },
        ));

        let position = tester.get_top_left("0");
        dismiss_item(&mut tester, 0, Right, dismiss_element);
        completer.borrow().complete(&mut tester.app(), Some(false));
        tester.pump_and_settle();
        assert_eq!(tester.get_top_left("0"), position);
    }

    #[test]
    fn a_null_resize_duration_calls_on_dismissed_at_once() {
        let mut tester = WidgetTester::new();
        let resized = Rc::new(Cell::new(false));
        let dismissed = Rc::new(Cell::new(false));
        let (on_resized, on_dismissed) = (Rc::clone(&resized), Rc::clone(&dismissed));
        tester.pump_widget(
            Directionality::new(
                TextDirection::Ltr,
                Dismissible::new(
                    waft_foundation::UniqueKey::new().into(),
                    SizedBox::new()
                        .width(100.0)
                        .height(100.0)
                        .child(Label::new("0")),
                )
                .drag_start_behavior(DragStartBehavior::Down)
                .resize_duration(None)
                .on_dismissed(move |_app, _direction| on_dismissed.set(true))
                .on_resize(move |_app| on_resized.set(true)),
            )
            .into_widget(),
        );

        dismiss_element(&mut tester, "0", Right);
        tester.pump(Duration::ZERO);
        assert!(dismissed.get());
        assert!(!resized.get());
    }

    #[test]
    fn a_dismissed_dismissible_still_in_the_tree_panics() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let on_dismissed_results = Rc::clone(&results);
        tester.pump_widget(
            Directionality::new(
                TextDirection::Ltr,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let results = Rc::clone(&on_dismissed_results);
                    ListView::new()
                        .drag_start_behavior(DragStartBehavior::Down)
                        .item_extent(100.0)
                        .children([Dismissible::new(
                            ValueKey::new(1i64).into(),
                            SizedBox::new()
                                .width(100.0)
                                .height(100.0)
                                .child(Label::new(1.to_string())),
                        )
                        .drag_start_behavior(DragStartBehavior::Down)
                        .on_dismissed(move |app, direction| {
                            let results = Rc::clone(&results);
                            set_state(app, &mut move |_app| {
                                results.reported_dismiss_direction.set(direction);
                                assert!(!results.dismissed_items.borrow().contains(&1));
                                results.dismissed_items.borrow_mut().push(1);
                            });
                        })
                        .cross_axis_end_offset(CROSS_AXIS_END_OFFSET)
                        .into_widget()])
                        .into_widget()
                }),
            )
            .into_widget(),
        );
        assert!(results.dismissed_items().is_empty());
        let error = catch_unwind(AssertUnwindSafe(|| {
            dismiss_item(&mut tester, 1, Right, dismiss_element)
        }))
        .expect_err("the rebuild that keeps the dismissed Dismissible panics");
        assert_eq!(results.dismissed_items(), [1]);
        let message = match error.downcast::<&str>() {
            Ok(message) => message.to_string(),
            Err(error) => *error.downcast::<String>().expect("a panic message"),
        };
        let mut lines = message.lines();
        assert_eq!(
            lines.next(),
            Some("A dismissed Dismissible widget is still part of the tree.")
        );
        // Dart's last diagnostic, the hint.
        assert_eq!(
            lines.next(),
            Some(
                "Make sure to implement the onDismissed handler and to immediately remove the \
                 Dismissible widget from the application once that handler has fired."
            )
        );
        assert_eq!(lines.next(), None);
    }

    // ---- not in dismissible_test.dart: the constructor's background assert ----

    fn backgrounds_test(dismissible: Dismissible) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, dismissible).into_widget()
    }

    fn backgrounds_item() -> Dismissible {
        Dismissible::new(
            waft_foundation::UniqueKey::new().into(),
            SizedBox::new().width(100.0).height(100.0),
        )
    }

    #[test]
    fn a_secondary_background_may_be_given_before_the_background() {
        let mut tester = WidgetTester::new();
        tester.pump_widget(backgrounds_test(
            backgrounds_item()
                .secondary_background(SizedBox::new())
                .background(SizedBox::new()),
        ));
    }

    #[test]
    #[should_panic(expected = "self.secondary_background.is_none() || self.background.is_some()")]
    fn a_secondary_background_without_a_background_panics_when_made_a_widget() {
        backgrounds_item()
            .secondary_background(SizedBox::new())
            .into_widget();
    }

    #[test]
    fn behavior_decides_how_the_dismissible_is_hit() {
        let mut tester = WidgetTester::new();
        let did_receive_pointer_down = Rc::new(Cell::new(false));
        let build_stack = |child: WidgetRef| {
            let received = Rc::clone(&did_receive_pointer_down);
            Directionality::new(
                TextDirection::Ltr,
                Stack::new().children([
                    Listener::new()
                        .on_pointer_down(Rc::new(move |_app, _event| received.set(true)))
                        .child(
                            Container::new()
                                .width(100.0)
                                .height(100.0)
                                .color(Color::from_argb(0xFF, 0x00, 0xFF, 0x00)),
                        )
                        .into_widget(),
                    child,
                ]),
            )
            .into_widget()
        };
        let dismissible = || {
            Dismissible::new(
                ValueKey::new(1i64).into(),
                SizedBox::new().width(100.0).height(100.0),
            )
        };

        tester.pump_widget(build_stack(dismissible().into_widget()));
        tester.tap_at(Offset::new(10.0, 10.0));
        assert!(!did_receive_pointer_down.get());

        for (behavior, received) in [
            (HitTestBehavior::DeferToChild, true),
            (HitTestBehavior::Opaque, false),
            (HitTestBehavior::Translucent, true),
        ] {
            did_receive_pointer_down.set(false);
            tester.pump_widget(build_stack(dismissible().behavior(behavior).into_widget()));
            tester.tap_at(Offset::new(10.0, 10.0));
            assert_eq!(did_receive_pointer_down.get(), received, "{behavior:?}");
        }
    }

    #[test]
    fn dismiss_direction_none_does_not_dismiss() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                dismiss_direction: DismissDirection::None,
                scroll_physics: Some(Rc::new(NeverScrollableScrollPhysics::new())),
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        for direction in [Left, Right, AxisUp, AxisDown] {
            dismiss_item(&mut tester, 0, direction, dismiss_element);
        }
        assert_eq!(tester.count("0"), 1);
    }

    #[test]
    fn dismiss_direction_none_does_not_prevent_scrolling() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        let controller = ScrollController::default(&mut tester.app());
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                controller: Some(controller),
                dismiss_direction: DismissDirection::None,
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());
        assert_eq!(controller.offset(&tester.app()), 0.0);

        dismiss_item(&mut tester, 0, Left, dismiss_element);
        assert_eq!(controller.offset(&tester.app()), 0.0);
        dismiss_item(&mut tester, 0, Right, dismiss_element);
        assert_eq!(controller.offset(&tester.app()), 0.0);
        dismiss_item(&mut tester, 0, AxisDown, dismiss_element);
        assert_eq!(controller.offset(&tester.app()), 0.0);
        dismiss_item(&mut tester, 0, AxisUp, dismiss_element);
        // Within f32: pointer positions reach the list through valo's f32 matrices
        // (waft-gestures' `push_transform` entry).
        assert!((controller.offset(&tester.app()) - 100.0).abs() < 1e-4);
    }

    #[test]
    fn on_update_reports_the_progress_and_the_threshold() {
        let results = Results::new();
        let mut tester = WidgetTester::new();
        tester.pump_widget(build_test(
            &results,
            BuildTest {
                scroll_direction: Axis::Horizontal,
                ..BuildTest::default()
            },
        ));
        assert!(results.dismissed_items().is_empty());

        // Unsuccessful dismiss, fractional progress reported
        drag_item(&mut tester, 0, Right, 20.0);
        assert_eq!(results.reported_dismiss_update_progress.get(), 0.2);

        // Successful dismiss therefore threshold has been reached
        dismiss_item(&mut tester, 0, Left, fling_element);
        assert_eq!(tester.count("0"), 0);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_update_reached_direction.get(),
            Some(DismissDirection::EndToStart)
        );
        assert!(results.reported_dismiss_update_reached.get());
        assert!(results.reported_dismiss_update_previous_reached.get());
        assert_eq!(results.reported_dismiss_update_progress.get(), 1.0);

        // Unsuccessful dismiss, threshold has not been reached
        check_fling_item_after_movement(&mut tester, 1, Right, rollback_element);
        assert_eq!(tester.count("1"), 1);
        assert_eq!(results.dismissed_items(), [0]);
        assert_eq!(
            results.reported_dismiss_update_reached_direction.get(),
            Some(DismissDirection::StartToEnd)
        );
        assert!(!results.reported_dismiss_update_reached.get());
        assert!(!results.reported_dismiss_update_previous_reached.get());
        assert_eq!(results.reported_dismiss_update_progress.get(), 0.0);

        // Another successful dismiss from another direction
        dismiss_item(&mut tester, 1, Right, fling_element);
        assert_eq!(tester.count("1"), 0);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_update_reached_direction.get(),
            Some(DismissDirection::StartToEnd)
        );
        assert!(results.reported_dismiss_update_reached.get());
        assert!(results.reported_dismiss_update_previous_reached.get());
        assert_eq!(results.reported_dismiss_update_progress.get(), 1.0);

        tester.pump_widget(build_test(
            &results,
            BuildTest {
                scroll_direction: Axis::Horizontal,
                confirm_dismiss: Some(Rc::new(|_app, _direction| Task::ready(Some(false)))),
                ..BuildTest::default()
            },
        ));

        // Threshold has been reached but dismiss was not confirmed
        dismiss_item(&mut tester, 2, Right, fling_element);
        assert_eq!(tester.count("2"), 1);
        assert_eq!(results.dismissed_items(), [0, 1]);
        assert_eq!(
            results.reported_dismiss_update_reached_direction.get(),
            Some(DismissDirection::StartToEnd)
        );
        assert!(!results.reported_dismiss_update_reached.get());
        assert!(!results.reported_dismiss_update_previous_reached.get());
        assert_eq!(results.reported_dismiss_update_progress.get(), 0.0);
    }

    #[test]
    fn changing_the_direction_keeps_the_child_state() {
        let mut tester = WidgetTester::new();
        let build_frame = |direction: DismissDirection| {
            Directionality::new(
                TextDirection::Ltr,
                Dismissible::new(
                    ValueKey::new("Dismissible").into(),
                    SizedBox::new()
                        .width(100.0)
                        .height(100.0)
                        .child(Label::new("I Love Flutter!")),
                )
                .drag_start_behavior(DragStartBehavior::Down)
                .direction(direction)
                .resize_duration(None),
            )
            .into_widget()
        };

        tester.pump_widget(build_frame(DismissDirection::Horizontal));
        let text_render_object_begin = tester.render_object("I Love Flutter!");

        tester.pump_widget(build_frame(DismissDirection::None));
        let text_render_object_end = tester.render_object("I Love Flutter!");

        assert_eq!(text_render_object_begin, text_render_object_end);
    }
}

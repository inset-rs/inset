//! Flutter counterpart: material_ui's `refresh_indicator.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AlwaysStoppedAnimation, Animatable, AnimationBehavior, AnimationController, AnimationRef,
    ColorTween, CurveTween, Curves, Drive, Interval, ValueTween,
};
use waft_embedder::Color;
use waft_foundation::{App, Completer, CompleterFuture, Handle, Listener, Ref, Task, ValueChanged};
use waft_painting::{Alignment, AlignmentDirectional, EdgeInsets};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    Align, AnimatedBuilder, AxisDirection, BuildContext, Container, IntoWidget,
    NotificationListener, OverscrollIndicatorNotification, Padding, Positioned, ScaleTransition,
    ScrollEndNotification, ScrollNotification, ScrollNotificationPredicate,
    ScrollStartNotification, ScrollUpdateNotification, SizeTransition, Stack, State, StateData,
    StatefulWidget, TickerProviderStateMixin, TickerProviderStateMixinData,
    ViewportNotificationMixin, WidgetRef,
};

use crate::progress_indicator::RefreshProgressIndicator;

/// The over-scroll distance that moves the indicator to its maximum displacement, as a
/// percentage of the scrollable's container extent.
const K_DRAG_CONTAINER_EXTENT_PERCENTAGE: f64 = 0.25;

/// How much the scroll's drag gesture can overshoot the indicator's displacement.
const K_DRAG_SIZE_FACTOR_LIMIT: f64 = 1.5;

/// When the scroll ends, the duration of the indicator's animation to its displacement.
const K_INDICATOR_SNAP_DURATION: Duration = Duration::from_millis(150);

/// The duration of the scale transition that starts when the refresh action has completed.
const K_INDICATOR_SCALE_DURATION: Duration = Duration::from_millis(200);

/// The signature for a function that's called when the user has dragged a
/// [`RefreshIndicator`] far enough to demonstrate that they want the app to refresh. The
/// returned task must complete when the refresh operation is finished.
pub type RefreshCallback = Rc<dyn Fn(&mut App) -> Task<()>>;

/// Indicates current status of Material [`RefreshIndicator`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshIndicatorStatus {
    /// Pointer is down.
    Drag,
    /// Dragged far enough that an up event will run the `on_refresh` callback.
    Armed,
    /// Animating to the indicator's final "displacement".
    Snap,
    /// Running the refresh callback.
    Refresh,
    /// Animating the indicator's fade-out after refreshing.
    Done,
    /// Animating the indicator's fade-out after not arming.
    Canceled,
}

/// Used to configure how [`RefreshIndicator`] can be triggered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshIndicatorTriggerMode {
    /// The indicator can be triggered regardless of the scroll position of the scrollable
    /// when the drag starts.
    Anywhere,
    /// The indicator can only be triggered if the scrollable is at the edge when the drag
    /// starts.
    OnEdge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IndicatorType {
    Material,
    Adaptive,
    NoSpinner,
}

/// A widget that supports the Material "swipe to refresh" idiom.
///
/// When the child's scrollable descendant overscrolls, an animated circular progress
/// indicator is faded into view. When the scroll ends, if the indicator has been dragged far
/// enough for it to become completely opaque, the `on_refresh` callback is called. The
/// callback is expected to update the scrollable's contents and then complete the task it
/// returns. The refresh indicator disappears after the callback's task has completed.
///
/// The trigger mode is configured by [`RefreshIndicator::trigger_mode`].
///
/// ## Troubleshooting
///
/// ### Refresh indicator does not show up
///
/// The [`RefreshIndicator`] will appear if its scrollable descendant can be overscrolled,
/// i.e. if the scrollable's content is bigger than its viewport. To ensure that the
/// [`RefreshIndicator`] will always appear, even if the scrollable's content fits within its
/// viewport, set the scrollable's physics to `AlwaysScrollableScrollPhysics`.
///
/// See also:
///
///  * <https://material.io/design/platform-guidance/android-swipe-to-refresh.html>
///  * [`RefreshIndicatorState`], which can be used to programmatically show the refresh
///    indicator.
pub struct RefreshIndicator {
    pub key: Option<waft_widgets::KeyRef>,
    /// The widget below this widget in the tree.
    ///
    /// The refresh indicator will be stacked on top of this child. The indicator will appear
    /// when the child's scrollable descendant is over-scrolled.
    pub child: WidgetRef,
    /// The distance from the child's top or bottom `edge_offset` where the refresh indicator
    /// will settle.
    pub displacement: f64,
    /// The offset where the refresh indicator starts to appear on drag start.
    pub edge_offset: f64,
    /// A function that is called when the user has dragged the refresh indicator far enough
    /// to demonstrate that they want the app to refresh.
    pub on_refresh: RefreshCallback,
    /// Called to get the current status of the refresh indicator to update the UI as needed.
    pub on_status_change: Option<ValueChanged<Option<RefreshIndicatorStatus>>>,
    /// The progress indicator's foreground color.
    pub color: Option<Color>,
    /// The progress indicator's background color.
    pub background_color: Option<Color>,
    /// Whether a [`ScrollNotification`] should be handled by this widget.
    pub notification_predicate: ScrollNotificationPredicate,
    /// The progress indicator's semantics label.
    pub semantics_label: Option<String>,
    /// The progress indicator's semantics value.
    pub semantics_value: Option<String>,
    /// The refresh indicator's stroke width.
    pub stroke_width: f64,
    /// How this refresh indicator can be triggered when users overscroll.
    pub trigger_mode: RefreshIndicatorTriggerMode,
    /// The elevation of the underlying refresh indicator.
    pub elevation: f64,
    indicator_type: IndicatorType,
}

impl RefreshIndicator {
    /// Creates a refresh indicator; Dart's optional named arguments are the setters.
    pub fn new<K>(on_refresh: RefreshCallback, child: impl IntoWidget<K>) -> RefreshIndicator {
        RefreshIndicator {
            key: None,
            child: child.into_widget(),
            displacement: 40.0,
            edge_offset: 0.0,
            on_refresh,
            on_status_change: None,
            color: None,
            background_color: None,
            notification_predicate: Rc::new(waft_widgets::default_scroll_notification_predicate),
            semantics_label: None,
            semantics_value: None,
            stroke_width: RefreshProgressIndicator::DEFAULT_STROKE_WIDTH,
            trigger_mode: RefreshIndicatorTriggerMode::OnEdge,
            elevation: 2.0,
            indicator_type: IndicatorType::Material,
        }
    }

    /// Creates an adaptive refresh indicator: a Cupertino activity indicator on iOS and
    /// macOS, and a Material one elsewhere.
    pub fn adaptive<K>(on_refresh: RefreshCallback, child: impl IntoWidget<K>) -> RefreshIndicator {
        RefreshIndicator {
            indicator_type: IndicatorType::Adaptive,
            ..RefreshIndicator::new(on_refresh, child)
        }
    }

    /// Creates a refresh indicator with no spinner that calls `on_refresh` when successfully
    /// armed by a drag event.
    pub fn no_spinner<K>(
        on_refresh: RefreshCallback,
        child: impl IntoWidget<K>,
    ) -> RefreshIndicator {
        RefreshIndicator {
            displacement: 0.0,
            edge_offset: 0.0,
            color: None,
            background_color: None,
            stroke_width: 0.0,
            indicator_type: IndicatorType::NoSpinner,
            ..RefreshIndicator::new(on_refresh, child)
        }
    }

    /// Dart `RefreshIndicator(key:)`.
    pub fn key(mut self, key: waft_widgets::KeyRef) -> RefreshIndicator {
        self.key = Some(key);
        self
    }

    /// Dart `RefreshIndicator(displacement:)`.
    pub fn displacement(mut self, displacement: f64) -> RefreshIndicator {
        self.displacement = displacement;
        self
    }

    /// Dart `RefreshIndicator(edgeOffset:)`.
    pub fn edge_offset(mut self, edge_offset: f64) -> RefreshIndicator {
        self.edge_offset = edge_offset;
        self
    }

    /// Dart `RefreshIndicator(onStatusChange:)`.
    pub fn on_status_change(
        mut self,
        on_status_change: ValueChanged<Option<RefreshIndicatorStatus>>,
    ) -> RefreshIndicator {
        self.on_status_change = Some(on_status_change);
        self
    }

    /// Dart `RefreshIndicator(color:)`.
    pub fn color(mut self, color: Color) -> RefreshIndicator {
        self.color = Some(color);
        self
    }

    /// Dart `RefreshIndicator(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> RefreshIndicator {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `RefreshIndicator(notificationPredicate:)`.
    pub fn notification_predicate(
        mut self,
        notification_predicate: ScrollNotificationPredicate,
    ) -> RefreshIndicator {
        self.notification_predicate = notification_predicate;
        self
    }

    /// Dart `RefreshIndicator(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: impl Into<String>) -> RefreshIndicator {
        self.semantics_label = Some(semantics_label.into());
        self
    }

    /// Dart `RefreshIndicator(semanticsValue:)`.
    pub fn semantics_value(mut self, semantics_value: impl Into<String>) -> RefreshIndicator {
        self.semantics_value = Some(semantics_value.into());
        self
    }

    /// Dart `RefreshIndicator(strokeWidth:)`.
    pub fn stroke_width(mut self, stroke_width: f64) -> RefreshIndicator {
        self.stroke_width = stroke_width;
        self
    }

    /// Dart `RefreshIndicator(triggerMode:)`.
    pub fn trigger_mode(mut self, trigger_mode: RefreshIndicatorTriggerMode) -> RefreshIndicator {
        self.trigger_mode = trigger_mode;
        self
    }

    /// Dart `RefreshIndicator(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> RefreshIndicator {
        debug_assert!(elevation >= 0.0);
        self.elevation = elevation;
        self
    }
}

impl fmt::Debug for RefreshIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshIndicator")
            .field("displacement", &self.displacement)
            .field("edge_offset", &self.edge_offset)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RefreshIndicator {
    type State = RefreshIndicatorState;

    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RefreshIndicatorState {
        RefreshIndicatorState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            position_controller: None,
            scale_controller: None,
            position_factor: None,
            scale_factor: None,
            value: None,
            value_color: None,
            status: None,
            pending_refresh: None,
            is_indicator_at_top: None,
            drag_offset: None,
            effective_value_color: None,
        }
    }
}

/// Contains the state for a [`RefreshIndicator`]. This class can be used to programmatically
/// show the refresh indicator, see [`show`](Self::show).
pub struct RefreshIndicatorState {
    state: StateData<RefreshIndicator>,
    ticker_provider: TickerProviderStateMixinData,
    position_controller: Option<Ref<AnimationController>>,
    scale_controller: Option<Ref<AnimationController>>,
    position_factor: Option<AnimationRef<f64>>,
    scale_factor: Option<AnimationRef<f64>>,
    value: Option<AnimationRef<f64>>,
    value_color: Option<AnimationRef<Option<Color>>>,
    status: Option<RefreshIndicatorStatus>,
    pending_refresh: Option<Completer<()>>,
    is_indicator_at_top: Option<bool>,
    drag_offset: Option<f64>,
    effective_value_color: Option<Color>,
}

impl RefreshIndicatorState {
    fn position_controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .position_controller
            .clone()
            .expect("created in init_state")
    }

    fn scale_controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .scale_controller
            .clone()
            .expect("created in init_state")
    }

    fn position_factor(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        app.get(self)
            .position_factor
            .clone()
            .expect("created in init_state")
    }

    fn scale_factor(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        app.get(self)
            .scale_factor
            .clone()
            .expect("created in init_state")
    }

    fn value(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        app.get(self).value.clone().expect("created in init_state")
    }

    fn value_color(self: Handle<Self>, app: &App) -> AnimationRef<Option<Color>> {
        app.get(self)
            .value_color
            .clone()
            .expect("set in did_change_dependencies")
    }

    fn effective_value_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .effective_value_color
            .expect("set in did_change_dependencies")
    }

    fn set_status(self: Handle<Self>, app: &mut App, status: Option<RefreshIndicatorStatus>) {
        self.set_state(app, |state| state.status = status);
        let on_status_change = self.widget(app).on_status_change.clone();
        if let Some(on_status_change) = on_status_change {
            on_status_change(app, status);
        }
    }

    fn setup_color_tween(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let theme = crate::theme::Theme::of(app, context);
        let color = self.widget(app).color.unwrap_or(theme.color_scheme.primary);
        app.get_mut(self).effective_value_color = Some(color);
        let value_color: AnimationRef<Option<Color>> = if color.a == 0.0 {
            app.create(AlwaysStoppedAnimation::new(Some(color))).erase()
        } else {
            let color_tween = ColorTween::new(app, Some(color.with_alpha(0)), Some(color));
            let curve_tween = CurveTween::new(
                app,
                Rc::new(Interval::new(
                    0.0,
                    1.0 / K_DRAG_SIZE_FACTOR_LIMIT,
                    Curves::linear(),
                )),
            );
            self.position_controller(app)
                .drive(app, color_tween.chain(curve_tween))
        };
        app.get_mut(self).value_color = Some(value_color);
    }

    fn should_start(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) -> bool {
        let start = notification
            .as_any()
            .downcast_ref::<ScrollStartNotification>();
        let update = notification
            .as_any()
            .downcast_ref::<ScrollUpdateNotification>();
        let dragged = start.is_some_and(|start| start.drag_details.is_some())
            || (update.is_some_and(|update| {
                update.drag_details.is_some()
                    && self.widget(app).trigger_mode == RefreshIndicatorTriggerMode::Anywhere
            }));
        if !dragged || app.get(self).status.is_some() {
            return false;
        }
        let metrics = notification.metrics();
        let at_edge = (metrics.axis_direction() == AxisDirection::Up
            && metrics.extent_after() == 0.0)
            || (metrics.axis_direction() == AxisDirection::Down && metrics.extent_before() == 0.0);
        at_edge && self.start(app, metrics.axis_direction())
    }

    fn handle_scroll_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) -> bool {
        let predicate = self.widget(app).notification_predicate.clone();
        if !predicate(notification) {
            return false;
        }
        if self.should_start(app, notification) {
            self.set_status(app, Some(RefreshIndicatorStatus::Drag));
            return false;
        }

        let indicator_at_top_now = match notification.metrics().axis_direction() {
            AxisDirection::Down | AxisDirection::Up => Some(true),
            AxisDirection::Left | AxisDirection::Right => None,
        };
        if indicator_at_top_now != app.get(self).is_indicator_at_top {
            let status = app.get(self).status;
            if status == Some(RefreshIndicatorStatus::Drag)
                || status == Some(RefreshIndicatorStatus::Armed)
            {
                self.dismiss(app, RefreshIndicatorStatus::Canceled);
            }
        } else if let Some(update) = notification
            .as_any()
            .downcast_ref::<ScrollUpdateNotification>()
        {
            let status = app.get(self).status;
            if status == Some(RefreshIndicatorStatus::Drag)
                || status == Some(RefreshIndicatorStatus::Armed)
            {
                let direction = notification.metrics().axis_direction();
                if direction == AxisDirection::Down {
                    let delta = update.scroll_delta.expect("a scroll update has a delta");
                    app.get_mut(self).drag_offset =
                        Some(app.get(self).drag_offset.expect("a drag is under way") - delta);
                } else if direction == AxisDirection::Up {
                    let delta = update.scroll_delta.expect("a scroll update has a delta");
                    app.get_mut(self).drag_offset =
                        Some(app.get(self).drag_offset.expect("a drag is under way") + delta);
                }
                self.check_drag_offset(app, notification.metrics().viewport_dimension());
            }
            if app.get(self).status == Some(RefreshIndicatorStatus::Armed)
                && update.drag_details.is_none()
            {
                self.show_indicator(app);
            }
        } else if let Some(overscroll) = notification
            .as_any()
            .downcast_ref::<waft_widgets::OverscrollNotification>()
        {
            let status = app.get(self).status;
            if status == Some(RefreshIndicatorStatus::Drag)
                || status == Some(RefreshIndicatorStatus::Armed)
            {
                let direction = notification.metrics().axis_direction();
                if direction == AxisDirection::Down {
                    app.get_mut(self).drag_offset = Some(
                        app.get(self).drag_offset.expect("a drag is under way")
                            - overscroll.overscroll,
                    );
                } else if direction == AxisDirection::Up {
                    app.get_mut(self).drag_offset = Some(
                        app.get(self).drag_offset.expect("a drag is under way")
                            + overscroll.overscroll,
                    );
                }
                self.check_drag_offset(app, notification.metrics().viewport_dimension());
            }
        } else if notification
            .as_any()
            .downcast_ref::<ScrollEndNotification>()
            .is_some()
        {
            match app.get(self).status {
                Some(RefreshIndicatorStatus::Armed) => {
                    if self.position_controller(app).value(app) < 1.0 {
                        self.dismiss(app, RefreshIndicatorStatus::Canceled);
                    } else {
                        self.show_indicator(app);
                    }
                }
                Some(RefreshIndicatorStatus::Drag) => {
                    self.dismiss(app, RefreshIndicatorStatus::Canceled);
                }
                _ => {}
            }
        }
        false
    }

    fn handle_indicator_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &OverscrollIndicatorNotification,
    ) -> bool {
        if notification.depth() != 0 || !notification.leading {
            return false;
        }
        if app.get(self).status == Some(RefreshIndicatorStatus::Drag) {
            notification.disallow_indicator();
            return true;
        }
        false
    }

    fn start(self: Handle<Self>, app: &mut App, direction: AxisDirection) -> bool {
        if app.get(self).status.is_some()
            || app.get(self).is_indicator_at_top.is_some()
            || app.get(self).drag_offset.is_some()
        {
            return false;
        }
        match direction {
            AxisDirection::Down | AxisDirection::Up => {
                app.get_mut(self).is_indicator_at_top = Some(true);
            }
            AxisDirection::Left | AxisDirection::Right => {
                app.get_mut(self).is_indicator_at_top = None;
                // Horizontal scroll views are not supported.
                return false;
            }
        }
        app.get_mut(self).drag_offset = Some(0.0);
        self.scale_controller(app).set_value(app, 0.0);
        self.position_controller(app).set_value(app, 0.0);
        true
    }

    fn check_drag_offset(self: Handle<Self>, app: &mut App, container_extent: f64) {
        let status = app.get(self).status;
        debug_assert!(
            status == Some(RefreshIndicatorStatus::Drag)
                || status == Some(RefreshIndicatorStatus::Armed)
        );
        let mut new_value = app.get(self).drag_offset.expect("a drag is under way")
            / (container_extent * K_DRAG_CONTAINER_EXTENT_PERCENTAGE);
        if status == Some(RefreshIndicatorStatus::Armed) {
            new_value = new_value.max(1.0 / K_DRAG_SIZE_FACTOR_LIMIT);
        }
        self.position_controller(app)
            .set_value(app, new_value.clamp(0.0, 1.0));
        let effective_alpha = self.effective_value_color(app).a;
        let value_alpha = self.value_color(app).value(app).map(|color| color.a);
        if app.get(self).status == Some(RefreshIndicatorStatus::Drag)
            && value_alpha == Some(effective_alpha)
        {
            self.set_status(app, Some(RefreshIndicatorStatus::Armed));
        }
    }

    /// Stops showing the refresh indicator. Dart awaits a completed future first, so the
    /// state change happens on the microtask queue and not during a scroll notification's
    /// layout.
    fn dismiss(self: Handle<Self>, app: &mut App, new_mode: RefreshIndicatorStatus) {
        debug_assert!(
            new_mode == RefreshIndicatorStatus::Canceled
                || new_mode == RefreshIndicatorStatus::Done
        );
        app.schedule_microtask(Listener::new(move |app| {
            if !self.mounted(app) {
                return;
            }
            self.set_status(app, Some(new_mode));
            let controller = match new_mode {
                RefreshIndicatorStatus::Done => self.scale_controller(app),
                RefreshIndicatorStatus::Canceled => self.position_controller(app),
                _ => return,
            };
            let target = if new_mode == RefreshIndicatorStatus::Done {
                1.0
            } else {
                0.0
            };
            let animated = controller.animate_to(
                app,
                target,
                Some(K_INDICATOR_SCALE_DURATION),
                Curves::linear(),
            );
            drop(app.spawn(async move |cx| {
                animated.await;
                cx.update(|app| {
                    if !self.mounted(app) || app.get(self).status != Some(new_mode) {
                        return;
                    }
                    let state = app.get_mut(self);
                    state.drag_offset = None;
                    state.is_indicator_at_top = None;
                    self.set_status(app, None);
                });
            }));
        }));
    }

    fn show_indicator(self: Handle<Self>, app: &mut App) {
        let status = app.get(self).status;
        if status == Some(RefreshIndicatorStatus::Refresh)
            || status == Some(RefreshIndicatorStatus::Snap)
        {
            return;
        }
        let completer = Completer::<()>::new();
        app.get_mut(self).pending_refresh = Some(completer);
        self.set_status(app, Some(RefreshIndicatorStatus::Snap));

        let controller = self.position_controller(app);
        let animated = controller.animate_to(
            app,
            1.0 / K_DRAG_SIZE_FACTOR_LIMIT,
            Some(K_INDICATOR_SNAP_DURATION),
            Curves::linear(),
        );
        let on_refresh = self.widget(app).on_refresh.clone();
        drop(app.spawn(async move |cx| {
            animated.await;
            let ready = cx.update(|app| {
                if !self.mounted(app) || app.get(self).status != Some(RefreshIndicatorStatus::Snap)
                {
                    return false;
                }
                self.set_status(app, Some(RefreshIndicatorStatus::Refresh));
                true
            });
            if !ready {
                return;
            }
            let refresh = cx.update(|app| on_refresh(app));
            refresh.await;
            cx.update(|app| {
                if !self.mounted(app)
                    || app.get(self).status != Some(RefreshIndicatorStatus::Refresh)
                {
                    return;
                }
                if let Some(completer) = app.get(self).pending_refresh.clone() {
                    completer.complete(app, ());
                }
                self.dismiss(app, RefreshIndicatorStatus::Done);
            });
        }));
    }

    /// Show the refresh indicator and run the refresh callback as if it had been started
    /// interactively. If this method is called while the refresh callback is running, it
    /// quietly does nothing.
    ///
    /// The future returned from this method completes when the `on_refresh` callback's task
    /// completes.
    pub fn show(self: Handle<Self>, app: &mut App, at_top: bool) -> CompleterFuture<()> {
        let status = app.get(self).status;
        if status != Some(RefreshIndicatorStatus::Refresh)
            && status != Some(RefreshIndicatorStatus::Snap)
        {
            if status.is_none() {
                self.start(
                    app,
                    if at_top {
                        AxisDirection::Down
                    } else {
                        AxisDirection::Up
                    },
                );
            }
            self.show_indicator(app);
        }
        app.get(self)
            .pending_refresh
            .as_ref()
            .expect("show_indicator made one")
            .future()
    }
}

impl fmt::Debug for RefreshIndicatorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshIndicatorState")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl TickerProviderStateMixin for RefreshIndicatorState {
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

/// Dart's `vsync: this`.
impl TickerProviderObject for RefreshIndicatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for RefreshIndicatorState {
    type Widget = RefreshIndicator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let position_controller = AnimationController::new(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let position_factor =
            position_controller.drive(app, ValueTween::new(0.0, K_DRAG_SIZE_FACTOR_LIMIT));
        let value = position_controller.drive(app, ValueTween::new(0.0, 0.75));
        let scale_controller = AnimationController::new(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let scale_factor = scale_controller.drive(app, ValueTween::new(1.0, 0.0));
        let state = app.get_mut(self);
        state.position_controller = Some(position_controller);
        state.position_factor = Some(position_factor);
        state.value = Some(value);
        state.scale_controller = Some(scale_controller);
        state.scale_factor = Some(scale_factor);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.setup_color_tween(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RefreshIndicator) {
        if old_widget.color != self.widget(app).color {
            self.setup_color_tween(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.position_controller(app).dispose(app);
        self.scale_controller(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(crate::debug::debug_check_has_material_localizations(
            app, context
        ));

        let this = self;
        let inner = NotificationListener::<OverscrollIndicatorNotification>::new(
            self.widget(app).child.clone(),
        )
        .on_notification(move |app, notification| {
            this.handle_indicator_notification(app, notification)
        })
        .into_widget();
        let child = NotificationListener::<dyn ScrollNotification>::new(inner)
            .on_notification(move |app, notification| {
                this.handle_scroll_notification(app, notification)
            })
            .into_widget();

        let status = app.get(self).status;
        let mut children = vec![child];
        if let Some(status) = status {
            let widget = self.widget(app);
            let at_top = app
                .get(self)
                .is_indicator_at_top
                .expect("the indicator has a side while it shows");
            let displacement = widget.displacement;
            let edge_offset = widget.edge_offset;
            let semantics_label = widget.semantics_label.clone();
            let semantics_value = widget.semantics_value.clone();
            let background_color = widget.background_color;
            let color = widget.color;
            let stroke_width = widget.stroke_width;
            let elevation = widget.elevation;
            let indicator_type = widget.indicator_type;
            let position_factor = self.position_factor(app);
            let scale_factor = self.scale_factor(app);
            let position_controller = self.position_controller(app);
            let position_animation: AnimationRef<f64> = position_controller.erase();
            let value = self.value(app);
            let value_color = self.value_color(app);
            let show_indeterminate = matches!(
                status,
                RefreshIndicatorStatus::Refresh | RefreshIndicatorStatus::Done
            );
            let indicator = SizeTransition::new(position_factor)
                .alignment(
                    AlignmentDirectional::new(-1.0, if at_top { 1.0 } else { -1.0 }).into(),
                )
                .child(
                    Padding::new(
                        if at_top {
                            EdgeInsets::only(0.0, displacement, 0.0, 0.0)
                        } else {
                            EdgeInsets::only(0.0, 0.0, 0.0, displacement)
                        }
                        .into(),
                    )
                    .child(
                        Align::new()
                            .alignment(if at_top {
                                Alignment::TOP_CENTER.into()
                            } else {
                                Alignment::BOTTOM_CENTER.into()
                            })
                            .child(
                                ScaleTransition::new(scale_factor).child(
                                    AnimatedBuilder::new(
                                        Rc::new(position_animation),
                                        move |app, context, _child| {
                                            let localizations =
                                                <dyn crate::material_localizations::MaterialLocalizations>::of(
                                                    app, context,
                                                );
                                            let material_indicator =
                                                RefreshProgressIndicator::new()
                                                    .semantics_label(
                                                        semantics_label.clone().unwrap_or_else(|| {
                                                            localizations
                                                                .refresh_indicator_semantic_label()
                                                        }),
                                                    )
                                                    .value_color(value_color.clone())
                                                    .stroke_width(stroke_width)
                                                    .elevation(elevation);
                                            let material_indicator = if show_indeterminate {
                                                material_indicator
                                            } else {
                                                material_indicator.value(value.value(app))
                                            };
                                            let material_indicator = if let Some(
                                                background_color,
                                            ) = background_color
                                            {
                                                material_indicator
                                                    .background_color(background_color)
                                            } else {
                                                material_indicator
                                            };
                                            let material_indicator = if let Some(
                                                semantics_value,
                                            ) = &semantics_value
                                            {
                                                material_indicator
                                                    .semantics_value(semantics_value.clone())
                                            } else {
                                                material_indicator
                                            };

                                            let cupertino_indicator =
                                                waft_cupertino::CupertinoActivityIndicator::new();

                                            match indicator_type {
                                                IndicatorType::Material => {
                                                    material_indicator.into_widget()
                                                }
                                                IndicatorType::Adaptive => {
                                                    let platform =
                                                        crate::theme::Theme::of(app, context)
                                                            .platform;
                                                    match platform {
                                                        waft_embedder::TargetPlatform::IOS
                                                        | waft_embedder::TargetPlatform::MacOS => {
                                                            if let Some(color) = color {
                                                                cupertino_indicator
                                                                    .color(color)
                                                                    .into_widget()
                                                            } else {
                                                                cupertino_indicator.into_widget()
                                                            }
                                                        }
                                                        _ => material_indicator.into_widget(),
                                                    }
                                                }
                                                IndicatorType::NoSpinner => {
                                                    Container::new().into_widget()
                                                }
                                            }
                                        },
                                    )
                                    .into_widget(),
                                ),
                            ),
                    ),
                );

            let mut positioned = Positioned::new(indicator);
            if at_top {
                positioned = positioned.top(edge_offset);
            } else {
                positioned = positioned.bottom(edge_offset);
            }
            children.push(positioned.left(0.0).right(0.0).into_widget());
        }

        Stack::new().children(children).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::{Offset, Size, TargetPlatform};
    use waft_widgets::testing::{
        TargetPlatformVariant, WidgetTester, find, test_widgets, test_widgets_variant,
    };
    use waft_widgets::{
        AlwaysScrollableScrollPhysics, Center, IntoWidget, ListView, SizedBox, StatefulElement,
        Text,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn refresh_called() -> (RefreshCallback, Rc<Cell<bool>>) {
        let called = Rc::new(Cell::new(false));
        let seen = Rc::clone(&called);
        let callback: RefreshCallback = Rc::new(move |_app| {
            seen.set(true);
            Task::ready(())
        });
        (callback, called)
    }

    /// Dart's `holdRefresh`: a refresh that never completes.
    fn hold_refresh() -> (RefreshCallback, Rc<Cell<bool>>) {
        let called = Rc::new(Cell::new(false));
        let seen = Rc::clone(&called);
        let callback: RefreshCallback = Rc::new(move |app| {
            seen.set(true);
            let completer = Completer::<()>::new();
            let future = completer.future();
            app.spawn(async move |_cx| {
                let _keep = completer;
                future.await
            })
        });
        (callback, called)
    }

    fn list() -> ListView {
        ListView::new()
            .physics(Rc::new(AlwaysScrollableScrollPhysics::new()))
            .children(
                ["A", "B", "C", "D", "E", "F"]
                    .into_iter()
                    .map(|text| {
                        SizedBox::new()
                            .height(200.0)
                            .child(Text::new(text))
                            .into_widget()
                    })
                    .collect::<Vec<_>>(),
            )
    }

    fn pump(tester: &mut WidgetTester, refresh: RefreshCallback, theme: ThemeData) {
        tester.pump_widget(material_app(
            theme,
            RefreshIndicator::new(refresh, list()).into_widget(),
        ));
    }

    fn state(tester: &WidgetTester) -> Handle<RefreshIndicatorState> {
        let element = tester.element(find::by_type::<RefreshIndicator>());
        let app = tester.cell.borrow();
        element
            .downcast::<StatefulElement<RefreshIndicator>>(&app)
            .expect("a refresh indicator's element")
            .state(&app)
    }

    #[test]
    fn a_drag_down_runs_the_refresh_callback() {
        test_widgets(|tester| {
            let (refresh, called) = refresh_called();
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            pump(tester, refresh, theme);

            tester.fling(find::text("A"), Offset::new(0.0, 300.0), 1000.0);
            tester.pump();
            assert_eq!(
                find::by_type::<RefreshProgressIndicator>()
                    .evaluate(tester)
                    .len(),
                1,
                "the indicator came in"
            );

            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            assert!(called.get(), "the refresh ran");
        });
    }

    #[test]
    fn show_runs_the_refresh_and_completes_after_it() {
        test_widgets(|tester| {
            let (refresh, called) = refresh_called();
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            pump(tester, refresh, theme);

            let completed = Rc::new(Cell::new(false));
            let seen = Rc::clone(&completed);
            let future = state(tester).show(&mut tester.cell.borrow_mut(), true);
            future.then(&mut tester.cell.borrow_mut(), move |_app, ()| {
                seen.set(true);
            });
            tester.pump();
            assert!(!completed.get());

            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            assert!(called.get());
            assert!(completed.get(), "the future completed after the refresh");
        });
    }

    #[test]
    fn show_waits_for_a_refresh_that_never_completes() {
        test_widgets(|tester| {
            let (refresh, called) = hold_refresh();
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            pump(tester, refresh, theme);

            let completed = Rc::new(Cell::new(false));
            let seen = Rc::clone(&completed);
            let future = state(tester).show(&mut tester.cell.borrow_mut(), true);
            future.then(&mut tester.cell.borrow_mut(), move |_app, ()| {
                seen.set(true);
            });
            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            assert!(called.get());
            assert!(!completed.get(), "the refresh never completed");

            called.set(false);
            let again = state(tester).show(&mut tester.cell.borrow_mut(), true);
            drop(again);
            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            assert!(!called.get(), "a second show while refreshing does nothing");
        });
    }

    #[test]
    fn no_spinner_never_builds_a_progress_indicator() {
        test_widgets(|tester| {
            let (refresh, called) = refresh_called();
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                RefreshIndicator::no_spinner(refresh, list()).into_widget(),
            ));
            tester.fling(find::text("A"), Offset::new(0.0, 300.0), 1000.0);
            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            assert!(called.get());
            assert!(
                find::by_type::<RefreshProgressIndicator>()
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    #[test]
    fn adaptive_shows_a_cupertino_indicator_on_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let (refresh, _called) = hold_refresh();
            let theme = ThemeData::builder()
                .platform(TargetPlatform::IOS)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                RefreshIndicator::adaptive(refresh, list()).into_widget(),
            ));
            drop(state(tester).show(&mut tester.cell.borrow_mut(), true));
            tester.pump();
            assert_eq!(
                find::by_type::<waft_cupertino::CupertinoActivityIndicator>()
                    .evaluate(tester)
                    .len(),
                1
            );
            assert!(
                find::by_type::<RefreshProgressIndicator>()
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    #[test]
    fn the_value_color_defaults_to_the_color_scheme_primary() {
        test_widgets(|tester| {
            let primary = Color::new(0xFF4CAF50);
            let mut scheme = ColorScheme::light();
            scheme.primary = primary;
            let theme = ThemeData::builder()
                .color_scheme(scheme)
                .build(&tester.cell.borrow());
            let (refresh, _called) = hold_refresh();
            tester.pump_widget(material_app(
                theme,
                RefreshIndicator::new(refresh, list())
                    .trigger_mode(RefreshIndicatorTriggerMode::Anywhere)
                    .into_widget(),
            ));

            tester.fling(find::text("A"), Offset::new(0.0, 300.0), 1000.0);
            tester.pump();
            let value_color = tester
                .widget(
                    find::by_type::<RefreshProgressIndicator>(),
                    |indicator: &RefreshProgressIndicator| {
                        indicator
                            .circular_progress_indicator
                            .progress_indicator
                            .value_color
                            .clone()
                    },
                )
                .expect("the indicator has a value color");
            let app = tester.cell.borrow();
            assert_eq!(value_color.value(&app), Some(primary));
        });
    }

    #[test]
    fn the_status_callback_reports_each_stage() {
        test_widgets(|tester| {
            let (refresh, called) = refresh_called();
            let drag = Rc::new(Cell::new(false));
            let armed = Rc::new(Cell::new(false));
            let snap = Rc::new(Cell::new(false));
            let done = Rc::new(Cell::new(false));
            let (d, a, s2, dn) = (
                Rc::clone(&drag),
                Rc::clone(&armed),
                Rc::clone(&snap),
                Rc::clone(&done),
            );
            let statuses = Rc::new(
                move |_app: &mut App, status: Option<RefreshIndicatorStatus>| match status {
                    Some(RefreshIndicatorStatus::Drag) => d.set(true),
                    Some(RefreshIndicatorStatus::Armed) => a.set(true),
                    Some(RefreshIndicatorStatus::Snap) => s2.set(true),
                    Some(RefreshIndicatorStatus::Done) => dn.set(true),
                    _ => {}
                },
            );
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                RefreshIndicator::no_spinner(refresh, list())
                    .on_status_change(statuses)
                    .into_widget(),
            ));
            tester.fling(find::text("A"), Offset::new(0.0, 300.0), 1000.0);
            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            tester.pump_by(Duration::from_secs(1));
            assert!(called.get());
            assert!(drag.get(), "drag");
            assert!(armed.get(), "armed");
            assert!(snap.get(), "snap");
            assert!(done.get(), "done");
        });
    }

    #[test]
    fn the_indicator_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let (refresh, _called) = refresh_called();
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        SizedBox::shrink()
                            .child(RefreshIndicator::new(refresh, SizedBox::shrink())),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<RefreshIndicator>()),
                Size::ZERO
            );
        });
    }
}

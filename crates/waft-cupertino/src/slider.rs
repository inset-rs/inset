//! Flutter counterpart: `cupertino_ui/slider.dart`.
//!
//! [`CupertinoSlider`] builds a private leaf render object, Dart's `_RenderCupertinoSlider`,
//! which extends `RenderConstrainedBox`: it holds the constrained box's fields itself and
//! repeats its layout and intrinsics, so a change to `RenderConstrainedBox` must be made here
//! too. The semantics configuration and its increase and decrease actions wait with
//! accessibility, and the haptics follow the platform `App` reports.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Animation, AnimationBehavior, AnimationController, Curves};
use waft_embedder::{
    Color, Matrix4, Offset, Paint, RRect, Rect, Size, TargetPlatform, TextBaseline, TextDirection,
    clamp_double, lerp_double,
};
use waft_foundation::{App, Handle, K_IS_WEB, Listener, ValueChanged};
use waft_gestures::{
    DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    HorizontalDragGestureRecognizer, PointerEnterEventListener, PointerEvent,
    PointerExitEventListener, PointerHoverEventListener, RecognizerLeaf,
};
use waft_painting::{AnyColor, ClipContext, draw_rrect};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestEntry, BoxHitTestResult, Constraints,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectBase,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    TickerProviderRef,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{
    HapticFeedback, MouseCursor, MouseCursorRef, MouseTrackerAnnotation, SystemMouseCursors,
};
use waft_widgets::{
    BuildContext, Directionality, IntoWidget, KeyRef, LeafRenderObjectWidget, Opacity,
    RenderObjectWidget, State, StateData, StatefulWidget, TickerProviderStateMixin,
    TickerProviderStateMixinData, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::theme::CupertinoTheme;
use crate::thumb_painter::CupertinoThumbPainter;

/// Dart's `_SliderValueChanged`: the new value and whether the drag was fast.
type SliderValueChanged = Rc<dyn Fn(&mut App, f64, bool)>;

// Matches the disabled visual opacity of the native iOS UISlider.
const K_DISABLED_OPACITY: f64 = 0.5;

/// Defines the threshold for determining a "fast" slider drag.
///
/// Measured in slider extent per second.
///
/// For example, a threshold of 1.0 means that the user must drag with a velocity that will
/// move the slider from start to end in 1 second.
///
/// A threshold of 0.5 means that the user must drag with a velocity that will move the slider
/// 50% in 1 second.
///
/// This value is estimated using a physical iPhone 15 Pro running iOS 18.
const K_VELOCITY_THRESHOLD: f64 = 1.0;

/// An iOS-style slider.
///
/// Used to select from a range of values.
///
/// A slider can be used to select from either a continuous or a discrete set of values. The
/// default is use a continuous range of values from [`min`](Self::min) to [`max`](Self::max).
/// To use discrete values, use a non-null value for [`divisions`](Self::divisions), which
/// indicates the number of discrete intervals. For example, if [`min`](Self::min) is 0.0 and
/// [`max`](Self::max) is 50.0 and [`divisions`](Self::divisions) is 5, then the slider can
/// take on the values discrete values 0.0, 10.0, 20.0, 30.0, 40.0, and 50.0.
///
/// The slider itself does not maintain any state. Instead, when the state of the slider
/// changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most widgets that
/// use a slider will listen for the [`on_changed`](Self::on_changed) callback and rebuild the
/// slider with a new [`value`](Self::value) to update the visual appearance of the slider.
///
/// See also:
///
///  * <https://developer.apple.com/design/human-interface-guidelines/sliders/>
#[derive(Clone)]
pub struct CupertinoSlider {
    pub key: Option<KeyRef>,
    /// The currently selected value for this slider.
    ///
    /// The slider's thumb is drawn at a position that corresponds to this value.
    pub value: f64,
    /// Called when the user selects a new value for the slider.
    ///
    /// The slider passes the new value to the callback but does not actually change state
    /// until the parent widget rebuilds the slider with the new value.
    ///
    /// If null, the slider will be displayed as disabled.
    ///
    /// The callback provided to on_changed should update the state of the parent
    /// `StatefulWidget` using the `State::set_state` method, so that the parent gets rebuilt.
    ///
    /// See also:
    ///
    ///  * [`on_change_start`](Self::on_change_start) for a callback that is called when the
    ///    user starts changing the value.
    ///  * [`on_change_end`](Self::on_change_end) for a callback that is called when the user
    ///    stops changing the value.
    pub on_changed: Option<ValueChanged<f64>>,
    /// Called when the user starts selecting a new value for the slider.
    ///
    /// This callback shouldn't be used to update the slider [`value`](Self::value) (use
    /// [`on_changed`](Self::on_changed) for that), but rather to be notified when the user has
    /// started selecting a new value by starting a drag.
    ///
    /// The value passed will be the last [`value`](Self::value) that the slider had before the
    /// change began.
    ///
    /// See also:
    ///
    ///  * [`on_change_end`](Self::on_change_end) for a callback that is called when the value
    ///    change is complete.
    pub on_change_start: Option<ValueChanged<f64>>,
    /// Called when the user is done selecting a new value for the slider.
    ///
    /// This callback shouldn't be used to update the slider [`value`](Self::value) (use
    /// [`on_changed`](Self::on_changed) for that), but rather to know when the user has
    /// completed selecting a new [`value`](Self::value) by ending a drag.
    ///
    /// See also:
    ///
    ///  * [`on_change_start`](Self::on_change_start) for a callback that is called when a
    ///    value change begins.
    pub on_change_end: Option<ValueChanged<f64>>,
    /// The minimum value the user can select.
    ///
    /// Defaults to 0.0.
    pub min: f64,
    /// The maximum value the user can select.
    ///
    /// Defaults to 1.0.
    pub max: f64,
    /// The number of discrete divisions.
    ///
    /// If null, the slider is continuous.
    pub divisions: Option<i32>,
    /// The color to use for the portion of the slider that has been selected.
    ///
    /// Defaults to the `CupertinoTheme`'s primary color if null.
    pub active_color: Option<AnyColor>,
    /// The color to use for the thumb of the slider.
    ///
    /// Defaults to [`CupertinoColors::WHITE`].
    pub thumb_color: AnyColor,
}

impl CupertinoSlider {
    /// Creates an iOS-style slider.
    ///
    /// The slider itself does not maintain any state. Instead, when the state of the slider
    /// changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most widgets
    /// that use a slider will listen for the [`on_changed`](Self::on_changed) callback and
    /// rebuild the slider with a new [`value`](Self::value) to update the visual appearance of
    /// the slider.
    ///
    /// * `value` determines currently selected value for this slider.
    /// * `on_changed` is called when the user selects a new value for the slider.
    /// * [`on_change_start`](Self::on_change_start) is called when the user starts to select a
    ///   new value for the slider.
    /// * [`on_change_end`](Self::on_change_end) is called when the user is done selecting a new
    ///   value for the slider.
    pub fn new(value: f64, on_changed: Option<ValueChanged<f64>>) -> CupertinoSlider {
        CupertinoSlider {
            key: None,
            value,
            on_changed,
            on_change_start: None,
            on_change_end: None,
            min: 0.0,
            max: 1.0,
            divisions: None,
            active_color: None,
            thumb_color: CupertinoColors::WHITE,
        }
    }

    /// Dart `CupertinoSlider(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSlider {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSlider(onChangeStart:)`.
    pub fn on_change_start(mut self, on_change_start: ValueChanged<f64>) -> CupertinoSlider {
        self.on_change_start = Some(on_change_start);
        self
    }

    /// Dart `CupertinoSlider(onChangeEnd:)`.
    pub fn on_change_end(mut self, on_change_end: ValueChanged<f64>) -> CupertinoSlider {
        self.on_change_end = Some(on_change_end);
        self
    }

    /// Dart `CupertinoSlider(min:)`.
    pub fn min(mut self, min: f64) -> CupertinoSlider {
        self.min = min;
        self
    }

    /// Dart `CupertinoSlider(max:)`.
    pub fn max(mut self, max: f64) -> CupertinoSlider {
        self.max = max;
        self
    }

    /// Dart `CupertinoSlider(divisions:)`.
    pub fn divisions(mut self, divisions: i32) -> CupertinoSlider {
        debug_assert!(divisions > 0);
        self.divisions = Some(divisions);
        self
    }

    /// Dart `CupertinoSlider(activeColor:)`.
    pub fn active_color(mut self, active_color: impl Into<AnyColor>) -> CupertinoSlider {
        self.active_color = Some(active_color.into());
        self
    }

    /// Dart `CupertinoSlider(thumbColor:)`.
    pub fn thumb_color(mut self, thumb_color: impl Into<AnyColor>) -> CupertinoSlider {
        self.thumb_color = thumb_color.into();
        self
    }
}

impl Debug for CupertinoSlider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoSlider")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoSlider {
    type State = CupertinoSliderState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.value >= self.min && self.value <= self.max);
    }

    fn create_state(&self) -> CupertinoSliderState {
        CupertinoSliderState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
        }
    }
}

/// Dart's `_CupertinoSliderState`.
pub struct CupertinoSliderState {
    state: StateData<CupertinoSlider>,
    ticker_provider: TickerProviderStateMixinData,
}

impl CupertinoSliderState {
    fn handle_changed(self: Handle<Self>, app: &mut App, value: f64, is_fast_drag: bool) {
        let widget = self.widget(app);
        debug_assert!(widget.on_changed.is_some());
        let (min, max, current) = (widget.min, widget.max, widget.value);
        let on_changed = widget.on_changed.clone();
        let lerp_value = lerp_double(Some(min), Some(max), value).expect("min and max are set");
        let is_at_edge = lerp_value == max || lerp_value == min;

        if lerp_value != current {
            if is_at_edge {
                Self::emit_haptic_feedback(app, is_fast_drag);
            }
            if let Some(on_changed) = on_changed {
                on_changed(app, lerp_value);
            }
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, value: f64) {
        let widget = self.widget(app);
        debug_assert!(widget.on_change_start.is_some());
        let (min, max) = (widget.min, widget.max);
        if let Some(on_change_start) = widget.on_change_start.clone() {
            on_change_start(
                app,
                lerp_double(Some(min), Some(max), value).expect("min and max are set"),
            );
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, value: f64) {
        let widget = self.widget(app);
        debug_assert!(widget.on_change_end.is_some());
        let (min, max) = (widget.min, widget.max);
        if let Some(on_change_end) = widget.on_change_end.clone() {
            on_change_end(
                app,
                lerp_double(Some(min), Some(max), value).expect("min and max are set"),
            );
        }
    }

    fn emit_haptic_feedback(app: &App, is_fast_drag: bool) {
        match app.platform().target_platform() {
            TargetPlatform::IOS => {
                // The values are estimated using a physical iPhone 15 Pro running iOS 18.
                if is_fast_drag {
                    HapticFeedback::medium_impact(app);
                } else {
                    HapticFeedback::selection_click(app);
                }
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }
}

impl TickerProviderStateMixin for CupertinoSliderState {
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
impl TickerProviderObject for CupertinoSliderState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoSliderState {
    type Widget = CupertinoSlider;
    waft_widgets::state_accessors!();

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let active_color = CupertinoDynamicColor::resolve(
            &widget
                .active_color
                .clone()
                .unwrap_or_else(|| CupertinoTheme::of(app, context).primary_color()),
            app,
            context,
        )
        .color();
        let on_changed: Option<SliderValueChanged> = widget.on_changed.is_some().then(|| {
            Rc::new(move |app: &mut App, value: f64, is_fast_drag: bool| {
                self.handle_changed(app, value, is_fast_drag)
            }) as SliderValueChanged
        });
        let on_change_start: Option<ValueChanged<f64>> =
            widget.on_change_start.is_some().then(|| {
                Rc::new(move |app: &mut App, value: f64| self.handle_drag_start(app, value))
                    as ValueChanged<f64>
            });
        let on_change_end: Option<ValueChanged<f64>> = widget.on_change_end.is_some().then(|| {
            Rc::new(move |app: &mut App, value: f64| self.handle_drag_end(app, value))
                as ValueChanged<f64>
        });
        Opacity::new(if widget.on_changed.is_none() {
            K_DISABLED_OPACITY
        } else {
            1.0
        })
        .child(CupertinoSliderRenderObjectWidget {
            value: (widget.value - widget.min) / (widget.max - widget.min),
            divisions: widget.divisions,
            active_color,
            thumb_color: widget.thumb_color.clone(),
            on_changed,
            on_change_start,
            on_change_end,
            vsync: TickerProviderRef::new(self),
        })
        .into_widget()
    }
}

/// Dart's `_CupertinoSliderRenderObjectWidget`.
struct CupertinoSliderRenderObjectWidget {
    value: f64,
    divisions: Option<i32>,
    active_color: Color,
    thumb_color: AnyColor,
    on_changed: Option<SliderValueChanged>,
    on_change_start: Option<ValueChanged<f64>>,
    on_change_end: Option<ValueChanged<f64>>,
    vsync: TickerProviderRef,
}

impl Debug for CupertinoSliderRenderObjectWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CupertinoSliderRenderObjectWidget")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for CupertinoSliderRenderObjectWidget {
    type RenderObject = RenderCupertinoSlider;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let thumb_color = CupertinoDynamicColor::resolve(&self.thumb_color, app, context).color();
        let track_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_FILL, app, context).color();
        let text_direction = Directionality::of(app, context);
        RenderCupertinoSlider::new(
            app,
            self.value,
            self.divisions,
            self.active_color,
            thumb_color,
            track_color,
            self.on_changed.clone(),
            self.on_change_start.clone(),
            self.on_change_end.clone(),
            self.vsync.clone(),
            text_direction,
            if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                <dyn MouseCursor>::defer()
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderCupertinoSlider>,
    ) {
        let thumb_color = CupertinoDynamicColor::resolve(&self.thumb_color, app, context).color();
        let track_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_FILL, app, context).color();
        let text_direction = Directionality::of(app, context);
        render_object.set_value(app, self.value);
        render_object.set_divisions(app, self.divisions);
        render_object.set_active_color(app, self.active_color);
        render_object.set_thumb_color(app, thumb_color);
        render_object.set_track_color(app, track_color);
        render_object.set_on_changed(app, self.on_changed.clone());
        render_object.get_mut(app).on_change_start = self.on_change_start.clone();
        render_object.get_mut(app).on_change_end = self.on_change_end.clone();
        render_object.set_text_direction(app, text_direction);
        // Ticker provider cannot change since there's a 1:1 relationship between
        // the _SliderRenderObjectWidget object and the _SliderState object.
    }
}

impl LeafRenderObjectWidget for CupertinoSliderRenderObjectWidget {}

const K_PADDING: f64 = 8.0;
const K_SLIDER_HEIGHT: f64 = 2.0 * (CupertinoThumbPainter::RADIUS + K_PADDING);
const K_SLIDER_WIDTH: f64 = 176.0; // Matches Material Design slider.
const K_DISCRETE_TRANSITION_DURATION: Duration = Duration::from_millis(500);

/// Dart's `_RenderCupertinoSlider extends RenderConstrainedBox implements
/// MouseTrackerAnnotation`.
///
/// The constrained box's fields are this leaf's own (`additional_constraints`, and the child
/// slot a leaf widget never fills), and its layout and intrinsics bodies are repeated here:
/// `RenderConstrainedBox` is a concrete struct with no bag or trait to inherit them from.
struct RenderCupertinoSlider {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    additional_constraints: BoxConstraints,
    value: f64,
    divisions: Option<i32>,
    active_color: Color,
    thumb_color: Color,
    track_color: Color,
    on_changed: Option<SliderValueChanged>,
    on_change_start: Option<ValueChanged<f64>>,
    on_change_end: Option<ValueChanged<f64>>,
    text_direction: TextDirection,
    position: Handle<AnimationController>,
    drag: Handle<HorizontalDragGestureRecognizer>,
    current_drag_value: f64,
    last_update_timestamp: Option<Duration>,
    cursor: MouseCursorRef,
    on_enter: Option<PointerEnterEventListener>,
    #[expect(
        dead_code,
        reason = "Dart's public field, which nothing reads or writes"
    )]
    on_hover: Option<PointerHoverEventListener>,
    on_exit: Option<PointerExitEventListener>,
}

impl RenderCupertinoSlider {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor parameters, in its order"
    )]
    fn new(
        app: &mut App,
        value: f64,
        divisions: Option<i32>,
        active_color: Color,
        thumb_color: Color,
        track_color: Color,
        on_changed: Option<SliderValueChanged>,
        on_change_start: Option<ValueChanged<f64>>,
        on_change_end: Option<ValueChanged<f64>>,
        vsync: TickerProviderRef,
        text_direction: TextDirection,
        cursor: MouseCursorRef,
    ) -> RenderHandle<RenderCupertinoSlider> {
        debug_assert!((0.0..=1.0).contains(&value));
        let additional_constraints =
            BoxConstraints::tight_for(Some(K_SLIDER_WIDTH), Some(K_SLIDER_HEIGHT));
        debug_assert!(additional_constraints.debug_assert_is_valid(false));
        let drag = HorizontalDragGestureRecognizer::new(app);
        let position = AnimationController::create(
            app,
            Some(value),
            Some(K_DISCRETE_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        );
        let this = RenderHandle::new_box(
            app,
            RenderCupertinoSlider {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                additional_constraints,
                value,
                divisions,
                active_color,
                thumb_color,
                track_color,
                on_changed,
                on_change_start,
                on_change_end,
                text_direction,
                position,
                drag,
                current_drag_value: 0.0,
                last_update_timestamp: None,
                cursor,
                on_enter: None,
                on_hover: None,
                on_exit: None,
            },
        );
        drag.set_on_start(
            app,
            Some(Rc::new(move |app: &mut App, details: DragStartDetails| {
                this.handle_drag_start(app, details)
            })),
        );
        drag.set_on_update(
            app,
            Some(Rc::new(move |app: &mut App, details: DragUpdateDetails| {
                this.handle_drag_update(app, details)
            })),
        );
        drag.set_on_end(
            app,
            Some(Rc::new(move |app: &mut App, details: DragEndDetails| {
                this.handle_drag_end(app, details)
            })),
        );
        position.add_listener(
            app,
            Listener::handle_method(this.handle(), mark_needs_paint),
        );
        this
    }

    #[expect(
        dead_code,
        reason = "Dart's getter; its readers are the semantics members, which wait"
    )]
    fn value(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).value
    }

    fn set_value(self: RenderHandle<Self>, app: &mut App, new_value: f64) {
        debug_assert!((0.0..=1.0).contains(&new_value));
        if new_value == self.get(app).value {
            return;
        }
        self.get_mut(app).value = new_value;
        let position = self.get(app).position;
        if self.divisions(app).is_some() {
            position.animate_to(app, new_value, None, Curves::fast_out_slow_in());
        } else {
            position.set_value(app, new_value);
        }
        // `markNeedsSemanticsUpdate()` waits with accessibility.
    }

    fn divisions(self: RenderHandle<Self>, app: &App) -> Option<i32> {
        self.get(app).divisions
    }

    fn set_divisions(self: RenderHandle<Self>, app: &mut App, value: Option<i32>) {
        if value == self.get(app).divisions {
            return;
        }
        self.get_mut(app).divisions = value;
        self.mark_needs_paint(app);
    }

    fn set_active_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if value == self.get(app).active_color {
            return;
        }
        self.get_mut(app).active_color = value;
        self.mark_needs_paint(app);
    }

    fn set_thumb_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if value == self.get(app).thumb_color {
            return;
        }
        self.get_mut(app).thumb_color = value;
        self.mark_needs_paint(app);
    }

    fn set_track_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if value == self.get(app).track_color {
            return;
        }
        self.get_mut(app).track_color = value;
        self.mark_needs_paint(app);
    }

    fn on_changed(self: RenderHandle<Self>, app: &App) -> Option<SliderValueChanged> {
        self.get(app).on_changed.clone()
    }

    /// Dart's setter marks the semantics dirty when interactivity flips; that waits with
    /// accessibility, so it only stores the callback.
    fn set_on_changed(self: RenderHandle<Self>, app: &mut App, value: Option<SliderValueChanged>) {
        self.get_mut(app).on_changed = value;
    }

    fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_paint(app);
    }

    fn discretized_current_drag_value(self: RenderHandle<Self>, app: &App) -> f64 {
        let mut drag_value = clamp_double(self.get(app).current_drag_value, 0.0, 1.0);
        if let Some(divisions) = self.divisions(app) {
            drag_value = (drag_value * f64::from(divisions)).round() / f64::from(divisions);
        }
        drag_value
    }

    fn track_left() -> f64 {
        K_PADDING
    }

    fn track_right(self: RenderHandle<Self>, app: &App) -> f64 {
        self.size(app).width() - K_PADDING
    }

    fn thumb_center(self: RenderHandle<Self>, app: &App) -> f64 {
        let value = self.get(app).value;
        let visual_position = match self.text_direction(app) {
            TextDirection::Rtl => 1.0 - value,
            TextDirection::Ltr => value,
        };
        lerp_double(
            Some(Self::track_left() + CupertinoThumbPainter::RADIUS),
            Some(self.track_right(app) - CupertinoThumbPainter::RADIUS),
            visual_position,
        )
        .expect("both ends are set")
    }

    fn is_interactive(self: RenderHandle<Self>, app: &App) -> bool {
        self.on_changed(app).is_some()
    }

    fn handle_drag_start(self: RenderHandle<Self>, app: &mut App, details: DragStartDetails) {
        self.start_interaction(app, details);
    }

    fn handle_drag_update(self: RenderHandle<Self>, app: &mut App, details: DragUpdateDetails) {
        if !self.is_interactive(app) {
            return;
        }
        let extent = K_PADDING
            .max(self.size(app).width() - 2.0 * (K_PADDING + CupertinoThumbPainter::RADIUS));
        let value_delta = details
            .primary_delta
            .expect("a horizontal drag has a primary delta")
            / extent;
        self.get_mut(app).current_drag_value += match self.text_direction(app) {
            TextDirection::Rtl => -value_delta,
            TextDirection::Ltr => value_delta,
        };

        // Default to false if no source timestamp is available.
        let mut is_fast = false;
        let current_timestamp = details.source_time_stamp;
        if let (Some(current_timestamp), Some(last_update_timestamp)) =
            (current_timestamp, self.get(app).last_update_timestamp)
        {
            let time_delta = duration_in_milliseconds(current_timestamp, last_update_timestamp);
            let velocity = value_delta.abs() * 1000.0 / time_delta as f64;
            // Velocity is in units of slider extent per second.
            // Value of 0.5 means the user is dragging at 50% of the slider extent per second.
            is_fast = velocity > K_VELOCITY_THRESHOLD;
        }
        self.get_mut(app).last_update_timestamp = current_timestamp;
        let on_changed = self.on_changed(app).expect("an interactive slider");
        on_changed(app, self.discretized_current_drag_value(app), is_fast);
    }

    fn handle_drag_end(self: RenderHandle<Self>, app: &mut App, _details: DragEndDetails) {
        self.end_interaction(app);
    }

    fn start_interaction(self: RenderHandle<Self>, app: &mut App, details: DragStartDetails) {
        if self.is_interactive(app) {
            if let Some(on_change_start) = self.get(app).on_change_start.clone() {
                on_change_start(app, self.discretized_current_drag_value(app));
            }
            let value = self.get(app).value;
            let this = self.get_mut(app);
            this.current_drag_value = value;
            this.last_update_timestamp = details.source_time_stamp;
            let on_changed = self.on_changed(app).expect("an interactive slider");
            on_changed(app, self.discretized_current_drag_value(app), false);
        }
    }

    fn end_interaction(self: RenderHandle<Self>, app: &mut App) {
        if let Some(on_change_end) = self.get(app).on_change_end.clone() {
            on_change_end(app, self.discretized_current_drag_value(app));
        }
        let this = self.get_mut(app);
        this.current_drag_value = 0.0;
        this.last_update_timestamp = None;
    }

    #[expect(
        dead_code,
        reason = "Dart's setter; the widget sets the cursor only at creation"
    )]
    fn set_cursor(self: RenderHandle<Self>, app: &mut App, value: MouseCursorRef) {
        if self.get(app).cursor.ne(&value) {
            self.get_mut(app).cursor = value;
            // A repaint is needed in order to trigger a device update of
            // [MouseTracker] so that this new value can be found.
            self.mark_needs_paint(app);
        }
    }
}

/// Dart's `Duration.inMilliseconds` of `current - last`, which truncates toward zero.
fn duration_in_milliseconds(current: Duration, last: Duration) -> i64 {
    let micros = current.as_micros() as i64 - last.as_micros() as i64;
    micros / 1000
}

/// Dart's `_position..addListener(markNeedsPaint)`.
fn mark_needs_paint(this: Handle<RenderCupertinoSlider>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

impl RenderObjectWithChildMixin for RenderCupertinoSlider {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderProxyBoxMixin for RenderCupertinoSlider {}

impl RenderObject for RenderCupertinoSlider {
    waft_rendering::render_object_accessors!();

    /// `RenderConstrainedBox.performLayout`.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let additional = self.get(app).additional_constraints;
        if let Some(child) = self.child(app) {
            child.layout(app, additional.enforce(constraints), true);
            self.set_size(app, child.size(app));
        } else {
            self.set_size(app, additional.enforce(constraints).constrain(Size::ZERO));
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let position = self.get(app).position.value(app);
        let (active_color, track_color) = (self.get(app).active_color, self.get(app).track_color);
        let (visual_position, left_color, right_color) = match self.text_direction(app) {
            TextDirection::Rtl => (1.0 - position, active_color, track_color),
            TextDirection::Ltr => (position, track_color, active_color),
        };

        let size = self.size(app);
        let track_center = offset.dy() + size.height() / 2.0;
        let track_left = offset.dx() + Self::track_left();
        let track_top = track_center - 1.0;
        let track_bottom = track_center + 1.0;
        let track_right = offset.dx() + self.track_right(app);
        let track_active = offset.dx() + self.thumb_center(app);
        let thumb_color = self.get(app).thumb_color;

        let canvas = context.canvas();
        if visual_position > 0.0 {
            let paint = Paint::from_color(right_color.into());
            // Use RRect instead of RSuperellipse here since the radius is too
            // small to make enough visual difference.
            draw_rrect(
                canvas,
                RRect::from_ltrbxy(track_left, track_top, track_active, track_bottom, 1.0, 1.0),
                &paint,
            );
        }

        if visual_position < 1.0 {
            let paint = Paint::from_color(left_color.into());
            // Use RRect instead of RSuperellipse here since the radius is too
            // small to make enough visual difference.
            draw_rrect(
                canvas,
                RRect::from_ltrbxy(track_active, track_top, track_right, track_bottom, 1.0, 1.0),
                &paint,
            );
        }

        let thumb_center = Offset::new(track_active, track_center);
        CupertinoThumbPainter::new().color(thumb_color).paint(
            canvas,
            Rect::from_circle(thumb_center, CupertinoThumbPainter::RADIUS),
        );
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }

    /// Dart's `implements MouseTrackerAnnotation`, never valid for the tracker.
    fn mouse_tracker_annotation(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<MouseTrackerAnnotation> {
        let this = self.get(app);
        Some(MouseTrackerAnnotation {
            on_enter: this.on_enter.clone(),
            on_exit: this.on_exit.clone(),
            cursor: Rc::clone(&this.cursor),
            valid_for_mouse_tracker: false,
        })
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        let (drag, position) = (self.get(app).drag, self.get(app).position);
        RecognizerLeaf::dispose(drag, app);
        position.dispose(app);
        RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderCupertinoSlider {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        (position.dx() - self.thumb_center(app)).abs() < CupertinoThumbPainter::RADIUS + K_PADDING
    }

    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        _entry: &BoxHitTestEntry,
    ) {
        if let PointerEvent::Down(event) = event
            && self.is_interactive(app)
        {
            self.get(app).drag.add_pointer(app, event.clone());
        }
    }

    /// `RenderConstrainedBox.computeMinIntrinsicWidth`.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_width() && additional.has_tight_width() {
            return additional.min_width;
        }
        let width = RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height);
        debug_assert!(width.is_finite());
        if additional.has_infinite_width() {
            width
        } else {
            additional.constrain_width(width)
        }
    }

    /// `RenderConstrainedBox.computeMaxIntrinsicWidth`.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_width() && additional.has_tight_width() {
            return additional.min_width;
        }
        let width = RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height);
        debug_assert!(width.is_finite());
        if additional.has_infinite_width() {
            width
        } else {
            additional.constrain_width(width)
        }
    }

    /// `RenderConstrainedBox.computeMinIntrinsicHeight`.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_height() && additional.has_tight_height() {
            return additional.min_height;
        }
        let height = RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width);
        debug_assert!(height.is_finite());
        if additional.has_infinite_height() {
            height
        } else {
            additional.constrain_height(height)
        }
    }

    /// `RenderConstrainedBox.computeMaxIntrinsicHeight`.
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let additional = self.get(app).additional_constraints;
        if additional.has_bounded_height() && additional.has_tight_height() {
            return additional.min_height;
        }
        let height = RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width);
        debug_assert!(height.is_finite());
        if additional.has_infinite_height() {
            height
        } else {
            additional.constrain_height(height)
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    /// `RenderConstrainedBox.computeDryBaseline`.
    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let additional = self.get(app).additional_constraints;
        self.child(app).and_then(|child| {
            child.get_dry_baseline(app, additional.enforce(constraints), baseline)
        })
    }

    /// `RenderConstrainedBox.computeDryLayout`.
    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let additional = self.get(app).additional_constraints;
        match self.child(app) {
            Some(child) => child.get_dry_layout(app, additional.enforce(constraints)),
            None => additional.enforce(constraints).constrain(Size::ZERO),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::valo::Op;
    use waft_embedder::{
        Brightness, HapticFeedbackType, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind,
    };
    use waft_foundation::AppCell;
    use waft_gestures::GestureBinding;
    use waft_rendering::{PictureLayer, RendererBinding};
    use waft_scheduler::SchedulerBinding;
    use waft_test::TestPlatform;
    use waft_widgets::{Center, GlobalKey, MediaQuery, MediaQueryData, SizedBox};

    use super::*;
    use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
    use crate::test_support::{build, pump, test_view};
    use crate::theme::CupertinoThemeData;

    /// The haptics a test platform played, in order.
    #[derive(Default)]
    struct RecordedHaptics(RefCell<Vec<HapticFeedbackType>>);

    impl waft_embedder::Haptics for RecordedHaptics {
        fn feedback(&self, kind: HapticFeedbackType) {
            self.0.borrow_mut().push(kind);
        }
    }

    /// What the host's callbacks heard.
    #[derive(Default)]
    struct Heard {
        starts: Vec<f64>,
        ends: Vec<f64>,
    }

    /// The `StatefulBuilder` every Dart test wraps its slider in: it owns the value.
    struct Host {
        key: Option<KeyRef>,
        initial: f64,
        text_direction: TextDirection,
        wrap: Rc<dyn Fn(WidgetRef) -> WidgetRef>,
        configure: Rc<dyn Fn(CupertinoSlider) -> CupertinoSlider>,
        heard: Rc<RefCell<Heard>>,
    }

    impl Debug for Host {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Host")
        }
    }

    impl StatefulWidget for Host {
        type State = HostState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> HostState {
            HostState {
                state: StateData::new(),
                value: self.initial,
                enabled: true,
                slider_key: GlobalKey::new(),
            }
        }
    }

    struct HostState {
        state: StateData<Host>,
        value: f64,
        enabled: bool,
        slider_key: GlobalKey,
    }

    impl State for HostState {
        type Widget = Host;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let on_changed: Option<ValueChanged<f64>> = app.get(self).enabled.then(|| {
                Rc::new(move |app: &mut App, new_value: f64| {
                    self.set_state(app, |state| state.value = new_value);
                }) as ValueChanged<f64>
            });
            let heard = Rc::clone(&self.widget(app).heard);
            let (starts, ends) = (Rc::clone(&heard), heard);
            let slider = CupertinoSlider::new(app.get(self).value, on_changed)
                .key(app.get(self).slider_key.clone().into())
                .on_change_start(Rc::new(move |_app: &mut App, value| {
                    starts.borrow_mut().starts.push(value)
                }))
                .on_change_end(Rc::new(move |_app: &mut App, value| {
                    ends.borrow_mut().ends.push(value)
                }));
            let slider = (self.widget(app).configure)(slider);
            let centered = (self.widget(app).wrap)(Center::new().child(slider).into_widget());
            Directionality::new(self.widget(app).text_direction, centered).into_widget()
        }
    }

    struct Setup {
        platform: TargetPlatform,
        initial: f64,
        text_direction: TextDirection,
        wrap: Rc<dyn Fn(WidgetRef) -> WidgetRef>,
        configure: Rc<dyn Fn(CupertinoSlider) -> CupertinoSlider>,
    }

    impl Setup {
        fn new() -> Setup {
            Setup {
                platform: TargetPlatform::IOS,
                initial: 0.0,
                text_direction: TextDirection::Ltr,
                wrap: Rc::new(|child| child),
                configure: Rc::new(|slider| slider),
            }
        }

        fn on(mut self, platform: TargetPlatform) -> Setup {
            self.platform = platform;
            self
        }

        fn initial(mut self, initial: f64) -> Setup {
            self.initial = initial;
            self
        }

        fn rtl(mut self) -> Setup {
            self.text_direction = TextDirection::Rtl;
            self
        }

        /// Wraps the centred slider, as a Dart test's `CupertinoApp(theme:)` or trait widgets.
        fn wrap(mut self, wrap: impl Fn(WidgetRef) -> WidgetRef + 'static) -> Setup {
            self.wrap = Rc::new(wrap);
            self
        }

        fn theme(self, theme: CupertinoThemeData) -> Setup {
            self.wrap(move |child| CupertinoTheme::new(theme.clone(), child).into_widget())
        }

        fn configure(
            mut self,
            configure: impl Fn(CupertinoSlider) -> CupertinoSlider + 'static,
        ) -> Setup {
            self.configure = Rc::new(configure);
            self
        }

        fn mount(self) -> Fixture {
            let haptics = Rc::new(RecordedHaptics::default());
            let platform = TestPlatform::new()
                .on(self.platform)
                .with_view(test_view())
                .with_haptics(Rc::clone(&haptics) as Rc<dyn waft_embedder::Haptics>);
            let cell = AppCell::with_platform(Rc::new(platform));
            let heard: Rc<RefCell<Heard>> = Rc::default();
            let host_key = GlobalKey::new();
            build(
                &cell,
                Host {
                    key: Some(host_key.clone().into()),
                    initial: self.initial,
                    text_direction: self.text_direction,
                    wrap: self.wrap,
                    configure: self.configure,
                    heard: Rc::clone(&heard),
                }
                .into_widget(),
            );
            let host = host_key
                .current_state::<HostState>(&mut cell.borrow_mut())
                .expect("the host mounted");
            Fixture {
                cell,
                host,
                haptics,
                heard,
                clock: Cell::new(Duration::ZERO),
                last_position: Offset::ZERO,
            }
        }
    }

    struct Fixture {
        cell: Rc<AppCell>,
        host: Handle<HostState>,
        haptics: Rc<RecordedHaptics>,
        heard: Rc<RefCell<Heard>>,
        clock: Cell<Duration>,
        /// Where the last pointer event was, for the next one's delta.
        last_position: Offset,
    }

    const UNIT: f64 = CupertinoThumbPainter::RADIUS;

    impl Fixture {
        fn value(&self) -> f64 {
            self.cell.borrow().get(self.host).value
        }

        fn haptics(&self) -> Vec<HapticFeedbackType> {
            self.haptics.0.borrow().clone()
        }

        /// Rebuilds the host with `change` applied, as Dart's `updateState` and `pump`.
        fn set(&mut self, change: impl FnOnce(&mut HostState)) {
            self.host.set_state(&mut self.cell.borrow_mut(), change);
            self.pump();
        }

        fn pump(&mut self) {
            let at = self.clock.get() + Duration::from_millis(16);
            self.clock.set(at);
            pump(&mut self.cell.borrow_mut(), at);
        }

        fn settle(&mut self) {
            for _ in 0..60 {
                self.pump();
            }
        }

        /// The slider's top-left corner in the view: the 400x300 logical view centres the
        /// 176x44 slider.
        fn top_left() -> Offset {
            Offset::new((400.0 - 176.0) / 2.0, (300.0 - 44.0) / 2.0)
        }

        fn center() -> Offset {
            Self::top_left() + Offset::new(176.0 / 2.0, 44.0 / 2.0)
        }

        fn bottom_right() -> Offset {
            Self::top_left() + Offset::new(176.0, 44.0)
        }

        /// The test view is 2x: logical coordinates double into the packet.
        fn send_at(
            &mut self,
            change: PointerChange,
            kind: PointerDeviceKind,
            at: Offset,
            time_stamp: Duration,
        ) {
            let delta = match change {
                PointerChange::Add | PointerChange::Down => Offset::ZERO,
                _ => at - self.last_position,
            };
            self.last_position = at;
            let mut app = self.cell.borrow_mut();
            GestureBinding::instance(&mut app).handle_pointer_data_packet(
                &mut app,
                PointerDataPacket::new(vec![PointerData {
                    change,
                    kind,
                    time_stamp,
                    device: 1,
                    pointer_identifier: 1,
                    buttons: if matches!(change, PointerChange::Down | PointerChange::Move) {
                        1
                    } else {
                        0
                    },
                    physical_x: at.dx() * 2.0,
                    physical_y: at.dy() * 2.0,
                    physical_delta_x: delta.dx() * 2.0,
                    physical_delta_y: delta.dy() * 2.0,
                    ..PointerData::default()
                }]),
            );
            app.drain_microtasks();
        }

        fn touch(&mut self, change: PointerChange, at: Offset) {
            let at_time = self.clock.get() + Duration::from_millis(10);
            self.clock.set(at_time);
            self.send_at(change, PointerDeviceKind::Touch, at, at_time);
        }

        /// Dart's `tester.dragFrom`: down, a move of the default drag slop, the rest, up.
        fn drag_from(&mut self, start: Offset, dx: f64) {
            const DRAG_SLOP: f64 = 20.0;
            self.touch(PointerChange::Down, start);
            if dx.abs() > DRAG_SLOP {
                self.touch(
                    PointerChange::Move,
                    start + Offset::new(DRAG_SLOP.copysign(dx), 0.0),
                );
            }
            self.touch(PointerChange::Move, start + Offset::new(dx, 0.0));
            self.touch(PointerChange::Up, start + Offset::new(dx, 0.0));
            self.pump();
        }

        /// Dart's `tester.timedDragFrom` at its default 60 Hz: a down at zero, then a move at
        /// every interval, the first in place, each record after a frame, as Dart's
        /// `handlePointerEventRecord` pumps before it.
        fn timed_drag_from(&mut self, start: Offset, dx: f64, duration: Duration) {
            let intervals = (duration.as_micros() as f64 * 60.0 / 1e6) as u32;
            let base = self.clock.get() + Duration::from_secs(1);
            self.record_at(base, PointerChange::Down, start);
            for t in 0..=intervals {
                let time_stamp = base + duration * t / intervals;
                let at = start + Offset::new(dx * f64::from(t) / f64::from(intervals), 0.0);
                self.record_at(time_stamp, PointerChange::Move, at);
            }
            self.record_at(
                base + duration,
                PointerChange::Up,
                start + Offset::new(dx, 0.0),
            );
            self.pump();
        }

        /// One record of Dart's `handlePointerEventRecord`: a frame, then the event.
        fn record_at(&mut self, time_stamp: Duration, change: PointerChange, at: Offset) {
            self.clock.set(time_stamp);
            pump(&mut self.cell.borrow_mut(), time_stamp);
            self.send_at(change, PointerDeviceKind::Touch, at, time_stamp);
        }

        fn tap(&mut self, at: Offset) {
            self.touch(PointerChange::Down, at);
            self.touch(PointerChange::Up, at);
        }

        fn render_object(&self) -> AnyRenderObject {
            let key = self.cell.borrow().get(self.host).slider_key.clone();
            let mut app = self.cell.borrow_mut();
            let root = key
                .current_context(&mut app)
                .expect("the slider mounted")
                .find_render_object(&app)
                .expect("the slider has been laid out");
            find(&app, root, &|app, node| {
                node.downcast::<RenderCupertinoSlider>(app).is_some()
            })
            .expect("the slider's render object")
        }

        fn opacity(&self) -> f64 {
            let key = self.cell.borrow().get(self.host).slider_key.clone();
            let mut app = self.cell.borrow_mut();
            let root = key
                .current_context(&mut app)
                .expect("the slider mounted")
                .find_render_object(&app)
                .expect("the slider has been laid out");
            let opacity = find(&app, root, &|app, node| {
                node.downcast::<waft_rendering::RenderOpacity>(app)
                    .is_some()
            })
            .and_then(|node| node.downcast::<waft_rendering::RenderOpacity>(&app))
            .expect("the slider's opacity");
            opacity.opacity(&app)
        }

        fn size(&self) -> Size {
            let object = self.render_object();
            let app = self.cell.borrow();
            object.as_box().expect("a box").size(&app)
        }

        /// The colour of every draw in the view's pictures, in paint order.
        fn drawn(&self) -> Vec<u32> {
            let mut root = self.render_object();
            let app = self.cell.borrow();
            while let Some(parent) = root.parent(&app) {
                root = parent;
            }
            let layer = root.debug_layer(&app).expect("painted");
            layer
                .depth_first_iterate_children(&app)
                .into_iter()
                .filter_map(|child| {
                    app.handle::<PictureLayer>(child.id())
                        .and_then(|picture| picture.picture(&app).map(|p| p.ops().to_vec()))
                })
                .flatten()
                .filter_map(|op| match op {
                    Op::DrawRect { paint, .. }
                    | Op::DrawPath { paint, .. }
                    | Op::RRectBlur { paint, .. } => Some(argb(paint.color)),
                    _ => None,
                })
                .collect()
        }

        fn transient_callback_count(&self) -> usize {
            SchedulerBinding::transient_callback_count(&mut self.cell.borrow_mut())
        }
    }

    fn find(
        app: &App,
        node: AnyRenderObject,
        matches: &dyn Fn(&App, AnyRenderObject) -> bool,
    ) -> Option<AnyRenderObject> {
        if matches(app, node) {
            return Some(node);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find(app, child, matches);
            }
        });
        found
    }

    fn argb(color: waft_embedder::valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    fn argb_of(color: &AnyColor) -> u32 {
        argb(color.color().into())
    }

    /// Dart's `dragSlider`: three thumb radii right from the thumb at the start.
    fn drag_slider(fixture: &mut Fixture) {
        fixture.drag_from(Fixture::top_left() + Offset::new(UNIT, UNIT), 3.0 * UNIT);
    }

    #[test]
    fn a_disabled_slider_dims_and_restores_in_both_brightnesses() {
        for brightness in [Brightness::Dark, Brightness::Light] {
            let mut fixture = Setup::new()
                .initial(0.5)
                .theme(CupertinoThemeData::new().with_brightness(brightness))
                .configure(|slider| {
                    slider
                        .active_color(CupertinoColors::SYSTEM_PURPLE)
                        .thumb_color(CupertinoColors::SYSTEM_ORANGE)
                })
                .mount();
            assert_eq!(fixture.opacity(), 1.0, "{brightness:?}");
            let enabled_size = fixture.size();
            let render_object = fixture.render_object();

            fixture.set(|host| host.enabled = false);
            assert_eq!(fixture.opacity(), 0.5, "{brightness:?}");
            assert_eq!(
                fixture.render_object(),
                render_object,
                "the same render object"
            );
            assert_eq!(fixture.size(), enabled_size);
            fixture.drag_from(Fixture::center(), 50.0);
            assert_eq!(fixture.value(), 0.5);

            fixture.set(|host| host.enabled = true);
            assert_eq!(fixture.opacity(), 1.0, "{brightness:?}");
            assert_eq!(
                fixture.render_object(),
                render_object,
                "the same render object"
            );
            fixture.drag_from(Fixture::center(), 50.0);
            assert!(fixture.value() > 0.5, "{brightness:?}: {}", fixture.value());
        }
    }

    #[test]
    fn a_tap_off_the_thumb_does_not_move_the_slider() {
        for setup in [Setup::new(), Setup::new().rtl()] {
            let mut fixture = setup.mount();
            assert_eq!(fixture.value(), 0.0);
            fixture.tap(Fixture::center());
            assert_eq!(fixture.value(), 0.0);
            fixture.pump();
            assert_eq!(
                fixture.transient_callback_count(),
                0,
                "no animation started"
            );
        }
    }

    #[test]
    fn a_drag_calls_on_change_start_and_on_change_end_once() {
        let mut fixture = Setup::new().mount();
        drag_slider(&mut fixture);
        assert_eq!(fixture.heard.borrow().starts.len(), 1);
        assert_eq!(fixture.heard.borrow().ends.len(), 1);
        fixture.pump();
        assert_eq!(
            fixture.transient_callback_count(),
            0,
            "no animation started"
        );
    }

    #[test]
    fn a_drag_moves_the_slider_left_to_right() {
        let mut fixture = Setup::new().mount();
        assert_eq!(fixture.value(), 0.0);
        let delta = 3.0 * UNIT;
        fixture.drag_from(Fixture::top_left() + Offset::new(UNIT, UNIT), delta);
        let final_value = delta / (fixture.size().width() - 2.0 * (8.0 + UNIT));
        assert_eq!(fixture.heard.borrow().starts, [0.0]);
        assert_eq!(fixture.value(), final_value);
        assert_eq!(fixture.heard.borrow().ends, [final_value]);
        fixture.pump();
        assert_eq!(
            fixture.transient_callback_count(),
            0,
            "no animation started"
        );
    }

    #[test]
    fn a_drag_moves_a_right_to_left_slider_leftward() {
        let mut fixture = Setup::new().rtl().mount();
        assert_eq!(fixture.value(), 0.0);
        let delta = 3.0 * UNIT;
        fixture.drag_from(Fixture::bottom_right() - Offset::new(UNIT, UNIT), -delta);
        let final_value = delta / (fixture.size().width() - 2.0 * (8.0 + UNIT));
        assert_eq!(fixture.heard.borrow().starts, [0.0]);
        assert_eq!(fixture.value(), final_value);
        assert_eq!(fixture.heard.borrow().ends, [final_value]);
        fixture.pump();
        assert_eq!(
            fixture.transient_callback_count(),
            0,
            "no animation started"
        );
    }

    #[test]
    fn hitting_an_edge_plays_haptics_by_drag_speed_on_ios() {
        let mut fixture = Setup::new().mount();
        assert!(
            fixture.haptics().is_empty(),
            "none when the slider is created"
        );

        let thumb_center = Fixture::top_left() + Offset::new(UNIT, UNIT);
        fixture.drag_from(thumb_center, 50.0);
        fixture.pump();
        let thumb_center = thumb_center + Offset::new(50.0, 0.0);
        assert!(
            fixture.haptics().is_empty(),
            "none when the slider is moved"
        );

        // Move the slider to the end quickly.
        fixture.timed_drag_from(thumb_center, 1000.0, Duration::from_millis(100));
        assert_eq!(fixture.haptics(), [HapticFeedbackType::MediumImpact]);

        // Move the slider to the start slowly.
        let top_right = Fixture::top_left() + Offset::new(176.0, 0.0);
        let thumb_center = top_right - Offset::new(UNIT, -UNIT);
        let dx = -(thumb_center.dx() - Fixture::top_left().dx() - UNIT * 2.0);
        fixture.timed_drag_from(thumb_center, dx, Duration::from_millis(1100));
        assert_eq!(fixture.value(), 0.0);
        assert_eq!(
            fixture.haptics(),
            [
                HapticFeedbackType::MediumImpact,
                HapticFeedbackType::SelectionClick
            ]
        );
    }

    #[test]
    fn hitting_an_edge_plays_no_haptics_off_ios() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            let mut fixture = Setup::new().on(platform).mount();
            fixture.drag_from(Fixture::top_left() + Offset::new(UNIT, UNIT), 1000.0);
            assert_eq!(fixture.value(), 1.0, "{platform:?}");
            assert!(fixture.haptics().is_empty(), "{platform:?}");
        }
    }

    #[test]
    fn the_active_track_follows_the_theme() {
        let fixture = Setup::new().initial(0.5).mount();
        assert_eq!(fixture.drawn()[0], 0xff007aff, "systemBlue");

        let fixture = Setup::new()
            .initial(0.5)
            .theme(CupertinoThemeData::new().with_brightness(Brightness::Dark))
            .mount();
        assert_eq!(fixture.drawn()[0], 0xff0a84ff, "systemBlue's dark colour");
    }

    #[test]
    fn the_active_colour_overrides_the_theme() {
        let fixture = Setup::new()
            .initial(0.5)
            .theme(CupertinoThemeData::new().with_brightness(Brightness::Dark))
            .configure(|slider| slider.active_color(CupertinoColors::ACTIVE_GREEN))
            .mount();
        assert_eq!(fixture.drawn()[0], 0xff30d158, "systemGreen's dark colour");
    }

    #[test]
    fn a_dynamic_active_colour_resolves_against_every_trait() {
        let active_color = CupertinoDynamicColor::new(
            Color::new(0x00000001),
            Color::new(0x00000002),
            Color::new(0x00000004),
            Color::new(0x00000006),
            Color::new(0x00000003),
            Color::new(0x00000005),
            Color::new(0x00000007),
            Color::new(0x00000008),
        );
        use CupertinoUserInterfaceLevelData::{Base, Elevated};
        for (brightness, level, high_contrast, expected) in [
            (Brightness::Light, Base, false, 0x00000001),
            (Brightness::Dark, Base, false, 0x00000002),
            (Brightness::Dark, Elevated, false, 0x00000005),
            (Brightness::Dark, Base, true, 0x00000006),
            (Brightness::Dark, Elevated, true, 0x00000008),
            (Brightness::Light, Base, true, 0x00000004),
            (Brightness::Light, Elevated, false, 0x00000003),
            (Brightness::Light, Elevated, true, 0x00000007),
        ] {
            let fixture = Setup::new()
                .initial(0.5)
                .wrap(move |child| {
                    CupertinoTheme::new(
                        CupertinoThemeData::new().with_brightness(brightness),
                        CupertinoUserInterfaceLevel::new(
                            level,
                            MediaQuery::new(
                                MediaQueryData::new().high_contrast(high_contrast),
                                child,
                            )
                            .into_widget(),
                        )
                        .into_widget(),
                    )
                    .into_widget()
                })
                .configure(move |slider| slider.active_color(active_color.into_any()))
                .mount();
            // Every variant is transparent, and a transparent draw is not recorded, so the
            // resolved colour is read where Dart's `paints..rrect(color:)` would find it drawn.
            let key = fixture.cell.borrow().get(fixture.host).slider_key.clone();
            let mut app = fixture.cell.borrow_mut();
            let render = key
                .current_context(&mut app)
                .and_then(|context| context.find_render_object(&app))
                .expect("laid out");
            let slider = find(&app, render, &|app, node| {
                node.downcast::<RenderCupertinoSlider>(app).is_some()
            })
            .and_then(|node| node.downcast::<RenderCupertinoSlider>(&app))
            .expect("the slider's render object");
            assert_eq!(
                slider.get(&app).active_color.to_argb32(),
                expected,
                "{brightness:?} {level:?} {high_contrast}"
            );
        }
    }

    #[test]
    fn the_track_colour_is_dynamic() {
        const SYSTEM_FILL_LIGHT: u32 = 0x33787880;
        const SYSTEM_FILL_DARK: u32 = 0x5b787880;
        for (brightness, expected, other) in [
            (Brightness::Light, SYSTEM_FILL_LIGHT, SYSTEM_FILL_DARK),
            (Brightness::Dark, SYSTEM_FILL_DARK, SYSTEM_FILL_LIGHT),
        ] {
            let fixture = Setup::new()
                .theme(CupertinoThemeData::new().with_brightness(brightness))
                .configure(|slider| slider.active_color(CupertinoColors::ACTIVE_GREEN))
                .mount();
            let drawn = fixture.drawn();
            assert!(drawn.contains(&expected), "{brightness:?}: {drawn:x?}");
            assert!(!drawn.contains(&other), "{brightness:?}: {drawn:x?}");
        }
    }

    #[test]
    fn the_thumb_colour_can_be_overridden() {
        for thumb_color in [
            CupertinoColors::SYSTEM_PURPLE,
            CupertinoColors::ACTIVE_ORANGE,
        ] {
            let expected = argb_of(&thumb_color);
            let fixture = Setup::new()
                .configure(move |slider| slider.thumb_color(thumb_color.clone()))
                .mount();
            // The track, three shadows and the border, then the thumb.
            assert_eq!(fixture.drawn()[5], expected);
        }
    }

    #[test]
    fn hovering_the_thumb_keeps_the_basic_cursor_off_the_web() {
        let mut fixture = Setup::new().mount();
        let cursor = |fixture: &Fixture| {
            let mut app = fixture.cell.borrow_mut();
            RendererBinding::instance(&mut app)
                .mouse_tracker(&app)
                .debug_device_active_cursor(&app, 1)
        };
        let at = fixture.clock.get();
        fixture.send_at(
            PointerChange::Add,
            PointerDeviceKind::Mouse,
            Offset::new(10.0, 10.0),
            at,
        );
        fixture.settle();
        assert_eq!(cursor(&fixture), Some(SystemMouseCursors::BASIC.into()));

        let at = fixture.clock.get();
        fixture.send_at(
            PointerChange::Hover,
            PointerDeviceKind::Mouse,
            Fixture::top_left() + Offset::new(15.0, 0.0),
            at,
        );
        fixture.settle();
        let expected: MouseCursorRef = if K_IS_WEB {
            SystemMouseCursors::CLICK.into()
        } else {
            SystemMouseCursors::BASIC.into()
        };
        assert_eq!(cursor(&fixture), Some(expected));
    }

    #[test]
    fn a_slider_squeezed_to_nothing_lays_out_at_zero() {
        let cell = crate::test_support::test_cell();
        let key = GlobalKey::new();
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    SizedBox::shrink().child(
                        CupertinoSlider::new(0.0, Some(Rc::new(|_app: &mut App, _| {})))
                            .key(key.clone().into()),
                    ),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let render = key
            .current_context(&mut app)
            .and_then(|context| context.find_render_object(&app))
            .and_then(|object| object.as_box())
            .expect("the slider laid out");
        assert_eq!(render.size(&app), Size::ZERO);
    }

    #[test]
    fn a_divided_slider_animates_to_its_new_value() {
        let mut fixture = Setup::new().configure(|slider| slider.divisions(4)).mount();
        fixture.set(|host| host.value = 0.5);
        assert!(
            fixture.transient_callback_count() > 0,
            "the position animates"
        );
        fixture.settle();
        assert_eq!(fixture.transient_callback_count(), 0);
        let object = fixture.render_object();
        let app = fixture.cell.borrow();
        let slider = object
            .downcast::<RenderCupertinoSlider>(&app)
            .expect("the slider");
        assert_eq!(slider.get(&app).position.value(&app), 0.5);
    }

    #[test]
    fn a_divided_drag_snaps_to_its_divisions() {
        let mut fixture = Setup::new().configure(|slider| slider.divisions(4)).mount();
        // 42 of the 132-pixel extent is 0.318, nearest the quarter.
        drag_slider(&mut fixture);
        assert_eq!(fixture.value(), 0.25);
        assert_eq!(fixture.heard.borrow().ends, [0.25]);
    }
}

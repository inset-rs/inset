//! Flutter counterpart: material_ui's `range_slider.dart`, with its part
//! `generated/range_slider_defaults_m3.g.dart` in `generated/range_slider_defaults_m3.rs`.
//!
//! [`RangeSlider`] builds the private leaf render object Dart calls `_RenderRangeSlider`, a
//! `RenderBox` with `RelayoutWhenSystemFontsChangeMixin`, and paints its value indicators
//! through an `OverlayPortal` whose child is `_RenderValueIndicator`. The shapes paint with
//! the label painters the render object owns: it lends a painter out of its arena slot for
//! each shape call. The semantics configuration, its increase and decrease actions, the
//! `markNeedsSemanticsUpdate` calls and the semantic formatter's use wait with accessibility.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnyAnimation, CurvedAnimation,
    Curves,
};
use waft_embedder::{
    Color, Offset, Rect, Size, TargetPlatform, TextDirection, clamp_double, lerp_double,
};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref, Timer, ValueChanged};
use waft_gestures::{
    DeviceGestureSettings, DragEndDetails, DragGestureRecognizer, DragStartDetails,
    DragUpdateDetails, GestureArenaTeam, HorizontalDragGestureRecognizer, PointerEnterEvent,
    PointerEvent, PointerExitEvent, RecognizerLeaf, TapDownDetails, TapGestureRecognizer,
    TapUpDetails,
};
use waft_painting::{
    EdgeInsetsGeometry, K_DEFAULT_FONT_SIZE, PaintingBinding, TextPainter, TextScaler, TextSpan,
    TextStyle,
};
use waft_rendering::{
    AnyRenderObject, BoxConstraints, BoxHitTestEntry, LayerLink, PaintingContext, PipelineOwner,
    RelayoutWhenSystemFontsChangeData, RelayoutWhenSystemFontsChangeMixin, RenderBox,
    RenderBoxData, RenderHandle, RenderObject, RenderObjectBase, RenderObjectData,
};
use waft_scheduler::{SchedulerBinding, Ticker, TickerCallback, TickerProviderObject};
use waft_services::{LogicalKeyboardKey, MouseCursorRef};
use waft_widgets::{
    ActionRef, AnyFocusNode, BuildContext, CallbackAction, CompositedTransformFollower,
    CompositedTransformTarget, Directionality, FocusNode, FocusNodeLeaf, FocusNodeRef,
    FocusableActionDetector, GlobalKey, Intent, IntentRef, IntoWidget, KeyRef,
    LeafRenderObjectWidget, MediaQuery, MouseRegion, NavigationMode, OverlayPortal,
    OverlayPortalController, Padding, RenderObjectWidget, Row, ShortcutActivatorRef, ShortcutMap,
    Shortcuts, SingleActivator, SizedBox, Stack, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef, WidgetState,
    WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
    WidgetStates, debug_check_has_media_query,
};

use crate::constants::K_RADIAL_REACTION_DURATION;
use crate::debug::debug_check_has_material;
use crate::generated::range_slider_defaults_m3::RangeSliderDefaultsM3;
use crate::range_slider_parts::{
    RangeLabels, RangeSliderThumbShapeRef, RangeSliderTickMarkShapeRef, RangeSliderTrackShapeRef,
    RangeSliderValueIndicatorShapeRef, RangeThumbSelector, RangeValues, RoundRangeSliderThumbShape,
    RoundRangeSliderTickMarkShape, RoundedRectRangeSliderTrackShape,
};
use crate::slider::always_stopped_at_one;
use crate::slider_theme::{
    SemanticFormatterCallback, ShowValueIndicator, SliderTheme, SliderThemeData,
    SliderThemeDataGetters, Thumb,
};
use crate::slider_value_indicator_shape::{
    RectangularRangeSliderValueIndicatorShape, RoundSliderOverlayShape, SliderComponentShapeRef,
};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;
use waft_painting::AnyColor;

/// [`RangeSlider`] uses this callback to paint the value indicator on the overlay.
/// Since the value indicator is painted on the Overlay; this method paints the
/// value indicator in a `RenderBox` that appears in the `Overlay`.
pub type PaintRangeValueIndicator = Rc<dyn Fn(&mut App, &mut PaintingContext, Offset)>;

/// A Material Design range slider.
///
/// Used to select a range from a range of values.
///
/// A range slider can be used to select from either a continuous or a discrete
/// set of values. The default is to use a continuous range of values from [`min`](Self::min)
/// to [`max`](Self::max). To use discrete values, use a non-null value for
/// [`divisions`](Self::divisions), which indicates the number of discrete intervals. For
/// example, if [`min`](Self::min) is 0.0 and [`max`](Self::max) is 50.0 and
/// [`divisions`](Self::divisions) is 5, then the slider can take on the discrete values 0.0,
/// 10.0, 20.0, 30.0, 40.0, and 50.0.
///
/// The terms for the parts of a slider are:
///
///  * The "thumbs", which are the shapes that slide horizontally when the user
///    drags them to change the selected range.
///  * The "track", which is the horizontal line that the thumbs can be dragged
///    along.
///  * The "tick marks", which mark the discrete values of a discrete slider.
///  * The "overlay", which is a highlight that's drawn over a thumb in response
///    to a user tap-down gesture.
///  * The "value indicators", which are the shapes that pop up when the user
///    is dragging a thumb to show the value being selected.
///  * The "active" segment of the slider is the segment between the two thumbs.
///  * The "inactive" slider segments are the two track intervals outside of the
///    slider's thumbs.
///
/// The range slider will be disabled if [`on_changed`](Self::on_changed) is null or if the
/// range given by [`min`](Self::min)..[`max`](Self::max) is empty (i.e. if
/// [`min`](Self::min) is equal to [`max`](Self::max)).
///
/// The range slider widget itself does not maintain any state. Instead, when
/// the state of the slider changes, the widget calls the [`on_changed`](Self::on_changed)
/// callback. Most widgets that use a range slider will listen for the
/// [`on_changed`](Self::on_changed) callback and rebuild the slider with new
/// [`values`](Self::values) to update the visual appearance of the slider. To know when the
/// value starts to change, or when it is done changing, set the optional callbacks
/// [`on_change_start`](Self::on_change_start) and/or [`on_change_end`](Self::on_change_end).
///
/// By default, a slider will be as wide as possible, centered vertically. When
/// given unbounded constraints, it will attempt to make the track 144 pixels
/// wide (including margins on each side) and will shrink-wrap vertically.
///
/// Requires one of its ancestors to be a `Material` widget. This is typically
/// provided by a `Scaffold` widget.
///
/// Requires one of its ancestors to be a `MediaQuery` widget. Typically, a
/// `MediaQuery` widget is introduced by the `MaterialApp` or `WidgetsApp`
/// widget at the top of your application widget tree.
///
/// To determine how it should be displayed (e.g. colors, thumb shape, etc.),
/// a slider uses the [`SliderThemeData`] available from either a [`SliderTheme`]
/// widget, or the `ThemeData::slider_theme` inside a `Theme` widget above it in
/// the widget tree. You can also override some of the colors with the
/// [`active_color`](Self::active_color) and [`inactive_color`](Self::inactive_color)
/// properties, although more fine-grained control of the colors, and other visual
/// properties is achieved using a [`SliderThemeData`].
///
/// See also:
///
///  * [`SliderTheme`] and [`SliderThemeData`] for information about controlling
///    the visual appearance of the slider.
///  * `Slider`, for a single-valued slider.
///  * `Radio`, for selecting among a set of explicit values.
///  * `Checkbox` and `Switch`, for toggling a particular value on or off.
///  * <https://material.io/design/components/sliders.html>
///  * `MediaQuery`, from which the text scale factor is obtained.
#[derive(Clone)]
pub struct RangeSlider {
    pub key: Option<KeyRef>,

    /// The currently selected values for this range slider.
    ///
    /// The slider's thumbs are drawn at horizontal positions that corresponds to
    /// these values.
    pub values: RangeValues,

    /// Called when the user is selecting a new value for the slider by dragging.
    ///
    /// The slider passes the new values to the callback but does not actually
    /// change state until the parent widget rebuilds the slider with the new
    /// values.
    ///
    /// If null, the slider will be displayed as disabled.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of the
    /// parent `StatefulWidget` using the `State::set_state` method, so that the parent gets
    /// rebuilt.
    ///
    /// See also:
    ///
    ///  * [`on_change_start`](Self::on_change_start), which is called when the user starts
    ///    changing the values.
    ///  * [`on_change_end`](Self::on_change_end), which is called when the user stops changing
    ///    the values.
    pub on_changed: Option<ValueChanged<RangeValues>>,

    /// Called when the user starts selecting new values for the slider.
    ///
    /// This callback shouldn't be used to update the slider [`values`](Self::values) (use
    /// [`on_changed`](Self::on_changed) for that). Rather, it should be used to be notified when
    /// the user has started selecting a new value by starting a drag or with a tap.
    ///
    /// The values passed will be the last [`values`](Self::values) that the slider had before
    /// the change began.
    ///
    /// See also:
    ///
    ///  * [`on_change_end`](Self::on_change_end) for a callback that is called when the value
    ///    change is complete.
    pub on_change_start: Option<ValueChanged<RangeValues>>,

    /// Called when the user is done selecting new values for the slider.
    ///
    /// This differs from [`on_changed`](Self::on_changed) because it is only called once at the
    /// end of the interaction, while [`on_changed`](Self::on_changed) is called as the value is
    /// getting updated within the interaction.
    ///
    /// This callback shouldn't be used to update the slider [`values`](Self::values) (use
    /// [`on_changed`](Self::on_changed) for that). Rather, it should be used to know when the
    /// user has completed selecting a new [`values`](Self::values) by ending a drag or a click.
    ///
    /// See also:
    ///
    ///  * [`on_change_start`](Self::on_change_start) for a callback that is called when a value
    ///    change begins.
    pub on_change_end: Option<ValueChanged<RangeValues>>,

    /// The minimum value the user can select.
    ///
    /// Defaults to 0.0. Must be less than or equal to [`max`](Self::max).
    ///
    /// If the [`max`](Self::max) is equal to the [`min`](Self::min), then the slider is
    /// disabled.
    pub min: f64,

    /// The maximum value the user can select.
    ///
    /// Defaults to 1.0. Must be greater than or equal to [`min`](Self::min).
    ///
    /// If the [`max`](Self::max) is equal to the [`min`](Self::min), then the slider is
    /// disabled.
    pub max: f64,

    /// The number of discrete divisions.
    ///
    /// Typically used with [`labels`](Self::labels) to show the current discrete values.
    ///
    /// If null, the slider is continuous.
    pub divisions: Option<i32>,

    /// Labels to show as text in the `SliderThemeData::range_value_indicator_shape`
    /// when the slider is active and `SliderThemeData::show_value_indicator`
    /// is satisfied.
    ///
    /// There are two labels: one for the start thumb and one for the end thumb.
    ///
    /// Each label is rendered using the active `ThemeData`'s `TextTheme::body_large` text
    /// style, with the theme data's `ColorScheme::on_primary` color. The label's text style
    /// can be overridden with `SliderThemeData::value_indicator_text_style`.
    ///
    /// If null, then the value indicator will not be displayed.
    ///
    /// See also:
    ///
    ///  * `RangeSliderValueIndicatorShape` for how to create a custom value indicator shape.
    pub labels: Option<RangeLabels>,

    /// The color of the track's active segment, i.e. the span of track between the thumbs.
    ///
    /// Defaults to `ColorScheme::primary` of the ambient `ThemeData`.
    ///
    /// Using a [`SliderTheme`] gives more fine-grained control over the appearance of various
    /// components of the slider.
    pub active_color: Option<Color>,

    /// The color of the track's inactive segments, i.e. the span of tracks between the min
    /// and the start thumb, and the end thumb and the max.
    ///
    /// Defaults to `ColorScheme::primary` with 24% opacity of the ambient `ThemeData`.
    ///
    /// Using a [`SliderTheme`] gives more fine-grained control over the appearance of various
    /// components of the slider.
    pub inactive_color: Option<Color>,

    /// The highlight color that's typically used to indicate that
    /// the range slider thumb is in [`WidgetState::Hovered`] or [`WidgetState::Dragged`]
    /// states.
    ///
    /// If this property is null, [`RangeSlider`] will use [`active_color`](Self::active_color)
    /// with an opacity of 0.12, `SliderThemeData::overlay_color`, or the default value of
    /// `ColorScheme::primary` with an opacity of 0.12.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If null, then the value of `SliderThemeData::mouse_cursor` is used. If that is also
    /// null, then `WidgetStateMouseCursor::clickable` is used.
    ///
    /// See also:
    ///
    ///  * [`WidgetStateMouseCursor`], which can be used to create a `MouseCursor`
    ///    that is also a `WidgetStateProperty<MouseCursor>`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// The callback used to create a semantic value from the slider's values.
    ///
    /// Defaults to formatting values as a percentage.
    ///
    /// Waits with accessibility: the callback is held and not called.
    pub semantic_formatter_callback: Option<SemanticFormatterCallback>,

    /// The padding between the slider and the edge of its bounds.
    ///
    /// If it is null, the value of `SliderThemeData::padding` is used. If that is also null,
    /// the default padding is the same as the thumb overlay, which is 24 on each side.
    pub padding: Option<EdgeInsetsGeometry>,

    /// When true, the [`RangeSlider`] will use the 2023 Material Design appearance.
    /// If not set, then the [`RangeSlider`] will behave as if this is set to true, or
    /// the value of `SliderThemeData::year2023`.
    ///
    /// If this is set to false, the [`RangeSlider`] will use the latest Material Design
    /// appearance, which was introduced in December 2023.
    ///
    /// Defaults to true.
    pub year2023: Option<bool>,
}

impl RangeSlider {
    /// Touch width for the tap boundary of the slider thumbs.
    const MIN_TOUCH_TARGET_WIDTH: f64 = crate::constants::K_MIN_INTERACTIVE_DIMENSION;

    /// Creates a Material Design range slider.
    ///
    /// The range slider widget itself does not maintain any state. Instead, when
    /// the state of the slider changes, the widget calls the [`on_changed`](Self::on_changed)
    /// callback. Most widgets that use a range slider will listen for the
    /// [`on_changed`](Self::on_changed) callback and rebuild the slider with new
    /// [`values`](Self::values) to update the visual appearance of the slider. To know when the
    /// value starts to change, or when it is done changing, set the optional callbacks
    /// [`on_change_start`](Self::on_change_start) and/or [`on_change_end`](Self::on_change_end).
    ///
    /// * [`values`](Self::values), which determines currently selected values for this range
    ///   slider.
    /// * [`on_changed`](Self::on_changed), which is called while the user is selecting a new
    ///   value for the range slider.
    /// * [`on_change_start`](Self::on_change_start), which is called when the user starts to
    ///   select a new value for the range slider.
    /// * [`on_change_end`](Self::on_change_end), which is called when the user is done selecting
    ///   a new value for the range slider.
    ///
    /// You can override some of the colors with the [`active_color`](Self::active_color) and
    /// [`inactive_color`](Self::inactive_color) properties, although more fine-grained control
    /// of the appearance is achieved using a [`SliderThemeData`].
    ///
    /// The [`min`](Self::min) must be less than or equal to the [`max`](Self::max).
    ///
    /// The [`RangeValues::start`] attribute of the [`values`](Self::values) parameter must be
    /// less than or equal to its [`RangeValues::end`] attribute. The [`RangeValues::start`]
    /// and [`RangeValues::end`] attributes of the [`values`](Self::values) parameter must be
    /// greater than or equal to the [`min`](Self::min) parameter and less than or equal to
    /// the [`max`](Self::max) parameter.
    ///
    /// The [`divisions`](Self::divisions) parameter must be null or greater than zero.
    pub fn new(values: RangeValues, on_changed: Option<ValueChanged<RangeValues>>) -> RangeSlider {
        RangeSlider {
            key: None,
            values,
            on_changed,
            on_change_start: None,
            on_change_end: None,
            min: 0.0,
            max: 1.0,
            divisions: None,
            labels: None,
            active_color: None,
            inactive_color: None,
            overlay_color: None,
            mouse_cursor: None,
            semantic_formatter_callback: None,
            padding: None,
            year2023: None,
        }
    }

    /// Dart `RangeSlider(key:)`.
    pub fn key(mut self, key: KeyRef) -> RangeSlider {
        self.key = Some(key);
        self
    }

    /// Dart `RangeSlider(onChangeStart:)`.
    pub fn on_change_start(
        mut self,
        on_change_start: impl Fn(&mut App, RangeValues) + 'static,
    ) -> RangeSlider {
        self.on_change_start = Some(Rc::new(on_change_start));
        self
    }

    /// Dart `RangeSlider(onChangeEnd:)`.
    pub fn on_change_end(
        mut self,
        on_change_end: impl Fn(&mut App, RangeValues) + 'static,
    ) -> RangeSlider {
        self.on_change_end = Some(Rc::new(on_change_end));
        self
    }

    /// Dart `RangeSlider(min:)`.
    pub fn min(mut self, min: f64) -> RangeSlider {
        self.min = min;
        self
    }

    /// Dart `RangeSlider(max:)`.
    pub fn max(mut self, max: f64) -> RangeSlider {
        self.max = max;
        self
    }

    /// Dart `RangeSlider(divisions:)`.
    pub fn divisions(mut self, divisions: i32) -> RangeSlider {
        self.divisions = Some(divisions);
        self
    }

    /// Dart `RangeSlider(labels:)`.
    pub fn labels(mut self, labels: RangeLabels) -> RangeSlider {
        self.labels = Some(labels);
        self
    }

    /// Dart `RangeSlider(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> RangeSlider {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `RangeSlider(inactiveColor:)`.
    pub fn inactive_color(mut self, inactive_color: Color) -> RangeSlider {
        self.inactive_color = Some(inactive_color);
        self
    }

    /// Dart `RangeSlider(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> RangeSlider {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `RangeSlider(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> RangeSlider {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `RangeSlider(semanticFormatterCallback:)`.
    pub fn semantic_formatter_callback(
        mut self,
        semantic_formatter_callback: SemanticFormatterCallback,
    ) -> RangeSlider {
        self.semantic_formatter_callback = Some(semantic_formatter_callback);
        self
    }

    /// Dart `RangeSlider(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> RangeSlider {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `RangeSlider(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 range slider appearance. \
                Defaults to true. In the future, this flag will default to false. Use \
                SliderThemeData to customize individual properties. This feature was \
                deprecated after v3.30.0-0.1.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> RangeSlider {
        self.year2023 = Some(year2023);
        self
    }
}

impl Debug for RangeSlider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("RangeSlider");
        properties
            .field("value_start", &self.values.start)
            .field("value_end", &self.values.end);
        if self.on_changed.is_none() {
            properties.field("on_changed", &"disabled");
        }
        properties
            .field("on_change_start", &self.on_change_start.is_some())
            .field("on_change_end", &self.on_change_end.is_some())
            .field("min", &self.min)
            .field("max", &self.max)
            .field("divisions", &self.divisions)
            .field(
                "label_start",
                &self.labels.as_ref().map(|labels| &labels.start),
            )
            .field("label_end", &self.labels.as_ref().map(|labels| &labels.end))
            .field("active_color", &self.active_color)
            .field("inactive_color", &self.inactive_color)
            .field(
                "semantic_formatter_callback",
                &self.semantic_formatter_callback.is_some(),
            )
            .finish()
    }
}

impl StatefulWidget for RangeSlider {
    type State = RangeSliderState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.min <= self.max);
        debug_assert!(self.values.start <= self.values.end);
        debug_assert!(self.values.start >= self.min && self.values.start <= self.max);
        debug_assert!(self.values.end >= self.min && self.values.end <= self.max);
        debug_assert!(self.divisions.is_none_or(|divisions| divisions > 0));
    }

    fn create_state(&self) -> RangeSliderState {
        RangeSliderState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            start_focus_node: None,
            end_focus_node: None,
            render_object_key: GlobalKey::new(),
            is_editing_in_directional_mode: false,
            navigation_mode: NavigationMode::Traditional,
            start_action_map: HashMap::new(),
            end_action_map: HashMap::new(),
            overlay_controller: None,
            value_indicator_controller: None,
            enable_controller: None,
            start_position_controller: None,
            end_position_controller: None,
            interaction_timer: None,
            paint_top_value_indicator: None,
            paint_bottom_value_indicator: None,
            dragging: false,
            show_start_focus_highlight: false,
            show_end_focus_highlight: false,
            hovering: false,
            show_hover_highlight: false,
            value_indicator_overlay_portal_controller: None,
            layer_link: None,
            default_range_thumb_selector: Rc::new(default_range_thumb_selector),
        }
    }
}

/// Dart's `_RangeSliderState`.
///
/// The fields Dart makes in its initializers (the focus nodes, the overlay portal controller,
/// which Dart shows there, and the layer link) are made in `init_state`, the first point with
/// an `App`.
pub struct RangeSliderState {
    state: StateData<RangeSlider>,
    ticker_provider: TickerProviderStateMixinData,
    start_focus_node: Option<Ref<FocusNode>>,
    end_focus_node: Option<Ref<FocusNode>>,
    render_object_key: GlobalKey,
    // Whether the slider is currently in the value adjustment mode while using
    // directional navigation (e.g. a TV remote or D-pad). When false, arrow keys
    // are left unhandled so they can move the focus between views; when true,
    // arrow keys adjust the slider value. Toggled by the enter key. This has no
    // effect when using traditional navigation.
    is_editing_in_directional_mode: bool,
    // The ambient navigation mode, cached so build() doesn't have to re-read the
    // MediaQuery (and re-pick a shortcut map) on every pass.
    navigation_mode: NavigationMode,
    // Action mapping for the start thumb.
    start_action_map: HashMap<TypeId, ActionRef>,
    // Action mapping for the end thumb.
    end_action_map: HashMap<TypeId, ActionRef>,
    // Animation controller that is run when the overlay (a.k.a radial reaction)
    // changes visibility in response to user interaction.
    overlay_controller: Option<Ref<AnimationController>>,
    // Animation controller that is run when the value indicators change visibility.
    value_indicator_controller: Option<Ref<AnimationController>>,
    // Animation controller that is run when enabling/disabling the slider.
    enable_controller: Option<Ref<AnimationController>>,
    // Animation controllers that are run when transitioning between one value
    // and the next on a discrete slider.
    start_position_controller: Option<Ref<AnimationController>>,
    end_position_controller: Option<Ref<AnimationController>>,
    interaction_timer: Option<Timer>,
    // Value Indicator paint Animation that appears on the Overlay.
    paint_top_value_indicator: Option<PaintRangeValueIndicator>,
    paint_bottom_value_indicator: Option<PaintRangeValueIndicator>,
    dragging: bool,
    show_start_focus_highlight: bool,
    show_end_focus_highlight: bool,
    hovering: bool,
    show_hover_highlight: bool,
    // Always keep the ValueIndicator visible on the Overlay; otherwise, it cannot be updated
    // during the build phase.
    value_indicator_overlay_portal_controller: Option<Ref<OverlayPortalController>>,
    layer_link: Option<Ref<LayerLink>>,
    // Dart's `this._defaultRangeThumbSelector` tear-off, one object for the state's life, so
    // the theme built from it equals the one of the last build.
    default_range_thumb_selector: RangeThumbSelector,
}

thread_local! {
    // Keyboard mapping for a focused range slider.
    static TRADITIONAL_NAV_SHORTCUT_MAP: ShortcutMap = vec![
        adjust_shortcut(LogicalKeyboardKey::ARROW_UP, SliderAdjustmentType::Up),
        adjust_shortcut(LogicalKeyboardKey::ARROW_DOWN, SliderAdjustmentType::Down),
        adjust_shortcut(LogicalKeyboardKey::ARROW_LEFT, SliderAdjustmentType::Left),
        adjust_shortcut(LogicalKeyboardKey::ARROW_RIGHT, SliderAdjustmentType::Right),
    ];

    // Keyboard mapping for a focused range slider when using directional
    // navigation and not in the value adjustment mode. Only the enter key is
    // handled, to enter the value adjustment mode. The arrow keys are left
    // unhandled so they can move the focus between views.
    static DIRECTIONAL_NAV_SHORTCUT_MAP: ShortcutMap = vec![toggle_edit_mode_shortcut()];

    // Keyboard mapping for a focused range slider when using directional
    // navigation and in the value adjustment mode. The horizontal inputs adjust
    // the value and the enter key exits the value adjustment mode. The vertical
    // inputs are not handled to allow navigating out of the slider.
    static DIRECTIONAL_NAV_EDITING_SHORTCUT_MAP: ShortcutMap = vec![
        adjust_shortcut(LogicalKeyboardKey::ARROW_LEFT, SliderAdjustmentType::Left),
        adjust_shortcut(LogicalKeyboardKey::ARROW_RIGHT, SliderAdjustmentType::Right),
        toggle_edit_mode_shortcut(),
    ];
}

fn adjust_shortcut(
    key: LogicalKeyboardKey,
    adjustment_type: SliderAdjustmentType,
) -> (ShortcutActivatorRef, IntentRef) {
    (
        Rc::new(SingleActivator::new(key)) as ShortcutActivatorRef,
        Rc::new(AdjustSliderIntent::new(adjustment_type)) as IntentRef,
    )
}

fn toggle_edit_mode_shortcut() -> (ShortcutActivatorRef, IntentRef) {
    (
        Rc::new(SingleActivator::new(LogicalKeyboardKey::ENTER)) as ShortcutActivatorRef,
        Rc::new(ToggleRangeSliderEditModeIntent) as IntentRef,
    )
}

impl RangeSliderState {
    const ENABLE_ANIMATION_DURATION: Duration = Duration::from_millis(75);
    const VALUE_INDICATOR_ANIMATION_DURATION: Duration = Duration::from_millis(100);
}

fn erased_controller(
    controller: &Option<Ref<AnimationController>>,
    name: &str,
) -> AnimationRef<f64> {
    controller
        .as_ref()
        .unwrap_or_else(|| panic!("init_state made the {name} controller"))
        .erase()
}

fn controller_of(
    controller: &Option<Ref<AnimationController>>,
    name: &str,
) -> Handle<AnimationController> {
    controller
        .handle()
        .unwrap_or_else(|| panic!("init_state made the {name} controller"))
}

impl RangeSliderState {
    fn overlay_controller_ref(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        erased_controller(&app.get(self).overlay_controller, "overlay")
    }

    fn value_indicator_controller_ref(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        erased_controller(&app.get(self).value_indicator_controller, "value indicator")
    }

    fn enable_controller_ref(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        erased_controller(&app.get(self).enable_controller, "enable")
    }

    fn overlay_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        controller_of(&app.get(self).overlay_controller, "overlay")
    }

    fn value_indicator_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        controller_of(&app.get(self).value_indicator_controller, "value indicator")
    }

    fn enable_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        controller_of(&app.get(self).enable_controller, "enable")
    }

    fn start_position_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        controller_of(&app.get(self).start_position_controller, "start position")
    }

    fn end_position_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        controller_of(&app.get(self).end_position_controller, "end position")
    }

    fn start_focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        FocusNodeLeaf::as_node(
            app.get(self)
                .start_focus_node
                .handle()
                .expect("init_state made the start focus node"),
        )
    }

    fn end_focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        FocusNodeLeaf::as_node(
            app.get(self)
                .end_focus_node
                .handle()
                .expect("init_state made the end focus node"),
        )
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).on_changed.is_some()
    }

    fn handle_start_focus_highlight_changed(
        self: Handle<Self>,
        app: &mut App,
        show_focus_highlight: bool,
    ) {
        if show_focus_highlight != app.get(self).show_start_focus_highlight {
            self.set_state(app, |state| {
                state.show_start_focus_highlight = show_focus_highlight;
            });
        }
    }

    fn handle_end_focus_highlight_changed(
        self: Handle<Self>,
        app: &mut App,
        show_focus_highlight: bool,
    ) {
        if show_focus_highlight != app.get(self).show_end_focus_highlight {
            self.set_state(app, |state| {
                state.show_end_focus_highlight = show_focus_highlight;
            });
        }
    }

    fn handle_hover_changed(self: Handle<Self>, app: &mut App, hovering: bool) {
        if hovering != app.get(self).hovering {
            let enabled = self.enabled(app);
            self.set_state(app, |state| {
                state.hovering = hovering;
                state.show_hover_highlight = hovering && enabled;
            });
        }
    }

    fn handle_changed(self: Handle<Self>, app: &mut App, values: RangeValues) {
        debug_assert!(self.enabled(app));
        let lerp_values = self.lerp_range_values(app, values);
        if lerp_values != self.widget(app).values {
            let on_changed = self
                .widget(app)
                .on_changed
                .clone()
                .expect("an enabled slider");
            on_changed(app, lerp_values);
        }
    }

    fn action_handler(
        self: Handle<Self>,
        app: &mut App,
        intent: &AdjustSliderIntent,
        thumb: Thumb,
    ) {
        let render_object_key = app.get(self).render_object_key.clone();
        let context = render_object_key
            .current_context(app)
            .expect("the slider's render object widget is mounted");
        let directionality = Directionality::of(app, context);
        let increase = match intent.adjustment_type {
            SliderAdjustmentType::Up => true,
            SliderAdjustmentType::Down => false,
            SliderAdjustmentType::Left => directionality == TextDirection::Rtl,
            SliderAdjustmentType::Right => directionality == TextDirection::Ltr,
        };
        let slider = context
            .find_render_object(app)
            .and_then(|render_object| render_object.downcast::<RenderRangeSlider>(app))
            .expect("the slider's render object is a _RenderRangeSlider");
        match thumb {
            Thumb::Start => {
                if increase {
                    slider.increase_start_action(app)
                } else {
                    slider.decrease_start_action(app)
                }
            }
            Thumb::End => {
                if increase {
                    slider.increase_end_action(app)
                } else {
                    slider.decrease_end_action(app)
                }
            }
        }
    }

    // Toggles the value adjustment mode used by directional navigation. Entered
    // and exited by pressing the enter key on a focused thumb.
    fn toggle_edit_mode(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.is_editing_in_directional_mode = !state.is_editing_in_directional_mode;
        });
    }

    // Exits the value adjustment mode when a thumb loses focus, so the slider
    // does not stay in the editing state after the user navigates away.
    fn handle_focus_changed(self: Handle<Self>, app: &mut App, has_focus: bool) {
        if !has_focus && app.get(self).is_editing_in_directional_mode {
            self.set_state(app, |state| {
                state.is_editing_in_directional_mode = false;
            });
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, values: RangeValues) {
        self.set_state(app, |state| {
            state.dragging = true;
        });
        if let Some(on_change_start) = self.widget(app).on_change_start.clone() {
            let lerp_values = self.lerp_range_values(app, values);
            on_change_start(app, lerp_values);
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, values: RangeValues) {
        self.set_state(app, |state| {
            state.dragging = false;
        });
        if let Some(on_change_end) = self.widget(app).on_change_end.clone() {
            let lerp_values = self.lerp_range_values(app, values);
            on_change_end(app, lerp_values);
        }
    }

    // Returns a number between min and max, proportional to value, which must
    // be between 0.0 and 1.0.
    fn lerp(self: Handle<Self>, app: &App, value: f64) -> f64 {
        debug_assert!(value >= 0.0);
        debug_assert!(value <= 1.0);
        let widget = self.widget(app);
        if let Some(divisions) = widget.divisions {
            let divisions = f64::from(divisions);
            return widget.min
                + (value * divisions).round() * (widget.max - widget.min) / divisions;
        }
        lerp_double(Some(widget.min), Some(widget.max), value).expect("both bounds")
    }

    // Returns a new range value with the start and end lerped.
    fn lerp_range_values(self: Handle<Self>, app: &App, values: RangeValues) -> RangeValues {
        RangeValues::new(self.lerp(app, values.start), self.lerp(app, values.end))
    }

    // Returns a number between 0.0 and 1.0, given a value between min and max.
    fn unlerp(self: Handle<Self>, app: &App, value: f64) -> f64 {
        let widget = self.widget(app);
        debug_assert!(value <= widget.max);
        debug_assert!(value >= widget.min);
        if widget.max > widget.min {
            (value - widget.min) / (widget.max - widget.min)
        } else {
            0.0
        }
    }

    // Returns a new range value with the start and end unlerped.
    fn unlerp_range_values(self: Handle<Self>, app: &App, values: RangeValues) -> RangeValues {
        RangeValues::new(self.unlerp(app, values.start), self.unlerp(app, values.end))
    }

    fn build_thumb_focus_detector(
        self: Handle<Self>,
        app: &App,
        focus_node: FocusNodeRef,
        actions: HashMap<TypeId, ActionRef>,
        on_show_focus_highlight: impl Fn(&mut App, bool) + 'static,
    ) -> WidgetRef {
        FocusableActionDetector::new(SizedBox::shrink())
            .focus_node(focus_node)
            .enabled(self.enabled(app))
            .include_focus_semantics(false)
            .actions(actions)
            .on_show_focus_highlight(on_show_focus_highlight)
            .on_focus_change(move |app, has_focus| self.handle_focus_changed(app, has_focus))
            .into_widget()
    }

    fn build_value_indicator(
        self: Handle<Self>,
        app: &mut App,
        show_value_indicator: ShowValueIndicator,
    ) -> WidgetRef {
        let layer_link = app
            .get(self)
            .layer_link
            .clone()
            .expect("init_state made the layer link");
        let value_indicator = || {
            CompositedTransformFollower::new(layer_link.clone())
                .child(ValueIndicatorRenderObjectWidget { state: self })
                .into_widget()
        };
        let divisions = self.widget(app).divisions;
        #[expect(deprecated, reason = "Dart's switch names the deprecated `always`")]
        match show_value_indicator {
            ShowValueIndicator::Never => SizedBox::shrink().into_widget(),
            ShowValueIndicator::OnlyForDiscrete => {
                if divisions.is_some() {
                    value_indicator()
                } else {
                    SizedBox::shrink().into_widget()
                }
            }
            ShowValueIndicator::OnlyForContinuous => {
                if divisions.is_none() {
                    value_indicator()
                } else {
                    SizedBox::shrink().into_widget()
                }
            }
            ShowValueIndicator::AlwaysVisible
            | ShowValueIndicator::Always
            | ShowValueIndicator::OnDrag => value_indicator(),
        }
    }
}

/// Finds the closest thumb. If both thumbs are close to each other and within
/// the touch radius, neither is selected immediately while the drag
/// displacement is zero. The first non-zero displacement determines which
/// thumb is selected: a negative displacement selects the left thumb,
/// a positive one selects the right thumb.
/// If only one or zero thumbs are within the touch radius,
/// the closest one is selected.
///
/// `dx` is the horizontal delta or displacement of the drag update.
fn default_range_thumb_selector(
    text_direction: TextDirection,
    values: RangeValues,
    tap_value: f64,
    thumb_size: Size,
    track_size: Size,
    dx: f64,
) -> Option<Thumb> {
    let touch_radius = thumb_size.width().max(RangeSlider::MIN_TOUCH_TARGET_WIDTH) / 2.0;
    let in_start_touch_target =
        (tap_value - values.start).abs() * track_size.width() < touch_radius;
    let in_end_touch_target = (tap_value - values.end).abs() * track_size.width() < touch_radius;

    // Use dx if the thumb touch targets overlap. If dx is 0 and the drag
    // position is in both touch targets, no thumb is selected because it is
    // ambiguous to which thumb should be selected. If the dx is non-zero, the
    // thumb selection is determined by the direction of the dx. The left thumb
    // is chosen for negative dx, and the right thumb is chosen for positive dx.
    if in_start_touch_target && in_end_touch_target {
        let (towards_start, towards_end) = match text_direction {
            TextDirection::Ltr => (dx < 0.0, dx > 0.0),
            TextDirection::Rtl => (dx > 0.0, dx < 0.0),
        };
        if towards_start {
            return Some(Thumb::Start);
        }
        if towards_end {
            return Some(Thumb::End);
        }
    } else {
        // Choose the closest thumb and snap position.
        if tap_value * 2.0 < values.start + values.end {
            return Some(Thumb::Start);
        } else {
            return Some(Thumb::End);
        }
    }
    None
}

impl TickerProviderStateMixin for RangeSliderState {
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
impl TickerProviderObject for RangeSliderState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for RangeSliderState {
    type Widget = RangeSlider;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let start_focus_node = FocusNode::new(app);
        let end_focus_node = FocusNode::new(app);
        let portal_controller =
            OverlayPortalController::new(app, Some("RangeSlider ValueIndicator".to_string()));
        portal_controller.show(app);
        let layer_link = LayerLink::new(app);

        let adjust = |app: &mut App, thumb: Thumb| -> ActionRef {
            CallbackAction::<AdjustSliderIntent>::new(
                app,
                Rc::new(move |app: &mut App, intent: &AdjustSliderIntent| {
                    self.action_handler(app, intent, thumb);
                    None
                }),
            )
            .into_erased()
        };
        let toggle = |app: &mut App| -> ActionRef {
            CallbackAction::<ToggleRangeSliderEditModeIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, _intent: &ToggleRangeSliderEditModeIntent| {
                        self.toggle_edit_mode(app);
                        None
                    },
                ),
            )
            .into_erased()
        };
        let start_action_map = HashMap::from([
            (
                TypeId::of::<AdjustSliderIntent>(),
                adjust(app, Thumb::Start),
            ),
            (TypeId::of::<ToggleRangeSliderEditModeIntent>(), toggle(app)),
        ]);
        let end_action_map = HashMap::from([
            (TypeId::of::<AdjustSliderIntent>(), adjust(app, Thumb::End)),
            (TypeId::of::<ToggleRangeSliderEditModeIntent>(), toggle(app)),
        ]);
        {
            let this = app.get_mut(self);
            this.start_focus_node = Some(start_focus_node);
            this.end_focus_node = Some(end_focus_node);
            this.value_indicator_overlay_portal_controller = Some(portal_controller);
            this.layer_link = Some(layer_link);
            this.start_action_map = start_action_map;
            this.end_action_map = end_action_map;
        }

        let controller = |app: &mut App, value: Option<f64>, duration: Duration| {
            AnimationController::new(
                app,
                value,
                Some(duration),
                None,
                0.0,
                1.0,
                AnimationBehavior::Normal,
                self,
            )
        };
        let enabled = self.enabled(app);
        let start = self.unlerp(app, self.widget(app).values.start);
        let end = self.unlerp(app, self.widget(app).values.end);
        let overlay_controller = controller(app, None, K_RADIAL_REACTION_DURATION);
        let value_indicator_controller =
            controller(app, None, Self::VALUE_INDICATOR_ANIMATION_DURATION);
        let enable_controller = controller(
            app,
            Some(if enabled { 1.0 } else { 0.0 }),
            Self::ENABLE_ANIMATION_DURATION,
        );
        let start_position_controller = controller(app, Some(start), Duration::ZERO);
        let end_position_controller = controller(app, Some(end), Duration::ZERO);
        let this = app.get_mut(self);
        this.overlay_controller = Some(overlay_controller);
        this.value_indicator_controller = Some(value_indicator_controller);
        this.enable_controller = Some(enable_controller);
        this.start_position_controller = Some(start_position_controller);
        this.end_position_controller = Some(end_position_controller);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let navigation_mode = MediaQuery::navigation_mode_of(app, context);
        app.get_mut(self).navigation_mode = navigation_mode;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RangeSlider) {
        let widget = self.widget(app);
        let (was_enabled, is_enabled) =
            (old_widget.on_changed.is_some(), widget.on_changed.is_some());
        // Dart returns early when both callbacks are the same object; a closure has no
        // identity here, so the enabled flag is the only thing the rest of the body reads.
        if was_enabled != is_enabled {
            let enable_controller = self.enable_controller(app);
            if is_enabled {
                enable_controller.forward(app, None);
            } else {
                enable_controller.reverse(app, None);
            }
            let hovering = app.get(self).hovering;
            app.get_mut(self).show_hover_highlight = hovering && is_enabled;
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).interaction_timer.take() {
            timer.cancel(app);
        }
        let controllers = [
            self.overlay_controller(app),
            self.value_indicator_controller(app),
            self.enable_controller(app),
            self.start_position_controller(app),
            self.end_position_controller(app),
        ];
        for controller in controllers {
            controller.dispose(app);
        }
        self.start_focus_node(app).dispose(app);
        self.end_focus_node(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_media_query(app, context));

        let widget = self.widget(app).clone();
        let theme = Theme::of(app, context);
        let mut slider_theme = SliderTheme::of(app, context);
        let year2023 = widget.year2023.or(slider_theme.year2023).unwrap_or(true);
        let defaults: Box<dyn SliderThemeDataGetters> = if theme.use_material3 && !year2023 {
            Box::new(RangeSliderDefaultsM3::new(context))
        } else {
            Box::new(RangeSliderDefaultsM2::new(context))
        };

        // If the widget has active or inactive colors specified, then we plug them
        // in to the slider theme as best we can. If the developer wants more
        // control than that, then they need to use a SliderTheme. The default
        // colors come from the ThemeData.colorScheme. These colors, along with
        // the default shapes and text styles are aligned to the Material
        // Guidelines.

        let enabled = self.enabled(app);
        let (hovering, show_start_focus_highlight, show_end_focus_highlight, dragging) = {
            let state = app.get(self);
            (
                state.hovering,
                state.show_start_focus_highlight,
                state.show_end_focus_highlight,
                state.dragging,
            )
        };
        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }
        if hovering {
            states.insert(WidgetState::Hovered);
        }
        if show_start_focus_highlight || show_end_focus_highlight {
            states.insert(WidgetState::Focused);
        }
        if dragging {
            states.insert(WidgetState::Dragged);
        }

        // The value indicator's color is not the same as the thumb and active track
        // (which can be defined by activeColor) if the
        // RectangularSliderValueIndicatorShape is used. In all other cases, the
        // value indicator is assumed to be the same as the active color.
        let value_indicator_shape: RangeSliderValueIndicatorShapeRef = slider_theme
            .range_value_indicator_shape
            .clone()
            .or_else(|| defaults.range_value_indicator_shape(app))
            .expect("the defaults have a range value indicator shape");
        let value_indicator_color = if value_indicator_shape
            .as_any()
            .is::<RectangularRangeSliderValueIndicatorShape>()
        {
            #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
            let fallback = Color::alpha_blend(
                theme.color_scheme.on_surface.with_opacity(0.60),
                theme.color_scheme.surface.with_opacity(0.90),
            );
            slider_theme.value_indicator_color.unwrap_or(fallback)
        } else {
            widget
                .active_color
                .or(slider_theme.value_indicator_color)
                .or_else(|| defaults.value_indicator_color(app))
                .expect("the defaults have a value indicator color")
        };

        #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
        let effective_overlay_color = widget
            .overlay_color
            .as_ref()
            .and_then(|overlay_color| overlay_color.resolve(&states))
            .or_else(|| {
                widget
                    .active_color
                    .map(|active_color| active_color.with_opacity(0.12))
            })
            .or_else(|| {
                <dyn WidgetStateProperty<Color>>::resolve_as(&slider_theme.overlay_color, &states)
                    .map(|color| color.color())
            })
            .or_else(|| {
                defaults
                    .overlay_color(app)
                    .map(|overlay_color| overlay_color.color())
            });

        let default_range_thumb_selector = app.get(self).default_range_thumb_selector.clone();
        slider_theme = SliderThemeData {
            track_height: slider_theme
                .track_height
                .or_else(|| defaults.track_height(app)),
            active_track_color: widget
                .active_color
                .or(slider_theme.active_track_color)
                .or_else(|| defaults.active_track_color(app)),
            inactive_track_color: widget
                .inactive_color
                .or(slider_theme.inactive_track_color)
                .or_else(|| defaults.inactive_track_color(app)),
            disabled_active_track_color: slider_theme
                .disabled_active_track_color
                .or_else(|| defaults.disabled_active_track_color(app)),
            disabled_inactive_track_color: slider_theme
                .disabled_inactive_track_color
                .or_else(|| defaults.disabled_inactive_track_color(app)),
            active_tick_mark_color: widget
                .inactive_color
                .or(slider_theme.active_tick_mark_color)
                .or_else(|| defaults.active_tick_mark_color(app)),
            inactive_tick_mark_color: widget
                .active_color
                .or(slider_theme.inactive_tick_mark_color)
                .or_else(|| defaults.inactive_tick_mark_color(app)),
            disabled_active_tick_mark_color: slider_theme
                .disabled_active_tick_mark_color
                .or_else(|| defaults.disabled_active_tick_mark_color(app)),
            disabled_inactive_tick_mark_color: slider_theme
                .disabled_inactive_tick_mark_color
                .or_else(|| defaults.disabled_inactive_tick_mark_color(app)),
            thumb_color: widget
                .active_color
                .or(slider_theme.thumb_color)
                .or_else(|| defaults.thumb_color(app)),
            overlapping_shape_stroke_color: slider_theme
                .overlapping_shape_stroke_color
                .or_else(|| defaults.overlapping_shape_stroke_color(app)),
            disabled_thumb_color: slider_theme
                .disabled_thumb_color
                .or_else(|| defaults.disabled_thumb_color(app)),
            overlay_color: effective_overlay_color
                .map(Into::into)
                .or_else(|| slider_theme.overlay_color.clone()),
            value_indicator_color: Some(value_indicator_color),
            range_track_shape: slider_theme
                .range_track_shape
                .clone()
                .or_else(|| defaults.range_track_shape(app)),
            range_tick_mark_shape: slider_theme
                .range_tick_mark_shape
                .clone()
                .or_else(|| defaults.range_tick_mark_shape(app)),
            range_thumb_shape: slider_theme
                .range_thumb_shape
                .clone()
                .or_else(|| defaults.range_thumb_shape(app)),
            overlay_shape: slider_theme
                .overlay_shape
                .clone()
                .or_else(|| defaults.overlay_shape(app)),
            range_value_indicator_shape: Some(value_indicator_shape),
            show_value_indicator: slider_theme
                .show_value_indicator
                .or_else(|| defaults.show_value_indicator(app)),
            value_indicator_text_style: slider_theme
                .value_indicator_text_style
                .clone()
                .or_else(|| defaults.value_indicator_text_style(app)),
            min_thumb_separation: slider_theme
                .min_thumb_separation
                .or_else(|| defaults.min_thumb_separation(app)),
            thumb_selector: Some(
                slider_theme
                    .thumb_selector
                    .clone()
                    .unwrap_or(default_range_thumb_selector),
            ),
            padding: widget.padding.or(slider_theme.padding),
            thumb_size: slider_theme
                .thumb_size
                .clone()
                .or_else(|| defaults.thumb_size(app)),
            track_gap: slider_theme.track_gap.or_else(|| defaults.track_gap(app)),
            ..slider_theme
        };
        let effective_mouse_cursor: MouseCursorRef = widget
            .mouse_cursor
            .as_ref()
            .and_then(|mouse_cursor| mouse_cursor.resolve(&states))
            .or_else(|| {
                slider_theme
                    .mouse_cursor
                    .as_ref()
                    .and_then(|mouse_cursor| mouse_cursor.resolve(&states))
            })
            .unwrap_or_else(|| WidgetStateMouseCursor::clickable().resolve(&states));

        // This size is used as the max bounds for the painting of the value
        // indicators. It must be kept in sync with the function with the same name
        // in slider.dart.
        let screen_size = MediaQuery::size_of(app, context);

        let font_size = slider_theme
            .value_indicator_text_style
            .as_ref()
            .and_then(|style| style.font_size)
            .unwrap_or(K_DEFAULT_FONT_SIZE);
        let font_size_to_scale = if font_size == 0.0 {
            K_DEFAULT_FONT_SIZE
        } else {
            font_size
        };
        let effective_text_scale =
            MediaQuery::text_scaler_of(app, context).scale(font_size_to_scale) / font_size_to_scale;

        let layer_link = app
            .get(self)
            .layer_link
            .clone()
            .expect("init_state made the layer link");
        let show_value_indicator = slider_theme
            .show_value_indicator
            .expect("the defaults have a show_value_indicator");
        let portal_controller = app
            .get(self)
            .value_indicator_overlay_portal_controller
            .clone()
            .expect("init_state made the overlay portal controller");
        let on_changed: Option<ValueChanged<RangeValues>> = (enabled && widget.max > widget.min)
            .then(|| {
                Rc::new(move |app: &mut App, values: RangeValues| self.handle_changed(app, values))
                    as ValueChanged<RangeValues>
            });
        let on_change_start: ValueChanged<RangeValues> =
            Rc::new(move |app: &mut App, values: RangeValues| self.handle_drag_start(app, values));
        let on_change_end: ValueChanged<RangeValues> =
            Rc::new(move |app: &mut App, values: RangeValues| self.handle_drag_end(app, values));
        let render_object_widget = RangeSliderRenderObjectWidget {
            key: Some(app.get(self).render_object_key.clone().into()),
            values: self.unlerp_range_values(app, widget.values),
            divisions: widget.divisions,
            labels: widget.labels.clone(),
            slider_theme: slider_theme.clone(),
            text_scale_factor: effective_text_scale,
            screen_size,
            on_changed,
            on_change_start,
            on_change_end,
            state: self,
            semantic_formatter_callback: widget.semantic_formatter_callback.clone(),
            hovering: app.get(self).show_hover_highlight,
            start_thumb_show_focus_highlight: show_start_focus_highlight,
            end_thumb_show_focus_highlight: show_end_focus_highlight,
        };
        let mut result = CompositedTransformTarget::new(layer_link)
            .child(
                OverlayPortal::new(portal_controller, move |app: &mut App, _context| {
                    self.build_value_indicator(app, show_value_indicator)
                })
                .child(render_object_widget),
            )
            .into_widget();

        let padding = widget.padding.or(slider_theme.padding);
        if let Some(padding) = padding {
            result = Padding::new(padding).child(result).into_widget();
        }

        let navigation_mode = app.get(self).navigation_mode;
        let is_editing_in_directional_mode = app.get(self).is_editing_in_directional_mode;
        let shortcut_map = match navigation_mode {
            NavigationMode::Directional => {
                if is_editing_in_directional_mode {
                    DIRECTIONAL_NAV_EDITING_SHORTCUT_MAP.with(Clone::clone)
                } else {
                    DIRECTIONAL_NAV_SHORTCUT_MAP.with(Clone::clone)
                }
            }
            NavigationMode::Traditional => TRADITIONAL_NAV_SHORTCUT_MAP.with(Clone::clone),
        };

        let start_focus_node = app
            .get(self)
            .start_focus_node
            .as_ref()
            .map(Ref::erase)
            .expect("init_state made the start focus node");
        let end_focus_node = app
            .get(self)
            .end_focus_node
            .as_ref()
            .map(Ref::erase)
            .expect("init_state made the end focus node");
        let start_action_map = app.get(self).start_action_map.clone();
        let end_action_map = app.get(self).end_action_map.clone();
        let start_detector = self.build_thumb_focus_detector(
            app,
            start_focus_node,
            start_action_map,
            move |app, show_focus_highlight| {
                self.handle_start_focus_highlight_changed(app, show_focus_highlight)
            },
        );
        let end_detector = self.build_thumb_focus_detector(
            app,
            end_focus_node,
            end_action_map,
            move |app, show_focus_highlight| {
                self.handle_end_focus_highlight_changed(app, show_focus_highlight)
            },
        );

        Stack::new()
            .children([
                // Adds two invisible focus nodes to the range slider for its two thumbs.
                Shortcuts::new(
                    shortcut_map,
                    Row::new().children([start_detector, end_detector]),
                )
                .into_widget(),
                MouseRegion::new()
                    .on_enter(Rc::new(move |app: &mut App, _event: PointerEnterEvent| {
                        self.handle_hover_changed(app, true)
                    }))
                    .on_exit(Rc::new(move |app: &mut App, _event: PointerExitEvent| {
                        self.handle_hover_changed(app, false)
                    }))
                    .cursor(effective_mouse_cursor)
                    .child(result)
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// Dart's `_RangeSliderRenderObjectWidget`.
struct RangeSliderRenderObjectWidget {
    key: Option<KeyRef>,
    values: RangeValues,
    divisions: Option<i32>,
    labels: Option<RangeLabels>,
    slider_theme: SliderThemeData,
    text_scale_factor: f64,
    screen_size: Size,
    on_changed: Option<ValueChanged<RangeValues>>,
    on_change_start: ValueChanged<RangeValues>,
    on_change_end: ValueChanged<RangeValues>,
    semantic_formatter_callback: Option<SemanticFormatterCallback>,
    state: Handle<RangeSliderState>,
    hovering: bool,
    start_thumb_show_focus_highlight: bool,
    end_thumb_show_focus_highlight: bool,
}

impl Debug for RangeSliderRenderObjectWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_RangeSliderRenderObjectWidget")
            .field("values", &self.values)
            .field("divisions", &self.divisions)
            .field("labels", &self.labels)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for RangeSliderRenderObjectWidget {
    type RenderObject = RenderRangeSlider;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        let platform = Theme::of(app, context).platform;
        let gesture_settings = MediaQuery::gesture_settings_of(app, context);
        RenderRangeSlider::new(
            app,
            self.values,
            self.divisions,
            self.labels.clone(),
            self.slider_theme.clone(),
            self.text_scale_factor,
            self.screen_size,
            platform,
            self.on_changed.clone(),
            self.semantic_formatter_callback.clone(),
            Some(self.on_change_start.clone()),
            Some(self.on_change_end.clone()),
            self.state,
            text_direction,
            self.hovering,
            self.start_thumb_show_focus_highlight,
            self.end_thumb_show_focus_highlight,
            gesture_settings,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderRangeSlider>,
    ) {
        let text_direction = Directionality::of(app, context);
        let platform = Theme::of(app, context).platform;
        let gesture_settings = MediaQuery::gesture_settings_of(app, context);
        // We should update the `divisions` ahead of `values`, because the `values`
        // setter dependent on the `divisions`.
        render_object.set_divisions(app, self.divisions);
        render_object.set_values(app, self.values);
        render_object.set_labels(app, self.labels.clone());
        render_object.set_slider_theme(app, self.slider_theme.clone());
        render_object.set_text_scale_factor(app, self.text_scale_factor);
        render_object.set_screen_size(app, self.screen_size);
        render_object.set_on_changed(app, self.on_changed.clone());
        render_object.get_mut(app).on_change_start = Some(self.on_change_start.clone());
        render_object.get_mut(app).on_change_end = Some(self.on_change_end.clone());
        render_object.set_text_direction(app, text_direction);
        render_object.get_mut(app).semantic_formatter_callback =
            self.semantic_formatter_callback.clone();
        render_object.set_platform(app, platform);
        render_object.set_hovering(app, self.hovering);
        render_object
            .set_start_thumb_show_focus_highlight(app, self.start_thumb_show_focus_highlight);
        render_object.set_end_thumb_show_focus_highlight(app, self.end_thumb_show_focus_highlight);
        render_object.set_gesture_settings(app, Some(gesture_settings));
    }
}

impl LeafRenderObjectWidget for RangeSliderRenderObjectWidget {}

/// Dart's `_RenderRangeSlider extends RenderBox with RelayoutWhenSystemFontsChangeMixin`.
struct RenderRangeSlider {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    system_fonts: RelayoutWhenSystemFontsChangeData,
    values: RangeValues,
    divisions: Option<i32>,
    labels: Option<RangeLabels>,
    slider_theme: SliderThemeData,
    text_scale_factor: f64,
    screen_size: Size,
    platform: TargetPlatform,
    on_changed: Option<ValueChanged<RangeValues>>,
    semantic_formatter_callback: Option<SemanticFormatterCallback>,
    on_change_start: Option<ValueChanged<RangeValues>>,
    on_change_end: Option<ValueChanged<RangeValues>>,
    state: Handle<RangeSliderState>,
    text_direction: TextDirection,
    hovering: bool,
    start_thumb_show_focus_highlight: bool,
    end_thumb_show_focus_highlight: bool,
    hovering_start_thumb: bool,
    hovering_end_thumb: bool,
    // Keep track of the last selected thumb so they can be drawn in the
    // right order.
    last_thumb_selection: Option<Thumb>,
    overlay_animation: Ref<CurvedAnimation>,
    value_indicator_animation: Ref<CurvedAnimation>,
    enable_animation: Ref<CurvedAnimation>,
    start_label_painter: TextPainter,
    end_label_painter: TextPainter,
    drag: Ref<HorizontalDragGestureRecognizer>,
    tap: Ref<TapGestureRecognizer>,
    active: bool,
    new_values: RangeValues,
    start_thumb_center: Offset,
    end_thumb_center: Offset,
    overlay_start_rect: Option<Rect>,
    overlay_end_rect: Option<Rect>,
}

impl RenderRangeSlider {
    const POSITION_ANIMATION_DURATION: Duration = Duration::from_millis(75);

    // This value is the touch target, 48, multiplied by 3.
    const MIN_PREFERRED_TRACK_WIDTH: f64 = 144.0;

    // Buffer to account for the internal padding of standard Material value indicator shapes,
    // preventing them from bleeding off the screen edges.
    //
    // The value 64.0 is a heuristic that covers the minimum size of the shape
    // (padding + minimum label width) at a text scale factor of roughly 2.0.
    const VALUE_INDICATOR_HORIZONTAL_BUFFER: f64 = 64.0;

    const MINIMUM_INTERACTION_TIME: Duration = Duration::from_millis(500);

    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor parameters, in their order"
    )]
    fn new(
        app: &mut App,
        values: RangeValues,
        divisions: Option<i32>,
        labels: Option<RangeLabels>,
        slider_theme: SliderThemeData,
        text_scale_factor: f64,
        screen_size: Size,
        platform: TargetPlatform,
        on_changed: Option<ValueChanged<RangeValues>>,
        semantic_formatter_callback: Option<SemanticFormatterCallback>,
        on_change_start: Option<ValueChanged<RangeValues>>,
        on_change_end: Option<ValueChanged<RangeValues>>,
        state: Handle<RangeSliderState>,
        text_direction: TextDirection,
        hovering: bool,
        start_thumb_show_focus_highlight: bool,
        end_thumb_show_focus_highlight: bool,
        gesture_settings: DeviceGestureSettings,
    ) -> RenderHandle<RenderRangeSlider> {
        debug_assert!(values.start >= 0.0 && values.start <= 1.0);
        debug_assert!(values.end >= 0.0 && values.end <= 1.0);
        let drag = HorizontalDragGestureRecognizer::new(app);
        let tap = TapGestureRecognizer::new(app);
        let overlay_animation = CurvedAnimation::new(
            app,
            state.overlay_controller_ref(app),
            Curves::fast_out_slow_in(),
            None,
        );
        let value_indicator_animation = CurvedAnimation::new(
            app,
            state.value_indicator_controller_ref(app),
            Curves::fast_out_slow_in(),
            None,
        );
        let enable_animation = CurvedAnimation::new(
            app,
            state.enable_controller_ref(app),
            Curves::ease_in_out(),
            None,
        );
        let this = RenderHandle::new_box(
            app,
            RenderRangeSlider {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                system_fonts: RelayoutWhenSystemFontsChangeData::new(),
                values,
                divisions,
                labels,
                slider_theme,
                text_scale_factor,
                screen_size,
                platform,
                on_changed,
                semantic_formatter_callback,
                on_change_start,
                on_change_end,
                state,
                text_direction,
                hovering,
                start_thumb_show_focus_highlight,
                end_thumb_show_focus_highlight,
                hovering_start_thumb: false,
                hovering_end_thumb: false,
                last_thumb_selection: None,
                overlay_animation,
                value_indicator_animation,
                enable_animation,
                start_label_painter: TextPainter::new(),
                end_label_painter: TextPainter::new(),
                drag: drag.clone(),
                tap: tap.clone(),
                active: false,
                new_values: RangeValues::new(0.0, 0.0),
                start_thumb_center: Offset::ZERO,
                end_thumb_center: Offset::ZERO,
                overlay_start_rect: None,
                overlay_end_rect: None,
            },
        );
        this.update_label_painters(app);
        let team = GestureArenaTeam::new(app);
        drag.set_team(app, team.clone());
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
        drag.set_on_cancel(
            app,
            Some(Listener::new(move |app: &mut App| {
                this.end_interaction(app)
            })),
        );
        drag.set_gesture_settings(app, Some(gesture_settings));
        tap.set_team(app, team);
        tap.set_on_tap_down(
            app,
            Some(Rc::new(move |app: &mut App, details: TapDownDetails| {
                this.handle_tap_down(app, details)
            })),
        );
        tap.set_on_tap_up(
            app,
            Some(Rc::new(move |app: &mut App, details: TapUpDetails| {
                this.handle_tap_up(app, details)
            })),
        );
        tap.set_gesture_settings(app, Some(gesture_settings));
        this
    }

    // Compute the largest width and height needed to paint the slider shapes,
    // other than the track shape. It is assumed that these shapes are vertically
    // centered on the track.
    fn max_slider_part_width(self: RenderHandle<Self>, app: &App) -> f64 {
        self.slider_part_sizes(app)
            .iter()
            .map(|size| size.width())
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn max_slider_part_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.slider_part_sizes(app)
            .iter()
            .map(|size| size.height())
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn thumb_size_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.range_thumb_shape(app)
            .get_preferred_size(self.is_enabled(app), self.is_discrete(app))
            .height()
    }

    fn overlay_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.overlay_shape(app)
            .get_preferred_size(self.is_enabled(app), self.is_discrete(app))
            .height()
    }

    fn slider_part_sizes(self: RenderHandle<Self>, app: &App) -> [Size; 3] {
        let (is_enabled, is_discrete) = (self.is_enabled(app), self.is_discrete(app));
        let slider_theme = &self.get(app).slider_theme;
        [
            Size::new(
                self.overlay_shape(app)
                    .get_preferred_size(is_enabled, is_discrete)
                    .width(),
                if slider_theme.padding.is_some() {
                    self.thumb_size_height(app)
                } else {
                    self.overlay_height(app)
                },
            ),
            self.range_thumb_shape(app)
                .get_preferred_size(is_enabled, is_discrete),
            slider_theme
                .range_tick_mark_shape
                .as_ref()
                .expect("the slider fills in its range tick mark shape")
                .get_preferred_size(slider_theme, Some(is_enabled)),
        ]
    }

    fn min_preferred_track_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app)
            .slider_theme
            .track_height
            .expect("the slider fills in its track height")
    }

    fn range_thumb_shape(self: RenderHandle<Self>, app: &App) -> RangeSliderThumbShapeRef {
        self.get(app)
            .slider_theme
            .range_thumb_shape
            .clone()
            .expect("the slider fills in its range thumb shape")
    }

    fn overlay_shape(self: RenderHandle<Self>, app: &App) -> SliderComponentShapeRef {
        self.get(app)
            .slider_theme
            .overlay_shape
            .clone()
            .expect("the slider fills in its overlay shape")
    }

    fn range_track_shape(self: RenderHandle<Self>, app: &App) -> RangeSliderTrackShapeRef {
        self.get(app)
            .slider_theme
            .range_track_shape
            .clone()
            .expect("the slider fills in its range track shape")
    }

    fn range_tick_mark_shape(self: RenderHandle<Self>, app: &App) -> RangeSliderTickMarkShapeRef {
        self.get(app)
            .slider_theme
            .range_tick_mark_shape
            .clone()
            .expect("the slider fills in its range tick mark shape")
    }

    fn range_value_indicator_shape(
        self: RenderHandle<Self>,
        app: &App,
    ) -> RangeSliderValueIndicatorShapeRef {
        self.get(app)
            .slider_theme
            .range_value_indicator_shape
            .clone()
            .expect("the slider fills in its range value indicator shape")
    }

    // This rect is used in gesture calculations, where the gesture coordinates
    // are relative to the sliders origin. Therefore, the offset is passed as
    // (0,0).
    fn track_rect(self: RenderHandle<Self>, app: &App) -> Rect {
        let slider_theme = &self.get(app).slider_theme;
        self.range_track_shape(app).get_preferred_rect(
            app,
            self.as_box(),
            Offset::ZERO,
            slider_theme,
            None,
            Some(false),
        )
    }

    fn is_enabled(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).on_changed.is_some()
    }

    fn is_discrete(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app)
            .divisions
            .is_some_and(|divisions| divisions > 0)
    }

    fn min_thumb_separation_value(self: RenderHandle<Self>, app: &App) -> f64 {
        if self.is_discrete(app) {
            0.0
        } else {
            self.get(app)
                .slider_theme
                .min_thumb_separation
                .expect("the slider fills in its min thumb separation")
                / self.track_rect(app).width()
        }
    }

    fn values(self: RenderHandle<Self>, app: &App) -> RangeValues {
        self.get(app).values
    }

    fn set_values(self: RenderHandle<Self>, app: &mut App, new_values: RangeValues) {
        debug_assert!(new_values.start >= 0.0 && new_values.start <= 1.0);
        debug_assert!(new_values.end >= 0.0 && new_values.end <= 1.0);
        debug_assert!(new_values.start <= new_values.end);
        let converted_values = if self.is_discrete(app) {
            self.discretize_range_values(app, new_values)
        } else {
            new_values
        };
        if converted_values == self.get(app).values {
            return;
        }
        self.get_mut(app).values = converted_values;
        let state = self.get(app).state;
        let (start_position_controller, end_position_controller) = (
            state.start_position_controller(app),
            state.end_position_controller(app),
        );
        if self.is_discrete(app) {
            // Reset the duration to match the distance that we're traveling, so that
            // whatever the distance, we still do it in _positionAnimationDuration,
            // and if we get re-targeted in the middle, it still takes that long to
            // get to the new location.
            let start_distance =
                (converted_values.start - start_position_controller.value(app)).abs();
            app.get_mut(start_position_controller).duration = Some(if start_distance != 0.0 {
                Self::POSITION_ANIMATION_DURATION.mul_f64(1.0 / start_distance)
            } else {
                Duration::ZERO
            });
            start_position_controller.animate_to(
                app,
                converted_values.start,
                None,
                Curves::ease_in_out(),
            );
            let end_distance = (converted_values.end - end_position_controller.value(app)).abs();
            app.get_mut(end_position_controller).duration = Some(if end_distance != 0.0 {
                Self::POSITION_ANIMATION_DURATION.mul_f64(1.0 / end_distance)
            } else {
                Duration::ZERO
            });
            end_position_controller.animate_to(
                app,
                converted_values.end,
                None,
                Curves::ease_in_out(),
            );
        } else {
            start_position_controller.set_value(app, converted_values.start);
            end_position_controller.set_value(app, converted_values.end);
        }
        // `markNeedsSemanticsUpdate()` waits with accessibility.
    }

    /// Dart's setter marks the semantics dirty when the platform changes; that waits with
    /// accessibility, so it only stores the platform.
    fn set_platform(self: RenderHandle<Self>, app: &mut App, value: TargetPlatform) {
        if self.get(app).platform == value {
            return;
        }
        self.get_mut(app).platform = value;
    }

    #[cfg(test)]
    fn gesture_settings(self: RenderHandle<Self>, app: &App) -> Option<DeviceGestureSettings> {
        self.get(app).drag.gesture_settings(app)
    }

    fn set_gesture_settings(
        self: RenderHandle<Self>,
        app: &mut App,
        gesture_settings: Option<DeviceGestureSettings>,
    ) {
        let (drag, tap) = (self.get(app).drag.handle(), self.get(app).tap.handle());
        drag.set_gesture_settings(app, gesture_settings);
        tap.set_gesture_settings(app, gesture_settings);
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

    fn labels(self: RenderHandle<Self>, app: &App) -> Option<&RangeLabels> {
        self.get(app).labels.as_ref()
    }

    fn set_labels(self: RenderHandle<Self>, app: &mut App, labels: Option<RangeLabels>) {
        if labels == self.get(app).labels {
            return;
        }
        self.get_mut(app).labels = labels;
        self.update_label_painters(app);
    }

    fn slider_theme(self: RenderHandle<Self>, app: &App) -> &SliderThemeData {
        &self.get(app).slider_theme
    }

    fn set_slider_theme(self: RenderHandle<Self>, app: &mut App, value: SliderThemeData) {
        if value == self.get(app).slider_theme {
            return;
        }
        self.get_mut(app).slider_theme = value;
        self.mark_needs_paint(app);
    }

    fn text_scale_factor(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).text_scale_factor
    }

    fn set_text_scale_factor(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value == self.get(app).text_scale_factor {
            return;
        }
        self.get_mut(app).text_scale_factor = value;
        self.update_label_painters(app);
    }

    fn screen_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).screen_size
    }

    fn set_screen_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if value == self.get(app).screen_size {
            return;
        }
        self.get_mut(app).screen_size = value;
        self.mark_needs_paint(app);
    }

    fn on_changed(self: RenderHandle<Self>, app: &App) -> Option<ValueChanged<RangeValues>> {
        self.get(app).on_changed.clone()
    }

    /// Dart's setter returns early when the callback is the same object; a closure has no
    /// identity here, so it always stores the new one and acts only when interactivity flips,
    /// as Dart's does. Marking the semantics dirty waits with accessibility.
    fn set_on_changed(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<ValueChanged<RangeValues>>,
    ) {
        let was_enabled = self.is_enabled(app);
        self.get_mut(app).on_changed = value;
        if was_enabled != self.is_enabled(app) {
            self.mark_needs_paint(app);
        }
    }

    fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if value == self.get(app).text_direction {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.update_label_painters(app);
    }

    fn hovering(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).hovering
    }

    fn set_hovering(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).hovering {
            return;
        }
        self.get_mut(app).hovering = value;
        self.update_for_hover(app, value);
    }

    fn start_thumb_show_focus_highlight(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).start_thumb_show_focus_highlight
    }

    fn set_start_thumb_show_focus_highlight(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).start_thumb_show_focus_highlight {
            return;
        }
        self.get_mut(app).start_thumb_show_focus_highlight = value;
        self.mark_needs_paint(app);
    }

    fn end_thumb_show_focus_highlight(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).end_thumb_show_focus_highlight
    }

    fn set_end_thumb_show_focus_highlight(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).end_thumb_show_focus_highlight {
            return;
        }
        self.get_mut(app).end_thumb_show_focus_highlight = value;
        self.mark_needs_paint(app);
    }

    fn hovering_start_thumb(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).hovering_start_thumb
    }

    fn set_hovering_start_thumb(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).hovering_start_thumb {
            return;
        }
        self.get_mut(app).hovering_start_thumb = value;
        let hovering = self.hovering(app);
        self.update_for_hover(app, hovering);
    }

    fn hovering_end_thumb(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).hovering_end_thumb
    }

    fn set_hovering_end_thumb(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).hovering_end_thumb {
            return;
        }
        self.get_mut(app).hovering_end_thumb = value;
        let hovering = self.hovering(app);
        self.update_for_hover(app, hovering);
    }

    fn update_for_hover(self: RenderHandle<Self>, app: &mut App, hovered: bool) {
        let state = self.get(app).state;
        // Only show overlay when pointer is hovering the thumb.
        if hovered && (self.hovering_start_thumb(app) || self.hovering_end_thumb(app)) {
            state.overlay_controller(app).forward(app, None);
        } else {
            // Keep the active drag overlay visible even if the pointer moves outside
            // the thumb bounds during an in-progress drag.
            if !self.get(app).active {
                state.overlay_controller(app).reverse(app, None);
            }
        }
    }

    fn should_always_show_value_indicator(self: RenderHandle<Self>, app: &App) -> bool {
        self.slider_theme(app).show_value_indicator == Some(ShowValueIndicator::AlwaysVisible)
    }

    fn should_show_value_indicator_when_dragged(self: RenderHandle<Self>, app: &App) -> bool {
        #[expect(deprecated, reason = "Dart's switch names the deprecated `always`")]
        match self
            .slider_theme(app)
            .show_value_indicator
            .expect("the slider fills in its show_value_indicator")
        {
            ShowValueIndicator::OnlyForDiscrete => self.is_discrete(app),
            ShowValueIndicator::OnlyForContinuous => !self.is_discrete(app),
            ShowValueIndicator::Always | ShowValueIndicator::OnDrag => true,
            ShowValueIndicator::Never | ShowValueIndicator::AlwaysVisible => false,
        }
    }

    fn thumb_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.range_thumb_shape(app)
            .get_preferred_size(self.is_enabled(app), self.is_discrete(app))
    }

    fn adjustment_unit(self: RenderHandle<Self>, app: &App) -> f64 {
        match self.get(app).platform {
            // Matches iOS implementation of material slider.
            TargetPlatform::IOS => 0.1,
            // Matches Android implementation of material slider.
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => 0.05,
        }
    }

    fn update_label_painters(self: RenderHandle<Self>, app: &mut App) {
        self.update_label_painter(app, Thumb::Start);
        self.update_label_painter(app, Thumb::End);
    }

    fn update_label_painter(self: RenderHandle<Self>, app: &mut App, thumb: Thumb) {
        let Some(labels) = self.labels(app).cloned() else {
            return;
        };

        let text = match thumb {
            Thumb::Start => labels.start,
            Thumb::End => labels.end,
        };

        // Reserve space for the bubble's internal padding and screen margins.
        let screen_size = self.screen_size(app);
        let safe_max_width =
            (screen_size.width() - Self::VALUE_INDICATOR_HORIZONTAL_BUFFER).max(0.0);
        let mut text_span = TextSpan::new().text(text);
        if let Some(style) = self.slider_theme(app).value_indicator_text_style.clone() {
            text_span = text_span.style(style);
        }
        let text_direction = self.text_direction(app);
        let text_scale_factor = self.text_scale_factor(app);
        let fonts = PaintingBinding::instance(app).fonts(app);
        let (this, fonts) = app.get_disjoint_mut(self.handle(), fonts);
        let label_painter = match thumb {
            Thumb::Start => &mut this.start_label_painter,
            Thumb::End => &mut this.end_label_painter,
        };
        label_painter.set_text(Some(Rc::new(text_span)));
        label_painter.set_text_direction(Some(text_direction));
        label_painter.set_text_scaler(TextScaler::linear(text_scale_factor));
        label_painter.set_max_lines(Some(1));
        // Standard Unicode ellipsis
        label_painter.set_ellipsis(Some("\u{2026}".to_string()));
        label_painter.layout(
            fonts,
            0.0,
            if screen_size.width().is_finite() && screen_size.width() > 0.0 {
                safe_max_width
            } else {
                f64::INFINITY
            },
        );

        // Changing the textDirection can result in the layout changing, because the
        // bidi algorithm might line up the glyphs differently which can result in
        // different ligatures, different shapes, etc. So we always markNeedsLayout.
        RenderBox::mark_needs_layout(self, app);
    }

    /// Lends a label painter to a shape's call, which Dart hands the painter object itself:
    /// it lives in this render object's arena slot, which the shape's `App` reaches.
    fn with_label_painter<R>(
        self: RenderHandle<Self>,
        app: &mut App,
        thumb: Thumb,
        call: impl FnOnce(&mut App, &mut TextPainter) -> R,
    ) -> R {
        let slot = |this: &mut RenderRangeSlider| match thumb {
            Thumb::Start => std::mem::replace(&mut this.start_label_painter, TextPainter::new()),
            Thumb::End => std::mem::replace(&mut this.end_label_painter, TextPainter::new()),
        };
        let mut label_painter = slot(self.get_mut(app));
        let result = call(app, &mut label_painter);
        let this = self.get_mut(app);
        match thumb {
            Thumb::Start => this.start_label_painter = label_painter,
            Thumb::End => this.end_label_painter = label_painter,
        }
        result
    }

    fn get_value_from_visual_position(
        self: RenderHandle<Self>,
        app: &App,
        visual_position: f64,
    ) -> f64 {
        match self.text_direction(app) {
            TextDirection::Rtl => 1.0 - visual_position,
            TextDirection::Ltr => visual_position,
        }
    }

    fn get_value_from_global_position(
        self: RenderHandle<Self>,
        app: &App,
        global_position: Offset,
    ) -> f64 {
        let local_dx = self
            .as_box()
            .global_to_local(app, global_position, None)
            .dx();
        let track_rect = self.track_rect(app);
        // Mirror the padded coordinate system used when painting tick marks and
        // thumbs on discrete rounded tracks (see _RenderSlider._getValueFromGlobalPosition).
        let visual_position = if self.is_discrete(app) && self.range_track_shape(app).is_rounded() {
            let padding = track_rect.height();
            let adjusted_width = track_rect.width() - padding;
            if adjusted_width <= 0.0 {
                0.5
            } else {
                (local_dx - track_rect.left - padding / 2.0) / adjusted_width
            }
        } else {
            (local_dx - track_rect.left) / track_rect.width()
        };
        self.get_value_from_visual_position(app, visual_position)
    }

    fn discretize(self: RenderHandle<Self>, app: &App, value: f64) -> f64 {
        let mut result = clamp_double(value, 0.0, 1.0);
        if self.is_discrete(app) {
            let divisions = f64::from(self.divisions(app).expect("a discrete slider"));
            result = (result * divisions).round() / divisions;
        }
        result
    }

    fn discretize_range_values(
        self: RenderHandle<Self>,
        app: &App,
        values: RangeValues,
    ) -> RangeValues {
        RangeValues::new(
            self.discretize(app, values.start),
            self.discretize(app, values.end),
        )
    }

    fn start_interaction(self: RenderHandle<Self>, app: &mut App, global_position: Offset) {
        let state = self.get(app).state;
        if !state.mounted(app) {
            return;
        }
        if self.get(app).active {
            return;
        }

        let tap_value = clamp_double(
            self.get_value_from_global_position(app, global_position),
            0.0,
            1.0,
        );
        let thumb_selector = self
            .slider_theme(app)
            .thumb_selector
            .clone()
            .expect("the slider fills in its thumb selector");
        let (text_direction, values, thumb_size, size) = (
            self.text_direction(app),
            self.values(app),
            self.thumb_size(app),
            self.size(app),
        );
        let last_thumb_selection =
            thumb_selector(text_direction, values, tap_value, thumb_size, size, 0.0);
        self.get_mut(app).last_thumb_selection = last_thumb_selection;

        if let Some(last_thumb_selection) = last_thumb_selection {
            match last_thumb_selection {
                Thumb::Start => state.start_focus_node(app).request_focus(app, None),
                Thumb::End => state.end_focus_node(app).request_focus(app, None),
            }

            // We supply the *current* values as the start locations, so that if we have
            // a tap, it consists of a call to onChangeStart with the previous value and
            // a call to onChangeEnd with the new value.
            let current_values = self.discretize_range_values(app, values);
            let new_values = match last_thumb_selection {
                Thumb::Start => RangeValues::new(tap_value, current_values.end),
                Thumb::End => RangeValues::new(current_values.start, tap_value),
            };
            let this = self.get_mut(app);
            this.active = true;
            this.new_values = new_values;
            self.update_label_painter(app, last_thumb_selection);

            if let Some(on_change_start) = self.get(app).on_change_start.clone() {
                on_change_start(app, current_values);
            }

            let on_changed = self.on_changed(app).expect("an enabled slider");
            let discrete_values = self.discretize_range_values(app, new_values);
            on_changed(app, discrete_values);

            state.overlay_controller(app).forward(app, None);
            if self.should_show_value_indicator_when_dragged(app) {
                state.value_indicator_controller(app).forward(app, None);
                if let Some(timer) = app.get_mut(state).interaction_timer.take() {
                    timer.cancel(app);
                }
                let time_dilation = SchedulerBinding::time_dilation(app);
                // The timer keeps the render slider, as Dart's closure does: a slider
                // replaced before it fires still reads `active`.
                let this = self.handle().keep(app);
                let timer = Timer::new(
                    app,
                    Self::MINIMUM_INTERACTION_TIME.mul_f64(time_dilation),
                    Listener::new(move |app: &mut App| {
                        app.get_mut(state).interaction_timer = None;
                        let value_indicator_controller = state.value_indicator_controller(app);
                        let active = app.get(&this).active;
                        if !active && value_indicator_controller.is_completed(app) {
                            value_indicator_controller.reverse(app, None);
                        }
                    }),
                );
                app.get_mut(state).interaction_timer = Some(timer);
            }
        }
    }

    fn handle_drag_update(self: RenderHandle<Self>, app: &mut App, details: DragUpdateDetails) {
        let state = self.get(app).state;
        if !state.mounted(app) {
            return;
        }

        let drag_value = self.get_value_from_global_position(app, details.global_position);

        // If no selection has been made yet, test for thumb selection again now
        // that the value of dx can be non-zero. If this is the first selection of
        // the interaction, then onChangeStart must be called.
        let mut should_call_on_change_start = false;
        if self.get(app).last_thumb_selection.is_none() {
            let thumb_selector = self
                .slider_theme(app)
                .thumb_selector
                .clone()
                .expect("the slider fills in its thumb selector");
            let (text_direction, values, thumb_size, size) = (
                self.text_direction(app),
                self.values(app),
                self.thumb_size(app),
                self.size(app),
            );
            let last_thumb_selection = thumb_selector(
                text_direction,
                values,
                drag_value,
                thumb_size,
                size,
                details.delta.dx(),
            );
            self.get_mut(app).last_thumb_selection = last_thumb_selection;
            if last_thumb_selection.is_some() {
                should_call_on_change_start = true;
                self.get_mut(app).active = true;
                state.overlay_controller(app).forward(app, None);
                if self.should_show_value_indicator_when_dragged(app) {
                    state.value_indicator_controller(app).forward(app, None);
                }
            }
        }

        if let (true, Some(last_thumb_selection)) =
            (self.is_enabled(app), self.get(app).last_thumb_selection)
        {
            let current_values = self.discretize_range_values(app, self.values(app));
            if should_call_on_change_start
                && let Some(on_change_start) = self.get(app).on_change_start.clone()
            {
                on_change_start(app, current_values);
            }
            let current_drag_value = self.discretize(app, drag_value);

            let min_thumb_separation_value = self.min_thumb_separation_value(app);
            let new_values = match last_thumb_selection {
                Thumb::Start => RangeValues::new(
                    current_drag_value.min(current_values.end - min_thumb_separation_value),
                    current_values.end,
                ),
                Thumb::End => RangeValues::new(
                    current_values.start,
                    current_drag_value.max(current_values.start + min_thumb_separation_value),
                ),
            };
            self.get_mut(app).new_values = new_values;
            let on_changed = self.on_changed(app).expect("an enabled slider");
            let discrete_values = self.discretize_range_values(app, new_values);
            on_changed(app, discrete_values);
        }
    }

    fn end_interaction(self: RenderHandle<Self>, app: &mut App) {
        let state = self.get(app).state;
        if !state.mounted(app) {
            return;
        }

        if self.should_show_value_indicator_when_dragged(app)
            && app.get(state).interaction_timer.is_none()
        {
            state.value_indicator_controller(app).reverse(app, None);
        }

        if self.get(app).active
            && state.mounted(app)
            && self.get(app).last_thumb_selection.is_some()
        {
            let discrete_values = self.discretize_range_values(app, self.get(app).new_values);
            if let Some(on_change_end) = self.get(app).on_change_end.clone() {
                on_change_end(app, discrete_values);
            }
            self.get_mut(app).active = false;
        }
        state.overlay_controller(app).reverse(app, None);
    }

    fn handle_drag_start(self: RenderHandle<Self>, app: &mut App, details: DragStartDetails) {
        self.start_interaction(app, details.global_position);
    }

    fn handle_drag_end(self: RenderHandle<Self>, app: &mut App, _details: DragEndDetails) {
        self.end_interaction(app);
    }

    fn handle_tap_down(self: RenderHandle<Self>, app: &mut App, details: TapDownDetails) {
        self.start_interaction(app, details.global_position);
    }

    fn handle_tap_up(self: RenderHandle<Self>, app: &mut App, _details: TapUpDetails) {
        self.end_interaction(app);
    }

    fn semantic_action_unit(self: RenderHandle<Self>, app: &App) -> f64 {
        match self.divisions(app) {
            Some(divisions) => 1.0 / f64::from(divisions),
            None => self.adjustment_unit(app),
        }
    }

    /// Dart's `increaseStartAction`, which the arrow keys run.
    fn increase_start_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_enabled(app) {
            let values = RangeValues::new(self.increased_start_value(app), self.values(app).end);
            let on_changed = self.on_changed(app).expect("an enabled slider");
            on_changed(app, values);
        }
    }

    /// Dart's `decreaseStartAction`, which the arrow keys run.
    fn decrease_start_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_enabled(app) {
            let values = RangeValues::new(self.decreased_start_value(app), self.values(app).end);
            let on_changed = self.on_changed(app).expect("an enabled slider");
            on_changed(app, values);
        }
    }

    /// Dart's `increaseEndAction`, which the arrow keys run.
    fn increase_end_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_enabled(app) {
            let values = RangeValues::new(self.values(app).start, self.increased_end_value(app));
            let on_changed = self.on_changed(app).expect("an enabled slider");
            on_changed(app, values);
        }
    }

    /// Dart's `decreaseEndAction`, which the arrow keys run.
    fn decrease_end_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_enabled(app) {
            let values = RangeValues::new(self.values(app).start, self.decreased_end_value(app));
            let on_changed = self.on_changed(app).expect("an enabled slider");
            on_changed(app, values);
        }
    }

    fn increased_start_value(self: RenderHandle<Self>, app: &App) -> f64 {
        let values = self.values(app);
        // Due to floating-point operations, this value can actually be greater than
        // expected (e.g. 0.4 + 0.2 = 0.600000000001), so we limit to 2 decimal points.
        let increased_start_value = format!("{:.2}", values.start + self.semantic_action_unit(app))
            .parse::<f64>()
            .expect("a formatted number parses");
        if increased_start_value <= values.end - self.min_thumb_separation_value(app) {
            increased_start_value
        } else {
            values.start
        }
    }

    fn decreased_start_value(self: RenderHandle<Self>, app: &App) -> f64 {
        clamp_double(
            self.values(app).start - self.semantic_action_unit(app),
            0.0,
            1.0,
        )
    }

    fn increased_end_value(self: RenderHandle<Self>, app: &App) -> f64 {
        clamp_double(
            self.values(app).end + self.semantic_action_unit(app),
            0.0,
            1.0,
        )
    }

    fn decreased_end_value(self: RenderHandle<Self>, app: &App) -> f64 {
        let values = self.values(app);
        let decreased_end_value = values.end - self.semantic_action_unit(app);
        if decreased_end_value >= values.start + self.min_thumb_separation_value(app) {
            decreased_end_value
        } else {
            values.end
        }
    }
}

/// Dart's `addListener(markNeedsPaint)` on the range slider's animations and focus nodes.
fn mark_range_slider_needs_paint(this: Handle<RenderRangeSlider>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

impl RelayoutWhenSystemFontsChangeMixin for RenderRangeSlider {
    fn relayout_when_system_fonts_change_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RelayoutWhenSystemFontsChangeData {
        &self.get(app).system_fonts
    }

    fn relayout_when_system_fonts_change_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RelayoutWhenSystemFontsChangeData {
        &mut self.get_mut(app).system_fonts
    }

    fn system_fonts_did_change(self: RenderHandle<Self>, app: &mut App) {
        self.as_render_object(app).mark_needs_layout(app);
        self.get_mut(app).start_label_painter.mark_needs_layout();
        self.get_mut(app).end_label_painter.mark_needs_layout();
        self.update_label_painters(app);
    }
}

impl RenderObject for RenderRangeSlider {
    waft_rendering::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    /// `RenderBox.performLayout`: a box sized by its parent lays out nothing more.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(self.sized_by_parent(app));
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, _owner: Handle<PipelineOwner>) {
        RelayoutWhenSystemFontsChangeMixin::attach_system_fonts(self, app);
        let listener = Listener::handle_method(self.handle(), mark_range_slider_needs_paint);
        let this = self.get(app);
        let (overlay, value_indicator, enable, state) = (
            this.overlay_animation.clone(),
            this.value_indicator_animation.clone(),
            this.enable_animation.clone(),
            this.state,
        );
        overlay.add_listener(app, listener.clone());
        value_indicator.add_listener(app, listener.clone());
        enable.add_listener(app, listener.clone());
        state
            .start_position_controller(app)
            .add_listener(app, listener.clone());
        state
            .end_position_controller(app)
            .add_listener(app, listener.clone());
        state
            .start_focus_node(app)
            .add_listener(app, listener.clone());
        state.end_focus_node(app).add_listener(app, listener);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let listener = Listener::handle_method(self.handle(), mark_range_slider_needs_paint);
        let this = self.get(app);
        let (overlay, value_indicator, enable, state) = (
            this.overlay_animation.clone(),
            this.value_indicator_animation.clone(),
            this.enable_animation.clone(),
            this.state,
        );
        overlay.remove_listener(app, &listener);
        value_indicator.remove_listener(app, &listener);
        enable.remove_listener(app, &listener);
        state
            .start_position_controller(app)
            .remove_listener(app, &listener);
        state
            .end_position_controller(app)
            .remove_listener(app, &listener);
        state.start_focus_node(app).remove_listener(app, &listener);
        state.end_focus_node(app).remove_listener(app, &listener);
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        let (drag, tap) = (self.get(app).drag.handle(), self.get(app).tap.handle());
        RecognizerLeaf::dispose(drag, app);
        RecognizerLeaf::dispose(tap, app);
        self.get_mut(app).start_label_painter.dispose();
        self.get_mut(app).end_label_painter.dispose();
        let this = self.get(app);
        let (enable, value_indicator, overlay) = (
            this.enable_animation.handle(),
            this.value_indicator_animation.handle(),
            this.overlay_animation.handle(),
        );
        for animation in [enable, value_indicator, overlay] {
            animation.dispose(app);
        }
        RenderObjectBase::dispose(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let state = self.get(app).state;
        let start_value = state.start_position_controller(app).value(app);
        let end_value = state.end_position_controller(app).value(app);
        let text_direction = self.text_direction(app);

        // The visual position is the position of the thumb from 0 to 1 from left
        // to right. In left to right, this is the same as the value, but it is
        // reversed for right to left text.
        let (start_visual_position, end_visual_position) = match text_direction {
            TextDirection::Rtl => (1.0 - start_value, 1.0 - end_value),
            TextDirection::Ltr => (start_value, end_value),
        };

        let is_discrete = self.is_discrete(app);
        let is_enabled = self.is_enabled(app);
        let slider_theme = self.slider_theme(app).clone();
        let range_track_shape = self.range_track_shape(app);
        let track_rect = range_track_shape.get_preferred_rect(
            app,
            self.as_box(),
            offset,
            &slider_theme,
            None,
            Some(is_discrete),
        );
        let padding = if range_track_shape.is_rounded() {
            track_rect.height()
        } else {
            0.0
        };
        let thumb_y_offset = track_rect.center().dy();
        let thumb_position = |visual_position: f64| {
            if is_discrete {
                track_rect.left + visual_position * (track_rect.width() - padding) + padding / 2.0
            } else {
                track_rect.left + visual_position * track_rect.width()
            }
        };
        let start_thumb_position = thumb_position(start_visual_position);
        let end_thumb_position = thumb_position(end_visual_position);
        let thumb_preferred_size = self
            .range_thumb_shape(app)
            .get_preferred_size(is_enabled, is_discrete);
        let thumb_padding = if padding > thumb_preferred_size.width() / 2.0 {
            padding / 2.0
        } else {
            0.0
        };
        let start_thumb_center = Offset::new(
            clamp_double(
                start_thumb_position,
                track_rect.left + thumb_padding,
                track_rect.right - thumb_padding,
            ),
            thumb_y_offset,
        );
        let end_thumb_center = Offset::new(
            clamp_double(
                end_thumb_position,
                track_rect.left + thumb_padding,
                track_rect.right - thumb_padding,
            ),
            thumb_y_offset,
        );
        {
            let this = self.get_mut(app);
            this.start_thumb_center = start_thumb_center;
            this.end_thumb_center = end_thumb_center;
        }
        if is_enabled {
            let overlay_size = self
                .overlay_shape(app)
                .get_preferred_size(is_enabled, false);
            let this = self.get_mut(app);
            this.overlay_start_rect = Some(Rect::from_circle(
                start_thumb_center,
                overlay_size.width() / 2.0,
            ));
            this.overlay_end_rect = Some(Rect::from_circle(
                end_thumb_center,
                overlay_size.width() / 2.0,
            ));
        }

        // If [RangeSlider.year2023] is false, the thumbs uses handle thumb shape and gapped track shape.
        // The handle width and track gaps are adjusted when the thumb is pressed.
        let no_states = WidgetStates::new();
        let mut thumb_width = slider_theme
            .thumb_size
            .as_ref()
            .and_then(|thumb_size| thumb_size.resolve(&no_states))
            .map(|size| size.width());
        let thumb_height = slider_theme
            .thumb_size
            .as_ref()
            .and_then(|thumb_size| thumb_size.resolve(&no_states))
            .map(|size| size.height());
        let mut track_gap = slider_theme.track_gap;
        let pressed_thumb_width = slider_theme
            .thumb_size
            .as_ref()
            .and_then(|thumb_size| thumb_size.resolve(&WidgetStates::from([WidgetState::Pressed])))
            .map(|size| size.width());
        if let (true, Some(width), Some(pressed_width), Some(gap)) = (
            self.get(app).active,
            thumb_width,
            pressed_thumb_width,
            track_gap,
        ) {
            let delta = width - pressed_width;
            thumb_width = Some(pressed_width);
            if gap > 0.0 {
                track_gap = Some(gap - delta / 2.0);
            }
        }

        let enable_animation = self.get(app).enable_animation.as_animation();
        let overlay_animation = self.get(app).overlay_animation.as_animation();
        let value_indicator_animation = self.get(app).value_indicator_animation.as_animation();
        let mut track_theme = slider_theme.copy_with();
        if let Some(track_gap) = track_gap {
            track_theme.track_gap = Some(track_gap);
        }
        range_track_shape.paint(
            app,
            context,
            offset,
            self.as_box(),
            &track_theme,
            enable_animation,
            start_thumb_center,
            end_thumb_center,
            is_enabled,
            is_discrete,
            text_direction,
        );

        let last_thumb_selection = self.get(app).last_thumb_selection;
        let start_thumb_selected =
            last_thumb_selection == Some(Thumb::Start) && !self.hovering_end_thumb(app);
        let end_thumb_selected =
            last_thumb_selection == Some(Thumb::End) && !self.hovering_start_thumb(app);
        let screen_size = self.screen_size(app);
        let resolved_screen_size = if screen_size.is_empty() {
            self.size(app)
        } else {
            screen_size
        };
        let text_scale_factor = self.text_scale_factor(app);
        let overlay_shape = self.overlay_shape(app);

        let paint_overlay = |this: RenderHandle<Self>,
                             app: &mut App,
                             context: &mut PaintingContext,
                             thumb: Thumb,
                             center: Offset,
                             activation_animation: AnyAnimation<f64>,
                             value: f64| {
            this.with_label_painter(app, thumb, |app, label_painter| {
                overlay_shape.paint(
                    app,
                    context,
                    center,
                    activation_animation,
                    enable_animation,
                    is_discrete,
                    label_painter,
                    this.as_box(),
                    &slider_theme,
                    text_direction,
                    value,
                    text_scale_factor,
                    resolved_screen_size,
                );
            });
        };

        if self.start_thumb_show_focus_highlight(app) {
            let always = always_stopped_at_one(app);
            paint_overlay(
                self,
                app,
                context,
                Thumb::Start,
                start_thumb_center,
                always,
                start_value,
            );
        }

        if self.end_thumb_show_focus_highlight(app) {
            let always = always_stopped_at_one(app);
            paint_overlay(
                self,
                app,
                context,
                Thumb::End,
                end_thumb_center,
                always,
                end_value,
            );
        }

        if !overlay_animation.is_dismissed(app) {
            if start_thumb_selected || self.hovering_start_thumb(app) {
                paint_overlay(
                    self,
                    app,
                    context,
                    Thumb::Start,
                    start_thumb_center,
                    overlay_animation,
                    start_value,
                );
            }
            if end_thumb_selected || self.hovering_end_thumb(app) {
                paint_overlay(
                    self,
                    app,
                    context,
                    Thumb::End,
                    end_thumb_center,
                    overlay_animation,
                    end_value,
                );
            }
        }

        if is_discrete {
            let divisions = self.divisions(app).expect("a discrete slider");
            let range_tick_mark_shape = self.range_tick_mark_shape(app);
            let tick_mark_width = range_tick_mark_shape
                .get_preferred_size(&slider_theme, Some(is_enabled))
                .width();
            let discrete_track_padding = track_rect.height();
            let adjusted_track_width = track_rect.width() - discrete_track_padding;
            // If the tick marks would be too dense, don't bother painting them.
            if adjusted_track_width / f64::from(divisions) >= 3.0 * tick_mark_width {
                let dy = track_rect.center().dy();
                for i in 0..=divisions {
                    let value = f64::from(i) / f64::from(divisions);
                    // The ticks are mapped to be within the track, so the tick mark width
                    // must be subtracted from the track width.
                    let dx = track_rect.left
                        + value * adjusted_track_width
                        + discrete_track_padding / 2.0;
                    let tick_mark_offset = Offset::new(dx, dy);
                    range_tick_mark_shape.paint(
                        app,
                        context,
                        tick_mark_offset,
                        self.as_box(),
                        &slider_theme,
                        enable_animation,
                        start_thumb_center,
                        end_thumb_center,
                        Some(is_enabled),
                        text_direction,
                    );
                }
            }
        }

        let thumb_delta = (end_thumb_center.dx() - start_thumb_center.dx()).abs();

        let is_last_thumb_start = last_thumb_selection == Some(Thumb::Start);
        let (bottom_thumb, top_thumb) = if is_last_thumb_start {
            (Thumb::End, Thumb::Start)
        } else {
            (Thumb::Start, Thumb::End)
        };
        let (bottom_thumb_center, top_thumb_center) = if is_last_thumb_start {
            (end_thumb_center, start_thumb_center)
        } else {
            (start_thumb_center, end_thumb_center)
        };
        let (bottom_value, top_value) = if is_last_thumb_start {
            (end_value, start_value)
        } else {
            (start_value, end_value)
        };
        let should_paint_value_indicators = is_enabled
            && self.labels(app).is_some()
            && ((self.should_show_value_indicator_when_dragged(app)
                && !value_indicator_animation.is_dismissed(app))
                || self.should_always_show_value_indicator(app));

        if should_paint_value_indicators {
            let paint_bottom: PaintRangeValueIndicator = Rc::new(
                move |app: &mut App, context: &mut PaintingContext, _offset: Offset| {
                    self.paint_value_indicator(
                        app,
                        context,
                        bottom_thumb_center,
                        false,
                        bottom_thumb,
                        bottom_value,
                    );
                },
            );
            app.get_mut(state).paint_bottom_value_indicator = Some(paint_bottom);
        }

        let range_thumb_shape = self.range_thumb_shape(app);
        let thumb_theme = match (thumb_width, thumb_height) {
            (Some(thumb_width), Some(thumb_height)) => {
                let mut thumb_theme = slider_theme.copy_with();
                thumb_theme.thumb_size = Some(Rc::new(WidgetStatePropertyAll::new(Some(
                    Size::new(thumb_width, thumb_height),
                ))));
                thumb_theme
            }
            _ => slider_theme.clone(),
        };
        range_thumb_shape.paint(
            app,
            context,
            bottom_thumb_center,
            value_indicator_animation,
            enable_animation,
            Some(is_discrete),
            None,
            Some(false),
            Some(text_direction),
            &thumb_theme,
            Some(bottom_thumb),
            Some(if bottom_thumb == Thumb::Start {
                start_thumb_selected
            } else {
                end_thumb_selected
            }),
        );

        if should_paint_value_indicators {
            let range_value_indicator_shape = self.range_value_indicator_shape(app);
            let start_offset = self.with_label_painter(app, Thumb::Start, |app, label_painter| {
                range_value_indicator_shape.get_horizontal_shift(
                    app,
                    Some(self.as_box()),
                    Some(start_thumb_center),
                    Some(label_painter),
                    Some(value_indicator_animation),
                    Some(text_scale_factor),
                    Some(resolved_screen_size),
                )
            });
            let end_offset = self.with_label_painter(app, Thumb::End, |app, label_painter| {
                range_value_indicator_shape.get_horizontal_shift(
                    app,
                    Some(self.as_box()),
                    Some(end_thumb_center),
                    Some(label_painter),
                    Some(value_indicator_animation),
                    Some(text_scale_factor),
                    Some(resolved_screen_size),
                )
            });
            let is_enabled_now = self.is_enabled(app);
            let start_half_width = range_value_indicator_shape
                .get_preferred_size(
                    is_enabled_now,
                    is_discrete,
                    &self.get(app).start_label_painter,
                    text_scale_factor,
                )
                .width()
                / 2.0;
            let end_half_width = range_value_indicator_shape
                .get_preferred_size(
                    is_enabled_now,
                    is_discrete,
                    &self.get(app).end_label_painter,
                    text_scale_factor,
                )
                .width()
                / 2.0;
            let inner_overflow = start_half_width
                + end_half_width
                + match text_direction {
                    TextDirection::Ltr => start_offset - end_offset,
                    TextDirection::Rtl => end_offset - start_offset,
                };

            let paint_top: PaintRangeValueIndicator = Rc::new(
                move |app: &mut App, context: &mut PaintingContext, _offset: Offset| {
                    self.paint_value_indicator(
                        app,
                        context,
                        top_thumb_center,
                        thumb_delta < inner_overflow,
                        top_thumb,
                        top_value,
                    );
                },
            );
            app.get_mut(state).paint_top_value_indicator = Some(paint_top);
        }

        range_thumb_shape.paint(
            app,
            context,
            top_thumb_center,
            overlay_animation,
            enable_animation,
            Some(is_discrete),
            None,
            Some(
                thumb_delta
                    < self
                        .range_thumb_shape(app)
                        .get_preferred_size(self.is_enabled(app), is_discrete)
                        .width(),
            ),
            Some(text_direction),
            &thumb_theme,
            Some(top_thumb),
            Some(if top_thumb == Thumb::Start {
                start_thumb_selected
            } else {
                end_thumb_selected
            }),
        );
    }
}

impl RenderRangeSlider {
    /// The body of Dart's `paintTopValueIndicator` and `paintBottomValueIndicator` closures,
    /// which read the render object's state when the overlay's value indicator calls them.
    fn paint_value_indicator(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        is_on_top: bool,
        thumb: Thumb,
        value: f64,
    ) {
        if !self.as_render_object(app).attached(app) {
            return;
        }
        let always = self.should_always_show_value_indicator(app);
        let (activation_animation, enable_animation) = if always {
            (always_stopped_at_one(app), always_stopped_at_one(app))
        } else {
            let this = self.get(app);
            (
                this.value_indicator_animation.as_animation(),
                this.enable_animation.as_animation(),
            )
        };
        let range_value_indicator_shape = self.range_value_indicator_shape(app);
        let slider_theme = self.slider_theme(app).clone();
        let (is_discrete, text_direction, text_scale_factor) = (
            self.is_discrete(app),
            self.text_direction(app),
            self.text_scale_factor(app),
        );
        let screen_size = self.screen_size(app);
        let resolved_screen_size = if screen_size.is_empty() {
            self.size(app)
        } else {
            screen_size
        };
        self.with_label_painter(app, thumb, |app, label_painter| {
            range_value_indicator_shape.paint(
                app,
                context,
                center,
                activation_animation,
                enable_animation,
                Some(is_discrete),
                Some(is_on_top),
                label_painter,
                Some(text_scale_factor),
                Some(resolved_screen_size),
                self.as_box(),
                &slider_theme,
                Some(text_direction),
                Some(value),
                Some(thumb),
            );
        });
    }
}

impl RenderBox for RenderRangeSlider {
    waft_rendering::render_box_accessors!();

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }

    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        _entry: &BoxHitTestEntry,
    ) {
        if let PointerEvent::Down(down) = event
            && self.is_enabled(app)
        {
            // We need to add the drag first so that it has priority.
            let (drag, tap) = (self.get(app).drag.handle(), self.get(app).tap.handle());
            drag.add_pointer(app, down.clone());
            tap.add_pointer(app, down.clone());
        }
        if self.is_enabled(app) {
            if let Some(overlay_start_rect) = self.get(app).overlay_start_rect {
                self.set_hovering_start_thumb(
                    app,
                    overlay_start_rect.contains(event.local_position()),
                );
            }
            if let Some(overlay_end_rect) = self.get(app).overlay_end_rect {
                self.set_hovering_end_thumb(app, overlay_end_rect.contains(event.local_position()));
            }
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        Self::MIN_PREFERRED_TRACK_WIDTH + self.max_slider_part_width(app)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        Self::MIN_PREFERRED_TRACK_WIDTH + self.max_slider_part_width(app)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, _width: f64) -> f64 {
        self.min_preferred_track_height(app)
            .max(self.max_slider_part_height(app))
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, _width: f64) -> f64 {
        self.min_preferred_track_height(app)
            .max(self.max_slider_part_height(app))
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        Size::new(
            if constraints.has_bounded_width() {
                constraints.max_width
            } else {
                Self::MIN_PREFERRED_TRACK_WIDTH + self.max_slider_part_width(app)
            },
            if constraints.has_bounded_height() {
                constraints.max_height
            } else {
                self.min_preferred_track_height(app)
                    .max(self.max_slider_part_height(app))
            },
        )
    }
}

/// Dart's `_ValueIndicatorRenderObjectWidget`.
#[derive(Debug)]
struct ValueIndicatorRenderObjectWidget {
    state: Handle<RangeSliderState>,
}

impl RenderObjectWidget for ValueIndicatorRenderObjectWidget {
    type RenderObject = RenderValueIndicator;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderValueIndicator::new(app, self.state).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderValueIndicator>,
    ) {
        render_object.get_mut(app).state = self.state;
    }
}

impl LeafRenderObjectWidget for ValueIndicatorRenderObjectWidget {}

/// Dart's `_RenderValueIndicator extends RenderBox with RelayoutWhenSystemFontsChangeMixin`.
struct RenderValueIndicator {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    system_fonts: RelayoutWhenSystemFontsChangeData,
    value_indicator_animation: Ref<CurvedAnimation>,
    state: Handle<RangeSliderState>,
}

impl RenderValueIndicator {
    fn new(app: &mut App, state: Handle<RangeSliderState>) -> RenderHandle<RenderValueIndicator> {
        let value_indicator_animation = CurvedAnimation::new(
            app,
            state.value_indicator_controller_ref(app),
            Curves::fast_out_slow_in(),
            None,
        );
        RenderHandle::new_box(
            app,
            RenderValueIndicator {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                system_fonts: RelayoutWhenSystemFontsChangeData::new(),
                value_indicator_animation,
                state,
            },
        )
    }
}

/// Dart's `addListener(markNeedsPaint)` on the value indicator's animations.
fn mark_value_indicator_needs_paint(this: Handle<RenderValueIndicator>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

impl RelayoutWhenSystemFontsChangeMixin for RenderValueIndicator {
    fn relayout_when_system_fonts_change_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RelayoutWhenSystemFontsChangeData {
        &self.get(app).system_fonts
    }

    fn relayout_when_system_fonts_change_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RelayoutWhenSystemFontsChangeData {
        &mut self.get_mut(app).system_fonts
    }
}

impl RenderObject for RenderValueIndicator {
    waft_rendering::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    /// `RenderBox.performLayout`: a box sized by its parent lays out nothing more.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(self.sized_by_parent(app));
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, _owner: Handle<PipelineOwner>) {
        RelayoutWhenSystemFontsChangeMixin::attach_system_fonts(self, app);
        let listener = Listener::handle_method(self.handle(), mark_value_indicator_needs_paint);
        let animation = self.get(app).value_indicator_animation.handle();
        let state = self.get(app).state;
        animation.add_listener(app, listener.clone());
        state
            .start_position_controller(app)
            .add_listener(app, listener.clone());
        state
            .end_position_controller(app)
            .add_listener(app, listener);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let listener = Listener::handle_method(self.handle(), mark_value_indicator_needs_paint);
        let animation = self.get(app).value_indicator_animation.handle();
        let state = self.get(app).state;
        animation.remove_listener(app, &listener);
        state
            .start_position_controller(app)
            .remove_listener(app, &listener);
        state
            .end_position_controller(app)
            .remove_listener(app, &listener);
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let state = self.get(app).state;
        if let Some(paint_bottom_value_indicator) =
            app.get(state).paint_bottom_value_indicator.clone()
        {
            paint_bottom_value_indicator(app, context, offset);
        }
        if let Some(paint_top_value_indicator) = app.get(state).paint_top_value_indicator.clone() {
            paint_top_value_indicator(app, context, offset);
        }
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        let value_indicator_animation = self.get(app).value_indicator_animation.handle();
        value_indicator_animation.dispose(app);
        RenderObjectBase::dispose(self, app);
    }
}

impl RenderBox for RenderValueIndicator {
    waft_rendering::render_box_accessors!();

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.smallest()
    }
}

/// Dart's `_RangeSliderDefaultsM2`.
struct RangeSliderDefaultsM2 {
    /// Dart's `super(..)` arguments.
    slider_theme_data: SliderThemeData,
    context: DefaultsContext,
}

impl RangeSliderDefaultsM2 {
    fn new(context: BuildContext) -> RangeSliderDefaultsM2 {
        RangeSliderDefaultsM2 {
            slider_theme_data: SliderThemeData::new().track_height(4.0),
            context: DefaultsContext::new(context),
        }
    }
}

#[allow(deprecated)]
impl SliderThemeDataGetters for RangeSliderDefaultsM2 {
    fn slider_theme_data(&self) -> &SliderThemeData {
        &self.slider_theme_data
    }

    fn active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.24))
    }

    fn disabled_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.32))
    }

    fn disabled_inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.12))
    }

    fn active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary.with_opacity(0.54))
    }

    fn inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.54))
    }

    fn disabled_active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary.with_opacity(0.12))
    }

    fn disabled_inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.12))
    }

    fn thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn overlapping_shape_stroke_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.surface)
    }

    fn disabled_thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(Color::alpha_blend(
            colors.on_surface.with_opacity(0.38),
            colors.surface,
        ))
    }

    fn overlay_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.12).into())
    }

    fn value_indicator_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            theme
                .text_theme
                .body_large
                .as_ref()
                .expect("the text theme has a body large style")
                .copy_with()
                .color(colors.on_primary),
        )
    }

    fn value_indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn range_track_shape(&self, _app: &mut App) -> Option<RangeSliderTrackShapeRef> {
        Some(Rc::new(RoundedRectRangeSliderTrackShape::new()))
    }

    fn range_tick_mark_shape(&self, _app: &mut App) -> Option<RangeSliderTickMarkShapeRef> {
        Some(Rc::new(RoundRangeSliderTickMarkShape::new()))
    }

    fn range_thumb_shape(&self, _app: &mut App) -> Option<RangeSliderThumbShapeRef> {
        Some(Rc::new(RoundRangeSliderThumbShape::new()))
    }

    fn overlay_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderOverlayShape::new()))
    }

    fn range_value_indicator_shape(
        &self,
        _app: &mut App,
    ) -> Option<RangeSliderValueIndicatorShapeRef> {
        Some(Rc::new(RectangularRangeSliderValueIndicatorShape::new()))
    }

    fn show_value_indicator(&self, _app: &mut App) -> Option<ShowValueIndicator> {
        Some(ShowValueIndicator::OnlyForDiscrete)
    }

    fn min_thumb_separation(&self, _app: &mut App) -> Option<f64> {
        Some(8.0)
    }
}

/// Dart's `_AdjustSliderIntent`.
#[derive(Clone, Copy, Debug)]
struct AdjustSliderIntent {
    adjustment_type: SliderAdjustmentType,
}

impl AdjustSliderIntent {
    const fn new(adjustment_type: SliderAdjustmentType) -> AdjustSliderIntent {
        AdjustSliderIntent { adjustment_type }
    }
}

impl Intent for AdjustSliderIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderAdjustmentType {
    Right,
    Left,
    Up,
    Down,
}

// Toggles the value adjustment mode used by directional navigation, so the
// thumb that has focus enters or exits the editing state.
/// Dart's `_ToggleRangeSliderEditModeIntent`.
#[derive(Clone, Copy, Debug)]
struct ToggleRangeSliderEditModeIntent;

impl Intent for ToggleRangeSliderEditModeIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/range_slider_test.dart` and the `RangeSlider` cases of
    //! `test/slider_theme_test.dart`; the goldens and the semantics-tree cases are not ported.

    use std::cell::{Cell, RefCell};

    use waft_widgets::{Center, StatefulBuilder};

    use super::*;
    use crate::material::Material;
    use crate::range_slider_parts::{RangeSliderTickMarkShape, RangeSliderValueIndicatorShape};
    use crate::slider::tests::{default_theme, tester_on};
    use crate::test_support::{
        CanvasCall, PaintCall, WidgetTester, colors_match, count_calls, material_app, paints_calls,
        with_main_thread_stack,
    };
    use crate::theme_data::ThemeData;

    /// What a test's range slider callbacks heard.
    struct Heard {
        values: Cell<RangeValues>,
        start_values: Cell<Option<RangeValues>>,
        end_values: Cell<Option<RangeValues>>,
        updates: Cell<usize>,
    }

    impl Heard {
        fn starting_at(start: f64, end: f64) -> Rc<Heard> {
            Rc::new(Heard {
                values: Cell::new(RangeValues::new(start, end)),
                start_values: Cell::new(None),
                end_values: Cell::new(None),
                updates: Cell::new(0),
            })
        }
    }

    /// A range slider whose `values` and callbacks are wired to `heard`, `configure`d.
    fn range_slider(
        heard: &Rc<Heard>,
        set_state: waft_widgets::StateSetter,
        configure: &dyn Fn(RangeSlider) -> RangeSlider,
    ) -> RangeSlider {
        let on_changed_heard = Rc::clone(heard);
        let start_heard = Rc::clone(heard);
        let end_heard = Rc::clone(heard);
        configure(
            RangeSlider::new(
                heard.values.get(),
                Some(Rc::new(move |app: &mut App, new_values: RangeValues| {
                    let heard = Rc::clone(&on_changed_heard);
                    set_state(app, &mut |_app| {
                        heard.updates.set(heard.updates.get() + 1);
                        heard.values.set(new_values);
                    });
                })),
            )
            .on_change_start(move |_app, values| start_heard.start_values.set(Some(values)))
            .on_change_end(move |_app, values| end_heard.end_values.set(Some(values))),
        )
    }

    /// Dart's `MaterialApp(home: Directionality(child: StatefulBuilder(... Material(child:
    /// Center(child: wrap(RangeSlider(...)))))))`, the shape most cases pump.
    fn range_slider_app(
        theme: ThemeData,
        text_direction: TextDirection,
        heard: &Rc<Heard>,
        wrap: impl Fn(&mut App, BuildContext, RangeSlider) -> WidgetRef + 'static,
        configure: impl Fn(RangeSlider) -> RangeSlider + 'static,
    ) -> WidgetRef {
        let heard = Rc::clone(heard);
        material_app(
            theme,
            Directionality::new(
                text_direction,
                StatefulBuilder::new(move |app, context, set_state| {
                    let slider = range_slider(&heard, set_state, &configure);
                    Material::new()
                        .child(Center::new().child(wrap(app, context, slider)))
                        .into_widget()
                }),
            )
            .into_widget(),
        )
    }

    fn plain(_app: &mut App, _context: BuildContext, slider: RangeSlider) -> WidgetRef {
        slider.into_widget()
    }

    fn pump(
        tester: &mut WidgetTester,
        text_direction: TextDirection,
        heard: &Rc<Heard>,
        configure: impl Fn(RangeSlider) -> RangeSlider + 'static,
    ) {
        let theme = default_theme(tester);
        tester.pump_widget(range_slider_app(
            theme,
            text_direction,
            heard,
            plain,
            configure,
        ));
    }

    fn slider_rect(tester: &WidgetTester) -> Rect {
        let slider = tester.find_by_type::<RangeSlider>()[0];
        tester.global_rect(tester.render_box_of(slider))
    }

    /// The bounds of the track: the slider's edges translated inwards by the overlay radius.
    fn track_bounds(tester: &WidgetTester) -> (Offset, Offset) {
        let rect = slider_rect(tester);
        (
            rect.top_left() + Offset::new(24.0, 0.0),
            rect.bottom_right() + Offset::new(-24.0, 0.0),
        )
    }

    fn near(actual: f64, expected: f64, epsilon: f64) -> bool {
        (actual - expected).abs() <= epsilon
    }

    // 'Range Slider can move when tapped (continuous LTR)'.
    #[test]
    fn a_range_slider_moves_when_tapped_continuous_ltr() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.8);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| slider);

            // The closest thumb is selected when tapping between the thumbs outside the touch
            // boundaries
            assert_eq!(heard.values.get(), RangeValues::new(0.3, 0.8));
            //  taps at 0.5
            tester.tap(slider_rect(&tester).center(), 1);
            tester.pump(Duration::ZERO);
            assert_eq!(heard.values.get(), RangeValues::new(0.5, 0.8));

            // Get the bounds of the track by finding the slider edges and translating
            // inwards by the overlay radius.
            let (top_left, bottom_right) = track_bounds(&tester);

            // The start thumb is selected when tapping the left inactive track.
            let left_target = top_left + (bottom_right - top_left) * 0.1;
            tester.tap(left_target, 2);
            assert!(near(heard.values.get().start, 0.1, 0.01));
            assert_eq!(heard.values.get().end, 0.8);

            // The end thumb is selected when tapping the right inactive track.
            tester.pump(Duration::ZERO);
            let right_target = top_left + (bottom_right - top_left) * 0.9;
            tester.tap(right_target, 3);
            assert!(near(heard.values.get().start, 0.1, 0.01));
            assert!(near(heard.values.get().end, 0.9, 0.01));
        });
    }

    // 'Range Slider can move when tapped (continuous RTL)'.
    #[test]
    fn a_range_slider_moves_when_tapped_continuous_rtl() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 1.0);
            pump(&mut tester, TextDirection::Rtl, &heard, |slider| slider);

            // The closest thumb is selected when tapping between the thumbs outside the touch
            // boundaries
            assert_eq!(heard.values.get(), RangeValues::new(0.3, 1.0));
            // taps at 0.5
            tester.tap(slider_rect(&tester).center(), 1);
            tester.pump(Duration::ZERO);
            assert_eq!(heard.values.get(), RangeValues::new(0.5, 1.0));

            let (top_left, bottom_right) = track_bounds(&tester);

            // The end thumb is selected when tapping the left inactive track.
            let left_target = top_left + (bottom_right - top_left) * 0.1;
            tester.tap(left_target, 2);
            assert_eq!(heard.values.get().start, 0.5);
            assert!(near(heard.values.get().end, 0.9, 0.01));

            // The start thumb is selected when tapping the right inactive track.
            tester.pump(Duration::ZERO);
            let right_target = top_left + (bottom_right - top_left) * 0.9;
            tester.tap(right_target, 3);
            assert!(near(heard.values.get().start, 0.1, 0.01));
            assert!(near(heard.values.get().end, 0.9, 0.01));
        });
    }

    // 'Range Slider can move when tapped (discrete LTR)'.
    #[test]
    fn a_range_slider_moves_when_tapped_discrete_ltr() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(30.0, 80.0);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| {
                slider.max(100.0).divisions(10)
            });

            // The closest thumb is selected when tapping between the thumbs outside the touch
            // boundaries
            assert_eq!(heard.values.get(), RangeValues::new(30.0, 80.0));
            // taps at 0.5
            tester.tap(slider_rect(&tester).center(), 1);
            tester.pump_and_settle();
            assert_eq!(heard.values.get(), RangeValues::new(50.0, 80.0));

            let (top_left, bottom_right) = track_bounds(&tester);

            // The start thumb is selected when tapping the left inactive track.
            let left_target = top_left + (bottom_right - top_left) * 0.1;
            tester.tap(left_target, 2);
            tester.pump_and_settle();
            assert_eq!(heard.values.get().start.round(), 10.0);
            assert_eq!(heard.values.get().end.round(), 80.0);

            // The end thumb is selected when tapping the right inactive track.
            tester.pump(Duration::ZERO);
            let right_target = top_left + (bottom_right - top_left) * 0.9;
            tester.tap(right_target, 3);
            tester.pump_and_settle();
            assert_eq!(heard.values.get().start.round(), 10.0);
            assert_eq!(heard.values.get().end.round(), 90.0);
        });
    }

    // 'Range Slider can move when tapped (discrete RTL)'.
    #[test]
    fn a_range_slider_moves_when_tapped_discrete_rtl() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(30.0, 80.0);
            pump(&mut tester, TextDirection::Rtl, &heard, |slider| {
                slider.max(100.0).divisions(10)
            });

            // The closest thumb is selected when tapping between the thumbs outside the touch
            // boundaries
            assert_eq!(heard.values.get(), RangeValues::new(30.0, 80.0));
            // taps at 0.5
            tester.tap(slider_rect(&tester).center(), 1);
            tester.pump_and_settle();
            assert_eq!(heard.values.get(), RangeValues::new(50.0, 80.0));

            let (top_left, bottom_right) = track_bounds(&tester);

            // The start thumb is selected when tapping the left inactive track.
            let left_target = top_left + (bottom_right - top_left) * 0.1;
            tester.tap(left_target, 2);
            tester.pump_and_settle();
            assert_eq!(heard.values.get().start.round(), 50.0);
            assert_eq!(heard.values.get().end.round(), 90.0);

            // The end thumb is selected when tapping the right inactive track.
            tester.pump(Duration::ZERO);
            let right_target = top_left + (bottom_right - top_left) * 0.9;
            tester.tap(right_target, 3);
            tester.pump_and_settle();
            assert_eq!(heard.values.get().start.round(), 10.0);
            assert_eq!(heard.values.get().end.round(), 90.0);
        });
    }

    /// The four 'thumbs can be dragged to the min and max' cases: `max` is 1.0 or 100 with
    /// divisions; under RTL the thumbs swap.
    fn thumbs_are_dragged_to_the_min_and_max(
        text_direction: TextDirection,
        start: f64,
        end: f64,
        max: f64,
        divisions: Option<i32>,
    ) {
        let mut tester = tester_on(TargetPlatform::Android);
        let heard = Heard::starting_at(start, end);
        pump(&mut tester, text_direction, &heard, move |slider| {
            let slider = slider.max(max);
            match divisions {
                Some(divisions) => slider.divisions(divisions),
                None => slider,
            }
        });

        // Get the bounds of the track by finding the slider edges and translating
        // inwards by the overlay radius.
        let (top_left, bottom_right) = track_bounds(&tester);
        let span = bottom_right - top_left;
        let (first_target, first_expected, second_target, second_expected) = match text_direction {
            TextDirection::Ltr => (0.3, (false, 0.0), 0.7, (true, max)),
            TextDirection::Rtl => (0.3, (true, max), 0.7, (false, 0.0)),
        };
        let read = |heard: &Heard, end: bool| {
            if end {
                heard.values.get().end
            } else {
                heard.values.get().start
            }
        };
        // Drag the thumb under the left target to the left edge, and the other to the right.
        let left_target = top_left + span * first_target;
        tester.drag_from(left_target, span * -0.4, 1);
        assert_eq!(read(&heard, first_expected.0), first_expected.1);

        tester.pump_and_settle();
        let right_target = top_left + span * second_target;
        tester.drag_from(right_target, span * 0.4, 2);
        assert_eq!(read(&heard, second_expected.0), second_expected.1);
    }

    // 'Range Slider thumbs can be dragged to the min and max (continuous LTR)'.
    #[test]
    fn range_slider_thumbs_can_be_dragged_to_the_min_and_max_continuous_ltr() {
        with_main_thread_stack(|| {
            thumbs_are_dragged_to_the_min_and_max(TextDirection::Ltr, 0.3, 0.7, 1.0, None);
        });
    }

    // 'Range Slider thumbs can be dragged to the min and max (continuous RTL)'.
    #[test]
    fn range_slider_thumbs_can_be_dragged_to_the_min_and_max_continuous_rtl() {
        with_main_thread_stack(|| {
            thumbs_are_dragged_to_the_min_and_max(TextDirection::Rtl, 0.3, 0.7, 1.0, None);
        });
    }

    // 'Range Slider thumbs can be dragged to the min and max (discrete LTR)'.
    #[test]
    fn range_slider_thumbs_can_be_dragged_to_the_min_and_max_discrete_ltr() {
        with_main_thread_stack(|| {
            thumbs_are_dragged_to_the_min_and_max(TextDirection::Ltr, 30.0, 70.0, 100.0, Some(10));
        });
    }

    // 'Range Slider thumbs can be dragged to the min and max (discrete RTL)'.
    #[test]
    fn range_slider_thumbs_can_be_dragged_to_the_min_and_max_discrete_rtl() {
        with_main_thread_stack(|| {
            thumbs_are_dragged_to_the_min_and_max(TextDirection::Rtl, 30.0, 70.0, 100.0, Some(10));
        });
    }

    // 'minThumbSeparation has same width as surrounding box, values still bounded (ltr / rtl)'.
    fn min_thumb_separation_keeps_values_bounded(text_direction: TextDirection) {
        const BOUNDING_BOX_SIZE: f64 = 200.0;
        let mut tester = tester_on(TargetPlatform::Android);
        let heard = Heard::starting_at(0.0, 1.0);
        let theme = default_theme(&tester);
        tester.pump_widget(range_slider_app(
            theme,
            text_direction,
            &heard,
            |app, context, slider| {
                let data = SliderTheme::of(app, context)
                    .copy_with()
                    .min_thumb_separation(BOUNDING_BOX_SIZE);
                waft_widgets::SizedBox::new()
                    .width(BOUNDING_BOX_SIZE)
                    .child(SliderTheme::new(data, slider))
                    .into_widget()
            },
            |slider| slider,
        ));

        tester.drag_from(slider_rect(&tester).center(), Offset::ZERO, 1);
        tester.pump_and_settle();

        let values = heard.values.get();
        assert!((0.0..=1.0).contains(&values.start));
        assert!((0.0..=1.0).contains(&values.end));
    }

    #[test]
    fn min_thumb_separation_as_wide_as_the_box_keeps_values_bounded_ltr() {
        with_main_thread_stack(|| min_thumb_separation_keeps_values_bounded(TextDirection::Ltr));
    }

    #[test]
    fn min_thumb_separation_as_wide_as_the_box_keeps_values_bounded_rtl() {
        with_main_thread_stack(|| min_thumb_separation_keeps_values_bounded(TextDirection::Rtl));
    }

    /// The eight 'thumbs can be dragged together and the start / end thumb can be dragged
    /// apart' cases.
    fn thumbs_are_dragged_together_and_apart(
        text_direction: TextDirection,
        discrete: bool,
        drag_start_apart: bool,
    ) {
        let mut tester = tester_on(TargetPlatform::Android);
        let (scale, epsilon) = if discrete { (100.0, 0.01) } else { (1.0, 0.05) };
        let heard = Heard::starting_at(0.3 * scale, 0.7 * scale);
        pump(&mut tester, text_direction, &heard, move |slider| {
            if discrete {
                slider.max(100.0).divisions(10)
            } else {
                slider
            }
        });

        let (top_left, bottom_right) = track_bounds(&tester);
        let span = bottom_right - top_left;
        let middle = top_left + bottom_right / 2.0;
        let (first_is_start, second_is_start) = match text_direction {
            TextDirection::Ltr => (true, false),
            TextDirection::Rtl => (false, true),
        };
        let read = |is_start: bool| {
            if is_start {
                heard.values.get().start
            } else {
                heard.values.get().end
            }
        };

        // Drag the first thumb towards the center.
        let left_target = top_left + span * 0.3;
        tester.drag_from(left_target, middle - left_target, 1);
        assert!(near(read(first_is_start), 0.5 * scale, epsilon));

        // Drag the second thumb towards the center.
        tester.pump_and_settle();
        let right_target = top_left + span * 0.7;
        tester.drag_from(right_target, middle - right_target, 2);
        assert!(near(read(second_is_start), 0.5 * scale, epsilon));
        if discrete && text_direction == TextDirection::Rtl && drag_start_apart {
            assert!(near(heard.values.get().end, 50.0, epsilon));
        }

        // Drag a thumb apart.
        tester.pump_and_settle();
        let toward_start = match (text_direction, drag_start_apart) {
            (TextDirection::Ltr, true) | (TextDirection::Rtl, false) => -1.0,
            (TextDirection::Ltr, false) | (TextDirection::Rtl, true) => 1.0,
        };
        tester.drag_from(middle, span * (0.3 * toward_start), 3);
        if drag_start_apart {
            assert!(near(heard.values.get().start, 0.2 * scale, epsilon));
        } else {
            assert!(near(heard.values.get().end, 0.8 * scale, epsilon));
        }
    }

    macro_rules! together_and_apart_cases {
        ($($name:ident: $direction:expr, $discrete:expr, $start_apart:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    with_main_thread_stack(|| {
                        thumbs_are_dragged_together_and_apart(
                            $direction,
                            $discrete,
                            $start_apart,
                        )
                    });
                }
            )*
        };
    }

    // 'Range Slider thumbs can be dragged together and the start thumb can be dragged apart
    // (continuous / discrete, LTR / RTL)', and the same for the end thumb.
    together_and_apart_cases! {
        thumbs_dragged_together_then_the_start_thumb_apart_continuous_ltr:
            TextDirection::Ltr, false, true;
        thumbs_dragged_together_then_the_start_thumb_apart_continuous_rtl:
            TextDirection::Rtl, false, true;
        thumbs_dragged_together_then_the_start_thumb_apart_discrete_ltr:
            TextDirection::Ltr, true, true;
        thumbs_dragged_together_then_the_start_thumb_apart_discrete_rtl:
            TextDirection::Rtl, true, true;
        thumbs_dragged_together_then_the_end_thumb_apart_continuous_ltr:
            TextDirection::Ltr, false, false;
        thumbs_dragged_together_then_the_end_thumb_apart_continuous_rtl:
            TextDirection::Rtl, false, false;
        thumbs_dragged_together_then_the_end_thumb_apart_discrete_ltr:
            TextDirection::Ltr, true, false;
        thumbs_dragged_together_then_the_end_thumb_apart_discrete_rtl:
            TextDirection::Rtl, true, false;
    }

    // 'Discrete range slider has no floating-point rounding errors'.
    #[test]
    fn a_discrete_range_slider_has_no_floating_point_rounding_errors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let values_list = Rc::new(RefCell::new(Vec::new()));
            let heard_list = Rc::clone(&values_list);
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Directionality::new(
                    TextDirection::Ltr,
                    Material::new().child(
                        Center::new().child(
                            SizedBox::new().width(200.0).child(
                                RangeSlider::new(
                                    RangeValues::new(0.0, 35.0),
                                    Some(Rc::new(
                                        move |_app: &mut App, new_values: RangeValues| {
                                            heard_list.borrow_mut().push(new_values);
                                        },
                                    )),
                                )
                                .max(35.0)
                                .divisions(35),
                            ),
                        ),
                    ),
                )
                .into_widget(),
            ));

            let (top_left, bottom_right) = track_bounds(&tester);
            let active_track_width = bottom_right.dx() - top_left.dx();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, top_left);
            tester.move_to(
                &mut gesture,
                top_left + Offset::new(active_track_width * 29.0 / 35.0, 0.0),
                true,
            );
            tester.up(&gesture);
            let values_list = values_list.borrow();
            assert!(!values_list.is_empty());
            assert_eq!(*values_list.last().unwrap(), RangeValues::new(29.0, 35.0));
            for values in values_list.iter() {
                assert_eq!(values.start, values.start.round());
                assert_eq!(values.end, values.end.round());
            }
        });
    }

    // 'Range Slider onChangeEnd and onChangeStart are called on an interaction initiated by
    // tap'.
    #[test]
    fn range_slider_calls_on_change_start_and_end_for_an_interaction_initiated_by_tap() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(30.0, 70.0);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| {
                slider.max(100.0)
            });

            let (top_left, bottom_right) = track_bounds(&tester);
            // Drag the start thumb towards the center.
            let left_target = top_left + (bottom_right - top_left) * 0.3;
            assert!(heard.start_values.get().is_none());
            assert!(heard.end_values.get().is_none());
            tester.drag_from(left_target, (bottom_right - top_left) * 0.2, 1);
            let start_values = heard.start_values.get().expect("onChangeStart ran");
            let end_values = heard.end_values.get().expect("onChangeEnd ran");
            assert!(near(start_values.start, 30.0, 1.0));
            assert!(near(start_values.end, 70.0, 1.0));
            assert!(near(heard.values.get().start, 50.0, 1.0));
            assert!(near(heard.values.get().end, 70.0, 1.0));
            assert!(near(end_values.start, 50.0, 1.0));
            assert!(near(end_values.end, 70.0, 1.0));
        });
    }

    // 'Range Slider onChangeEnd and onChangeStart are called on an interaction initiated by
    // drag'.
    #[test]
    fn range_slider_calls_on_change_start_and_end_for_an_interaction_initiated_by_drag() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(30.0, 70.0);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| {
                slider.max(100.0)
            });

            let (top_left, bottom_right) = track_bounds(&tester);
            let span = bottom_right - top_left;
            // Drag the thumbs together.
            let left_target = top_left + span * 0.3;
            tester.drag_from(left_target, span * 0.2, 1);
            tester.pump_and_settle();
            let right_target = top_left + span * 0.7;
            tester.drag_from(right_target, span * -0.2, 2);
            tester.pump_and_settle();
            assert!(near(heard.values.get().start, 50.0, 1.0));
            assert!(near(heard.values.get().end, 51.0, 1.0));

            // Drag the end thumb to the right.
            let middle_target = top_left + span * 0.5;
            tester.drag_from(middle_target, span * 0.4, 3);
            tester.pump_and_settle();
            let start_values = heard.start_values.get().expect("onChangeStart ran");
            let end_values = heard.end_values.get().expect("onChangeEnd ran");
            assert!(near(start_values.start, 50.0, 1.0));
            assert!(near(start_values.end, 51.0, 1.0));
            assert!(near(end_values.start, 50.0, 1.0));
            assert!(near(end_values.end, 90.0, 1.0));
        });
    }

    fn range_render(tester: &WidgetTester) -> AnyRenderObject {
        tester
            .first::<RenderRangeSlider>()
            .as_render_object(&tester.cell.borrow())
    }

    /// Dart's `buildTheme()`.
    fn themed_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(crate::colors::Colors::BLUE)
            .slider_theme(
                SliderThemeData::new()
                    .disabled_thumb_color(Color::new(0xff000001))
                    .disabled_active_tick_mark_color(Color::new(0xff000002))
                    .disabled_active_track_color(Color::new(0xff000003))
                    .disabled_inactive_tick_mark_color(Color::new(0xff000004))
                    .disabled_inactive_track_color(Color::new(0xff000005))
                    .active_track_color(Color::new(0xff000006))
                    .active_tick_mark_color(Color::new(0xff000007))
                    .inactive_track_color(Color::new(0xff000008))
                    .inactive_tick_mark_color(Color::new(0xff000009))
                    .overlay_color(Color::new(0xff000010))
                    .thumb_color(Color::new(0xff000011))
                    .value_indicator_color(Color::new(0xff000012)),
            )
            .build(&tester.cell.borrow())
    }

    /// Dart's `buildThemedApp`.
    #[derive(Default)]
    struct ThemedArgs {
        active_color: Option<Color>,
        inactive_color: Option<Color>,
        divisions: Option<i32>,
        disabled: bool,
        /// Where Dart's `values = newValues` lands.
        record: Option<Rc<Cell<RangeValues>>>,
    }

    fn themed_app(theme: &ThemeData, args: ThemedArgs) -> WidgetRef {
        let values = RangeValues::new(0.5, 0.75);
        let record = args.record.clone();
        let on_changed: Option<ValueChanged<RangeValues>> = (!args.disabled).then(|| {
            Rc::new(move |_app: &mut App, new_values: RangeValues| {
                if let Some(record) = &record {
                    record.set(new_values);
                }
            }) as ValueChanged<RangeValues>
        });
        let mut slider = RangeSlider::new(values, on_changed).labels(RangeLabels::new(
            format!("{:.2}", values.start),
            format!("{:.2}", values.end),
        ));
        if let Some(divisions) = args.divisions {
            slider = slider.divisions(divisions);
        }
        if let Some(active_color) = args.active_color {
            slider = slider.active_color(active_color);
        }
        if let Some(inactive_color) = args.inactive_color {
            slider = slider.inactive_color(inactive_color);
        }
        material_app(
            theme.clone(),
            Directionality::new(
                TextDirection::Ltr,
                Material::new().child(Center::new().child(Theme::new(theme.clone(), slider))),
            )
            .into_widget(),
        )
    }

    /// Dart's `paints..rect(color:)`: whether `render` draws a rect in `color`; a rounded rect
    /// is another canvas call.
    fn paints_rect(tester: &WidgetTester, render: AnyRenderObject, color: Option<Color>) -> bool {
        tester.painted_ops(render).iter().any(|op| {
            matches!(op, waft_embedder::valo::Op::DrawRect { paint, .. }
                if color.is_none_or(|color| colors_match(paint.color, color)))
        })
    }

    fn rrect(color: Option<Color>) -> PaintCall {
        PaintCall::rrect(color)
    }

    fn circle(color: Option<Color>) -> PaintCall {
        PaintCall::circle(color)
    }

    /// Dart's `expect(render, paints..calls)`.
    fn paints(tester: &WidgetTester, render: AnyRenderObject, calls: &[PaintCall]) -> bool {
        paints_calls(&tester.painted_ops(render), calls)
    }

    // 'Range Slider uses the right theme colors for the right shapes for a default enabled
    // slider'.
    #[test]
    fn a_range_slider_uses_the_theme_colors_for_the_right_shapes_when_enabled() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(&theme, ThemedArgs::default()));

            // Check default theme for enabled widget.
            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(slider_theme.inactive_track_color),
                    rrect(slider_theme.inactive_track_color),
                    rrect(slider_theme.active_track_color),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[
                    circle(slider_theme.thumb_color),
                    circle(slider_theme.thumb_color)
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.active_tick_mark_color)]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.inactive_tick_mark_color)]
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes when setting the active
    // color'.
    #[test]
    fn a_range_slider_uses_its_active_color_for_the_active_shapes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            const ACTIVE_COLOR: Color = Color::new(0xcafefeed);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    active_color: Some(ACTIVE_COLOR),
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(slider_theme.inactive_track_color),
                    rrect(slider_theme.inactive_track_color),
                    rrect(Some(ACTIVE_COLOR)),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[circle(Some(ACTIVE_COLOR)), circle(Some(ACTIVE_COLOR))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.thumb_color)]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes when setting the inactive
    // color'.
    #[test]
    fn a_range_slider_uses_its_inactive_color_for_the_inactive_shapes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            const INACTIVE_COLOR: Color = Color::new(0xdeadbeef);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    inactive_color: Some(INACTIVE_COLOR),
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(slider_theme.active_track_color),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[
                    circle(slider_theme.thumb_color),
                    circle(slider_theme.thumb_color)
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes with active and inactive
    // colors'.
    #[test]
    fn a_range_slider_uses_its_active_and_inactive_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            const ACTIVE_COLOR: Color = Color::new(0xcafefeed);
            const INACTIVE_COLOR: Color = Color::new(0xdeadbeef);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    active_color: Some(ACTIVE_COLOR),
                    inactive_color: Some(INACTIVE_COLOR),
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(Some(ACTIVE_COLOR)),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[circle(Some(ACTIVE_COLOR)), circle(Some(ACTIVE_COLOR))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.thumb_color)]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes for a discrete slider'.
    #[test]
    fn a_discrete_range_slider_uses_the_theme_colors_for_the_right_shapes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    divisions: Some(3),
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(slider_theme.inactive_track_color),
                    rrect(slider_theme.inactive_track_color),
                    rrect(slider_theme.active_track_color),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[
                    circle(slider_theme.inactive_tick_mark_color),
                    circle(slider_theme.inactive_tick_mark_color),
                    circle(slider_theme.active_tick_mark_color),
                    circle(slider_theme.inactive_tick_mark_color),
                    circle(slider_theme.thumb_color),
                    circle(slider_theme.thumb_color),
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes for a discrete slider with
    // active and inactive colors'.
    #[test]
    fn a_discrete_range_slider_uses_its_active_and_inactive_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            const ACTIVE_COLOR: Color = Color::new(0xcafefeed);
            const INACTIVE_COLOR: Color = Color::new(0xdeadbeef);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    active_color: Some(ACTIVE_COLOR),
                    inactive_color: Some(INACTIVE_COLOR),
                    divisions: Some(3),
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(Some(INACTIVE_COLOR)),
                    rrect(Some(ACTIVE_COLOR)),
                ]
            ));
            assert!(paints(
                &tester,
                render,
                &[
                    circle(Some(ACTIVE_COLOR)),
                    circle(Some(ACTIVE_COLOR)),
                    circle(Some(INACTIVE_COLOR)),
                    circle(Some(ACTIVE_COLOR)),
                    circle(Some(ACTIVE_COLOR)),
                    circle(Some(ACTIVE_COLOR)),
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.thumb_color)]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.disabled_inactive_track_color
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.active_tick_mark_color)]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.inactive_tick_mark_color)]
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes for a default disabled
    // slider'.
    #[test]
    fn a_disabled_range_slider_uses_the_disabled_theme_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    disabled: true,
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(slider_theme.disabled_inactive_track_color),
                    rrect(slider_theme.disabled_inactive_track_color),
                    rrect(slider_theme.disabled_active_track_color),
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.inactive_track_color
            ));
        });
    }

    // 'Range Slider uses the right theme colors for the right shapes for a disabled slider with
    // active and inactive colors'.
    #[test]
    fn a_disabled_range_slider_ignores_its_active_and_inactive_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    active_color: Some(Color::new(0xcafefeed)),
                    inactive_color: Some(Color::new(0xdeadbeef)),
                    disabled: true,
                    ..ThemedArgs::default()
                },
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    rrect(slider_theme.disabled_inactive_track_color),
                    rrect(slider_theme.disabled_inactive_track_color),
                    rrect(slider_theme.disabled_active_track_color),
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[circle(slider_theme.thumb_color)]
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.active_track_color
            ));
            assert!(!paints_rect(
                &tester,
                render,
                slider_theme.inactive_track_color
            ));
        });
    }

    fn active_cursor_is(tester: &WidgetTester, cursor: waft_services::SystemMouseCursor) -> bool {
        tester.active_cursor(1) == cursor.into()
    }

    /// Dart's `MaterialApp(home: Directionality(child: Material(child: Center(child:
    /// child))))`.
    fn material_body<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(
            theme,
            Directionality::new(
                TextDirection::Ltr,
                Material::new().child(Center::new().child(child)),
            )
            .into_widget(),
        )
    }

    fn noop() -> Option<ValueChanged<RangeValues>> {
        Some(Rc::new(|_app: &mut App, _values: RangeValues| {}))
    }

    // 'RangeSlider changes mouse cursor when hovered'.
    #[test]
    fn a_range_slider_changes_the_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            use waft_services::SystemMouseCursors;
            let mut tester = tester_on(TargetPlatform::Android);
            let values = RangeValues::new(50.0, 70.0);
            let theme = default_theme(&tester);
            let frame = |slider: RangeSlider| {
                material_body(
                    theme.clone(),
                    MouseRegion::new()
                        .cursor(SystemMouseCursors::FORBIDDEN.into())
                        .child(slider),
                )
            };
            // Test default cursor.
            tester.pump_widget(frame(RangeSlider::new(values, noop()).max(100.0)));

            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, slider_rect(&tester).center());
            assert!(active_cursor_is(&tester, SystemMouseCursors::CLICK));

            // Test custom cursor.
            tester.pump_widget(frame(
                RangeSlider::new(values, noop())
                    .max(100.0)
                    .mouse_cursor(Rc::new(WidgetStatePropertyAll::new(Some(
                        SystemMouseCursors::TEXT.into(),
                    )))),
            ));
            tester.pump(Duration::ZERO);
            assert!(active_cursor_is(&tester, SystemMouseCursors::TEXT));
        });
    }

    // 'RangeSlider WidgetStateMouseCursor resolves correctly'.
    #[test]
    fn a_range_slider_resolves_a_widget_state_mouse_cursor() {
        with_main_thread_stack(|| {
            use waft_services::{PhysicalKeyboardKey, SystemMouseCursors};
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(20.0, 75.0);
            let theme = default_theme(&tester);
            let build_frame = |enabled: bool| {
                let heard = Rc::clone(&heard);
                material_app(
                    theme.clone(),
                    Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(
                            waft_widgets::Column::new()
                                .main_axis_alignment(waft_rendering::MainAxisAlignment::Center)
                                .children([Center::new()
                                .child(StatefulBuilder::new(move |_app, _context, set_state| {
                                    let slider = range_slider(&heard, set_state, &|slider| {
                                        slider.max(100.0).mouse_cursor(<dyn WidgetStateProperty<
                                            Option<MouseCursorRef>,
                                        >>::resolve_with(
                                            |states| {
                                                Some(if states.contains(&WidgetState::Disabled) {
                                                    SystemMouseCursors::FORBIDDEN.into()
                                                } else if states.contains(&WidgetState::Dragged) {
                                                    SystemMouseCursors::MOVE.into()
                                                } else if states.contains(&WidgetState::Hovered) {
                                                    SystemMouseCursors::GRAB.into()
                                                } else {
                                                    SystemMouseCursors::CLICK.into()
                                                })
                                            },
                                        ))
                                    });
                                    if enabled {
                                        slider.into_widget()
                                    } else {
                                        RangeSlider {
                                            on_changed: None,
                                            ..slider
                                        }
                                        .into_widget()
                                    }
                                }))
                                .into_widget()]),
                        ),
                    )
                    .into_widget(),
                )
            };

            let mut gesture = WidgetTester::mouse(1, 0);
            // Dart's tester starts with an empty view; this one has none until it pumps.
            tester.pump_widget(waft_widgets::SizedBox::new().into_widget());

            // System default.
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump_widget(build_frame(false));
            assert!(active_cursor_is(&tester, SystemMouseCursors::BASIC));

            // Disabled.
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);
            tester.pump(Duration::ZERO);
            assert!(active_cursor_is(&tester, SystemMouseCursors::FORBIDDEN));

            // Hovered.
            tester.pump_widget(build_frame(true));
            assert!(active_cursor_is(&tester, SystemMouseCursors::GRAB));

            // Dragged.
            let mut pressed = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            pressed.position = gesture.position;
            tester.down(&mut pressed, slider_rect(&tester).center());
            let at = pressed.position + Offset::new(20.0, 0.0);
            tester.move_to(&mut pressed, at, true);
            tester.pump(Duration::ZERO);
            assert!(active_cursor_is(&tester, SystemMouseCursors::MOVE));

            // Hovered.
            tester.up(&pressed);
            gesture.position = pressed.position;
            tester.pump(Duration::ZERO);
            assert!(active_cursor_is(&tester, SystemMouseCursors::GRAB));

            // System default.
            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump(Duration::ZERO);
            assert!(active_cursor_is(&tester, SystemMouseCursors::BASIC));
        });
    }

    /// The pair of live focus nodes a mounted range slider owns.
    fn thumb_focus_nodes(tester: &WidgetTester) -> (AnyFocusNode, AnyFocusNode) {
        let element = tester.find_by_type::<RangeSlider>()[0];
        let state = element
            .state_handle::<RangeSliderState>(&tester.cell.borrow())
            .expect("a range slider state");
        let app = tester.cell.borrow();
        (state.start_focus_node(&app), state.end_focus_node(&app))
    }

    fn primary_focus(tester: &WidgetTester) -> Option<AnyFocusNode> {
        waft_widgets::primary_focus(&mut tester.cell.borrow_mut())
    }

    fn send_key(tester: &mut WidgetTester, logical: LogicalKeyboardKey) {
        use waft_services::PhysicalKeyboardKey;
        let physical = if logical == LogicalKeyboardKey::ARROW_RIGHT {
            PhysicalKeyboardKey::ARROW_RIGHT
        } else if logical == LogicalKeyboardKey::ARROW_LEFT {
            PhysicalKeyboardKey::ARROW_LEFT
        } else if logical == LogicalKeyboardKey::ARROW_UP {
            PhysicalKeyboardKey::ARROW_UP
        } else if logical == LogicalKeyboardKey::ARROW_DOWN {
            PhysicalKeyboardKey::ARROW_DOWN
        } else if logical == LogicalKeyboardKey::ENTER {
            PhysicalKeyboardKey::ENTER
        } else {
            PhysicalKeyboardKey::TAB
        };
        tester.send_key_event(physical, logical);
        tester.pump_and_settle();
    }

    // 'RangeSlider can be focused using keyboard focus'.
    #[test]
    fn a_range_slider_can_be_focused_using_keyboard_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(20.0, 80.0);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| {
                slider.max(100.0)
            });

            // Focus on the start thumb
            assert_eq!(tester.find_by_type::<RangeSlider>().len(), 1);
            let (start_focus_node, end_focus_node) = thumb_focus_nodes(&tester);
            start_focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(primary_focus(&tester), Some(start_focus_node));

            // Tab to focus on the end thumb
            send_key(&mut tester, LogicalKeyboardKey::TAB);
            assert_eq!(primary_focus(&tester), Some(end_focus_node));
        });
    }

    /// Dart's `pumpRangeSlider` of the directional-navigation group: a range slider between two
    /// focusable neighbors, with the arrow keys bound to directional focus traversal.
    struct DirectionalFixture {
        heard: Rc<Heard>,
        left_neighbor: Ref<FocusNode>,
        right_neighbor: Ref<FocusNode>,
    }

    fn pump_directional_range_slider(
        tester: &mut WidgetTester,
        navigation_mode: NavigationMode,
    ) -> DirectionalFixture {
        use waft_widgets::{
            DirectionalFocusIntent, Focus, MediaQueryData, Shortcuts, TraversalDirection,
        };
        let shortcuts: ShortcutMap = [
            (LogicalKeyboardKey::ARROW_LEFT, TraversalDirection::Left),
            (LogicalKeyboardKey::ARROW_RIGHT, TraversalDirection::Right),
            (LogicalKeyboardKey::ARROW_UP, TraversalDirection::Up),
            (LogicalKeyboardKey::ARROW_DOWN, TraversalDirection::Down),
        ]
        .into_iter()
        .map(|(key, direction)| {
            (
                Rc::new(SingleActivator::new(key)) as ShortcutActivatorRef,
                Rc::new(DirectionalFocusIntent::new(direction)) as IntentRef,
            )
        })
        .collect();
        let left_neighbor = FocusNode::new(&mut tester.cell.borrow_mut());
        let right_neighbor = FocusNode::new(&mut tester.cell.borrow_mut());
        let heard = Heard::starting_at(40.0, 80.0);
        let theme = default_theme(tester);
        let (left, right, shared) = (
            left_neighbor.erase::<AnyFocusNode>(),
            right_neighbor.erase::<AnyFocusNode>(),
            Rc::clone(&heard),
        );
        tester.pump_widget(material_app(
            theme,
            Shortcuts::new(
                shortcuts,
                Material::new().child(Directionality::new(
                    TextDirection::Ltr,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let slider = range_slider(&shared, set_state, &|slider| slider.max(100.0));
                        MediaQuery::new(
                            MediaQueryData::default().navigation_mode(navigation_mode),
                            Center::new().child(
                                Row::new()
                                    .main_axis_size(waft_rendering::MainAxisSize::Min)
                                    .children([
                                        Focus::new(SizedBox::new().width(50.0).height(200.0))
                                            .focus_node(left.clone())
                                            .into_widget(),
                                        SizedBox::new().width(300.0).child(slider).into_widget(),
                                        Focus::new(SizedBox::new().width(50.0).height(200.0))
                                            .focus_node(right.clone())
                                            .into_widget(),
                                    ]),
                            ),
                        )
                        .into_widget()
                    }),
                )),
            )
            .into_widget(),
        ));
        DirectionalFixture {
            heard,
            left_neighbor,
            right_neighbor,
        }
    }

    fn focus_start_thumb(tester: &mut WidgetTester) -> AnyFocusNode {
        let (start_focus_node, _) = thumb_focus_nodes(tester);
        start_focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        start_focus_node
    }

    // 'arrow keys do not change the value when not in editing mode'.
    #[test]
    fn directional_arrow_keys_move_the_focus_outside_editing_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let fixture = pump_directional_range_slider(&mut tester, NavigationMode::Directional);
            let (left, right) = (
                FocusNodeLeaf::as_node(fixture.left_neighbor.handle()),
                FocusNodeLeaf::as_node(fixture.right_neighbor.handle()),
            );
            let start_focus_node = focus_start_thumb(&mut tester);

            send_key(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            // arrowRight should move focus, not change the value, outside editing mode
            assert_eq!(fixture.heard.values.get(), RangeValues::new(40.0, 80.0));
            // arrowRight should move the focus to the right neighbor
            assert_eq!(primary_focus(&tester), Some(right));

            send_key(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            // arrowLeft should move focus, not change the value, outside editing mode
            assert_eq!(fixture.heard.values.get(), RangeValues::new(40.0, 80.0));
            // arrowLeft should move the focus back to the start thumb
            assert_eq!(primary_focus(&tester), Some(start_focus_node));

            send_key(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            assert_eq!(fixture.heard.values.get(), RangeValues::new(40.0, 80.0));
            // a second arrowLeft should move the focus to the left neighbor
            assert_eq!(primary_focus(&tester), Some(left));
        });
    }

    // 'pressing enter enters editing mode and arrow keys adjust the value'.
    #[test]
    fn pressing_enter_enters_editing_mode_and_arrow_keys_adjust_the_value() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let fixture = pump_directional_range_slider(&mut tester, NavigationMode::Directional);
            focus_start_thumb(&mut tester);

            // Enter editing mode.
            send_key(&mut tester, LogicalKeyboardKey::ENTER);

            // In LTR, the right arrow increases the focused (start) thumb...
            send_key(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            let increased_start = fixture.heard.values.get().start;
            assert!(increased_start > 40.0);
            assert_eq!(fixture.heard.values.get().end, 80.0);

            // ...and the left arrow decreases it.
            send_key(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            assert!(fixture.heard.values.get().start < increased_start);
            assert_eq!(fixture.heard.values.get().end, 80.0);
        });
    }

    // 'pressing enter again exits editing mode'.
    #[test]
    fn pressing_enter_again_exits_editing_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let fixture = pump_directional_range_slider(&mut tester, NavigationMode::Directional);
            focus_start_thumb(&mut tester);

            // Enter then exit editing mode.
            send_key(&mut tester, LogicalKeyboardKey::ENTER);
            send_key(&mut tester, LogicalKeyboardKey::ENTER);

            // Arrow keys no longer adjust the value.
            send_key(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            assert_eq!(fixture.heard.values.get(), RangeValues::new(40.0, 80.0));
        });
    }

    // 'losing focus exits editing mode'.
    #[test]
    fn losing_focus_exits_editing_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let fixture = pump_directional_range_slider(&mut tester, NavigationMode::Directional);
            let start_focus_node = focus_start_thumb(&mut tester);

            // Enter editing mode.
            send_key(&mut tester, LogicalKeyboardKey::ENTER);

            // Losing focus should reset the editing mode.
            start_focus_node.unfocus(
                &mut tester.cell.borrow_mut(),
                waft_widgets::UnfocusDisposition::Scope,
            );
            tester.pump_and_settle();

            // Re-focus and verify arrow keys no longer adjust the value, proving the
            // editing mode was reset when focus was lost.
            start_focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            send_key(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            assert_eq!(fixture.heard.values.get(), RangeValues::new(40.0, 80.0));
        });
    }

    // 'arrow keys change the value directly in traditional navigation mode'.
    #[test]
    fn arrow_keys_change_the_value_directly_in_traditional_navigation_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let fixture = pump_directional_range_slider(&mut tester, NavigationMode::Traditional);
            focus_start_thumb(&mut tester);

            // No need to enter an editing mode in traditional navigation.
            send_key(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            assert!(fixture.heard.values.get().start > 40.0);
            assert_eq!(fixture.heard.values.get().end, 80.0);

            send_key(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            assert!(near(fixture.heard.values.get().start, 40.0, 1e-9));
            assert_eq!(fixture.heard.values.get().end, 80.0);
        });
    }

    /// Dart's `Material.of(...)` matched with `paints..circle(color:)`.
    fn paints_circle_of_color(tester: &WidgetTester, color: Color) -> bool {
        crate::slider::tests::paints_circle(tester, range_render(tester), color)
    }

    fn enabled_range_slider_app(
        theme: ThemeData,
        values: &Rc<Cell<RangeValues>>,
        enabled: bool,
        configure: impl Fn(RangeSlider) -> RangeSlider + 'static,
    ) -> WidgetRef {
        let values = Rc::clone(values);
        material_app(
            theme,
            Directionality::new(
                TextDirection::Ltr,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let on_changed_values = Rc::clone(&values);
                    let on_changed: Option<ValueChanged<RangeValues>> = enabled.then(|| {
                        Rc::new(move |app: &mut App, new_values: RangeValues| {
                            let values = Rc::clone(&on_changed_values);
                            set_state(app, &mut |_app| values.set(new_values));
                        }) as ValueChanged<RangeValues>
                    });
                    Material::new()
                        .child(
                            Center::new()
                                .child(configure(RangeSlider::new(values.get(), on_changed))),
                        )
                        .into_widget()
                }),
            )
            .into_widget(),
        )
    }

    // 'RangeSlider can be hovered and has correct hover color'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn a_range_slider_can_be_hovered_and_has_the_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(50.0, 70.0)));
            let theme = default_theme(&tester);
            let hover_color = theme.color_scheme.primary.with_opacity(0.12);
            let build_app = |enabled: bool| {
                enabled_range_slider_app(theme.clone(), &values, enabled, |slider| {
                    slider.max(100.0)
                })
            };

            tester.pump_widget(build_app(true));
            // RangeSlider does not have overlay when enabled and not hovered.
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, hover_color));

            // Start hovering.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);

            // RangeSlider has overlay when enabled and hovered.
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();
            assert!(paints_circle_of_color(&tester, hover_color));

            // RangeSlider does not have an overlay when disabled and hovered.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, hover_color));
        });
    }

    // 'RangeSlider is draggable and has correct dragged color'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn a_range_slider_is_draggable_and_has_the_dragged_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(50.0, 70.0)));
            let theme = default_theme(&tester);
            let dragged_color = theme.color_scheme.primary.with_opacity(0.12);
            tester.pump_widget(enabled_range_slider_app(theme, &values, true, |slider| {
                slider.max(100.0)
            }));

            // RangeSlider does not have overlay when enabled and not dragged.
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, dragged_color));

            // Start dragging.
            let mut drag = WidgetTester::touch(1);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            // Less than configured touch slop, more than default touch slop
            let at = drag.position + Offset::new(19.0, 0.0);
            tester.move_to(&mut drag, at, true);
            tester.pump(Duration::ZERO);

            // RangeSlider has overlay when enabled and dragged.
            assert!(paints_circle_of_color(&tester, dragged_color));
        });
    }

    // 'RangeSlider overlayColor supports hovered and dragged states'.
    #[test]
    fn a_range_slider_overlay_color_supports_hovered_and_dragged_states() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(50.0, 70.0)));
            const HOVER_COLOR: Color = Color::new(0xffff0000);
            const DRAGGED_COLOR: Color = Color::new(0xff0000ff);
            let theme = default_theme(&tester);
            let build_app = |enabled: bool| {
                enabled_range_slider_app(theme.clone(), &values, enabled, move |slider| {
                    let slider =
                        slider.max(100.0).overlay_color(
                            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                                if states.contains(&WidgetState::Hovered) {
                                    return Some(HOVER_COLOR);
                                }
                                if states.contains(&WidgetState::Dragged) {
                                    return Some(DRAGGED_COLOR);
                                }
                                None
                            }),
                        );
                    if enabled {
                        slider
                            .on_change_start(|_app, _values| {})
                            .on_change_end(|_app, _values| {})
                    } else {
                        slider
                    }
                })
            };

            tester.pump_widget(build_app(true));
            // RangeSlider does not have overlay when enabled and not hovered.
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, HOVER_COLOR));

            // Hover on the range slider but outside the thumb.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, slider_rect(&tester).top_left(), false);
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, HOVER_COLOR));

            // Hover on the thumb.
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);
            tester.pump_and_settle();
            assert!(paints_circle_of_color(&tester, HOVER_COLOR));

            // Hover on the slider but outside the thumb.
            tester.move_to(&mut gesture, slider_rect(&tester).bottom_right(), false);
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, HOVER_COLOR));

            // Reset range slider values.
            values.set(RangeValues::new(50.0, 70.0));
            // RangeSlider does not have overlay when enabled and not dragged.
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, DRAGGED_COLOR));

            // Start dragging.
            let mut drag = WidgetTester::touch(2);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            // Less than configured touch slop, more than default touch slop.
            let at = drag.position + Offset::new(19.0, 0.0);
            tester.move_to(&mut drag, at, true);
            tester.pump(Duration::ZERO);

            // RangeSlider has overlay when enabled and dragged.
            assert!(paints_circle_of_color(&tester, DRAGGED_COLOR));

            // Stop dragging.
            tester.up(&drag);
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, DRAGGED_COLOR));
        });
    }

    // 'RangeSlider onChangeStart and onChangeEnd fire once'.
    #[test]
    fn range_slider_on_change_start_and_end_fire_once() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/128433
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(40.0, 80.0);
            let (start_fired, end_fired) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
            let (start, end) = (Rc::clone(&start_fired), Rc::clone(&end_fired));
            let theme = default_theme(&tester);
            let _ = heard;
            tester.pump_widget(material_body(
                theme,
                waft_widgets::GestureDetector::new()
                    .on_horizontal_drag_update(Rc::new(|_app: &mut App, _details| {}))
                    .child(
                        RangeSlider::new(RangeValues::new(40.0, 80.0), noop())
                            .max(100.0)
                            .on_change_start(move |_app, _values| start.set(start.get() + 1))
                            .on_change_end(move |_app, _values| end.set(end.get() + 1)),
                    ),
            ));

            tester.drag_from(slider_rect(&tester).top_left(), Offset::new(100.0, 0.0), 1);
            assert_eq!(start_fired.get(), 1);
            assert_eq!(end_fired.get(), 1);
        });
    }

    // 'RangeSlider in a ListView does not throw an exception'.
    #[test]
    fn a_range_slider_in_a_list_view_does_not_throw() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/126648
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Directionality::new(
                    TextDirection::Ltr,
                    Material::new().child(
                        waft_widgets::ListView::new().children([
                            SizedBox::new().height(600.0).into_widget(),
                            RangeSlider::new(RangeValues::new(40.0, 80.0), noop())
                                .max(100.0)
                                .into_widget(),
                        ]),
                    ),
                )
                .into_widget(),
            ));

            assert_eq!(tester.find_by_type::<RangeSlider>().len(), 1);
        });
    }

    // 'Range Slider top thumb gets stroked when overlapping'.
    #[test]
    fn the_top_range_slider_thumb_is_stroked_when_overlapping() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.7);
            let theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .primary_swatch(crate::colors::Colors::BLUE)
                .slider_theme(
                    SliderThemeData::new()
                        .thumb_color(Color::new(0xff000001))
                        .overlapping_shape_stroke_color(Color::new(0xff000002)),
                )
                .build(&tester.cell.borrow());
            let slider_theme = theme.slider_theme.clone();
            let app_theme = default_theme(&tester);
            tester.pump_widget(range_slider_app(
                app_theme,
                TextDirection::Ltr,
                &heard,
                move |_app, _context, slider| {
                    crate::theme::Theme::new(theme.clone(), slider).into_widget()
                },
                |slider| slider,
            ));

            let (top_left, bottom_right) = track_bounds(&tester);
            let span = bottom_right - top_left;
            let middle = top_left + bottom_right / 2.0;
            // Drag the thumbs towards the center.
            let left_target = top_left + span * 0.3;
            tester.drag_from(left_target, middle - left_target, 1);
            tester.pump_and_settle();
            let right_target = top_left + span * 0.7;
            tester.drag_from(right_target, middle - right_target, 2);
            assert!(near(heard.values.get().start, 0.5, 0.03));
            assert!(near(heard.values.get().end, 0.5, 0.03));
            tester.pump_and_settle();

            // The drag gesture has finished, so no overlayColor should show up.
            assert!(paints(
                &tester,
                range_render(&tester),
                &[
                    circle(slider_theme.thumb_color),
                    circle(slider_theme.overlapping_shape_stroke_color),
                    circle(slider_theme.thumb_color),
                ]
            ));
        });
    }

    // 'Range Slider thumb gets stroked when overlapping'.
    #[test]
    fn a_range_slider_thumb_is_stroked_when_overlapping() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.7);
            let theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .primary_swatch(crate::colors::Colors::BLUE)
                .slider_theme(
                    SliderThemeData::new()
                        .value_indicator_color(Color::new(0xff000001))
                        .show_value_indicator(ShowValueIndicator::OnlyForContinuous),
                )
                .build(&tester.cell.borrow());
            let app_theme = default_theme(&tester);
            tester.pump_widget(range_slider_app(
                app_theme,
                TextDirection::Ltr,
                &heard,
                {
                    let theme = theme.clone();
                    move |_app, _context, slider| {
                        crate::theme::Theme::new(theme.clone(), slider).into_widget()
                    }
                },
                |slider| {
                    let values = slider.values;
                    slider.labels(RangeLabels::new(
                        format!("{}", values.start),
                        format!("{}", values.end),
                    ))
                },
            ));

            let (top_left, bottom_right) = track_bounds(&tester);
            let span = bottom_right - top_left;
            let middle = top_left + bottom_right / 2.0;
            // Drag the thumbs towards the center.
            let left_target = top_left + span * 0.3;
            tester.drag_from(left_target, middle - left_target, 1);
            tester.pump_and_settle();
            let right_target = top_left + span * 0.7;
            tester.drag_from(right_target, middle - right_target, 2);
            tester.pump_and_settle();
            assert!(near(heard.values.get().start, 0.5, 0.03));
            assert!(near(heard.values.get().end, 0.5, 0.03));

            let mut gesture = WidgetTester::touch(3);
            tester.down(&mut gesture, middle);
            tester.pump_and_settle();

            // The first circle is the thumb, the second one is the overlapping shape
            // circle, and the last one is the second thumb.
            let stroke = theme.slider_theme.overlapping_shape_stroke_color;
            assert!(paints(
                &tester,
                range_render(&tester),
                &[circle(None), circle(stroke), circle(None)]
            ));
            tester.up(&gesture);
            assert!(paints(
                &tester,
                range_render(&tester),
                &[circle(None), circle(stroke), circle(None)]
            ));
        });
    }

    /// How many shadows the range slider paints: Dart's `path(color: Colors.black)` calls,
    /// which `debugDisableShadows` makes of a shadow, a `DrawShadow` here while shadows show.
    /// Dart's `expect(Overlay, paints..path(color: Colors.black)..path(color: Colors.black)
    /// ..path(color: valueIndicatorColor)..paragraph())`: the thumbs' two shadows, then the
    /// value indicator's fill and label.
    fn overlay_paints_shadows_and_indicator(tester: &WidgetTester, color: Color) -> bool {
        paints_calls(
            &tester.painted_ops_deep(overlay_render(tester)),
            &[
                any_thumb_shadow(),
                any_thumb_shadow(),
                PaintCall::path(Some(color)),
                PaintCall::Paragraph,
            ],
        )
    }

    /// Dart's `expectValueIndicator` in 'Value indicator appears when it should': whether the
    /// value indicator box, the last render object in the tree, paints a path in the theme's
    /// value indicator colour and a paragraph.
    fn indicator_visible(tester: &WidgetTester, color: Color) -> bool {
        let Some(value_indicator_box) = tester.all::<RenderValueIndicator>().into_iter().last()
        else {
            return false;
        };
        let render = value_indicator_box.as_render_object(&tester.cell.borrow());
        paints(
            tester,
            render,
            &[PaintCall::path(Some(color)), PaintCall::Paragraph],
        )
    }

    fn expect_value_indicator(
        tester: &mut WidgetTester,
        is_visible: bool,
        theme: &SliderThemeData,
        divisions: Option<i32>,
        enabled: bool,
        dragged: bool,
    ) {
        let base_theme = ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(crate::colors::Colors::BLUE)
            .build(&tester.cell.borrow());
        let value = RangeValues::new(1.0, 5.0);
        let on_changed: Option<ValueChanged<RangeValues>> = enabled.then(|| {
            Rc::new(|_app: &mut App, _values: RangeValues| {}) as ValueChanged<RangeValues>
        });
        let mut slider = RangeSlider::new(value, on_changed)
            .max(10.0)
            .labels(RangeLabels::new(
                format!("{}", value.start),
                format!("{}", value.end),
            ));
        if let Some(divisions) = divisions {
            slider = slider.divisions(divisions);
        }
        let app_theme = default_theme(tester);
        tester.pump_widget(material_app(
            app_theme,
            Material::new()
                .child(Center::new().child(Theme::new(
                    base_theme,
                    SliderTheme::new(theme.clone(), slider),
                )))
                .into_widget(),
        ));
        let mut gesture = WidgetTester::touch(1);
        if dragged {
            tester.down(&mut gesture, slider_rect(tester).center());
        }
        // Wait for value indicator animation to finish.
        tester.pump_and_settle();
        let color = theme
            .value_indicator_color
            .expect("the case's theme has one");
        assert_eq!(indicator_visible(tester, color), is_visible);
        if dragged {
            tester.up(&gesture);
        }
    }

    // 'Value indicator appears when it should'.
    #[test]
    fn the_range_value_indicator_appears_when_it_should() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let base_theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .primary_swatch(crate::colors::Colors::BLUE)
                .build(&tester.cell.borrow());
            let mut theme = base_theme
                .slider_theme
                .copy_with()
                .value_indicator_color(crate::colors::Colors::RED.primary());
            let expect = |tester: &mut WidgetTester,
                          theme: &SliderThemeData,
                          rows: &[(bool, Option<i32>, bool, bool)]| {
                for (is_visible, divisions, enabled, dragged) in rows {
                    expect_value_indicator(
                        tester,
                        *is_visible,
                        theme,
                        *divisions,
                        *enabled,
                        *dragged,
                    );
                }
            };

            // Default (showValueIndicator set to onlyForDiscrete).
            expect(
                &mut tester,
                &theme,
                &[
                    (true, Some(10), true, true),
                    (false, Some(10), false, true),
                    (false, None, true, true),
                    (false, None, false, true),
                    (false, Some(10), true, false),
                    (false, Some(3), false, false),
                    (false, None, true, false),
                    (false, None, false, false),
                ],
            );

            // With showValueIndicator set to onlyForContinuous.
            theme = theme
                .copy_with()
                .show_value_indicator(ShowValueIndicator::OnlyForContinuous);
            expect(
                &mut tester,
                &theme,
                &[
                    (false, Some(10), true, true),
                    (false, Some(10), false, true),
                    (true, None, true, true),
                    (false, None, false, true),
                    (false, Some(10), true, false),
                    (false, Some(3), false, false),
                    (false, None, true, false),
                    (false, None, false, false),
                ],
            );

            // discrete enabled widget with showValueIndicator set to onDrag.
            theme = theme
                .copy_with()
                .show_value_indicator(ShowValueIndicator::OnDrag);
            expect(
                &mut tester,
                &theme,
                &[
                    (true, Some(10), true, true),
                    (false, Some(10), false, true),
                    (true, None, true, true),
                    (false, None, false, true),
                    (false, Some(10), true, false),
                    (false, Some(3), false, false),
                    (false, None, true, false),
                    (false, None, false, false),
                ],
            );

            // discrete enabled widget with showValueIndicator set to never.
            theme = theme
                .copy_with()
                .show_value_indicator(ShowValueIndicator::Never);
            expect(
                &mut tester,
                &theme,
                &[
                    (false, Some(10), true, true),
                    (false, Some(10), false, true),
                    (false, None, true, true),
                    (false, None, false, true),
                    (false, Some(10), true, false),
                    (false, Some(3), false, false),
                    (false, None, true, false),
                    (false, None, false, false),
                ],
            );

            // discrete enabled widget with showValueIndicator set to alwaysVisible.
            theme = theme
                .copy_with()
                .show_value_indicator(ShowValueIndicator::AlwaysVisible);
            expect(
                &mut tester,
                &theme,
                &[
                    (true, Some(3), true, true),
                    (true, Some(3), false, true),
                    (true, None, true, true),
                    (true, None, false, true),
                    (true, Some(3), true, false),
                    (true, Some(3), false, false),
                    (true, None, true, false),
                    (true, None, false, false),
                ],
            );
        });
    }

    fn range_slider_size(tester: &WidgetTester) -> Size {
        tester
            .render_box_of(tester.find_by_type::<RangeSlider>()[0])
            .size(&tester.cell.borrow())
    }

    // 'RangeSlider does not crash at zero area'.
    #[test]
    fn a_range_slider_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .body(
                        Center::new().child(
                            SizedBox::shrink()
                                .child(RangeSlider::new(RangeValues::new(0.0, 1.0), noop())),
                        ),
                    )
                    .into_widget(),
            ));

            assert_eq!(range_slider_size(&tester), Size::ZERO);
        });
    }

    // 'Range Slider can be painted in a narrower constraint when track shape is
    // RoundedRectRange'.
    #[test]
    fn a_range_slider_can_be_painted_in_a_narrower_constraint_with_a_rounded_track() {
        with_main_thread_stack(|| {
            use waft_embedder::{RRect, Radius};
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            tester.pump_widget(material_body(
                theme,
                SizedBox::new()
                    .height(10.0)
                    .width(0.0)
                    .child(RangeSlider::new(RangeValues::new(0.25, 0.5), None)),
            ));

            let render = range_render(&tester);
            let circular = Radius::circular(2.0);
            assert!(paints(
                &tester,
                render,
                &[
                    // left inactive track RRect
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            -24.0,
                            3.0,
                            -12.0,
                            7.0,
                            circular,
                            Radius::ZERO,
                            Radius::ZERO,
                            circular,
                        ),
                        None,
                    ),
                    // right inactive track RRect
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            0.0,
                            3.0,
                            24.0,
                            7.0,
                            Radius::ZERO,
                            circular,
                            circular,
                            Radius::ZERO,
                        ),
                        None,
                    ),
                    // active track RRect
                    PaintCall::rrect_at(RRect::from_ltrbr(-14.0, 2.0, 2.0, 8.0, circular), None),
                    // thumbs
                    PaintCall::Circle {
                        x: Some(-12.0),
                        y: Some(5.0),
                        radius: Some(10.0),
                        color: None
                    },
                    PaintCall::Circle {
                        x: Some(0.0),
                        y: Some(5.0),
                        radius: Some(10.0),
                        color: None
                    },
                ]
            ));
        });
    }

    // 'Range Slider can be painted in a narrower constraint when track shape is Rectangular'.
    #[test]
    fn a_range_slider_can_be_painted_in_a_narrower_constraint_with_a_rectangular_track() {
        with_main_thread_stack(|| {
            use crate::range_slider_parts::RectangularRangeSliderTrackShape;
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        .range_track_shape(Rc::new(RectangularRangeSliderTrackShape::new())),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_body(
                theme,
                SizedBox::new()
                    .height(10.0)
                    .width(0.0)
                    .child(RangeSlider::new(RangeValues::new(0.25, 0.5), None)),
            ));

            let render = range_render(&tester);
            // There should no gap between the inactive track and active track.
            let rects: Vec<Rect> = tester
                .painted_ops(render)
                .iter()
                .filter_map(|op| match op {
                    waft_embedder::valo::Op::DrawRect { rect, .. } => Some(Rect::from_ltwh(
                        f64::from(rect.x),
                        f64::from(rect.y),
                        f64::from(rect.width),
                        f64::from(rect.height),
                    )),
                    _ => None,
                })
                .collect();
            assert_eq!(
                rects,
                [
                    // left inactive track
                    Rect::from_ltrb(-24.0, 3.0, -12.0, 7.0),
                    // active track
                    Rect::from_ltrb(-12.0, 3.0, 0.0, 7.0),
                    // right inactive track
                    Rect::from_ltrb(0.0, 3.0, 24.0, 7.0),
                ]
            );
            // thumbs
            assert!(paints(
                &tester,
                render,
                &[
                    PaintCall::Circle {
                        x: Some(-12.0),
                        y: Some(5.0),
                        radius: Some(10.0),
                        color: None
                    },
                    PaintCall::Circle {
                        x: Some(0.0),
                        y: Some(5.0),
                        radius: Some(10.0),
                        color: None
                    },
                ]
            ));
        });
    }

    // 'Update the divisions and values at the same time for RangeSlider'.
    #[test]
    fn updating_the_divisions_and_values_at_the_same_time_places_the_thumbs() {
        with_main_thread_stack(|| {
            // Regress test for https://github.com/flutter/flutter/issues/65943
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let build_frame = |max_value: f64| {
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            Center::new().child(
                                RangeSlider::new(RangeValues::new(5.0, 8.0), noop())
                                    .max(max_value)
                                    .divisions(max_value as i32),
                            ),
                        )
                        .into_widget(),
                )
            };

            tester.pump_widget(build_frame(10.0));
            let render = range_render(&tester);

            // Update the divisions from 10 to 15, the thumbs should be paint at the correct
            // position.
            tester.pump_widget(build_frame(15.0));
            tester.pump_and_settle(); // Finish the animation.
            let track_bounds: Vec<Rect> = tester
                .painted(render)
                .into_iter()
                .filter_map(|drawn| match drawn {
                    crate::test_support::Drawn::Path(_, bounds) => Some(bounds),
                    _ => None,
                })
                .collect();
            // The third path is the active track's rounded rect.
            let active_track = track_bounds[2];

            const PADDING: f64 = 4.0;
            // The 1st thumb should at one-third(5 / 15) of the Slider.
            // The 2nd thumb should at (8 / 15) of the Slider.
            // The left of the active track shape is the position of the 1st thumb.
            // The right of the active track shape is the position of the 2nd thumb.
            // 24.0 is the default margin, (800.0 - 24.0 - 24.0 - padding) is the slider's width.
            // Where the padding value equals to the track height.
            assert!(near(
                active_track.left,
                (800.0 - 24.0 - 24.0 - PADDING) * (5.0 / 15.0) + 24.0,
                0.01
            ));
            assert!(near(
                active_track.right,
                (800.0 - 24.0 - 24.0 - PADDING) * (8.0 / 15.0) + 24.0 + PADDING,
                0.01
            ));
        });
    }

    /// Dart's `LoggingRangeSliderValueIndicatorShape`: a value indicator that paints nothing
    /// and logs the label painter's text and width.
    /// What the logging shape recorded of a label: its plain text and its style.
    type LoggedLabels = Rc<RefCell<Vec<(String, Option<TextStyle>)>>>;

    #[derive(Debug)]
    struct LoggingRangeSliderValueIndicatorShape {
        log_label: LoggedLabels,
        log_painter_width: Rc<RefCell<Vec<f64>>>,
    }

    impl RangeSliderValueIndicatorShape for LoggingRangeSliderValueIndicatorShape {
        fn get_preferred_size(
            &self,
            _is_enabled: bool,
            _is_discrete: bool,
            _label_painter: &TextPainter,
            _text_scale_factor: f64,
        ) -> Size {
            Size::new(10.0, 10.0)
        }

        fn paint(
            &self,
            _app: &mut App,
            _context: &mut PaintingContext,
            _center: Offset,
            _activation_animation: AnyAnimation<f64>,
            _enable_animation: AnyAnimation<f64>,
            _is_discrete: Option<bool>,
            _is_on_top: Option<bool>,
            label_painter: &mut TextPainter,
            _text_scale_factor: Option<f64>,
            _size_with_overflow: Option<Size>,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _text_direction: Option<TextDirection>,
            _value: Option<f64>,
            _thumb: Option<Thumb>,
        ) {
            let span = label_painter.text().cloned().expect("labelPainter.text!");
            self.log_label
                .borrow_mut()
                .push((label_painter.plain_text().to_owned(), span.style().cloned()));
            self.log_painter_width
                .borrow_mut()
                .push(label_painter.width());
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // 'Value indicator appears on tap'.
    #[test]
    fn the_range_value_indicator_appears_on_tap() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            let discrete_values = RangeValues::new(20.0, 40.0);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        RangeSlider::new(discrete_values, noop())
                            .labels(RangeLabels::new(
                                format!("{}", discrete_values.start.round()),
                                format!("{}", discrete_values.end.round()),
                            ))
                            .divisions(5)
                            .max(100.0),
                    )
                    .into_widget(),
            ));

            tester.tap(slider_rect(&tester).center(), 1);
            tester.pump_and_settle();

            assert!(overlay_paints_shadows_and_indicator(
                &tester,
                slider_theme
                    .value_indicator_color
                    .expect("the theme has a value indicator color")
            ));
        });
    }

    // 'RangeSlider value indicator text when year2023 is false'.
    #[test]
    fn the_range_value_indicator_text_when_year2023_is_false() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let values = RangeValues::new(25.0, 75.0);
            let log_label = Rc::new(RefCell::new(Vec::new()));
            let shape = Rc::new(LoggingRangeSliderValueIndicatorShape {
                log_label: Rc::clone(&log_label),
                log_painter_width: Rc::default(),
            });
            let theme = ThemeData::builder()
                .slider_theme(SliderThemeData::new().range_value_indicator_shape(shape))
                .build(&tester.cell.borrow());
            #[expect(deprecated, reason = "Dart's case passes `year2023`")]
            let slider = RangeSlider::new(values, noop())
                .year2023(false)
                .max(100.0)
                .labels(RangeLabels::new(
                    format!("{}", values.start.round()),
                    format!("{}", values.end.round()),
                ))
                .divisions(4);
            tester.pump_widget(material_app(
                theme.clone(),
                Material::new()
                    .child(Center::new().child(slider))
                    .into_widget(),
            ));

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).top_left());
            tester.pump_and_settle();

            let (text, style) = log_label
                .borrow()
                .last()
                .cloned()
                .expect("the shape painted");
            assert_eq!(text, "25");
            let style = style.expect("the label's style");
            assert_eq!(style.font_size, Some(14.0));
            assert_eq!(
                style.color.as_ref().map(|color| color.color()),
                Some(theme.color_scheme.on_inverse_surface())
            );
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    // 'RangeSlider labels respect horizontal buffer and avoid screen overflow'.
    #[test]
    #[expect(deprecated, reason = "Dart's case passes the deprecated `always`")]
    fn range_slider_labels_respect_the_horizontal_buffer() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let log_painter_width = Rc::new(RefCell::new(Vec::new()));
            let shape = Rc::new(LoggingRangeSliderValueIndicatorShape {
                log_label: Rc::default(),
                log_painter_width: Rc::clone(&log_painter_width),
            });
            const LONG_LABEL_START: &str =
                "A very long start label string that exceeds standard screen widths";
            const LONG_LABEL_END: &str =
                "A very long end label string that exceeds standard screen widths";
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .body(SliderTheme::new(
                        SliderThemeData::new()
                            .show_value_indicator(ShowValueIndicator::Always)
                            .range_value_indicator_shape(shape),
                        RangeSlider::new(RangeValues::new(0.2, 0.8), noop())
                            .divisions(10)
                            .labels(RangeLabels::new(LONG_LABEL_START, LONG_LABEL_END)),
                    ))
                    .into_widget(),
            ));

            // Press the range slider to trigger the value indicators
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).center());
            tester.pump_and_settle();

            let screen_width = 800.0;
            let widths = log_painter_width.borrow();
            assert!(!widths.is_empty());
            assert!(*widths.last().unwrap() <= screen_width - 64.0);
            drop(widths);
            tester.up(&gesture);
        });
    }

    /// Dart's `_PositionRecordingRangeTickMarkShape`: paints nothing, records each tick's
    /// center.
    #[derive(Debug)]
    struct PositionRecordingRangeTickMarkShape {
        positions: Rc<RefCell<Vec<Offset>>>,
    }

    impl RangeSliderTickMarkShape for PositionRecordingRangeTickMarkShape {
        fn get_preferred_size(
            &self,
            _slider_theme: &SliderThemeData,
            _is_enabled: Option<bool>,
        ) -> Size {
            Size::ZERO
        }

        fn paint(
            &self,
            _app: &mut App,
            _context: &mut PaintingContext,
            center: Offset,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _enable_animation: AnyAnimation<f64>,
            _start_thumb_center: Offset,
            _end_thumb_center: Offset,
            _is_enabled: Option<bool>,
            _text_direction: TextDirection,
        ) {
            self.positions.borrow_mut().push(center);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // 'Discrete RangeSlider snaps at the visual midpoint between ticks'.
    #[test]
    #[expect(clippy::needless_range_loop, reason = "Dart's loops index `ticks`")]
    fn a_discrete_range_slider_snaps_at_the_visual_midpoint_between_ticks() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/184391.
            const DIVISIONS: usize = 10;
            let mut tester = tester_on(TargetPlatform::Android);
            let values = Rc::new(Cell::new(RangeValues::new(0.0, 10.0)));
            let tick_centers = Rc::new(RefCell::new(Vec::new()));
            let theme = default_theme(&tester);
            let (shared_values, shared_ticks) = (Rc::clone(&values), Rc::clone(&tick_centers));
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(SliderTheme::new(
                        SliderThemeData::new().range_tick_mark_shape(Rc::new(
                            PositionRecordingRangeTickMarkShape {
                                positions: shared_ticks,
                            },
                        )),
                        StatefulBuilder::new(move |_app, _context, set_state| {
                            let values = Rc::clone(&shared_values);
                            let on_changed_values = Rc::clone(&values);
                            Center::new()
                                .child(
                                    RangeSlider::new(
                                        values.get(),
                                        Some(Rc::new(
                                            move |app: &mut App, new_values: RangeValues| {
                                                let values = Rc::clone(&on_changed_values);
                                                set_state(app, &mut |_app| values.set(new_values));
                                            },
                                        )),
                                    )
                                    .max(10.0)
                                    .divisions(DIVISIONS as i32),
                                )
                                .into_widget()
                        }),
                    ))
                    .into_widget(),
            ));

            // Later repaints append to the list, so keep only the first frame's ticks.
            let ticks: Vec<Offset> = tick_centers
                .borrow()
                .iter()
                .take(DIVISIONS + 1)
                .copied()
                .collect();
            assert_eq!(ticks.len(), DIVISIONS + 1);

            // A small offset keeps the taps well clear of the exact midpoint while
            // staying smaller than the smallest drift of the old formula.
            const MIDPOINT_OFFSET: f64 = 0.05;
            let midpoint_dx = |i: usize| (ticks[i].dx() + ticks[i + 1].dx()) / 2.0;

            // Taps in the lower half move the start thumb.
            for i in 0..4 {
                tester.tap(
                    Offset::new(midpoint_dx(i) - MIDPOINT_OFFSET, ticks[i].dy()),
                    1,
                );
                tester.pump(Duration::ZERO);
                // Just left of the midpoint of ticks i and i + 1
                assert_eq!(values.get().start, i as f64);
                tester.tap(
                    Offset::new(midpoint_dx(i) + MIDPOINT_OFFSET, ticks[i].dy()),
                    2,
                );
                tester.pump(Duration::ZERO);
                // Just right of the midpoint of ticks i and i + 1
                assert_eq!(values.get().start, (i + 1) as f64);
            }

            // Taps closer to the end thumb than the start thumb (which is now at 4) move
            // the end thumb.
            for i in (DIVISIONS - 3)..DIVISIONS {
                tester.tap(
                    Offset::new(midpoint_dx(i) - MIDPOINT_OFFSET, ticks[i].dy()),
                    3,
                );
                tester.pump(Duration::ZERO);
                assert_eq!(values.get().end, i as f64);
                tester.tap(
                    Offset::new(midpoint_dx(i) + MIDPOINT_OFFSET, ticks[i].dy()),
                    4,
                );
                tester.pump(Duration::ZERO);
                assert_eq!(values.get().end, (i + 1) as f64);
            }
        });
    }

    /// The `overlayColor` cases' app: a range slider (0.3, 0.7) with `overlay_color`.
    fn overlay_color_app(
        theme: ThemeData,
        values: &Rc<Cell<RangeValues>>,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> WidgetRef {
        enabled_range_slider_app(theme, values, true, move |slider| {
            slider.overlay_color(overlay_color.clone())
        })
    }

    // 'RangeSlider keeps drag overlay when touch moves outside thumb'.
    #[test]
    fn a_range_slider_keeps_the_drag_overlay_when_touch_moves_outside_the_thumb() {
        with_main_thread_stack(|| {
            const OVERLAY_COLOR: Color = Color::new(0xff800080);
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTouch);
            let values = Rc::new(Cell::new(RangeValues::new(0.3, 0.7)));
            let theme = default_theme(&tester);
            tester.pump_widget(overlay_color_app(
                theme,
                &values,
                Rc::new(WidgetStatePropertyAll::new(Some(OVERLAY_COLOR))),
            ));

            let rect = slider_rect(&tester);
            let start_thumb_pos = rect.top_left() + (rect.bottom_right() - rect.top_left()) * 0.3;
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, start_thumb_pos);
            let at = gesture.position + Offset::new(40.0, -80.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump_and_settle();
            assert!(paints_circle_of_color(&tester, OVERLAY_COLOR));

            tester.up(&gesture);
            tester.pump_and_settle();
            assert!(!paints_circle_of_color(&tester, OVERLAY_COLOR));
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::Automatic);
        });
    }

    // 'RangeSlider touch focus does not show persistent overlay'.
    #[test]
    fn a_range_slider_touch_focus_does_not_show_a_persistent_overlay() {
        with_main_thread_stack(|| {
            const OVERLAY_COLOR: Color = Color::new(0xff800080);
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTouch);
            let values = Rc::new(Cell::new(RangeValues::new(0.3, 0.7)));
            let theme = default_theme(&tester);
            tester.pump_widget(overlay_color_app(
                theme,
                &values,
                Rc::new(WidgetStatePropertyAll::new(Some(OVERLAY_COLOR))),
            ));

            let rect = slider_rect(&tester);
            let start_thumb_pos = rect.top_left() + (rect.bottom_right() - rect.top_left()) * 0.3;
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, start_thumb_pos);
            tester.pump(Duration::ZERO);
            let at = gesture.position + Offset::new(10.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.up(&gesture);
            tester.pump_and_settle();

            let (start_focus_node, _) = thumb_focus_nodes(&tester);
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(!paints_circle_of_color(&tester, OVERLAY_COLOR));
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::Automatic);
        });
    }

    // 'RangeSlider keyboard focus shows focused overlay'.
    #[test]
    fn a_range_slider_keyboard_focus_shows_a_focused_overlay() {
        with_main_thread_stack(|| {
            const OVERLAY_COLOR: Color = Color::new(0xff800080);
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(0.3, 0.7)));
            let theme = default_theme(&tester);
            tester.pump_widget(overlay_color_app(
                theme,
                &values,
                Rc::new(WidgetStatePropertyAll::new(Some(OVERLAY_COLOR))),
            ));

            send_key(&mut tester, LogicalKeyboardKey::TAB);

            let (start_focus_node, _) = thumb_focus_nodes(&tester);
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(paints_circle_of_color(&tester, OVERLAY_COLOR));
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::Automatic);
        });
    }

    // 'RangeSlider overlayColor resolves focused state'.
    #[test]
    fn a_range_slider_overlay_color_resolves_the_focused_state() {
        with_main_thread_stack(|| {
            const FOCUSED_COLOR: Color = Color::new(0xff800080);
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(0.3, 0.7)));
            let theme = default_theme(&tester);
            tester.pump_widget(overlay_color_app(
                theme,
                &values,
                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                    states
                        .contains(&WidgetState::Focused)
                        .then_some(FOCUSED_COLOR)
                }),
            ));

            assert!(!paints_circle_of_color(&tester, FOCUSED_COLOR));

            send_key(&mut tester, LogicalKeyboardKey::TAB);

            let (start_focus_node, _) = thumb_focus_nodes(&tester);
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(paints_circle_of_color(&tester, FOCUSED_COLOR));
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::Automatic);
        });
    }

    // 'RangeSlider taps should set focus on start/end thumbs'.
    #[test]
    fn taps_set_focus_on_the_start_and_end_thumbs() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.7);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| slider);

            let rect = slider_rect(&tester);
            let span = rect.bottom_right() - rect.top_left();

            // Tap near the start thumb (0.3)
            tester.tap(rect.top_left() + span * 0.3, 1);
            tester.pump(Duration::ZERO);

            // Verify focus changed to start thumb
            let (start_focus_node, end_focus_node) = thumb_focus_nodes(&tester);
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(start_focus_node));

            // Reset focus
            primary_focus(&tester).expect("a primary focus").unfocus(
                &mut tester.cell.borrow_mut(),
                waft_widgets::UnfocusDisposition::Scope,
            );
            tester.pump(Duration::ZERO);

            // Tap near the end thumb (0.7)
            tester.tap(rect.top_left() + span * 0.7, 2);
            tester.pump(Duration::ZERO);

            // Verify focus changed to end thumb
            assert!(end_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(end_focus_node));
        });
    }

    // 'RangeSlider drag should set focus on start/end thumbs'.
    #[test]
    fn drags_set_focus_on_the_start_and_end_thumbs() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.7);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| slider);

            let rect = slider_rect(&tester);
            let span = rect.bottom_right() - rect.top_left();
            let (start_focus_node, end_focus_node) = thumb_focus_nodes(&tester);

            // Drag start thumb
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, rect.top_left() + span * 0.3);
            tester.pump(Duration::ZERO);

            // Verify focus on start drag
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(start_focus_node));
            let at = gesture.position + Offset::new(10.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);

            // Reset focus
            primary_focus(&tester).expect("a primary focus").unfocus(
                &mut tester.cell.borrow_mut(),
                waft_widgets::UnfocusDisposition::Scope,
            );
            tester.pump(Duration::ZERO);

            // Drag end thumb
            let mut end_gesture = WidgetTester::touch(2);
            tester.down(&mut end_gesture, rect.top_left() + span * 0.7);
            tester.pump(Duration::ZERO);

            // Verify focus on end drag
            assert!(end_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(end_focus_node));
            let at = end_gesture.position + Offset::new(-10.0, 0.0);
            tester.move_to(&mut end_gesture, at, true);
            tester.up(&end_gesture);
            tester.pump(Duration::ZERO);
        });
    }

    // 'RangeSlider tap start thumb then tab should focus end thumb'.
    #[test]
    fn tapping_the_start_thumb_then_tab_focuses_the_end_thumb() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.3, 0.7);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| slider);

            let rect = slider_rect(&tester);
            let span = rect.bottom_right() - rect.top_left();

            // Tap near the start thumb (0.3)
            tester.tap(rect.top_left() + span * 0.3, 1);
            tester.pump(Duration::ZERO);

            // Verify start thumb has focus
            let (start_focus_node, end_focus_node) = thumb_focus_nodes(&tester);
            assert!(start_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(start_focus_node));

            // Press Tab
            tester.send_key_event(
                waft_services::PhysicalKeyboardKey::TAB,
                LogicalKeyboardKey::TAB,
            );
            tester.pump(Duration::ZERO);

            // Verify end thumb has focus
            assert!(end_focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert_eq!(primary_focus(&tester), Some(end_focus_node));
        });
    }

    // 'Drag gesture uses provided gesture settings'.
    #[test]
    fn the_range_slider_drag_gesture_uses_the_gesture_settings_given() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/105833
            let mut tester = tester_on(TargetPlatform::Android);
            let values = Rc::new(Cell::new(RangeValues::new(0.1, 0.5)));
            let drag_started = Rc::new(Cell::new(false));
            let slider_end = Rc::new(Cell::new(false));
            let theme = default_theme(&tester);
            let build = |touch_slop: f64, with_end: bool| {
                let (values, drag_started, slider_end) = (
                    Rc::clone(&values),
                    Rc::clone(&drag_started),
                    Rc::clone(&slider_end),
                );
                material_app(
                    theme.clone(),
                    Directionality::new(
                        TextDirection::Ltr,
                        StatefulBuilder::new(move |app, context, set_state| {
                            let on_changed_values = Rc::clone(&values);
                            let started = Rc::clone(&drag_started);
                            let ended = Rc::clone(&slider_end);
                            let mut slider = RangeSlider::new(
                                values.get(),
                                Some(Rc::new(move |app: &mut App, new_values: RangeValues| {
                                    let values = Rc::clone(&on_changed_values);
                                    set_state(app, &mut |_app| values.set(new_values));
                                })),
                            );
                            if with_end {
                                slider = slider.on_change_end(move |_app, _values| ended.set(true));
                            }
                            let data = MediaQuery::of(app, context)
                                .copy_with()
                                .gesture_settings(DeviceGestureSettings::new(Some(touch_slop)));
                            Material::new()
                                .child(
                                    Center::new().child(
                                        waft_widgets::GestureDetector::new()
                                            .behavior(waft_rendering::HitTestBehavior::DeferToChild)
                                            .on_horizontal_drag_start(Rc::new(
                                                move |_app: &mut App, _details: DragStartDetails| {
                                                    started.set(true)
                                                },
                                            )
                                                as waft_gestures::GestureDragStartCallback)
                                            .child(MediaQuery::new(data, slider).into_widget()),
                                    ),
                                )
                                .into_widget()
                        }),
                    )
                    .into_widget(),
                )
            };
            tester.pump_widget(build(20.0, false));

            let mut drag = WidgetTester::touch(1);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);

            // Less than configured touch slop, more than default touch slop
            let at = drag.position + Offset::new(19.0, 0.0);
            tester.move_to(&mut drag, at, true);
            tester.pump(Duration::ZERO);

            assert_eq!(values.get(), RangeValues::new(0.1, 0.5));
            assert!(drag_started.get());

            drag_started.set(false);

            tester.up(&drag);
            tester.pump_and_settle();

            let mut drag = WidgetTester::touch(2);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);

            tester.pump_widget(build(10.0, true));

            // More than touch slop.
            let at = drag.position + Offset::new(12.0, 0.0);
            tester.move_to(&mut drag, at, true);

            tester.up(&drag);
            tester.pump_and_settle();

            assert!(slider_end.get());
            assert!(!drag_started.get());
            assert_eq!(
                tester
                    .first::<RenderRangeSlider>()
                    .gesture_settings(&tester.cell.borrow()),
                Some(DeviceGestureSettings::new(Some(10.0)))
            );
        });
    }

    /// The two 'top value indicator gets stroked when overlapping' cases: the thumbs dragged
    /// together under `text_scale`, then pressed.
    #[expect(deprecated, reason = "Dart's case passes the deprecated `always`")]
    fn the_top_value_indicator_is_stroked_when_overlapping(text_scale: Option<f64>) {
        let mut tester = tester_on(TargetPlatform::Android);
        let heard = Heard::starting_at(0.3, 0.7);
        let theme = ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(crate::colors::Colors::BLUE)
            .slider_theme(
                SliderThemeData::new()
                    .value_indicator_color(Color::new(0xff000001))
                    .overlapping_shape_stroke_color(Color::new(0xff000002))
                    .show_value_indicator(ShowValueIndicator::Always),
            )
            .build(&tester.cell.borrow());
        let slider_theme = theme.slider_theme.clone();
        let app_theme = default_theme(&tester);
        tester.pump_widget(range_slider_app(
            app_theme,
            TextDirection::Ltr,
            &heard,
            {
                let theme = theme.clone();
                move |_app, _context, slider| {
                    let themed = Theme::new(theme.clone(), slider).into_widget();
                    match text_scale {
                        Some(scale) => MediaQuery::new(
                            waft_widgets::MediaQueryData::default()
                                .text_scaler(waft_painting::TextScaler::linear(scale)),
                            themed,
                        )
                        .into_widget(),
                        None => themed,
                    }
                }
            },
            |slider| {
                let values = slider.values;
                slider.labels(RangeLabels::new(
                    format!("{:.2}", values.start),
                    format!("{:.2}", values.end),
                ))
            },
        ));

        let (top_left, bottom_right) = track_bounds(&tester);
        let span = bottom_right - top_left;
        let middle = top_left + bottom_right / 2.0;
        // Drag the thumbs towards the center.
        let left_target = top_left + span * 0.3;
        tester.drag_from(left_target, middle - left_target, 1);
        tester.pump_and_settle();
        let right_target = top_left + span * 0.7;
        tester.drag_from(right_target, middle - right_target, 2);
        tester.pump_and_settle();
        assert!(near(heard.values.get().start, 0.5, 0.03));
        assert!(near(heard.values.get().end, 0.5, 0.03));

        let mut gesture = WidgetTester::touch(3);
        tester.down(&mut gesture, middle);
        tester.pump_and_settle();
        assert!(overlay_paints_shadows_and_indicator(
            &tester,
            slider_theme
                .value_indicator_color
                .expect("the theme has a value indicator color")
        ));
        tester.up(&gesture);
    }

    // 'Range Slider top value indicator gets stroked when overlapping'.
    #[test]
    fn the_top_range_value_indicator_is_stroked_when_overlapping() {
        with_main_thread_stack(|| the_top_value_indicator_is_stroked_when_overlapping(None));
    }

    // 'Range Slider top value indicator gets stroked when overlapping with large text scale'.
    #[test]
    fn the_top_range_value_indicator_is_stroked_when_overlapping_with_a_large_text_scale() {
        with_main_thread_stack(|| the_top_value_indicator_is_stroked_when_overlapping(Some(2.0)));
    }

    // 'Discrete RangeSlider does not apply thumb padding in a non-rounded track shape'.
    #[test]
    fn a_discrete_range_slider_does_not_apply_thumb_padding_in_a_non_rounded_track() {
        with_main_thread_stack(|| {
            use crate::range_slider_parts::RectangularRangeSliderTrackShape;
            // Regression test for https://github.com/flutter/flutter/issues/161805
            let mut tester = tester_on(TargetPlatform::Android);
            // The default track left and right padding.
            const SLIDER_PADDING: f64 = 24.0;
            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        // Thumb padding is applied based on the track height.
                        .track_height(100.0)
                        .range_track_shape(Rc::new(RectangularRangeSliderTrackShape::new())),
                )
                .build(&tester.cell.borrow());
            let primary = theme.color_scheme.primary;
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        SizedBox::new().width(300.0).child(
                            RangeSlider::new(RangeValues::new(0.0, 100.0), noop())
                                .max(100.0)
                                .divisions(100),
                        ),
                    )
                    .into_widget(),
            ));

            let render = range_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    // Start thumb.
                    PaintCall::Circle {
                        x: Some(SLIDER_PADDING),
                        y: Some(300.0),
                        radius: None,
                        color: Some(primary)
                    },
                    // End thumb.
                    PaintCall::Circle {
                        x: Some(800.0 - SLIDER_PADDING),
                        y: Some(300.0),
                        radius: None,
                        color: Some(primary)
                    },
                ]
            ));
        });
    }

    // 'RangeSlider supports DropRangeSliderValueIndicatorShape'.
    #[test]
    fn a_range_slider_supports_the_drop_value_indicator_shape() {
        with_main_thread_stack(|| {
            use crate::range_slider_parts::DropRangeSliderValueIndicatorShape;
            let mut tester = tester_on(TargetPlatform::Android);
            let values = RangeValues::new(25.0, 75.0);
            const VALUE_INDICATOR_COLOR: Color = Color::new(0xffff0000);
            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        .range_value_indicator_shape(Rc::new(
                            DropRangeSliderValueIndicatorShape::new(),
                        ))
                        .value_indicator_color(VALUE_INDICATOR_COLOR),
                )
                .build(&tester.cell.borrow());
            #[expect(deprecated, reason = "Dart's case passes `year2023`")]
            let slider = RangeSlider::new(values, noop())
                .year2023(false)
                .max(100.0)
                .labels(RangeLabels::new(
                    format!("{}", values.start.round()),
                    format!("{}", values.end.round()),
                ))
                .divisions(4);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(slider))
                    .into_widget(),
            ));

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).top_left());
            tester.pump_and_settle();
            let value_indicator = tester
                .all::<RenderValueIndicator>()
                .into_iter()
                .last()
                .expect("a value indicator")
                .as_render_object(&tester.cell.borrow());
            assert!(paints(
                &tester,
                value_indicator,
                &[PaintCall::path(Some(VALUE_INDICATOR_COLOR))]
            ));
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    /// The live handles a mounted range slider owns: its state's controllers, focus nodes,
    /// value-indicator portal controller and layer link, and its two render objects' curved
    /// animations.
    fn range_slider_owned_handles(tester: &WidgetTester) -> Vec<waft_foundation::HandleId> {
        let (render, value_indicator) = (
            tester.first::<RenderRangeSlider>(),
            tester.first::<RenderValueIndicator>(),
        );
        let app = tester.cell.borrow();
        let (render, value_indicator) = (render.get(&app), value_indicator.get(&app));
        let state = app.get(render.state);
        let mut handles: Vec<waft_foundation::HandleId> = Vec::new();
        for controller in [
            &state.overlay_controller,
            &state.value_indicator_controller,
            &state.enable_controller,
            &state.start_position_controller,
            &state.end_position_controller,
        ] {
            handles.push(
                controller
                    .as_ref()
                    .expect("made in init_state")
                    .handle()
                    .into(),
            );
        }
        for focus_node in [&state.start_focus_node, &state.end_focus_node] {
            handles.push(
                focus_node
                    .as_ref()
                    .expect("made in init_state")
                    .handle()
                    .into(),
            );
        }
        handles.push(
            state
                .value_indicator_overlay_portal_controller
                .as_ref()
                .expect("made in init_state")
                .handle()
                .into(),
        );
        handles.push(
            state
                .layer_link
                .as_ref()
                .expect("made in init_state")
                .handle()
                .into(),
        );
        let animations: [waft_foundation::HandleId; 4] = [
            render.overlay_animation.handle().into(),
            render.value_indicator_animation.handle().into(),
            render.enable_animation.handle().into(),
            value_indicator.value_indicator_animation.handle().into(),
        ];
        handles.extend(animations);
        handles
    }

    /// Not a Dart case: what Dart's collector frees once a range slider is gone, its
    /// controllers, focus nodes, portal controller, layer link and curved animations, is freed
    /// here with the last reference to each.
    #[test]
    fn an_unmounted_range_slider_leaves_none_of_its_objects_alive() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0, 4.0);
            pump(&mut tester, TextDirection::Ltr, &heard, |slider| {
                slider
                    .max(4.0)
                    .divisions(4)
                    .labels(RangeLabels::new("0", "4"))
            });
            let handles = range_slider_owned_handles(&tester);
            {
                let app = tester.cell.borrow();
                assert!(handles.iter().all(|&handle| app.contains(handle)));
            }

            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_and_settle();
            let app = tester.cell.borrow();
            for handle in handles {
                assert!(!app.contains(handle), "{handle:?} is still alive");
            }
        });
    }

    /// Not the Dart case, which also counts the `Overlay`'s `drawPath` calls (6, then 2): valo
    /// records a `drawRRect` and a `drawPath` of a rounded rectangle alike, which a button's
    /// physical shape paints as the one, so only the paragraph counts and the sequence are
    /// read here.
    #[test]
    fn a_range_slider_disposed_mid_indicator_removes_its_value_indicator_from_the_overlay() {
        with_main_thread_stack(|| {
            use waft_widgets::{Builder, Column, Navigator, Text, WidgetBuilder};

            use crate::elevated_button::ElevatedButton;
            use crate::page::MaterialPageRoute;
            use crate::scaffold::Scaffold;

            let mut tester = tester_on(TargetPlatform::Android);
            let values = Rc::new(Cell::new(RangeValues::new(0.5, 0.75)));
            const FILL_COLOR: Color = Color::new(0xf55f5f5f);
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let builder_values = Rc::clone(&values);
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(Builder::new(move |_app, context| {
                        // The builder is used to pass the context from the MaterialApp widget
                        // to the [Navigator]. This context is required in order for the
                        // Navigator to work.
                        let on_changed_values = Rc::clone(&builder_values);
                        let current = builder_values.get();
                        Column::new()
                            .children(vec![
                                RangeSlider::new(
                                    current,
                                    Some(Rc::new(
                                        move |_app: &mut App, new_values: RangeValues| {
                                            on_changed_values.set(new_values);
                                        },
                                    )),
                                )
                                .labels(RangeLabels::new(
                                    format!("{:.2}", current.start),
                                    format!("{:.2}", current.end),
                                ))
                                .divisions(5)
                                .into_widget(),
                                ElevatedButton::new(
                                    Some(Listener::new(move |app| {
                                        let inner: WidgetBuilder =
                                            Rc::new(|_app: &mut App, context: BuildContext| {
                                                ElevatedButton::new(
                                                    Some(Listener::new(move |app| {
                                                        Navigator::pop(app, context, None);
                                                    })),
                                                    Some(Text::new("Inner page")),
                                                )
                                                .into_widget()
                                            });
                                        let route = MaterialPageRoute::new(app, inner);
                                        Navigator::push_replacement(
                                            app,
                                            context,
                                            route.erase(),
                                            None,
                                        );
                                    })),
                                    Some(Text::new("Next")),
                                )
                                .into_widget(),
                            ])
                            .into_widget()
                    }))
                    .into_widget(),
            ));

            let overlay_ops =
                |tester: &WidgetTester| tester.painted_ops_deep(overlay_render(tester));
            let top_right = slider_rect(&tester).top_right() + Offset::new(-24.0, 0.0);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, top_right);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();

            assert_eq!(tester.find_by_type::<RangeSlider>().len(), 1);
            let sequence = [
                // Represents the raised button with next text.
                any_thumb_shadow(),
                PaintCall::Paragraph,
                // Represents the range slider.
                PaintCall::path(Some(FILL_COLOR)),
                PaintCall::Paragraph,
                PaintCall::path(Some(FILL_COLOR)),
                PaintCall::Paragraph,
            ];
            assert!(paints_calls(&overlay_ops(&tester), &sequence));
            // Represents the Raised Button and Range Slider.
            assert_eq!(
                count_calls(&overlay_ops(&tester), CanvasCall::DrawParagraph),
                3
            );

            let next = tester.find_onstage_text("Next")[0];
            let next = tester.global_rect(tester.render_box_of(next)).center();
            tester.tap(next, 2);
            tester.pump_and_settle();

            assert!(tester.find_by_type::<RangeSlider>().is_empty());
            assert!(!paints_calls(&overlay_ops(&tester), &sequence[2..]));
            // Represents the raised button with inner page text.
            assert_eq!(
                count_calls(&overlay_ops(&tester), CanvasCall::DrawParagraph),
                1
            );
            assert_eq!(tester.find_onstage_text("Inner page").len(), 1);

            // Don't stop holding the value indicator.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    /// slider_theme_test's `_buildRangeApp`.
    fn build_range_app(
        theme: ThemeData,
        slider_theme: SliderThemeData,
        values: RangeValues,
        divisions: Option<i32>,
    ) -> WidgetRef {
        let mut slider = RangeSlider::new(
            values,
            Some(Rc::new(|_app: &mut App, _values: RangeValues| {})),
        )
        .labels(RangeLabels::new(
            format!("{:?}", values.start),
            format!("{:?}", values.end),
        ));
        if let Some(divisions) = divisions {
            slider = slider.divisions(divisions);
        }
        material_app(
            theme,
            crate::scaffold::Scaffold::new()
                .body(Center::new().child(SliderTheme::new(slider_theme, slider)))
                .into_widget(),
        )
    }

    /// Dart's `paintsExactlyCountTimes(#drawPath, n)` counted over the range slider and its
    /// value indicators: the non-circle paths they draw, which valo records a `drawCircle`
    /// beside.
    fn path_count(tester: &WidgetTester) -> usize {
        let renders = [
            range_render(tester),
            tester
                .first::<RenderValueIndicator>()
                .as_render_object(&tester.cell.borrow()),
        ];
        renders
            .into_iter()
            .map(|render| {
                tester
                    .painted_ops(render)
                    .iter()
                    .filter(|op| {
                        matches!(op, waft_embedder::valo::Op::DrawPath { path, .. }
                            if crate::slider::tests::circle_of(path).is_none())
                    })
                    .count()
            })
            .sum()
    }

    // 'PaddleRangeSliderValueIndicatorShape skips all painting at zero scale' and 'Default
    // range indicator shape skips all painting at zero scale'.
    fn skips_all_painting_at_zero_scale(slider_theme: SliderThemeData) {
        let mut tester = tester_on(TargetPlatform::Android);
        let theme = default_theme(&tester);
        tester.pump_widget(build_range_app(
            theme,
            slider_theme,
            RangeValues::new(0.0, 0.5),
            Some(4),
        ));

        // Tap the center of the track to kick off the animation of the value indicator.
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, slider_rect(&tester).center());
        // No value indicator path to paint at scale 0.
        tester.pump(Duration::ZERO);
        assert_eq!(path_count(&tester), 0);

        // Painting a path for each value indicator.
        tester.pump(Duration::from_millis(16));
        assert_eq!(path_count(&tester), 2);
        tester.up(&gesture);
    }

    #[test]
    fn the_paddle_range_value_indicator_skips_all_painting_at_zero_scale() {
        with_main_thread_stack(|| {
            use crate::slider_value_indicator_shape::PaddleRangeSliderValueIndicatorShape;
            skips_all_painting_at_zero_scale(
                SliderThemeData::new()
                    .track_height(0.0)
                    .range_value_indicator_shape(Rc::new(
                        PaddleRangeSliderValueIndicatorShape::new(),
                    )),
            );
        });
    }

    #[test]
    #[expect(deprecated, reason = "Dart's case passes the deprecated `always`")]
    fn the_default_range_value_indicator_skips_all_painting_at_zero_scale() {
        with_main_thread_stack(|| {
            skips_all_painting_at_zero_scale(
                SliderThemeData::new()
                    .track_height(0.0)
                    .overlay_shape(<dyn crate::SliderComponentShape>::no_overlay())
                    .thumb_shape(<dyn crate::SliderComponentShape>::no_thumb())
                    .tick_mark_shape(<dyn crate::SliderTickMarkShape>::no_tick_mark())
                    .show_value_indicator(ShowValueIndicator::Always),
            );
        });
    }

    /// slider_theme_test's four `valueIndicatorStrokeColor` cases for the range slider's
    /// rectangular and paddle value indicators: with `overlapping` the two thumbs sit together
    /// and the top indicator takes the overlapping shape's stroke colour.
    #[expect(deprecated, reason = "Dart's case passes the deprecated `always`")]
    fn value_indicator_stroke_colors(
        shape: crate::range_slider_parts::RangeSliderValueIndicatorShapeRef,
        overlapping: bool,
    ) {
        const VALUE_INDICATOR_COLOR: Color = Color::new(0xff000001);
        const STROKE_COLOR: Color = Color::new(0xff000002);
        const OVERLAPPING_COLOR: Color = Color::new(0xff000003);
        let mut tester = tester_on(TargetPlatform::Android);
        let mut slider_theme = SliderThemeData::new()
            .show_value_indicator(ShowValueIndicator::Always)
            .range_value_indicator_shape(shape)
            .value_indicator_color(VALUE_INDICATOR_COLOR)
            .value_indicator_stroke_color(STROKE_COLOR);
        if overlapping {
            slider_theme = slider_theme.overlapping_shape_stroke_color(OVERLAPPING_COLOR);
        }
        let values = if overlapping {
            RangeValues::new(0.0, 0.0)
        } else {
            RangeValues::new(0.0, 0.5)
        };
        let theme = ThemeData::builder()
            .slider_theme(slider_theme)
            .build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Material::new()
                .child(Center::new().child(RangeSlider::new(values, noop()).labels(
                    RangeLabels::new(format!("{:?}", values.start), format!("{:?}", values.end)),
                )))
                .into_widget(),
        ));

        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, slider_rect(&tester).center());
        // Wait for value indicator animation to finish.
        tester.pump_and_settle();

        let second_stroke = if overlapping {
            OVERLAPPING_COLOR
        } else {
            STROKE_COLOR
        };
        // Dart's `paints` sequence off the `Overlay`: the thumbs' two shadows, then each
        // indicator's stroke and fill.
        assert!(paints_calls(
            &tester.painted_ops_deep(overlay_render(&tester)),
            &[
                any_thumb_shadow(),
                any_thumb_shadow(),
                PaintCall::path(Some(STROKE_COLOR)),
                PaintCall::path(Some(VALUE_INDICATOR_COLOR)),
                PaintCall::path(Some(second_stroke)),
                PaintCall::path(Some(VALUE_INDICATOR_COLOR)),
            ]
        ));
        tester.up(&gesture);
    }

    // 'RectangularRangeSliderValueIndicatorShape supports
    // SliderTheme.valueIndicatorStrokeColor'.
    #[test]
    fn the_rectangular_range_value_indicator_supports_the_stroke_color() {
        with_main_thread_stack(|| {
            value_indicator_stroke_colors(
                Rc::new(
                    crate::slider_value_indicator_shape::RectangularRangeSliderValueIndicatorShape::new(),
                ),
                false,
            );
        });
    }

    // '... on overlapping indicator'.
    #[test]
    fn the_rectangular_range_value_indicator_supports_the_stroke_color_when_overlapping() {
        with_main_thread_stack(|| {
            value_indicator_stroke_colors(
                Rc::new(
                    crate::slider_value_indicator_shape::RectangularRangeSliderValueIndicatorShape::new(),
                ),
                true,
            );
        });
    }

    // 'PaddleRangeSliderValueIndicatorShape supports SliderTheme.valueIndicatorStrokeColor'.
    #[test]
    fn the_paddle_range_value_indicator_supports_the_stroke_color() {
        with_main_thread_stack(|| {
            value_indicator_stroke_colors(
                Rc::new(
                    crate::slider_value_indicator_shape::PaddleRangeSliderValueIndicatorShape::new(
                    ),
                ),
                false,
            );
        });
    }

    // '... on overlapping indicator'.
    #[test]
    fn the_paddle_range_value_indicator_supports_the_stroke_color_when_overlapping() {
        with_main_thread_stack(|| {
            value_indicator_stroke_colors(
                Rc::new(
                    crate::slider_value_indicator_shape::PaddleRangeSliderValueIndicatorShape::new(
                    ),
                ),
                true,
            );
        });
    }

    /// Dart's `path(color: black, style: stroke, strokeWidth: 2 * elevation)` of a thumb's
    /// shadow: a `DrawShadow` of `elevation` while shadows show, and the stroked path
    /// `debugDisableShadows` draws instead when it is set.
    fn thumb_shadow_call(elevation: f64) -> PaintCall {
        if waft_painting::debug_disable_shadows() {
            PaintCall::stroked_path(crate::colors::Colors::BLACK, elevation * 2.0)
        } else {
            PaintCall::Shadow {
                elevation: Some(elevation),
                color: None,
            }
        }
    }

    fn range_paints(tester: &WidgetTester, calls: &[PaintCall]) -> bool {
        paints_calls(&tester.painted_ops(range_render(tester)), calls)
    }

    // 'RangeSlider overlay appears correctly for specific thumb interactions'.
    #[test]
    fn the_range_slider_overlay_appears_for_specific_thumb_interactions() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let values = Rc::new(Cell::new(RangeValues::new(50.0, 70.0)));
            const HOVER_COLOR: Color = Color::new(0xffff0000);
            const DRAG_COLOR: Color = Color::new(0xff0000ff);
            let theme = default_theme(&tester);
            tester.pump_widget(enabled_range_slider_app(theme, &values, true, |slider| {
                slider
                    .max(100.0)
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Hovered) {
                                return Some(HOVER_COLOR);
                            }
                            if states.contains(&WidgetState::Dragged) {
                                return Some(DRAG_COLOR);
                            }
                            None
                        },
                    ))
                    .on_change_start(|_app, _values| {})
                    .on_change_end(|_app, _values| {})
            }));
            tester.pump_and_settle();

            // Initial state - no overlay.
            assert!(!paints_circle_of_color(&tester, DRAG_COLOR));

            // Drag start thumb to left.
            let top_thumb_location = slider_rect(&tester).center();
            let mut drag_start_thumb = WidgetTester::touch(1);
            tester.down(&mut drag_start_thumb, top_thumb_location);
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            let at = drag_start_thumb.position + Offset::new(-20.0, 0.0);
            tester.move_to(&mut drag_start_thumb, at, true);
            tester.pump_and_settle();

            // Verify overlay is visible and shadow is visible on single thumb.
            assert!(range_paints(
                &tester,
                &[
                    PaintCall::circle(Some(DRAG_COLOR)),
                    thumb_shadow_call(1.0),
                    thumb_shadow_call(6.0),
                ]
            ));

            // Move back and release.
            let at = drag_start_thumb.position + Offset::new(20.0, 0.0);
            tester.move_to(&mut drag_start_thumb, at, true);
            tester.up(&drag_start_thumb);
            tester.pump_and_settle();

            // Verify overlay and shadow disappears
            assert!(!range_paints(
                &tester,
                &[
                    PaintCall::circle(Some(DRAG_COLOR)),
                    thumb_shadow_call(1.0),
                    thumb_shadow_call(1.0),
                ]
            ));

            // Drag end thumb and return to original position.
            let bottom_thumb_location = slider_rect(&tester).center() + Offset::new(220.0, 0.0);
            let mut drag_end_thumb = WidgetTester::touch(2);
            tester.down(&mut drag_end_thumb, bottom_thumb_location);
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            let at = drag_end_thumb.position + Offset::new(20.0, 0.0);
            tester.move_to(&mut drag_end_thumb, at, true);
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);
            let at = drag_end_thumb.position + Offset::new(-20.0, 0.0);
            tester.move_to(&mut drag_end_thumb, at, true);
            tester.up(&drag_end_thumb);
            tester.pump_and_settle();

            // Verify overlay disappears.
            assert!(!paints_circle_of_color(&tester, DRAG_COLOR));

            // Hover on start thumb.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, top_thumb_location, false);
            tester.pump_and_settle();

            // Verify overlay appears only for start thumb and no shadow is visible.
            assert!(range_paints(
                &tester,
                &[
                    PaintCall::circle(Some(HOVER_COLOR)),
                    thumb_shadow_call(1.0),
                    thumb_shadow_call(1.0),
                ]
            ));
            // 2 thumbs, 1 overlay for hover, and 1 overlay for focus.
            assert_eq!(
                count_calls(
                    &tester.painted_ops(range_render(&tester)),
                    CanvasCall::DrawCircle
                ),
                4
            );

            // Move away from thumb
            tester.move_to(&mut gesture, slider_rect(&tester).top_right(), false);
            tester.pump_and_settle();

            // Verify overlay disappears
            assert!(!paints_circle_of_color(&tester, HOVER_COLOR));
        });
    }

    /// The value indicator box of Dart's cases: `tester.renderObject(find.byType(Overlay))`.
    fn overlay_render(tester: &WidgetTester) -> AnyRenderObject {
        tester
            .render_box_of(tester.find_by_type::<waft_widgets::Overlay>()[0])
            .as_object()
    }

    /// A thumb's shadow as Dart's `path(color: Colors.black)`, of either kind.
    fn any_thumb_shadow() -> PaintCall {
        if waft_painting::debug_disable_shadows() {
            PaintCall::Path {
                color: Some(crate::colors::Colors::BLACK),
                stroke_width: None,
                stroked: None,
            }
        } else {
            PaintCall::Shadow {
                elevation: None,
                color: None,
            }
        }
    }

    // 'Range Slider uses the right theme colors for the right shapes when the value indicators
    // are showing'.
    #[test]
    fn a_range_slider_uses_the_theme_colors_when_the_value_indicators_are_showing() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = themed_theme(&tester);
            let slider_theme = theme.slider_theme.clone();
            let recorded = Rc::new(Cell::new(RangeValues::new(0.5, 0.75)));
            tester.pump_widget(themed_app(
                &theme,
                ThemedArgs {
                    divisions: Some(3),
                    record: Some(Rc::clone(&recorded)),
                    ..ThemedArgs::default()
                },
            ));

            let top_right = slider_rect(&tester).top_right() + Offset::new(-24.0, 0.0);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, top_right);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
            assert_eq!(recorded.get().end, 1.0);

            assert!(paints_calls(
                &tester.painted_ops_deep(overlay_render(&tester)),
                &[
                    any_thumb_shadow(), // shadow
                    any_thumb_shadow(), // shadow
                    PaintCall::path(Some(slider_theme.value_indicator_color.expect("a color"))),
                    PaintCall::Paragraph,
                ]
            ));
            tester.up(&gesture);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
        });
    }

    // 'Default RangeSlider when year2023 is false'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn the_default_range_slider_when_year2023_is_false() {
        with_main_thread_stack(|| {
            use waft_embedder::{RRect, Radius};
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let color_scheme = theme.color_scheme.clone();
            let active_track_color = color_scheme.primary;
            let inactive_track_color = color_scheme.secondary_container();
            let disabled_active_track_color = color_scheme.on_surface.with_opacity(0.38);
            let disabled_inactive_track_color = color_scheme.on_surface.with_opacity(0.12);
            let active_tick_mark_color = color_scheme.on_primary;
            let inactive_tick_mark_color = color_scheme.on_secondary_container();
            let disabled_active_tick_mark_color = color_scheme.on_inverse_surface();
            let disabled_inactive_tick_mark_color = color_scheme.on_surface;
            let thumb_color = color_scheme.primary;
            let disabled_thumb_color = color_scheme.on_surface.with_opacity(0.38);
            let value_indicator_color = color_scheme.inverse_surface();
            let values = RangeValues::new(25.0, 75.0);
            let build_app = |divisions: Option<i32>, enabled: bool| {
                let on_changed: Option<ValueChanged<RangeValues>> = enabled.then(|| {
                    Rc::new(|_app: &mut App, _values: RangeValues| {}) as ValueChanged<RangeValues>
                });
                let mut slider = RangeSlider::new(values, on_changed)
                    .year2023(false)
                    .max(100.0)
                    .labels(RangeLabels::new(
                        format!("{}", values.start.round()),
                        format!("{}", values.end.round()),
                    ));
                if let Some(divisions) = divisions {
                    slider = slider.divisions(divisions);
                }
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(Center::new().child(Theme::new(theme.clone(), slider)))
                        .into_widget(),
                )
            };
            let rrect = |color| PaintCall::rrect(Some(color));
            let circle = |color| PaintCall::circle(Some(color));
            let paints_now =
                |tester: &WidgetTester, calls: &[PaintCall]| range_paints(tester, calls);

            tester.pump_widget(build_app(None, true));

            // Test default track shape.
            let track_outer_corner_radius = Radius::circular(8.0);
            let track_inner_corner_radius = Radius::circular(2.0);
            assert!(paints_now(
                &tester,
                &[
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            24.0,
                            292.0,
                            206.0,
                            308.0,
                            track_outer_corner_radius,
                            track_inner_corner_radius,
                            track_inner_corner_radius,
                            track_outer_corner_radius,
                        ),
                        Some(inactive_track_color),
                    ),
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            594.0,
                            292.0,
                            776.0,
                            308.0,
                            track_inner_corner_radius,
                            track_outer_corner_radius,
                            track_outer_corner_radius,
                            track_inner_corner_radius,
                        ),
                        Some(inactive_track_color),
                    ),
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(218.0, 292.0, 582.0, 308.0, track_inner_corner_radius),
                        Some(active_track_color),
                    ),
                ]
            ));

            // Test default colors for enabled slider.
            assert!(paints_now(
                &tester,
                &[
                    PaintCall::circle(None),
                    PaintCall::circle(None),
                    rrect(thumb_color),
                    rrect(thumb_color),
                ]
            ));
            assert!(!paints_now(
                &tester,
                &[
                    PaintCall::circle(None),
                    PaintCall::circle(None),
                    rrect(disabled_thumb_color),
                    rrect(disabled_thumb_color),
                ]
            ));
            assert!(!paints_now(&tester, &[rrect(disabled_active_track_color)]));
            assert!(!paints_now(
                &tester,
                &[rrect(disabled_inactive_track_color)]
            ));

            // Test defaults colors for discrete slider.
            tester.pump_widget(build_app(Some(4), true));
            assert!(paints_now(
                &tester,
                &[
                    rrect(inactive_track_color),
                    rrect(inactive_track_color),
                    rrect(active_track_color),
                    circle(inactive_tick_mark_color),
                    circle(active_tick_mark_color),
                    circle(inactive_tick_mark_color),
                ]
            ));
            assert!(!paints_now(&tester, &[rrect(disabled_thumb_color)]));
            assert!(!paints_now(&tester, &[rrect(disabled_active_track_color)]));
            assert!(!paints_now(
                &tester,
                &[rrect(disabled_inactive_track_color)]
            ));

            // Test defaults colors for disabled slider.
            tester.pump_widget(build_app(None, false));
            tester.pump_and_settle();
            assert!(paints_now(
                &tester,
                &[
                    rrect(disabled_inactive_track_color),
                    rrect(disabled_inactive_track_color),
                    rrect(disabled_active_track_color),
                    rrect(disabled_thumb_color),
                    rrect(disabled_thumb_color),
                ]
            ));
            assert!(!paints_now(
                &tester,
                &[rrect(thumb_color), rrect(thumb_color)]
            ));
            assert!(!paints_now(&tester, &[rrect(active_track_color)]));
            assert!(!paints_now(&tester, &[rrect(inactive_track_color)]));

            // Test defaults colors for disabled discrete slider.
            tester.pump_widget(build_app(Some(4), false));
            assert!(paints_now(
                &tester,
                &[
                    rrect(disabled_inactive_track_color),
                    rrect(disabled_inactive_track_color),
                    rrect(disabled_active_track_color),
                    circle(disabled_inactive_tick_mark_color),
                    circle(disabled_active_tick_mark_color),
                    circle(disabled_inactive_tick_mark_color),
                    rrect(disabled_thumb_color),
                    rrect(disabled_thumb_color),
                ]
            ));
            assert!(!paints_now(
                &tester,
                &[rrect(thumb_color), rrect(thumb_color)]
            ));
            assert!(!paints_now(&tester, &[rrect(active_track_color)]));
            assert!(!paints_now(&tester, &[rrect(inactive_track_color)]));

            tester.pump_widget(build_app(Some(4), true));
            tester.pump_and_settle();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).top_left());
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
            assert!(paints_calls(
                &tester.painted_ops_deep(overlay_render(&tester)),
                &[PaintCall::Scale, rrect(value_indicator_color)]
            ));
            tester.up(&gesture);
        });
    }

    // slider_theme_test's 'Can customize RangeSlider track gap when year2023 is false'.
    #[test]
    fn the_range_slider_track_gap_can_be_customized_when_year2023_is_false() {
        with_main_thread_stack(|| {
            use waft_embedder::{RRect, Radius};
            let mut tester = tester_on(TargetPlatform::Android);
            let build_range_slider = |tester: &WidgetTester, track_gap: f64| {
                let theme = ThemeData::builder()
                    .slider_theme(SliderThemeData::new().track_gap(track_gap))
                    .build(&tester.cell.borrow());
                #[expect(deprecated, reason = "Dart's case passes `year2023`")]
                let slider = RangeSlider::new(RangeValues::new(25.0, 75.0), noop())
                    .year2023(false)
                    .max(100.0);
                material_app(
                    theme,
                    Material::new()
                        .child(Center::new().child(slider))
                        .into_widget(),
                )
            };
            let outer = Radius::circular(8.0);
            let inner = Radius::circular(2.0);
            let tracks = |left_inactive_right: f64, right_inactive_left: f64| {
                [
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            24.0,
                            292.0,
                            left_inactive_right,
                            308.0,
                            outer,
                            inner,
                            inner,
                            outer,
                        ),
                        None,
                    ),
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            right_inactive_left,
                            292.0,
                            776.0,
                            308.0,
                            inner,
                            outer,
                            outer,
                            inner,
                        ),
                        None,
                    ),
                ]
            };
            let active = |left: f64, right: f64| {
                PaintCall::rrect_at(RRect::from_ltrbr(left, 292.0, right, 308.0, inner), None)
            };

            tester.pump_widget(build_range_slider(&tester, 0.0));
            let mut expected = tracks(212.0, 588.0).to_vec();
            expected.push(active(212.0, 588.0));
            assert!(range_paints(&tester, &expected));

            tester.pump_widget(build_range_slider(&tester, 10.0));
            tester.pump_and_settle();
            let mut expected = tracks(202.0, 598.0).to_vec();
            expected.push(active(222.0, 578.0));
            assert!(range_paints(&tester, &expected));
        });
    }
}

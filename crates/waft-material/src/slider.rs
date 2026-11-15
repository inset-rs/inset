//! Flutter counterpart: material_ui's `slider.dart`, with its part
//! `generated/slider_defaults_m3.g.dart` in `generated/slider_defaults_m3.rs`.
//!
//! [`Slider`] builds the private leaf render object Dart calls `_RenderSlider`, a `RenderBox`
//! with `RelayoutWhenSystemFontsChangeMixin`, and paints its value indicator through an
//! `OverlayPortal` whose child is `_RenderValueIndicator`. The shapes paint with the label
//! painter the render object owns: it lends the painter out of its arena slot for each shape
//! call. The semantics configuration, its increase, decrease and focus actions, the
//! accessibility focus callback and the semantic formatter's use wait with accessibility.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AlwaysStoppedAnimation, Animation, AnimationBehavior, AnimationController, AnyAnimation,
    CurvedAnimation, Curves,
};
use waft_cupertino::{CupertinoColors, CupertinoSlider};
use waft_embedder::{
    Color, FontWeight, Offset, Rect, Size, TargetPlatform, TextDirection, clamp_double,
};
use waft_foundation::{App, Handle, Listener, Timer, ValueChanged};
use waft_gestures::{
    DeviceGestureSettings, DragEndDetails, DragGestureRecognizer, DragStartDetails,
    DragUpdateDetails, GestureArenaTeam, HorizontalDragGestureRecognizer, PointerEvent,
    RecognizerLeaf, TapDownDetails, TapGestureRecognizer, TapUpDetails,
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
    Action, AnyAction, AnyFocusNode, BuildContext, CallbackAction, CompositedTransformFollower,
    CompositedTransformTarget, Directionality, FocusNode, FocusNodeLeaf, FocusableActionDetector,
    GlobalKey, Intent, IntentRef, IntoWidget, KeyRef, LeafRenderObjectWidget, MediaQuery,
    NavigationMode, OverlayPortal, OverlayPortalController, Padding, RenderObjectWidget,
    ShortcutActivatorRef, ShortcutMap, SingleActivator, SizedBox, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef, WidgetState,
    WidgetStateColor, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyAll,
    WidgetStatePropertyRef, WidgetStates, debug_check_has_media_query,
};

use crate::colors::Colors;
use crate::constants::K_RADIAL_REACTION_DURATION;
use crate::debug::debug_check_has_material;
use crate::generated::slider_defaults_m3::SliderDefaultsM3;
use crate::slider_parts::SliderTickMarkShapeRef;
use crate::slider_parts::SliderTrackShapeRef;
use crate::slider_parts::{
    DropSliderValueIndicatorShape, RoundSliderThumbShape, RoundSliderTickMarkShape,
    RoundedRectSliderTrackShape, RoundedRectSliderValueIndicatorShape,
};
use crate::slider_theme::{
    SemanticFormatterCallback, ShowValueIndicator, SliderTheme, SliderThemeData,
    SliderThemeDataGetters,
};
use crate::slider_value_indicator_shape::SliderComponentShapeRef;
use crate::slider_value_indicator_shape::{
    RectangularSliderValueIndicatorShape, RoundSliderOverlayShape,
};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;
use waft_painting::AnyColor;

/// [`Slider`] uses this callback to paint the value indicator on the overlay.
///
/// Since the value indicator is painted on the Overlay; this method paints the
/// value indicator in a `RenderBox` that appears in the `Overlay`.
pub type PaintValueIndicator = Rc<dyn Fn(&mut App, &mut PaintingContext, Offset)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderType {
    Material,
    Adaptive,
}

/// Possible ways for a user to interact with a [`Slider`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SliderInteraction {
    /// Allows the user to interact with a [`Slider`] by tapping or sliding anywhere
    /// on the track.
    ///
    /// Essentially all possible interactions are allowed.
    ///
    /// This is different from [`SliderInteraction::SlideOnly`] as when you try
    /// to slide anywhere other than the thumb, the thumb will move to the first
    /// point of contact.
    TapAndSlide,

    /// Allows the user to interact with a [`Slider`] by only tapping anywhere on
    /// the track.
    ///
    /// Sliding interaction is ignored.
    TapOnly,

    /// Allows the user to interact with a [`Slider`] only by sliding anywhere on
    /// the track.
    ///
    /// Tapping interaction is ignored.
    SlideOnly,

    /// Allows the user to interact with a [`Slider`] only by sliding the thumb.
    ///
    /// Tapping and sliding interactions on the track are ignored.
    SlideThumb,
}

/// A Material Design slider.
///
/// Used to select from a range of values.
///
/// A slider can be used to select from either a continuous or a discrete set of
/// values. The default is to use a continuous range of values from [`min`](Self::min) to
/// [`max`](Self::max). To use discrete values, use a non-null value for
/// [`divisions`](Self::divisions), which indicates the number of discrete intervals. For
/// example, if [`min`](Self::min) is 0.0 and [`max`](Self::max) is 50.0 and
/// [`divisions`](Self::divisions) is 5, then the slider can take on the discrete values 0.0,
/// 10.0, 20.0, 30.0, 40.0, and 50.0.
///
/// The terms for the parts of a slider are:
///
///  * The "thumb", which is a shape that slides horizontally when the user
///    drags it.
///  * The "track", which is the line that the slider thumb slides along.
///  * The "value indicator", which is a shape that pops up when the user
///    is dragging the thumb to indicate the value being selected.
///  * The "active" side of the slider is the side between the thumb and the
///    minimum value.
///  * The "inactive" side of the slider is the side between the thumb and the
///    maximum value.
///
/// The slider will be disabled if [`on_changed`](Self::on_changed) is null or if the range
/// given by [`min`](Self::min)..[`max`](Self::max) is empty (i.e. if [`min`](Self::min) is
/// equal to [`max`](Self::max)).
///
/// The slider widget itself does not maintain any state. Instead, when the state
/// of the slider changes, the widget calls the [`on_changed`](Self::on_changed) callback.
/// Most widgets that use a slider will listen for the [`on_changed`](Self::on_changed)
/// callback and rebuild the slider with a new [`value`](Self::value) to update the visual
/// appearance of the slider. To know when the value starts to change, or when it is done
/// changing, set the optional callbacks [`on_change_start`](Self::on_change_start) and/or
/// [`on_change_end`](Self::on_change_end).
///
/// By default, a slider will be as wide as possible, centered vertically. When
/// given unbounded constraints, it will attempt to make the track 144 pixels
/// wide (with margins on each side) and will shrink-wrap vertically.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// Requires one of its ancestors to be a `MediaQuery` widget. Typically, these
/// are introduced by the `MaterialApp` or `WidgetsApp` widget at the top of
/// your application widget tree.
///
/// To determine how it should be displayed (e.g. colors, thumb shape, etc.),
/// a slider uses the [`SliderThemeData`] available from either a [`SliderTheme`]
/// widget or the `ThemeData::slider_theme` a `Theme` widget above it in the
/// widget tree. You can also override some of the colors with the
/// [`active_color`](Self::active_color) and [`inactive_color`](Self::inactive_color)
/// properties, although more fine-grained control of the look is achieved using a
/// [`SliderThemeData`].
///
/// Dart's constructor asserts run when the slider is turned into a `WidgetRef`
/// (`into_widget`), once every setter has run.
///
/// See also:
///
///  * [`SliderTheme`] and [`SliderThemeData`] for information about controlling
///    the visual appearance of the slider.
///  * `Radio`, for selecting among a set of explicit values.
///  * `Checkbox` and `Switch`, for toggling a particular value on or off.
///  * <https://material.io/design/components/sliders.html>
///  * `MediaQuery`, from which the text scale factor is obtained.
#[derive(Clone)]
pub struct Slider {
    pub key: Option<KeyRef>,

    /// The currently selected value for this slider.
    ///
    /// The slider's thumb is drawn at a position that corresponds to this value.
    pub value: f64,

    /// The secondary track value for this slider.
    ///
    /// If not null, a secondary track using [`secondary_active_color`](Self::secondary_active_color)
    /// color is drawn between the thumb and this value, over the inactive track.
    ///
    /// If less than [`value`](Self::value), then the secondary track is not shown.
    ///
    /// It can be ideal for media scenarios such as showing the buffering progress
    /// while the [`value`](Self::value) shows the play progress.
    pub secondary_track_value: Option<f64>,

    /// Called during a drag when the user is selecting a new value for the slider
    /// by dragging.
    ///
    /// The slider passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the slider with the new
    /// value.
    ///
    /// If null, the slider will be displayed as disabled.
    ///
    /// The callback provided to on_changed should update the state of the parent
    /// `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
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
    /// started selecting a new value by starting a drag or with a tap.
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
    /// completed selecting a new [`value`](Self::value) by ending a drag or a click.
    ///
    /// See also:
    ///
    ///  * [`on_change_start`](Self::on_change_start) for a callback that is called when a
    ///    value change begins.
    pub on_change_end: Option<ValueChanged<f64>>,

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
    /// Typically used with [`label`](Self::label) to show the current discrete value.
    ///
    /// If null, the slider is continuous.
    pub divisions: Option<i32>,

    /// A label to show inside the value indicator above the slider's thumb.
    ///
    /// By default, the label is shown while dragging a discrete slider, where
    /// [`divisions`](Self::divisions) is non-null. It can also be shown for a continuous
    /// slider, where [`divisions`](Self::divisions) is null, by setting
    /// [`show_value_indicator`](Self::show_value_indicator) to
    /// [`ShowValueIndicator::OnlyForContinuous`], [`ShowValueIndicator::OnDrag`], or
    /// [`ShowValueIndicator::AlwaysVisible`].
    ///
    /// If [`show_value_indicator`](Self::show_value_indicator) is null, the ambient
    /// `SliderThemeData::show_value_indicator` determines when the label is shown.
    ///
    /// The label is rendered using the active `ThemeData`'s `TextTheme::body_large`
    /// text style, with the theme data's `ColorScheme::on_primary` color. The
    /// label's text style can be overridden with
    /// `SliderThemeData::value_indicator_text_style`.
    ///
    /// If null, then the value indicator will not be displayed.
    ///
    /// Ignored if this slider is created with [`Slider::adaptive`].
    ///
    /// See also:
    ///
    ///  * `SliderComponentShape` for how to create a custom value indicator
    ///    shape.
    pub label: Option<String>,

    /// The color to use for the portion of the slider track that is active.
    ///
    /// The "active" side of the slider is the side between the thumb and the
    /// minimum value.
    ///
    /// If null, `SliderThemeData::active_track_color` of the ambient
    /// [`SliderTheme`] is used. If that is null, `ColorScheme::primary` of the
    /// surrounding `ThemeData` is used.
    ///
    /// Using a [`SliderTheme`] gives much more fine-grained control over the
    /// appearance of various components of the slider.
    pub active_color: Option<Color>,

    /// The color for the inactive portion of the slider track.
    ///
    /// The "inactive" side of the slider is the side between the thumb and the
    /// maximum value.
    ///
    /// If null, `SliderThemeData::inactive_track_color` of the ambient [`SliderTheme`]
    /// is used. If [`year2023`](Self::year2023) is false and `ThemeData::use_material3` is
    /// true, then `ColorScheme::secondary_container` is used and if
    /// `ThemeData::use_material3` is false, `ColorScheme::primary` with an opacity of 0.24 is
    /// used. Otherwise, `ColorScheme::surface_container_highest` is used.
    ///
    /// Using a [`SliderTheme`] gives much more fine-grained control over the
    /// appearance of various components of the slider.
    ///
    /// Ignored if this slider is created with [`Slider::adaptive`].
    pub inactive_color: Option<Color>,

    /// The color to use for the portion of the slider track between the thumb and
    /// the [`secondary_track_value`](Self::secondary_track_value).
    ///
    /// Defaults to the `SliderThemeData::secondary_active_track_color` of the current
    /// [`SliderTheme`].
    ///
    /// If that is also null, defaults to `ColorScheme::primary` with an
    /// opacity of 0.54.
    ///
    /// Using a [`SliderTheme`] gives much more fine-grained control over the
    /// appearance of various components of the slider.
    ///
    /// Ignored if this slider is created with [`Slider::adaptive`].
    pub secondary_active_color: Option<Color>,

    /// The color of the thumb.
    ///
    /// If this color is null, [`Slider`] will use [`active_color`](Self::active_color), If
    /// [`active_color`](Self::active_color) is also null, [`Slider`] will use
    /// `SliderThemeData::thumb_color`.
    ///
    /// If that is also null, defaults to `ColorScheme::primary`.
    ///
    /// * `CupertinoSlider` will have a white thumb
    ///   (like the native default iOS slider).
    pub thumb_color: Option<Color>,

    /// The highlight color that's typically used to indicate that
    /// the slider thumb is focused, hovered, or dragged.
    ///
    /// If this property is null, [`Slider`] will use [`active_color`](Self::active_color) with
    /// an opacity of 0.12, If null, `SliderThemeData::overlay_color`
    /// will be used.
    ///
    /// If that is also null, If `ThemeData::use_material3` is true,
    /// Slider will use `ColorScheme::primary` with an opacity of 0.08 when
    /// slider thumb is hovered and with an opacity of 0.1 when slider thumb
    /// is focused or dragged, If `ThemeData::use_material3` is false, defaults
    /// to `ColorScheme::primary` with an opacity of 0.12.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Dragged`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of `SliderThemeData::mouse_cursor` is used. If that
    /// is also null, then [`WidgetStateMouseCursor::clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// The callback used to create a semantic value from a slider value.
    ///
    /// Defaults to formatting values as a percentage.
    ///
    /// This is used by accessibility frameworks like TalkBack on Android to
    /// inform users what the currently selected value is with more context.
    ///
    /// Ignored if this slider is created with [`Slider::adaptive`]
    pub semantic_formatter_callback: Option<SemanticFormatterCallback>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<AnyFocusNode>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// Allowed way for the user to interact with the [`Slider`].
    ///
    /// For example, if this is set to [`SliderInteraction::TapOnly`], the user can
    /// interact with the slider only by tapping anywhere on the track. Sliding
    /// will have no effect.
    ///
    /// Defaults to [`SliderInteraction::TapAndSlide`].
    pub allowed_interaction: Option<SliderInteraction>,

    /// Determines the padding around the [`Slider`].
    ///
    /// If specified, this padding overrides the default vertical padding of
    /// the [`Slider`], defaults to the height of the overlay shape, and the
    /// horizontal padding, defaults to the width of the thumb shape or
    /// overlay shape, whichever is larger.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Determines the conditions under which the value indicator is shown.
    ///
    /// If [`show_value_indicator`](Self::show_value_indicator) is null then the
    /// ambient `SliderThemeData::show_value_indicator` is used. If that is also
    /// null, defaults to [`ShowValueIndicator::OnlyForDiscrete`].
    pub show_value_indicator: Option<ShowValueIndicator>,

    /// When true, the [`Slider`] will use the 2023 Material Design 3 appearance.
    /// Defaults to true.
    ///
    /// If this is set to false, the [`Slider`] will use the latest Material Design 3
    /// appearance, which was introduced in December 2023.
    ///
    /// If `ThemeData::use_material3` is false, then this property is ignored.
    ///
    /// Deprecated in Flutter after v3.27.0-0.1.pre; its setter carries the deprecation.
    pub year2023: Option<bool>,

    slider_type: SliderType,
}

impl Slider {
    /// Creates a Material Design slider.
    ///
    /// The slider itself does not maintain any state. Instead, when the state of
    /// the slider changes, the widget calls the [`on_changed`](Self::on_changed) callback.
    /// Most widgets that use a slider will listen for the [`on_changed`](Self::on_changed)
    /// callback and rebuild the slider with a new [`value`](Self::value) to update the visual
    /// appearance of the slider.
    ///
    /// * [`value`](Self::value) determines currently selected value for this slider.
    /// * [`on_changed`](Self::on_changed) is called while the user is selecting a new value
    ///   for the slider.
    /// * [`on_change_start`](Self::on_change_start) is called when the user starts to select a
    ///   new value for the slider.
    /// * [`on_change_end`](Self::on_change_end) is called when the user is done selecting a
    ///   new value for the slider.
    ///
    /// You can override some of the colors with the [`active_color`](Self::active_color) and
    /// [`inactive_color`](Self::inactive_color) properties, although more fine-grained control
    /// of the appearance is achieved using a [`SliderThemeData`].
    pub fn new(value: f64, on_changed: Option<ValueChanged<f64>>) -> Slider {
        Slider::with_type(value, on_changed, SliderType::Material)
    }

    /// Creates an adaptive [`Slider`] based on the target platform, following
    /// Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// Creates a `CupertinoSlider` if the target platform is iOS or macOS, creates a
    /// Material Design slider otherwise.
    ///
    /// If a `CupertinoSlider` is created, the following parameters are ignored:
    /// [`secondary_track_value`](Self::secondary_track_value), [`label`](Self::label),
    /// [`inactive_color`](Self::inactive_color),
    /// [`secondary_active_color`](Self::secondary_active_color),
    /// [`semantic_formatter_callback`](Self::semantic_formatter_callback),
    /// [`show_value_indicator`](Self::show_value_indicator).
    ///
    /// The target platform is based on the current `Theme`: `ThemeData::platform`.
    ///
    /// Dart's `Slider.adaptive` has no `padding` parameter: its `padding` is null, so a
    /// [`padding`](Self::padding) set on it is ignored ([`effective_padding`](Self::effective_padding)).
    pub fn adaptive(value: f64, on_changed: Option<ValueChanged<f64>>) -> Slider {
        Slider::with_type(value, on_changed, SliderType::Adaptive)
    }

    fn with_type(
        value: f64,
        on_changed: Option<ValueChanged<f64>>,
        slider_type: SliderType,
    ) -> Slider {
        Slider {
            key: None,
            value,
            secondary_track_value: None,
            on_changed,
            on_change_start: None,
            on_change_end: None,
            min: 0.0,
            max: 1.0,
            divisions: None,
            label: None,
            active_color: None,
            inactive_color: None,
            secondary_active_color: None,
            thumb_color: None,
            overlay_color: None,
            mouse_cursor: None,
            semantic_formatter_callback: None,
            focus_node: None,
            autofocus: false,
            allowed_interaction: None,
            padding: None,
            show_value_indicator: None,
            year2023: None,
            slider_type,
        }
    }

    /// Dart `Slider(key:)`.
    pub fn key(mut self, key: KeyRef) -> Slider {
        self.key = Some(key);
        self
    }

    /// Dart `Slider(secondaryTrackValue:)`.
    pub fn secondary_track_value(mut self, secondary_track_value: f64) -> Slider {
        self.secondary_track_value = Some(secondary_track_value);
        self
    }

    /// Dart `Slider(onChangeStart:)`.
    pub fn on_change_start(mut self, on_change_start: ValueChanged<f64>) -> Slider {
        self.on_change_start = Some(on_change_start);
        self
    }

    /// Dart `Slider(onChangeEnd:)`.
    pub fn on_change_end(mut self, on_change_end: ValueChanged<f64>) -> Slider {
        self.on_change_end = Some(on_change_end);
        self
    }

    /// Dart `Slider(min:)`.
    pub fn min(mut self, min: f64) -> Slider {
        self.min = min;
        self
    }

    /// Dart `Slider(max:)`.
    pub fn max(mut self, max: f64) -> Slider {
        self.max = max;
        self
    }

    /// Dart `Slider(divisions:)`.
    pub fn divisions(mut self, divisions: i32) -> Slider {
        debug_assert!(divisions > 0);
        self.divisions = Some(divisions);
        self
    }

    /// Dart `Slider(label:)`.
    pub fn label(mut self, label: impl Into<String>) -> Slider {
        self.label = Some(label.into());
        self
    }

    /// Dart `Slider(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> Slider {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `Slider(inactiveColor:)`.
    pub fn inactive_color(mut self, inactive_color: Color) -> Slider {
        self.inactive_color = Some(inactive_color);
        self
    }

    /// Dart `Slider(secondaryActiveColor:)`.
    pub fn secondary_active_color(mut self, secondary_active_color: Color) -> Slider {
        self.secondary_active_color = Some(secondary_active_color);
        self
    }

    /// Dart `Slider(thumbColor:)`.
    pub fn thumb_color(mut self, thumb_color: Color) -> Slider {
        self.thumb_color = Some(thumb_color);
        self
    }

    /// Dart `Slider(overlayColor:)`.
    pub fn overlay_color(mut self, overlay_color: WidgetStatePropertyRef<Option<Color>>) -> Slider {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `Slider(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> Slider {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `Slider(semanticFormatterCallback:)`.
    pub fn semantic_formatter_callback(
        mut self,
        semantic_formatter_callback: SemanticFormatterCallback,
    ) -> Slider {
        self.semantic_formatter_callback = Some(semantic_formatter_callback);
        self
    }

    /// Dart `Slider(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> Slider {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Slider(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Slider {
        self.autofocus = autofocus;
        self
    }

    /// Dart `Slider(allowedInteraction:)`.
    pub fn allowed_interaction(mut self, allowed_interaction: SliderInteraction) -> Slider {
        self.allowed_interaction = Some(allowed_interaction);
        self
    }

    /// Dart `Slider(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Slider {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `Slider(showValueIndicator:)`.
    pub fn show_value_indicator(mut self, show_value_indicator: ShowValueIndicator) -> Slider {
        self.show_value_indicator = Some(show_value_indicator);
        self
    }

    /// Dart `Slider(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 slider appearance. \
                Defaults to true. In the future, this flag will default to false. Use \
                SliderThemeData to customize individual properties. This feature was \
                deprecated after v3.27.0-0.1.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> Slider {
        self.year2023 = Some(year2023);
        self
    }

    /// Dart's `padding` field: what [`padding`](Self::padding) was given, and null for a
    /// slider made with [`Slider::adaptive`], whose constructor sets it so.
    pub fn effective_padding(&self) -> Option<EdgeInsetsGeometry> {
        match self.slider_type {
            SliderType::Material => self.padding,
            SliderType::Adaptive => None,
        }
    }
}

impl Debug for Slider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("Slider");
        properties
            .field("value", &self.value)
            .field("secondary_track_value", &self.secondary_track_value);
        if self.on_changed.is_none() {
            properties.field("on_changed", &"disabled");
        }
        properties
            .field("on_change_start", &self.on_change_start.is_some())
            .field("on_change_end", &self.on_change_end.is_some())
            .field("min", &self.min)
            .field("max", &self.max)
            .field("divisions", &self.divisions)
            .field("label", &self.label)
            .field("active_color", &self.active_color)
            .field("inactive_color", &self.inactive_color)
            .field("secondary_active_color", &self.secondary_active_color)
            .field(
                "semantic_formatter_callback",
                &self.semantic_formatter_callback.is_some(),
            )
            .field("focus_node", &self.focus_node.is_some());
        if self.autofocus {
            properties.field("autofocus", &"autofocus");
        }
        properties.finish()
    }
}

impl StatefulWidget for Slider {
    type State = SliderState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.min <= self.max);
        debug_assert!(
            self.value >= self.min && self.value <= self.max,
            "Value {} is not between minimum {} and maximum {}",
            self.value,
            self.min,
            self.max
        );
        debug_assert!(
            self.secondary_track_value
                .is_none_or(|secondary| secondary >= self.min && secondary <= self.max),
            "SecondaryValue {:?} is not between {} and {}",
            self.secondary_track_value,
            self.min,
            self.max
        );
    }

    fn create_state(&self) -> SliderState {
        SliderState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            overlay_controller: None,
            value_indicator_controller: None,
            enable_controller: None,
            position_controller: None,
            interaction_timer: None,
            render_object_key: GlobalKey::new(),
            action_map: HashMap::new(),
            paint_value_indicator: None,
            dragging: false,
            current_changed_value: None,
            focus_node: None,
            value_indicator_overlay_portal_controller: None,
            focused: false,
            hovering: false,
            layer_link: None,
        }
    }
}

/// Dart's `_SliderState`.
///
/// The fields Dart makes in its initializers (the overlay portal controller, which Dart
/// shows there, and the layer link) are made in `init_state`, the first point with an
/// `App`.
pub struct SliderState {
    state: StateData<Slider>,
    ticker_provider: TickerProviderStateMixinData,
    // Animation controller that is run when the overlay (a.k.a radial reaction)
    // is shown in response to user interaction.
    overlay_controller: Option<Handle<AnimationController>>,
    // Animation controller that is run when the value indicator is being shown
    // or hidden.
    value_indicator_controller: Option<Handle<AnimationController>>,
    // Animation controller that is run when enabling/disabling the slider.
    enable_controller: Option<Handle<AnimationController>>,
    // Animation controller that is run when transitioning between one value
    // and the next on a discrete slider.
    position_controller: Option<Handle<AnimationController>>,
    interaction_timer: Option<Timer>,
    render_object_key: GlobalKey,
    // Action mapping for a focused slider.
    action_map: HashMap<TypeId, AnyAction>,
    // Value Indicator Animation that appears on the Overlay.
    paint_value_indicator: Option<PaintValueIndicator>,
    dragging: bool,
    // For discrete sliders, _handleChanged might receive the same value
    // multiple times. To avoid calling widget.onChanged repeatedly, the
    // value from _handleChanged is temporarily saved here.
    current_changed_value: Option<f64>,
    focus_node: Option<AnyFocusNode>,
    // Always keep the ValueIndicator visible on the Overlay; otherwise, it cannot be updated
    // during the build phase.
    value_indicator_overlay_portal_controller: Option<Handle<OverlayPortalController>>,
    focused: bool,
    hovering: bool,
    layer_link: Option<Handle<LayerLink>>,
}

thread_local! {
    // Keyboard mapping for a focused slider.
    static TRADITIONAL_NAV_SHORTCUT_MAP: ShortcutMap = vec![
        adjust_shortcut(LogicalKeyboardKey::ARROW_UP, SliderAdjustmentType::Up),
        adjust_shortcut(LogicalKeyboardKey::ARROW_DOWN, SliderAdjustmentType::Down),
        adjust_shortcut(LogicalKeyboardKey::ARROW_LEFT, SliderAdjustmentType::Left),
        adjust_shortcut(LogicalKeyboardKey::ARROW_RIGHT, SliderAdjustmentType::Right),
    ];

    // Keyboard mapping for a focused slider when using directional navigation.
    // The vertical inputs are not handled to allow navigating out of the slider.
    static DIRECTIONAL_NAV_SHORTCUT_MAP: ShortcutMap = vec![
        adjust_shortcut(LogicalKeyboardKey::ARROW_LEFT, SliderAdjustmentType::Left),
        adjust_shortcut(LogicalKeyboardKey::ARROW_RIGHT, SliderAdjustmentType::Right),
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

impl SliderState {
    const ENABLE_ANIMATION_DURATION: Duration = Duration::from_millis(75);
    const VALUE_INDICATOR_ANIMATION_DURATION: Duration = Duration::from_millis(100);

    fn overlay_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .overlay_controller
            .expect("init_state made the overlay controller")
    }

    fn value_indicator_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .value_indicator_controller
            .expect("init_state made the value indicator controller")
    }

    fn enable_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .enable_controller
            .expect("init_state made the enable controller")
    }

    fn position_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .position_controller
            .expect("init_state made the position controller")
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).on_changed.is_some()
    }

    fn effective_focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        self.widget(app)
            .focus_node
            .or(app.get(self).focus_node)
            .expect("init_state made a focus node when the widget has none")
    }

    fn handle_changed(self: Handle<Self>, app: &mut App, value: f64) {
        debug_assert!(self.widget(app).on_changed.is_some());
        let lerp_value = self.lerp(app, value);
        if app.get(self).current_changed_value != Some(lerp_value) {
            app.get_mut(self).current_changed_value = Some(lerp_value);
            if lerp_value != self.widget(app).value {
                let on_changed = self
                    .widget(app)
                    .on_changed
                    .clone()
                    .expect("an enabled slider");
                on_changed(app, lerp_value);
            }
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, value: f64) {
        self.set_state(app, |state| {
            state.dragging = true;
        });
        if let Some(on_change_start) = self.widget(app).on_change_start.clone() {
            let lerp_value = self.lerp(app, value);
            on_change_start(app, lerp_value);
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, value: f64) {
        self.set_state(app, |state| {
            state.dragging = false;
        });
        app.get_mut(self).current_changed_value = None;
        if let Some(on_change_end) = self.widget(app).on_change_end.clone() {
            let lerp_value = self.lerp(app, value);
            on_change_end(app, lerp_value);
        }
    }

    fn action_handler(self: Handle<Self>, app: &mut App, intent: &AdjustSliderIntent) {
        let render_object_key = app.get(self).render_object_key.clone();
        let context = render_object_key
            .current_context(app)
            .expect("the slider's render object widget is mounted");
        let directionality = Directionality::of(app, context);
        let should_increase = match intent.adjustment_type {
            SliderAdjustmentType::Up => true,
            SliderAdjustmentType::Down => false,
            SliderAdjustmentType::Left => directionality == TextDirection::Rtl,
            SliderAdjustmentType::Right => directionality == TextDirection::Ltr,
        };

        let slider = context
            .find_render_object(app)
            .and_then(|render_object| render_object.downcast::<RenderSlider>(app))
            .expect("the slider's render object is a _RenderSlider");
        if should_increase {
            slider.increase_action(app)
        } else {
            slider.decrease_action(app)
        }
    }

    fn handle_focus_highlight_changed(self: Handle<Self>, app: &mut App, focused: bool) {
        if focused != app.get(self).focused {
            self.set_state(app, |state| {
                state.focused = focused;
            });
        }
    }

    fn handle_hover_changed(self: Handle<Self>, app: &mut App, hovering: bool) {
        if hovering != app.get(self).hovering {
            self.set_state(app, |state| {
                state.hovering = hovering;
            });
        }
    }

    // Returns a number between min and max, proportional to value, which must
    // be between 0.0 and 1.0.
    fn lerp(self: Handle<Self>, app: &App, value: f64) -> f64 {
        debug_assert!(value >= 0.0);
        debug_assert!(value <= 1.0);
        let widget = self.widget(app);
        if let Some(divisions) = widget.divisions {
            return widget.min
                + (value * f64::from(divisions)).round() * (widget.max - widget.min)
                    / f64::from(divisions);
        }
        value * (widget.max - widget.min) + widget.min
    }

    fn discretize(self: Handle<Self>, app: &App, value: f64) -> f64 {
        debug_assert!(self.widget(app).divisions.is_some());
        debug_assert!((0.0..=1.0).contains(&value));

        let divisions = f64::from(self.widget(app).divisions.expect("a discrete slider"));
        (value * divisions).round() / divisions
    }

    fn convert(self: Handle<Self>, app: &App, value: f64) -> f64 {
        let mut ret = self.unlerp(app, value);
        if self.widget(app).divisions.is_some() {
            ret = self.discretize(app, ret);
        }
        ret
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

    fn build_material_slider(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let theme = Theme::of(app, context);
        let mut slider_theme = SliderTheme::of(app, context);
        let year2023 = widget.year2023.or(slider_theme.year2023).unwrap_or(true);
        let defaults: Box<dyn SliderThemeDataGetters> = if theme.use_material3 {
            if year2023 {
                Box::new(SliderDefaultsM3Year2023::new(context))
            } else {
                Box::new(SliderDefaultsM3::new(context))
            }
        } else {
            Box::new(SliderDefaultsM2::new(context))
        };

        // If the widget has active or inactive colors specified, then we plug them
        // in to the slider theme as best we can. If the developer wants more
        // control than that, then they need to use a SliderTheme. The default
        // colors come from the ThemeData.colorScheme. These colors, along with
        // the default shapes and text styles are aligned to the Material
        // Guidelines.

        const DEFAULT_SHOW_VALUE_INDICATOR: ShowValueIndicator =
            ShowValueIndicator::OnlyForDiscrete;
        const DEFAULT_ALLOWED_INTERACTION: SliderInteraction = SliderInteraction::TapAndSlide;

        let enabled = self.enabled(app);
        let (hovering, focused, dragging) = {
            let state = app.get(self);
            (state.hovering, state.focused, state.dragging)
        };
        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }
        if hovering {
            states.insert(WidgetState::Hovered);
        }
        if focused {
            states.insert(WidgetState::Focused);
        }
        if dragging {
            states.insert(WidgetState::Dragged);
        }

        // The value indicator's color is not the same as the thumb and active track
        // (which can be defined by activeColor) if the
        // RectangularSliderValueIndicatorShape is used. In all other cases, the
        // value indicator is assumed to be the same as the active color.
        let value_indicator_shape = slider_theme
            .value_indicator_shape
            .clone()
            .or_else(|| defaults.value_indicator_shape(app))
            .expect("the defaults have a value indicator shape");
        let value_indicator_color = if value_indicator_shape
            .as_any()
            .is::<RectangularSliderValueIndicatorShape>()
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
                <dyn WidgetStateProperty<Color>>::resolve_as(&defaults.overlay_color(app), &states)
                    .map(|color| color.color())
            });

        let mut value_indicator_text_style = slider_theme
            .value_indicator_text_style
            .clone()
            .or_else(|| defaults.value_indicator_text_style(app))
            .expect("the defaults have a value indicator text style");
        if MediaQuery::bold_text_of(app, context) {
            value_indicator_text_style = value_indicator_text_style
                .merge(Some(&TextStyle::new().font_weight(FontWeight::BOLD)));
        }

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
            secondary_active_track_color: widget
                .secondary_active_color
                .or(slider_theme.secondary_active_track_color)
                .or_else(|| defaults.secondary_active_track_color(app)),
            disabled_active_track_color: slider_theme
                .disabled_active_track_color
                .or_else(|| defaults.disabled_active_track_color(app)),
            disabled_inactive_track_color: slider_theme
                .disabled_inactive_track_color
                .or_else(|| defaults.disabled_inactive_track_color(app)),
            disabled_secondary_active_track_color: slider_theme
                .disabled_secondary_active_track_color
                .or_else(|| defaults.disabled_secondary_active_track_color(app)),
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
                .thumb_color
                .or(widget.active_color)
                .or(slider_theme.thumb_color)
                .or_else(|| defaults.thumb_color(app)),
            disabled_thumb_color: slider_theme
                .disabled_thumb_color
                .or_else(|| defaults.disabled_thumb_color(app)),
            overlay_color: effective_overlay_color
                .map(Into::into)
                .or_else(|| slider_theme.overlay_color.clone()),
            value_indicator_color: Some(value_indicator_color),
            track_shape: slider_theme
                .track_shape
                .clone()
                .or_else(|| defaults.track_shape(app)),
            tick_mark_shape: slider_theme
                .tick_mark_shape
                .clone()
                .or_else(|| defaults.tick_mark_shape(app)),
            thumb_shape: slider_theme
                .thumb_shape
                .clone()
                .or_else(|| defaults.thumb_shape(app)),
            overlay_shape: slider_theme
                .overlay_shape
                .clone()
                .or_else(|| defaults.overlay_shape(app)),
            value_indicator_shape: Some(value_indicator_shape),
            show_value_indicator: Some(
                widget
                    .show_value_indicator
                    .or(slider_theme.show_value_indicator)
                    .unwrap_or(DEFAULT_SHOW_VALUE_INDICATOR),
            ),
            value_indicator_text_style: Some(value_indicator_text_style),
            padding: widget.effective_padding().or(slider_theme.padding),
            thumb_size: slider_theme
                .thumb_size
                .clone()
                .or_else(|| defaults.thumb_size(app)),
            track_gap: slider_theme.track_gap.or_else(|| defaults.track_gap(app)),
            ..slider_theme
        };
        let effective_mouse_cursor: MouseCursorRef =
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget.mouse_cursor, &states)
                .or_else(|| {
                    slider_theme
                        .mouse_cursor
                        .as_ref()
                        .and_then(|mouse_cursor| mouse_cursor.resolve(&states))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::clickable().resolve(&states));
        let effective_allowed_interaction = widget
            .allowed_interaction
            .or(slider_theme.allowed_interaction)
            .unwrap_or(DEFAULT_ALLOWED_INTERACTION);

        // This size is used as the max bounds for the painting of the value
        // indicators It must be kept in sync with the function with the same name
        // in range_slider.dart.
        let screen_size = MediaQuery::size_of(app, context);

        // Dart's `handleDidGainAccessibilityFocus` (Windows) is a semantics callback and
        // waits with accessibility.

        let shortcut_map = match MediaQuery::navigation_mode_of(app, context) {
            NavigationMode::Directional => DIRECTIONAL_NAV_SHORTCUT_MAP.with(Clone::clone),
            NavigationMode::Traditional => TRADITIONAL_NAV_SHORTCUT_MAP.with(Clone::clone),
        };

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
        let text_scaler = if theme.use_material3 {
            // TODO(tahatesser): This is an eye-balled value.
            // This needs to be updated when accessibility
            // guidelines are available on the material specs page
            // https://m3.material.io/components/sliders/accessibility.
            MediaQuery::text_scaler_of(app, context).clamp(0.0, 1.3)
        } else {
            MediaQuery::text_scaler_of(app, context)
        };
        let effective_text_scale = text_scaler.scale(font_size_to_scale) / font_size_to_scale;

        let on_changed: Option<ValueChanged<f64>> =
            (widget.on_changed.is_some() && widget.max > widget.min).then(|| {
                Rc::new(move |app: &mut App, value: f64| self.handle_changed(app, value))
                    as ValueChanged<f64>
            });
        let on_change_start: ValueChanged<f64> =
            Rc::new(move |app: &mut App, value: f64| self.handle_drag_start(app, value));
        let on_change_end: ValueChanged<f64> =
            Rc::new(move |app: &mut App, value: f64| self.handle_drag_end(app, value));
        let layer_link = app
            .get(self)
            .layer_link
            .expect("init_state made the layer link");
        let show_value_indicator = slider_theme
            .show_value_indicator
            .expect("the theme's show_value_indicator was filled in above");
        let render_object_widget = SliderRenderObjectWidget {
            key: Some(app.get(self).render_object_key.clone().into()),
            value: self.convert(app, widget.value),
            secondary_track_value: widget
                .secondary_track_value
                .map(|secondary_track_value| self.convert(app, secondary_track_value)),
            divisions: widget.divisions,
            label: widget.label.clone(),
            slider_theme: slider_theme.clone(),
            text_scale_factor: effective_text_scale,
            screen_size,
            on_changed,
            on_change_start,
            on_change_end,
            state: self,
            has_focus: focused,
            hovering,
            allowed_interaction: effective_allowed_interaction,
        };
        let mut result = CompositedTransformTarget::new(layer_link)
            .child(render_object_widget)
            .into_widget();

        let padding = widget.effective_padding().or(slider_theme.padding);
        if let Some(padding) = padding {
            result = Padding::new(padding).child(result).into_widget();
        }
        let controller = app
            .get(self)
            .value_indicator_overlay_portal_controller
            .expect("init_state made the overlay portal controller");
        result = OverlayPortal::new(controller, move |app: &mut App, _context: BuildContext| {
            self.build_value_indicator(app, show_value_indicator)
        })
        .child(result)
        .into_widget();

        let focus_node = self.effective_focus_node(app);
        FocusableActionDetector::new(result)
            .actions(app.get(self).action_map.clone())
            .shortcuts(shortcut_map)
            .focus_node(focus_node)
            .autofocus(widget.autofocus)
            .enabled(enabled)
            .on_show_focus_highlight(move |app, focused| {
                self.handle_focus_highlight_changed(app, focused)
            })
            .on_show_hover_highlight(move |app, hovering| self.handle_hover_changed(app, hovering))
            .mouse_cursor(effective_mouse_cursor)
            .include_focus_semantics(false)
            .into_widget()
    }

    fn build_cupertino_slider(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app);
        // The render box of a slider has a fixed height but takes up the available
        // width. Wrapping the [CupertinoSlider] in this manner will help maintain
        // the same size.
        let mut slider = CupertinoSlider::new(widget.value, widget.on_changed.clone())
            .min(widget.min)
            .max(widget.max);
        if let Some(on_change_start) = widget.on_change_start.clone() {
            slider = slider.on_change_start(on_change_start);
        }
        if let Some(on_change_end) = widget.on_change_end.clone() {
            slider = slider.on_change_end(on_change_end);
        }
        if let Some(divisions) = widget.divisions {
            slider = slider.divisions(divisions);
        }
        if let Some(active_color) = widget.active_color {
            slider = slider.active_color(active_color);
        }
        slider = match widget.thumb_color {
            Some(thumb_color) => slider.thumb_color(thumb_color),
            None => slider.thumb_color(CupertinoColors::WHITE),
        };
        SizedBox::new()
            .width(f64::INFINITY)
            .child(slider)
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
            .expect("init_state made the layer link");
        let value_indicator = || {
            CompositedTransformFollower::new(layer_link)
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

impl TickerProviderStateMixin for SliderState {
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
impl TickerProviderObject for SliderState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SliderState {
    type Widget = Slider;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let portal_controller =
            OverlayPortalController::new(app, Some("Slider ValueIndicator".to_string()));
        portal_controller.show(app);
        app.get_mut(self).value_indicator_overlay_portal_controller = Some(portal_controller);
        let layer_link = LayerLink::new(app);
        app.get_mut(self).layer_link = Some(layer_link);

        let controller = |app: &mut App, duration: Duration| {
            AnimationController::create(
                app,
                None,
                Some(duration),
                None,
                0.0,
                1.0,
                AnimationBehavior::Normal,
                self,
            )
        };
        let overlay_controller = controller(app, K_RADIAL_REACTION_DURATION);
        let value_indicator_controller = controller(app, Self::VALUE_INDICATOR_ANIMATION_DURATION);
        let enable_controller = controller(app, Self::ENABLE_ANIMATION_DURATION);
        let position_controller = controller(app, Duration::ZERO);
        let this = app.get_mut(self);
        this.overlay_controller = Some(overlay_controller);
        this.value_indicator_controller = Some(value_indicator_controller);
        this.enable_controller = Some(enable_controller);
        this.position_controller = Some(position_controller);
        let enabled = self.widget(app).on_changed.is_some();
        enable_controller.set_value(app, if enabled { 1.0 } else { 0.0 });
        let value = self.convert(app, self.widget(app).value);
        position_controller.set_value(app, value);
        let adjust = CallbackAction::<AdjustSliderIntent>::new(
            app,
            Rc::new(move |app: &mut App, intent: &AdjustSliderIntent| {
                self.action_handler(app, intent);
                None
            }),
        );
        app.get_mut(self)
            .action_map
            .insert(TypeId::of::<AdjustSliderIntent>(), adjust.as_action());
        if self.widget(app).focus_node.is_none() {
            // Only create a new node if the widget doesn't have one.
            if app.get(self).focus_node.is_none() {
                let focus_node = FocusNode::new(app).as_node();
                app.get_mut(self).focus_node = Some(focus_node);
            }
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
            self.position_controller(app),
        ];
        for controller in controllers {
            controller.dispose(app);
            app.destroy(controller);
        }
        if let Some(focus_node) = app.get(self).focus_node {
            focus_node.dispose(app);
        }
        for action in std::mem::take(&mut app.get_mut(self).action_map).into_values() {
            app.destroy(action.id());
        }
        TickerProviderStateMixin::dispose(self, app);
        // Dart lets the portal controller and the link go with the state.
        let this = app.get_mut(self);
        let (portal_controller, layer_link) = (
            this.value_indicator_overlay_portal_controller.take(),
            this.layer_link.take(),
        );
        if let Some(portal_controller) = portal_controller {
            app.destroy(portal_controller);
        }
        if let Some(layer_link) = layer_link {
            app.destroy(layer_link);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_media_query(app, context));

        match self.widget(app).slider_type {
            SliderType::Material => self.build_material_slider(app, context),
            SliderType::Adaptive => {
                let theme = Theme::of(app, context);
                match theme.platform {
                    TargetPlatform::Android
                    | TargetPlatform::Fuchsia
                    | TargetPlatform::Linux
                    | TargetPlatform::Windows => self.build_material_slider(app, context),
                    TargetPlatform::IOS | TargetPlatform::MacOS => self.build_cupertino_slider(app),
                }
            }
        }
    }
}

/// Dart's `_SliderRenderObjectWidget`.
struct SliderRenderObjectWidget {
    key: Option<KeyRef>,
    value: f64,
    secondary_track_value: Option<f64>,
    divisions: Option<i32>,
    label: Option<String>,
    slider_theme: SliderThemeData,
    text_scale_factor: f64,
    screen_size: Size,
    on_changed: Option<ValueChanged<f64>>,
    on_change_start: ValueChanged<f64>,
    on_change_end: ValueChanged<f64>,
    state: Handle<SliderState>,
    has_focus: bool,
    hovering: bool,
    allowed_interaction: SliderInteraction,
}

impl Debug for SliderRenderObjectWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SliderRenderObjectWidget")
            .field("value", &self.value)
            .field("divisions", &self.divisions)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for SliderRenderObjectWidget {
    type RenderObject = RenderSlider;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        let platform = Theme::of(app, context).platform;
        let gesture_settings = MediaQuery::gesture_settings_of(app, context);
        RenderSlider::new(
            app,
            self.value,
            self.secondary_track_value,
            self.divisions,
            self.label.clone(),
            self.slider_theme.clone(),
            self.text_scale_factor,
            self.screen_size,
            platform,
            self.on_changed.clone(),
            Some(self.on_change_start.clone()),
            Some(self.on_change_end.clone()),
            self.state,
            text_direction,
            self.has_focus,
            self.hovering,
            gesture_settings,
            self.allowed_interaction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderSlider>,
    ) {
        let text_direction = Directionality::of(app, context);
        let platform = Theme::of(app, context).platform;
        let gesture_settings = MediaQuery::gesture_settings_of(app, context);
        // We should update the `divisions` ahead of `value`, because the `value`
        // setter dependent on the `divisions`.
        render_object.set_divisions(app, self.divisions);
        render_object.set_value(app, self.value);
        render_object.set_secondary_track_value(app, self.secondary_track_value);
        render_object.set_label(app, self.label.clone());
        render_object.set_slider_theme(app, self.slider_theme.clone());
        render_object.set_text_scale_factor(app, self.text_scale_factor);
        render_object.set_screen_size(app, self.screen_size);
        render_object.set_on_changed(app, self.on_changed.clone());
        render_object.get_mut(app).on_change_start = Some(self.on_change_start.clone());
        render_object.get_mut(app).on_change_end = Some(self.on_change_end.clone());
        render_object.set_text_direction(app, text_direction);
        render_object.set_platform(app, platform);
        render_object.set_has_focus(app, self.has_focus);
        render_object.set_hovering(app, self.hovering);
        render_object.set_gesture_settings(app, Some(gesture_settings));
        render_object.set_allowed_interaction(app, self.allowed_interaction);
        // Ticker provider cannot change since there's a 1:1 relationship between
        // the _SliderRenderObjectWidget object and the _SliderState object.
    }
}

impl LeafRenderObjectWidget for SliderRenderObjectWidget {}

/// Dart's `_RenderSlider extends RenderBox with RelayoutWhenSystemFontsChangeMixin`.
pub(crate) struct RenderSlider {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    system_fonts: RelayoutWhenSystemFontsChangeData,
    value: f64,
    secondary_track_value: Option<f64>,
    divisions: Option<i32>,
    label: Option<String>,
    slider_theme: SliderThemeData,
    text_scale_factor: f64,
    screen_size: Size,
    platform: TargetPlatform,
    on_changed: Option<ValueChanged<f64>>,
    on_change_start: Option<ValueChanged<f64>>,
    on_change_end: Option<ValueChanged<f64>>,
    state: Handle<SliderState>,
    text_direction: TextDirection,
    has_focus: bool,
    hovering: bool,
    hovering_thumb: bool,
    allowed_interaction: SliderInteraction,
    overlay_animation: Handle<CurvedAnimation>,
    value_indicator_animation: Handle<CurvedAnimation>,
    enable_animation: Handle<CurvedAnimation>,
    label_painter: TextPainter,
    drag: Handle<HorizontalDragGestureRecognizer>,
    tap: Handle<TapGestureRecognizer>,
    active: bool,
    current_drag_value: f64,
    overlay_rect: Option<Rect>,
}

impl RenderSlider {
    const POSITION_ANIMATION_DURATION: Duration = Duration::from_millis(75);
    const MINIMUM_INTERACTION_TIME: Duration = Duration::from_millis(500);

    // This value is the touch target, 48, multiplied by 3.
    const MIN_PREFERRED_TRACK_WIDTH: f64 = 144.0;

    // Buffer to account for the internal padding of standard Material value indicator shapes,
    // preventing them from bleeding off the screen edges.
    //
    // The value 64.0 is a heuristic that covers the minimum size of the shape
    // (padding + minimum label width) at a text scale factor of roughly 2.0.
    const K_VALUE_INDICATOR_HORIZONTAL_BUFFER: f64 = 64.0;

    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor parameters, in its order"
    )]
    fn new(
        app: &mut App,
        value: f64,
        secondary_track_value: Option<f64>,
        divisions: Option<i32>,
        label: Option<String>,
        slider_theme: SliderThemeData,
        text_scale_factor: f64,
        screen_size: Size,
        platform: TargetPlatform,
        on_changed: Option<ValueChanged<f64>>,
        on_change_start: Option<ValueChanged<f64>>,
        on_change_end: Option<ValueChanged<f64>>,
        state: Handle<SliderState>,
        text_direction: TextDirection,
        has_focus: bool,
        hovering: bool,
        gesture_settings: DeviceGestureSettings,
        allowed_interaction: SliderInteraction,
    ) -> RenderHandle<RenderSlider> {
        debug_assert!((0.0..=1.0).contains(&value));
        debug_assert!(secondary_track_value.is_none_or(|value| (0.0..=1.0).contains(&value)));
        let drag = HorizontalDragGestureRecognizer::new(app);
        let tap = TapGestureRecognizer::new(app);
        let overlay_animation = CurvedAnimation::create(
            app,
            state.overlay_controller(app).as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        let value_indicator_animation = CurvedAnimation::create(
            app,
            state.value_indicator_controller(app).as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        let enable_animation = CurvedAnimation::create(
            app,
            state.enable_controller(app).as_animation(),
            Curves::ease_in_out(),
            None,
        );
        let this = RenderHandle::new_box(
            app,
            RenderSlider {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                system_fonts: RelayoutWhenSystemFontsChangeData::new(),
                value,
                secondary_track_value,
                divisions,
                label,
                slider_theme,
                text_scale_factor,
                screen_size,
                platform,
                on_changed,
                on_change_start,
                on_change_end,
                state,
                text_direction,
                has_focus,
                hovering,
                hovering_thumb: false,
                allowed_interaction,
                overlay_animation,
                value_indicator_animation,
                enable_animation,
                label_painter: TextPainter::new(),
                drag,
                tap,
                active: false,
                current_drag_value: 0.0,
                overlay_rect: None,
            },
        );
        this.update_label_painter(app);
        let team = GestureArenaTeam::new(app);
        drag.set_team(app, team);
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
        self.thumb_shape(app)
            .get_preferred_size(self.is_interactive(app), self.is_discrete(app))
            .height()
    }

    fn overlay_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.overlay_shape(app)
            .get_preferred_size(self.is_interactive(app), self.is_discrete(app))
            .height()
    }

    fn slider_part_sizes(self: RenderHandle<Self>, app: &App) -> [Size; 3] {
        let (is_interactive, is_discrete) = (self.is_interactive(app), self.is_discrete(app));
        let slider_theme = &self.get(app).slider_theme;
        [
            Size::new(
                self.overlay_shape(app)
                    .get_preferred_size(is_interactive, is_discrete)
                    .width(),
                if slider_theme.padding.is_some() {
                    self.thumb_size_height(app)
                } else {
                    self.overlay_height(app)
                },
            ),
            self.thumb_shape(app)
                .get_preferred_size(is_interactive, is_discrete),
            slider_theme
                .tick_mark_shape
                .as_ref()
                .expect("the slider fills in its tick mark shape")
                .get_preferred_size(slider_theme, is_interactive),
        ]
    }

    fn min_preferred_track_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app)
            .slider_theme
            .track_height
            .expect("the slider fills in its track height")
    }

    fn thumb_shape(self: RenderHandle<Self>, app: &App) -> Rc<dyn crate::SliderComponentShape> {
        self.get(app)
            .slider_theme
            .thumb_shape
            .clone()
            .expect("the slider fills in its thumb shape")
    }

    fn overlay_shape(self: RenderHandle<Self>, app: &App) -> Rc<dyn crate::SliderComponentShape> {
        self.get(app)
            .slider_theme
            .overlay_shape
            .clone()
            .expect("the slider fills in its overlay shape")
    }

    fn track_shape(self: RenderHandle<Self>, app: &App) -> Rc<dyn crate::SliderTrackShape> {
        self.get(app)
            .slider_theme
            .track_shape
            .clone()
            .expect("the slider fills in its track shape")
    }

    // This rect is used in gesture calculations, where the gesture coordinates
    // are relative to the sliders origin. Therefore, the offset is passed as
    // (0,0).
    fn track_rect(self: RenderHandle<Self>, app: &App) -> Rect {
        let slider_theme = &self.get(app).slider_theme;
        self.track_shape(app).get_preferred_rect(
            app,
            self.as_box(),
            Offset::ZERO,
            slider_theme,
            None,
            Some(false),
        )
    }

    // Tick marks and the thumb are painted in an adjusted coordinate space that
    // reserves half a track height of padding on each end for discrete rounded
    // tracks, so gesture math needs to account for that same padding to stay
    // aligned with the visual tick positions.
    fn discrete_rounded_track_padding(self: RenderHandle<Self>, app: &App) -> f64 {
        if self.is_discrete(app) && self.track_shape(app).is_rounded() {
            self.track_rect(app).height()
        } else {
            0.0
        }
    }

    fn is_interactive(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).on_changed.is_some()
    }

    fn is_discrete(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app)
            .divisions
            .is_some_and(|divisions| divisions > 0)
    }

    fn value(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).value
    }

    fn set_value(self: RenderHandle<Self>, app: &mut App, new_value: f64) {
        debug_assert!((0.0..=1.0).contains(&new_value));
        let converted_value = if self.is_discrete(app) {
            self.discretize(app, new_value)
        } else {
            new_value
        };
        if converted_value == self.get(app).value {
            return;
        }
        self.get_mut(app).value = converted_value;
        let position_controller = self.get(app).state.position_controller(app);
        if self.is_discrete(app) {
            // Reset the duration to match the distance that we're traveling, so that
            // whatever the distance, we still do it in _positionAnimationDuration,
            // and if we get re-targeted in the middle, it still takes that long to
            // get to the new location.
            let distance = (converted_value - position_controller.value(app)).abs();
            app.get_mut(position_controller).duration = Some(if distance != 0.0 {
                Self::POSITION_ANIMATION_DURATION.mul_f64(1.0 / distance)
            } else {
                Duration::ZERO
            });
            position_controller.animate_to(app, converted_value, None, Curves::ease_in_out());
        } else {
            position_controller.set_value(app, converted_value);
        }
        // `markNeedsSemanticsUpdate()` waits with accessibility.
    }

    fn set_secondary_track_value(self: RenderHandle<Self>, app: &mut App, new_value: Option<f64>) {
        debug_assert!(new_value.is_none_or(|value| (0.0..=1.0).contains(&value)));
        if new_value == self.get(app).secondary_track_value {
            return;
        }
        self.get_mut(app).secondary_track_value = new_value;
        self.mark_needs_paint(app);
        // `markNeedsSemanticsUpdate()` waits with accessibility.
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
        let (drag, tap) = (self.get(app).drag, self.get(app).tap);
        drag.set_gesture_settings(app, gesture_settings);
        tap.set_gesture_settings(app, gesture_settings);
    }

    /// Dart's setter marks the semantics dirty when the platform changes; that waits with
    /// accessibility, so it only stores the platform.
    fn set_platform(self: RenderHandle<Self>, app: &mut App, value: TargetPlatform) {
        if self.get(app).platform == value {
            return;
        }
        self.get_mut(app).platform = value;
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

    fn label(self: RenderHandle<Self>, app: &App) -> Option<&str> {
        self.get(app).label.as_deref()
    }

    fn set_label(self: RenderHandle<Self>, app: &mut App, value: Option<String>) {
        if value == self.get(app).label {
            return;
        }
        self.get_mut(app).label = value;
        self.update_label_painter(app);
    }

    fn slider_theme(self: RenderHandle<Self>, app: &App) -> &SliderThemeData {
        &self.get(app).slider_theme
    }

    fn set_slider_theme(self: RenderHandle<Self>, app: &mut App, value: SliderThemeData) {
        if value == self.get(app).slider_theme {
            return;
        }
        self.get_mut(app).slider_theme = value;
        self.update_label_painter(app);
    }

    fn text_scale_factor(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).text_scale_factor
    }

    fn set_text_scale_factor(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value == self.get(app).text_scale_factor {
            return;
        }
        self.get_mut(app).text_scale_factor = value;
        self.update_label_painter(app);
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

    fn on_changed(self: RenderHandle<Self>, app: &App) -> Option<ValueChanged<f64>> {
        self.get(app).on_changed.clone()
    }

    /// Dart's setter returns early when the callback is the same object; a closure has no
    /// identity here, so it always stores the new one and acts only when interactivity flips,
    /// as Dart's does. Marking the semantics dirty waits with accessibility.
    fn set_on_changed(self: RenderHandle<Self>, app: &mut App, value: Option<ValueChanged<f64>>) {
        let was_interactive = self.is_interactive(app);
        self.get_mut(app).on_changed = value;
        if was_interactive != self.is_interactive(app) {
            let enable_controller = self.get(app).state.enable_controller(app);
            if self.is_interactive(app) {
                enable_controller.forward(app, None);
            } else {
                enable_controller.reverse(app, None);
            }
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
        self.update_label_painter(app);
    }

    /// True if this slider has the input focus.
    fn has_focus(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_focus
    }

    fn set_has_focus(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).has_focus {
            return;
        }
        self.get_mut(app).has_focus = value;
        self.update_for_focus(app, value);
        // `markNeedsSemanticsUpdate()` waits with accessibility.
    }

    /// True if this slider is being hovered over by a pointer.
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

    /// True if the slider is interactive and the slider thumb is being
    /// hovered over by a pointer.
    fn hovering_thumb(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).hovering_thumb
    }

    fn set_hovering_thumb(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.get(app).hovering_thumb {
            return;
        }
        self.get_mut(app).hovering_thumb = value;
        let hovering = self.hovering(app);
        self.update_for_hover(app, hovering);
    }

    fn allowed_interaction(self: RenderHandle<Self>, app: &App) -> SliderInteraction {
        self.get(app).allowed_interaction
    }

    /// Dart's setter marks the semantics dirty; that waits with accessibility, so it only
    /// stores the value.
    fn set_allowed_interaction(self: RenderHandle<Self>, app: &mut App, value: SliderInteraction) {
        if value == self.get(app).allowed_interaction {
            return;
        }
        self.get_mut(app).allowed_interaction = value;
    }

    fn update_for_focus(self: RenderHandle<Self>, app: &mut App, focused: bool) {
        let state = self.get(app).state;
        if focused {
            state.overlay_controller(app).forward(app, None);
            if self.should_show_value_indicator_when_dragged(app) {
                state.value_indicator_controller(app).forward(app, None);
            }
        } else {
            state.overlay_controller(app).reverse(app, None);
            if self.should_show_value_indicator_when_dragged(app) {
                state.value_indicator_controller(app).reverse(app, None);
            }
        }
    }

    fn update_for_hover(self: RenderHandle<Self>, app: &mut App, hovered: bool) {
        let state = self.get(app).state;
        // Only show overlay when pointer is hovering the thumb.
        if hovered && self.hovering_thumb(app) {
            state.overlay_controller(app).forward(app, None);
        } else {
            // Only remove overlay when Slider is inactive and unfocused.
            if !self.get(app).active && !self.has_focus(app) {
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

    fn adjustment_unit(self: RenderHandle<Self>, app: &App) -> f64 {
        match self.get(app).platform {
            // Matches iOS implementation of material slider.
            TargetPlatform::IOS | TargetPlatform::MacOS => 0.1,
            // Matches Android implementation of material slider.
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => 0.05,
        }
    }

    fn update_label_painter(self: RenderHandle<Self>, app: &mut App) {
        if let Some(label) = self.label(app).map(str::to_owned) {
            // Reserve space for the bubble's internal padding and screen margins.
            let screen_size = self.screen_size(app);
            let safe_max_width =
                (screen_size.width() - Self::K_VALUE_INDICATOR_HORIZONTAL_BUFFER).max(0.0);
            let mut text = TextSpan::new().text(label);
            if let Some(style) = self.slider_theme(app).value_indicator_text_style.clone() {
                text = text.style(style);
            }
            let text_direction = self.text_direction(app);
            let text_scale_factor = self.text_scale_factor(app);
            let fonts = PaintingBinding::instance(app).fonts(app);
            let (this, fonts) = app.get_disjoint_mut(self.handle(), fonts);
            let label_painter = &mut this.label_painter;
            label_painter.set_text(Some(Rc::new(text)));
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
        } else {
            self.get_mut(app).label_painter.set_text(None);
        }
        // Changing the textDirection can result in the layout changing, because the
        // bidi algorithm might line up the glyphs differently which can result in
        // different ligatures, different shapes, etc. So we always markNeedsLayout.
        RenderBox::mark_needs_layout(self, app);
    }

    /// Lends the label painter to a shape's `paint`, which Dart hands the painter object
    /// itself: it lives in this render object's arena slot, which the shape's `App` reaches.
    fn with_label_painter<R>(
        self: RenderHandle<Self>,
        app: &mut App,
        paint: impl FnOnce(&mut App, &mut TextPainter) -> R,
    ) -> R {
        let mut label_painter =
            std::mem::replace(&mut self.get_mut(app).label_painter, TextPainter::new());
        let result = paint(app, &mut label_painter);
        self.get_mut(app).label_painter = label_painter;
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
        let padding = self.discrete_rounded_track_padding(app);
        let track_rect = self.track_rect(app);
        let adjusted_width = track_rect.width() - padding;
        // Invert the padded track-geometry formula (see _discreteRoundedTrackPadding)
        // so tap coordinates align with the visual tick positions. Without this, the
        // snap midpoints in tap space diverge from the visual midpoints between tick
        // marks, biasing lower-half taps toward the wrong (lower) division.
        let visual_position = if adjusted_width <= 0.0 {
            0.5
        } else {
            (local_dx - track_rect.left - padding / 2.0) / adjusted_width
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

    fn start_interaction(self: RenderHandle<Self>, app: &mut App, global_position: Offset) {
        let state = self.get(app).state;
        if !state.mounted(app) {
            return;
        }
        if !self.get(app).active && self.is_interactive(app) {
            match self.allowed_interaction(app) {
                SliderInteraction::TapAndSlide | SliderInteraction::TapOnly => {
                    let current_drag_value =
                        self.get_value_from_global_position(app, global_position);
                    let this = self.get_mut(app);
                    this.active = true;
                    this.current_drag_value = current_drag_value;
                }
                SliderInteraction::SlideThumb => {
                    if self.is_pointer_on_overlay(app, global_position) {
                        let value = self.value(app);
                        let this = self.get_mut(app);
                        this.active = true;
                        this.current_drag_value = value;
                    }
                }
                SliderInteraction::SlideOnly => {
                    let value = self.value(app);
                    let this = self.get_mut(app);
                    this.active = true;
                    this.current_drag_value = value;
                }
            }

            if self.get(app).active {
                // We supply the *current* value as the start location, so that if we have
                // a tap, it consists of a call to onChangeStart with the previous value and
                // a call to onChangeEnd with the new value.
                if let Some(on_change_start) = self.get(app).on_change_start.clone() {
                    let value = self.discretize(app, self.value(app));
                    on_change_start(app, value);
                }
                let on_changed = self.on_changed(app).expect("an interactive slider");
                let value = self.discretize(app, self.get(app).current_drag_value);
                on_changed(app, value);
                state.overlay_controller(app).forward(app, None);
                if self.should_show_value_indicator_when_dragged(app) {
                    state.value_indicator_controller(app).forward(app, None);
                    if let Some(timer) = app.get_mut(state).interaction_timer.take() {
                        timer.cancel(app);
                    }
                    let time_dilation = SchedulerBinding::time_dilation(app);
                    // The timer keeps the render slider, as Dart's closure does: a slider
                    // replaced before it fires still reads `active`.
                    let this = app.retain(self.handle());
                    let timer = Timer::new(
                        app,
                        Self::MINIMUM_INTERACTION_TIME.mul_f64(time_dilation),
                        Listener::new(move |app: &mut App| {
                            app.get_mut(state).interaction_timer = None;
                            let value_indicator_controller = state.value_indicator_controller(app);
                            let active = app.get(this.get()).active;
                            if !active && value_indicator_controller.is_completed(app) {
                                value_indicator_controller.reverse(app, None);
                            }
                        }),
                    );
                    app.get_mut(state).interaction_timer = Some(timer);
                }
            }
        }
    }

    fn end_interaction(self: RenderHandle<Self>, app: &mut App) {
        let state = self.get(app).state;
        if !state.mounted(app) {
            return;
        }

        if self.get(app).active && state.mounted(app) {
            if let Some(on_change_end) = self.get(app).on_change_end.clone() {
                let value = self.discretize(app, self.get(app).current_drag_value);
                on_change_end(app, value);
            }
            let this = self.get_mut(app);
            this.active = false;
            this.current_drag_value = 0.0;
            state.overlay_controller(app).reverse(app, None);
            if self.should_show_value_indicator_when_dragged(app)
                && app.get(state).interaction_timer.is_none()
            {
                state.value_indicator_controller(app).reverse(app, None);
            }
        }
    }

    fn handle_drag_start(self: RenderHandle<Self>, app: &mut App, details: DragStartDetails) {
        self.start_interaction(app, details.global_position);
    }

    fn handle_drag_update(self: RenderHandle<Self>, app: &mut App, details: DragUpdateDetails) {
        if !self.get(app).state.mounted(app) {
            return;
        }

        match self.allowed_interaction(app) {
            SliderInteraction::TapAndSlide
            | SliderInteraction::SlideOnly
            | SliderInteraction::SlideThumb => {
                if self.get(app).active && self.is_interactive(app) {
                    // Use the same coordinate width as _getValueFromGlobalPosition so
                    // that dragging immediately after a tap does not jump.
                    let effective_width =
                        self.track_rect(app).width() - self.discrete_rounded_track_padding(app);
                    if effective_width <= 0.0 {
                        return;
                    }
                    let value_delta = details
                        .primary_delta
                        .expect("a horizontal drag has a primary delta")
                        / effective_width;
                    let delta = match self.text_direction(app) {
                        TextDirection::Rtl => -value_delta,
                        TextDirection::Ltr => value_delta,
                    };
                    self.get_mut(app).current_drag_value += delta;
                    let on_changed = self.on_changed(app).expect("an interactive slider");
                    let value = self.discretize(app, self.get(app).current_drag_value);
                    on_changed(app, value);
                }
            }
            SliderInteraction::TapOnly => {
                // cannot slide (drag) as its tapOnly.
            }
        }
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

    fn is_pointer_on_overlay(self: RenderHandle<Self>, app: &App, global_position: Offset) -> bool {
        self.get(app)
            .overlay_rect
            .expect("the slider has painted once")
            .contains(self.as_box().global_to_local(app, global_position, None))
    }

    /// Calculates the local coordinate center of the [`Slider`] thumb given its
    /// physical placement on the track from 0.0 (left) to 1.0 (right).
    ///
    /// The `visual_position` is provided by the caller so semantics can use the
    /// raw logical value while paint can use the smoothly animated value.
    fn calc_thumb_center(
        self: RenderHandle<Self>,
        app: &App,
        track_rect: Rect,
        visual_position: f64,
    ) -> Offset {
        let padding = if self.track_shape(app).is_rounded() {
            track_rect.height()
        } else {
            0.0
        };
        let thumb_position = if self.is_discrete(app) {
            track_rect.left + visual_position * (track_rect.width() - padding) + padding / 2.0
        } else {
            track_rect.left + visual_position * track_rect.width()
        };
        // Apply padding to trackRect.left and trackRect.right if the track height is
        // greater than the thumb radius to ensure the thumb is drawn within the track.
        let thumb_preferred_size = self
            .thumb_shape(app)
            .get_preferred_size(self.is_interactive(app), self.is_discrete(app));
        let thumb_padding = if padding > thumb_preferred_size.width() / 2.0 {
            padding / 2.0
        } else {
            0.0
        };
        Offset::new(
            clamp_double(
                thumb_position,
                track_rect.left + thumb_padding,
                track_rect.right - thumb_padding,
            ),
            track_rect.center().dy(),
        )
    }

    fn semantic_action_unit(self: RenderHandle<Self>, app: &App) -> f64 {
        match self.divisions(app) {
            Some(divisions) => 1.0 / f64::from(divisions),
            None => self.adjustment_unit(app),
        }
    }

    /// Dart's `increaseAction`, which the arrow keys run; semantics' increase waits with
    /// accessibility.
    fn increase_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_interactive(app) {
            let on_change_start = self
                .get(app)
                .on_change_start
                .clone()
                .expect("the slider's own");
            let current_value = self.current_value(app);
            on_change_start(app, current_value);
            let increase = self.increase_value(app);
            let on_changed = self.on_changed(app).expect("an interactive slider");
            on_changed(app, increase);
            let on_change_end = self
                .get(app)
                .on_change_end
                .clone()
                .expect("the slider's own");
            on_change_end(app, increase);
        }
    }

    /// Dart's `decreaseAction`, which the arrow keys run; semantics' decrease waits with
    /// accessibility.
    fn decrease_action(self: RenderHandle<Self>, app: &mut App) {
        if self.is_interactive(app) {
            let on_change_start = self
                .get(app)
                .on_change_start
                .clone()
                .expect("the slider's own");
            let current_value = self.current_value(app);
            on_change_start(app, current_value);
            let decrease = self.decrease_value(app);
            let on_changed = self.on_changed(app).expect("an interactive slider");
            on_changed(app, decrease);
            let on_change_end = self
                .get(app)
                .on_change_end
                .clone()
                .expect("the slider's own");
            on_change_end(app, decrease);
        }
    }

    fn current_value(self: RenderHandle<Self>, app: &App) -> f64 {
        clamp_double(self.value(app), 0.0, 1.0)
    }

    fn increase_value(self: RenderHandle<Self>, app: &App) -> f64 {
        clamp_double(self.value(app) + self.semantic_action_unit(app), 0.0, 1.0)
    }

    fn decrease_value(self: RenderHandle<Self>, app: &App) -> f64 {
        clamp_double(self.value(app) - self.semantic_action_unit(app), 0.0, 1.0)
    }
}

/// Dart's `addListener(markNeedsPaint)` on the slider's animations.
fn mark_slider_needs_paint(this: Handle<RenderSlider>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

impl RelayoutWhenSystemFontsChangeMixin for RenderSlider {
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
        self.get_mut(app).label_painter.mark_needs_layout();
        self.update_label_painter(app);
    }
}

impl RenderObject for RenderSlider {
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
        let listener = Listener::handle_method(self.handle(), mark_slider_needs_paint);
        let this = self.get(app);
        let (overlay, value_indicator, enable) = (
            this.overlay_animation,
            this.value_indicator_animation,
            this.enable_animation,
        );
        let position_controller = this.state.position_controller(app);
        overlay.add_listener(app, listener.clone());
        value_indicator.add_listener(app, listener.clone());
        enable.add_listener(app, listener.clone());
        position_controller.add_listener(app, listener);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let listener = Listener::handle_method(self.handle(), mark_slider_needs_paint);
        let this = self.get(app);
        let (overlay, value_indicator, enable) = (
            this.overlay_animation,
            this.value_indicator_animation,
            this.enable_animation,
        );
        let position_controller = this.state.position_controller(app);
        overlay.remove_listener(app, &listener);
        value_indicator.remove_listener(app, &listener);
        enable.remove_listener(app, &listener);
        position_controller.remove_listener(app, &listener);
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        let (drag, tap) = (self.get(app).drag, self.get(app).tap);
        RecognizerLeaf::dispose(drag, app);
        RecognizerLeaf::dispose(tap, app);
        self.get_mut(app).label_painter.dispose();
        let this = self.get(app);
        let (enable, value_indicator, overlay) = (
            this.enable_animation,
            this.value_indicator_animation,
            this.overlay_animation,
        );
        for animation in [enable, value_indicator, overlay] {
            animation.dispose(app);
            app.destroy(animation);
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
        let controller_value = state.position_controller(app).value(app);
        let text_direction = self.text_direction(app);

        // The visual position is the position of the thumb from 0 to 1 from left
        // to right. In left to right, this is the same as the value, but it is
        // reversed for right to left text.
        let secondary_track_value = self.get(app).secondary_track_value;
        let (visual_position, secondary_visual_position) =
            match (text_direction, secondary_track_value) {
                (TextDirection::Rtl, None) => (1.0 - controller_value, None),
                (TextDirection::Rtl, Some(secondary_track_value)) => {
                    (1.0 - controller_value, Some(1.0 - secondary_track_value))
                }
                (TextDirection::Ltr, secondary_track_value) => {
                    (controller_value, secondary_track_value)
                }
            };

        let is_discrete = self.is_discrete(app);
        let is_interactive = self.is_interactive(app);
        let slider_theme = self.slider_theme(app).clone();
        let track_rect = self.track_shape(app).get_preferred_rect(
            app,
            self.as_box(),
            offset,
            &slider_theme,
            None,
            Some(is_discrete),
        );

        let thumb_center = self.calc_thumb_center(app, track_rect, visual_position);

        if is_interactive {
            let overlay_size = self
                .overlay_shape(app)
                .get_preferred_size(is_interactive, false);
            self.get_mut(app).overlay_rect =
                Some(Rect::from_circle(thumb_center, overlay_size.width() / 2.0));
        }
        let secondary_offset = secondary_visual_position.map(|secondary_visual_position| {
            Offset::new(
                track_rect.left + secondary_visual_position * track_rect.width(),
                track_rect.center().dy(),
            )
        });

        // If [Slider.year2023] is false, the thumb uses handle thumb shape and gapped track shape.
        // The handle width and track gap are adjusted when the thumb is pressed.
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
            if width > 0.0 {
                thumb_width = Some(pressed_width);
            }
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
        self.track_shape(app).paint(
            app,
            context,
            offset,
            self.as_box(),
            &track_theme,
            enable_animation,
            thumb_center,
            secondary_offset,
            Some(is_interactive),
            Some(is_discrete),
            text_direction,
        );

        let value = self.value(app);
        let text_scale_factor = self.text_scale_factor(app);
        let screen_size = self.screen_size(app);
        let size_with_overflow = if screen_size.is_empty() {
            self.size(app)
        } else {
            screen_size
        };
        if !overlay_animation.is_dismissed(app) {
            let overlay_shape = self.overlay_shape(app);
            self.with_label_painter(app, |app, label_painter| {
                overlay_shape.paint(
                    app,
                    context,
                    thumb_center,
                    overlay_animation,
                    enable_animation,
                    is_discrete,
                    label_painter,
                    self.as_box(),
                    &slider_theme,
                    text_direction,
                    value,
                    text_scale_factor,
                    size_with_overflow,
                );
            });
        }

        if is_discrete {
            let divisions = self.divisions(app).expect("a discrete slider");
            let tick_mark_shape = slider_theme
                .tick_mark_shape
                .clone()
                .expect("the slider fills in its tick mark shape");
            let tick_mark_width = tick_mark_shape
                .get_preferred_size(&slider_theme, is_interactive)
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
                    tick_mark_shape.paint(
                        app,
                        context,
                        tick_mark_offset,
                        self.as_box(),
                        &slider_theme,
                        enable_animation,
                        thumb_center,
                        is_interactive,
                        text_direction,
                    );
                }
            }
        }

        if is_interactive
            && self.label(app).is_some()
            && ((self.should_show_value_indicator_when_dragged(app)
                && !value_indicator_animation.is_dismissed(app))
                || self.should_always_show_value_indicator(app))
        {
            let paint_value_indicator: PaintValueIndicator = Rc::new(
                move |app: &mut App, context: &mut PaintingContext, offset: Offset| {
                    if self.as_render_object(app).attached(app)
                        && self.get(app).label_painter.text().is_some()
                    {
                        let Some(value_indicator_shape) =
                            self.slider_theme(app).value_indicator_shape.clone()
                        else {
                            return;
                        };
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
                        let slider_theme = self.slider_theme(app).clone();
                        let text_direction = self.text_direction(app);
                        let value = self.value(app);
                        let text_scale_factor = self.text_scale_factor(app);
                        let screen_size = self.screen_size(app);
                        let size_with_overflow = if screen_size.is_empty() {
                            self.size(app)
                        } else {
                            screen_size
                        };
                        let is_discrete = self.is_discrete(app);
                        self.with_label_painter(app, |app, label_painter| {
                            value_indicator_shape.paint(
                                app,
                                context,
                                offset + thumb_center,
                                activation_animation,
                                enable_animation,
                                is_discrete,
                                label_painter,
                                self.as_box(),
                                &slider_theme,
                                text_direction,
                                value,
                                text_scale_factor,
                                size_with_overflow,
                            );
                        });
                    }
                },
            );
            app.get_mut(state).paint_value_indicator = Some(paint_value_indicator);
        } else {
            app.get_mut(state).paint_value_indicator = None;
        }

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
        let thumb_shape = self.thumb_shape(app);
        self.with_label_painter(app, |app, label_painter| {
            thumb_shape.paint(
                app,
                context,
                thumb_center,
                overlay_animation,
                enable_animation,
                is_discrete,
                label_painter,
                self.as_box(),
                &thumb_theme,
                text_direction,
                value,
                text_scale_factor,
                size_with_overflow,
            );
        });
    }
}

impl RenderBox for RenderSlider {
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
        if !self.get(app).state.mounted(app) {
            return;
        }
        if let PointerEvent::Down(down) = event
            && self.is_interactive(app)
        {
            // We need to add the drag first so that it has priority.
            let (drag, tap) = (self.get(app).drag, self.get(app).tap);
            drag.add_pointer(app, down.clone());
            tap.add_pointer(app, down.clone());
        }
        if self.is_interactive(app)
            && let Some(overlay_rect) = self.get(app).overlay_rect
        {
            self.set_hovering_thumb(app, overlay_rect.contains(event.local_position()));
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

/// Dart's `_ValueIndicatorRenderObjectWidget`.
#[derive(Debug)]
struct ValueIndicatorRenderObjectWidget {
    state: Handle<SliderState>,
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
pub(crate) struct RenderValueIndicator {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    system_fonts: RelayoutWhenSystemFontsChangeData,
    value_indicator_animation: Handle<CurvedAnimation>,
    state: Handle<SliderState>,
}

impl RenderValueIndicator {
    fn new(app: &mut App, state: Handle<SliderState>) -> RenderHandle<RenderValueIndicator> {
        let value_indicator_animation = CurvedAnimation::create(
            app,
            state.value_indicator_controller(app).as_animation(),
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
        let animation = self.get(app).value_indicator_animation;
        let position_controller = self.get(app).state.position_controller(app);
        animation.add_listener(app, listener.clone());
        position_controller.add_listener(app, listener);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let listener = Listener::handle_method(self.handle(), mark_value_indicator_needs_paint);
        let animation = self.get(app).value_indicator_animation;
        let position_controller = self.get(app).state.position_controller(app);
        animation.remove_listener(app, &listener);
        position_controller.remove_listener(app, &listener);
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let state = self.get(app).state;
        if let Some(paint_value_indicator) = app.get(state).paint_value_indicator.clone() {
            paint_value_indicator(app, context, offset);
        }
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        let value_indicator_animation = self.get(app).value_indicator_animation;
        value_indicator_animation.dispose(app);
        app.destroy(value_indicator_animation);
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

/// Dart's `const AlwaysStoppedAnimation<double>(1)`, which Dart canonicalizes into one
/// object, kept per [`App`] and made on first use; never destroyed.
#[derive(Default)]
struct AlwaysStoppedAtOne {
    animation: Option<Handle<AlwaysStoppedAnimation<f64>>>,
}

fn always_stopped_at_one(app: &mut App) -> AnyAnimation<f64> {
    let statics = app.singleton::<AlwaysStoppedAtOne>();
    if let Some(animation) = app.get(statics).animation {
        return animation.as_animation();
    }
    let animation = app.create(AlwaysStoppedAnimation::new(1.0));
    app.get_mut(statics).animation = Some(animation);
    animation.as_animation()
}

/// Dart's `_SliderDefaultsM2`.
struct SliderDefaultsM2 {
    /// Dart's `super(..)` arguments.
    slider_theme_data: SliderThemeData,
    context: DefaultsContext,
}

impl SliderDefaultsM2 {
    fn new(context: BuildContext) -> SliderDefaultsM2 {
        SliderDefaultsM2 {
            slider_theme_data: SliderThemeData::new().track_height(4.0),
            context: DefaultsContext::new(context),
        }
    }
}

#[allow(deprecated)]
impl SliderThemeDataGetters for SliderDefaultsM2 {
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

    fn secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.54))
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

    fn disabled_secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
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

    fn value_indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let slider_theme = SliderTheme::of(app, self.context.context());
        Some(
            if slider_theme
                .value_indicator_shape
                .as_ref()
                .is_some_and(|shape| shape.as_any().is::<RoundedRectSliderValueIndicatorShape>())
            {
                colors.inverse_surface()
            } else {
                colors.primary
            },
        )
    }

    fn value_indicator_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RectangularSliderValueIndicatorShape::new()))
    }

    fn thumb_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderThumbShape::new()))
    }

    fn track_shape(&self, _app: &mut App) -> Option<SliderTrackShapeRef> {
        Some(Rc::new(RoundedRectSliderTrackShape::new()))
    }

    fn overlay_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderOverlayShape::new()))
    }

    fn tick_mark_shape(&self, _app: &mut App) -> Option<SliderTickMarkShapeRef> {
        Some(Rc::new(RoundSliderTickMarkShape::new()))
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
}

/// Dart's `_SliderDefaultsM3Year2023`.
struct SliderDefaultsM3Year2023 {
    /// Dart's `super(..)` arguments.
    slider_theme_data: SliderThemeData,
    context: DefaultsContext,
}

impl SliderDefaultsM3Year2023 {
    fn new(context: BuildContext) -> SliderDefaultsM3Year2023 {
        SliderDefaultsM3Year2023 {
            slider_theme_data: SliderThemeData::new().track_height(4.0),
            context: DefaultsContext::new(context),
        }
    }
}

#[allow(deprecated)]
impl SliderThemeDataGetters for SliderDefaultsM3Year2023 {
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
        Some(colors.surface_container_highest())
    }

    fn secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.54))
    }

    fn disabled_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn disabled_inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.12))
    }

    fn disabled_secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.12))
    }

    fn active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary.with_opacity(0.38))
    }

    fn inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface_variant().with_opacity(0.38))
    }

    fn disabled_active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn disabled_inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
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
        let primary = colors.primary;
        Some(
            WidgetStateColor::resolve_with(move |states: &WidgetStates| {
                if states.contains(&WidgetState::Dragged) {
                    return primary.with_opacity(0.1);
                }
                if states.contains(&WidgetState::Hovered) {
                    return primary.with_opacity(0.08);
                }
                if states.contains(&WidgetState::Focused) {
                    return primary.with_opacity(0.1);
                }

                Colors::TRANSPARENT
            })
            .into_any(),
        )
    }

    fn value_indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn value_indicator_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(DropSliderValueIndicatorShape::new()))
    }

    fn thumb_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderThumbShape::new()))
    }

    fn track_shape(&self, _app: &mut App) -> Option<SliderTrackShapeRef> {
        Some(Rc::new(RoundedRectSliderTrackShape::new()))
    }

    fn overlay_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderOverlayShape::new()))
    }

    fn tick_mark_shape(&self, _app: &mut App) -> Option<SliderTickMarkShapeRef> {
        Some(Rc::new(RoundSliderTickMarkShape::new()))
    }

    fn value_indicator_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            theme
                .text_theme
                .label_medium
                .as_ref()
                .expect("the text theme has a label medium style")
                .copy_with()
                .color(colors.on_primary),
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::valo::Op;
    use waft_widgets::{Center, StatefulBuilder};

    use super::*;
    use crate::material::Material;
    use crate::slider_parts::SliderTickMarkShape;
    use crate::slider_value_indicator_shape::SliderComponentShape;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    /// Dart's `LoggingThumbShape`: a thumb shape that also logs its repaint center.
    #[derive(Debug)]
    struct LoggingThumbShape {
        log: Rc<RefCell<Vec<Offset>>>,
    }

    impl SliderComponentShape for LoggingThumbShape {
        fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
            Size::new(10.0, 10.0)
        }

        fn paint(
            &self,
            _app: &mut App,
            context: &mut PaintingContext,
            thumb_center: Offset,
            _activation_animation: AnyAnimation<f64>,
            _enable_animation: AnyAnimation<f64>,
            _is_discrete: bool,
            _label_painter: &mut TextPainter,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _text_direction: TextDirection,
            _value: f64,
            _text_scale_factor: f64,
            _size_with_overflow: Size,
        ) {
            use waft_painting::ClipContext;
            self.log.borrow_mut().push(thumb_center);
            context.canvas().draw_circle(
                thumb_center,
                5.0,
                &waft_embedder::Paint::from_color(Colors::RED.primary().into()),
            );
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Dart's `TallSliderTickMarkShape`.
    #[derive(Debug)]
    struct TallSliderTickMarkShape;

    impl SliderTickMarkShape for TallSliderTickMarkShape {
        fn get_preferred_size(&self, _slider_theme: &SliderThemeData, _is_enabled: bool) -> Size {
            Size::new(10.0, 200.0)
        }

        fn paint(
            &self,
            _app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _enable_animation: AnyAnimation<f64>,
            _thumb_center: Offset,
            _is_enabled: bool,
            _text_direction: TextDirection,
        ) {
            use waft_painting::ClipContext;
            context.canvas().draw_rect(
                Rect::from_ltwh(offset.dx(), offset.dy(), 10.0, 20.0),
                &waft_embedder::Paint::from_color(Colors::RED.primary().into()),
            );
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// What a test's slider callbacks heard.
    #[derive(Default)]
    struct Heard {
        value: Cell<f64>,
        start_value: Cell<Option<f64>>,
        end_value: Cell<Option<f64>>,
        updates: Cell<usize>,
        start_updates: Cell<usize>,
        end_updates: Cell<usize>,
    }

    impl Heard {
        fn starting_at(value: f64) -> Rc<Heard> {
            let heard = Rc::new(Heard::default());
            heard.value.set(value);
            heard
        }
    }

    /// A slider configured by `configure`, its `value` and callbacks wired to `heard`.
    fn slider(
        heard: &Rc<Heard>,
        set_state: StateSetterRef,
        configure: impl Fn(Slider) -> Slider,
    ) -> Slider {
        let on_changed_heard = Rc::clone(heard);
        let start_heard = Rc::clone(heard);
        let end_heard = Rc::clone(heard);
        configure(
            Slider::new(
                heard.value.get(),
                Some(Rc::new(move |app: &mut App, new_value: f64| {
                    let heard = Rc::clone(&on_changed_heard);
                    set_state(app, &mut |_app| {
                        heard.updates.set(heard.updates.get() + 1);
                        heard.value.set(new_value);
                    });
                })),
            )
            .on_change_start(Rc::new(move |_app: &mut App, value: f64| {
                start_heard
                    .start_updates
                    .set(start_heard.start_updates.get() + 1);
                start_heard.start_value.set(Some(value));
            }))
            .on_change_end(Rc::new(move |_app: &mut App, value: f64| {
                end_heard.end_updates.set(end_heard.end_updates.get() + 1);
                end_heard.end_value.set(Some(value));
            })),
        )
    }

    type StateSetterRef = waft_widgets::StateSetter;

    /// Dart's `MaterialApp(home: Directionality(child: StatefulBuilder(... Material(child:
    /// Center(child: wrap(Slider(...)))))))`, the shape most cases pump.
    fn stateful_slider_app(
        tester: &WidgetTester,
        theme: ThemeData,
        text_direction: TextDirection,
        heard: &Rc<Heard>,
        build: impl Fn(&mut App, BuildContext, Slider) -> WidgetRef + 'static,
        configure: impl Fn(Slider) -> Slider + 'static,
    ) -> WidgetRef {
        let _ = tester;
        let heard = Rc::clone(heard);
        material_app(
            theme,
            Directionality::new(
                text_direction,
                StatefulBuilder::new(move |app, context, set_state| {
                    let slider = slider(&heard, set_state, &configure);
                    Material::new()
                        .child(Center::new().child(build(app, context, slider)))
                        .into_widget()
                }),
            )
            .into_widget(),
        )
    }

    fn plain(_app: &mut App, _context: BuildContext, slider: Slider) -> WidgetRef {
        slider.into_widget()
    }

    /// A tester on `platform` whose fonts lay out text, as the debug banner and the labels need.
    pub(crate) fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    /// Dart's `ThemeData()`.
    pub(crate) fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder().build(&tester.cell.borrow())
    }

    pub(crate) fn slider_rect(tester: &WidgetTester) -> Rect {
        let slider = tester.find_by_type::<Slider>()[0];
        let render = tester.render_box_of(slider);
        tester.global_rect(render)
    }

    fn transient_callback_count(tester: &WidgetTester) -> usize {
        SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut())
    }

    // 'The initial value should respect the discrete value'.
    #[test]
    fn the_initial_value_should_respect_the_discrete_value() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.20);
            let log = Rc::new(RefCell::new(Vec::new()));
            let thumb_log = Rc::clone(&log);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                move |app, context, slider| {
                    let slider_theme =
                        SliderTheme::of(app, context)
                            .copy_with()
                            .thumb_shape(Rc::new(LoggingThumbShape {
                                log: Rc::clone(&thumb_log),
                            }));
                    SliderTheme::new(slider_theme, slider).into_widget()
                },
                |slider| slider.divisions(4),
            ));

            assert_eq!(heard.value.get(), 0.20);
            assert_eq!(log.borrow().len(), 1);
            assert_eq!(log.borrow()[0], Offset::new(213.0, 300.0));
        });
    }

    // 'Slider can move when tapped (LTR)'.
    #[test]
    fn a_slider_moves_when_tapped_ltr() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                plain,
                |slider| slider,
            ));

            assert_eq!(heard.value.get(), 0.0);
            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(heard.value.get(), 0.5);
            assert_eq!(heard.start_value.get(), Some(0.0));
            assert_eq!(heard.end_value.get(), Some(0.5));
            heard.start_value.set(None);
            heard.end_value.set(None);
            tester.pump(Duration::ZERO); // No animation should start.
            assert_eq!(transient_callback_count(&tester), 0);

            let rect = slider_rect(&tester);
            let target = rect.top_left() + (rect.bottom_right() - rect.top_left()) / 4.0;
            tester.tap(target, 2);
            assert!((heard.value.get() - 0.25).abs() <= 0.05);
            assert_eq!(heard.start_value.get(), Some(0.5));
            assert!((heard.end_value.get().unwrap() - 0.25).abs() <= 0.05);
            tester.pump(Duration::ZERO); // No animation should start.
            assert_eq!(transient_callback_count(&tester), 0);
        });
    }

    // 'Slider can move when tapped (RTL)'.
    #[test]
    fn a_slider_moves_when_tapped_rtl() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Rtl,
                &heard,
                plain,
                |slider| slider,
            ));

            assert_eq!(heard.value.get(), 0.0);
            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(heard.value.get(), 0.5);
            tester.pump(Duration::ZERO); // No animation should start.
            assert_eq!(transient_callback_count(&tester), 0);

            let rect = slider_rect(&tester);
            let target = rect.top_left() + (rect.bottom_right() - rect.top_left()) / 4.0;
            tester.tap(target, 2);
            assert!((heard.value.get() - 0.75).abs() <= 0.05);
            tester.pump(Duration::ZERO); // No animation should start.
            assert_eq!(transient_callback_count(&tester), 0);
        });
    }

    // "Slider doesn't send duplicate change events if tapped on the same value".
    #[test]
    fn a_slider_sends_no_duplicate_change_events_when_tapped_on_the_same_value() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                plain,
                |slider| slider,
            ));

            assert_eq!(heard.value.get(), 0.0);
            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(heard.value.get(), 0.5);
            assert_eq!(heard.start_value.get(), Some(0.0));
            assert_eq!(heard.end_value.get(), Some(0.5));
            tester.pump(Duration::ZERO);
            tester.tap(slider_rect(&tester).center(), 2);
            assert_eq!(heard.value.get(), 0.5);
            tester.pump(Duration::ZERO);
            assert_eq!(heard.updates.get(), 1);
            assert_eq!(heard.start_updates.get(), 2);
            assert_eq!(heard.end_updates.get(), 2);
        });
    }

    // 'Value indicator shows for a bit after being tapped'.
    #[test]
    fn the_value_indicator_shows_for_a_bit_after_a_tap() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                plain,
                |slider| slider.divisions(4),
            ));

            assert_eq!(heard.value.get(), 0.0);
            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(heard.value.get(), 0.5);
            tester.pump(Duration::from_millis(100));
            // Starts with the position animation and value indicator
            assert_eq!(transient_callback_count(&tester), 2);
            tester.pump(Duration::from_millis(100));
            // Value indicator is longer than position.
            assert_eq!(transient_callback_count(&tester), 1);
            tester.pump(Duration::from_millis(100));
            assert_eq!(transient_callback_count(&tester), 0);
            tester.pump(Duration::from_millis(100));
            assert_eq!(transient_callback_count(&tester), 0);
            tester.pump(Duration::from_millis(100));
            // Shown for long enough, value indicator is animated closed.
            assert_eq!(transient_callback_count(&tester), 1);
            tester.pump(Duration::from_millis(101));
            assert_eq!(transient_callback_count(&tester), 0);
        });
    }

    /// The bounds of `path` when it is what `Canvas::draw_circle` records: one square contour
    /// of a move, four conic quarters and a close, which a zero radius records too. A rounded
    /// rect's contour has lines between its corners.
    pub(crate) fn circle_of(path: &waft_embedder::valo::Path) -> Option<Rect> {
        let bounds = path.bounds();
        // The conics' f32 points leave the two extents a rounding apart.
        let square = (bounds.width - bounds.height).abs() <= 1e-4 * bounds.width.max(1.0);
        (path.elements().count() == 6 && square).then(|| {
            Rect::from_ltwh(
                f64::from(bounds.x),
                f64::from(bounds.y),
                f64::from(bounds.width),
                f64::from(bounds.height),
            )
        })
    }

    /// The circles `render` and its subtree draw: Dart's `drawCircle` calls.
    fn circles(tester: &WidgetTester, render: AnyRenderObject) -> Vec<Rect> {
        tester
            .painted_ops(render)
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { path, .. } => circle_of(path),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn slider_render(tester: &WidgetTester) -> AnyRenderObject {
        tester
            .first::<RenderSlider>()
            .as_render_object(&tester.cell.borrow())
    }

    // 'Tick marks are skipped when they are too dense'.
    #[test]
    fn tick_marks_are_skipped_when_too_dense() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let build_slider = |divisions: i32| {
                material_app(
                    theme.clone(),
                    Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(
                            Center::new().child(
                                Slider::new(0.25, Some(Rc::new(|_app: &mut App, _value: f64| {})))
                                    .max(100.0)
                                    .divisions(divisions),
                            ),
                        ),
                    )
                    .into_widget(),
                )
            };

            // Pump a slider with a reasonable amount of divisions to verify that the
            // tick marks are drawn when the number of tick marks is not too dense.
            tester.pump_widget(build_slider(4));

            // 5 tick marks and a thumb.
            assert_eq!(circles(&tester, slider_render(&tester)).len(), 6);

            // 200 divisions will produce a tick interval off less than 6,
            // which would be too dense to draw.
            tester.pump_widget(build_slider(200));

            // No tick marks are drawn because they are too dense, but the thumb is
            // still drawn.
            assert_eq!(circles(&tester, slider_render(&tester)).len(), 1);
        });
    }

    /// `tester.renderObject<RenderBox>(find.byType(Slider)).size`.
    pub(crate) fn slider_size(tester: &WidgetTester) -> Size {
        let slider = tester.find_by_type::<Slider>()[0];
        let render = tester.render_box_of(slider);
        render.size(&tester.cell.borrow())
    }

    // 'Slider sizing'.
    #[test]
    fn slider_sizing() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let app = |theme: ThemeData, body: WidgetRef| {
                material_app(
                    theme,
                    Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(Center::new().child(body)),
                    )
                    .into_widget(),
                )
            };
            tester.pump_widget(app(theme.clone(), Slider::new(0.5, None).into_widget()));
            assert_eq!(slider_size(&tester), Size::new(800.0, 600.0));

            tester.pump_widget(app(
                theme.clone(),
                waft_widgets::IntrinsicWidth::new()
                    .child(Slider::new(0.5, None))
                    .into_widget(),
            ));
            assert_eq!(slider_size(&tester), Size::new(144.0 + 2.0 * 24.0, 600.0));

            tester.pump_widget(app(
                theme,
                waft_widgets::OverflowBox::new()
                    .max_width(f64::INFINITY)
                    .max_height(f64::INFINITY)
                    .child(Slider::new(0.5, None))
                    .into_widget(),
            ));
            assert_eq!(slider_size(&tester), Size::new(144.0 + 2.0 * 24.0, 48.0));
        });
    }

    /// Dart's `MaterialApp(home: Directionality(child: Material(child: body)))`.
    pub(crate) fn material_body<K>(
        theme: ThemeData,
        text_direction: TextDirection,
        body: impl IntoWidget<K>,
    ) -> WidgetRef {
        material_app(
            theme,
            Directionality::new(text_direction, Material::new().child(body)).into_widget(),
        )
    }

    pub(crate) fn noop() -> Option<ValueChanged<f64>> {
        Some(Rc::new(|_app: &mut App, _value: f64| {}))
    }

    fn logging(log: &Rc<RefCell<Vec<f64>>>) -> ValueChanged<f64> {
        let log = Rc::clone(log);
        Rc::new(move |_app: &mut App, value: f64| log.borrow_mut().push(value))
    }

    // 'Slider take on discrete values'.
    #[test]
    fn a_slider_takes_on_discrete_values() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                |_app, _context, slider| {
                    SizedBox::new()
                        .width(144.0 + 2.0 * 16.0) // _kPreferredTotalWidth
                        .child(slider)
                        .into_widget()
                },
                |slider| slider.max(100.0).divisions(10),
            ));

            assert_eq!(heard.value.get(), 0.0);
            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(heard.value.get(), 50.0);
            tester.drag_from(slider_rect(&tester).center(), Offset::new(5.0, 0.0), 2);
            assert_eq!(heard.value.get(), 50.0);
            tester.drag_from(slider_rect(&tester).center(), Offset::new(40.0, 0.0), 3);
            assert_eq!(heard.value.get(), 80.0);

            tester.pump(Duration::ZERO); // Starts animation.
            assert!(transient_callback_count(&tester) > 0);
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(200));
            // Animation complete.
            assert_eq!(transient_callback_count(&tester), 0);
        });
    }

    // 'Discrete slider has no floating-point rounding errors'.
    #[test]
    fn a_discrete_slider_has_no_floating_point_rounding_errors() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let values = Rc::new(RefCell::new(Vec::new()));
            let theme = default_theme(&tester);
            tester.pump_widget(material_body(
                theme,
                TextDirection::Ltr,
                Center::new().child(
                    SizedBox::new().width(180.0).child(
                        Slider::new(0.0, Some(logging(&values)))
                            .max(35.0)
                            .divisions(35),
                    ),
                ),
            ));

            let rect = slider_rect(&tester);
            let active_track_width = rect.width() - 48.0;
            let start = rect.top_left() + Offset::new(24.0, rect.height() / 2.0);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, start);
            tester.move_to(
                &mut gesture,
                start + Offset::new(active_track_width * 29.0 / 35.0, 0.0),
                true,
            );
            tester.up(&gesture);

            let values = values.borrow();
            assert!(!values.is_empty());
            assert_eq!(*values.last().unwrap(), 29.0);
            for value in values.iter() {
                assert_eq!(*value, value.round());
            }
        });
    }

    // 'Slider can be given zero values'.
    #[test]
    fn a_slider_can_be_given_zero_values() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let log = Rc::new(RefCell::new(Vec::new()));
            let theme = default_theme(&tester);
            tester.pump_widget(material_body(
                theme.clone(),
                TextDirection::Ltr,
                Slider::new(0.0, Some(logging(&log))),
            ));

            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(*log.borrow(), vec![0.5]);
            log.borrow_mut().clear();

            tester.pump_widget(material_body(
                theme,
                TextDirection::Ltr,
                Slider::new(0.0, Some(logging(&log))).max(0.0),
            ));

            tester.tap(slider_rect(&tester).center(), 2);
            assert!(log.borrow().is_empty());
        });
    }

    // 'Slider can tap in vertical scroller'.
    #[test]
    fn a_slider_can_be_tapped_in_a_vertical_scroller() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(0.0));
            let on_changed_value = Rc::clone(&value);
            let theme = default_theme(&tester);
            tester.pump_widget(material_body(
                theme,
                TextDirection::Ltr,
                waft_widgets::ListView::new().children([
                    Slider::new(
                        value.get(),
                        Some(Rc::new(move |_app: &mut App, new_value: f64| {
                            on_changed_value.set(new_value)
                        })),
                    )
                    .into_widget(),
                    SizedBox::new().height(2000.0).into_widget(),
                ]),
            ));

            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(value.get(), 0.5);
        });
    }

    /// Dart's 'Slider drags immediately' for `text_direction`: the value after a press at the
    /// centre and a move one pixel right.
    fn drags_immediately(text_direction: TextDirection) -> f64 {
        let mut tester = tester_on(TargetPlatform::Android);
        let value = Rc::new(Cell::new(0.0));
        let on_changed_value = Rc::clone(&value);
        let theme = default_theme(&tester);
        tester.pump_widget(material_body(
            theme,
            text_direction,
            Center::new().child(Slider::new(
                value.get(),
                Some(Rc::new(move |_app: &mut App, new_value: f64| {
                    on_changed_value.set(new_value)
                })),
            )),
        ));

        let center = slider_rect(&tester).center();
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, center);

        assert_eq!(value.get(), 0.5);

        tester.move_to(&mut gesture, center + Offset::new(1.0, 0.0), true);
        let moved = value.get();
        tester.up(&gesture);
        moved
    }

    // 'Slider drags immediately (LTR)'.
    #[test]
    fn a_slider_drags_immediately_ltr() {
        with_main_thread_stack(|| {
            assert!(drags_immediately(TextDirection::Ltr) > 0.5);
        });
    }

    // 'Slider drags immediately (RTL)'.
    #[test]
    fn a_slider_drags_immediately_rtl() {
        with_main_thread_stack(|| {
            assert!(drags_immediately(TextDirection::Rtl) < 0.5);
        });
    }

    // 'Slider onChangeStart and onChangeEnd fire once'.
    #[test]
    fn on_change_start_and_on_change_end_fire_once() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/28115
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let start_heard = Rc::clone(&heard);
            let end_heard = Rc::clone(&heard);
            let theme = default_theme(&tester);
            tester.pump_widget(material_body(
                theme,
                TextDirection::Ltr,
                Center::new().child(
                    waft_widgets::GestureDetector::new()
                        .on_horizontal_drag_update(Rc::new(|_app: &mut App, _details| {}))
                        .child(
                            Slider::new(0.0, noop())
                                .on_change_start(Rc::new(move |_app: &mut App, _value: f64| {
                                    start_heard
                                        .start_updates
                                        .set(start_heard.start_updates.get() + 1)
                                }))
                                .on_change_end(Rc::new(move |_app: &mut App, _value: f64| {
                                    end_heard.end_updates.set(end_heard.end_updates.get() + 1)
                                })),
                        ),
                ),
            ));

            tester.drag_from(slider_rect(&tester).center(), Offset::new(20.0, 0.0), 1);

            assert_eq!(heard.start_updates.get(), 1);
            assert_eq!(heard.end_updates.get(), 1);
        });
    }

    /// Dart's `paints..circle(color: color)`: whether `render` or its subtree draws a circle
    /// in `color`.
    pub(crate) fn paints_circle(
        tester: &WidgetTester,
        render: AnyRenderObject,
        color: Color,
    ) -> bool {
        tester.painted_ops(render).iter().any(|op| {
            matches!(op, Op::DrawPath { path, paint, .. }
                if circle_of(path).is_some() && colors_match(paint.color, color))
        })
    }

    /// Dart's `MaterialApp(theme:, home: Material(child: Center(child: StatefulBuilder(...))))`
    /// whose builder makes a slider at `value` with `configure`, updating `value` when enabled.
    fn focusable_slider_app(
        theme: ThemeData,
        value: &Rc<Cell<f64>>,
        enabled: bool,
        configure: impl Fn(Slider) -> Slider + 'static,
    ) -> WidgetRef {
        let value = Rc::clone(value);
        material_app(
            theme,
            Material::new()
                .child(Center::new().child(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        let on_changed_value = Rc::clone(&value);
                        let on_changed: Option<ValueChanged<f64>> = enabled.then(|| {
                            Rc::new(move |app: &mut App, new_value: f64| {
                                let value = Rc::clone(&on_changed_value);
                                set_state(app, &mut |_app| value.set(new_value));
                            }) as ValueChanged<f64>
                        });
                        configure(Slider::new(value.get(), on_changed)).into_widget()
                    },
                )))
                .into_widget(),
        )
    }

    fn new_focus_node(tester: &WidgetTester) -> AnyFocusNode {
        FocusNode::new(&mut tester.cell.borrow_mut()).as_node()
    }

    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn primary_with_opacity(theme: &ThemeData, opacity: f64) -> Color {
        theme.color_scheme.primary.with_opacity(opacity)
    }

    // 'Material3 - Slider is focusable and has correct focus color'.
    #[test]
    fn material3_a_slider_is_focusable_and_has_the_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let focus_node = new_focus_node(&tester);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider.autofocus(true).focus_node(focus_node)
                })
            };
            tester.pump_widget(build_app(true));

            // Check that the overlay shows when focused.
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            let focused_color = primary_with_opacity(&theme, 0.1);
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                focused_color
            ));

            // Check that the overlay does not show when unfocused and disabled.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(!paints_circle(
                &tester,
                slider_render(&tester),
                focused_color
            ));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    // 'Slider has correct focus color from overlayColor property'.
    #[test]
    fn a_slider_takes_its_focus_color_from_overlay_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let focus_node = new_focus_node(&tester);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let purple = Colors::PURPLE.get(&500).expect("a swatch shade");
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider
                        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            move |states: &WidgetStates| {
                                if states.contains(&WidgetState::Focused) {
                                    return Some(purple);
                                }

                                Some(Colors::TRANSPARENT)
                            },
                        ))
                        .autofocus(true)
                        .focus_node(focus_node)
                })
            };
            tester.pump_widget(build_app(true));

            // Check that the overlay shows when focused.
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(paints_circle(&tester, slider_render(&tester), purple));

            // Check that the overlay does not show when focused and disabled.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(!paints_circle(&tester, slider_render(&tester), purple));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    // 'Slider can be hovered and has correct hover color'.
    #[test]
    fn a_slider_can_be_hovered_and_has_the_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let orange = Colors::ORANGE.get(&500).expect("a swatch shade");
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, |slider| slider)
            };
            tester.pump_widget(build_app(true));

            // Slider does not have overlay when enabled and not hovered.
            tester.pump_and_settle();
            assert!(!paints_circle(&tester, slider_render(&tester), orange));

            // Start hovering.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);

            // Slider has overlay when enabled and hovered.
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();
            let hovered_color = primary_with_opacity(&theme, 0.08);
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                hovered_color
            ));

            // Slider still shows correct hovered color after pressing/dragging
            let mut pressed = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            pressed.position = gesture.position;
            tester.down(&mut pressed, slider_rect(&tester).center());
            tester.pump(Duration::ZERO);
            tester.up(&pressed);
            tester.pump_and_settle();
            gesture.position = pressed.position;
            tester.move_to(&mut gesture, Offset::new(0.0, 100.0), false);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);
            tester.pump_and_settle();
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                hovered_color
            ));

            // Slider does not have an overlay when disabled and hovered.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!paints_circle(&tester, slider_render(&tester), orange));
        });
    }

    // 'Material3 - Slider is draggable and has correct dragged color'.
    #[test]
    fn material3_a_slider_is_draggable_and_has_the_dragged_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let focus_node = new_focus_node(&tester);
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider.focus_node(focus_node)
                })
            };
            tester.pump_widget(build_app(true));

            // Slider does not have overlay when enabled and not dragged.
            tester.pump_and_settle();
            let dragged_color = primary_with_opacity(&theme, 0.1);
            assert!(!paints_circle(
                &tester,
                slider_render(&tester),
                dragged_color
            ));

            // Start dragging.
            let mut drag = WidgetTester::touch(1);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);

            // Less than configured touch slop, more than default touch slop
            let at = drag.position + Offset::new(19.0, 0.0);
            tester.move_to(&mut drag, at, true);
            tester.pump(Duration::ZERO);
            // Valo records no draw that covers no pixel, and the overlay has just started
            // to grow from a zero radius, a circle Dart's `paints` matcher sees as a call; one
            // frame later it has a radius.
            tester.pump(Duration::from_millis(16));

            // Slider has overlay when enabled and dragged.
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                dragged_color
            ));

            tester.up(&drag);
            tester.pump_and_settle();

            // Slider without focus doesn't have overlay when enabled and dragged.
            assert!(!focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(!paints_circle(
                &tester,
                slider_render(&tester),
                dragged_color
            ));

            // Slider has overlay when enabled, dragged and focused.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            assert!(focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                dragged_color
            ));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    // 'OverlayColor property is correctly applied when activeColor is also provided'.
    #[test]
    fn overlay_color_applies_when_active_color_is_also_given() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let focus_node = new_focus_node(&tester);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            const ACTIVE_COLOR: Color = Color::new(0xffff0000);
            const OVERLAY_COLOR: Color = Color::new(0xff0000ff);
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider
                        .active_color(ACTIVE_COLOR)
                        .overlay_color(Rc::new(WidgetStatePropertyAll::new(Some(OVERLAY_COLOR))))
                        .focus_node(focus_node)
                })
            };
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();

            // Check that thumb color is using active color.
            assert!(paints_circle(&tester, slider_render(&tester), ACTIVE_COLOR));

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            // Check that the overlay shows when focused.
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(paints_circle(
                &tester,
                slider_render(&tester),
                OVERLAY_COLOR
            ));

            // Check that the overlay does not show when focused and disabled.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(!paints_circle(
                &tester,
                slider_render(&tester),
                OVERLAY_COLOR
            ));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// The values Dart's keyboard cases record: `startValue`, `currentValue`, `endValue`.
    struct KeyboardValues {
        start: Rc<Cell<f64>>,
        current: Rc<Cell<f64>>,
        end: Rc<Cell<f64>>,
    }

    /// Dart's 'Slider can be incremented and decremented by keyboard shortcuts' app: an
    /// autofocused slider at 0.5 in `text_direction`.
    fn keyboard_slider(tester: &mut WidgetTester, text_direction: TextDirection) -> KeyboardValues {
        tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
        let values = KeyboardValues {
            start: Rc::new(Cell::new(0.0)),
            current: Rc::new(Cell::new(0.5)),
            end: Rc::new(Cell::new(0.0)),
        };
        let (start, current, end) = (
            Rc::clone(&values.start),
            Rc::clone(&values.current),
            Rc::clone(&values.end),
        );
        let theme = default_theme(tester);
        tester.pump_widget(material_app(
            theme,
            Material::new()
                .child(Center::new().child(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        let (start, current, end) =
                            (Rc::clone(&start), Rc::clone(&current), Rc::clone(&end));
                        let (start_set_state, end_set_state) =
                            (Rc::clone(&set_state), Rc::clone(&set_state));
                        let slider = Slider::new(
                            current.get(),
                            Some(Rc::new(move |app: &mut App, new_value: f64| {
                                let current = Rc::clone(&current);
                                set_state(app, &mut |_app| current.set(new_value));
                            })),
                        )
                        .on_change_start(Rc::new(move |app: &mut App, new_value: f64| {
                            let start = Rc::clone(&start);
                            start_set_state(app, &mut |_app| start.set(new_value));
                        }))
                        .on_change_end(Rc::new(move |app: &mut App, new_value: f64| {
                            let end = Rc::clone(&end);
                            end_set_state(app, &mut |_app| end.set(new_value));
                        }))
                        .autofocus(true);
                        Directionality::new(text_direction, slider).into_widget()
                    },
                )))
                .into_widget(),
        ));
        tester.pump_and_settle();
        values
    }

    fn send_arrow(tester: &mut WidgetTester, logical: LogicalKeyboardKey) {
        use waft_services::PhysicalKeyboardKey;
        let physical = if logical == LogicalKeyboardKey::ARROW_RIGHT {
            PhysicalKeyboardKey::ARROW_RIGHT
        } else if logical == LogicalKeyboardKey::ARROW_LEFT {
            PhysicalKeyboardKey::ARROW_LEFT
        } else if logical == LogicalKeyboardKey::ARROW_UP {
            PhysicalKeyboardKey::ARROW_UP
        } else {
            PhysicalKeyboardKey::ARROW_DOWN
        };
        tester.send_key_event(physical, logical);
        tester.pump_and_settle();
    }

    fn assert_keyboard_values(values: &KeyboardValues, start: f64, current: f64, end: f64) {
        assert_eq!(values.start.get(), start);
        assert_eq!(values.current.get(), current);
        assert_eq!(values.end.get(), end);
    }

    /// The keyboard case for `platform`, whose arrows step by `step`: right, left, up and down
    /// in turn, right and left swapped by `text_direction`.
    fn keyboard_shortcuts_step(platform: TargetPlatform, text_direction: TextDirection, step: f64) {
        let mut tester = tester_on(platform);
        let values = keyboard_slider(&mut tester, text_direction);
        let (up, down) = (0.5 + step, 0.5 - step);
        let right = match text_direction {
            TextDirection::Ltr => up,
            TextDirection::Rtl => down,
        };

        send_arrow(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
        assert_keyboard_values(&values, 0.5, right, right);

        send_arrow(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
        assert_keyboard_values(&values, right, 0.5, 0.5);

        send_arrow(&mut tester, LogicalKeyboardKey::ARROW_UP);
        assert_keyboard_values(&values, 0.5, up, up);

        send_arrow(&mut tester, LogicalKeyboardKey::ARROW_DOWN);
        assert_keyboard_values(&values, up, 0.5, 0.5);
    }

    // 'Slider can be incremented and decremented by keyboard shortcuts - LTR' (Android,
    // Fuchsia, Linux, Windows).
    #[test]
    fn the_keyboard_steps_a_slider_by_five_percent_ltr() {
        with_main_thread_stack(|| {
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                keyboard_shortcuts_step(platform, TextDirection::Ltr, 0.05);
            }
        });
    }

    // 'Slider can be incremented and decremented by keyboard shortcuts - LTR' (iOS, macOS).
    #[test]
    fn the_keyboard_steps_a_slider_by_ten_percent_ltr_on_apple_platforms() {
        with_main_thread_stack(|| {
            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                keyboard_shortcuts_step(platform, TextDirection::Ltr, 0.1);
            }
        });
    }

    // 'Slider can be incremented and decremented by keyboard shortcuts - RTL' (Android,
    // Fuchsia, Linux, Windows).
    #[test]
    fn the_keyboard_steps_a_slider_by_five_percent_rtl() {
        with_main_thread_stack(|| {
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                keyboard_shortcuts_step(platform, TextDirection::Rtl, 0.05);
            }
        });
    }

    // 'Slider can be incremented and decremented by keyboard shortcuts - RTL' (iOS, macOS).
    #[test]
    fn the_keyboard_steps_a_slider_by_ten_percent_rtl_on_apple_platforms() {
        with_main_thread_stack(|| {
            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                keyboard_shortcuts_step(platform, TextDirection::Rtl, 0.1);
            }
        });
    }

    /// Dart's `_FixedSizeCircle`: a value indicator that paints a 40 pixel circle and its label.
    #[derive(Debug)]
    struct FixedSizeCircle;

    impl FixedSizeCircle {
        const CIRCLE_DIAMETER: f64 = 40.0;
    }

    impl SliderComponentShape for FixedSizeCircle {
        fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
            Size::square(Self::CIRCLE_DIAMETER)
        }

        fn paint(
            &self,
            app: &mut App,
            context: &mut PaintingContext,
            center: Offset,
            _activation_animation: AnyAnimation<f64>,
            _enable_animation: AnyAnimation<f64>,
            _is_discrete: bool,
            label_painter: &mut TextPainter,
            _parent_box: waft_rendering::AnyRenderBox,
            slider_theme: &SliderThemeData,
            _text_direction: TextDirection,
            _value: f64,
            _text_scale_factor: f64,
            _size_with_overflow: Size,
        ) {
            use waft_painting::ClipContext;
            let color = slider_theme
                .value_indicator_color
                .unwrap_or(Colors::PURPLE.primary());
            context.canvas().draw_circle(
                center,
                (Self::CIRCLE_DIAMETER / 2.0) as f32,
                &waft_embedder::Paint::from_color(color.into()),
            );
            let fonts = PaintingBinding::instance(app).fonts(app);
            label_painter.paint(app.get_mut(fonts), context.canvas(), center);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Dart's `expectIndicatorVisible`: whether the value indicator box, the last render object
    /// in the tree, paints a circle in the theme's value indicator colour and a paragraph.
    fn indicator_visible(tester: &WidgetTester, theme: &SliderThemeData) -> bool {
        let Some(value_indicator_box) = tester.all::<RenderValueIndicator>().into_iter().last()
        else {
            return false;
        };
        let render = value_indicator_box.as_render_object(&tester.cell.borrow());
        let ops = tester.painted_ops(render);
        let color = theme
            .value_indicator_color
            .expect("the case's theme has one");
        let circle = ops.iter().position(|op| {
            matches!(op, Op::DrawPath { path, paint, .. }
                if circle_of(path).is_some() && colors_match(paint.color, color))
        });
        circle.is_some_and(|circle| {
            ops[circle..]
                .iter()
                .any(|op| matches!(op, Op::GlyphRun { .. }))
        })
    }

    /// Dart's `expectValueIndicator` in 'Value indicator appears and disappears when it should'.
    fn expect_value_indicator(
        tester: &mut WidgetTester,
        visible_when_dragged: bool,
        visible_when_released: bool,
        theme: &SliderThemeData,
        divisions: Option<i32>,
        enabled: bool,
    ) {
        let base_theme = ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(Colors::BLUE)
            .build(&tester.cell.borrow());
        let value = 0.45;
        let mut slider =
            Slider::new(value, enabled.then(|| noop()).flatten()).label(format!("{value}"));
        if let Some(divisions) = divisions {
            slider = slider.divisions(divisions);
        }
        let app = material_body(
            default_theme(tester),
            TextDirection::Ltr,
            Center::new().child(crate::theme::Theme::new(
                base_theme,
                SliderTheme::new(theme.clone(), slider),
            )),
        );
        tester.pump_widget(app);
        assert_eq!(indicator_visible(tester, theme), visible_when_released);

        let center = slider_rect(tester).center();
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, center);
        tester.pump_and_settle();
        assert_eq!(indicator_visible(tester, theme), visible_when_dragged);

        tester.up(&gesture);
        tester.pump_and_settle();
        assert_eq!(indicator_visible(tester, theme), visible_when_released);

        // Reset state to avoid state leak.
        tester.pump_widget(SizedBox::new().into_widget());
    }

    /// Dart's `baseSliderTheme.copyWith(showValueIndicator:)` in that group.
    fn value_indicator_theme(
        tester: &WidgetTester,
        show_value_indicator: Option<ShowValueIndicator>,
    ) -> SliderThemeData {
        let base_theme = ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(Colors::BLUE)
            .build(&tester.cell.borrow());
        let mut theme = base_theme
            .slider_theme
            .copy_with()
            .value_indicator_color(Colors::RED.primary())
            .value_indicator_shape(Rc::new(FixedSizeCircle));
        if let Some(show_value_indicator) = show_value_indicator {
            theme = theme.show_value_indicator(show_value_indicator);
        }
        theme
    }

    /// The four `expectValueIndicator` calls of each case: discrete enabled, discrete
    /// disabled, continuous enabled and continuous disabled, with what they expect while
    /// dragged and once released.
    fn expect_value_indicators(
        show_value_indicator: Option<ShowValueIndicator>,
        discrete: (bool, bool),
        continuous: (bool, bool),
    ) {
        let mut tester = tester_on(TargetPlatform::Android);
        let theme = value_indicator_theme(&tester, show_value_indicator);
        expect_value_indicator(&mut tester, discrete.0, discrete.1, &theme, Some(3), true);
        expect_value_indicator(&mut tester, false, false, &theme, Some(3), false);
        expect_value_indicator(&mut tester, continuous.0, continuous.1, &theme, None, true);
        expect_value_indicator(&mut tester, false, false, &theme, None, false);
    }

    // 'Value indicator appears and disappears when it should:' 'showValueIndicator set to
    // onlyForDiscrete'.
    #[test]
    fn the_value_indicator_shows_only_for_discrete() {
        with_main_thread_stack(|| {
            // The default value is onlyForDiscrete. No modification is needed.
            expect_value_indicators(None, (true, false), (false, false));
        });
    }

    // 'showValueIndicator set to onlyForContinuous'.
    #[test]
    fn the_value_indicator_shows_only_for_continuous() {
        with_main_thread_stack(|| {
            expect_value_indicators(
                Some(ShowValueIndicator::OnlyForContinuous),
                (false, false),
                (true, false),
            );
        });
    }

    // 'showValueIndicator set to onDrag'.
    #[test]
    fn the_value_indicator_shows_on_drag() {
        with_main_thread_stack(|| {
            expect_value_indicators(
                Some(ShowValueIndicator::OnDrag),
                (true, false),
                (true, false),
            );
        });
    }

    // 'showValueIndicator set to never'.
    #[test]
    fn the_value_indicator_never_shows() {
        with_main_thread_stack(|| {
            expect_value_indicators(
                Some(ShowValueIndicator::Never),
                (false, false),
                (false, false),
            );
        });
    }

    // 'showValueIndicator set to alwaysVisible'.
    #[test]
    fn the_value_indicator_is_always_visible() {
        with_main_thread_stack(|| {
            expect_value_indicators(
                Some(ShowValueIndicator::AlwaysVisible),
                (true, true),
                (true, true),
            );
        });
    }

    // 'Slider.adaptive'.
    #[test]
    fn slider_adaptive() {
        with_main_thread_stack(|| {
            let value = Rc::new(Cell::new(0.5));
            let build_frame = |tester: &WidgetTester, platform: TargetPlatform| {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .build(&tester.cell.borrow());
                let value = Rc::clone(&value);
                material_app(
                    theme,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let on_changed_value = Rc::clone(&value);
                        Material::new()
                            .child(Center::new().child(Slider::adaptive(
                                value.get(),
                                Some(Rc::new(move |app: &mut App, new_value: f64| {
                                    let value = Rc::clone(&on_changed_value);
                                    set_state(app, &mut |_app| value.set(new_value));
                                })),
                            )))
                            .into_widget()
                    })
                    .into_widget(),
                )
            };

            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                let mut tester = tester_on(TargetPlatform::Android);
                value.set(0.5);
                tester.pump_widget(build_frame(&tester, platform));
                assert_eq!(tester.find_by_type::<Slider>().len(), 1);
                let cupertino = tester.find_by_type::<CupertinoSlider>();
                assert_eq!(cupertino.len(), 1);

                assert_eq!(value.get(), 0.5, "on {platform:?}");
                let center = tester
                    .global_rect(tester.render_box_of(cupertino[0]))
                    .center();
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, center);
                // Drag to the right end of the track.
                tester.move_to(&mut gesture, center + Offset::new(600.0, 0.0), true);
                assert_eq!(value.get(), 1.0, "on {platform:?}");
                tester.up(&gesture);
            }

            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                let mut tester = tester_on(TargetPlatform::Android);
                value.set(0.5);
                tester.pump_widget(build_frame(&tester, platform));
                tester.pump_and_settle(); // Finish the theme change animation.
                assert_eq!(tester.find_by_type::<Slider>().len(), 1);
                assert!(tester.find_by_type::<CupertinoSlider>().is_empty());

                assert_eq!(value.get(), 0.5, "on {platform:?}");
                let center = slider_rect(&tester).center();
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, center);
                // Drag to the right end of the track.
                tester.move_to(&mut gesture, center + Offset::new(600.0, 0.0), true);
                assert_eq!(value.get(), 1.0, "on {platform:?}");
                tester.up(&gesture);
            }
        });
    }

    // 'Slider respects height from theme'.
    #[test]
    fn a_slider_respects_the_height_of_its_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                |app, context, slider| {
                    let slider_theme = SliderTheme::of(app, context)
                        .copy_with()
                        .tick_mark_shape(Rc::new(TallSliderTickMarkShape));
                    waft_widgets::IntrinsicHeight::new()
                        .child(SliderTheme::new(slider_theme, slider))
                        .into_widget()
                },
                |slider| slider.divisions(4),
            ));

            assert_eq!(slider_size(&tester).height(), 200.0);
        });
    }

    pub(crate) fn active_cursor(tester: &WidgetTester) -> MouseCursorRef {
        tester.active_cursor(1)
    }

    pub(crate) fn is_cursor(
        cursor: &MouseCursorRef,
        expected: waft_services::SystemMouseCursor,
    ) -> bool {
        let expected: MouseCursorRef = expected.into();
        **cursor == *expected
    }

    // 'Slider changes mouse cursor when hovered'.
    #[test]
    fn a_slider_changes_the_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            use waft_services::SystemMouseCursors;
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let frame = |slider: Slider| {
                material_body(
                    theme.clone(),
                    TextDirection::Ltr,
                    Center::new().child(
                        waft_widgets::MouseRegion::new()
                            .cursor(SystemMouseCursors::FORBIDDEN.into())
                            .child(slider),
                    ),
                )
            };
            // Test Slider() constructor
            tester.pump_widget(frame(
                Slider::new(0.5, noop()).mouse_cursor(SystemMouseCursors::TEXT.into()),
            ));

            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, slider_rect(&tester).center());

            tester.pump(Duration::ZERO);

            assert!(is_cursor(&active_cursor(&tester), SystemMouseCursors::TEXT));

            // Test Slider.adaptive() constructor
            tester.pump_widget(frame(
                Slider::adaptive(0.5, noop()).mouse_cursor(SystemMouseCursors::TEXT.into()),
            ));

            assert!(is_cursor(&active_cursor(&tester), SystemMouseCursors::TEXT));

            // Test default cursor
            tester.pump_widget(frame(Slider::new(0.5, noop())));

            assert!(is_cursor(
                &active_cursor(&tester),
                SystemMouseCursors::CLICK
            ));
        });
    }

    // 'Slider WidgetStateMouseCursor resolves correctly'.
    #[test]
    fn a_slider_resolves_a_widget_state_mouse_cursor() {
        with_main_thread_stack(|| {
            use waft_services::{PhysicalKeyboardKey, SystemMouseCursors};
            let system_default_cursor = SystemMouseCursors::BASIC;
            let regular_cursor = SystemMouseCursors::CLICK;
            let disabled_cursor = SystemMouseCursors::FORBIDDEN;
            let focused_cursor = SystemMouseCursors::PRECISE;
            let hovered_cursor = SystemMouseCursors::GRAB;
            let dragged_cursor = SystemMouseCursors::MOVE;

            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let build_frame = |enabled: bool| {
                // Dart's `_StateDependentMouseCursor`.
                let cursor = WidgetStateMouseCursor::resolve_with(
                    move |states: &WidgetStates| {
                        let cursor = if states.contains(&WidgetState::Disabled) {
                            disabled_cursor
                        } else if states.contains(&WidgetState::Focused) {
                            focused_cursor
                        } else if states.contains(&WidgetState::Dragged) {
                            dragged_cursor
                        } else if states.contains(&WidgetState::Hovered) {
                            hovered_cursor
                        } else {
                            regular_cursor
                        };
                        cursor.into()
                    },
                    Some("_StateDependentMouseCursor"),
                );
                material_body(
                    theme.clone(),
                    TextDirection::Ltr,
                    waft_widgets::Column::new()
                        .main_axis_alignment(waft_rendering::MainAxisAlignment::Center)
                        .children([Center::new()
                            .child(
                                Slider::new(0.5, enabled.then(noop).flatten())
                                    .mouse_cursor(cursor.into()),
                            )
                            .into_widget()]),
                )
            };

            let mut gesture = WidgetTester::mouse(1, 0);

            // Dart's tester starts with an empty view; this one has none until it pumps.
            tester.pump_widget(SizedBox::new().into_widget());

            // System default.
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), system_default_cursor));

            // Disabled.
            tester.pump_widget(build_frame(false));
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), disabled_cursor));

            // Regular.
            tester.pump_widget(build_frame(true));
            assert!(is_cursor(&active_cursor(&tester), regular_cursor));

            // Hovered.
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), hovered_cursor));

            // Dragged.
            let mut pressed = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            pressed.position = gesture.position;
            tester.down(&mut pressed, slider_rect(&tester).center());
            let at = pressed.position + Offset::new(20.0, 0.0);
            tester.move_to(&mut pressed, at, true);
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), dragged_cursor));

            // Hovered.
            tester.up(&pressed);
            gesture.position = pressed.position;
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), hovered_cursor));

            // Focused.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), focused_cursor));

            // System default.
            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump(Duration::ZERO);
            assert!(is_cursor(&active_cursor(&tester), system_default_cursor));
        });
    }

    /// Dart's `MaterialApp(theme: ThemeData(platform: platform), home: Material(child: slider))`.
    fn platform_slider_app(
        tester: &WidgetTester,
        platform: TargetPlatform,
        slider: Slider,
    ) -> WidgetRef {
        let theme = ThemeData::builder()
            .platform(platform)
            .build(&tester.cell.borrow());
        material_app(theme, Material::new().child(slider).into_widget())
    }

    // 'Slider paints thumbColor'.
    #[test]
    fn a_slider_paints_its_thumb_color() {
        with_main_thread_stack(|| {
            const COLOR: Color = Color::new(0xffffc107);
            let mut tester = tester_on(TargetPlatform::Android);
            let slider_adaptive = platform_slider_app(
                &tester,
                TargetPlatform::IOS,
                Slider::new(0.0, noop()).thumb_color(COLOR),
            );

            tester.pump_widget(slider_adaptive);
            tester.pump_and_settle();

            assert!(paints_circle(&tester, slider_render(&tester), COLOR));
        });
    }

    // 'Slider.adaptive paints thumbColor on Android'.
    #[test]
    fn slider_adaptive_paints_its_thumb_color_on_android() {
        with_main_thread_stack(|| {
            const COLOR: Color = Color::new(0xffffc107);
            let mut tester = tester_on(TargetPlatform::Android);
            let slider_adaptive = platform_slider_app(
                &tester,
                TargetPlatform::Android,
                Slider::adaptive(0.0, noop()).thumb_color(COLOR),
            );

            tester.pump_widget(slider_adaptive);
            tester.pump_and_settle();

            assert!(paints_circle(&tester, slider_render(&tester), COLOR));
        });
    }

    /// The colour of the sixth rounded rect the `CupertinoSlider` paints: Dart's
    /// `paints..rrect()` five times then `..rrect(color:)`. A blurred shadow rect is Valo's
    /// `RRectBlur`, where Dart's is an `rrect` call with a mask filter.
    fn sixth_cupertino_rrect_color(tester: &WidgetTester) -> Option<waft_embedder::valo::Color> {
        let cupertino = tester.find_by_type::<CupertinoSlider>()[0];
        // The `Opacity` the Cupertino slider builds composites; what it paints is its child's.
        let opacity = tester.render_box_of(cupertino).as_object();
        let mut render = None;
        opacity.visit_children(&tester.cell.borrow(), &mut |child| render = Some(child));
        let render = render.expect("the opacity has the slider's render object");
        tester
            .painted_ops(render)
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { paint, .. }
                | Op::DrawRect { paint, .. }
                | Op::RRectBlur { paint, .. } => Some(paint.color),
                _ => None,
            })
            .nth(5)
    }

    // 'If thumbColor is null, it defaults to CupertinoColors.white'.
    #[test]
    fn an_adaptive_thumb_color_defaults_to_cupertino_white() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let slider_adaptive =
                platform_slider_app(&tester, TargetPlatform::IOS, Slider::adaptive(0.0, noop()));

            tester.pump_widget(slider_adaptive);
            tester.pump_and_settle();

            assert_eq!(
                sixth_cupertino_rrect_color(&tester),
                Some(CupertinoColors::WHITE.color().into())
            );
        });
    }

    // 'Slider.adaptive passes thumbColor to CupertinoSlider'.
    #[test]
    fn slider_adaptive_passes_its_thumb_color_to_the_cupertino_slider() {
        with_main_thread_stack(|| {
            const COLOR: Color = Color::new(0xffffc107);
            let mut tester = tester_on(TargetPlatform::Android);
            let slider_adaptive = platform_slider_app(
                &tester,
                TargetPlatform::IOS,
                Slider::adaptive(0.0, noop()).thumb_color(COLOR),
            );

            tester.pump_widget(slider_adaptive);
            tester.pump_and_settle();

            assert_eq!(sixth_cupertino_rrect_color(&tester), Some(COLOR.into()));
        });
    }

    // 'Drag gesture uses provided gesture settings'.
    #[test]
    fn the_drag_gesture_uses_the_gesture_settings_given() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/103566
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(0.5));
            let drag_started = Rc::new(Cell::new(false));
            let slider_end = Rc::new(Cell::new(false));
            let theme = default_theme(&tester);
            let build = |touch_slop: f64, with_end: bool| {
                let (value, drag_started, slider_end) = (
                    Rc::clone(&value),
                    Rc::clone(&drag_started),
                    Rc::clone(&slider_end),
                );
                material_body(
                    theme.clone(),
                    TextDirection::Ltr,
                    StatefulBuilder::new(move |app, context, set_state| {
                        let on_changed_value = Rc::clone(&value);
                        let started = Rc::clone(&drag_started);
                        let ended = Rc::clone(&slider_end);
                        let mut slider = Slider::new(
                            value.get(),
                            Some(Rc::new(move |app: &mut App, new_value: f64| {
                                let value = Rc::clone(&on_changed_value);
                                set_state(app, &mut |_app| value.set(new_value));
                            })),
                        );
                        if with_end {
                            slider = slider.on_change_end(Rc::new(
                                move |_app: &mut App, _end_value: f64| ended.set(true),
                            ));
                        }
                        let data = MediaQuery::of(app, context)
                            .copy_with()
                            .gesture_settings(DeviceGestureSettings::new(Some(touch_slop)));
                        Center::new()
                            .child(
                                waft_widgets::GestureDetector::new()
                                    .behavior(waft_rendering::HitTestBehavior::DeferToChild)
                                    .on_horizontal_drag_start(Rc::new(
                                        move |_app: &mut App, _details: DragStartDetails| {
                                            started.set(true)
                                        },
                                    )
                                        as waft_gestures::GestureDragStartCallback)
                                    .child(MediaQuery::new(data, slider).into_widget()),
                            )
                            .into_widget()
                    }),
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

            assert_eq!(value.get(), 0.5);
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
                    .first::<RenderSlider>()
                    .gesture_settings(&tester.cell.borrow()),
                Some(DeviceGestureSettings::new(Some(10.0)))
            );
        });
    }

    /// Dart's `allowedInteraction` cases' app: a slider at `value` in the centre of a material,
    /// logging its callbacks, under `theme`.
    fn interaction_app(
        theme: ThemeData,
        value: &Rc<Cell<f64>>,
        logs: &Rc<RefCell<Vec<&'static str>>>,
        allowed_interaction: Option<SliderInteraction>,
    ) -> WidgetRef {
        let (value, logs) = (Rc::clone(value), Rc::clone(logs));
        material_app(
            theme,
            Material::new()
                .child(Center::new().child(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        let (start_logs, changed_logs, end_logs) =
                            (Rc::clone(&logs), Rc::clone(&logs), Rc::clone(&logs));
                        let on_changed_value = Rc::clone(&value);
                        let mut slider = Slider::new(
                            value.get(),
                            Some(Rc::new(move |app: &mut App, new_value: f64| {
                                changed_logs.borrow_mut().push("onChanged");
                                let value = Rc::clone(&on_changed_value);
                                set_state(app, &mut |_app| value.set(new_value));
                            })),
                        )
                        .on_change_start(Rc::new(move |_app: &mut App, _value: f64| {
                            start_logs.borrow_mut().push("onChangeStart")
                        }))
                        .on_change_end(Rc::new(
                            move |_app: &mut App, _value: f64| {
                                end_logs.borrow_mut().push("onChangeEnd")
                            },
                        ));
                        if let Some(allowed_interaction) = allowed_interaction {
                            slider = slider.allowed_interaction(allowed_interaction);
                        }
                        slider.into_widget()
                    },
                )))
                .into_widget(),
        )
    }

    // 'Slider.allowedInteraction' 'SliderInteraction.tapOnly'.
    #[test]
    fn slider_interaction_tap_only() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(1.0));
            // (slider's left padding (overlayRadius), windowHeight / 2)
            let start_of_the_slider_track = Offset::new(24.0, 300.0);
            let center_of_the_slide_track = Offset::new(400.0, 300.0);
            let logs = Rc::new(RefCell::new(Vec::new()));

            // allow tap only
            let theme = default_theme(&tester);
            tester.pump_widget(interaction_app(
                theme,
                &value,
                &logs,
                Some(SliderInteraction::TapOnly),
            ));

            assert!(logs.borrow().is_empty());

            // test tap
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center_of_the_slide_track);
            tester.pump(Duration::ZERO);
            // changes from 1.0 -> 0.5
            assert_eq!(value.get(), 0.5);
            assert_eq!(*logs.borrow(), ["onChangeStart", "onChanged"]);

            // test slide
            tester.move_to(&mut gesture, start_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // has no effect, remains 0.5
            assert_eq!(value.get(), 0.5);
            assert_eq!(*logs.borrow(), ["onChangeStart", "onChanged"]);

            tester.up(&gesture);
            tester.pump(Duration::ZERO);
            assert_eq!(
                *logs.borrow(),
                ["onChangeStart", "onChanged", "onChangeEnd"]
            );
        });
    }

    // 'SliderInteraction.tapAndSlide (default)'.
    #[test]
    fn slider_interaction_tap_and_slide_is_the_default() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(1.0));
            // (slider's left padding (overlayRadius), windowHeight / 2)
            let start_of_the_slider_track = Offset::new(24.0, 300.0);
            let center_of_the_slide_track = Offset::new(400.0, 300.0);
            let end_of_the_slider_track = Offset::new(800.0 - 24.0, 300.0);
            let logs = Rc::new(RefCell::new(Vec::new()));

            let theme = default_theme(&tester);
            tester.pump_widget(interaction_app(theme, &value, &logs, None));

            assert!(logs.borrow().is_empty());

            // Test tap.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center_of_the_slide_track);
            tester.pump(Duration::ZERO);
            // changes from 1.0 -> 0.5
            assert_eq!(value.get(), 0.5);
            assert_eq!(*logs.borrow(), ["onChangeStart", "onChanged"]);

            // test slide
            tester.move_to(&mut gesture, start_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // changes from 0.5 -> 0.0
            assert_eq!(value.get(), 0.0);
            tester.move_to(&mut gesture, end_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // changes from 0.0 -> 1.0
            assert_eq!(value.get(), 1.0);
            assert_eq!(
                *logs.borrow(),
                ["onChangeStart", "onChanged", "onChanged", "onChanged"]
            );

            tester.up(&gesture);
            tester.pump(Duration::ZERO);

            assert_eq!(
                *logs.borrow(),
                [
                    "onChangeStart",
                    "onChanged",
                    "onChanged",
                    "onChanged",
                    "onChangeEnd"
                ]
            );
        });
    }

    /// Whether `render` paints a circle in `color`, centred at `x`, of `radius` if given:
    /// Dart's `paints..circle(color:, radius:, x:)`.
    fn paints_circle_at(
        tester: &WidgetTester,
        render: AnyRenderObject,
        color: Color,
        x: f64,
        radius: Option<f64>,
    ) -> bool {
        tester.painted_ops(render).iter().any(|op| match op {
            Op::DrawPath { path, paint, .. } if colors_match(paint.color, color) => circle_of(path)
                .is_some_and(|circle| {
                    (circle.center().dx() - x).abs() < 1e-3
                        && radius.is_none_or(|radius| (circle.width() / 2.0 - radius).abs() < 1e-3)
                }),
            _ => false,
        })
    }

    // 'SliderInteraction.slideOnly'.
    #[test]
    fn slider_interaction_slide_only() {
        with_main_thread_stack(|| {
            const OVERLAY_RADIUS: f64 = 23.0;
            let overlay_color = Colors::RED.primary();
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(1.0));
            // (slider's left padding (overlayRadius), windowHeight / 2)
            let start_of_the_slider_track = Offset::new(OVERLAY_RADIUS, 300.0);
            let center_of_the_slider_track = Offset::new(400.0, 300.0);
            let end_of_the_slider_track = Offset::new(800.0 - OVERLAY_RADIUS, 300.0);
            let x_pos_thumb = |t: f64| {
                start_of_the_slider_track.dx()
                    + (end_of_the_slider_track.dx() - start_of_the_slider_track.dx()) * t
            };
            let logs = Rc::new(RefCell::new(Vec::new()));

            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        .overlay_color(overlay_color)
                        .overlay_shape(Rc::new(
                            RoundSliderOverlayShape::new().overlay_radius(OVERLAY_RADIUS),
                        )),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(interaction_app(
                theme,
                &value,
                &logs,
                Some(SliderInteraction::SlideOnly),
            ));

            assert!(logs.borrow().is_empty());

            // Test tap.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center_of_the_slider_track);
            // Start animation.
            tester.pump(Duration::ZERO);
            // Go to mid-animation frame.
            tester.pump(K_RADIAL_REACTION_DURATION.mul_f64(0.5));
            assert!(paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                None
            ));
            // We have a non-linear asymmetric curve, so just verify the radius is not full.
            assert!(!paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                Some(OVERLAY_RADIUS)
            ));

            // Finish animation.
            tester.pump_and_settle();
            // Overlay drawn.
            assert!(paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                Some(OVERLAY_RADIUS)
            ));
            // Has no other effect as tap is disabled, remains 1.0.
            assert_eq!(value.get(), 1.0);
            assert_eq!(*logs.borrow(), ["onChangeStart"]);

            // Test slide.
            tester.move_to(&mut gesture, start_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // Changes from 1.0 -> 0.5.
            assert_eq!(value.get(), 0.5);
            // Overlay still there.
            assert!(paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                Some(OVERLAY_RADIUS)
            ));
            tester.move_to(&mut gesture, end_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // Changes from 0.0 -> 1.0.
            assert_eq!(value.get(), 1.0);
            assert_eq!(*logs.borrow(), ["onChangeStart", "onChanged", "onChanged"]);
            // Overlay still there.
            assert!(paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                Some(OVERLAY_RADIUS)
            ));

            tester.up(&gesture);

            // Start release animation.
            tester.pump(Duration::ZERO);
            // Go to mid-animation frame.
            tester.pump(K_RADIAL_REACTION_DURATION.mul_f64(0.5));
            assert!(paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                None
            ));
            // Verify the radius is not full.
            assert!(!paints_circle_at(
                &tester,
                slider_render(&tester),
                overlay_color,
                x_pos_thumb(value.get()),
                Some(OVERLAY_RADIUS)
            ));

            tester.pump_and_settle();
            // No overlay drawn.
            let color: waft_embedder::valo::Color = overlay_color.into();
            assert!(!tester.painted_ops(slider_render(&tester)).iter().any(|op| matches!(op,
                Op::DrawPath { path, paint, .. } if paint.color == color
                    && circle_of(path).is_some_and(|circle| (circle.width() / 2.0 - OVERLAY_RADIUS).abs() < 1e-3))));

            assert_eq!(
                *logs.borrow(),
                ["onChangeStart", "onChanged", "onChanged", "onChangeEnd"]
            );
        });
    }

    // 'SliderInteraction.slideThumb'.
    #[test]
    fn slider_interaction_slide_thumb() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(1.0));
            // (slider's left padding (overlayRadius), windowHeight / 2)
            let start_of_the_slider_track = Offset::new(24.0, 300.0);
            let center_of_the_slider_track = Offset::new(400.0, 300.0);
            let end_of_the_slider_track = Offset::new(800.0 - 24.0, 300.0);
            let logs = Rc::new(RefCell::new(Vec::new()));

            let theme = default_theme(&tester);
            tester.pump_widget(interaction_app(
                theme,
                &value,
                &logs,
                Some(SliderInteraction::SlideThumb),
            ));

            assert!(logs.borrow().is_empty());

            // test tap
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center_of_the_slider_track);
            tester.pump(Duration::ZERO);
            // has no effect, remains 1.0
            assert_eq!(value.get(), 1.0);
            assert!(logs.borrow().is_empty());

            // test slide
            tester.move_to(&mut gesture, start_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // has no effect, remains 1.0
            assert_eq!(value.get(), 1.0);
            assert!(logs.borrow().is_empty());

            // test slide thumb
            tester.up(&gesture);
            tester.down(&mut gesture, end_of_the_slider_track); // where the thumb is
            tester.pump(Duration::ZERO);
            // has no effect, remains 1.0
            assert_eq!(value.get(), 1.0);
            assert_eq!(*logs.borrow(), ["onChangeStart"]);

            tester.move_to(&mut gesture, center_of_the_slider_track, true);
            tester.pump(Duration::ZERO);
            // changes from 1.0 -> 0.5
            assert_eq!(value.get(), 0.5);
            assert_eq!(*logs.borrow(), ["onChangeStart", "onChanged"]);

            // test tap inside overlay but not on thumb, then slide
            tester.up(&gesture);
            // default overlay radius is 12, so 10 is inside the overlay
            tester.down(
                &mut gesture,
                center_of_the_slider_track + Offset::new(-10.0, 0.0),
            );
            tester.pump(Duration::ZERO);
            // changes from 1.0 -> 0.5
            assert_eq!(value.get(), 0.5);
            assert_eq!(
                *logs.borrow(),
                ["onChangeStart", "onChanged", "onChangeEnd", "onChangeStart"]
            );

            tester.move_to(
                &mut gesture,
                end_of_the_slider_track + Offset::new(-10.0, 0.0),
                true,
            );
            tester.pump(Duration::ZERO);
            // changes from 0.5 -> 1.0
            assert_eq!(value.get(), 1.0);
            assert_eq!(
                *logs.borrow(),
                [
                    "onChangeStart",
                    "onChanged",
                    "onChangeEnd",
                    "onChangeStart",
                    "onChanged"
                ]
            );

            tester.up(&gesture);
            tester.pump(Duration::ZERO);

            assert_eq!(
                *logs.borrow(),
                [
                    "onChangeStart",
                    "onChanged",
                    "onChangeEnd",
                    "onChangeStart",
                    "onChanged",
                    "onChangeEnd"
                ]
            );
        });
    }

    /// Dart's `LoggingRoundedRectSliderTrackShape`: a [`RoundedRectSliderTrackShape`] that logs
    /// the secondary offset it paints with.
    #[derive(Debug)]
    struct LoggingRoundedRectSliderTrackShape {
        secondary_offset_log: Rc<RefCell<Vec<Option<Offset>>>>,
    }

    impl crate::slider_parts::SliderTrackShape for LoggingRoundedRectSliderTrackShape {
        fn get_preferred_rect(
            &self,
            app: &App,
            parent_box: waft_rendering::AnyRenderBox,
            offset: Offset,
            slider_theme: &SliderThemeData,
            is_enabled: Option<bool>,
            is_discrete: Option<bool>,
        ) -> Rect {
            crate::slider_parts::SliderTrackShape::get_preferred_rect(
                &RoundedRectSliderTrackShape::new(),
                app,
                parent_box,
                offset,
                slider_theme,
                Some(is_enabled.unwrap_or(true)),
                Some(is_discrete.unwrap_or(false)),
            )
        }

        fn paint(
            &self,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
            parent_box: waft_rendering::AnyRenderBox,
            slider_theme: &SliderThemeData,
            enable_animation: AnyAnimation<f64>,
            thumb_center: Offset,
            secondary_offset: Option<Offset>,
            is_enabled: Option<bool>,
            is_discrete: Option<bool>,
            text_direction: TextDirection,
        ) {
            self.secondary_offset_log
                .borrow_mut()
                .push(secondary_offset);
            crate::slider_parts::SliderTrackShape::paint(
                &RoundedRectSliderTrackShape::new(),
                app,
                context,
                offset,
                parent_box,
                slider_theme,
                enable_animation,
                thumb_center,
                secondary_offset,
                Some(is_enabled.unwrap_or(false)),
                Some(is_discrete.unwrap_or(false)),
                text_direction,
            );
        }

        fn is_rounded(&self) -> bool {
            true
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // 'Can update renderObject when secondaryTrackValue is updated'.
    #[test]
    fn the_render_object_updates_with_the_secondary_track_value() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let log = Rc::new(RefCell::new(Vec::new()));
            let logging_track_shape = Rc::new(LoggingRoundedRectSliderTrackShape {
                secondary_offset_log: Rc::clone(&log),
            });
            let theme = ThemeData::builder()
                .slider_theme(SliderThemeData::new().track_shape(logging_track_shape))
                .build(&tester.cell.borrow());
            let build_slider = |secondary_track_value: Option<f64>| {
                let mut slider = Slider::new(0.0, noop());
                if let Some(secondary_track_value) = secondary_track_value {
                    slider = slider.secondary_track_value(secondary_track_value);
                }
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(Center::new().child(slider))
                        .into_widget(),
                )
            };

            tester.pump_widget(build_slider(None));
            tester.pump_and_settle();
            assert_eq!(*log.borrow().last().unwrap(), None);

            tester.pump_widget(build_slider(Some(0.2)));
            tester.pump_and_settle();
            assert_eq!(
                *log.borrow().last().unwrap(),
                Some(Offset::new(174.4, 300.0))
            );

            tester.pump_widget(build_slider(Some(0.5)));
            tester.pump_and_settle();
            assert_eq!(
                *log.borrow().last().unwrap(),
                Some(Offset::new(400.0, 300.0))
            );
        });
    }

    // 'Discrete Slider does not apply thumb padding in a non-rounded track shape'.
    #[test]
    fn a_discrete_slider_applies_no_thumb_padding_on_a_non_rounded_track() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/161805
            let mut tester = tester_on(TargetPlatform::Android);
            // The default track left and right padding.
            let slider_padding = 24.0;
            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        // Thumb padding is applied based on the track height.
                        .track_height(100.0)
                        .track_shape(Rc::new(
                            crate::slider_parts::RectangularSliderTrackShape::new(),
                        )),
                )
                .build(&tester.cell.borrow());
            let build_slider = |value: f64| {
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            SizedBox::new()
                                .width(300.0)
                                .child(Slider::new(value, noop()).max(100.0).divisions(100)),
                        )
                        .into_widget(),
                )
            };

            tester.pump_widget(build_slider(0.0));

            let primary = theme.color_scheme.primary;
            let paints_thumb_at = |tester: &WidgetTester, x: f64| {
                let color: waft_embedder::valo::Color = primary.into();
                tester
                    .painted_ops(slider_render(tester))
                    .iter()
                    .any(|op| match op {
                        Op::DrawPath { path, paint, .. } if paint.color == color => circle_of(path)
                            .is_some_and(|circle| {
                                (circle.center().dx() - x).abs() < 1e-3
                                    && (circle.center().dy() - 300.0).abs() < 1e-3
                            }),
                        _ => false,
                    })
            };
            assert!(paints_thumb_at(&tester, slider_padding));

            tester.pump_widget(build_slider(100.0));
            tester.pump_and_settle();

            assert!(paints_thumb_at(&tester, 800.0 - slider_padding));
        });
    }

    // 'Slider does not crash at zero area'.
    #[test]
    fn a_slider_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(SizedBox::shrink().child(Slider::new(1.0, noop()))))
                    .into_widget(),
            ));
            assert_eq!(slider_size(&tester), Size::ZERO);
        });
    }

    /// Dart's `LoggingValueIndicatorShape`: a value indicator shape that logs the width of
    /// the label painter it is given.
    #[derive(Debug)]
    struct LoggingValueIndicatorShape {
        log_painter_widths: Rc<RefCell<Vec<f64>>>,
    }

    impl SliderComponentShape for LoggingValueIndicatorShape {
        fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
            Size::new(10.0, 10.0)
        }

        fn paint(
            &self,
            _app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
            _activation_animation: AnyAnimation<f64>,
            _enable_animation: AnyAnimation<f64>,
            _is_discrete: bool,
            label_painter: &mut TextPainter,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _text_direction: TextDirection,
            _value: f64,
            _text_scale_factor: f64,
            _size_with_overflow: Size,
        ) {
            assert!(label_painter.text().is_some());
            self.log_painter_widths
                .borrow_mut()
                .push(label_painter.width());
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // 'Slider label respects horizontal buffer and avoids screen overflow'.
    #[test]
    fn the_label_keeps_the_horizontal_buffer_and_avoids_screen_overflow() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let log_painters = Rc::new(RefCell::new(Vec::new()));
            let shape = Rc::new(LoggingValueIndicatorShape {
                log_painter_widths: Rc::clone(&log_painters),
            });
            let theme = default_theme(&tester);
            #[expect(deprecated, reason = "Dart's case uses the deprecated `always`")]
            let show_value_indicator = ShowValueIndicator::Always;
            tester.pump_widget(material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .body(SliderTheme::new(
                        SliderThemeData::new()
                            .show_value_indicator(show_value_indicator)
                            .value_indicator_shape(shape),
                        Slider::new(0.5, noop()).divisions(10).label(
                            "A very long label string that exceeds standard screen widths to test clipping behavior",
                        ),
                    ))
                    .into_widget(),
            ));

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).center());
            tester.pump_and_settle();

            let screen_width = WidgetTester::SIZE.width();

            assert!(!log_painters.borrow().is_empty());
            assert!(*log_painters.borrow().last().unwrap() <= screen_width - 64.0);

            tester.up(&gesture);
        });
    }

    /// Dart's `_PositionRecordingTickMarkShape`: records the center of each tick mark as it is
    /// painted.
    #[derive(Debug)]
    struct PositionRecordingTickMarkShape {
        positions: Rc<RefCell<Vec<Offset>>>,
    }

    impl SliderTickMarkShape for PositionRecordingTickMarkShape {
        fn get_preferred_size(&self, _slider_theme: &SliderThemeData, _is_enabled: bool) -> Size {
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
            _thumb_center: Offset,
            _is_enabled: bool,
            _text_direction: TextDirection,
        ) {
            self.positions.borrow_mut().push(center);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Dart's `pumpDiscreteSlider`: pumps a slider whose tick mark centers are recorded, and
    /// returns the centers of the `divisions + 1` ticks painted on the first frame, ordered
    /// left to right. Dart's `ValueListenableBuilder` over the value is a `StatefulBuilder`
    /// whose `onChanged` sets it.
    fn pump_discrete_slider(
        tester: &mut WidgetTester,
        value: &Rc<Cell<f64>>,
        divisions: i32,
        text_direction: TextDirection,
        year2023: Option<bool>,
    ) -> Vec<Offset> {
        let tick_centers = Rc::new(RefCell::new(Vec::new()));
        let shape = Rc::new(PositionRecordingTickMarkShape {
            positions: Rc::clone(&tick_centers),
        });
        let theme = default_theme(tester);
        let value = Rc::clone(value);
        tester.pump_widget(material_app(
            theme,
            Directionality::new(
                text_direction,
                SliderTheme::new(
                    SliderThemeData::new().tick_mark_shape(shape),
                    Material::new().child(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let on_changed_value = Rc::clone(&value);
                            let mut slider = Slider::new(
                                value.get(),
                                Some(Rc::new(move |app: &mut App, new_value: f64| {
                                    let value = Rc::clone(&on_changed_value);
                                    set_state(app, &mut |_app| value.set(new_value));
                                })),
                            )
                            .max(f64::from(divisions))
                            .divisions(divisions);
                            if let Some(year2023) = year2023 {
                                #[expect(deprecated, reason = "Dart's case passes `year2023`")]
                                {
                                    slider = slider.year2023(year2023);
                                }
                            }
                            slider.into_widget()
                        },
                    ))),
                ),
            )
            .into_widget(),
        ));
        // Later repaints append to the list, so keep only the first frame's ticks.
        let first_frame: Vec<Offset> = tick_centers
            .borrow()
            .iter()
            .take(divisions as usize + 1)
            .copied()
            .collect();
        assert_eq!(first_frame.len(), divisions as usize + 1);
        first_frame
    }

    /// Dart's 'Discrete Slider snaps at the visual midpoint between ticks ($name)'.
    fn snaps_at_the_visual_midpoint(text_direction: TextDirection, year2023: Option<bool>) {
        let mut tester = tester_on(TargetPlatform::Android);
        const DIVISIONS: i32 = 10;
        let value = Rc::new(Cell::new(0.0));
        let tick_centers =
            pump_discrete_slider(&mut tester, &value, DIVISIONS, text_direction, year2023);
        // Ticks are painted left to right in pixel space regardless of text
        // direction, so in RTL the tick at pixel index i has the value divisions - i.
        let value_of_tick = |pixel_index: i32| match text_direction {
            TextDirection::Ltr => f64::from(pixel_index),
            TextDirection::Rtl => f64::from(DIVISIONS - pixel_index),
        };

        // A small offset keeps the taps well clear of the exact midpoint while
        // staying smaller than the smallest drift of the old formula.
        const MIDPOINT_OFFSET: f64 = 0.05;
        for i in 0..DIVISIONS {
            let index = i as usize;
            let midpoint_dx = (tick_centers[index].dx() + tick_centers[index + 1].dx()) / 2.0;
            let dy = tick_centers[index].dy();

            tester.tap(Offset::new(midpoint_dx - MIDPOINT_OFFSET, dy), 1);
            tester.pump(Duration::ZERO);
            assert_eq!(
                value.get(),
                value_of_tick(i),
                "Tapping just left of the midpoint between pixel ticks {i} and {}",
                i + 1
            );

            tester.tap(Offset::new(midpoint_dx + MIDPOINT_OFFSET, dy), 1);
            tester.pump(Duration::ZERO);
            assert_eq!(
                value.get(),
                value_of_tick(i + 1),
                "Tapping just right of the midpoint between pixel ticks {i} and {}",
                i + 1
            );
        }
    }

    // 'Discrete Slider snaps at the visual midpoint between ticks (M3)'.
    #[test]
    fn a_discrete_slider_snaps_at_the_visual_midpoint_m3() {
        with_main_thread_stack(|| snaps_at_the_visual_midpoint(TextDirection::Ltr, None));
    }

    // 'Discrete Slider snaps at the visual midpoint between ticks (M3 RTL)'.
    #[test]
    fn a_discrete_slider_snaps_at_the_visual_midpoint_m3_rtl() {
        with_main_thread_stack(|| snaps_at_the_visual_midpoint(TextDirection::Rtl, None));
    }

    // 'Discrete Slider snaps at the visual midpoint between ticks (M3 with year2023 false
    // (gapped track))'.
    #[test]
    fn a_discrete_slider_snaps_at_the_visual_midpoint_on_the_gapped_track() {
        with_main_thread_stack(|| snaps_at_the_visual_midpoint(TextDirection::Ltr, Some(false)));
    }

    // 'Discrete Slider snaps at the visual midpoint between ticks (M3 with year2023 false
    // (gapped track) RTL)'.
    #[test]
    fn a_discrete_slider_snaps_at_the_visual_midpoint_on_the_gapped_track_rtl() {
        with_main_thread_stack(|| snaps_at_the_visual_midpoint(TextDirection::Rtl, Some(false)));
    }

    // 'Discrete Slider snaps to a tick when tapping its center'.
    #[test]
    fn a_discrete_slider_snaps_to_a_tick_tapped_at_its_center() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            const DIVISIONS: i32 = 10;
            let value = Rc::new(Cell::new(0.0));
            let tick_centers =
                pump_discrete_slider(&mut tester, &value, DIVISIONS, TextDirection::Ltr, None);

            for i in (0..=DIVISIONS).rev() {
                tester.tap(tick_centers[i as usize], 1);
                tester.pump(Duration::ZERO);
                assert_eq!(value.get(), f64::from(i), "Tapping the center of tick {i}");
            }
        });
    }

    // 'Discrete Slider drag after a tap uses the same track coordinates as the tap'.
    #[test]
    fn a_discrete_drag_after_a_tap_uses_the_tap_track_coordinates() {
        with_main_thread_stack(|| {
            // With many divisions, even a small mismatch between the width used to map
            // a position to a value and the width used to scale drag deltas adds up to
            // more than half a division over a full-track drag. The gapped track has the
            // largest padding, which makes the mismatch most visible.
            let mut tester = tester_on(TargetPlatform::Android);
            const DIVISIONS: i32 = 100;
            let value = Rc::new(Cell::new(0.0));
            let tick_centers = pump_discrete_slider(
                &mut tester,
                &value,
                DIVISIONS,
                TextDirection::Ltr,
                Some(false),
            );

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, tick_centers[0]);
            // Move past the touch slop first so the drag starts mid-gesture and the
            // rest of the movement is scaled by the drag delta normalizer.
            let at = gesture.position + Offset::new(waft_gestures::K_TOUCH_SLOP * 2.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.move_to(&mut gesture, *tick_centers.last().unwrap(), true);
            tester.pump(Duration::ZERO);
            assert_eq!(value.get(), f64::from(DIVISIONS));

            tester.move_to(&mut gesture, tick_centers[0], true);
            tester.pump(Duration::ZERO);
            assert_eq!(value.get(), 0.0);
            tester.up(&gesture);
        });
    }

    /// One call of Dart's `paints` matcher: `..rrect(rrect:, color:)` or `..circle(color:)`,
    /// each argument checked only when given.
    pub(crate) enum Paints {
        RRect(Option<waft_embedder::RRect>, Option<Color>),
        Circle(Option<Color>),
        /// `..path(color:)`: any path.
        Path(Option<Color>),
        /// `..shadow(color:)`.
        Shadow(Option<Color>),
        /// `..paragraph()`: a run of glyphs.
        Paragraph,
    }

    /// Whether `op` is the call `expected` names: a rounded rect is any path or rect that is
    /// not a circle, compared with what `draw_rrect` records for the expected one.
    fn matches_call(op: &Op, expected: &Paints) -> bool {
        let same_color = |paint: &waft_embedder::Paint, color: &Option<Color>| {
            color.is_none_or(|color| colors_match(paint.color, color))
        };
        match (op, expected) {
            (Op::DrawPath { path, paint, .. }, Paints::Circle(color)) => {
                circle_of(path).is_some() && same_color(paint, color)
            }
            (Op::DrawPath { path, paint, .. }, Paints::RRect(rrect, color)) => {
                circle_of(path).is_none()
                    && same_color(paint, color)
                    && rrect.is_none_or(|rrect| {
                        let mut canvas = waft_embedder::Canvas::new();
                        waft_painting::draw_rrect(
                            &mut canvas,
                            rrect,
                            &waft_embedder::Paint::default(),
                        );
                        let list = canvas.build();
                        matches!(list.ops().first(), Some(Op::DrawPath { path: expected, .. }) if expected == path)
                    })
            }
            (Op::DrawRect { paint, .. }, Paints::RRect(None, color)) => same_color(paint, color),
            (Op::DrawPath { paint, .. }, Paints::Path(color)) => same_color(paint, color),
            (Op::DrawShadow { color: shadow, .. }, Paints::Shadow(color)) => {
                color.is_none_or(|color| colors_match(*shadow, color))
            }
            (Op::GlyphRun { .. }, Paints::Paragraph) => true,
            _ => false,
        }
    }

    /// flutter_test's `_colorsMatch`: each channel within 1/255, as Dart's `paints` matcher
    /// compares a call's colour, the painted one read back from its 32-bit floats as Dart's
    /// `Paint.color` is.
    pub(crate) fn colors_match(painted: waft_embedder::valo::Color, expected: Color) -> bool {
        const LIMIT: f64 = 1.0 / 255.0;
        (f64::from(painted.a) - expected.a).abs() < LIMIT
            && (f64::from(painted.r) - expected.r).abs() < LIMIT
            && (f64::from(painted.g) - expected.g).abs() < LIMIT
            && (f64::from(painted.b) - expected.b).abs() < LIMIT
    }

    /// Whether `op` is a call of the canvas method `expected` names, whatever its arguments:
    /// Valo records a rounded rect, a plain path and a circle all as paths, told apart by the
    /// circle's contour.
    fn is_call_of(op: &Op, expected: &Paints) -> bool {
        match (op, expected) {
            (Op::DrawPath { path, .. }, Paints::Circle(_)) => circle_of(path).is_some(),
            (Op::DrawPath { path, .. }, Paints::RRect(..) | Paints::Path(_)) => {
                circle_of(path).is_none()
            }
            (Op::DrawRect { .. }, Paints::RRect(None, _)) => true,
            (Op::DrawShadow { .. }, Paints::Shadow(_)) => true,
            (Op::GlyphRun { .. }, Paints::Paragraph) => true,
            _ => false,
        }
    }

    /// Dart's `expect(render, paints..a..b..)`: each call is the next call of its method after
    /// the last one matched, and has the arguments given; other methods' calls in between are
    /// skipped.
    pub(crate) fn paints(tester: &WidgetTester, render: AnyRenderObject, calls: &[Paints]) -> bool {
        let ops = tester.painted_ops(render);
        let mut ops = ops.iter();
        calls.iter().all(|expected| {
            ops.by_ref()
                .find(|op| is_call_of(op, expected))
                .is_some_and(|op| matches_call(op, expected))
        })
    }

    // 'Default Slider when year2023 is false'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn the_default_slider_when_year2023_is_false() {
        with_main_thread_stack(|| {
            let shadows = waft_painting::debug_disable_shadows();
            waft_painting::set_debug_disable_shadows(false);
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let color_scheme = theme.color_scheme.clone();
            let active_track_color = color_scheme.primary;
            let inactive_track_color = color_scheme.secondary_container();
            let secondary_active_track_color = color_scheme.primary.with_opacity(0.54);
            let disabled_active_track_color = color_scheme.on_surface.with_opacity(0.38);
            let disabled_inactive_track_color = color_scheme.on_surface.with_opacity(0.12);
            let disabled_secondary_active_track_color = color_scheme.on_surface.with_opacity(0.38);
            let active_tick_mark_color = color_scheme.on_primary;
            let inactive_tick_mark_color = color_scheme.on_secondary_container();
            let disabled_active_tick_mark_color = color_scheme.on_inverse_surface();
            let disabled_inactive_tick_mark_color = color_scheme.on_surface;
            let thumb_color = color_scheme.primary;
            let disabled_thumb_color = color_scheme.on_surface.with_opacity(0.38);
            let value_indicator_color = color_scheme.inverse_surface();
            let value = 0.45;
            let app_theme = default_theme(&tester);
            let build_app =
                |divisions: Option<i32>, enabled: bool, text_direction: TextDirection| {
                    let mut slider = Slider::new(value, enabled.then(noop).flatten())
                        .year2023(false)
                        .secondary_track_value(0.75)
                        .label(format!("{value}"));
                    if let Some(divisions) = divisions {
                        slider = slider.divisions(divisions);
                    }
                    material_body(
                        app_theme.clone(),
                        text_direction,
                        Center::new().child(crate::theme::Theme::new(theme.clone(), slider)),
                    )
                };

            let app = build_app(None, true, TextDirection::Ltr);
            tester.pump_widget(app);

            // Test default track shape.
            let outer = waft_embedder::Radius::circular(8.0);
            let inner = waft_embedder::Radius::circular(2.0);
            let rrect =
                |left, top, right, bottom, top_left, top_right, bottom_right, bottom_left| {
                    waft_embedder::RRect::from_ltrb_and_corners(
                        left,
                        top,
                        right,
                        bottom,
                        top_left,
                        top_right,
                        bottom_right,
                        bottom_left,
                    )
                };
            assert!(paints(
                &tester,
                slider_render(&tester),
                &[
                    // Active track.
                    Paints::RRect(
                        Some(rrect(24.0, 292.0, 356.4, 308.0, outer, inner, inner, outer)),
                        Some(active_track_color),
                    ),
                    // Inactive track.
                    Paints::RRect(
                        Some(rrect(
                            368.4, 292.0, 776.0, 308.0, inner, outer, outer, inner
                        )),
                        Some(inactive_track_color),
                    ),
                    // Secondary track.
                    Paints::RRect(
                        Some(rrect(
                            368.4, 292.0, 588.0, 308.0, inner, outer, outer, inner
                        )),
                        Some(secondary_active_track_color),
                    ),
                ],
            ));

            // Test default track shape in RTL. It is the mirror image of the one in LTR,
            // and the secondary track stops a track gap before the thumb.
            // Regression test for https://github.com/flutter/flutter/issues/194279
            let app = build_app(None, true, TextDirection::Rtl);
            tester.pump_widget(app);
            assert!(paints(
                &tester,
                slider_render(&tester),
                &[
                    // Inactive track.
                    Paints::RRect(
                        Some(rrect(24.0, 292.0, 431.6, 308.0, outer, inner, inner, outer)),
                        Some(inactive_track_color),
                    ),
                    // Active track.
                    Paints::RRect(
                        Some(rrect(
                            443.6, 292.0, 776.0, 308.0, inner, outer, outer, inner
                        )),
                        Some(active_track_color),
                    ),
                    // Secondary track.
                    Paints::RRect(
                        Some(rrect(
                            212.0, 292.0, 431.6, 308.0, outer, inner, inner, outer
                        )),
                        Some(secondary_active_track_color),
                    ),
                ],
            ));

            let app = build_app(None, true, TextDirection::Ltr);
            tester.pump_widget(app);

            // Test default colors for enabled slider.
            let render = slider_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[Paints::Circle(None), Paints::RRect(None, Some(thumb_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[
                    Paints::Circle(None),
                    Paints::Circle(Some(disabled_thumb_color))
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(disabled_active_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(disabled_inactive_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(
                    None,
                    Some(disabled_secondary_active_track_color)
                )]
            ));

            // Test defaults colors for discrete slider.
            let app = build_app(Some(3), true, TextDirection::Ltr);
            tester.pump_widget(app);
            let render = slider_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    Paints::RRect(None, Some(active_track_color)),
                    Paints::RRect(None, Some(inactive_track_color)),
                    Paints::RRect(None, Some(secondary_active_track_color)),
                    Paints::Circle(Some(active_tick_mark_color)),
                    Paints::Circle(Some(active_tick_mark_color)),
                    Paints::Circle(Some(inactive_tick_mark_color)),
                    Paints::Circle(Some(inactive_tick_mark_color)),
                ],
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::Circle(Some(disabled_thumb_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(disabled_active_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(disabled_inactive_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(
                    None,
                    Some(disabled_secondary_active_track_color)
                )]
            ));

            // Test defaults colors for disabled slider.
            let app = build_app(None, false, TextDirection::Ltr);
            tester.pump_widget(app);
            tester.pump_and_settle();
            let render = slider_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    Paints::RRect(None, Some(disabled_active_track_color)),
                    Paints::RRect(None, Some(disabled_inactive_track_color)),
                    Paints::RRect(None, Some(disabled_secondary_active_track_color)),
                ],
            ));
            assert!(paints(
                &tester,
                render,
                &[
                    Paints::Circle(None),
                    Paints::RRect(None, Some(disabled_thumb_color))
                ]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::Circle(None), Paints::RRect(None, Some(thumb_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(active_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(inactive_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(secondary_active_track_color))]
            ));

            // Test defaults colors for disabled discrete slider.
            let app = build_app(Some(3), false, TextDirection::Ltr);
            tester.pump_widget(app);
            let render = slider_render(&tester);
            assert!(paints(
                &tester,
                render,
                &[
                    Paints::RRect(None, Some(disabled_active_track_color)),
                    Paints::RRect(None, Some(disabled_inactive_track_color)),
                    Paints::RRect(None, Some(disabled_secondary_active_track_color)),
                    Paints::Circle(Some(disabled_active_tick_mark_color)),
                    Paints::Circle(Some(disabled_active_tick_mark_color)),
                    Paints::Circle(Some(disabled_inactive_tick_mark_color)),
                    Paints::Circle(Some(disabled_inactive_tick_mark_color)),
                    Paints::RRect(None, Some(disabled_thumb_color)),
                ],
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::Circle(None), Paints::RRect(None, Some(thumb_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(active_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(inactive_track_color))]
            ));
            assert!(!paints(
                &tester,
                render,
                &[Paints::RRect(None, Some(secondary_active_track_color))]
            ));

            let app = build_app(Some(3), true, TextDirection::Ltr);
            tester.pump_widget(app);
            tester.pump_and_settle();

            let center = slider_rect(&tester).center();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();

            let value_indicator_box = tester
                .first::<RenderValueIndicator>()
                .as_render_object(&tester.cell.borrow());
            let ops = tester.painted_ops(value_indicator_box);
            let scale = ops
                .iter()
                .position(|op| matches!(op, Op::Transform(_)))
                .expect("the value indicator scales");
            assert!(
                ops[scale..]
                    .iter()
                    .any(|op| matches_call(op, &Paints::RRect(None, Some(value_indicator_color))))
            );
            tester.up(&gesture);
            waft_painting::set_debug_disable_shadows(shadows);
        });
    }

    // 'Slider value indicator text when year2023 is false'.
    #[test]
    fn the_value_indicator_text_when_year2023_is_false() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = 50.0_f64;
            let log = Rc::new(RefCell::new(Vec::new()));
            let shape = Rc::new(LoggingValueIndicatorLabel {
                log: Rc::clone(&log),
            });
            let theme = ThemeData::builder()
                .slider_theme(SliderThemeData::new().value_indicator_shape(shape))
                .build(&tester.cell.borrow());
            #[expect(deprecated, reason = "Dart's case passes `year2023`")]
            let slider = Slider::new(value, noop())
                .year2023(false)
                .max(100.0)
                .divisions(4)
                .label(format!("{}", value.round()));
            tester.pump_widget(material_app(
                theme.clone(),
                Material::new()
                    .child(Center::new().child(slider))
                    .into_widget(),
            ));

            // Normal text
            let center = slider_rect(&tester).center();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            tester.pump_and_settle();

            let (text, style) = log.borrow().last().cloned().expect("the shape painted");
            assert_eq!(text, "50");
            assert_eq!(style.font_size, Some(14.0));
            assert_eq!(
                style.color.as_ref().map(|color| color.color()),
                Some(theme.color_scheme.on_inverse_surface())
            );

            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    /// Dart's `LoggingValueIndicatorShape` logging the label span: its plain text and style.
    #[derive(Debug)]
    struct LoggingValueIndicatorLabel {
        log: Rc<RefCell<Vec<(String, TextStyle)>>>,
    }

    impl SliderComponentShape for LoggingValueIndicatorLabel {
        fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
            Size::new(10.0, 10.0)
        }

        fn paint(
            &self,
            _app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
            _activation_animation: AnyAnimation<f64>,
            _enable_animation: AnyAnimation<f64>,
            _is_discrete: bool,
            label_painter: &mut TextPainter,
            _parent_box: waft_rendering::AnyRenderBox,
            _slider_theme: &SliderThemeData,
            _text_direction: TextDirection,
            _value: f64,
            _text_scale_factor: f64,
            _size_with_overflow: Size,
        ) {
            let span = label_painter.text().cloned().expect("labelPainter.text!");
            let style = span.style().cloned().expect("the label's style");
            self.log
                .borrow_mut()
                .push((label_painter.plain_text().to_owned(), style));
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // "Slider doesn't start any animations after dispose".
    #[test]
    fn a_slider_starts_no_animation_after_dispose() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                plain,
                |slider| slider.divisions(4),
            ));

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).center());
            tester.pump_and_settle();
            assert_eq!(heard.value.get(), 0.5);
            let at = gesture.position + Offset::new(-500.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump_and_settle();
            // Change the tree to dispose the original widget.
            tester.pump_widget(SizedBox::new().into_widget());
            assert_eq!(tester.pump_and_settle(), 1);
            tester.up(&gesture);
        });
    }

    // 'Overlay appear only when hovered on the thumb on desktop' (macOS, Linux, Windows).
    #[test]
    fn the_overlay_appears_only_when_the_thumb_is_hovered_on_desktop() {
        with_main_thread_stack(|| {
            for platform in [
                TargetPlatform::MacOS,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                let mut tester = tester_on(platform);
                let value = Rc::new(Cell::new(0.5));
                const OVERLAY_COLOR: Color = Color::new(0xffff0000);
                let theme = default_theme(&tester);
                let build_app = || {
                    focusable_slider_app(theme.clone(), &value, true, |slider| {
                        slider.overlay_color(Rc::new(WidgetStatePropertyAll::new(Some(
                            OVERLAY_COLOR,
                        ))))
                    })
                };
                tester.pump_widget(build_app());

                // Slider does not have overlay when enabled and not hovered.
                tester.pump_and_settle();
                assert!(!paints_circle(
                    &tester,
                    slider_render(&tester),
                    OVERLAY_COLOR
                ));

                // Hover on the slider but outside the thumb.
                let mut gesture = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, slider_rect(&tester).top_left(), false);

                tester.pump_widget(build_app());
                tester.pump_and_settle();
                assert!(!paints_circle(
                    &tester,
                    slider_render(&tester),
                    OVERLAY_COLOR
                ));

                // Hover on the thumb.
                tester.move_to(&mut gesture, slider_rect(&tester).center(), false);
                tester.pump_and_settle();
                assert!(
                    paints_circle(&tester, slider_render(&tester), OVERLAY_COLOR),
                    "on {platform:?}"
                );

                // Hover on the slider but outside the thumb.
                tester.move_to(&mut gesture, slider_rect(&tester).bottom_right(), false);
                tester.pump_and_settle();
                assert!(!paints_circle(
                    &tester,
                    slider_render(&tester),
                    OVERLAY_COLOR
                ));
            }
        });
    }

    // 'showValueIndicator takes priority over theme'.
    #[test]
    fn show_value_indicator_takes_priority_over_the_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let red = Colors::RED.primary();
            let build_app =
                |theme_show_value_indicator: Option<ShowValueIndicator>,
                 slider_show_value_indicator: Option<ShowValueIndicator>| {
                    let mut slider_theme = SliderThemeData::new().value_indicator_color(red);
                    if let Some(show_value_indicator) = theme_show_value_indicator {
                        slider_theme = slider_theme.show_value_indicator(show_value_indicator);
                    }
                    let mut slider = Slider::new(0.5, noop()).label("0.5");
                    if let Some(show_value_indicator) = slider_show_value_indicator {
                        slider = slider.show_value_indicator(show_value_indicator);
                    }
                    material_app(
                        theme.clone(),
                        Material::new()
                            .child(Center::new().child(SliderTheme::new(slider_theme, slider)))
                            .into_widget(),
                    )
                };
            let check_value_indicator = |tester: &WidgetTester, is_visible: bool| {
                // _RenderValueIndicator is the last render object in the tree.
                let visible = tester
                    .all::<RenderValueIndicator>()
                    .into_iter()
                    .last()
                    .is_some_and(|value_indicator_box| {
                        let render = value_indicator_box.as_render_object(&tester.cell.borrow());
                        paints(
                            tester,
                            render,
                            &[Paints::Path(Some(red)), Paints::Paragraph],
                        )
                    });
                assert_eq!(visible, is_visible);
            };

            tester.pump_widget(build_app(Some(ShowValueIndicator::Never), None));
            check_value_indicator(&tester, false);

            tester.pump_widget(build_app(
                Some(ShowValueIndicator::Never),
                Some(ShowValueIndicator::AlwaysVisible),
            ));
            check_value_indicator(&tester, true);

            tester.pump_widget(build_app(
                Some(ShowValueIndicator::AlwaysVisible),
                Some(ShowValueIndicator::Never),
            ));
            check_value_indicator(&tester, false);
        });
    }

    // 'Event on Slider should perform no-op if already unmounted'.
    #[test]
    fn an_event_on_an_unmounted_slider_does_nothing() {
        with_main_thread_stack(|| {
            // Test covering crashing found in Google internal issue b/192329942.
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(0.0));
            let should_show_slider = Rc::new(Cell::new(true));
            let show_setter: Rc<RefCell<Option<waft_widgets::StateSetter>>> =
                Rc::new(RefCell::new(None));
            let theme = default_theme(&tester);
            let (builder_value, builder_show, builder_setter) = (
                Rc::clone(&value),
                Rc::clone(&should_show_slider),
                Rc::clone(&show_setter),
            );
            tester.pump_widget(material_body(
                theme,
                TextDirection::Ltr,
                Center::new().child(StatefulBuilder::new(move |_app, _context, set_state| {
                    // Dart's `ValueListenableBuilder` over `shouldShowSliderListenable`.
                    *builder_setter.borrow_mut() = Some(Rc::clone(&set_state));
                    let on_changed_value = Rc::clone(&builder_value);
                    let child = if builder_show.get() {
                        Slider::new(
                            builder_value.get(),
                            Some(Rc::new(move |app: &mut App, new_value: f64| {
                                let value = Rc::clone(&on_changed_value);
                                set_state(app, &mut |_app| value.set(new_value));
                            })),
                        )
                        .into_widget()
                    } else {
                        SizedBox::expand().into_widget()
                    };
                    waft_widgets::GestureDetector::new()
                        .behavior(waft_rendering::HitTestBehavior::Translucent)
                        // Note: it is important that `onTap` is non-null so
                        // [GestureDetector] will register tap events.
                        .on_tap(Listener::new(|_app| {}))
                        .child(child)
                        .into_widget()
                })),
            ));

            // Move Slider.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, slider_rect(&tester).center());
            let at = gesture.position + Offset::new(1.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump_and_settle();

            // Hide Slider. Slider will dispose and unmount.
            let set_state = show_setter.borrow().clone().expect("the builder ran");
            {
                let should_show_slider = Rc::clone(&should_show_slider);
                set_state(&mut tester.cell.borrow_mut(), &mut |_app| {
                    should_show_slider.set(false)
                });
            }
            tester.pump_and_settle();

            // Move Slider after unmounted.
            let at = gesture.position + Offset::new(1.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump_and_settle();
        });
    }

    // 'Slider.padding can override the default Slider padding'.
    #[test]
    fn slider_padding_overrides_the_default_padding() {
        with_main_thread_stack(|| {
            use waft_embedder::{RRect, Radius};
            use waft_painting::{EdgeInsets, EdgeInsetsDirectional};
            let mut tester = tester_on(TargetPlatform::Android);
            let theme = default_theme(&tester);
            let build_slider = |padding: EdgeInsetsGeometry| {
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            Center::new().child(
                                waft_widgets::IntrinsicHeight::new()
                                    .child(Slider::new(0.5, noop()).padding(padding)),
                            ),
                        )
                        .into_widget(),
                )
            };
            let slider_render_box_size =
                |tester: &WidgetTester| tester.first::<RenderSlider>().size(&tester.cell.borrow());
            let tracks = |inactive: (f64, f64, f64, f64), active: (f64, f64, f64, f64)| {
                [
                    // Inactive track.
                    Paints::RRect(
                        Some(RRect::from_ltrbr(
                            inactive.0,
                            inactive.1,
                            inactive.2,
                            inactive.3,
                            Radius::circular(2.0),
                        )),
                        None,
                    ),
                    // Active track.
                    Paints::RRect(
                        Some(RRect::from_ltrbr(
                            active.0,
                            active.1,
                            active.2,
                            active.3,
                            Radius::circular(3.0),
                        )),
                        None,
                    ),
                ]
            };

            // Test Slider height and tracks spacing with zero padding.
            tester.pump_widget(build_slider(EdgeInsets::ZERO.into()));
            tester.pump_and_settle();

            // The height equals to the default thumb height.
            assert_eq!(slider_render_box_size(&tester), Size::new(800.0, 20.0));
            assert!(paints(
                &tester,
                slider_render(&tester),
                &tracks((398.0, 8.0, 800.0, 12.0), (0.0, 7.0, 402.0, 13.0))
            ));

            // Test Slider height and tracks spacing with directional padding.
            const START_PADDING: f64 = 100.0;
            const END_PADDING: f64 = 20.0;
            tester.pump_widget(build_slider(
                EdgeInsetsDirectional::only(START_PADDING, 0.0, END_PADDING, 0.0).into(),
            ));
            tester.pump_and_settle();

            assert_eq!(
                slider_render_box_size(&tester),
                Size::new(800.0 - START_PADDING - END_PADDING, 20.0)
            );
            assert!(paints(
                &tester,
                slider_render(&tester),
                &tracks((338.0, 8.0, 680.0, 12.0), (0.0, 7.0, 342.0, 13.0))
            ));

            // Test Slider height and tracks spacing with top and bottom padding.
            const TOP_PADDING: f64 = 100.0;
            const BOTTOM_PADDING: f64 = 20.0;
            const TRACK_HEIGHT: f64 = 20.0;
            tester.pump_widget(build_slider(
                EdgeInsetsDirectional::only(0.0, TOP_PADDING, 0.0, BOTTOM_PADDING).into(),
            ));
            tester.pump_and_settle();

            assert_eq!(
                slider_size(&tester),
                Size::new(800.0, TOP_PADDING + TRACK_HEIGHT + BOTTOM_PADDING)
            );
            assert_eq!(slider_render_box_size(&tester), Size::new(800.0, 20.0));
            assert!(paints(
                &tester,
                slider_render(&tester),
                &tracks((398.0, 8.0, 800.0, 12.0), (0.0, 7.0, 402.0, 13.0))
            ));
        });
    }

    /// Dart's 'Discrete Slider repaints and animates when dragged' and 'Discrete Slider
    /// repaints when dragged', whose bodies are the same: the thumb's repaint centres while a
    /// discrete slider is pressed, then dragged to its start and animated there.
    fn discrete_slider_repaints_when_dragged() {
        let mut tester = tester_on(TargetPlatform::Android);
        let heard = Heard::starting_at(0.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let thumb_log = Rc::clone(&log);
        let theme = default_theme(&tester);
        tester.pump_widget(stateful_slider_app(
            &tester,
            theme,
            TextDirection::Ltr,
            &heard,
            move |app, context, slider| {
                let slider_theme = SliderTheme::of(app, context)
                    .copy_with()
                    .thumb_shape(Rc::new(LoggingThumbShape {
                        log: Rc::clone(&thumb_log),
                    }));
                SliderTheme::new(slider_theme, slider).into_widget()
            },
            |slider| slider.divisions(4),
        ));

        let expected_log = vec![
            Offset::new(26.0, 300.0),
            Offset::new(26.0, 300.0),
            Offset::new(400.0, 300.0),
        ];
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, slider_rect(&tester).center());
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(100));
        assert_eq!(heard.value.get(), 0.5);
        assert_eq!(log.borrow().len(), 3);
        assert_eq!(*log.borrow(), expected_log);
        let at = gesture.position + Offset::new(-500.0, 0.0);
        tester.move_to(&mut gesture, at, true);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(10));
        assert_eq!(heard.value.get(), 0.0);
        assert_eq!(log.borrow().len(), 5);
        assert!((log.borrow().last().expect("logged").dx() - 386.6).abs() <= 0.1);
        // With no more gesture or value changes, the thumb position should still
        // be redrawn in the animated position.
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(10));
        assert_eq!(heard.value.get(), 0.0);
        assert_eq!(log.borrow().len(), 7);
        assert!((log.borrow().last().expect("logged").dx() - 344.8).abs() <= 0.1);
        // Final position.
        tester.pump(Duration::from_millis(80));
        assert_eq!(heard.value.get(), 0.0);
        assert_eq!(log.borrow().len(), 8);
        assert!((log.borrow().last().expect("logged").dx() - 26.0).abs() <= 0.1);
        tester.up(&gesture);
    }

    // 'Discrete Slider repaints and animates when dragged'.
    #[test]
    fn a_discrete_slider_repaints_and_animates_when_dragged() {
        with_main_thread_stack(discrete_slider_repaints_when_dragged);
    }

    // 'Discrete Slider repaints when dragged'.
    #[test]
    fn a_discrete_slider_repaints_when_dragged() {
        with_main_thread_stack(discrete_slider_repaints_when_dragged);
    }

    // 'Slider has correct hovered color from overlayColor property'.
    #[test]
    fn a_slider_takes_its_hovered_color_from_overlay_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let cyan = Colors::CYAN.get(&500).expect("a swatch shade");
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider.overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        move |states: &WidgetStates| {
                            if states.contains(&WidgetState::Hovered) {
                                return Some(cyan);
                            }

                            Some(Colors::TRANSPARENT)
                        },
                    ))
                })
            };
            tester.pump_widget(build_app(true));

            // Slider does not have overlay when enabled and not hovered.
            tester.pump_and_settle();
            assert!(!paints_circle(&tester, slider_render(&tester), cyan));

            // Start hovering.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, slider_rect(&tester).center(), false);

            // Slider has overlay when enabled and hovered.
            tester.pump_widget(build_app(true));
            tester.pump_and_settle();
            assert!(paints_circle(&tester, slider_render(&tester), cyan));

            // Slider does not have an overlay when disabled and hovered.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!paints_circle(&tester, slider_render(&tester), cyan));
        });
    }

    // 'Slider has correct dragged color from overlayColor property'.
    #[test]
    fn a_slider_takes_its_dragged_color_from_overlay_color() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = default_theme(&tester);
            let value = Rc::new(Cell::new(0.5));
            let focus_node = new_focus_node(&tester);
            let lime = Colors::LIME.get(&500).expect("a swatch shade");
            let build_app = |enabled: bool| {
                focusable_slider_app(theme.clone(), &value, enabled, move |slider| {
                    slider
                        .focus_node(focus_node)
                        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            move |states: &WidgetStates| {
                                if states.contains(&WidgetState::Dragged) {
                                    return Some(lime);
                                }

                                Some(Colors::TRANSPARENT)
                            },
                        ))
                })
            };
            tester.pump_widget(build_app(true));

            // Slider does not have overlay when enabled and not dragged.
            tester.pump_and_settle();
            assert!(!paints_circle(&tester, slider_render(&tester), lime));

            // Start dragging.
            let mut drag = WidgetTester::touch(1);
            tester.down(&mut drag, slider_rect(&tester).center());
            tester.pump(waft_gestures::K_PRESS_TIMEOUT);

            // Less than configured touch slop, more than default touch slop
            let at = drag.position + Offset::new(19.0, 0.0);
            tester.move_to(&mut drag, at, true);
            tester.pump(Duration::ZERO);
            // As in the dragged-colour case above: one frame later the overlay has a radius.
            tester.pump(Duration::from_millis(16));

            // Slider has overlay when enabled and dragged.
            assert!(paints_circle(&tester, slider_render(&tester), lime));

            tester.up(&drag);
            tester.pump_and_settle();

            // Slider without focus doesn't have overlay when enabled and dragged.
            assert!(!focus_node.has_focus(&mut tester.cell.borrow_mut()));
            assert!(!paints_circle(&tester, slider_render(&tester), lime));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    // 'In directional nav, Slider can be navigated out of by using up and down arrows'.
    #[test]
    fn in_directional_navigation_up_and_down_arrows_leave_a_slider() {
        with_main_thread_stack(|| {
            use waft_widgets::{
                Column, DirectionalFocusIntent, MediaQueryData, Shortcuts, StateSetter,
                TraversalDirection,
            };
            let shortcuts: ShortcutMap = [
                (LogicalKeyboardKey::ARROW_LEFT, TraversalDirection::Left),
                (LogicalKeyboardKey::ARROW_RIGHT, TraversalDirection::Right),
                (LogicalKeyboardKey::ARROW_DOWN, TraversalDirection::Down),
                (LogicalKeyboardKey::ARROW_UP, TraversalDirection::Up),
            ]
            .into_iter()
            .map(|(key, direction)| {
                (
                    Rc::new(SingleActivator::new(key)) as ShortcutActivatorRef,
                    Rc::new(DirectionalFocusIntent::new(direction)) as IntentRef,
                )
            })
            .collect();

            let mut tester = tester_on(TargetPlatform::Android);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let top_slider_value = Rc::new(Cell::new(0.5));
            let bottom_slider_value = Rc::new(Cell::new(0.5));
            let theme = default_theme(&tester);
            let (top, bottom) = (
                Rc::clone(&top_slider_value),
                Rc::clone(&bottom_slider_value),
            );
            let on_changed = |value: &Rc<Cell<f64>>, set_state: &StateSetter| {
                let (value, set_state) = (Rc::clone(value), Rc::clone(set_state));
                Some(Rc::new(move |app: &mut App, new_value: f64| {
                    let value = Rc::clone(&value);
                    set_state(app, &mut |_app| value.set(new_value));
                }) as ValueChanged<f64>)
            };
            tester.pump_widget(material_app(
                theme,
                Shortcuts::new(
                    shortcuts,
                    Material::new().child(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            MediaQuery::new(
                                MediaQueryData::default()
                                    .navigation_mode(NavigationMode::Directional),
                                Column::new().children(vec![
                                    Slider::new(top.get(), on_changed(&top, &set_state))
                                        .autofocus(true)
                                        .into_widget(),
                                    Slider::new(bottom.get(), on_changed(&bottom, &set_state))
                                        .into_widget(),
                                ]),
                            )
                            .into_widget()
                        },
                    ))),
                )
                .into_widget(),
            ));
            tester.pump_and_settle();

            let values = || (top_slider_value.get(), bottom_slider_value.get());

            // The top slider is auto-focused and can be adjusted with left and right arrow keys.
            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            // focused top Slider increased after first arrowRight; unfocused bottom Slider
            // unaffected by first arrowRight
            assert_eq!(values(), (0.55, 0.5));

            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            // focused top Slider decreased after first arrowLeft; unfocused bottom Slider
            // unaffected by first arrowLeft
            assert_eq!(values(), (0.5, 0.5));

            // Pressing the down-arrow key moves focus down to the bottom slider
            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_DOWN);
            // arrowDown unfocuses top Slider, does not alter its value; arrowDown focuses
            // bottom Slider, does not alter its value
            assert_eq!(values(), (0.5, 0.5));

            // The bottom slider is now focused and can be adjusted with left and right arrow keys.
            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            // unfocused top Slider unaffected by second arrowRight; focused bottom Slider
            // increased by second arrowRight
            assert_eq!(values(), (0.5, 0.55));

            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            // unfocused top Slider unaffected by second arrowLeft; focused bottom Slider
            // decreased by second arrowLeft
            assert_eq!(values(), (0.5, 0.5));

            // Pressing the up-arrow key moves focus back up to the top slider
            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_UP);
            // arrowUp focuses top Slider, does not alter its value; arrowUp unfocuses bottom
            // Slider, does not alter its value
            assert_eq!(values(), (0.5, 0.5));

            // The top slider is now focused again and can be adjusted with left and right arrow
            // keys.
            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_RIGHT);
            // focused top Slider increased after third arrowRight; unfocused bottom Slider
            // unaffected by third arrowRight
            assert_eq!(values(), (0.55, 0.5));

            send_arrow(&mut tester, LogicalKeyboardKey::ARROW_LEFT);
            // focused top Slider decreased after third arrowLeft; unfocused bottom Slider
            // unaffected by third arrowLeft
            assert_eq!(values(), (0.5, 0.5));
        });
    }

    /// The live handles a mounted slider owns: its state's controllers, value-indicator portal
    /// controller and layer link, and its two render objects' curved animations.
    fn slider_owned_handles(tester: &WidgetTester) -> Vec<waft_foundation::HandleId> {
        let (render, value_indicator) = (
            tester.first::<RenderSlider>(),
            tester.first::<RenderValueIndicator>(),
        );
        let app = tester.cell.borrow();
        let (render, value_indicator) = (render.get(&app), value_indicator.get(&app));
        let state = app.get(render.state);
        let mut handles: Vec<waft_foundation::HandleId> = [
            state.overlay_controller,
            state.value_indicator_controller,
            state.enable_controller,
            state.position_controller,
        ]
        .into_iter()
        .map(|controller| controller.expect("made in init_state").into())
        .collect();
        handles.push(
            state
                .value_indicator_overlay_portal_controller
                .expect("made in init_state")
                .into(),
        );
        handles.push(state.layer_link.expect("made in init_state").into());
        handles.extend(
            [
                render.overlay_animation,
                render.value_indicator_animation,
                render.enable_animation,
                value_indicator.value_indicator_animation,
            ]
            .map(waft_foundation::HandleId::from),
        );
        handles
    }

    /// Not a Dart case: Dart's collector frees what a disposed slider owned; here the slider
    /// destroys its controllers, portal controller, layer link and curved animations itself.
    #[test]
    fn an_unmounted_slider_leaves_none_of_its_objects_alive() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let heard = Heard::starting_at(0.0);
            let theme = default_theme(&tester);
            tester.pump_widget(stateful_slider_app(
                &tester,
                theme,
                TextDirection::Ltr,
                &heard,
                plain,
                |slider| slider.divisions(4).label("0"),
            ));
            let handles = slider_owned_handles(&tester);
            {
                let app = tester.cell.borrow();
                assert!(handles.iter().all(|&handle| app.contains(handle)));
            }

            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_and_settle();
            let app = tester.cell.borrow();
            for handle in handles {
                assert!(
                    !app.contains(handle) || app.is_disposed(handle),
                    "{handle:?} is still alive"
                );
            }
        });
    }

    /// Not a Dart case: the value indicator's timer reads the render slider, which a platform
    /// change replaces while the state lives on; the timer keeps the disposed render object
    /// readable until it fires, as Dart's closure keeps it alive.
    #[test]
    fn the_value_indicator_timer_outlives_a_render_slider_replaced_by_a_platform_change() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let value = Rc::new(Cell::new(0.0));
            let build_frame = |tester: &WidgetTester, platform: TargetPlatform| {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .build(&tester.cell.borrow());
                let value = Rc::clone(&value);
                material_app(
                    theme,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let on_changed_value = Rc::clone(&value);
                        Material::new()
                            .child(
                                Center::new().child(
                                    Slider::adaptive(
                                        value.get(),
                                        Some(Rc::new(move |app: &mut App, new_value: f64| {
                                            let value = Rc::clone(&on_changed_value);
                                            set_state(app, &mut |_app| value.set(new_value));
                                        })),
                                    )
                                    .divisions(4)
                                    .label("label")
                                    .show_value_indicator(ShowValueIndicator::OnDrag),
                                ),
                            )
                            .into_widget()
                    })
                    .into_widget(),
                )
            };
            tester.pump_widget(build_frame(&tester, TargetPlatform::Android));
            tester.pump_and_settle();

            tester.tap(slider_rect(&tester).center(), 1);
            assert_eq!(value.get(), 0.5);
            let render = tester.first::<RenderSlider>();
            let state = render.get(&tester.cell.borrow()).state;
            tester.pump(Duration::from_millis(100));

            tester.pump_widget(build_frame(&tester, TargetPlatform::IOS));
            // The theme's 200 ms change, ending before the timer's 500 ms.
            tester.pump(Duration::from_millis(300));
            assert_eq!(tester.find_by_type::<CupertinoSlider>().len(), 1);
            {
                let app = tester.cell.borrow();
                assert!(app.is_disposed(render.handle()));
                assert!(app.get(state).interaction_timer.is_some());
            }

            // Past the 500 ms the timer waits.
            tester.pump(Duration::from_millis(500));
            assert!(tester.cell.borrow().get(state).interaction_timer.is_none());
            tester.pump_and_settle();
            assert!(!tester.cell.borrow().contains(render.handle()));
        });
    }

    // 'Slider removes value indicator from overlay if Slider gets disposed without value
    // indicator animation completing.'
    #[test]
    fn a_slider_disposed_mid_indicator_removes_its_value_indicator_from_the_overlay() {
        with_main_thread_stack(|| {
            use waft_widgets::{Builder, Column, Navigator, Route, Text, WidgetBuilder};

            use crate::elevated_button::ElevatedButton;
            use crate::page::MaterialPageRoute;
            use crate::scaffold::Scaffold;

            let mut tester = tester_on(TargetPlatform::Android);
            const FILL_COLOR: Color = Color::new(0xf55f5f5f);
            let value = Rc::new(Cell::new(0.0));
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let builder_value = Rc::clone(&value);
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(Builder::new(move |_app, context| {
                        // The builder is used to pass the context from the MaterialApp widget
                        // to the [Navigator]. This context is required in order for the
                        // Navigator to work.
                        let on_changed_value = Rc::clone(&builder_value);
                        Column::new()
                            .children(vec![
                                Slider::new(
                                    builder_value.get(),
                                    Some(Rc::new(move |_app: &mut App, new_value: f64| {
                                        on_changed_value.set(new_value);
                                    })),
                                )
                                .max(100.0)
                                .divisions(3)
                                .label(format!("{}", builder_value.get().round()))
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
                                            Route::as_route(route),
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

            // Dart reads the `Overlay`'s render object, whose route children are repaint
            // boundaries `painted_ops` does not record inline; the value indicator box is the
            // render object in it that draws the slider's indicator.
            let indicator_paints = |tester: &WidgetTester| {
                tester
                    .all::<RenderValueIndicator>()
                    .into_iter()
                    .any(|value_indicator_box| {
                        let render = value_indicator_box.as_render_object(&tester.cell.borrow());
                        paints(
                            tester,
                            render,
                            &[Paints::Path(Some(FILL_COLOR)), Paints::Paragraph],
                        )
                    })
            };
            let top_right = slider_rect(&tester).top_right() + Offset::new(-24.0, 0.0);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, top_right);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();

            assert_eq!(tester.find_by_type::<Slider>().len(), 1);
            // Represents the Slider.
            assert!(indicator_paints(&tester));

            let next = tester.find_onstage_text("Next")[0];
            let next = tester.global_rect(tester.render_box_of(next)).center();
            tester.tap(next, 2);
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Slider>().is_empty());
            assert!(!indicator_paints(&tester));
            assert_eq!(tester.find_onstage_text("Inner page").len(), 1);

            // Don't stop holding the value indicator.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }
}

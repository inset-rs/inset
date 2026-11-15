//! Flutter counterpart: material_ui's `checkbox.dart`, with its part
//! `generated/checkbox_defaults_m3.g.dart` in `crate::generated::checkbox_defaults_m3`.
//!
//! [`CheckboxState`] is a [`ToggleableStateMixin`], and its painter an arena
//! [`ToggleablePainter`] the state keeps for its lifetime, as Dart's `_painter` field is.

use std::fmt::{self, Debug};
use std::time::Duration;

use waft_animation::AnimationStatus;
use waft_cupertino::CupertinoCheckbox;
use waft_embedder::{
    Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, Radius, Rect, Size, Stroke,
    TargetPlatform, clamp_double,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, OptionRetained, Ref, ValueChanged,
};
use waft_painting::{
    AnyBorderSide, BorderRadiusGeometry, BorderSide, EdgeInsets, OutlinedBorder, OutlinedBorderRef,
    RoundedRectangleBorder, ShapeBorder,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, FocusNodeRef, IntoWidget, KeyRef, Semantics, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, ToggleableStateMixin, ToggleableStateMixinData,
    WidgetRef, WidgetState, WidgetStateBorderSide, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates,
};

use crate::checkbox_theme::{CheckboxTheme, CheckboxThemeData, CheckboxThemeDataGetters};
use crate::colors::Colors;
use crate::constants::{
    K_MIN_INTERACTIVE_DIMENSION, K_RADIAL_REACTION_ALPHA, K_RADIAL_REACTION_DURATION,
    K_RADIAL_REACTION_RADIUS,
};
use crate::debug::debug_check_has_material;
use crate::generated::checkbox_defaults_m3::CheckboxDefaultsM3;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_CheckboxType`.
#[derive(Clone, Copy, Debug)]
enum CheckboxType {
    Material,
    Adaptive,
}

/// A Material Design checkbox.
///
/// The checkbox itself does not maintain any state. Instead, when the state of
/// the checkbox changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most
/// widgets that use a checkbox will listen for the [`on_changed`](Self::on_changed) callback
/// and rebuild the checkbox with a new [`value`](Self::value) to update the visual appearance
/// of the checkbox.
///
/// The checkbox can optionally display three values - true, false, and null -
/// if [`tristate`](Self::tristate) is true. When [`value`](Self::value) is null a dash is
/// displayed. By default [`tristate`](Self::tristate) is false and the checkbox's
/// [`value`](Self::value) must be true or false.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// See also:
///
///  * `CheckboxListTile`, which combines this widget with a `ListTile` so that
///    you can give the checkbox a label.
///  * `Switch`, a widget with semantics similar to [`Checkbox`].
///  * `Radio`, for selecting among a set of explicit values.
///  * `Slider`, for selecting a value in a range.
///  * <https://material.io/design/components/selection-controls.html#checkboxes>
///  * <https://material.io/design/components/lists.html#types>
#[derive(Clone)]
pub struct Checkbox {
    pub key: Option<KeyRef>,
    /// Whether this checkbox is checked.
    ///
    /// When [`tristate`](Self::tristate) is true, a value of null corresponds to the mixed
    /// state. When [`tristate`](Self::tristate) is false, this value must not be null.
    pub value: Option<bool>,
    /// Called when the value of the checkbox should change.
    ///
    /// The checkbox passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the checkbox with the new
    /// value.
    ///
    /// If this callback is null, the checkbox will be displayed as disabled
    /// and will not respond to input gestures.
    ///
    /// When the checkbox is tapped, if [`tristate`](Self::tristate) is false (the default)
    /// then the [`on_changed`](Self::on_changed) callback will be applied to `!value`. If
    /// [`tristate`](Self::tristate) is true this callback cycle from false to true to null.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
    pub on_changed: Option<ValueChanged<Option<bool>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// When [`value`](Self::value) is null and [`tristate`](Self::tristate) is true,
    /// [`WidgetState::Selected`] is included as a state.
    ///
    /// If this property is null, the value of [`CheckboxThemeData::mouse_cursor`] is used.
    /// If that is also null, [`WidgetStateMouseCursor::adaptive_clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The color to use when this checkbox is checked.
    ///
    /// Defaults to `ColorScheme::secondary`.
    ///
    /// If [`fill_color`](Self::fill_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, it will be used instead of this color.
    pub active_color: Option<Color>,
    /// The color that fills the checkbox, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) is used in the selected
    /// state. If that is also null, the value of [`CheckboxThemeData::fill_color`]
    /// is used. If that is also null, then `ThemeData::disabled_color` is used in
    /// the disabled state, `ColorScheme::secondary` is used in the
    /// selected state, and `ThemeData::unselected_widget_color` is used in the
    /// default state.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The color to use for the check icon when this checkbox is checked.
    ///
    /// If null, then the value of [`CheckboxThemeData::check_color`] is used. If
    /// that is also null, then Color(0xFFFFFFFF) is used.
    pub check_color: Option<Color>,
    /// If true the checkbox's [`value`](Self::value) can be true, false, or null.
    ///
    /// [`Checkbox`] displays a dash when its value is null.
    ///
    /// When a tri-state checkbox ([`tristate`](Self::tristate) is true) is tapped, its
    /// [`on_changed`](Self::on_changed) callback will be applied to true if the current value
    /// is false, to null if value is true, and to false if value is null (i.e. it cycles
    /// through false => true => null => false when tapped).
    ///
    /// If tristate is false (the default), [`value`](Self::value) must not be null.
    pub tristate: bool,
    /// Configures the minimum size of the tap target.
    ///
    /// If null, then the value of [`CheckboxThemeData::material_tap_target_size`] is
    /// used. If that is also null, then the value of
    /// `ThemeData::material_tap_target_size` is used.
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// Defines how compact the checkbox's layout will be.
    ///
    /// If null, then the value of [`CheckboxThemeData::visual_density`] is used. If
    /// that is also null and if `ThemeData::use_material3` is false, then the
    /// value of `ThemeData::visual_density` is used. Otherwise, the default value
    /// is [`VisualDensity::STANDARD`].
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the [`visual_density`](Self::visual_density)
    ///    for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,
    /// The color for the checkbox's `Material` when it has the input focus.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Focused`] state, it will be used instead.
    ///
    /// If null, then the value of [`CheckboxThemeData::overlay_color`] is used in the
    /// focused state. If that is also null, then the value of
    /// `ThemeData::focus_color` is used.
    pub focus_color: Option<Color>,
    /// The color for the checkbox's `Material` when a pointer is hovering over it.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Hovered`] state, it will be used instead.
    ///
    /// If null, then the value of [`CheckboxThemeData::overlay_color`] is used in the
    /// hovered state. If that is also null, then the value of
    /// `ThemeData::hover_color` is used.
    pub hover_color: Option<Color>,
    /// The color for the checkbox's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) with alpha
    /// [`K_RADIAL_REACTION_ALPHA`], [`focus_color`](Self::focus_color) and
    /// [`hover_color`](Self::hover_color) is used in the pressed, focused and hovered state.
    /// If that is also null, the value of [`CheckboxThemeData::overlay_color`] is used. If
    /// that is also null, then the value of `ColorScheme::secondary` with alpha
    /// [`K_RADIAL_REACTION_ALPHA`], `ThemeData::focus_color` and `ThemeData::hover_color`
    /// is used in the pressed, focused and hovered state.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of [`CheckboxThemeData::splash_radius`] is used. If
    /// that is also null, then [`K_RADIAL_REACTION_RADIUS`] is used.
    pub splash_radius: Option<f64>,
    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// The shape of the checkbox's `Material`.
    ///
    /// If this property is null then the ambient [`CheckboxThemeData::shape`]
    /// is used. If that's null then the shape will be a [`RoundedRectangleBorder`]
    /// with a circular corner radius of 1.0 in Material 2, and 2.0 in Material 3.
    pub shape: Option<OutlinedBorderRef>,
    /// The color and width of the checkbox's border.
    ///
    /// This property can be a [`WidgetStateBorderSide`] that can
    /// specify different border color and widths depending on the
    /// checkbox's state.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///  * [`WidgetState::Error`].
    ///
    /// If this property is not a [`WidgetStateBorderSide`] and it is
    /// non-null, then it is only rendered when the checkbox's value is
    /// false. The difference in interpretation is for backwards
    /// compatibility.
    ///
    /// If this property is null, then the ambient [`CheckboxThemeData::side`] is
    /// used. If that is also null, then the side will be width 2.
    pub side: Option<AnyBorderSide>,
    /// True if this checkbox wants to show an error state.
    ///
    /// The checkbox will have different default container color and check color when
    /// this is true. This is only used when `ThemeData::use_material3` is set to true.
    ///
    /// Defaults to false.
    pub is_error: bool,
    /// The semantic label for the checkbox that will be announced by screen readers.
    ///
    /// This is announced by assistive technologies (e.g TalkBack/VoiceOver).
    ///
    /// This label does not show in the UI.
    pub semantic_label: Option<String>,
    /// The insets applied around the check mark when the checkbox is checked or
    /// in its indeterminate (tristate) state.
    ///
    /// This insets the check mark inward; the size of the checkbox box itself
    /// is not affected. The check mark and its stroke shrink to fit the
    /// remaining space, so the mark stays proportional. Its size is the smaller
    /// of the remaining width and height, and it is centered in the remaining
    /// space, so an inset on one axis also shrinks it on the other. For example,
    /// `EdgeInsets::only(9.0, 0.0, 0.0, 0.0)` draws a half-size mark centered in the right
    /// half of the box.
    ///
    /// On iOS and macOS, when using [`Checkbox::adaptive`], this property has no
    /// effect, because the checkbox delegates to [`CupertinoCheckbox`], which does
    /// not support insets.
    ///
    /// Must be non-negative. If the horizontal or vertical insets are
    /// greater than or equal to the checkbox size, the check mark or indeterminate
    /// dash is not painted.
    ///
    /// If null, [`CheckboxThemeData::mark_insets`] is used. If that is also null,
    /// it defaults to [`EdgeInsets::ZERO`] and the check mark renders at full size.
    pub mark_insets: Option<EdgeInsets>,
    checkbox_type: CheckboxType,
}

impl Checkbox {
    /// The width of a checkbox widget.
    pub const WIDTH: f64 = 18.0;

    /// Creates a Material Design checkbox.
    ///
    /// The checkbox itself does not maintain any state. Instead, when the state of
    /// the checkbox changes, the widget calls the [`on_changed`](Self::on_changed) callback.
    /// Most widgets that use a checkbox will listen for the [`on_changed`](Self::on_changed)
    /// callback and rebuild the checkbox with a new [`value`](Self::value) to update the
    /// visual appearance of the checkbox.
    ///
    /// The following arguments are required:
    ///
    /// * `value`, which determines whether the checkbox is checked. The `value`
    ///   can only be null if [`tristate`](Self::tristate) is true.
    /// * `on_changed`, which is called when the value of the checkbox should
    ///   change. It can be set to null to disable the checkbox.
    pub fn new(value: Option<bool>, on_changed: Option<ValueChanged<Option<bool>>>) -> Checkbox {
        Checkbox::with_type(value, on_changed, CheckboxType::Material)
    }

    /// Creates an adaptive [`Checkbox`] based on whether the target platform is iOS
    /// or macOS, following Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// On iOS and macOS, this constructor creates a [`CupertinoCheckbox`], which has
    /// matching functionality and presentation as Material checkboxes, and are the
    /// graphics expected on iOS. On other platforms, this creates a Material
    /// design [`Checkbox`].
    ///
    /// If a [`CupertinoCheckbox`] is created, the following parameters are ignored:
    /// [`fill_color`](Self::fill_color), [`hover_color`](Self::hover_color),
    /// [`overlay_color`](Self::overlay_color), [`splash_radius`](Self::splash_radius),
    /// [`material_tap_target_size`](Self::material_tap_target_size),
    /// [`visual_density`](Self::visual_density), [`is_error`](Self::is_error),
    /// [`mark_insets`](Self::mark_insets). However, [`shape`](Self::shape) and
    /// [`side`](Self::side) will still affect the [`CupertinoCheckbox`] and should be
    /// handled if native fidelity is important.
    ///
    /// The target platform is based on the current `Theme`: `ThemeData::platform`.
    pub fn adaptive(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
    ) -> Checkbox {
        Checkbox::with_type(value, on_changed, CheckboxType::Adaptive)
    }

    fn with_type(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
        checkbox_type: CheckboxType,
    ) -> Checkbox {
        Checkbox {
            key: None,
            value,
            on_changed,
            mouse_cursor: None,
            active_color: None,
            fill_color: None,
            check_color: None,
            tristate: false,
            material_tap_target_size: None,
            visual_density: None,
            focus_color: None,
            hover_color: None,
            overlay_color: None,
            splash_radius: None,
            focus_node: None,
            autofocus: false,
            shape: None,
            side: None,
            is_error: false,
            semantic_label: None,
            mark_insets: None,
            checkbox_type,
        }
    }

    /// Dart `Checkbox(key:)`.
    pub fn key(mut self, key: KeyRef) -> Checkbox {
        self.key = Some(key);
        self
    }

    /// Dart `Checkbox(tristate:)`.
    pub fn tristate(mut self, tristate: bool) -> Checkbox {
        self.tristate = tristate;
        self
    }

    /// Dart `Checkbox(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> Checkbox {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `Checkbox(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> Checkbox {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `Checkbox(fillColor:)`.
    pub fn fill_color(mut self, fill_color: WidgetStatePropertyRef<Option<Color>>) -> Checkbox {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `Checkbox(checkColor:)`.
    pub fn check_color(mut self, check_color: Color) -> Checkbox {
        self.check_color = Some(check_color);
        self
    }

    /// Dart `Checkbox(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> Checkbox {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `Checkbox(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> Checkbox {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `Checkbox(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> Checkbox {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `Checkbox(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> Checkbox {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `Checkbox(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> Checkbox {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `Checkbox(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> Checkbox {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `Checkbox(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> Checkbox {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Checkbox(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Checkbox {
        self.autofocus = autofocus;
        self
    }

    /// Dart `Checkbox(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> Checkbox {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `Checkbox(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> Checkbox {
        self.side = Some(side.into());
        self
    }

    /// Dart `Checkbox(isError:)`.
    pub fn is_error(mut self, is_error: bool) -> Checkbox {
        self.is_error = is_error;
        self
    }

    /// Dart `Checkbox(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> Checkbox {
        self.semantic_label = Some(semantic_label.into());
        self
    }

    /// Dart `Checkbox(markInsets:)`.
    pub fn mark_insets(mut self, mark_insets: EdgeInsets) -> Checkbox {
        self.mark_insets = Some(mark_insets);
        self
    }
}

impl Debug for Checkbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Checkbox")
            .field("value", &self.value)
            .field("tristate", &self.tristate)
            .field(
                "on_changed",
                &if self.on_changed.is_some() {
                    "enabled"
                } else {
                    "disabled"
                },
            )
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Checkbox {
    type State = CheckboxState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.tristate || self.value.is_some());
    }

    fn create_state(&self) -> CheckboxState {
        CheckboxState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            toggleable: ToggleableStateMixinData::new(),
            painter: None,
            previous_value: None,
        }
    }
}

/// Dart's `_CheckboxState`.
pub struct CheckboxState {
    state: StateData<Checkbox>,
    ticker_provider: TickerProviderStateMixinData,
    toggleable: ToggleableStateMixinData,
    /// Dart's `final _CheckboxPainter _painter = _CheckboxPainter()`, created in `init_state`
    /// because a painter is an arena object and a state is created without the `App`.
    painter: Option<Ref<CheckboxPainter>>,
    previous_value: Option<bool>,
}

impl CheckboxState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<CheckboxPainter> {
        app.get(self)
            .painter
            .handle()
            .expect("created in init_state")
    }

    /// Dart's `_widgetFillColor`.
    fn widget_fill_color(self: Handle<Self>, app: &App) -> WidgetStatePropertyRef<Option<Color>> {
        let active_color = self.widget(app).active_color;
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return None;
            }
            if states.contains(&WidgetState::Selected) {
                return active_color;
            }
            None
        })
    }

    /// Dart's `_resolveSide`: `resolve_as` answers Dart's `is WidgetStateBorderSide` itself.
    fn resolve_side(side: &Option<AnyBorderSide>, states: &WidgetStates) -> Option<BorderSide> {
        if side
            .as_ref()
            .is_some_and(|side| side.extension::<WidgetStateBorderSide>().is_some())
        {
            return <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(side, states)
                .map(|side| *side);
        }
        if !states.contains(&WidgetState::Selected) {
            return side.as_ref().map(|side| **side);
        }
        None
    }

    /// Dart's `effectiveMouseCursor`, built from the widget's and the theme's cursors.
    fn effective_mouse_cursor(
        mouse_cursor: Option<MouseCursorRef>,
        theme_mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
    ) -> WidgetStatePropertyRef<MouseCursorRef> {
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(move |states: &WidgetStates| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                .or_else(|| {
                    theme_mouse_cursor
                        .as_ref()
                        .and_then(|mouse_cursor| mouse_cursor.resolve(states))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::adaptive_clickable().resolve(states))
        })
    }

    /// The [`CupertinoCheckbox`] Dart's `Checkbox.adaptive` builds on iOS and macOS.
    fn build_cupertino_checkbox(self: Handle<Self>, app: &App) -> WidgetRef {
        let widget = self.widget(app);
        let mut checkbox = CupertinoCheckbox::new(
            ToggleableStateMixin::value(self, app),
            ToggleableStateMixin::on_changed(self, app),
        )
        .tristate(ToggleableStateMixin::tristate(self, app))
        .autofocus(widget.autofocus);
        checkbox.mouse_cursor = widget.mouse_cursor.clone();
        checkbox.active_color = widget.active_color;
        checkbox.check_color = widget.check_color;
        checkbox.focus_color = widget.focus_color;
        checkbox.focus_node = widget.focus_node.clone();
        checkbox.side = widget.side.clone();
        checkbox.shape = widget.shape.clone();
        checkbox.semantic_label = widget.semantic_label.clone();
        checkbox.into_widget()
    }
}

impl TickerProviderStateMixin for CheckboxState {
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

/// Dart's `with TickerProviderStateMixin`.
impl TickerProviderObject for CheckboxState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ToggleableStateMixin for CheckboxState {
    fn toggleable_data(self: Handle<Self>, app: &App) -> &ToggleableStateMixinData {
        &app.get(self).toggleable
    }

    fn toggleable_data_mut(self: Handle<Self>, app: &mut App) -> &mut ToggleableStateMixinData {
        &mut app.get_mut(self).toggleable
    }

    fn on_changed(self: Handle<Self>, app: &App) -> Option<ValueChanged<Option<bool>>> {
        self.widget(app).on_changed.clone()
    }

    fn tristate(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).tristate
    }

    fn value(self: Handle<Self>, app: &App) -> Option<bool> {
        self.widget(app).value
    }

    fn reaction_animation_duration(self: Handle<Self>, _app: &App) -> Option<Duration> {
        Some(K_RADIAL_REACTION_DURATION)
    }
}

impl State for CheckboxState {
    type Widget = Checkbox;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let painter = CheckboxPainter::new(app);
        app.get_mut(self).painter = Some(painter);
        ToggleableStateMixin::init_state(self, app);
        app.get_mut(self).previous_value = self.widget(app).value;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Checkbox) {
        if old_widget.value != self.widget(app).value {
            app.get_mut(self).previous_value = old_widget.value;
            self.animate_to_value(app);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let painter = self.painter(app);
        ToggleablePainter::dispose(painter, app);
        ToggleableStateMixin::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();

        match widget.checkbox_type {
            CheckboxType::Material => {}
            CheckboxType::Adaptive => {
                let theme = Theme::of(app, context);
                match theme.platform {
                    TargetPlatform::Android
                    | TargetPlatform::Fuchsia
                    | TargetPlatform::Linux
                    | TargetPlatform::Windows => {}
                    TargetPlatform::IOS | TargetPlatform::MacOS => {
                        return self.build_cupertino_checkbox(app);
                    }
                }
            }
        }

        debug_assert!(debug_check_has_material(app, context));
        let checkbox_theme = CheckboxTheme::of(app, context);
        let defaults: Box<dyn CheckboxThemeDataGetters> = if Theme::of(app, context).use_material3 {
            Box::new(CheckboxDefaultsM3::new(app, context))
        } else {
            Box::new(CheckboxDefaultsM2::new(app, context))
        };
        let effective_material_tap_target_size = widget
            .material_tap_target_size
            .or(checkbox_theme.material_tap_target_size)
            .or_else(|| defaults.material_tap_target_size(app))
            .expect("the defaults give a tap target size");
        let effective_visual_density = widget
            .visual_density
            .or(checkbox_theme.visual_density)
            .or_else(|| defaults.visual_density(app))
            .expect("the defaults give a visual density");
        let mut size = match effective_material_tap_target_size {
            MaterialTapTargetSize::Padded => {
                Size::new(K_MIN_INTERACTIVE_DIMENSION, K_MIN_INTERACTIVE_DIMENSION)
            }
            MaterialTapTargetSize::ShrinkWrap => Size::new(
                K_MIN_INTERACTIVE_DIMENSION - 8.0,
                K_MIN_INTERACTIVE_DIMENSION - 8.0,
            ),
        };
        size = size + effective_visual_density.base_size_adjustment();

        let effective_mouse_cursor = Self::effective_mouse_cursor(
            widget.mouse_cursor.clone(),
            checkbox_theme.mouse_cursor.clone(),
        );

        // Colors need to be resolved in selected and non selected states separately
        let mut active_states = self.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = self.states(app);
        inactive_states.remove(&WidgetState::Selected);
        if widget.is_error {
            active_states.insert(WidgetState::Error);
            inactive_states.insert(WidgetState::Error);
        }
        let widget_fill_color = self.widget_fill_color(app);
        let resolve_fill_color = |states: &WidgetStates| {
            resolve(&widget.fill_color, states)
                .or_else(|| widget_fill_color.resolve(states))
                .or_else(|| resolve(&checkbox_theme.fill_color, states))
        };
        let active_color = resolve_fill_color(&active_states);
        let effective_active_color = active_color.unwrap_or_else(|| {
            resolve(&defaults.fill_color(app), &active_states).expect("default")
        });
        let inactive_color = resolve_fill_color(&inactive_states);
        let effective_inactive_color = inactive_color.unwrap_or_else(|| {
            resolve(&defaults.fill_color(app), &inactive_states).expect("default")
        });

        let effective_side = |app: &mut App, states: &WidgetStates| {
            Self::resolve_side(&widget.side, states)
                .or_else(|| Self::resolve_side(&checkbox_theme.side, states))
                .or_else(|| Self::resolve_side(&defaults.side(app), states))
                .expect("the defaults give a side")
        };
        let active_side = effective_side(app, &active_states);
        let inactive_side = effective_side(app, &inactive_states);

        let mut focused_states = self.states(app);
        focused_states.insert(WidgetState::Focused);
        if widget.is_error {
            focused_states.insert(WidgetState::Error);
        }
        let mut effective_focus_overlay_color = resolve(&widget.overlay_color, &focused_states)
            .or(widget.focus_color)
            .or_else(|| resolve(&checkbox_theme.overlay_color, &focused_states))
            .unwrap_or_else(|| {
                resolve(&defaults.overlay_color(app), &focused_states).expect("default")
            });

        let mut hovered_states = self.states(app);
        hovered_states.insert(WidgetState::Hovered);
        if widget.is_error {
            hovered_states.insert(WidgetState::Error);
        }
        let mut effective_hover_overlay_color = resolve(&widget.overlay_color, &hovered_states)
            .or(widget.hover_color)
            .or_else(|| resolve(&checkbox_theme.overlay_color, &hovered_states))
            .unwrap_or_else(|| {
                resolve(&defaults.overlay_color(app), &hovered_states).expect("default")
            });

        let resolve_pressed_overlay_color =
            |app: &mut App, states: &WidgetStates, fill_color: Option<Color>| {
                resolve(&widget.overlay_color, states)
                    .or_else(|| resolve(&checkbox_theme.overlay_color, states))
                    .or_else(|| fill_color.map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA)))
                    .unwrap_or_else(|| {
                        resolve(&defaults.overlay_color(app), states).expect("default")
                    })
            };
        let mut active_pressed_states = active_states;
        active_pressed_states.insert(WidgetState::Pressed);
        let effective_active_pressed_overlay_color =
            resolve_pressed_overlay_color(app, &active_pressed_states, active_color);

        let mut inactive_pressed_states = inactive_states;
        inactive_pressed_states.insert(WidgetState::Pressed);
        let effective_inactive_pressed_overlay_color =
            resolve_pressed_overlay_color(app, &inactive_pressed_states, inactive_color);

        if self.down_position(app).is_some() {
            let pressed_overlay_color = if self.states(app).contains(&WidgetState::Selected) {
                effective_active_pressed_overlay_color
            } else {
                effective_inactive_pressed_overlay_color
            };
            effective_hover_overlay_color = pressed_overlay_color;
            effective_focus_overlay_color = pressed_overlay_color;
        }

        let mut check_states = self.states(app);
        if widget.is_error {
            check_states.insert(WidgetState::Error);
        }
        let effective_check_color = widget
            .check_color
            .or_else(|| resolve(&checkbox_theme.check_color, &check_states))
            .unwrap_or_else(|| {
                resolve(&defaults.check_color(app), &check_states).expect("default")
            });

        let effective_splash_radius = widget
            .splash_radius
            .or(checkbox_theme.splash_radius)
            .or_else(|| defaults.splash_radius(app))
            .expect("the defaults give a splash radius");

        let mark_insets = widget
            .mark_insets
            .or(checkbox_theme.mark_insets)
            .unwrap_or(EdgeInsets::ZERO);

        let shape = widget
            .shape
            .clone()
            .or_else(|| checkbox_theme.shape.clone())
            .or_else(|| defaults.shape(app))
            .expect("the defaults give a shape");

        let states = self.states(app);
        let painter_ref = app
            .get(self)
            .painter
            .clone()
            .expect("created in init_state");
        let painter = painter_ref.handle();
        painter.set_position(app, self.position(app).erase());
        painter.set_reaction(app, self.reaction(app).erase());
        painter.set_reaction_focus_fade(app, self.reaction_focus_fade(app).erase());
        painter.set_reaction_hover_fade(app, self.reaction_hover_fade(app).erase());
        painter.set_inactive_reaction_color(app, effective_inactive_pressed_overlay_color);
        painter.set_reaction_color(app, effective_active_pressed_overlay_color);
        painter.set_hover_color(app, effective_hover_overlay_color);
        painter.set_focus_color(app, effective_focus_overlay_color);
        painter.set_splash_radius(app, effective_splash_radius);
        painter.set_down_position(app, self.down_position(app));
        painter.set_is_focused(app, Some(states.contains(&WidgetState::Focused)));
        painter.set_is_hovered(app, Some(states.contains(&WidgetState::Hovered)));
        painter.set_active_color(app, effective_active_color);
        painter.set_inactive_color(app, effective_inactive_color);
        painter.set_check_color(app, effective_check_color);
        painter.set_value(app, ToggleableStateMixin::value(self, app));
        painter.set_previous_value(app, app.get(self).previous_value);
        painter.set_shape(app, shape);
        painter.set_active_side(app, active_side);
        painter.set_mark_insets(app, mark_insets);
        painter.set_inactive_side(app, inactive_side);

        let mut semantics = Semantics::new().checked(widget.value.unwrap_or(false));
        if let Some(semantic_label) = &widget.semantic_label {
            semantics = semantics.label(semantic_label.clone());
        }
        if widget.tristate {
            semantics = semantics.mixed(widget.value.is_none());
        }
        semantics
            .child(self.build_toggleable(
                app,
                widget.focus_node,
                None,
                widget.autofocus,
                Some(effective_mouse_cursor),
                size,
                ToggleablePainterRef::new(painter_ref),
            ))
            .into_widget()
    }
}

/// Dart's `property?.resolve(states)` on a nullable `WidgetStateProperty<Color?>`.
fn resolve(
    property: &Option<WidgetStatePropertyRef<Option<Color>>>,
    states: &WidgetStates,
) -> Option<Color> {
    property
        .as_ref()
        .and_then(|property| property.resolve(states))
}

const K_EDGE_SIZE: f64 = Checkbox::WIDTH;
const K_STROKE_WIDTH: f64 = 2.0;

/// Dart's `_CheckboxPainter`.
struct CheckboxPainter {
    change_notifier: ChangeNotifierData,
    toggleable_painter: ToggleablePainterData,
    check_color: Option<Color>,
    value: Option<bool>,
    previous_value: Option<bool>,
    shape: Option<OutlinedBorderRef>,
    mark_insets: EdgeInsets,
    active_side: Option<BorderSide>,
    inactive_side: Option<BorderSide>,
}

impl CheckboxPainter {
    fn new(app: &mut App) -> Ref<CheckboxPainter> {
        app.create(CheckboxPainter {
            change_notifier: ChangeNotifierData::new(),
            toggleable_painter: ToggleablePainterData::new(),
            check_color: None,
            value: None,
            previous_value: None,
            shape: None,
            mark_insets: EdgeInsets::ZERO,
            active_side: None,
            inactive_side: None,
        })
    }

    fn check_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).check_color.expect("set before painting")
    }

    fn set_check_color(self: Handle<Self>, app: &mut App, value: Color) {
        if app.get(self).check_color == Some(value) {
            return;
        }
        app.get_mut(self).check_color = Some(value);
        self.notify_listeners(app);
    }

    fn value(self: Handle<Self>, app: &App) -> Option<bool> {
        app.get(self).value
    }

    fn set_value(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        if app.get(self).value == value {
            return;
        }
        app.get_mut(self).value = value;
        self.notify_listeners(app);
    }

    fn previous_value(self: Handle<Self>, app: &App) -> Option<bool> {
        app.get(self).previous_value
    }

    fn set_previous_value(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        if app.get(self).previous_value == value {
            return;
        }
        app.get_mut(self).previous_value = value;
        self.notify_listeners(app);
    }

    fn shape(self: Handle<Self>, app: &App) -> &dyn OutlinedBorder {
        app.get(self).shape.as_deref().expect("set before painting")
    }

    fn set_shape(self: Handle<Self>, app: &mut App, value: OutlinedBorderRef) {
        if let Some(shape) = &app.get(self).shape {
            let (old, new): (&dyn ShapeBorder, &dyn ShapeBorder) = (&**shape, &*value);
            if old == new {
                return;
            }
        }
        app.get_mut(self).shape = Some(value);
        self.notify_listeners(app);
    }

    fn mark_insets(self: Handle<Self>, app: &App) -> EdgeInsets {
        app.get(self).mark_insets
    }

    fn set_mark_insets(self: Handle<Self>, app: &mut App, value: EdgeInsets) {
        debug_assert!(value.is_non_negative(), "markInsets must be non-negative.");
        if app.get(self).mark_insets == value {
            return;
        }
        app.get_mut(self).mark_insets = value;
        // triggers repaint when it changes
        self.notify_listeners(app);
    }

    fn active_side(self: Handle<Self>, app: &App) -> BorderSide {
        app.get(self).active_side.expect("set before painting")
    }

    fn set_active_side(self: Handle<Self>, app: &mut App, value: BorderSide) {
        if app.get(self).active_side == Some(value) {
            return;
        }
        app.get_mut(self).active_side = Some(value);
        self.notify_listeners(app);
    }

    fn inactive_side(self: Handle<Self>, app: &App) -> BorderSide {
        app.get(self).inactive_side.expect("set before painting")
    }

    fn set_inactive_side(self: Handle<Self>, app: &mut App, value: BorderSide) {
        if app.get(self).inactive_side == Some(value) {
            return;
        }
        app.get_mut(self).inactive_side = Some(value);
        self.notify_listeners(app);
    }

    // The square outer bounds of the checkbox at t, with the specified origin.
    // At t == 0.0, the outer rect's size is _kEdgeSize (Checkbox.width)
    // At t == 0.5, .. is _kEdgeSize - _kStrokeWidth
    // At t == 1.0, .. is _kEdgeSize
    fn outer_rect_at(origin: Offset, t: f64) -> Rect {
        let inset = 1.0 - (t - 0.5).abs() * 2.0;
        let size = K_EDGE_SIZE - inset * K_STROKE_WIDTH;
        Rect::from_ltwh(origin.dx() + inset, origin.dy() + inset, size, size)
    }

    // The checkbox's fill color
    fn color_at(self: Handle<Self>, app: &App, t: f64) -> Color {
        // As t goes from 0.0 to 0.25, animate from the inactiveColor to activeColor.
        if t >= 0.25 {
            self.active_color(app)
        } else {
            Color::lerp(
                Some(self.inactive_color(app)),
                Some(self.active_color(app)),
                t * 4.0,
            )
            .expect("lerping non-null colors")
        }
    }

    // White stroke used to paint the check and dash.
    fn create_stroke_paint(self: Handle<Self>, app: &App) -> Paint {
        Paint {
            color: self.check_color(app).into(),
            style: PaintStyle::Stroke(Stroke::new(K_STROKE_WIDTH as f32)),
            ..Paint::default()
        }
    }

    // The square that the check mark or the indeterminate dash is drawn in.
    //
    // The square is inset by [markInsets] and then scaled down uniformly to fit
    // whatever space is left, so that the mark keeps its aspect ratio no matter
    // how asymmetric the insets are. It is centered in the space the insets leave
    // behind. Returns null when the insets leave no room to draw in.
    fn mark_square(self: Handle<Self>, app: &App) -> Option<Rect> {
        let mark_insets = self.mark_insets(app);
        let inner_width = clamp_double(K_EDGE_SIZE - mark_insets.horizontal(), 0.0, K_EDGE_SIZE);
        let inner_height = clamp_double(K_EDGE_SIZE - mark_insets.vertical(), 0.0, K_EDGE_SIZE);
        if inner_width <= 0.0 || inner_height <= 0.0 {
            return None;
        }
        let side = if inner_width < inner_height {
            inner_width
        } else {
            inner_height
        };
        Some(Rect::from_ltwh(
            mark_insets.left + (inner_width - side) / 2.0,
            mark_insets.top + (inner_height - side) / 2.0,
            side,
            side,
        ))
    }

    fn draw_box(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        outer: Rect,
        paint: &Paint,
        side: Option<BorderSide>,
    ) {
        let shape = self.shape(app);
        if shape.prefer_paint_interior() {
            shape.paint_interior(canvas, outer, paint, None);
        } else {
            let path = shape.get_outer_path(outer, None);
            canvas.draw_path(path.as_valo(), path.fill_type, paint);
        }
        if let Some(side) = side {
            shape.copy_with(Some(side)).paint(canvas, outer, None);
        }
    }

    fn draw_check(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        origin: Offset,
        t: f64,
        paint: &mut Paint,
    ) {
        debug_assert!((0.0..=1.0).contains(&t));
        // As t goes from 0.0 to 1.0, animate the two check mark strokes from the
        // short side to the long side.
        let Some(square) = self.mark_square(app) else {
            return;
        };
        // Shrink the stroke by the same factor so a smaller mark doesn't look chunky.
        set_stroke_width(paint, K_STROKE_WIDTH * (square.width() / K_EDGE_SIZE));

        let mut path = PathBuilder::new();
        let start = Offset::new(
            square.left + square.width() * 0.15,
            square.top + square.height() * 0.45,
        );
        let mid = Offset::new(
            square.left + square.width() * 0.4,
            square.top + square.height() * 0.7,
        );
        let end = Offset::new(
            square.left + square.width() * 0.85,
            square.top + square.height() * 0.25,
        );

        if t < 0.5 {
            let stroke_t = t * 2.0;
            let draw_mid = Offset::lerp(Some(start), Some(mid), stroke_t).expect("non-null");
            path.move_to(Offset::new(
                origin.dx() + start.dx(),
                origin.dy() + start.dy(),
            ));
            path.line_to(Offset::new(
                origin.dx() + draw_mid.dx(),
                origin.dy() + draw_mid.dy(),
            ));
        } else {
            let stroke_t = (t - 0.5) * 2.0;
            let draw_end = Offset::lerp(Some(mid), Some(end), stroke_t).expect("non-null");
            path.move_to(Offset::new(
                origin.dx() + start.dx(),
                origin.dy() + start.dy(),
            ));
            path.line_to(Offset::new(origin.dx() + mid.dx(), origin.dy() + mid.dy()));
            path.line_to(Offset::new(
                origin.dx() + draw_end.dx(),
                origin.dy() + draw_end.dy(),
            ));
        }
        canvas.draw_path(&path.build(), FillRule::NonZero, paint);
    }

    fn draw_dash(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        origin: Offset,
        t: f64,
        paint: &mut Paint,
    ) {
        debug_assert!((0.0..=1.0).contains(&t));
        // As t goes from 0.0 to 1.0, animate the horizontal line from the
        // mid point outwards.
        let Some(square) = self.mark_square(app) else {
            return;
        };
        set_stroke_width(paint, K_STROKE_WIDTH * (square.width() / K_EDGE_SIZE));

        let start = Offset::new(
            square.left + square.width() * 0.2,
            square.top + square.height() * 0.5,
        );
        let mid = Offset::new(
            square.left + square.width() * 0.5,
            square.top + square.height() * 0.5,
        );
        let end = Offset::new(
            square.left + square.width() * 0.8,
            square.top + square.height() * 0.5,
        );

        let draw_start = Offset::lerp(Some(start), Some(mid), 1.0 - t).expect("non-null");
        let draw_end = Offset::lerp(Some(mid), Some(end), t).expect("non-null");
        draw_line(canvas, origin + draw_start, origin + draw_end, paint);
    }
}

/// Dart's `paint.strokeWidth = width` on a stroke paint.
fn set_stroke_width(paint: &mut Paint, width: f64) {
    if let PaintStyle::Stroke(stroke) = &mut paint.style {
        stroke.width = width as f32;
    }
}

/// Dart's `Canvas.drawLine`: a stroked two-point path.
fn draw_line(canvas: &mut Canvas, p1: Offset, p2: Offset, paint: &Paint) {
    let mut path = PathBuilder::new();
    path.move_to(p1);
    path.line_to(p2);
    canvas.draw_path(&path.build(), FillRule::NonZero, paint);
}

impl ChangeNotifier for CheckboxPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ToggleablePainter for CheckboxPainter {
    fn toggleable_painter_data(self: Handle<Self>, app: &App) -> &ToggleablePainterData {
        &app.get(self).toggleable_painter
    }

    fn toggleable_painter_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ToggleablePainterData {
        &mut app.get_mut(self).toggleable_painter
    }

    fn paint(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, size: Size) {
        self.paint_radial_reaction(app, canvas, Offset::ZERO, size.center(Offset::ZERO));

        let mut stroke_paint = self.create_stroke_paint(app);
        let origin = size / 2.0 - Size::square(K_EDGE_SIZE) / 2.0;
        let position = self.position(app);
        let t_normalized = match position.status(app) {
            AnimationStatus::Forward | AnimationStatus::Completed => position.value(app),
            AnimationStatus::Reverse | AnimationStatus::Dismissed => 1.0 - position.value(app),
        };
        let value = self.value(app);
        let previous_value = self.previous_value(app);

        // Four cases: false to null, false to true, null to false, true to false
        if previous_value == Some(false) || value == Some(false) {
            let t = if value == Some(false) {
                1.0 - t_normalized
            } else {
                t_normalized
            };
            let outer = Self::outer_rect_at(origin, t);
            let paint = Paint::from_color(self.color_at(app, t).into());

            if t <= 0.5 {
                let border = BorderSide::lerp(self.inactive_side(app), self.active_side(app), t);
                self.draw_box(app, canvas, outer, &paint, Some(border));
            } else {
                self.draw_box(app, canvas, outer, &paint, Some(self.active_side(app)));
                let t_shrink = (t - 0.5) * 2.0;
                if previous_value.is_none() || value.is_none() {
                    self.draw_dash(app, canvas, origin, t_shrink, &mut stroke_paint);
                } else {
                    self.draw_check(app, canvas, origin, t_shrink, &mut stroke_paint);
                }
            }
        } else {
            // Two cases: null to true, true to null
            let outer = Self::outer_rect_at(origin, 1.0);
            let paint = Paint::from_color(self.color_at(app, 1.0).into());

            self.draw_box(app, canvas, outer, &paint, Some(self.active_side(app)));
            if t_normalized <= 0.5 {
                let t_shrink = 1.0 - t_normalized * 2.0;
                if previous_value.unwrap_or(false) {
                    self.draw_check(app, canvas, origin, t_shrink, &mut stroke_paint);
                } else {
                    self.draw_dash(app, canvas, origin, t_shrink, &mut stroke_paint);
                }
            } else {
                let t_expand = (t_normalized - 0.5) * 2.0;
                if value.unwrap_or(false) {
                    self.draw_check(app, canvas, origin, t_expand, &mut stroke_paint);
                } else {
                    self.draw_dash(app, canvas, origin, t_expand, &mut stroke_paint);
                }
            }
        }
    }
}

/// Hand coded defaults based on Material Design 2: Dart's `_CheckboxDefaultsM2`.
struct CheckboxDefaultsM2 {
    /// Dart's `super(..)` arguments.
    checkbox_theme_data: CheckboxThemeData,
    context: DefaultsContext,
}

impl CheckboxDefaultsM2 {
    fn new(app: &mut App, context: BuildContext) -> CheckboxDefaultsM2 {
        CheckboxDefaultsM2 {
            checkbox_theme_data: CheckboxThemeData::new(),
            context: DefaultsContext::with_theme(app, context),
        }
    }
}

impl CheckboxThemeDataGetters for CheckboxDefaultsM2 {
    fn checkbox_theme_data(&self) -> &CheckboxThemeData {
        &self.checkbox_theme_data
    }

    fn side(&self, app: &mut App) -> Option<AnyBorderSide> {
        let theme = self.context.theme(app);
        let disabled_color = theme.disabled_color;
        let unselected_widget_color = theme.unselected_widget_color;
        let side = WidgetStateBorderSide::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    return Some(BorderSide {
                        width: 2.0,
                        color: Colors::TRANSPARENT,
                        ..BorderSide::default()
                    });
                }
                return Some(BorderSide {
                    width: 2.0,
                    color: disabled_color,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Selected) {
                return Some(BorderSide {
                    width: 2.0,
                    color: Colors::TRANSPARENT,
                    ..BorderSide::default()
                });
            }
            Some(BorderSide {
                width: 2.0,
                color: unselected_widget_color,
                ..BorderSide::default()
            })
        });
        Some(side.into_any())
    }

    fn fill_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let disabled_color = theme.disabled_color;
        let secondary = theme.color_scheme.secondary;
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(disabled_color);
                    }
                    return Some(Colors::TRANSPARENT);
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(secondary);
                }
                Some(Colors::TRANSPARENT)
            },
        ))
    }

    fn check_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Color::new(0xFFFFFFFF),
        )))
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let fill_color = self.fill_color(app).expect("the fill color is given");
        let theme = self.context.theme(app);
        let hover_color = theme.hover_color;
        let focus_color = theme.focus_color;
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Pressed) {
                    return fill_color
                        .resolve(states)
                        .map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(hover_color);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(focus_color);
                }
                Some(Colors::TRANSPARENT)
            },
        ))
    }

    fn splash_radius(&self, _app: &mut App) -> Option<f64> {
        Some(K_RADIAL_REACTION_RADIUS)
    }

    fn material_tap_target_size(&self, app: &mut App) -> Option<MaterialTapTargetSize> {
        let theme = self.context.theme(app);
        Some(theme.material_tap_target_size)
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }

    fn shape(&self, _app: &mut App) -> Option<OutlinedBorderRef> {
        Some(OutlinedBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(1.0)),
        )))
    }
}

#[cfg(test)]
// Dart's cases build their expected colours with the deprecated `withOpacity`.
#[allow(deprecated)]
pub(crate) mod tests {
    use std::rc::Rc;
    use std::sync::Arc;
    use waft_widgets::AnyFocusNode;

    use waft_embedder::valo::{Op, Path};
    use waft_embedder::{PointerChange, RRect};
    use waft_foundation::K_IS_WEB;
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject,
    };
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Center, Container, FocusHighlightStrategy, FocusNode, MouseRegion,
        StatefulBuilder,
    };

    use super::*;
    use crate::material::Material;
    use crate::test_support::{TestPointer, WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    pub(crate) use crate::tooltip::tests::{m2, m3};

    pub(crate) fn tester() -> WidgetTester {
        tester_on(TargetPlatform::Android)
    }

    pub(crate) fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    /// A callback that ignores the value it is handed: Dart's `(bool? value) {}`.
    pub(crate) fn ignore() -> Option<ValueChanged<Option<bool>>> {
        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {}))
    }

    /// `tester.pumpWidget(MaterialApp(theme:, home: Material(child: Center(child: checkbox))))`.
    pub(crate) fn pump_checkbox(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        checkbox: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = theme(&app);
            let checkbox = checkbox(&mut app);
            material_app(
                theme,
                Material::new()
                    .child(Center::new().child(checkbox))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// A checkbox whose value is kept by a `StatefulBuilder` and set by its own `on_changed`.
    pub(crate) fn stateful(
        value: Rc<std::cell::Cell<Option<bool>>>,
        configure: impl Fn(Checkbox) -> Checkbox + 'static,
    ) -> WidgetRef {
        StatefulBuilder::new(move |_app, _context, set_state| {
            let value_now = Rc::clone(&value);
            configure(Checkbox::new(
                value.get(),
                Some(Rc::new(move |app: &mut App, new_value: Option<bool>| {
                    let value = Rc::clone(&value_now);
                    set_state(app, &mut |_app| value.set(new_value));
                })),
            ))
            .into_widget()
        })
        .into_widget()
    }

    /// Dart's `find.byType(Checkbox)`, which expects one.
    pub(crate) fn checkbox_element(tester: &WidgetTester) -> AnyElement {
        let checkboxes = tester.find_by_type::<Checkbox>();
        assert_eq!(checkboxes.len(), 1, "one Checkbox");
        checkboxes[0]
    }

    /// Dart's `tester.getCenter(find.byType(Checkbox))`.
    pub(crate) fn checkbox_center(tester: &WidgetTester) -> Offset {
        tester
            .global_rect(tester.render_box_of(checkbox_element(tester)))
            .center()
    }

    /// Dart's `tester.getSize(find.byType(Checkbox))`.
    pub(crate) fn checkbox_size(tester: &WidgetTester) -> Size {
        let render = tester.render_box_of(checkbox_element(tester));
        render.size(&tester.cell.borrow())
    }

    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    /// The ops the checkbox's painter records, in paint order, in the checkbox's coordinates.
    fn ops(tester: &WidgetTester) -> Vec<Op> {
        let element = checkbox_element(tester);
        let mut app = tester.cell.borrow_mut();
        let root = element
            .find_render_object(&app)
            .expect("the checkbox has been laid out");
        let custom = find::<RenderCustomPaint>(&app, root).expect("the checkbox's paint");
        let painter = custom.painter(&app).expect("the checkbox's painter");
        let size = custom.size(&app);
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, size);
        canvas.build().ops().to_vec()
    }

    /// One op the checkbox's painter records, reduced to what Dart's `paints` matchers compare.
    ///
    /// A draw with a transparent paint is not recorded at all, so Dart's
    /// `rrect(color: Colors.transparent)` has no entry: the case asserts that no other fill is
    /// drawn instead.
    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Drawn {
        /// A filled oval (Dart's `circle`): its colour as `0xAARRGGBB` and its bounds.
        Circle(u32, Rect),
        /// Any other filled path (Dart's `rrect`): its colour and its bounds.
        Fill(u32, Rect),
        /// A filled ring (Dart's `drrect`): its colour and its even-odd path.
        Ring(u32, Arc<Path>),
        /// A stroked path (Dart's `path`, and `line` when its bounds have no height): its
        /// colour, its stroke width and the path's bounds.
        Stroke(u32, f32, Rect),
    }

    fn argb(color: waft_embedder::valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    fn rect_of(rect: waft_embedder::valo::Rect) -> Rect {
        Rect::from_ltwh(
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.width),
            f64::from(rect.height),
        )
    }

    impl Drawn {
        fn of(op: &Op) -> Option<Drawn> {
            let Op::DrawPath {
                paint,
                path,
                fill_rule,
                content_bounds,
                ..
            } = op
            else {
                return None;
            };
            Some(match &paint.style {
                PaintStyle::Stroke(stroke) => {
                    Drawn::Stroke(argb(paint.color), stroke.width, rect_of(path.bounds()))
                }
                PaintStyle::Fill if *fill_rule == FillRule::EvenOdd => {
                    Drawn::Ring(argb(paint.color), Arc::clone(path))
                }
                PaintStyle::Fill if path.as_oval().is_some() => {
                    Drawn::Circle(argb(paint.color), rect_of(*content_bounds))
                }
                PaintStyle::Fill => Drawn::Fill(argb(paint.color), rect_of(*content_bounds)),
            })
        }
    }

    /// What the checkbox's painter draws, in paint order.
    pub(crate) fn drawn(tester: &WidgetTester) -> Vec<Drawn> {
        ops(tester).iter().filter_map(Drawn::of).collect()
    }

    /// One call of Dart's `paints` matcher.
    #[derive(Debug)]
    pub(crate) enum Paints {
        /// `circle(color:)`.
        Circle(Color),
        /// `circle(color:, radius:)`.
        CircleRadius(Color, f64),
        /// `circle(x:, y:)`.
        CircleAt(Offset),
        /// `rrect(color:)`: the box's fill.
        Rrect(Color),
        /// `path(color:)`: the check mark.
        Path(Color),
        /// `path(color:, style: PaintingStyle.stroke, strokeWidth:)`.
        PathStroke(Color, f32),
        /// `path(strokeWidth:)` or `line(strokeWidth:)`.
        StrokeWidth(f32),
        /// `line()`: the dash.
        Line,
        /// `line(strokeWidth:)`.
        LineWidth(f32),
        /// `drrect(color:)`: the border.
        Ring(Color),
        /// `drrect(color:, outer:, inner:)`.
        RingAt(Color, RRect, RRect),
        /// `drrect(outer:, inner:)`: a ring of any colour.
        RingShape(RRect, RRect),
    }

    /// The ring `RoundedRectangleBorder.paint` draws between `outer` and `inner`.
    fn ring_path(outer: RRect, inner: RRect) -> Arc<Path> {
        let mut canvas = Canvas::new();
        waft_painting::draw_drrect(
            &mut canvas,
            outer,
            inner,
            &Paint::from_color(Color::new(0xFF000000).into()),
        );
        match &canvas.build().ops()[0] {
            Op::DrawPath { path, .. } => Arc::clone(path),
            _ => panic!("draw_drrect records a path"),
        }
    }

    impl Paints {
        fn matches(&self, drawn: &Drawn) -> bool {
            match (self, drawn) {
                (Paints::Circle(color), Drawn::Circle(c, bounds)) => {
                    *c == color.to_argb32() && bounds.width() == bounds.height()
                }
                (Paints::CircleRadius(color, radius), Drawn::Circle(c, bounds)) => {
                    *c == color.to_argb32()
                        && bounds.width() == bounds.height()
                        && (bounds.width() - radius * 2.0).abs() < 1e-3
                }
                (Paints::CircleAt(center), Drawn::Circle(_, bounds)) => {
                    (bounds.center() - *center).distance() < 1e-3
                }
                (Paints::Rrect(color), Drawn::Fill(c, bounds)) => {
                    *c == color.to_argb32() && bounds.width() <= K_EDGE_SIZE + 1e-3
                }
                (Paints::Path(color), Drawn::Stroke(c, _, bounds)) => {
                    *c == color.to_argb32() && bounds.height() > 0.0
                }
                (Paints::PathStroke(color, width), Drawn::Stroke(c, w, bounds)) => {
                    *c == color.to_argb32() && w == width && bounds.height() > 0.0
                }
                (Paints::StrokeWidth(width), Drawn::Stroke(_, w, _)) => w == width,
                (Paints::Line, Drawn::Stroke(_, _, bounds)) => bounds.height() == 0.0,
                (Paints::LineWidth(width), Drawn::Stroke(_, w, bounds)) => {
                    w == width && bounds.height() == 0.0
                }
                (Paints::Ring(color), Drawn::Ring(c, _)) => *c == color.to_argb32(),
                (Paints::RingAt(color, outer, inner), Drawn::Ring(c, path)) => {
                    *c == color.to_argb32() && *path == ring_path(*outer, *inner)
                }
                (Paints::RingShape(outer, inner), Drawn::Ring(_, path)) => {
                    *path == ring_path(*outer, *inner)
                }
                _ => false,
            }
        }
    }

    /// Dart's `expect(renderer, paints..a..b)`: the calls are drawn in this order, others
    /// between them allowed.
    pub(crate) fn paints(tester: &WidgetTester, expected: &[Paints]) -> bool {
        let drawn = drawn(tester);
        let mut rest = drawn.iter();
        expected
            .iter()
            .all(|expected| rest.by_ref().any(|drawn| expected.matches(drawn)))
    }

    pub(crate) fn assert_paints(tester: &WidgetTester, expected: &[Paints]) {
        assert!(
            paints(tester, expected),
            "expected {expected:?} in {:?}",
            drawn(tester)
        );
    }

    pub(crate) fn assert_does_not_paint(tester: &WidgetTester, expected: &[Paints]) {
        assert!(
            !paints(tester, expected),
            "did not expect {expected:?} in {:?}",
            drawn(tester)
        );
    }

    /// Dart's `startGesture(center)`: a touch held down on the checkbox.
    pub(crate) fn press(tester: &mut WidgetTester, pointer: i64) -> TestPointer {
        let center = checkbox_center(tester);
        let mut touch = WidgetTester::touch(pointer);
        tester.down(&mut touch, center);
        touch
    }

    /// Dart's `createGesture(kind: mouse)`, `addPointer()` and `moveTo(center)`.
    pub(crate) fn hover(tester: &mut WidgetTester) -> TestPointer {
        let center = checkbox_center(tester);
        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, center, false);
        mouse
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<waft_widgets::AnyFocusNode>()
    }

    fn has_primary_focus(tester: &WidgetTester, node: AnyFocusNode) -> bool {
        node.has_primary_focus(&tester.cell.borrow())
    }

    fn rrect(left: f64, top: f64, right: f64, bottom: f64, radius: f64) -> RRect {
        RRect::from_ltrbr(left, top, right, bottom, Radius::circular(radius))
    }

    fn orange() -> Color {
        Colors::ORANGE.shade500()
    }

    #[test]
    fn checkbox_size_is_configurable_by_theme_data_material_tap_target_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for (size, expected) in [
                (MaterialTapTargetSize::Padded, 48.0),
                (MaterialTapTargetSize::ShrinkWrap, 40.0),
            ] {
                pump_checkbox(
                    &mut tester,
                    |app| {
                        ThemeData::new(app)
                            .copy_with()
                            .material_tap_target_size(size)
                            .build()
                    },
                    |_| Checkbox::new(Some(true), ignore()).into_widget(),
                );
                // `MaterialApp` animates to the new theme, which Dart's bare `Theme` does not.
                tester.pump_and_settle();
                assert_eq!(checkbox_size(&tester), Size::new(expected, expected));
            }
        });
    }

    #[test]
    fn checkbox_tristate_true() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(std::cell::Cell::new(None));
            let host = stateful(Rc::clone(&value), |checkbox| checkbox.tristate(true));
            pump_checkbox(&mut tester, m3, |_| host);
            let shown = |tester: &WidgetTester| {
                tester.widget(checkbox_element(tester), |checkbox: &Checkbox| {
                    checkbox.value
                })
            };
            assert_eq!(shown(&tester), None);
            for expected in [Some(false), Some(true), None] {
                let center = checkbox_center(&tester);
                tester.tap(center, 1);
                tester.pump_and_settle();
                assert_eq!(value.get(), expected);
            }
        });
    }

    fn tristate_rendering(theme: fn(&App) -> ThemeData) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors = theme(&tester.cell.borrow()).color_scheme.clone();
            let use_material3 = theme(&tester.cell.borrow()).use_material3;
            let (fill, check) = if use_material3 {
                (colors.primary, colors.on_primary)
            } else {
                (colors.secondary, Color::new(0xFFFFFFFF))
            };
            let frame = |tester: &mut WidgetTester, value: Option<bool>| {
                pump_checkbox(tester, theme, move |_| {
                    Checkbox::new(value, ignore()).tristate(true).into_widget()
                });
                tester.pump_and_settle();
            };
            let assert_empty = |tester: &WidgetTester| {
                // The transparent fill is not recorded: the ring is the first draw.
                assert!(
                    matches!(drawn(tester).first(), Some(Drawn::Ring(..))),
                    "{:?}",
                    drawn(tester)
                );
                // null is rendered as a line (a "dash")
                assert_does_not_paint(tester, &[Paints::Line]);
            };

            frame(&mut tester, Some(false));
            assert_empty(&tester);

            frame(&mut tester, Some(true));
            // checkmark is rendered as a path
            assert_paints(&tester, &[Paints::Rrect(fill), Paints::Path(check)]);

            frame(&mut tester, Some(false));
            assert_empty(&tester);

            frame(&mut tester, None);
            assert_paints(&tester, &[Paints::Line]);

            frame(&mut tester, Some(true));
            assert_paints(&tester, &[Paints::Rrect(fill), Paints::Path(check)]);

            frame(&mut tester, None);
            assert_paints(&tester, &[Paints::Line]);
        });
    }

    #[test]
    fn material2_checkbox_tristate_rendering_programmatic_transitions() {
        tristate_rendering(m2);
    }

    #[test]
    fn material3_checkbox_tristate_rendering_programmatic_transitions() {
        tristate_rendering(m3);
    }

    fn color_rendering(use_material3: bool, border_color: Color) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme =
                move |app: &App| ThemeData::builder().use_material3(use_material3).build(app);
            let frame = |tester: &mut WidgetTester,
                         theme: &dyn Fn(&App) -> ThemeData,
                         active_color: Option<Color>,
                         check_color: Option<Color>| {
                pump_checkbox(tester, theme, move |_| {
                    let mut checkbox = Checkbox::new(Some(true), ignore());
                    checkbox.active_color = active_color;
                    checkbox.check_color = check_color;
                    checkbox.into_widget()
                });
                tester.pump_and_settle();
            };

            // paints's color is 0xFFFFFFFF (default color)
            let check_color = Color::new(0xFFFFFFFF);
            frame(&mut tester, &theme, None, Some(check_color));
            assert_paints(
                &tester,
                &[Paints::Rrect(border_color), Paints::Path(check_color)],
            );

            // paints's color is 0xFF000000 (params)
            let check_color = Color::new(0xFF000000);
            frame(&mut tester, &theme, None, Some(check_color));
            assert_paints(
                &tester,
                &[Paints::Rrect(border_color), Paints::Path(check_color)],
            );

            // paints's color is 0xFF00FF00 (theme)
            let active_color = Color::new(0xFF00FF00);
            let themed = move |app: &App| {
                let color_scheme = if use_material3 {
                    ColorSchemeLight::light().with_primary(active_color)
                } else {
                    ColorSchemeLight::light().with_secondary(active_color)
                };
                theme(app).copy_with().color_scheme(color_scheme).build()
            };
            frame(&mut tester, &themed, None, None);
            assert_paints(&tester, &[Paints::Rrect(active_color)]);

            let active_color = Color::new(0xFF000000);
            frame(&mut tester, &theme, Some(active_color), None);
            assert_paints(&tester, &[Paints::Rrect(active_color)]);
        });
    }

    use crate::color_scheme::ColorScheme as ColorSchemeLight;

    #[test]
    fn material2_checkbox_color_rendering() {
        color_rendering(false, Color::new(0xff2196f3));
    }

    #[test]
    fn material3_checkbox_color_rendering() {
        color_rendering(true, Color::new(0xFF6750A4));
    }

    /// The focus cases' `buildApp`: a focused, autofocusing checkbox whose host keeps its value.
    fn focus_frame(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        value: Option<bool>,
        enabled: bool,
        node: FocusNodeRef,
    ) {
        pump_checkbox(tester, theme, move |_| {
            Checkbox::new(value, if enabled { ignore() } else { None })
                .focus_color(orange())
                .autofocus(true)
                .focus_node(node.clone())
                .into_widget()
        });
        tester.pump_and_settle();
    }

    #[test]
    fn material2_checkbox_is_focusable_and_has_correct_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            focus_frame(&mut tester, m2, Some(true), true, node.clone());
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[
                    Paints::Circle(orange()),
                    Paints::Rrect(Color::new(0xff2196f3)),
                    Paints::Path(Colors::WHITE),
                ],
            );

            // Check the false value.
            focus_frame(&mut tester, m2, Some(false), true, node.clone());
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[
                    Paints::Circle(orange()),
                    Paints::RingAt(
                        Color::new(0x8a000000),
                        rrect(15.0, 15.0, 33.0, 33.0, 1.0),
                        rrect(17.0, 17.0, 31.0, 31.0, 0.0),
                    ),
                ],
            );

            // Check what happens when disabled.
            focus_frame(&mut tester, m2, Some(false), false, node.clone());
            assert!(!has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[Paints::RingAt(
                    Color::new(0x61000000),
                    rrect(15.0, 15.0, 33.0, 33.0, 1.0),
                    rrect(17.0, 17.0, 31.0, 31.0, 0.0),
                )],
            );
        });
    }

    #[test]
    fn material3_checkbox_is_focusable_and_has_correct_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors = m3(&tester.cell.borrow()).color_scheme.clone();
            let node = focus_node(&tester);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            focus_frame(&mut tester, m3, Some(true), true, node.clone());
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[
                    Paints::Circle(orange()),
                    Paints::Rrect(colors.primary),
                    Paints::Path(colors.on_primary),
                ],
            );

            // Check the false value.
            focus_frame(&mut tester, m3, Some(false), true, node.clone());
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[
                    Paints::Circle(orange()),
                    Paints::RingAt(
                        colors.on_surface,
                        rrect(15.0, 15.0, 33.0, 33.0, 2.0),
                        rrect(17.0, 17.0, 31.0, 31.0, 0.0),
                    ),
                ],
            );

            // Check what happens when disabled.
            focus_frame(&mut tester, m3, Some(false), false, node.clone());
            assert!(!has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[Paints::RingAt(
                    colors.on_surface.with_opacity(0.38),
                    rrect(15.0, 15.0, 33.0, 33.0, 2.0),
                    rrect(17.0, 17.0, 31.0, 31.0, 0.0),
                )],
            );
        });
    }

    #[test]
    fn checkbox_with_splash_radius_set() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 30.0;
            pump_checkbox(&mut tester, m3, |_| {
                Checkbox::new(Some(false), ignore())
                    .focus_color(orange())
                    .autofocus(true)
                    .splash_radius(SPLASH_RADIUS)
                    .into_widget()
            });
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::CircleRadius(orange(), SPLASH_RADIUS)]);
        });
    }

    #[test]
    fn checkbox_starts_the_splash_in_center_even_when_tap_is_on_the_corner() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_checkbox(&mut tester, m3, |_| {
                Checkbox::new(Some(false), ignore()).into_widget()
            });
            let rect = tester.global_rect(tester.render_box_of(checkbox_element(&tester)));
            let checkbox_center_local = rect.center() - rect.top_left();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, rect.top_left());
            tester.pump(Duration::ZERO);
            // Wait for the splash to be drawn, but not long enough for it to animate towards
            // the center, since we want to catch it in its starting position.
            tester.pump(Duration::from_millis(1));
            assert_paints(&tester, &[Paints::CircleAt(checkbox_center_local)]);

            // Finish gesture to release resources.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    fn hover_color_case(theme: fn(&App) -> ThemeData) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let data = theme(&tester.cell.borrow());
            let colors = data.color_scheme.clone();
            let (fill, check) = if data.use_material3 {
                (Color::new(0xff6750a4), colors.on_primary)
            } else {
                (Color::new(0xff2196f3), Color::new(0xffffffff))
            };
            let frame = |tester: &mut WidgetTester, enabled: bool| {
                pump_checkbox(tester, theme, move |_| {
                    Checkbox::new(Some(true), if enabled { ignore() } else { None })
                        .hover_color(orange())
                        .into_widget()
                });
                tester.pump_and_settle();
            };

            frame(&mut tester, true);
            assert_paints(
                &tester,
                &[Paints::Rrect(fill), Paints::PathStroke(check, 2.0)],
            );

            // Start hovering
            let center = checkbox_center(&tester);
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);

            frame(&mut tester, true);
            assert_paints(
                &tester,
                &[
                    Paints::Circle(orange()),
                    Paints::Rrect(fill),
                    Paints::PathStroke(check, 2.0),
                ],
            );

            // Check what happens when disabled.
            frame(&mut tester, false);
            let (disabled_fill, disabled_check) = if data.use_material3 {
                (colors.on_surface.with_opacity(0.38), colors.surface)
            } else {
                (Color::new(0x61000000), Color::new(0xffffffff))
            };
            assert_paints(
                &tester,
                &[
                    Paints::Rrect(disabled_fill),
                    Paints::PathStroke(disabled_check, 2.0),
                ],
            );
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material2_checkbox_can_be_hovered_and_has_correct_hover_color() {
        hover_color_case(m2);
    }

    #[test]
    fn material3_checkbox_can_be_hovered_and_has_correct_hover_color() {
        hover_color_case(m3);
    }

    #[test]
    fn checkbox_can_be_toggled_by_keyboard_shortcuts() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(std::cell::Cell::new(Some(true)));
            let host = stateful(Rc::clone(&value), |checkbox| {
                checkbox.focus_color(orange()).autofocus(true)
            });
            pump_checkbox(&mut tester, m3, |_| host);
            tester.pump_and_settle();
            tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            // On web, checkboxes don't respond to the enter key.
            assert_eq!(value.get(), Some(K_IS_WEB));
            tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            assert_eq!(value.get(), Some(true));
            tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
            tester.pump_and_settle();
            assert_eq!(value.get(), Some(false));
            tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
            tester.pump_and_settle();
            assert_eq!(value.get(), Some(true));
        });
    }

    #[test]
    fn material3_checkbox_visual_density_cannot_be_overridden_by_theme_data_visual_density() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_checkbox(
                &mut tester,
                |app| {
                    ThemeData::new(app)
                        .copy_with()
                        .visual_density(VisualDensity::COMPACT)
                        .build()
                },
                |_| Checkbox::new(Some(true), ignore()).into_widget(),
            );
            tester.pump_and_settle();
            assert_eq!(checkbox_size(&tester), Size::new(48.0, 48.0));
        });
    }

    #[test]
    fn material3_checkbox_with_material_tap_target_size_padded_meets_material_guidelines_on_desktop()
     {
        for platform in [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                pump_checkbox(&mut tester, m3, |_| {
                    Checkbox::new(Some(true), ignore())
                        .material_tap_target_size(MaterialTapTargetSize::Padded)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_eq!(
                    checkbox_size(&tester),
                    Size::new(48.0, 48.0),
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn checkbox_responds_to_density_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for (visual_density, expected) in [
                (None, Size::new(48.0, 48.0)),
                (Some(VisualDensity::STANDARD), Size::new(48.0, 48.0)),
                (Some(VisualDensity::COMPACT), Size::new(40.0, 40.0)),
                (Some(VisualDensity::new(3.0, 3.0)), Size::new(60.0, 60.0)),
                (Some(VisualDensity::new(-3.0, -3.0)), Size::new(36.0, 36.0)),
                (Some(VisualDensity::new(3.0, -3.0)), Size::new(60.0, 36.0)),
            ] {
                pump_checkbox(&mut tester, m3, move |_| {
                    let mut checkbox = Checkbox::new(Some(true), ignore());
                    checkbox.visual_density = visual_density;
                    checkbox.into_widget()
                });
                tester.pump_and_settle();
                assert_eq!(checkbox_size(&tester), expected, "{visual_density:?}");
            }
        });
    }

    #[test]
    fn checkbox_stops_hover_animation_when_removed_from_the_tree() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = |app: &App| {
                ThemeData::builder()
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .build(app)
            };
            let value = Rc::new(std::cell::Cell::new(Some(true)));
            let host = stateful(Rc::clone(&value), |checkbox| checkbox);
            pump_checkbox(&mut tester, theme, |_| host);

            let checkbox_center = checkbox_center(&tester);
            let mut test_gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut test_gesture, Offset::ZERO);
            tester.move_to(&mut test_gesture, checkbox_center, false);

            tester.pump(Duration::ZERO); // start animation
            // hover animation duration is 50 ms. It is half-way.
            tester.pump(Duration::from_millis(25));

            // Hover animation should not trigger an exception when the checkbox is removed
            // before the hover animation should complete.
            pump_checkbox(&mut tester, theme, |_| Container::new().into_widget());

            tester.send(&test_gesture, PointerChange::Remove);
        });
    }

    fn selected_grab_mouse_cursor() -> MouseCursorRef {
        WidgetStateMouseCursor::resolve_with(
            |states| {
                if states.contains(&WidgetState::Selected) {
                    SystemMouseCursors::GRAB.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                }
            },
            Some("_SelectedGrabMouseCursor()"),
        )
        .into()
    }

    #[test]
    fn checkbox_changes_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = |tester: &mut WidgetTester, checkbox: Checkbox| {
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        m3(&app),
                        crate::scaffold::Scaffold::new()
                            .body(
                                waft_widgets::Align::new()
                                    .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                                    .child(
                                        Material::new().child(
                                            MouseRegion::new()
                                                .cursor(SystemMouseCursors::FORBIDDEN.into())
                                                .child(checkbox),
                                        ),
                                    ),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);
            };

            // Test Checkbox() constructor
            frame(
                &mut tester,
                Checkbox::new(Some(true), ignore()).mouse_cursor(SystemMouseCursors::TEXT.into()),
            );
            let center = checkbox_center(&tester);
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, center);
            tester.pump(Duration::ZERO);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            frame(&mut tester, Checkbox::new(Some(true), ignore()));
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert_eq!(Some(tester.active_cursor(1)), Some(expected));

            // Test default cursor when disabled
            frame(&mut tester, Checkbox::new(Some(true), None));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());

            // Test cursor when tristate
            frame(
                &mut tester,
                Checkbox::new(None, None)
                    .tristate(true)
                    .mouse_cursor(selected_grab_mouse_cursor()),
            );
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::GRAB.into());

            tester.pump_and_settle();
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_fill_color_resolves_in_enabled_disabled_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const ACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000002);
            let fill_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(ACTIVE_DISABLED_FILL_COLOR);
                }
                Some(ACTIVE_ENABLED_FILL_COLOR)
            });
            for (enabled, expected) in [
                (true, ACTIVE_ENABLED_FILL_COLOR),
                (false, ACTIVE_DISABLED_FILL_COLOR),
            ] {
                let fill_color = Rc::clone(&fill_color);
                pump_checkbox(&mut tester, m3, move |_| {
                    Checkbox::new(Some(true), if enabled { ignore() } else { None })
                        .fill_color(fill_color)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_paints(&tester, &[Paints::Rrect(expected)]);
            }
        });
    }

    #[test]
    fn checkbox_fill_color_resolves_in_hovered_focused_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_FILL_COLOR: Color = Color::new(0xFF000001);
            const FOCUSED_FILL_COLOR: Color = Color::new(0xFF000002);
            let fill_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVERED_FILL_COLOR);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(FOCUSED_FILL_COLOR);
                }
                Some(Colors::TRANSPARENT)
            });
            pump_checkbox(&mut tester, m3, {
                let node = node.clone();
                move |_| {
                    Checkbox::new(Some(true), ignore())
                        .focus_node(node.clone())
                        .autofocus(true)
                        .fill_color(fill_color)
                        .into_widget()
                }
            });
            tester.pump_and_settle();
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(&tester, &[Paints::Rrect(FOCUSED_FILL_COLOR)]);

            // Start hovering
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::Rrect(HOVERED_FILL_COLOR)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_respects_shape_and_side() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let rounded_rectangle_border = RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(5.0)),
            );
            let side = BorderSide {
                width: 4.0,
                color: Color::new(0xfff44336),
                ..BorderSide::default()
            };
            let border = rounded_rectangle_border;
            pump_checkbox(&mut tester, m3, move |_| {
                Checkbox::new(Some(false), ignore())
                    .shape(border)
                    .side(side)
                    .into_widget()
            });
            tester.pump_and_settle();

            tester.widget(checkbox_element(&tester), |checkbox: &Checkbox| {
                let shape: &dyn ShapeBorder = checkbox.shape.as_deref().expect("a shape");
                assert!(shape == &rounded_rectangle_border as &dyn ShapeBorder);
                assert_eq!(checkbox.side, Some(AnyBorderSide::new(side)));
            });
            assert_paints(
                &tester,
                &[Paints::RingAt(
                    Color::new(0xfff44336),
                    rrect(15.0, 15.0, 33.0, 33.0, 5.0),
                    rrect(19.0, 19.0, 29.0, 29.0, 1.0),
                )],
            );
        });
    }

    /// The default overlay cases' `buildCheckbox`: a checkbox in a `Scaffold`'s body.
    fn overlay_frame(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        configure: impl FnOnce(Checkbox) -> Checkbox,
        active: bool,
        focused: bool,
        node: FocusNodeRef,
    ) {
        let checkbox = configure(
            Checkbox::new(Some(active), ignore())
                .focus_node(node.clone())
                .autofocus(focused),
        );
        let root = {
            let app = tester.cell.borrow();
            material_app(
                theme(&app),
                crate::scaffold::Scaffold::new()
                    .body(checkbox)
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// Dart's "Finish gestures to release resources": `gesture.up()` for each touch still down,
    /// then `pumpAndSettle`.
    fn finish_gestures(tester: &mut WidgetTester, gestures: &[&TestPointer]) {
        for gesture in gestures {
            tester.up(gesture);
        }
        tester.pump_and_settle();
    }

    fn reset(tester: &mut WidgetTester) {
        tester.pump_widget(Container::new().into_widget());
    }

    fn default_overlay_case(
        theme: fn(&App) -> ThemeData,
        inactive_pressed: fn(&ThemeData) -> Color,
        active_pressed: fn(&ThemeData) -> Color,
        focused: fn(&ThemeData) -> Color,
        hovered: fn(&ThemeData) -> Color,
    ) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let data = theme(&tester.cell.borrow());

            overlay_frame(
                &mut tester,
                theme,
                |checkbox| checkbox,
                false,
                false,
                node.clone(),
            );
            let gesture1 = press(&mut tester, 1);
            tester.pump_and_settle();
            // Default inactive pressed Checkbox should have overlay color from default
            // fillColor
            assert_paints(&tester, &[Paints::Circle(inactive_pressed(&data))]);

            overlay_frame(
                &mut tester,
                theme,
                |checkbox| checkbox,
                true,
                false,
                node.clone(),
            );
            let gesture2 = press(&mut tester, 2);
            tester.pump_and_settle();
            // Default active pressed Checkbox should have overlay color from default fillColor
            assert_paints(&tester, &[Paints::Circle(active_pressed(&data))]);

            reset(&mut tester);
            overlay_frame(
                &mut tester,
                theme,
                |checkbox| checkbox,
                false,
                true,
                node.clone(),
            );
            tester.pump_and_settle();
            assert!(has_primary_focus(&tester, node.handle()));
            // Focused Checkbox should use default focused overlay color
            assert_paints(&tester, &[Paints::Circle(focused(&data))]);

            reset(&mut tester);
            overlay_frame(
                &mut tester,
                theme,
                |checkbox| checkbox,
                false,
                false,
                node.clone(),
            );
            let gesture3 = hover(&mut tester);
            tester.pump_and_settle();
            // Hovered Checkbox should use default hovered overlay color
            assert_paints(&tester, &[Paints::Circle(hovered(&data))]);

            // Finish gestures to release resources.
            finish_gestures(&mut tester, &[&gesture1, &gesture2]);
            tester.send(&gesture3, PointerChange::Remove);
        });
    }

    #[test]
    fn material2_checkbox_default_overlay_color_in_active_pressed_focused_hovered_states() {
        default_overlay_case(
            m2,
            |theme| {
                theme
                    .unselected_widget_color
                    .with_alpha(K_RADIAL_REACTION_ALPHA)
            },
            |theme| {
                theme
                    .color_scheme
                    .secondary
                    .with_alpha(K_RADIAL_REACTION_ALPHA)
            },
            |theme| theme.focus_color,
            |theme| theme.hover_color,
        );
    }

    #[test]
    fn material3_checkbox_default_overlay_color_in_active_pressed_focused_hovered_states() {
        default_overlay_case(
            m3,
            |theme| theme.color_scheme.primary.with_opacity(0.1),
            |theme| theme.color_scheme.on_surface.with_opacity(0.1),
            |theme| theme.color_scheme.on_surface.with_opacity(0.1),
            |theme| theme.color_scheme.on_surface.with_opacity(0.08),
        );
    }

    #[test]
    fn checkbox_overlay_color_resolves_in_active_pressed_focused_hovered_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const FOCUS_OVERLAY_COLOR: Color = Color::new(0xFF000004);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
            const FOCUS_COLOR: Color = Color::new(0xFF000006);
            const SPLASH_RADIUS: f64 = 24.0;

            let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Pressed) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                    }
                    return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVER_OVERLAY_COLOR);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(FOCUS_OVERLAY_COLOR);
                }
                None
            });
            let frame =
                |tester: &mut WidgetTester, active: bool, focused: bool, use_overlay: bool| {
                    let overlay_color = Rc::clone(&overlay_color);
                    overlay_frame(
                        tester,
                        m3,
                        move |checkbox| {
                            let checkbox = checkbox
                                .fill_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                                    FILL_COLOR,
                                )))
                                .hover_color(HOVER_COLOR)
                                .focus_color(FOCUS_COLOR)
                                .splash_radius(SPLASH_RADIUS);
                            if use_overlay {
                                checkbox.overlay_color(overlay_color)
                            } else {
                                checkbox
                            }
                        },
                        active,
                        focused,
                        node.clone(),
                    );
                };

            frame(&mut tester, false, false, false);
            let gesture1 = press(&mut tester, 1);
            tester.pump_and_settle();
            // Default inactive pressed Checkbox should have overlay color from fillColor
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA),
                    SPLASH_RADIUS,
                )],
            );

            frame(&mut tester, true, false, false);
            let gesture2 = press(&mut tester, 2);
            tester.pump_and_settle();
            // Default active pressed Checkbox should have overlay color from fillColor
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA),
                    SPLASH_RADIUS,
                )],
            );

            frame(&mut tester, false, false, true);
            let gesture3 = press(&mut tester, 3);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    INACTIVE_PRESSED_OVERLAY_COLOR,
                    SPLASH_RADIUS,
                )],
            );

            frame(&mut tester, true, false, true);
            let gesture4 = press(&mut tester, 4);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    ACTIVE_PRESSED_OVERLAY_COLOR,
                    SPLASH_RADIUS,
                )],
            );

            reset(&mut tester);
            frame(&mut tester, false, true, true);
            tester.pump_and_settle();
            assert!(has_primary_focus(&tester, node.handle()));
            // Focused Checkbox should use overlay color over focusColor
            assert_paints(
                &tester,
                &[Paints::CircleRadius(FOCUS_OVERLAY_COLOR, SPLASH_RADIUS)],
            );

            // Start hovering
            let gesture5 = hover(&mut tester);
            tester.pump_and_settle();
            // Hovered Checkbox should use overlay color over hoverColor
            assert_paints(
                &tester,
                &[Paints::CircleRadius(HOVER_OVERLAY_COLOR, SPLASH_RADIUS)],
            );

            // Finish gestures to release resources.
            finish_gestures(&mut tester, &[&gesture1, &gesture2, &gesture3, &gesture4]);
            tester.send(&gesture5, PointerChange::Remove);
        });
    }

    #[test]
    fn tristate_checkbox_overlay_color_resolves_in_pressed_active_inactive_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const SPLASH_RADIUS: f64 = 24.0;
            let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Pressed) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                    }
                    return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                }
                None
            });
            let value = Rc::new(std::cell::Cell::new(Some(false)));
            let host = stateful(Rc::clone(&value), move |checkbox| {
                checkbox
                    .tristate(true)
                    .overlay_color(Rc::clone(&overlay_color))
                    .splash_radius(SPLASH_RADIUS)
            });
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    m3(&app),
                    crate::scaffold::Scaffold::new().body(host).into_widget(),
                )
            };
            tester.pump_widget(root);

            // The checkbox is inactive.
            let mut gesture = press(&mut tester, 1);
            tester.pump_and_settle();
            assert_eq!(value.get(), Some(false));
            assert_paints(
                &tester,
                &[Paints::CircleRadius(
                    INACTIVE_PRESSED_OVERLAY_COLOR,
                    SPLASH_RADIUS,
                )],
            );

            for (pointer, expected_value, expected_color) in [
                // The checkbox is active.
                (2, Some(true), ACTIVE_PRESSED_OVERLAY_COLOR),
                // The checkbox is active in tri-state.
                (3, None, ACTIVE_PRESSED_OVERLAY_COLOR),
                // The checkbox is inactive again.
                (4, Some(false), INACTIVE_PRESSED_OVERLAY_COLOR),
            ] {
                tester.up(&gesture);
                gesture = press(&mut tester, pointer);
                tester.pump_and_settle();
                assert_eq!(value.get(), expected_value);
                assert_paints(
                    &tester,
                    &[Paints::CircleRadius(expected_color, SPLASH_RADIUS)],
                );
            }

            tester.up(&gesture);
        });
    }

    #[test]
    fn do_not_crash_when_widget_disappears_while_pointer_is_down() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_checkbox(&mut tester, m3, |_| {
                Checkbox::new(Some(true), ignore()).into_widget()
            });
            // Put a pointer down on the screen.
            let gesture = press(&mut tester, 1);
            tester.pump(Duration::ZERO);
            // While the pointer is down, the widget disappears.
            pump_checkbox(&mut tester, m3, |_| Container::new().into_widget());
            assert!(tester.find_by_type::<Checkbox>().is_empty());
            // Release pointer after widget disappeared.
            tester.up(&gesture);
        });
    }

    #[test]
    fn checkbox_border_side_side_only_applies_when_unselected_in_m2() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const BORDER_COLOR: Color = Color::new(0xfff44336);
            const ACTIVE_COLOR: Color = Color::new(0xff123456);
            let side = BorderSide {
                width: 4.0,
                color: BORDER_COLOR,
                ..BorderSide::default()
            };
            let frame = |tester: &mut WidgetTester, value: Option<bool>, enabled: bool| {
                pump_checkbox(tester, m2, move |_| {
                    Checkbox::new(value, if enabled { ignore() } else { None })
                        .tristate(value.is_none())
                        .active_color(ACTIVE_COLOR)
                        .side(side)
                        .into_widget()
                });
                tester.pump_and_settle();
            };
            let expect_border = |tester: &WidgetTester| {
                assert_paints(
                    tester,
                    &[Paints::RingAt(
                        BORDER_COLOR,
                        rrect(15.0, 15.0, 33.0, 33.0, 1.0),
                        rrect(19.0, 19.0, 29.0, 29.0, 0.0),
                    )],
                );
            };

            // Checkbox is unselected, so the specified BorderSide appears.
            frame(&mut tester, Some(false), true);
            expect_border(&tester);
            frame(&mut tester, Some(false), false);
            expect_border(&tester);

            // Checkbox is selected/indeterminate, so the specified BorderSide is transparent,
            // which is not recorded.
            for value in [Some(true), None] {
                frame(&mut tester, value, true);
                assert_does_not_paint(&tester, &[Paints::Ring(BORDER_COLOR)]);
                assert_paints(&tester, &[Paints::Rrect(ACTIVE_COLOR)]); // checkbox fill
            }
        });
    }

    fn widget_state_border_side_case(theme: fn(&App) -> ThemeData, radius: f64) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const BORDER_COLOR: Color = Color::new(0xfff44336);
            let side = BorderSide {
                width: 4.0,
                color: BORDER_COLOR,
                ..BorderSide::default()
            };
            for (value, enabled) in [
                (Some(false), true),
                (Some(false), false),
                (Some(true), true),
                (None, true),
            ] {
                pump_checkbox(&mut tester, theme, move |_| {
                    Checkbox::new(value, if enabled { ignore() } else { None })
                        .tristate(value.is_none())
                        .side(WidgetStateBorderSide::resolve_with(move |_| Some(side)))
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_paints(
                    &tester,
                    &[Paints::RingAt(
                        BORDER_COLOR,
                        rrect(15.0, 15.0, 33.0, 33.0, radius),
                        rrect(19.0, 19.0, 29.0, 29.0, 0.0),
                    )],
                );
            }
        });
    }

    #[test]
    fn material2_checkbox_widget_state_border_side_applies_unconditionally() {
        widget_state_border_side_case(m2, 1.0);
    }

    #[test]
    fn material3_checkbox_widget_state_border_side_applies_unconditionally() {
        widget_state_border_side_case(m3, 2.0);
    }

    #[test]
    fn disabled_checkbox_shows_tooltip() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const LONG_PRESS_TOOLTIP: &str = "long press tooltip";
            const TAP_TOOLTIP: &str = "tap tooltip";
            let frame = |tester: &mut WidgetTester, tooltip: crate::tooltip::Tooltip| {
                tester.pump_widget(
                    crate::app::MaterialApp::new()
                        .home(Material::new().child(tooltip.child(Checkbox::new(Some(true), None))))
                        .into_widget(),
                );
            };
            let tooltip_center = |tester: &WidgetTester| {
                let tooltip = tester.find_by_type::<crate::tooltip::Tooltip>()[0];
                tester.global_rect(tester.render_box_of(tooltip)).center()
            };
            frame(
                &mut tester,
                crate::tooltip::Tooltip::new().message(LONG_PRESS_TOOLTIP),
            );

            // Default tooltip shows up after long pressed.
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            let center = tooltip_center(&tester);
            tester.tap(center, 1);
            tester.pump(Duration::from_millis(10));
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            let mut gesture_long_press = WidgetTester::touch(2);
            tester.down(&mut gesture_long_press, center);
            tester.pump(Duration::ZERO);
            tester.pump(waft_gestures::K_LONG_PRESS_TIMEOUT);
            tester.up(&gesture_long_press);
            tester.pump(Duration::ZERO);

            assert_eq!(tester.find_text(LONG_PRESS_TOOLTIP).len(), 1);

            // Tooltip shows up after tapping when set triggerMode to TooltipTriggerMode.tap.
            frame(
                &mut tester,
                crate::tooltip::Tooltip::new()
                    .trigger_mode(waft_widgets::TooltipTriggerMode::Tap)
                    .message(TAP_TOOLTIP),
            );

            tester.pump(Duration::from_secs(24 * 60 * 60));
            tester.pump_and_settle();
            assert!(tester.find_text(TAP_TOOLTIP).is_empty());
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            let center = tooltip_center(&tester);
            tester.tap(center, 3);
            tester.pump(Duration::from_millis(10));
            assert_eq!(tester.find_text(TAP_TOOLTIP).len(), 1);
        });
    }

    #[test]
    fn material3_checkbox_has_default_error_color_when_is_error_is_set_to_true() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            let colors = m3(&tester.cell.borrow()).color_scheme.clone();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(std::cell::Cell::new(Some(true)));
            let frame = |tester: &mut WidgetTester, auto_focus: bool| {
                let host = stateful(Rc::clone(&value), {
                    let node = node.clone();
                    move |checkbox| {
                        checkbox
                            .is_error(true)
                            .autofocus(auto_focus)
                            .focus_node(node.clone())
                    }
                });
                pump_checkbox(tester, m3, |_| host);
                tester.pump_and_settle();
            };

            // Focused
            frame(&mut tester, true);
            assert!(has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[
                    Paints::Circle(colors.error.with_opacity(0.1)),
                    Paints::Rrect(colors.error),
                    Paints::Path(colors.on_error),
                ],
            );

            // Default color
            reset(&mut tester);
            frame(&mut tester, false);
            assert!(!has_primary_focus(&tester, node.handle()));
            assert_paints(
                &tester,
                &[Paints::Rrect(colors.error), Paints::Path(colors.on_error)],
            );

            // Start hovering
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[
                    Paints::Circle(colors.error.with_opacity(0.08)),
                    Paints::Rrect(colors.error),
                ],
            );

            // Start pressing
            let gesture_long_press = press(&mut tester, 2);
            tester.pump(Duration::ZERO);
            assert_paints(
                &tester,
                &[
                    Paints::Circle(colors.error.with_opacity(0.1)),
                    Paints::Rrect(colors.error),
                ],
            );
            tester.up(&gesture_long_press);
            tester.pump(Duration::ZERO);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material3_checkbox_widget_state_border_side_applies_in_error_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            const BORDER_COLOR: Color = Color::new(0xffffeb3b);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(std::cell::Cell::new(Some(false)));
            let frame = |tester: &mut WidgetTester, auto_focus: bool| {
                let host = stateful(Rc::clone(&value), {
                    let node = node.clone();
                    move |checkbox| {
                        checkbox
                            .is_error(true)
                            .side(WidgetStateBorderSide::resolve_with(|states| {
                                if states.contains(&WidgetState::Error) {
                                    return Some(BorderSide {
                                        color: BORDER_COLOR,
                                        width: 4.0,
                                        ..BorderSide::default()
                                    });
                                }
                                Some(BorderSide {
                                    color: Colors::RED.primary(),
                                    width: 2.0,
                                    ..BorderSide::default()
                                })
                            }))
                            .autofocus(auto_focus)
                            .focus_node(node.clone())
                    }
                });
                pump_checkbox(tester, m3, |_| host);
                tester.pump_and_settle();
            };
            let expect_border = |tester: &WidgetTester| {
                assert_paints(
                    tester,
                    &[Paints::RingAt(
                        BORDER_COLOR,
                        rrect(15.0, 15.0, 33.0, 33.0, 2.0),
                        rrect(19.0, 19.0, 29.0, 29.0, 0.0),
                    )],
                );
            };

            frame(&mut tester, true);
            expect_border(&tester);

            // Focused
            frame(&mut tester, true);
            assert!(has_primary_focus(&tester, node.handle()));
            expect_border(&tester);

            // Default color
            reset(&mut tester);
            frame(&mut tester, false);
            assert!(!has_primary_focus(&tester, node.handle()));
            expect_border(&tester);

            // Start hovering
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            expect_border(&tester);

            // Start pressing
            let gesture_long_press = press(&mut tester, 2);
            tester.pump(Duration::ZERO);
            expect_border(&tester);
            tester.up(&gesture_long_press);
            tester.pump(Duration::ZERO);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material3_checkbox_has_correct_default_shape() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_checkbox(&mut tester, m3, |_| {
                Checkbox::new(Some(false), ignore()).into_widget()
            });
            tester.pump_and_settle();

            let expected_shape = m3(&tester.cell.borrow()).checkbox_theme.shape;
            assert!(expected_shape.is_none());
            tester.widget(checkbox_element(&tester), |checkbox: &Checkbox| {
                assert!(checkbox.shape.is_none());
            });
            assert_paints(
                &tester,
                &[Paints::RingShape(
                    rrect(15.0, 15.0, 33.0, 33.0, 2.0),
                    rrect(17.0, 17.0, 31.0, 31.0, 0.0),
                )],
            );
        });
    }

    #[test]
    fn checkbox_adaptive_shows_the_correct_platform_widget() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = |tester: &mut WidgetTester, platform: TargetPlatform| {
                pump_checkbox(
                    tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    |_| Checkbox::adaptive(Some(false), ignore()).into_widget(),
                );
                tester.pump_and_settle();
            };

            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                frame(&mut tester, platform);
                assert_eq!(
                    tester.find_by_type::<CupertinoCheckbox>().len(),
                    1,
                    "{platform:?}"
                );
            }

            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                frame(&mut tester, platform);
                assert!(
                    tester.find_by_type::<CupertinoCheckbox>().is_empty(),
                    "{platform:?}"
                );
            }
        });
    }

    #[test]
    fn checkbox_adaptive_respects_checkbox_mouse_cursor_on_ios_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let frame = |tester: &mut WidgetTester, mouse_cursor: Option<MouseCursorRef>| {
                    let root = {
                        let app = tester.cell.borrow();
                        let mut checkbox = Checkbox::adaptive(Some(true), ignore());
                        checkbox.mouse_cursor = mouse_cursor;
                        crate::app::MaterialApp::new()
                            .theme(m3(&app))
                            .home(Material::new().child(checkbox))
                            .into_widget()
                    };
                    tester.pump_widget(root);
                };

                frame(&mut tester, None);
                let cupertino = tester.find_by_type::<CupertinoCheckbox>()[0];
                let center = tester.global_rect(tester.render_box_of(cupertino)).center();
                let mut gesture = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut gesture, center);
                tester.pump(Duration::ZERO);
                tester.move_to(&mut gesture, center, false);
                let expected: MouseCursorRef = if K_IS_WEB {
                    SystemMouseCursors::CLICK.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                };
                assert_eq!(
                    Some(tester.active_cursor(1)),
                    Some(expected),
                    "{platform:?}"
                );

                // Test mouse cursor can be configured.
                frame(&mut tester, Some(SystemMouseCursors::CLICK.into()));
                assert_eq!(
                    tester.active_cursor(1),
                    SystemMouseCursors::CLICK.into(),
                    "{platform:?}"
                );

                // Test Checkbox.adaptive can resolve a WidgetStateMouseCursor.
                frame(&mut tester, Some(selected_grab_mouse_cursor()));
                assert_eq!(
                    tester.active_cursor(1),
                    SystemMouseCursors::GRAB.into(),
                    "{platform:?}"
                );

                tester.send(&gesture, PointerChange::Remove);
            });
        }
    }

    fn fill_color_when_unchecked_case(
        theme: fn(&App) -> ThemeData,
        enabled_border: fn(&ThemeData) -> Color,
        disabled_border: fn(&ThemeData) -> Color,
    ) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let data = theme(&tester.cell.borrow());
            const ACTIVE_BACKGROUND_COLOR: Color = Color::new(0xff123456);
            const INACTIVE_BACKGROUND_COLOR: Color = Color::new(0xff654321);
            let frame = |tester: &mut WidgetTester, enabled: bool| {
                pump_checkbox(tester, theme, move |_| {
                    Checkbox::new(Some(false), if enabled { ignore() } else { None })
                        .fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                if states.contains(&WidgetState::Selected) {
                                    return Some(ACTIVE_BACKGROUND_COLOR);
                                }
                                Some(INACTIVE_BACKGROUND_COLOR)
                            },
                        ))
                        .into_widget()
                });
                tester.pump_and_settle();
            };

            // Checkbox is unselected, so the default BorderSide appears and fillColor is
            // checkbox's background color.
            frame(&mut tester, true);
            assert_paints(&tester, &[Paints::Ring(enabled_border(&data))]);
            assert_paints(&tester, &[Paints::Rrect(INACTIVE_BACKGROUND_COLOR)]);

            frame(&mut tester, false);
            assert_paints(&tester, &[Paints::Ring(disabled_border(&data))]);
            assert_paints(&tester, &[Paints::Rrect(INACTIVE_BACKGROUND_COLOR)]);
        });
    }

    #[test]
    fn material2_checkbox_respects_fill_color_when_it_is_unchecked() {
        fill_color_when_unchecked_case(
            m2,
            |theme| theme.unselected_widget_color,
            |theme| theme.disabled_color,
        );
    }

    #[test]
    fn material3_checkbox_respects_fill_color_when_it_is_unchecked() {
        fill_color_when_unchecked_case(
            m3,
            |theme| theme.color_scheme.on_surface_variant(),
            |theme| theme.color_scheme.on_surface.with_opacity(0.38),
        );
    }

    #[test]
    fn checkbox_renders_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(
                crate::app::MaterialApp::new()
                    .home(Center::new().child(waft_widgets::SizedBox::shrink().child(
                        crate::scaffold::Scaffold::new().body(Checkbox::new(Some(true), None)),
                    )))
                    .into_widget(),
            );
        });
    }

    /// Dart's `_padFrame`.
    pub(crate) fn pad_frame(
        tester: &mut WidgetTester,
        value: Option<bool>,
        mark_insets: Option<EdgeInsets>,
        tristate: bool,
    ) {
        tester.pump_widget(
            crate::app::MaterialApp::new()
                .home(Material::new().child(Center::new().child({
                    let mut checkbox = Checkbox::new(value, ignore()).tristate(tristate);
                    checkbox.mark_insets = mark_insets;
                    checkbox
                })))
                .into_widget(),
        );
    }

    /// Dart's `_paintedCheckBounds`: the bounds of the check mark path.
    fn painted_check_bounds(tester: &WidgetTester) -> Rect {
        drawn(tester)
            .into_iter()
            .find_map(|drawn| match drawn {
                Drawn::Stroke(_, _, bounds) => Some(bounds),
                _ => None,
            })
            .expect("No check mark was painted.")
    }

    /// Dart's `_paintedDashLength`: the length of the indeterminate dash.
    fn painted_dash_length(tester: &WidgetTester) -> f64 {
        drawn(tester)
            .into_iter()
            .find_map(|drawn| match drawn {
                Drawn::Stroke(_, _, bounds) if bounds.height() == 0.0 => Some(bounds.width()),
                _ => None,
            })
            .expect("No indeterminate dash was painted.")
    }

    #[test]
    fn checkbox_mark_insets_defaults_to_null() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), None, false);
            tester.widget(checkbox_element(&tester), |checkbox: &Checkbox| {
                assert_eq!(checkbox.mark_insets, None);
            });
        });
    }

    #[test]
    fn checkbox_with_no_mark_insets_paints_the_check_at_full_stroke_width() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), None, false);
            tester.pump_and_settle();
            // default: scale == 1.0, so strokeWidth stays at _kStrokeWidth (2.0)
            assert_paints(&tester, &[Paints::StrokeWidth(2.0)]);
        });
    }

    #[test]
    fn checkbox_mark_insets_shrinks_the_check_mark_and_its_stroke() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), Some(EdgeInsets::all(4.5)), false);
            tester.pump_and_settle();
            // inner = 18 - 9 = 9, scale = 9/18 = 0.5, strokeWidth = 2.0 * 0.5 = 1.0
            assert_paints(&tester, &[Paints::StrokeWidth(1.0)]);
        });
    }

    #[test]
    fn checkbox_mark_insets_shrinks_the_indeterminate_dash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, None, Some(EdgeInsets::all(4.5)), true);
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::LineWidth(1.0)]);
        });
    }

    #[test]
    fn checkbox_mark_insets_larger_than_the_box_collapses_the_mark_without_crashing() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), Some(EdgeInsets::all(9.0)), false);
            tester.pump_and_settle();
            // mark collapsed away
            assert!(
                !drawn(&tester)
                    .iter()
                    .any(|drawn| matches!(drawn, Drawn::Stroke(..))),
                "{:?}",
                drawn(&tester)
            );
        });
    }

    #[test]
    fn checkbox_mark_insets_keeps_the_check_mark_proportional_with_asymmetric_insets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), None, false);
            tester.pump_and_settle();
            let uninset = painted_check_bounds(&tester);
            let aspect = uninset.width() / uninset.height();

            // Each of these leaves a different amount of room on each axis. The mark
            // shrinks to the smaller of the two, so its shape never changes.
            for mark_insets in [
                EdgeInsets::only(9.0, 0.0, 0.0, 0.0),
                EdgeInsets::only(0.0, 9.0, 0.0, 0.0),
                EdgeInsets::only(0.0, 0.0, 4.5, 0.0),
                EdgeInsets::symmetric(0.0, 6.0),
            ] {
                pad_frame(&mut tester, Some(true), Some(mark_insets), false);
                tester.pump_and_settle();
                let inset = painted_check_bounds(&tester);
                assert!(
                    (inset.width() / inset.height() - aspect).abs() <= 0.001,
                    "check mark was distorted by {mark_insets:?}"
                );
                assert!(inset.width() <= uninset.width());
                assert!(inset.height() <= uninset.height());
            }
        });
    }

    #[test]
    fn checkbox_mark_insets_centers_a_half_size_mark_in_the_right_half_of_the_box() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, Some(true), None, false);
            tester.pump_and_settle();
            let uninset = painted_check_bounds(&tester);
            // The 18x18 box is centered in the checkbox's render box.
            let box_origin = checkbox_size(&tester).center(Offset::ZERO) - Offset::new(9.0, 9.0);

            pad_frame(
                &mut tester,
                Some(true),
                Some(EdgeInsets::only(9.0, 0.0, 0.0, 0.0)),
                false,
            );
            tester.pump_and_settle();
            let inset = painted_check_bounds(&tester);

            // The remaining space is 9x18, so the mark is drawn in a 9x9 square at
            // (9, 4.5) within the box: half size, vertically centered.
            let to_half_square =
                |p: Offset| box_origin + Offset::new(9.0, 4.5) + (p - box_origin) / 2.0;
            let expected = Rect::from_points(
                to_half_square(uninset.top_left()),
                to_half_square(uninset.bottom_right()),
            );
            assert!((inset.left - expected.left).abs() <= 0.001);
            assert!((inset.top - expected.top).abs() <= 0.001);
            assert!((inset.width() - uninset.width() / 2.0).abs() <= 0.001);
            assert!((inset.height() - uninset.height() / 2.0).abs() <= 0.001);
        });
    }

    #[test]
    fn checkbox_mark_insets_shrinks_the_indeterminate_dash_by_the_smaller_inset() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pad_frame(&mut tester, None, None, true);
            tester.pump_and_settle();
            let uninset = painted_dash_length(&tester);

            // Only the horizontal axis is inset, but the dash still scales by half,
            // matching the check mark rather than stretching to the inner width.
            pad_frame(
                &mut tester,
                None,
                Some(EdgeInsets::only(9.0, 0.0, 0.0, 0.0)),
                true,
            );
            tester.pump_and_settle();
            assert!((painted_dash_length(&tester) - uninset / 2.0).abs() <= 0.001);
        });
    }

    /// `MaterialApp(theme: ThemeData(checkboxTheme: CheckboxThemeData(markInsets:)), home:
    /// Material(child: Center(child: checkbox)))`.
    fn pump_with_theme_mark_insets(
        tester: &mut WidgetTester,
        theme_mark_insets: EdgeInsets,
        checkbox: Checkbox,
    ) {
        let root = {
            let app = tester.cell.borrow();
            crate::app::MaterialApp::new()
                .theme(
                    ThemeData::builder()
                        .checkbox_theme(CheckboxThemeData::new().mark_insets(theme_mark_insets))
                        .build(&app),
                )
                .home(Material::new().child(Center::new().child(checkbox)))
                .into_widget()
        };
        tester.pump_widget(root);
    }

    #[test]
    fn checkbox_mark_insets_falls_back_to_checkbox_theme_data_mark_insets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_theme_mark_insets(
                &mut tester,
                EdgeInsets::all(4.5),
                Checkbox::new(Some(true), ignore()),
            );
            tester.pump_and_settle();
            // theme markInsets applied
            assert_paints(&tester, &[Paints::StrokeWidth(1.0)]);
        });
    }

    #[test]
    fn checkbox_mark_insets_overrides_checkbox_theme_data_mark_insets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_theme_mark_insets(
                &mut tester,
                EdgeInsets::ZERO,
                Checkbox::new(Some(true), ignore()).mark_insets(EdgeInsets::all(4.5)),
            );
            tester.pump_and_settle();
            // widget value wins
            assert_paints(&tester, &[Paints::StrokeWidth(1.0)]);
        });
    }
}

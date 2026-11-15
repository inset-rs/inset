//! Flutter counterpart: material_ui's `radio.dart` (with `generated/radio_defaults_m3.g.dart`).
//!
//! [`Radio`] builds a [`RawRadio`], whose builder hands `_RadioPaint` the radio's state as an
//! [`AnyToggleableState`]; the paint state keeps its arena [`ToggleablePainter`] for its
//! lifetime, as Dart's `_painter` field is. The deprecated `groupValue` and `onChanged` reach
//! the radio through `_RadioRegistry`, an arena object the state creates on first use.
//! [`Radio::adaptive`] builds a [`CupertinoRadio`] on iOS and macOS.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::Animation;
use waft_cupertino::CupertinoRadio;
use waft_embedder::{Canvas, Color, Offset, Paint, PaintStyle, Size, TargetPlatform};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, ValueChanged};
use waft_painting::{AnyBorderSide, BorderSide, CircleBorder, ShapeBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{
    AnyFocusNode, AnyRadioClient, AnyRadioGroupRegistry, AnyToggleableState, BuildContext,
    CustomPaint, FocusNode, FocusNodeLeaf, IntoWidget, KeyRef, RadioBuilder, RadioGroup,
    RadioGroupRegistry, RawRadio, State, StateData, StatefulWidget, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, WidgetRef, WidgetState, WidgetStateBorderSide,
    WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::Colors;
use crate::constants::{
    K_MIN_INTERACTIVE_DIMENSION, K_RADIAL_REACTION_ALPHA, K_RADIAL_REACTION_RADIUS,
};
use crate::debug::debug_check_has_material;
use crate::generated::radio_defaults_m3::radio_defaults_m3;
use crate::radio_theme::{RadioTheme, RadioThemeData};
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RadioType {
    Material,
    Adaptive,
}

const K_OUTER_RADIUS: f64 = 8.0;
const K_INNER_RADIUS: f64 = 4.5;

/// A Material Design radio button.
///
/// This widget builds a [`RawRadio`] with a material UI.
///
/// Used to select between a number of mutually exclusive values. When one radio
/// button in a group is selected, the other radio buttons in the group cease to
/// be selected. The values are of type `T`, the type parameter of the [`Radio`]
/// class. Enums are commonly used for this purpose.
///
/// This widget typically has a [`RadioGroup`] ancestor, which takes in a
/// [`RadioGroup::group_value`], and the [`Radio`] under it with matching [`value`](Self::value)
/// will be selected.
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// See also:
///
///  * `RadioListTile`, which combines this widget with a `ListTile` so that
///    you can give the radio button a label.
///  * `Slider`, for selecting a value in a range.
///  * `Checkbox` and `Switch`, for toggling a particular value on or off.
///  * <https://material.io/design/components/selection-controls.html#radio-buttons>
#[derive(Clone)]
pub struct Radio<T: Clone + PartialEq + 'static> {
    pub key: Option<KeyRef>,
    /// The value represented by this radio button.
    pub value: T,
    /// The currently selected value for a group of radio buttons.
    ///
    /// This radio button is considered selected if its [`value`](Self::value) matches the
    /// [`group_value`](Self::group_value).
    ///
    /// This is deprecated, use [`RadioGroup`] to manage group value instead.
    pub group_value: Option<T>,
    /// Called when the user selects this radio button.
    ///
    /// The radio button passes [`value`](Self::value) as a parameter to this callback. The
    /// radio button does not actually change state until the parent widget rebuilds the radio
    /// button with the new [`group_value`](Self::group_value).
    ///
    /// If null, the radio button will be displayed as disabled.
    ///
    /// The provided callback will not be invoked if this radio button is already selected and
    /// [`toggleable`](Self::toggleable) is not set to true.
    ///
    /// If the [`toggleable`](Self::toggleable) is set to true, tapping a already selected radio
    /// will invoke this callback with `null` as value.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of the
    /// parent `StatefulWidget` using the `State::set_state` method, so that the parent gets
    /// rebuilt.
    ///
    /// This is deprecated, use [`RadioGroup`] to handle value change instead.
    pub on_changed: Option<ValueChanged<Option<T>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, the value of [`RadioThemeData::mouse_cursor`] is used. If that is
    /// also null, [`WidgetStateMouseCursor::adaptive_clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// Set to true if this radio button is allowed to be returned to an indeterminate state by
    /// selecting it again when selected.
    ///
    /// To indicate returning to an indeterminate state, [`RadioGroup::on_changed`] of the
    /// [`RadioGroup`] above the widget tree will be called with null.
    ///
    /// If true, [`RadioGroup::on_changed`] is called with [`value`](Self::value) when selected
    /// while [`RadioGroup::group_value`] != [`value`](Self::value), and with null when selected
    /// again while [`RadioGroup::group_value`] == [`value`](Self::value).
    ///
    /// If false, [`RadioGroup::on_changed`] will be called with [`value`](Self::value) when it
    /// is selected while [`RadioGroup::group_value`] != [`value`](Self::value), and only by
    /// selecting another radio button in the group (i.e. changing the value of
    /// [`RadioGroup::group_value`]) can this radio button be unselected.
    ///
    /// The default is false.
    pub toggleable: bool,
    /// The color to use when this radio button is selected.
    ///
    /// Defaults to `ColorScheme::secondary`.
    ///
    /// If [`fill_color`](Self::fill_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, it will be used instead of this color.
    pub active_color: Option<Color>,
    /// The color that fills the radio button, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) is used in the selected
    /// state. If that is also null, then the value of [`RadioThemeData::fill_color`] is used.
    /// If that is also null and `ThemeData::use_material3` is false, then
    /// `ThemeData::disabled_color` is used in the disabled state, `ColorScheme::secondary`
    /// is used in the selected state, and `ThemeData::unselected_widget_color` is used in the
    /// default state; if `ThemeData::use_material3` is true, then `ColorScheme::on_surface`
    /// is used in the disabled state, `ColorScheme::primary` is used in the
    /// selected state and `ColorScheme::on_surface_variant` is used in the default state.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Configures the minimum size of the tap target.
    ///
    /// If null, then the value of [`RadioThemeData::material_tap_target_size`] is used.
    /// If that is also null, then the value of `ThemeData::material_tap_target_size`
    /// is used.
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// Defines how compact the radio's layout will be.
    ///
    /// If null, then the value of [`RadioThemeData::visual_density`] is used. If that
    /// is also null, then the value of `ThemeData::visual_density` is used.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the [`visual_density`](Self::visual_density)
    ///    for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,
    /// The color for the radio's `Material` when it has the input focus.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Focused`] state, it will be used instead.
    ///
    /// If null, then the value of [`RadioThemeData::overlay_color`] is used in the
    /// focused state. If that is also null, then the value of
    /// `ThemeData::focus_color` is used.
    pub focus_color: Option<Color>,
    /// The color for the radio's `Material` when a pointer is hovering over it.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Hovered`] state, it will be used instead.
    ///
    /// If null, then the value of [`RadioThemeData::overlay_color`] is used in the
    /// hovered state. If that is also null, then the value of
    /// `ThemeData::hover_color` is used.
    pub hover_color: Option<Color>,
    /// The color for the radio's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) with alpha
    /// `K_RADIAL_REACTION_ALPHA`, [`focus_color`](Self::focus_color) and
    /// [`hover_color`](Self::hover_color) is used in the pressed, focused and hovered state. If
    /// that is also null, the value of [`RadioThemeData::overlay_color`] is used. If that is
    /// also null, then in Material 2, the value of `ColorScheme::secondary` with alpha
    /// `K_RADIAL_REACTION_ALPHA`, `ThemeData::focus_color` and `ThemeData::hover_color`
    /// is used in the pressed, focused and hovered state. In Material3, the default
    /// values are:
    ///   * selected
    ///     * pressed - Theme.colorScheme.onSurface(0.1)
    ///     * hovered - Theme.colorScheme.primary(0.08)
    ///     * focused - Theme.colorScheme.primary(0.1)
    ///   * pressed - Theme.colorScheme.primary(0.1)
    ///   * hovered - Theme.colorScheme.onSurface(0.08)
    ///   * focused - Theme.colorScheme.onSurface(0.1)
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of [`RadioThemeData::splash_radius`] is used. If that
    /// is also null, then `K_RADIAL_REACTION_RADIUS` is used.
    pub splash_radius: Option<f64>,
    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed by
    /// this widget. The widget will be focusable even if a `focus_node` is not supplied. If
    /// supplied, the given `focus_node` will be _hosted_ by this widget, but not owned.
    pub focus_node: Option<AnyFocusNode>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// Controls whether the checkmark style is used in an iOS-style radio.
    ///
    /// Only usable under the [`Radio::adaptive`] constructor. If set to true, on
    /// Apple platforms the radio button will appear as an iOS styled checkmark.
    /// Controls the [`CupertinoRadio`] through [`CupertinoRadio::use_checkmark_style`].
    ///
    /// Defaults to false.
    pub use_cupertino_checkmark_style: bool,
    /// The registry this radio registers to.
    ///
    /// Unless provided, the [`BuildContext`] will be used to look up the ancestor
    /// [`RadioGroupRegistry`].
    pub group_registry: Option<AnyRadioGroupRegistry<T>>,
    radio_type: RadioType,
    /// Whether this widget is interactive.
    ///
    /// If not provided, this widget will be interactable if one of the following is true:
    ///
    /// * A [`on_changed`](Self::on_changed) is provided.
    /// * Having a [`RadioGroup`] with the same type T above this widget.
    /// * A [`group_registry`](Self::group_registry) is provided.
    ///
    /// If this is set to true, one of the above condition must also be true. Otherwise, an
    /// assertion error is thrown.
    pub enabled: Option<bool>,
    /// The color of the background of the radio button, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the ambient [`RadioThemeData::background_color`] is used.
    /// If that is also null the default value is transparent in all states.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The side for the circular border of the radio button, in all
    /// [`WidgetState`]s.
    ///
    /// This property can be a [`BorderSide`] or a [`WidgetStateBorderSide`] to leverage
    /// widget state resolution.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the ambient [`RadioThemeData::side`] is used. If that is
    /// also null, the default value is a border using the fill color.
    pub side: Option<AnyBorderSide>,
    /// The radius of the inner circle of the radio button, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the ambient [`RadioThemeData::inner_radius`] is used.
    /// If that is also null, the default value is `4.5` in all states.
    pub inner_radius: Option<WidgetStatePropertyRef<Option<f64>>>,
}

impl<T: Clone + PartialEq + 'static> Radio<T> {
    /// Creates a Material Design radio button.
    ///
    /// This widget typically has a [`RadioGroup`] ancestor, which takes in a
    /// [`RadioGroup::group_value`], and the [`Radio`] under it with matching
    /// [`value`](Self::value) will be selected.
    ///
    /// The [`value`](Self::value) is required.
    pub fn new(value: T) -> Radio<T> {
        Radio::with_type(value, RadioType::Material)
    }

    /// Creates an adaptive [`Radio`] based on whether the target platform is iOS
    /// or macOS, following Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// On iOS and macOS, this constructor creates a [`CupertinoRadio`], which has
    /// matching functionality and presentation as Material checkboxes, and are the
    /// graphics expected on iOS. On other platforms, this creates a Material
    /// design [`Radio`].
    ///
    /// If a [`CupertinoRadio`] is created, the following parameters are ignored:
    /// [`mouse_cursor`](Self::mouse_cursor), [`fill_color`](Self::fill_color),
    /// [`hover_color`](Self::hover_color), [`overlay_color`](Self::overlay_color),
    /// [`splash_radius`](Self::splash_radius),
    /// [`material_tap_target_size`](Self::material_tap_target_size),
    /// [`visual_density`](Self::visual_density).
    ///
    /// [`use_cupertino_checkmark_style`](Self::use_cupertino_checkmark_style) is used only if a
    /// [`CupertinoRadio`] is created.
    ///
    /// The target platform is based on the current `Theme`: `ThemeData::platform`.
    pub fn adaptive(value: T) -> Radio<T> {
        Radio::with_type(value, RadioType::Adaptive)
    }

    fn with_type(value: T, radio_type: RadioType) -> Radio<T> {
        Radio {
            key: None,
            value,
            group_value: None,
            on_changed: None,
            mouse_cursor: None,
            toggleable: false,
            active_color: None,
            fill_color: None,
            material_tap_target_size: None,
            visual_density: None,
            focus_color: None,
            hover_color: None,
            overlay_color: None,
            splash_radius: None,
            focus_node: None,
            autofocus: false,
            use_cupertino_checkmark_style: false,
            group_registry: None,
            radio_type,
            enabled: None,
            background_color: None,
            side: None,
            inner_radius: None,
        }
    }

    /// Dart `Radio(key:)`.
    pub fn key(mut self, key: KeyRef) -> Radio<T> {
        self.key = Some(key);
        self
    }

    /// Dart `Radio(groupValue:)`.
    #[deprecated(
        note = "Use a RadioGroup ancestor to manage group value instead. This feature was \
                deprecated after v3.32.0-0.0.pre."
    )]
    pub fn group_value(mut self, group_value: T) -> Radio<T> {
        self.group_value = Some(group_value);
        self
    }

    /// Dart `Radio(onChanged:)`.
    #[deprecated(
        note = "Use RadioGroup to handle value change instead. This feature was deprecated \
                after v3.32.0-0.0.pre."
    )]
    pub fn on_changed(mut self, on_changed: ValueChanged<Option<T>>) -> Radio<T> {
        self.on_changed = Some(on_changed);
        self
    }

    /// Dart `Radio(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> Radio<T> {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `Radio(toggleable:)`.
    pub fn toggleable(mut self, toggleable: bool) -> Radio<T> {
        self.toggleable = toggleable;
        self
    }

    /// Dart `Radio(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> Radio<T> {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `Radio(fillColor:)`.
    pub fn fill_color(mut self, fill_color: WidgetStatePropertyRef<Option<Color>>) -> Radio<T> {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `Radio(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> Radio<T> {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `Radio(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> Radio<T> {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `Radio(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> Radio<T> {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `Radio(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> Radio<T> {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `Radio(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> Radio<T> {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `Radio(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> Radio<T> {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `Radio(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> Radio<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Radio(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Radio<T> {
        self.autofocus = autofocus;
        self
    }

    /// Dart `Radio.adaptive(useCupertinoCheckmarkStyle:)`; only a radio made by
    /// [`Radio::adaptive`] reads it.
    pub fn use_cupertino_checkmark_style(
        mut self,
        use_cupertino_checkmark_style: bool,
    ) -> Radio<T> {
        self.use_cupertino_checkmark_style = use_cupertino_checkmark_style;
        self
    }

    /// Dart `Radio(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> Radio<T> {
        self.enabled = Some(enabled);
        self
    }

    /// Dart `Radio(groupRegistry:)`.
    pub fn group_registry(mut self, group_registry: AnyRadioGroupRegistry<T>) -> Radio<T> {
        self.group_registry = Some(group_registry);
        self
    }

    /// Dart `Radio(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> Radio<T> {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `Radio(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> Radio<T> {
        self.side = Some(side.into());
        self
    }

    /// Dart `Radio(innerRadius:)`.
    pub fn inner_radius(mut self, inner_radius: WidgetStatePropertyRef<Option<f64>>) -> Radio<T> {
        self.inner_radius = Some(inner_radius);
        self
    }
}

impl<T: Clone + PartialEq + 'static> Debug for Radio<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Radio")
            .field("radio_type", &self.radio_type)
            .field("toggleable", &self.toggleable)
            .field("enabled", &self.enabled)
            .field("group_registry", &self.group_registry)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for Radio<T> {
    type State = RadioState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RadioState<T> {
        RadioState {
            state: StateData::new(),
            internal_focus_node: None,
            internal_radio_registry: None,
        }
    }
}

/// Dart's `_RadioState`; public because [`Radio`]'s `StatefulWidget::State` names it.
pub struct RadioState<T: Clone + PartialEq + 'static> {
    state: StateData<Radio<T>>,
    internal_focus_node: Option<AnyFocusNode>,
    internal_radio_registry: Option<Handle<RadioRegistry<T>>>,
}

impl<T: Clone + PartialEq + 'static> RadioState<T> {
    fn focus_node(self: Handle<Self>, app: &mut App) -> AnyFocusNode {
        if let Some(focus_node) = self.widget(app).focus_node {
            return focus_node;
        }
        if let Some(focus_node) = app.get(self).internal_focus_node {
            return focus_node;
        }
        let focus_node = FocusNode::new(app).as_node();
        app.get_mut(self).internal_focus_node = Some(focus_node);
        focus_node
    }

    fn enabled(self: Handle<Self>, app: &mut App) -> bool {
        let widget = self.widget(app);
        if let Some(enabled) = widget.enabled {
            return enabled;
        }
        if widget.on_changed.is_some() || widget.group_registry.is_some() {
            return true;
        }
        let context = self.context(app);
        RadioGroup::<T>::maybe_of(app, context).is_some()
    }

    fn effective_registry(self: Handle<Self>, app: &mut App) -> AnyRadioGroupRegistry<T> {
        if let Some(group_registry) = self.widget(app).group_registry {
            return group_registry;
        }

        let context = self.context(app);
        if let Some(inherited_registry) = RadioGroup::<T>::maybe_of(app, context) {
            return inherited_registry;
        }

        // Handles deprecated API.
        let registry = match app.get(self).internal_radio_registry {
            Some(registry) => registry,
            None => {
                let registry = app.create(RadioRegistry { state: self });
                app.get_mut(self).internal_radio_registry = Some(registry);
                registry
            }
        };
        registry.as_radio_group_registry()
    }

    /// Dart's `CupertinoRadio<T>(...)` that [`Radio::adaptive`] builds on iOS and macOS.
    #[allow(deprecated)]
    fn build_cupertino_radio(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app);
        let mut radio = CupertinoRadio::new(widget.value.clone())
            .toggleable(widget.toggleable)
            .autofocus(widget.autofocus)
            .use_checkmark_style(widget.use_cupertino_checkmark_style);
        if let Some(group_value) = widget.group_value.clone() {
            radio = radio.group_value(group_value);
        }
        if let Some(on_changed) = widget.on_changed.clone() {
            radio = radio.on_changed(on_changed);
        }
        if let Some(mouse_cursor) = widget.mouse_cursor.clone() {
            radio = radio.mouse_cursor(mouse_cursor);
        }
        if let Some(active_color) = widget.active_color {
            radio = radio.active_color(active_color);
        }
        if let Some(focus_color) = widget.focus_color {
            radio = radio.focus_color(focus_color);
        }
        let focus_node = self.focus_node(app);
        let group_registry = self.effective_registry(app);
        let enabled = self.enabled(app);
        radio
            .focus_node(focus_node)
            .group_registry(group_registry)
            .enabled(enabled)
            .into_widget()
    }

    /// Dart's `effectiveMouseCursor`, built from the widget's cursor and the theme's.
    fn effective_mouse_cursor(
        mouse_cursor: Option<MouseCursorRef>,
        radio_theme: &RadioThemeData,
    ) -> WidgetStatePropertyRef<MouseCursorRef> {
        let theme_mouse_cursor = radio_theme.mouse_cursor.clone();
        let adaptive_clickable: MouseCursorRef =
            WidgetStateMouseCursor::adaptive_clickable().into();
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(move |states: &WidgetStates| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                .or_else(|| {
                    theme_mouse_cursor
                        .as_ref()
                        .and_then(|mouse_cursor| mouse_cursor.resolve(states))
                })
                .unwrap_or_else(|| {
                    <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                        &adaptive_clickable,
                        states,
                    )
                })
        })
    }

    /// The `builder` Dart passes `RawRadio`: it reads the state's widget when the raw radio
    /// builds, as the Dart closure does.
    fn radio_builder(self: Handle<Self>) -> RadioBuilder {
        Rc::new(move |app: &mut App, _context, state: AnyToggleableState| {
            let widget = self.widget(app);
            RadioPaint {
                toggleable_state: state,
                active_color: widget.active_color,
                fill_color: widget.fill_color.clone(),
                hover_color: widget.hover_color,
                focus_color: widget.focus_color,
                overlay_color: widget.overlay_color.clone(),
                splash_radius: widget.splash_radius,
                visual_density: widget.visual_density,
                material_tap_target_size: widget.material_tap_target_size,
                background_color: widget.background_color.clone(),
                side: widget.side.clone(),
                inner_radius: widget.inner_radius.clone(),
            }
            .into_widget()
        })
    }
}

impl<T: Clone + PartialEq + 'static> State for RadioState<T> {
    type Widget = Radio<T>;
    waft_widgets::state_accessors!();

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(focus_node) = app.get(self).internal_focus_node {
            focus_node.dispose(app);
        }
        // The raw radio below has unregistered already: children dispose first.
        if let Some(registry) = app.get_mut(self).internal_radio_registry.take() {
            app.destroy(registry);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            !self.widget(app).enabled.unwrap_or(false)
                || self.widget(app).on_changed.is_some()
                || self.widget(app).group_registry.is_some()
                || RadioGroup::<T>::maybe_of(app, context).is_some(),
            "Radio is enabled but has no Radio.onChange or registry above"
        );
        debug_assert!(debug_check_has_material(app, context));
        match self.widget(app).radio_type {
            RadioType::Material => {}

            RadioType::Adaptive => {
                let theme = Theme::of(app, context);
                match theme.platform {
                    TargetPlatform::Android
                    | TargetPlatform::Fuchsia
                    | TargetPlatform::Linux
                    | TargetPlatform::Windows => {}
                    TargetPlatform::IOS | TargetPlatform::MacOS => {
                        return self.build_cupertino_radio(app);
                    }
                }
            }
        }

        let radio_theme = RadioTheme::of(app, context);
        let effective_mouse_cursor =
            Self::effective_mouse_cursor(self.widget(app).mouse_cursor.clone(), &radio_theme);
        let value = self.widget(app).value.clone();
        let toggleable = self.widget(app).toggleable;
        let focus_node = self.focus_node(app);
        let autofocus = self.widget(app).autofocus;
        let group_registry = self.effective_registry(app);
        let enabled = self.enabled(app);
        RawRadio::new(
            value,
            effective_mouse_cursor,
            toggleable,
            focus_node,
            autofocus,
            Some(group_registry),
            enabled,
            self.radio_builder(),
        )
        .into_widget()
    }
}

/// A registry for deprecated API.
// TODO(chunhtai): Remove this once deprecated API is removed.
struct RadioRegistry<T: Clone + PartialEq + 'static> {
    state: Handle<RadioState<T>>,
}

impl<T: Clone + PartialEq + 'static> RadioGroupRegistry<T> for RadioRegistry<T> {
    fn group_value(self: Handle<Self>, app: &App) -> Option<T> {
        let state = app.get(self).state;
        state.widget(app).group_value.clone()
    }

    fn register_client(self: Handle<Self>, _app: &mut App, _radio: AnyRadioClient<T>) {}

    fn unregister_client(self: Handle<Self>, _app: &mut App, _radio: AnyRadioClient<T>) {}

    fn on_changed(self: Handle<Self>, app: &mut App, value: Option<T>) {
        let state = app.get(self).state;
        let on_changed = Rc::clone(
            state
                .widget(app)
                .on_changed
                .as_ref()
                .expect("a registry of a radio with onChanged"),
        );
        on_changed(app, value);
    }
}

/// Dart's `_RadioPaint`.
struct RadioPaint {
    toggleable_state: AnyToggleableState,
    active_color: Option<Color>,
    fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    hover_color: Option<Color>,
    focus_color: Option<Color>,
    overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    splash_radius: Option<f64>,
    visual_density: Option<VisualDensity>,
    material_tap_target_size: Option<MaterialTapTargetSize>,
    background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    side: Option<AnyBorderSide>,
    inner_radius: Option<WidgetStatePropertyRef<Option<f64>>>,
}

impl Debug for RadioPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioPaint")
            .field("toggleable_state", &self.toggleable_state)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RadioPaint {
    type State = RadioPaintState;

    fn create_state(&self) -> RadioPaintState {
        RadioPaintState {
            state: StateData::new(),
            painter: None,
        }
    }
}

/// Dart's `_RadioPaintState`.
struct RadioPaintState {
    state: StateData<RadioPaint>,
    /// Dart's `final _RadioPainter _painter = _RadioPainter()`, created in `init_state` because
    /// a painter is an arena object and a state is created without the `App`.
    painter: Option<Handle<RadioPainter>>,
}

impl RadioPaintState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<RadioPainter> {
        app.get(self).painter.expect("created in init_state")
    }

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

    /// Dart's `_resolveSide`: Dart's `side is WidgetStateProperty` asks whether the side is a
    /// [`WidgetStateBorderSide`].
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
}

/// Dart's `property?.resolve(states)` for a nullable property of a nullable value.
fn resolve<T>(
    property: &Option<WidgetStatePropertyRef<Option<T>>>,
    states: &WidgetStates,
) -> Option<T> {
    property
        .as_ref()
        .and_then(|property| property.resolve(states))
}

impl State for RadioPaintState {
    type Widget = RadioPaint;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let painter = RadioPainter::new(app);
        app.get_mut(self).painter = Some(painter);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let painter = self.painter(app);
        ToggleablePainter::dispose(painter, app);
        app.get_mut(self).painter = None;
        app.destroy(painter);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let radio_theme = RadioTheme::of(app, context);
        let defaults = if Theme::of(app, context).use_material3 {
            radio_defaults_m3(app, context)
        } else {
            radio_defaults_m2(app, context)
        };
        let widget = self.widget(app);
        let toggleable_state = widget.toggleable_state;
        let widget_fill_color = self.widget_fill_color(app);
        let default_fill_color = defaults
            .fill_color
            .as_ref()
            .expect("the defaults' fill color");
        let default_overlay_color = defaults
            .overlay_color
            .as_ref()
            .expect("the defaults' overlay color");
        let default_background_color = defaults
            .background_color
            .as_ref()
            .expect("the defaults' background color");

        // Colors need to be resolved in selected and non selected states separately
        // so that they can be lerped between.
        let mut active_states = toggleable_state.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = toggleable_state.states(app);
        inactive_states.remove(&WidgetState::Selected);
        let active_color = resolve(&widget.fill_color, &active_states)
            .or_else(|| widget_fill_color.resolve(&active_states))
            .or_else(|| resolve(&radio_theme.fill_color, &active_states));
        let effective_active_color = active_color.unwrap_or_else(|| {
            default_fill_color
                .resolve(&active_states)
                .expect("the defaults resolve a fill color")
        });
        let inactive_color = resolve(&widget.fill_color, &inactive_states)
            .or_else(|| widget_fill_color.resolve(&inactive_states))
            .or_else(|| resolve(&radio_theme.fill_color, &inactive_states));
        let effective_inactive_color = inactive_color.unwrap_or_else(|| {
            default_fill_color
                .resolve(&inactive_states)
                .expect("the defaults resolve a fill color")
        });
        let active_background_color = resolve(&widget.background_color, &active_states)
            .or_else(|| resolve(&radio_theme.background_color, &active_states))
            .unwrap_or_else(|| {
                default_background_color
                    .resolve(&active_states)
                    .expect("the defaults resolve a background color")
            });
        let inactive_background_color = resolve(&widget.background_color, &inactive_states)
            .or_else(|| resolve(&radio_theme.background_color, &inactive_states))
            .unwrap_or_else(|| {
                default_background_color
                    .resolve(&inactive_states)
                    .expect("the defaults resolve a background color")
            });

        let mut focused_states = toggleable_state.states(app);
        focused_states.insert(WidgetState::Focused);
        let mut effective_focus_overlay_color = resolve(&widget.overlay_color, &focused_states)
            .or(widget.focus_color)
            .or_else(|| resolve(&radio_theme.overlay_color, &focused_states))
            .unwrap_or_else(|| {
                default_overlay_color
                    .resolve(&focused_states)
                    .expect("the defaults resolve an overlay color")
            });

        let mut hovered_states = toggleable_state.states(app);
        hovered_states.insert(WidgetState::Hovered);
        let mut effective_hover_overlay_color = resolve(&widget.overlay_color, &hovered_states)
            .or(widget.hover_color)
            .or_else(|| resolve(&radio_theme.overlay_color, &hovered_states))
            .unwrap_or_else(|| {
                default_overlay_color
                    .resolve(&hovered_states)
                    .expect("the defaults resolve an overlay color")
            });

        // Dart's `activeStates..add(WidgetState.pressed)` adds to the set itself, which the
        // active side and the inner radius below resolve in.
        active_states.insert(WidgetState::Pressed);
        let active_pressed_states = &active_states;
        let effective_active_pressed_overlay_color =
            resolve(&widget.overlay_color, active_pressed_states)
                .or_else(|| resolve(&radio_theme.overlay_color, active_pressed_states))
                .or_else(|| active_color.map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA)))
                .unwrap_or_else(|| {
                    default_overlay_color
                        .resolve(active_pressed_states)
                        .expect("the defaults resolve an overlay color")
                });

        inactive_states.insert(WidgetState::Pressed);
        let inactive_pressed_states = &inactive_states;
        let effective_inactive_pressed_overlay_color =
            resolve(&widget.overlay_color, inactive_pressed_states)
                .or_else(|| resolve(&radio_theme.overlay_color, inactive_pressed_states))
                .or_else(|| inactive_color.map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA)))
                .unwrap_or_else(|| {
                    default_overlay_color
                        .resolve(inactive_pressed_states)
                        .expect("the defaults resolve an overlay color")
                });

        if toggleable_state.down_position(app).is_some() {
            effective_hover_overlay_color = if toggleable_state
                .states(app)
                .contains(&WidgetState::Selected)
            {
                effective_active_pressed_overlay_color
            } else {
                effective_inactive_pressed_overlay_color
            };
            effective_focus_overlay_color = if toggleable_state
                .states(app)
                .contains(&WidgetState::Selected)
            {
                effective_active_pressed_overlay_color
            } else {
                effective_inactive_pressed_overlay_color
            };
        }

        let effective_material_tap_target_size = widget
            .material_tap_target_size
            .or(radio_theme.material_tap_target_size)
            .or(defaults.material_tap_target_size)
            .expect("the defaults' tap target size");
        let effective_visual_density = widget
            .visual_density
            .or(radio_theme.visual_density)
            .or(defaults.visual_density)
            .expect("the defaults' visual density");
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
        let active_side = Self::resolve_side(&widget.side, &active_states)
            .or_else(|| Self::resolve_side(&radio_theme.side, &active_states))
            .unwrap_or(BorderSide {
                color: effective_active_color,
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                ..BorderSide::default()
            });
        let inactive_side = Self::resolve_side(&widget.side, &inactive_states)
            .or_else(|| Self::resolve_side(&radio_theme.side, &inactive_states))
            .unwrap_or(BorderSide {
                color: effective_inactive_color,
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                ..BorderSide::default()
            });

        let inner_radius = resolve(&widget.inner_radius, &active_states)
            .or_else(|| resolve(&radio_theme.inner_radius, &active_states))
            .unwrap_or(K_INNER_RADIUS);

        let splash_radius = widget
            .splash_radius
            .or(radio_theme.splash_radius)
            .unwrap_or(K_RADIAL_REACTION_RADIUS);
        let current_states = toggleable_state.states(app);
        let painter = self.painter(app);
        painter.set_position(app, toggleable_state.position(app).as_animation());
        painter.set_reaction(app, toggleable_state.reaction(app).as_animation());
        painter.set_reaction_focus_fade(
            app,
            toggleable_state.reaction_focus_fade(app).as_animation(),
        );
        painter.set_reaction_hover_fade(
            app,
            toggleable_state.reaction_hover_fade(app).as_animation(),
        );
        painter.set_inactive_reaction_color(app, effective_inactive_pressed_overlay_color);
        painter.set_reaction_color(app, effective_active_pressed_overlay_color);
        painter.set_hover_color(app, effective_hover_overlay_color);
        painter.set_focus_color(app, effective_focus_overlay_color);
        painter.set_splash_radius(app, splash_radius);
        painter.set_down_position(app, toggleable_state.down_position(app));
        painter.set_is_focused(app, Some(current_states.contains(&WidgetState::Focused)));
        painter.set_is_hovered(app, Some(current_states.contains(&WidgetState::Hovered)));
        painter.set_active_color(app, effective_active_color);
        painter.set_inactive_color(app, effective_inactive_color);
        painter.set_active_background_color(app, Some(active_background_color));
        painter.set_inactive_background_color(app, Some(inactive_background_color));
        painter.set_active_side(app, Some(active_side));
        painter.set_inactive_side(app, Some(inactive_side));
        painter.set_inner_radius(app, Some(inner_radius));

        CustomPaint::new()
            .size(size)
            .painter(ToggleablePainterRef::new(painter))
            .into_widget()
    }
}

/// Dart's `_RadioPainter`.
struct RadioPainter {
    change_notifier: ChangeNotifierData,
    toggleable_painter: ToggleablePainterData,
    inactive_background_color: Option<Color>,
    active_background_color: Option<Color>,
    inactive_side: Option<BorderSide>,
    active_side: Option<BorderSide>,
    inner_radius: Option<f64>,
}

impl RadioPainter {
    fn new(app: &mut App) -> Handle<RadioPainter> {
        app.create(RadioPainter {
            change_notifier: ChangeNotifierData::new(),
            toggleable_painter: ToggleablePainterData::new(),
            inactive_background_color: None,
            active_background_color: None,
            inactive_side: None,
            active_side: None,
            inner_radius: None,
        })
    }

    fn inactive_background_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .inactive_background_color
            .expect("set before painting")
    }

    fn set_inactive_background_color(self: Handle<Self>, app: &mut App, value: Option<Color>) {
        if app.get(self).inactive_background_color == value {
            return;
        }
        app.get_mut(self).inactive_background_color = value;
        self.notify_listeners(app);
    }

    fn active_background_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .active_background_color
            .expect("set before painting")
    }

    fn set_active_background_color(self: Handle<Self>, app: &mut App, value: Option<Color>) {
        if app.get(self).active_background_color == value {
            return;
        }
        app.get_mut(self).active_background_color = value;
        self.notify_listeners(app);
    }

    fn inactive_side(self: Handle<Self>, app: &App) -> BorderSide {
        app.get(self).inactive_side.expect("set before painting")
    }

    fn set_inactive_side(self: Handle<Self>, app: &mut App, value: Option<BorderSide>) {
        if app.get(self).inactive_side == value {
            return;
        }
        app.get_mut(self).inactive_side = value;
        self.notify_listeners(app);
    }

    fn active_side(self: Handle<Self>, app: &App) -> BorderSide {
        app.get(self).active_side.expect("set before painting")
    }

    fn set_active_side(self: Handle<Self>, app: &mut App, value: Option<BorderSide>) {
        if app.get(self).active_side == value {
            return;
        }
        app.get_mut(self).active_side = value;
        self.notify_listeners(app);
    }

    fn inner_radius(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).inner_radius.expect("set before painting")
    }

    fn set_inner_radius(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        if app.get(self).inner_radius == value {
            return;
        }
        app.get_mut(self).inner_radius = value;
        self.notify_listeners(app);
    }
}

impl ChangeNotifier for RadioPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ToggleablePainter for RadioPainter {
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

        let rect = Offset::ZERO & size;
        let center = rect.center();
        let effective_rect = (center & Size::square(K_OUTER_RADIUS * 2.0))
            .translate(-K_OUTER_RADIUS, -K_OUTER_RADIUS);
        let position = self.position(app).value(app);

        // Background
        let background_paint = Paint {
            color: Color::lerp(
                Some(self.inactive_background_color(app)),
                Some(self.active_background_color(app)),
                position,
            )
            .expect("lerping non-null colors")
            .into(),
            style: PaintStyle::Fill,
            ..Paint::default()
        };
        canvas.draw_circle(center, K_OUTER_RADIUS as f32, &background_paint);

        // Outer circle
        let side = BorderSide::lerp(self.inactive_side(app), self.active_side(app), position);
        CircleBorder::new(side, 0.0).paint(canvas, effective_rect, None);

        // Inner circle
        if !self.position(app).is_dismissed(app) {
            let inner_circle_paint = Paint {
                style: PaintStyle::Fill,
                color: Color::lerp(
                    Some(self.inactive_color(app)),
                    Some(self.active_color(app)),
                    position,
                )
                .expect("lerping non-null colors")
                .into(),
                ..Paint::default()
            };
            canvas.draw_circle(
                center,
                (self.inner_radius(app) * position) as f32,
                &inner_circle_paint,
            );
        }
    }
}

/// Hand coded defaults based on Material Design 2: Dart's `_RadioDefaultsM2(context)`, the
/// [`RadioThemeData`] whose fields are the values of that subclass's getters.
fn radio_defaults_m2(app: &mut App, context: BuildContext) -> RadioThemeData {
    let theme = Theme::of(app, context);
    let disabled_color = theme.disabled_color;
    let secondary = theme.color_scheme.secondary;
    let unselected_widget_color = theme.unselected_widget_color;
    let hover_color = theme.hover_color;
    let focus_color = theme.focus_color;
    let fill_color = move |states: &WidgetStates| -> Color {
        if states.contains(&WidgetState::Disabled) {
            return disabled_color;
        }
        if states.contains(&WidgetState::Selected) {
            return secondary;
        }
        unselected_widget_color
    };

    RadioThemeData::new()
        .fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| Some(fill_color(states)),
        ))
        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Pressed) {
                    return Some(fill_color(states).with_alpha(K_RADIAL_REACTION_ALPHA));
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
        .material_tap_target_size(theme.material_tap_target_size)
        .visual_density(theme.visual_density)
        .background_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::TRANSPARENT,
        )))
}

#[cfg(test)]
#[allow(deprecated)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::valo::Op;
    use waft_embedder::{PointerChange, Rect};
    use waft_gestures::{K_LONG_PRESS_TIMEOUT, K_PRIMARY_BUTTON};
    use waft_painting::AlignmentGeometry;
    use waft_rendering::{AnyRenderObject, RenderBox};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, SystemMouseCursors};
    use waft_widgets::{
        Align, AnyElement, Center, Container, FocusHighlightStrategy, GlobalKey, LookupBoundary,
        MouseRegion, Row, SizedBox, StatefulBuilder, TooltipTriggerMode, WidgetStateColor,
    };

    use super::*;
    use crate::material::{Material, RenderInkFeatures};
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::Tooltip;

    /// Where a 100x100 `Container` centred in the 800x600 view lies.
    pub(crate) const CENTER_RECT: Rect = Rect::from_ltrb(350.0, 250.0, 450.0, 350.0);

    /// A tester on Android with the OS fonts, which `MaterialApp`'s debug banner lays out.
    pub(crate) fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    /// Dart's `ThemeData()`.
    pub(crate) fn m3(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    /// Dart's `ThemeData(useMaterial3: false)`.
    pub(crate) fn m2(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    pub(crate) fn no_op<T: 'static>() -> ValueChanged<Option<T>> {
        Rc::new(|_app, _value| {})
    }

    fn log_into(log: &Rc<RefCell<Vec<Option<i32>>>>) -> ValueChanged<Option<i32>> {
        let log = Rc::clone(log);
        Rc::new(move |_app, value| log.borrow_mut().push(value))
    }

    /// A `WidgetStateProperty<Color>` given where Dart's `WidgetStateProperty<Color?>` goes:
    /// Dart's `WidgetStateColor.resolveWith(resolve)`.
    pub(crate) fn colors(
        resolve: impl Fn(&WidgetStates) -> Color + 'static,
    ) -> WidgetStatePropertyRef<Option<Color>> {
        let color = WidgetStateColor::resolve_with(resolve);
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            Some(color.resolve(states))
        })
    }

    /// Dart's `Container(width: 100, height: 100, color: Colors.white, child: child)`.
    fn white_box<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Container::new()
            .width(100.0)
            .height(100.0)
            .color(Colors::WHITE)
            .child(child)
            .into_widget()
    }

    /// Dart's `MaterialApp(theme: theme, home: Material(child: Center(child: child)))`.
    fn centered_app<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(
            theme,
            Material::new()
                .child(Center::new().child(child))
                .into_widget(),
        )
    }

    /// Dart's `MaterialApp(theme: theme, home: Scaffold(body: child))`.
    pub(crate) fn scaffold_app<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(theme, Scaffold::new().body(child).into_widget())
    }

    /// Dart's `find.byType(Radio<T>)`: the one radio of that type.
    pub(crate) fn find_radio<T: Clone + PartialEq + 'static>(tester: &WidgetTester) -> AnyElement {
        let found = tester.find_by_type::<Radio<T>>();
        assert_eq!(found.len(), 1, "one Radio");
        found[0]
    }

    pub(crate) fn center_of_radio<T: Clone + PartialEq + 'static>(tester: &WidgetTester) -> Offset {
        let render = tester.render_box_of(find_radio::<T>(tester));
        tester.global_rect(render).center()
    }

    /// Dart's `Material.of(tester.element(find.byType(Radio<T>)))`: the ink features of the
    /// material the radio paints into, which Dart's `paints` matcher paints with its subtree.
    pub(crate) fn radio_material<T: Clone + PartialEq + 'static>(
        tester: &WidgetTester,
    ) -> AnyRenderObject {
        let radio = find_radio::<T>(tester);
        let app = tester.cell.borrow();
        let ink =
            LookupBoundary::find_ancestor_render_object_of_type::<RenderInkFeatures>(&app, radio)
                .expect("a Material above the radio");
        RenderBox::as_object(ink)
    }

    /// Dart's `createGesture(kind: mouse)`, `addPointer()` and `moveTo(at)`.
    pub(crate) fn hover_at(tester: &mut WidgetTester, at: Offset) {
        let mut mouse = WidgetTester::mouse(1, K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, at, false);
    }

    /// Dart's `tester.press(finder)`: a touch pointer `id` down at `at`, left down.
    pub(crate) fn press_at(tester: &mut WidgetTester, at: Offset, id: i64) {
        let mut touch = WidgetTester::touch(id);
        tester.down(&mut touch, at);
    }

    /// One recorded canvas op, reduced to what the radio cases' `paints` matchers compare. A
    /// draw with a transparent paint is not recorded at all, so Dart's
    /// `..circle(color: Colors.transparent)` has nothing to match here and is left out of the
    /// expectations.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) enum Painted {
        /// `drawRect`: its colour as `0xAARRGGBB` and its rect.
        Rect(u32, Rect),
        /// `drawCircle`: its colour, its radius and, for a stroke, its stroke width.
        Circle {
            color: u32,
            radius: f32,
            stroke_width: Option<f32>,
        },
        Save,
        Restore,
    }

    fn argb(color: waft_embedder::valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    impl Painted {
        fn of(op: &Op) -> Option<Painted> {
            match op {
                Op::DrawRect { paint, rect, .. } => Some(Painted::Rect(
                    argb(paint.color),
                    Rect::from_ltwh(
                        f64::from(rect.x),
                        f64::from(rect.y),
                        f64::from(rect.width),
                        f64::from(rect.height),
                    ),
                )),
                Op::DrawPath { path, paint, .. } => {
                    let bounds = path.bounds();
                    if (bounds.width - bounds.height).abs() >= 1e-3 {
                        return None;
                    }
                    Some(Painted::Circle {
                        color: argb(paint.color),
                        radius: bounds.width / 2.0,
                        stroke_width: match &paint.style {
                            PaintStyle::Stroke(stroke) => Some(stroke.width),
                            PaintStyle::Fill => None,
                        },
                    })
                }
                Op::Save => Some(Painted::Save),
                Op::Restore => Some(Painted::Restore),
                _ => None,
            }
        }
    }

    /// What `render` and its subtree draw, painted at the origin.
    pub(crate) fn painted(tester: &WidgetTester, render: AnyRenderObject) -> Vec<Painted> {
        tester
            .pictures(render)
            .iter()
            .flat_map(|picture| {
                picture
                    .ops()
                    .iter()
                    .filter_map(Painted::of)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// One step of Dart's `paints` matcher.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Paints {
        Rect {
            color: Option<Color>,
            rect: Option<Rect>,
        },
        Circle {
            color: Option<Color>,
            radius: Option<f64>,
            stroke_width: Option<f64>,
        },
        Save,
        Restore,
    }

    /// Dart's `..rect(color:, rect:)`.
    pub(crate) fn rect(color: Color, rect: Rect) -> Paints {
        Paints::Rect {
            color: Some(color),
            rect: Some(rect),
        }
    }

    /// Dart's `..rect()`.
    pub(crate) fn any_rect() -> Paints {
        Paints::Rect {
            color: None,
            rect: None,
        }
    }

    /// Dart's `..circle(color:)`.
    pub(crate) fn circle(color: Color) -> Paints {
        Paints::Circle {
            color: Some(color),
            radius: None,
            stroke_width: None,
        }
    }

    impl Paints {
        /// Dart's `radius:` argument of `..circle`.
        pub(crate) fn radius(self, radius: f64) -> Paints {
            match self {
                Paints::Circle {
                    color,
                    stroke_width,
                    ..
                } => Paints::Circle {
                    color,
                    radius: Some(radius),
                    stroke_width,
                },
                other => other,
            }
        }

        /// Dart's `strokeWidth:` argument of `..circle`, with or without `style: stroke`.
        pub(crate) fn stroke_width(self, stroke_width: f64) -> Paints {
            match self {
                Paints::Circle { color, radius, .. } => Paints::Circle {
                    color,
                    radius,
                    stroke_width: Some(stroke_width),
                },
                other => other,
            }
        }

        fn is_same_call(&self, painted: &Painted) -> bool {
            matches!(
                (self, painted),
                (Paints::Rect { .. }, Painted::Rect(..))
                    | (Paints::Circle { .. }, Painted::Circle { .. })
                    | (Paints::Save, Painted::Save)
                    | (Paints::Restore, Painted::Restore)
            )
        }

        fn matches(&self, painted: &Painted) -> bool {
            match (*self, *painted) {
                (Paints::Rect { color, rect }, Painted::Rect(painted_color, painted_rect)) => {
                    color.is_none_or(|color| color.value() == painted_color)
                        && rect.is_none_or(|rect| rect == painted_rect)
                }
                (
                    Paints::Circle {
                        color,
                        radius,
                        stroke_width,
                    },
                    Painted::Circle {
                        color: painted_color,
                        radius: painted_radius,
                        stroke_width: painted_stroke_width,
                    },
                ) => {
                    color.is_none_or(|color| color.value() == painted_color)
                        && radius.is_none_or(|radius| (radius as f32 - painted_radius).abs() < 1e-3)
                        && stroke_width.is_none_or(|stroke_width| {
                            painted_stroke_width == Some(stroke_width as f32)
                        })
                }
                (Paints::Save, Painted::Save) | (Paints::Restore, Painted::Restore) => true,
                _ => false,
            }
        }
    }

    /// Dart's `expect(render, paints..a()..b())`: each step finds the next call of its kind,
    /// skipping calls of other kinds, and that call must match it.
    pub(crate) fn assert_paints(
        tester: &WidgetTester,
        render: AnyRenderObject,
        expected: &[Paints],
    ) {
        let actual = painted(tester, render);
        let mut calls = actual.iter();
        for step in expected {
            let call = calls
                .find(|call| step.is_same_call(call))
                .unwrap_or_else(|| panic!("no call left for {step:?} in {actual:?}"));
            assert!(
                step.matches(call),
                "{call:?} does not match {step:?} in {actual:?}"
            );
        }
    }

    /// The first `T` at or below `node`, depth first.
    fn find_below<T: waft_rendering::RenderObject>(
        app: &App,
        node: AnyRenderObject,
    ) -> Option<waft_rendering::RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find_below::<T>(app, child);
            }
        });
        found
    }

    /// The radio's painter, for what valo records no draw of.
    fn painter<T: Clone + PartialEq + 'static>(tester: &WidgetTester) -> Handle<RadioPainter> {
        let radio = tester.render_box_of(find_radio::<T>(tester)).as_object();
        let app = tester.cell.borrow();
        let custom = find_below::<waft_rendering::RenderCustomPaint>(&app, radio)
            .expect("the radio's paint");
        let painter = custom.painter(&app).expect("the radio's painter");
        painter
            .as_any()
            .downcast_ref::<ToggleablePainterRef<RadioPainter>>()
            .expect("a radio painter")
            .painter()
    }

    #[test]
    fn a_radio_reports_its_value_when_tapped_unless_selected_or_without_on_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();
            let wrap = |radio: Radio<i32>| {
                Theme::new(
                    theme.clone(),
                    Material::new().child(Center::new().child(radio.key(key.clone().into()))),
                )
                .into_widget()
            };

            tester.pump_widget(wrap(
                Radio::new(1).group_value(2).on_changed(log_into(&log)),
            ));
            tester.tap(tester.center_of(&key), 1);
            assert_eq!(*log.borrow(), [Some(1)]);
            log.borrow_mut().clear();

            tester.pump_widget(wrap(
                Radio::new(1)
                    .group_value(1)
                    .on_changed(log_into(&log))
                    .active_color(Colors::GREEN.shade500()),
            ));
            tester.tap(tester.center_of(&key), 2);
            assert!(log.borrow().is_empty());

            tester.pump_widget(wrap(Radio::new(1).group_value(2)));
            tester.tap(tester.center_of(&key), 3);
            assert!(log.borrow().is_empty());
        });
    }

    #[test]
    fn a_disabled_radio_does_not_report_a_tap() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();

            tester.pump_widget(
                Theme::new(
                    theme,
                    Material::new().child(
                        Center::new().child(
                            Radio::new(1)
                                .key(key.clone().into())
                                .group_value(2)
                                .enabled(false)
                                .on_changed(log_into(&log)),
                        ),
                    ),
                )
                .into_widget(),
            );
            tester.tap(tester.center_of(&key), 1);
            assert!(log.borrow().is_empty());
        });
    }

    #[test]
    fn a_toggleable_radio_reports_null_when_tapped_while_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();
            let wrap = |radio: Radio<i32>| {
                Theme::new(
                    theme.clone(),
                    Material::new().child(Center::new().child(radio.key(key.clone().into()))),
                )
                .into_widget()
            };

            tester.pump_widget(wrap(
                Radio::new(1)
                    .group_value(2)
                    .on_changed(log_into(&log))
                    .toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 1);
            assert_eq!(*log.borrow(), [Some(1)]);
            log.borrow_mut().clear();

            tester.pump_widget(wrap(
                Radio::new(1)
                    .group_value(1)
                    .on_changed(log_into(&log))
                    .toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 2);
            assert_eq!(*log.borrow(), [None]);
            log.borrow_mut().clear();

            tester.pump_widget(wrap(
                Radio::new(1).on_changed(log_into(&log)).toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 3);
            assert_eq!(*log.borrow(), [Some(1)]);
        });
    }

    #[test]
    fn the_radio_size_is_configurable_by_the_theme_tap_target_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let size_of = |tester: &mut WidgetTester, tap_target_size: MaterialTapTargetSize| {
                let key = GlobalKey::new();
                tester.pump_widget(
                    Theme::new(
                        theme
                            .copy_with()
                            .material_tap_target_size(tap_target_size)
                            .build(),
                        waft_widgets::Directionality::new(
                            waft_embedder::TextDirection::Ltr,
                            Material::new().child(
                                Center::new().child(
                                    Radio::new(true)
                                        .key(key.clone().into())
                                        .group_value(true)
                                        .on_changed(no_op()),
                                ),
                            ),
                        ),
                    )
                    .into_widget(),
                );
                let render = tester.render_object(&key).as_box().expect("a box");
                render.size(&tester.cell.borrow())
            };

            assert_eq!(
                size_of(&mut tester, MaterialTapTargetSize::Padded),
                Size::new(48.0, 48.0)
            );
            assert_eq!(
                size_of(&mut tester, MaterialTapTargetSize::ShrinkWrap),
                Size::new(40.0, 40.0)
            );
        });
    }

    #[test]
    fn a_radio_with_a_splash_radius_paints_its_focus_reaction_that_large() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 30.0;
            let theme = m3(&tester);
            tester.pump_widget(centered_app(
                theme,
                white_box(
                    Radio::new(0)
                        .on_changed(no_op())
                        .focus_color(Colors::ORANGE.shade500())
                        .autofocus(true)
                        .group_value(0)
                        .splash_radius(SPLASH_RADIUS),
                ),
            ));
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[circle(Colors::ORANGE.shade500()).radius(SPLASH_RADIUS)],
            );
        });
    }

    /// Dart's `buildApp({enabled})` of the focus and hover cases: a `Radio<int>` of value 0 in
    /// a white box, enabled through `onChanged`.
    fn focus_and_hover_app(
        theme: ThemeData,
        group_value: i32,
        enabled: bool,
        configure: impl FnOnce(Radio<i32>) -> Radio<i32>,
    ) -> WidgetRef {
        let mut radio = Radio::new(0).group_value(group_value);
        if enabled {
            radio = radio.on_changed(no_op());
        }
        centered_app(theme, white_box(configure(radio)))
    }

    #[test]
    fn material2_a_radio_is_focusable_and_has_the_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let theme = m2(&tester);
            let configure = move |radio: Radio<i32>| {
                radio
                    .focus_color(Colors::ORANGE.shade500())
                    .autofocus(true)
                    .focus_node(focus_node)
            };

            tester.pump_widget(focus_and_hover_app(theme.clone(), 0, true, configure));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Colors::ORANGE.shade500()),
                    circle(Color::new(0xff2196f3)),
                    circle(Color::new(0xff2196f3)),
                ],
            );

            // Check when the radio isn't selected.
            tester.pump_widget(focus_and_hover_app(theme.clone(), 1, true, configure));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Colors::ORANGE.shade500()),
                    circle(Color::new(0x8a000000)).stroke_width(2.0),
                ],
            );

            // Check when the radio is selected, but disabled.
            tester.pump_widget(focus_and_hover_app(theme, 0, false, configure));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Color::new(0x61000000)),
                    circle(Color::new(0x61000000)),
                ],
            );
        });
    }

    #[test]
    fn material3_a_radio_is_focusable_and_has_the_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let theme = m3(&tester);
            let colors = theme.color_scheme.clone();
            let configure = move |radio: Radio<i32>| {
                radio
                    .focus_color(Colors::ORANGE.shade500())
                    .autofocus(true)
                    .focus_node(focus_node)
            };

            tester.pump_widget(focus_and_hover_app(theme.clone(), 0, true, configure));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Colors::ORANGE.shade500()),
                    circle(colors.primary),
                    circle(colors.primary),
                ],
            );

            // Check when the radio isn't selected.
            tester.pump_widget(focus_and_hover_app(theme.clone(), 1, true, configure));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    any_rect(),
                    circle(Colors::ORANGE.shade500()),
                    circle(colors.on_surface),
                ],
            );

            // Check when the radio is selected, but disabled.
            tester.pump_widget(focus_and_hover_app(theme, 0, false, configure));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(colors.on_surface.with_opacity(0.38)),
                    circle(colors.on_surface.with_opacity(0.38)),
                ],
            );
        });
    }

    #[test]
    fn material2_a_radio_can_be_hovered_and_has_the_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let theme = m2(&tester);
            let configure = |radio: Radio<i32>| radio.hover_color(Colors::ORANGE.shade500());

            tester.pump_widget(focus_and_hover_app(theme.clone(), 0, true, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Color::new(0xff2196f3)),
                    circle(Color::new(0xff2196f3)),
                ],
            );

            // Start hovering
            let mut mouse = WidgetTester::mouse(1, K_PRIMARY_BUTTON);
            let center = center_of_radio::<i32>(&tester);
            tester.move_to(&mut mouse, center, false);

            // Check when the radio isn't selected.
            tester.pump_widget(focus_and_hover_app(theme.clone(), 1, true, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Colors::ORANGE.shade500()),
                    circle(Color::new(0x8a000000)).stroke_width(2.0),
                ],
            );

            // Check when the radio is selected, but disabled.
            tester.pump_widget(focus_and_hover_app(theme, 0, false, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Color::new(0x61000000)),
                    circle(Color::new(0x61000000)),
                ],
            );
        });
    }

    #[test]
    fn material3_a_radio_can_be_hovered_and_has_the_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let theme = m3(&tester);
            let colors = theme.color_scheme.clone();
            let configure = |radio: Radio<i32>| radio.hover_color(Colors::ORANGE.shade500());

            tester.pump_widget(focus_and_hover_app(theme.clone(), 0, true, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(colors.primary),
                    circle(colors.primary),
                ],
            );

            // Start hovering
            let mut mouse = WidgetTester::mouse(1, K_PRIMARY_BUTTON);
            let center = center_of_radio::<i32>(&tester);
            tester.move_to(&mut mouse, center, false);

            // Check when the radio isn't selected.
            tester.pump_widget(focus_and_hover_app(theme.clone(), 1, true, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(Colors::ORANGE.shade500()),
                    circle(colors.on_surface).stroke_width(2.0),
                ],
            );

            // Check when the radio is selected, but disabled.
            tester.pump_widget(focus_and_hover_app(theme, 0, false, configure));
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<i32>(&tester),
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(colors.on_surface.with_opacity(0.38)),
                    circle(colors.on_surface.with_opacity(0.38)),
                ],
            );
        });
    }

    #[test]
    fn a_radio_can_be_controlled_by_keyboard_shortcuts() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let group_value = Rc::new(Cell::new(Some(1)));
            let focus_node2 = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            focus_node2.set_debug_label(&mut tester.cell.borrow_mut(), Some("radio2".into()));
            let theme = m3(&tester);

            let builder_value = Rc::clone(&group_value);
            tester.pump_widget(centered_app(
                theme,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let radio = |value: i32| {
                        let group_value = Rc::clone(&builder_value);
                        let set_state = Rc::clone(&set_state);
                        let mut radio = Radio::new(value)
                            .on_changed(Rc::new(move |app: &mut App, new_value: Option<i32>| {
                                set_state(app, &mut |_app| group_value.set(new_value));
                            }))
                            .hover_color(Colors::ORANGE.shade500());
                        if let Some(group_value) = builder_value.get() {
                            radio = radio.group_value(group_value);
                        }
                        radio
                    };
                    Container::new()
                        .width(200.0)
                        .height(100.0)
                        .color(Colors::WHITE)
                        .child(Row::new().children([
                            radio(0).autofocus(true).into_widget(),
                            radio(1).into_widget(),
                            radio(2).focus_node(focus_node2).into_widget(),
                        ]))
                        .into_widget()
                }),
            ));
            tester.pump_and_settle();

            tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            // On web, radios don't respond to the enter key.
            assert_eq!(
                group_value.get(),
                if waft_foundation::K_IS_WEB {
                    Some(1)
                } else {
                    Some(0)
                }
            );

            focus_node2.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
            tester.pump_and_settle();
            assert_eq!(group_value.get(), Some(2));
        });
    }

    /// widgets' radio_group_test 'Radio group can use arrow key', with Material radios.
    #[test]
    fn arrow_keys_move_the_selection_through_a_radio_group() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let group_value: Rc<Cell<Option<i32>>> = Rc::default();
            let theme = m3(&tester);
            let key0 = GlobalKey::new();

            let builder_value = Rc::clone(&group_value);
            let builder_key = key0.clone();
            tester.pump_widget(centered_app(
                theme,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let group_value = Rc::clone(&builder_value);
                    let mut group = RadioGroup::<i32>::new(
                        Rc::new(move |app: &mut App, value: Option<i32>| {
                            set_state(app, &mut |_app| group_value.set(value));
                        }),
                        waft_widgets::Column::new().children([
                            Radio::new(0)
                                .key(builder_key.clone().into())
                                .focus_node(focus_node)
                                .into_widget(),
                            Radio::new(1).into_widget(),
                            Radio::new(2).into_widget(),
                        ]),
                    );
                    if let Some(value) = builder_value.get() {
                        group = group.group_value(value);
                    }
                    group.into_widget()
                }),
            ));

            tester.tap(tester.center_of(&key0), 1);
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(group_value.get(), Some(0));
            assert!(focus_node.has_focus(&mut tester.cell.borrow_mut()));

            let press = |tester: &mut WidgetTester,
                         physical: PhysicalKeyboardKey,
                         logical: LogicalKeyboardKey| {
                tester.send_key_event(physical, logical);
                tester.pump_and_settle();
                group_value.get()
            };
            assert_eq!(
                press(
                    &mut tester,
                    PhysicalKeyboardKey::ARROW_DOWN,
                    LogicalKeyboardKey::ARROW_DOWN
                ),
                Some(1)
            );
            assert_eq!(
                press(
                    &mut tester,
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT
                ),
                Some(2)
            );
            // Wrap around
            assert_eq!(
                press(
                    &mut tester,
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT
                ),
                Some(0)
            );
            // Wrap around
            assert_eq!(
                press(
                    &mut tester,
                    PhysicalKeyboardKey::ARROW_LEFT,
                    LogicalKeyboardKey::ARROW_LEFT
                ),
                Some(2)
            );
            assert_eq!(
                press(
                    &mut tester,
                    PhysicalKeyboardKey::ARROW_UP,
                    LogicalKeyboardKey::ARROW_UP
                ),
                Some(1)
            );
        });
    }

    #[test]
    fn a_radio_responds_to_density_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let build_test = |tester: &mut WidgetTester, visual_density: VisualDensity| {
                tester.pump_widget(centered_app(
                    theme.clone(),
                    Radio::new(0)
                        .visual_density(visual_density)
                        .key(key.clone().into())
                        .on_changed(no_op())
                        .group_value(0),
                ));
                tester.pump_and_settle();
                let render = tester.render_object(&key).as_box().expect("a box");
                render.size(&tester.cell.borrow())
            };

            assert_eq!(
                build_test(&mut tester, VisualDensity::STANDARD),
                Size::new(48.0, 48.0)
            );
            assert_eq!(
                build_test(&mut tester, VisualDensity::new(3.0, 3.0)),
                Size::new(60.0, 60.0)
            );
            assert_eq!(
                build_test(&mut tester, VisualDensity::new(-3.0, -3.0)),
                Size::new(36.0, 36.0)
            );
            assert_eq!(
                build_test(&mut tester, VisualDensity::new(3.0, -3.0)),
                Size::new(60.0, 36.0)
            );
        });
    }

    #[test]
    fn a_radio_changes_the_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let app_with = |radio: Radio<i32>| {
                scaffold_app(
                    theme.clone(),
                    Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                        Material::new().child(
                            MouseRegion::new()
                                .cursor(SystemMouseCursors::FORBIDDEN.into())
                                .child(radio),
                        ),
                    ),
                )
            };

            // Test Radio() constructor
            tester.pump_widget(app_with(
                Radio::new(1)
                    .mouse_cursor(SystemMouseCursors::TEXT.into())
                    .on_changed(no_op())
                    .group_value(2),
            ));
            let mut mouse = WidgetTester::mouse(1, K_PRIMARY_BUTTON);
            let center = center_of_radio::<i32>(&tester);
            tester.add_pointer(&mut mouse, center);
            tester.pump(Duration::ZERO);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            tester.pump_widget(app_with(Radio::new(1).on_changed(no_op()).group_value(2)));
            assert_eq!(
                tester.active_cursor(1),
                if waft_foundation::K_IS_WEB {
                    SystemMouseCursors::CLICK.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                }
            );

            // Test default cursor when disabled
            tester.pump_widget(app_with(Radio::new(1).group_value(2)));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
        });
    }

    #[test]
    fn the_fill_color_resolves_in_enabled_and_disabled_states() {
        with_main_thread_stack(|| {
            const ACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000004);
            let fill_color = colors(|states| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return ACTIVE_DISABLED_FILL_COLOR;
                    }
                    return INACTIVE_DISABLED_FILL_COLOR;
                }
                if states.contains(&WidgetState::Selected) {
                    return ACTIVE_ENABLED_FILL_COLOR;
                }
                INACTIVE_ENABLED_FILL_COLOR
            });
            let mut tester = tester();
            let theme = m3(&tester);
            let mut check = |group_value: i32, enabled: bool, expected: &[Paints]| {
                let fill_color = Rc::clone(&fill_color);
                tester.pump_widget(focus_and_hover_app(
                    theme.clone(),
                    group_value,
                    enabled,
                    move |radio| radio.fill_color(fill_color),
                ));
                tester.pump_and_settle();
                assert_paints(&tester, radio_material::<i32>(&tester), expected);
            };

            // Selected and enabled.
            check(
                0,
                true,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(ACTIVE_ENABLED_FILL_COLOR),
                    circle(ACTIVE_ENABLED_FILL_COLOR),
                ],
            );
            // Check when the radio isn't selected.
            check(
                1,
                true,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(INACTIVE_ENABLED_FILL_COLOR).stroke_width(2.0),
                ],
            );
            // Check when the radio is selected, but disabled.
            check(
                0,
                false,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(ACTIVE_DISABLED_FILL_COLOR),
                    circle(ACTIVE_DISABLED_FILL_COLOR),
                ],
            );
            // Check when the radio is unselected and disabled.
            check(
                1,
                false,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(INACTIVE_DISABLED_FILL_COLOR).stroke_width(2.0),
                ],
            );
        });
    }

    /// The fill colour of the hovered and focused cases.
    fn hovered_or_focused_fill_color() -> WidgetStatePropertyRef<Option<Color>> {
        colors(|states| {
            if states.contains(&WidgetState::Hovered) {
                return Color::new(0xFF000001);
            }
            if states.contains(&WidgetState::Focused) {
                return Color::new(0xFF000002);
            }
            Colors::TRANSPARENT
        })
    }

    fn fill_color_in_hovered_and_focused_states(m3_theme: bool, focus: Color, hover: Color) {
        let mut tester = tester();
        tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
        let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
        focus_node.set_debug_label(&mut tester.cell.borrow_mut(), Some("radio".into()));
        let theme = if m3_theme { m3(&tester) } else { m2(&tester) };
        let fill_color = hovered_or_focused_fill_color();

        tester.pump_widget(focus_and_hover_app(theme, 0, true, move |radio| {
            radio
                .autofocus(true)
                .focus_node(focus_node)
                .fill_color(fill_color)
        }));
        tester.pump_and_settle();
        assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        assert_paints(
            &tester,
            radio_material::<i32>(&tester),
            &[
                if m3_theme {
                    any_rect()
                } else {
                    rect(Color::new(0xffffffff), CENTER_RECT)
                },
                circle(focus),
                circle(Color::new(0xFF000002)),
            ],
        );

        // Start hovering
        let center = center_of_radio::<i32>(&tester);
        hover_at(&mut tester, center);
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<i32>(&tester),
            &[
                rect(Color::new(0xffffffff), CENTER_RECT),
                circle(hover),
                circle(Color::new(0xFF000001)),
            ],
        );
    }

    #[test]
    fn material2_the_fill_color_resolves_in_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            let hover_color = m2(&tester()).hover_color;
            fill_color_in_hovered_and_focused_states(false, Colors::BLACK12, hover_color);
        });
    }

    #[test]
    fn material3_the_fill_color_resolves_in_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            let primary = m3(&tester()).color_scheme.primary;
            fill_color_in_hovered_and_focused_states(
                true,
                primary.with_opacity(0.1),
                primary.with_opacity(0.08),
            );
        });
    }

    #[test]
    fn the_overlay_color_resolves_in_active_pressed_focused_and_hovered_states() {
        with_main_thread_stack(|| {
            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const FOCUS_OVERLAY_COLOR: Color = Color::new(0xFF000004);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
            const FOCUS_COLOR: Color = Color::new(0xFF000006);
            const SPLASH_RADIUS: f64 = 24.0;
            let get_overlay_color = |states: &WidgetStates| -> Option<Color> {
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
            };
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let theme = m3(&tester);
            let build_radio = |active: bool, focused: bool, use_overlay: bool| {
                let mut radio = Radio::new(active)
                    .focus_node(focus_node)
                    .autofocus(focused)
                    .group_value(true)
                    .on_changed(no_op())
                    .fill_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                        FILL_COLOR,
                    )))
                    .hover_color(HOVER_COLOR)
                    .focus_color(FOCUS_COLOR)
                    .splash_radius(SPLASH_RADIUS);
                if use_overlay {
                    radio = radio.overlay_color(
                        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(get_overlay_color),
                    );
                }
                scaffold_app(theme.clone(), radio)
            };
            let reaction = |color: Color| [circle(color).radius(SPLASH_RADIUS)];

            tester.pump_widget(build_radio(false, false, false));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 1);
            tester.pump_and_settle();
            // Default inactive pressed Radio should have overlay color from fillColor
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA)),
            );

            tester.pump_widget(build_radio(true, false, false));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 2);
            tester.pump_and_settle();
            // Default active pressed Radio should have overlay color from fillColor
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA)),
            );

            tester.pump_widget(build_radio(false, false, true));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 3);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(INACTIVE_PRESSED_OVERLAY_COLOR),
            );

            tester.pump_widget(build_radio(true, false, true));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 4);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(ACTIVE_PRESSED_OVERLAY_COLOR),
            );

            tester.pump_widget(Container::new().into_widget());
            tester.pump_widget(build_radio(false, true, true));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            // Focused Radio should use overlay color FOCUS_OVERLAY_COLOR over FOCUS_COLOR
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(FOCUS_OVERLAY_COLOR),
            );

            // Start hovering
            let center = center_of_radio::<bool>(&tester);
            hover_at(&mut tester, center);
            tester.pump_and_settle();
            // Hovered Radio should use overlay color HOVER_OVERLAY_COLOR over HOVER_COLOR
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &reaction(HOVER_OVERLAY_COLOR),
            );
        });
    }

    #[test]
    fn a_radio_that_disappears_while_the_pointer_is_down_does_not_crash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let build_radio = |show: bool| {
                if show {
                    centered_app(
                        theme.clone(),
                        Radio::new(true)
                            .key(key.clone().into())
                            .group_value(false)
                            .on_changed(no_op()),
                    )
                } else {
                    centered_app(theme.clone(), Container::new())
                }
            };

            tester.pump_widget(build_radio(true));
            let center = tester.center_of(&key);
            // Put a pointer down on the screen.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            tester.pump(Duration::ZERO);
            // While the pointer is down, the widget disappears.
            tester.pump_widget(build_radio(false));
            assert!(key.current_context(&mut tester.cell.borrow_mut()).is_none());
            // Release pointer after widget disappeared.
            tester.up(&gesture);
        });
    }

    #[test]
    fn a_disabled_radio_shows_a_tooltip() {
        with_main_thread_stack(|| {
            const LONG_PRESS_TOOLTIP: &str = "long press tooltip";
            const TAP_TOOLTIP: &str = "tap tooltip";
            let mut tester = tester();
            let theme = m3(&tester);
            let tooltip_center = |tester: &WidgetTester| {
                let tooltip = tester.find_by_type::<Tooltip>()[0];
                tester.global_rect(tester.render_box_of(tooltip)).center()
            };
            tester.pump_widget(material_app(
                theme.clone(),
                Material::new()
                    .child(
                        Tooltip::new()
                            .message(LONG_PRESS_TOOLTIP)
                            .child(Radio::new(true).group_value(false)),
                    )
                    .into_widget(),
            ));

            // Default tooltip shows up after long pressed.
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            tester.tap(tooltip_center(&tester), 1);
            tester.pump(Duration::from_millis(10));
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            let mut gesture_long_press = WidgetTester::touch(2);
            tester.down(&mut gesture_long_press, tooltip_center(&tester));
            tester.pump(Duration::ZERO);
            tester.pump(K_LONG_PRESS_TIMEOUT);
            tester.up(&gesture_long_press);
            tester.pump(Duration::ZERO);

            assert_eq!(tester.find_text(LONG_PRESS_TOOLTIP).len(), 1);

            // Tooltip shows up after tapping when set triggerMode to TooltipTriggerMode.tap.
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Tooltip::new()
                            .trigger_mode(TooltipTriggerMode::Tap)
                            .message(TAP_TOOLTIP)
                            .child(Radio::new(true).group_value(false)),
                    )
                    .into_widget(),
            ));

            tester.pump(Duration::from_secs(24 * 60 * 60));
            tester.pump_and_settle();
            assert!(tester.find_text(TAP_TOOLTIP).is_empty());
            assert!(tester.find_text(LONG_PRESS_TOOLTIP).is_empty());

            tester.tap(tooltip_center(&tester), 3);
            tester.pump(Duration::from_millis(10));
            assert_eq!(tester.find_text(TAP_TOOLTIP).len(), 1);
        });
    }

    fn default_colors(m3_theme: bool) {
        let mut tester = tester();
        let theme = if m3_theme { m3(&tester) } else { m2(&tester) };
        let selected = if m3_theme {
            theme.color_scheme.primary
        } else {
            Color::new(0xFF2196F3)
        };
        let disabled = if m3_theme {
            theme.color_scheme.on_surface.with_opacity(0.38)
        } else {
            Colors::BLACK38
        };
        let build_radio = |enabled: bool| {
            let mut radio = Radio::new(true).group_value(true);
            if enabled {
                radio = radio.on_changed(no_op());
            }
            scaffold_app(theme.clone(), radio)
        };

        tester.pump_widget(build_radio(true));
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[
                // Outer circle - primary value
                circle(selected),
                // Inner circle - primary value
                circle(selected),
                Paints::Restore,
            ],
        );

        tester.pump_widget(Container::new().into_widget());
        tester.pump_widget(build_radio(true));
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[Paints::Save, circle(selected), Paints::Restore],
        );

        tester.pump_widget(Container::new().into_widget());
        tester.pump_widget(build_radio(false));
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[circle(disabled)],
        );
    }

    #[test]
    fn material2_a_radio_has_the_default_colors() {
        with_main_thread_stack(|| default_colors(false));
    }

    #[test]
    fn material3_a_radio_has_the_default_colors() {
        with_main_thread_stack(|| default_colors(true));
    }

    #[test]
    fn material2_a_radio_has_the_default_overlay_colors_in_hover_focus_and_press_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let theme = m2(&tester);
            let colors = theme.color_scheme.clone();
            let build_radio = |focused: bool, selected: bool| {
                scaffold_app(
                    theme.clone(),
                    Radio::new(true)
                        .focus_node(focus_node)
                        .autofocus(focused)
                        .group_value(selected)
                        .on_changed(no_op()),
                )
            };

            // default selected radio
            tester.pump_widget(build_radio(false, true));
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[circle(colors.secondary)],
            );

            // selected radio in pressed state
            tester.pump_widget(build_radio(false, true));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 1);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.secondary.with_alpha(0x1F)),
                    circle(colors.secondary),
                ],
            );

            // unselected radio in pressed state
            tester.pump_widget(build_radio(false, false));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 2);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(theme.unselected_widget_color.with_alpha(0x1F)),
                    circle(theme.unselected_widget_color),
                ],
            );

            // selected radio in focused state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(true, true));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[circle(theme.focus_color), circle(colors.secondary)],
            );

            // unselected radio in focused state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(true, false));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(theme.focus_color),
                    circle(theme.unselected_widget_color),
                ],
            );

            // selected radio in hovered state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(false, true));
            let center = center_of_radio::<bool>(&tester);
            hover_at(&mut tester, center);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[circle(theme.hover_color), circle(colors.secondary)],
            );
        });
    }

    #[test]
    fn material3_a_radio_has_the_default_overlay_colors_in_hover_focus_and_press_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
            let theme = m3(&tester);
            let colors = theme.color_scheme.clone();
            let build_radio = |focused: bool, selected: bool| {
                scaffold_app(
                    theme.clone(),
                    Radio::new(true)
                        .focus_node(focus_node)
                        .autofocus(focused)
                        .group_value(selected)
                        .on_changed(no_op()),
                )
            };

            // default selected radio
            tester.pump_widget(build_radio(false, true));
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[circle(colors.primary.with_opacity(1.0))],
            );

            // selected radio in pressed state
            tester.pump_widget(build_radio(false, true));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 1);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.on_surface.with_opacity(0.1)),
                    circle(colors.primary.with_opacity(1.0)),
                ],
            );

            // unselected radio in pressed state
            tester.pump_widget(build_radio(false, false));
            let center = center_of_radio::<bool>(&tester);
            press_at(&mut tester, center, 2);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.primary.with_opacity(0.1)),
                    circle(colors.on_surface_variant().with_opacity(1.0)),
                ],
            );

            // selected radio in focused state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(true, true));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.primary.with_opacity(0.1)),
                    circle(colors.primary.with_opacity(1.0)),
                ],
            );

            // unselected radio in focused state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(true, false));
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.on_surface.with_opacity(0.1)),
                    circle(colors.on_surface.with_opacity(1.0)),
                ],
            );

            // selected radio in hovered state
            tester.pump_widget(Container::new().into_widget()); // reset test
            tester.pump_widget(build_radio(false, true));
            let center = center_of_radio::<bool>(&tester);
            hover_at(&mut tester, center);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                radio_material::<bool>(&tester),
                &[
                    circle(colors.primary.with_opacity(0.08)),
                    circle(colors.primary.with_opacity(1.0)),
                ],
            );
        });
    }

    #[test]
    fn radio_adaptive_shows_the_widget_of_the_platform() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_app = |tester: &WidgetTester, platform: TargetPlatform| {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .build(&tester.cell.borrow());
                centered_app(theme, Radio::adaptive(1).group_value(2).on_changed(no_op()))
            };

            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                let app = build_app(&tester, platform);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert_eq!(
                    tester.find_by_type::<CupertinoRadio<i32>>().len(),
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
                let app = build_app(&tester, platform);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert!(
                    tester.find_by_type::<CupertinoRadio<i32>>().is_empty(),
                    "{platform:?}"
                );
            }
        });
    }

    fn default_overlay_and_fill_colors_resolve_the_pressed_state(
        m3_theme: bool,
        hover: impl Fn(&ThemeData) -> Color,
        pressed: impl Fn(&ThemeData) -> Color,
        focus: impl Fn(&ThemeData) -> Color,
        fill: impl Fn(&ThemeData) -> Color,
    ) {
        let mut tester = tester();
        tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
        let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
        let theme = if m3_theme { m3(&tester) } else { m2(&tester) };

        tester.pump_widget(scaffold_app(
            theme.clone(),
            Radio::new(true)
                .focus_node(focus_node)
                .group_value(true)
                .on_changed(no_op()),
        ));

        // Hover
        let center = center_of_radio::<bool>(&tester);
        let mut gesture = WidgetTester::mouse(1, K_PRIMARY_BUTTON);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, center, false);
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[circle(hover(&theme)), circle(fill(&theme))],
        );

        // Highlighted (pressed).
        tester.send(&gesture, PointerChange::Down);
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[circle(pressed(&theme)), circle(fill(&theme))],
        );
        // Remove pressed and hovered states
        tester.up(&gesture);
        tester.pump_and_settle();
        tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
        tester.pump_and_settle();

        // Focused.
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        assert_paints(
            &tester,
            radio_material::<bool>(&tester),
            &[circle(focus(&theme)), circle(fill(&theme))],
        );
    }

    #[test]
    fn material2_the_default_overlay_and_fill_colors_resolve_the_pressed_state() {
        with_main_thread_stack(|| {
            default_overlay_and_fill_colors_resolve_the_pressed_state(
                false,
                |theme| theme.hover_color,
                |theme| {
                    theme
                        .color_scheme
                        .secondary
                        .with_alpha(K_RADIAL_REACTION_ALPHA)
                },
                |theme| theme.focus_color,
                |theme| theme.color_scheme.secondary,
            );
        });
    }

    #[test]
    fn material3_the_default_overlay_and_fill_colors_resolve_the_pressed_state() {
        with_main_thread_stack(|| {
            default_overlay_and_fill_colors_resolve_the_pressed_state(
                true,
                |theme| theme.color_scheme.primary.with_opacity(0.08),
                |theme| theme.color_scheme.on_surface.with_opacity(0.1),
                |theme| theme.color_scheme.primary.with_opacity(0.1),
                |theme| theme.color_scheme.primary,
            );
        });
    }

    /// Dart's `buildApp` of the cases on `backgroundColor`, `side` and `innerRadius`: a
    /// `Radio<int>` of value 0 under a `RadioGroup<int>` in a white box.
    fn group_app(
        theme: ThemeData,
        group_value: i32,
        configure: impl FnOnce(Radio<i32>) -> Radio<i32>,
    ) -> WidgetRef {
        centered_app(
            theme,
            white_box(
                RadioGroup::<i32>::new(no_op(), configure(Radio::new(0))).group_value(group_value),
            ),
        )
    }

    #[test]
    fn the_background_color_resolves_in_enabled_and_disabled_states() {
        with_main_thread_stack(|| {
            const ACTIVE_ENABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000004);
            let background_color = colors(|states| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return ACTIVE_DISABLED_BACKGROUND_COLOR;
                    }
                    return INACTIVE_DISABLED_BACKGROUND_COLOR;
                }
                if states.contains(&WidgetState::Selected) {
                    return ACTIVE_ENABLED_BACKGROUND_COLOR;
                }
                INACTIVE_ENABLED_BACKGROUND_COLOR
            });
            let mut tester = tester();
            let theme = m3(&tester);
            let mut check = |group_value: i32, enabled: bool, expected: Color| {
                let background_color = Rc::clone(&background_color);
                tester.pump_widget(group_app(theme.clone(), group_value, move |radio| {
                    radio.background_color(background_color).enabled(enabled)
                }));
                tester.pump_and_settle();
                assert_paints(
                    &tester,
                    radio_material::<i32>(&tester),
                    &[rect(Color::new(0xffffffff), CENTER_RECT), circle(expected)],
                );
            };

            // Selected and enabled.
            check(0, true, ACTIVE_ENABLED_BACKGROUND_COLOR);
            // Check when the radio isn't selected.
            check(1, true, INACTIVE_ENABLED_BACKGROUND_COLOR);
            // Check when the radio is selected, but disabled.
            check(0, false, ACTIVE_DISABLED_BACKGROUND_COLOR);
            // Check when the radio is unselected and disabled.
            check(1, false, INACTIVE_DISABLED_BACKGROUND_COLOR);
        });
    }

    /// The focused-then-hovered cases on `backgroundColor`, `side` and `innerRadius`: what the
    /// radio paints after its overlay when focused, and when hovered too.
    fn resolves_in_hovered_and_focused_states(
        configure: impl FnOnce(Radio<i32>) -> Radio<i32>,
        focused: &[Paints],
        hovered: &[Paints],
    ) {
        let mut tester = tester();
        tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
        let focus_node = FocusNode::new(&mut tester.cell.borrow_mut()).as_node();
        focus_node.set_debug_label(&mut tester.cell.borrow_mut(), Some("radio".into()));
        let theme = m3(&tester);
        let primary = theme.color_scheme.primary;

        tester.pump_widget(group_app(theme, 0, move |radio| {
            configure(radio.autofocus(true).focus_node(focus_node))
        }));
        tester.pump_and_settle();
        assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        let mut expected = vec![
            any_rect(),
            circle(primary.with_values(Some(0.1), None, None, None, None)),
        ];
        expected.extend_from_slice(focused);
        assert_paints(&tester, radio_material::<i32>(&tester), &expected);

        // Start hovering
        let center = center_of_radio::<i32>(&tester);
        hover_at(&mut tester, center);
        tester.pump_and_settle();
        let mut expected = vec![
            rect(Color::new(0xffffffff), CENTER_RECT),
            circle(primary.with_values(Some(0.08), None, None, None, None)),
        ];
        expected.extend_from_slice(hovered);
        assert_paints(&tester, radio_material::<i32>(&tester), &expected);
    }

    #[test]
    fn the_background_color_resolves_in_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            const HOVERED_BACKGROUND_COLOR: Color = Color::new(0xFF000001);
            const FOCUSED_BACKGROUND_COLOR: Color = Color::new(0xFF000002);
            let background_color = colors(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return HOVERED_BACKGROUND_COLOR;
                }
                if states.contains(&WidgetState::Focused) {
                    return FOCUSED_BACKGROUND_COLOR;
                }
                Colors::TRANSPARENT
            });
            resolves_in_hovered_and_focused_states(
                move |radio| radio.background_color(background_color),
                &[circle(FOCUSED_BACKGROUND_COLOR)],
                &[circle(HOVERED_BACKGROUND_COLOR)],
            );
        });
    }

    #[test]
    fn the_side_resolves_in_enabled_and_disabled_states() {
        with_main_thread_stack(|| {
            let active_enabled_side = BorderSide {
                color: Color::new(0xFF000001),
                ..BorderSide::default()
            };
            let active_disabled_side = BorderSide {
                color: Color::new(0xFF000002),
                width: 2.0,
                ..BorderSide::default()
            };
            let inactive_enabled_side = BorderSide {
                color: Color::new(0xFF000003),
                width: 3.0,
                ..BorderSide::default()
            };
            let inactive_disabled_side = BorderSide {
                color: Color::new(0xFF000004),
                width: 4.0,
                ..BorderSide::default()
            };
            let mut tester = tester();
            let theme = m3(&tester);
            let mut check = |group_value: i32, enabled: bool, expected: BorderSide| {
                let side = WidgetStateBorderSide::resolve_with(move |states| {
                    if states.contains(&WidgetState::Disabled) {
                        if states.contains(&WidgetState::Selected) {
                            return Some(active_disabled_side);
                        }
                        return Some(inactive_disabled_side);
                    }
                    if states.contains(&WidgetState::Selected) {
                        return Some(active_enabled_side);
                    }
                    Some(inactive_enabled_side)
                });
                tester.pump_widget(group_app(theme.clone(), group_value, move |radio| {
                    radio.side(side).enabled(enabled)
                }));
                tester.pump_and_settle();
                assert_paints(
                    &tester,
                    radio_material::<i32>(&tester),
                    &[
                        rect(Color::new(0xffffffff), CENTER_RECT),
                        circle(expected.color).stroke_width(expected.width),
                    ],
                );
            };

            // Selected and enabled.
            check(0, true, active_enabled_side);
            // Check when the radio isn't selected.
            check(1, true, inactive_enabled_side);
            // Check when the radio is selected, but disabled.
            check(0, false, active_disabled_side);
            // Check when the radio is unselected and disabled.
            check(1, false, inactive_disabled_side);
        });
    }

    #[test]
    fn the_side_resolves_in_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            let hovered_side = BorderSide {
                color: Color::new(0xFF000001),
                ..BorderSide::default()
            };
            let focused_side = BorderSide {
                color: Color::new(0xFF000002),
                width: 2.0,
                ..BorderSide::default()
            };
            let side = WidgetStateBorderSide::resolve_with(move |states| {
                if states.contains(&WidgetState::Hovered) {
                    return Some(hovered_side);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(focused_side);
                }
                None
            });
            resolves_in_hovered_and_focused_states(
                move |radio| radio.side(side),
                &[circle(focused_side.color).stroke_width(focused_side.width)],
                &[circle(hovered_side.color).stroke_width(hovered_side.width)],
            );
        });
    }

    #[test]
    fn the_inner_radius_resolves_in_enabled_and_disabled_states() {
        with_main_thread_stack(|| {
            const ENABLED_INNER_RADIUS: f64 = 1.0;
            const DISABLED_INNER_RADIUS: f64 = 2.0;
            let inner_radius = <dyn WidgetStateProperty<Option<f64>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(DISABLED_INNER_RADIUS);
                }
                Some(ENABLED_INNER_RADIUS)
            });
            let mut tester = tester();
            let theme = m3(&tester);
            let colors = theme.color_scheme.clone();
            let mut check = |enabled: bool, expected: &[Paints]| {
                let inner_radius = Rc::clone(&inner_radius);
                tester.pump_widget(group_app(theme.clone(), 0, move |radio| {
                    radio.inner_radius(inner_radius).enabled(enabled)
                }));
                tester.pump_and_settle();
                assert_paints(&tester, radio_material::<i32>(&tester), expected);
            };

            // Enabled.
            check(
                true,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(colors.primary),
                    circle(colors.primary).radius(ENABLED_INNER_RADIUS),
                ],
            );
            // Disabled.
            check(
                false,
                &[
                    rect(Color::new(0xffffffff), CENTER_RECT),
                    circle(colors.on_surface.with_alpha(97)),
                    circle(colors.on_surface.with_alpha(97)).radius(DISABLED_INNER_RADIUS),
                ],
            );
        });
    }

    #[test]
    fn the_inner_radius_resolves_in_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            const HOVERED_INNER_RADIUS: f64 = 1.0;
            const FOCUSED_INNER_RADIUS: f64 = 2.0;
            let inner_radius = <dyn WidgetStateProperty<Option<f64>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVERED_INNER_RADIUS);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(FOCUSED_INNER_RADIUS);
                }
                None
            });
            let primary = m3(&tester()).color_scheme.primary;
            resolves_in_hovered_and_focused_states(
                move |radio| radio.inner_radius(inner_radius),
                &[
                    circle(primary),
                    circle(primary).radius(FOCUSED_INNER_RADIUS),
                ],
                &[
                    circle(primary),
                    circle(primary).radius(HOVERED_INNER_RADIUS),
                ],
            );
        });
    }

    #[test]
    fn a_radio_takes_a_string_value_compared_with_eq() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let key = GlobalKey::new();
            let changes: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
            let build = |group_value: Option<&str>| {
                let changes = Rc::clone(&changes);
                let mut group = RadioGroup::<String>::new(
                    Rc::new(move |_app, value| changes.borrow_mut().push(value)),
                    Radio::new(String::from("jefferson"))
                        .key(key.clone().into())
                        .toggleable(true),
                );
                if let Some(group_value) = group_value {
                    group = group.group_value(String::from(group_value));
                }
                centered_app(theme.clone(), group)
            };

            tester.pump_widget(build(Some("lafayette")));
            tester.tap(tester.center_of(&key), 1);
            assert_eq!(*changes.borrow(), [Some(String::from("jefferson"))]);

            // An equal string that is another object selects the radio.
            tester.pump_widget(build(Some("jefferson")));
            tester.pump_and_settle();
            assert_eq!(
                painter::<String>(&tester)
                    .position(&tester.cell.borrow())
                    .value(&tester.cell.borrow()),
                1.0
            );

            // Selecting a toggleable radio again unselects it.
            tester.tap(tester.center_of(&key), 2);
            assert_eq!(*changes.borrow(), [Some(String::from("jefferson")), None]);
        });
    }

    #[test]
    fn a_radio_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            tester.pump_widget(scaffold_app(
                theme,
                Center::new().child(SizedBox::shrink().child(Radio::new(true))),
            ));
            let render = tester.render_box_of(find_radio::<bool>(&tester));
            assert_eq!(render.size(&tester.cell.borrow()), Size::ZERO);
        });
    }
}

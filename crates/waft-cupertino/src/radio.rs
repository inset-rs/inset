//! Flutter counterpart: `cupertino_ui/radio.dart`.
//!
//! [`CupertinoRadio`] builds a [`RawRadio`], whose builder hands `_RadioPaint` the radio's state
//! as an [`AnyToggleableState`]; the paint state keeps its arena [`ToggleablePainter`] for its
//! lifetime, as Dart's `_painter` field is. The deprecated `groupValue` and `onChanged` reach the
//! radio through `_RadioRegistry`, an arena object the state creates on first use. The
//! `Semantics` wrapper waits with accessibility (in `RawRadio`).

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::valo::Cap;
use waft_embedder::{
    Brightness, Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, Rect, Size, Stroke,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, OptionRetained, Ref, ValueChanged,
};
use waft_painting::{AlignmentGeometry, AnyColor, Gradient, HSLColor, LinearGradient};
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    AnyRadioClient, AnyRadioGroupRegistry, AnyToggleableState, BuildContext, CustomPaint,
    FocusNode, FocusNodeLeaf, FocusNodeRef, IntoWidget, KeyRef, RadioBuilder, RadioGroup,
    RadioGroupRegistry, RawRadio, State, StateData, StatefulWidget, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::constants::{
    K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS, K_CUPERTINO_FOCUS_COLOR_OPACITY,
    K_CUPERTINO_FOCUS_COLOR_SATURATION,
};
use crate::theme::CupertinoTheme;

const SIZE: Size = Size::new(18.0, 18.0);
const K_OUTER_RADIUS: f64 = 7.0;
const K_INNER_RADIUS: f64 = 2.975;

// Eyeballed from a radio on a physical Macbook Pro running macOS version 14.5.
/// Dart's `final Color _kDisabledOuterColor`.
fn k_disabled_outer_color() -> Color {
    with_opacity(CupertinoColors::WHITE.color(), 0.50)
}
const K_DISABLED_INNER_COLOR: CupertinoDynamicColor = CupertinoDynamicColor::with_brightness(
    Color::from_argb(64, 0, 0, 0),
    Color::from_argb(64, 255, 255, 255),
);
const K_DISABLED_BORDER_COLOR: CupertinoDynamicColor = CupertinoDynamicColor::with_brightness(
    Color::from_argb(64, 0, 0, 0),
    Color::from_argb(64, 0, 0, 0),
);
const K_DEFAULT_BORDER_COLOR: CupertinoDynamicColor = CupertinoDynamicColor::with_brightness(
    Color::from_argb(255, 209, 209, 214),
    Color::from_argb(64, 0, 0, 0),
);

/// Dart's `_kDefaultInnerColor`: its light colour is `CupertinoColors.white`.
fn k_default_inner_color() -> AnyColor {
    CupertinoDynamicColor::with_brightness(
        CupertinoColors::WHITE.color(),
        Color::from_argb(255, 222, 232, 248),
    )
    .into_any()
}

/// Dart's `_kDefaultOuterColor`: its light colour is `CupertinoColors.activeBlue`'s.
fn k_default_outer_color() -> AnyColor {
    CupertinoDynamicColor::with_brightness(
        CupertinoColors::ACTIVE_BLUE.color(),
        Color::from_argb(255, 50, 100, 215),
    )
    .into_any()
}

const K_PRESSED_OVERLAY_OPACITY: f64 = 0.15;
const K_CHECKMARK_STROKE_WIDTH: f64 = 2.0;
const K_FOCUS_OUTLINE_STROKE_WIDTH: f64 = 3.0;
const K_BORDER_OUTLINE_STROKE_WIDTH: f64 = 0.3;
// In dark mode, the outer color of a radio is an opacity gradient of the
// background color.
const K_DARK_GRADIENT_OPACITIES: [f64; 2] = [0.14, 0.29];
const K_DISABLED_DARK_GRADIENT_OPACITIES: [f64; 2] = [0.08, 0.14];

/// Dart's deprecated `Color.withOpacity`, deprecated here too: its rounding to an 8-bit alpha
/// is part of the behaviour being ported.
#[allow(deprecated)]
fn with_opacity(color: Color, opacity: f64) -> Color {
    color.with_opacity(opacity)
}

/// A widget that builds a [`RawRadio`] with a macOS-style UI.
///
/// Used to select between a number of mutually exclusive values. When one radio button in a
/// group is selected, the other radio buttons in the group are deselected. The values are of
/// type `T`, the type parameter of the [`CupertinoRadio`] class. Enums are commonly used for
/// this purpose.
///
/// This widget typically has a [`RadioGroup`] ancestor, which takes in a
/// [`RadioGroup::group_value`], and the [`CupertinoRadio`] under it with matching
/// [`value`](Self::value) will be selected.
///
/// See also:
///
///  * [`CupertinoSlider`](crate::CupertinoSlider), for selecting a value in a range.
///  * [`CupertinoCheckbox`](crate::CupertinoCheckbox) and
///    [`CupertinoSwitch`](crate::CupertinoSwitch), for toggling a particular value on or off.
///  * `Radio`, the Material Design equivalent.
///  * <https://developer.apple.com/design/human-interface-guidelines/components/selection-and-input/toggles/>
#[derive(Clone)]
pub struct CupertinoRadio<T: Clone + PartialEq + 'static> {
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
    pub on_changed: Option<ValueChanged<Option<T>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then [`SystemMouseCursors::BASIC`] is used when this radio button is disabled.
    /// When this radio button is enabled, [`SystemMouseCursors::CLICK`] is used on Web, and
    /// [`SystemMouseCursors::BASIC`] is used on other platforms.
    ///
    /// See also:
    ///
    ///  * `WidgetStateMouseCursor`, a mouse cursor that implements `WidgetStateProperty` which
    ///    is used in APIs that need to accept either a mouse cursor or a
    ///    `WidgetStateProperty`.
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
    /// Controls whether the radio displays in a checkbox style or the default iOS radio style.
    ///
    /// Defaults to false.
    pub use_checkmark_style: bool,
    /// The color to use when this radio button is selected.
    ///
    /// Defaults to [`CupertinoColors::ACTIVE_BLUE`].
    pub active_color: Option<Color>,
    /// The color to use when this radio button is not selected.
    ///
    /// Defaults to [`CupertinoColors::WHITE`].
    pub inactive_color: Option<Color>,
    /// The color that fills the inner circle of the radio button when selected.
    ///
    /// Defaults to [`CupertinoColors::WHITE`].
    pub fill_color: Option<Color>,
    /// The color for the radio's border when it has the input focus.
    ///
    /// If null, then a paler form of the [`active_color`](Self::active_color) will be used.
    pub focus_color: Option<Color>,
    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed by
    /// this widget. The widget will be focusable even if a `focus_node` is not supplied. If
    /// supplied, the given `focus_node` will be _hosted_ by this widget, but not owned.
    pub focus_node: Option<FocusNodeRef>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// The registry this radio registers to.
    ///
    /// Unless provided, the [`BuildContext`] will be used to look up the ancestor
    /// [`RadioGroupRegistry`].
    pub group_registry: Option<AnyRadioGroupRegistry<T>>,
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
}

impl<T: Clone + PartialEq + 'static> CupertinoRadio<T> {
    /// Creates a macOS-styled radio button.
    ///
    /// The following arguments are required:
    ///
    /// * [`value`](Self::value) and [`group_value`](Self::group_value) together determine
    ///   whether the radio button is selected.
    /// * [`on_changed`](Self::on_changed) is called when the user selects this radio button.
    pub fn new(value: T) -> CupertinoRadio<T> {
        CupertinoRadio {
            key: None,
            value,
            group_value: None,
            on_changed: None,
            mouse_cursor: None,
            toggleable: false,
            use_checkmark_style: false,
            active_color: None,
            inactive_color: None,
            fill_color: None,
            focus_color: None,
            focus_node: None,
            autofocus: false,
            group_registry: None,
            enabled: None,
        }
    }

    /// Dart `CupertinoRadio(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoRadio<T> {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoRadio(groupValue:)`.
    #[deprecated(
        note = "Use a RadioGroup ancestor to manage group value instead. This feature was \
                deprecated after v3.32.0-0.0.pre."
    )]
    pub fn group_value(mut self, group_value: T) -> CupertinoRadio<T> {
        self.group_value = Some(group_value);
        self
    }

    /// Dart `CupertinoRadio(onChanged:)`.
    #[deprecated(
        note = "Use RadioGroup to handle value change instead. This feature was deprecated \
                after v3.32.0-0.0.pre."
    )]
    pub fn on_changed(mut self, on_changed: ValueChanged<Option<T>>) -> CupertinoRadio<T> {
        self.on_changed = Some(on_changed);
        self
    }

    /// Dart `CupertinoRadio(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> CupertinoRadio<T> {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CupertinoRadio(toggleable:)`.
    pub fn toggleable(mut self, toggleable: bool) -> CupertinoRadio<T> {
        self.toggleable = toggleable;
        self
    }

    /// Dart `CupertinoRadio(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> CupertinoRadio<T> {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `CupertinoRadio(inactiveColor:)`.
    pub fn inactive_color(mut self, inactive_color: Color) -> CupertinoRadio<T> {
        self.inactive_color = Some(inactive_color);
        self
    }

    /// Dart `CupertinoRadio(fillColor:)`.
    pub fn fill_color(mut self, fill_color: Color) -> CupertinoRadio<T> {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `CupertinoRadio(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> CupertinoRadio<T> {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `CupertinoRadio(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> CupertinoRadio<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoRadio(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoRadio<T> {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoRadio(useCheckmarkStyle:)`.
    pub fn use_checkmark_style(mut self, use_checkmark_style: bool) -> CupertinoRadio<T> {
        self.use_checkmark_style = use_checkmark_style;
        self
    }

    /// Dart `CupertinoRadio(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> CupertinoRadio<T> {
        self.enabled = Some(enabled);
        self
    }

    /// Dart `CupertinoRadio(groupRegistry:)`.
    pub fn group_registry(mut self, group_registry: AnyRadioGroupRegistry<T>) -> CupertinoRadio<T> {
        self.group_registry = Some(group_registry);
        self
    }
}

impl<T: Clone + PartialEq + 'static> Debug for CupertinoRadio<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoRadio")
            .field("toggleable", &self.toggleable)
            .field("enabled", &self.enabled)
            .field("group_registry", &self.group_registry)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for CupertinoRadio<T> {
    type State = CupertinoRadioState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoRadioState<T> {
        CupertinoRadioState {
            state: StateData::new(),
            internal_focus_node: None,
            internal_radio_registry: None,
        }
    }
}

/// Dart's `_CupertinoRadioState`.
pub struct CupertinoRadioState<T: Clone + PartialEq + 'static> {
    state: StateData<CupertinoRadio<T>>,
    internal_focus_node: Option<Ref<FocusNode>>,
    internal_radio_registry: Option<Ref<RadioRegistry<T>>>,
}

impl<T: Clone + PartialEq + 'static> CupertinoRadioState<T> {
    fn effective_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            return focus_node;
        }
        if let Some(focus_node) = &app.get(self).internal_focus_node {
            return focus_node.erase();
        }
        let focus_node = FocusNode::new(app);
        app.get_mut(self).internal_focus_node = Some(focus_node.clone());
        focus_node.into_erased()
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
        let registry = match app.get(self).internal_radio_registry.handle() {
            Some(registry) => registry,
            None => {
                let registry = app.create(RadioRegistry { state: self });
                let handle = registry.handle();
                app.get_mut(self).internal_radio_registry = Some(registry);
                handle
            }
        };
        registry.as_radio_group_registry()
    }

    /// Dart's `effectiveMouseCursor`, built from the widget's cursor.
    fn effective_mouse_cursor(
        mouse_cursor: Option<MouseCursorRef>,
    ) -> WidgetStatePropertyRef<MouseCursorRef> {
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(move |states: &WidgetStates| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                .unwrap_or_else(|| {
                    if !states.contains(&WidgetState::Disabled) && K_IS_WEB {
                        SystemMouseCursors::CLICK.into()
                    } else {
                        SystemMouseCursors::BASIC.into()
                    }
                })
        })
    }

    /// The `builder` Dart passes `RawRadio`: it reads the state's widget when the raw radio
    /// builds, as the Dart closure does.
    fn radio_builder(self: Handle<Self>) -> RadioBuilder {
        Rc::new(move |app: &mut App, _context, state: AnyToggleableState| {
            let widget = self.widget(app);
            let active_color = widget.active_color;
            let inactive_color = widget.inactive_color;
            let fill_color = widget.fill_color;
            let focus_color = widget.focus_color;
            let use_checkmark_style = widget.use_checkmark_style;
            let is_active = self.enabled(app);
            let focused = self.effective_focus_node(app).has_focus(app);
            RadioPaint {
                active_color,
                inactive_color,
                fill_color,
                focus_color,
                use_checkmark_style,
                is_active,
                toggleable_state: state,
                focused,
            }
            .into_widget()
        })
    }
}

impl<T: Clone + PartialEq + 'static> State for CupertinoRadioState<T> {
    type Widget = CupertinoRadio<T>;
    waft_widgets::state_accessors!();

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(focus_node) = app.get(self).internal_focus_node.handle() {
            focus_node.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            !self.widget(app).enabled.unwrap_or(false)
                || self.widget(app).on_changed.is_some()
                || self.widget(app).group_registry.is_some()
                || RadioGroup::<T>::maybe_of(app, context).is_some(),
            "Radio is enabled but has no CupertinoRadio.onChange, CupertinoRadio.groupRegistry, \
             or RadioGroup above"
        );
        let effective_mouse_cursor =
            Self::effective_mouse_cursor(self.widget(app).mouse_cursor.clone());

        let value = self.widget(app).value.clone();
        let group_registry = self.effective_registry(app);
        let toggleable = self.widget(app).toggleable;
        let focus_node = self.effective_focus_node(app);
        let autofocus = self.widget(app).autofocus;
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
    state: Handle<CupertinoRadioState<T>>,
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
    inactive_color: Option<Color>,
    fill_color: Option<Color>,
    focus_color: Option<Color>,
    use_checkmark_style: bool,
    is_active: bool,
    focused: bool,
}

impl Debug for RadioPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioPaint")
            .field("toggleable_state", &self.toggleable_state)
            .field("is_active", &self.is_active)
            .field("focused", &self.focused)
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
    painter: Option<Ref<RadioPainter>>,
}

impl RadioPaintState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<RadioPainter> {
        app.get(self)
            .painter
            .handle()
            .expect("created in init_state")
    }

    /// Dart's `_defaultOuterColor` resolved in `states`: its closure resolves a dynamic colour
    /// against the state's context, which needs the `App`, so the property is this method.
    fn default_outer_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        states: &WidgetStates,
    ) -> Color {
        if states.contains(&WidgetState::Disabled) {
            return CupertinoDynamicColor::resolve(&k_disabled_outer_color().into(), app, context)
                .color();
        }
        if states.contains(&WidgetState::Selected) {
            return self.widget(app).active_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(&k_default_outer_color(), app, context).color()
            });
        }
        self.widget(app)
            .inactive_color
            .unwrap_or_else(|| CupertinoColors::WHITE.color())
    }

    /// Dart's `_defaultInnerColor` resolved in `states`, a method for the reason
    /// [`default_outer_color`](Self::default_outer_color) is.
    fn default_inner_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        states: &WidgetStates,
    ) -> Color {
        if states.contains(&WidgetState::Disabled) && states.contains(&WidgetState::Selected) {
            return self.widget(app).fill_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(&K_DISABLED_INNER_COLOR.to_any(), app, context)
                    .color()
            });
        }
        if states.contains(&WidgetState::Selected) {
            return self.widget(app).fill_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(&k_default_inner_color(), app, context).color()
            });
        }
        CupertinoColors::WHITE.color()
    }

    /// Dart's `_defaultBorderColor` resolved in `states`, a method for the reason
    /// [`default_outer_color`](Self::default_outer_color) is.
    fn default_border_color(app: &mut App, context: BuildContext, states: &WidgetStates) -> Color {
        if (states.contains(&WidgetState::Selected) || states.contains(&WidgetState::Focused))
            && !states.contains(&WidgetState::Disabled)
        {
            return CupertinoColors::TRANSPARENT.color();
        }
        if states.contains(&WidgetState::Disabled) {
            return CupertinoDynamicColor::resolve(&K_DISABLED_BORDER_COLOR.to_any(), app, context)
                .color();
        }
        CupertinoDynamicColor::resolve(&K_DEFAULT_BORDER_COLOR.to_any(), app, context).color()
    }
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
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let toggleable_state = self.widget(app).toggleable_state;
        // Colors need to be resolved in selected and non selected states separately.
        let mut active_states = toggleable_state.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = toggleable_state.states(app);
        inactive_states.remove(&WidgetState::Selected);

        // Since the states getter always makes a new set, make a copy to use
        // throughout the lifecycle of this build method.
        let current_states = toggleable_state.states(app);

        let effective_active_color = self.default_outer_color(app, context, &active_states);

        let effective_inactive_color = self.default_outer_color(app, context, &inactive_states);

        let effective_focus_overlay_color = self.widget(app).focus_color.unwrap_or_else(|| {
            HSLColor::from_color(with_opacity(
                effective_active_color,
                K_CUPERTINO_FOCUS_COLOR_OPACITY,
            ))
            .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
            .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
            .to_color()
        });

        let effective_fill_color = self.default_inner_color(app, context, &current_states);

        let effective_border_color = Self::default_border_color(app, context, &current_states);

        let painter = app
            .get(self)
            .painter
            .clone()
            .expect("created in init_state");
        painter.set_position(app, toggleable_state.position(app).erase());
        painter.set_reaction(app, toggleable_state.reaction(app).erase());
        painter.set_focus_color(app, effective_focus_overlay_color);
        painter.set_down_position(app, toggleable_state.down_position(app));
        painter.set_is_focused(app, Some(self.widget(app).focused));
        painter.set_active_color(app, effective_active_color);
        painter.set_inactive_color(app, effective_inactive_color);
        painter.set_fill_color(app, effective_fill_color);
        painter.set_value(app, toggleable_state.value(app));
        painter.set_checkmark_style(app, self.widget(app).use_checkmark_style);
        painter.set_is_active(app, Some(self.widget(app).is_active));
        painter.set_border_color(app, effective_border_color);
        let brightness = CupertinoTheme::of(app, context).brightness();
        painter.set_brightness(app, brightness);

        CustomPaint::new()
            .size(SIZE)
            .painter(ToggleablePainterRef::new(painter))
            .into_widget()
    }
}

/// Dart's `_RadioPainter`.
struct RadioPainter {
    change_notifier: ChangeNotifierData,
    toggleable_painter: ToggleablePainterData,
    value: Option<bool>,
    fill_color: Option<Color>,
    checkmark_style: bool,
    brightness: Option<Brightness>,
    border_color: Option<Color>,
}

impl RadioPainter {
    fn new(app: &mut App) -> Ref<RadioPainter> {
        app.create(RadioPainter {
            change_notifier: ChangeNotifierData::new(),
            toggleable_painter: ToggleablePainterData::new(),
            value: None,
            fill_color: None,
            checkmark_style: false,
            brightness: None,
            border_color: None,
        })
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

    fn fill_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).fill_color.expect("set before painting")
    }

    fn set_fill_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).fill_color {
            return;
        }
        app.get_mut(self).fill_color = Some(value);
        self.notify_listeners(app);
    }

    fn checkmark_style(self: Handle<Self>, app: &App) -> bool {
        app.get(self).checkmark_style
    }

    fn set_checkmark_style(self: Handle<Self>, app: &mut App, value: bool) {
        if value == app.get(self).checkmark_style {
            return;
        }
        app.get_mut(self).checkmark_style = value;
        self.notify_listeners(app);
    }

    fn brightness(self: Handle<Self>, app: &App) -> Option<Brightness> {
        app.get(self).brightness
    }

    fn set_brightness(self: Handle<Self>, app: &mut App, value: Option<Brightness>) {
        if app.get(self).brightness == value {
            return;
        }
        app.get_mut(self).brightness = value;
        self.notify_listeners(app);
    }

    fn border_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).border_color.expect("set before painting")
    }

    fn set_border_color(self: Handle<Self>, app: &mut App, value: Color) {
        if app.get(self).border_color == Some(value) {
            return;
        }
        app.get_mut(self).border_color = Some(value);
        self.notify_listeners(app);
    }

    fn draw_pressed_overlay(self: Handle<Self>, app: &App, canvas: &mut Canvas, center: Offset) {
        let pressed_paint = Paint::from_color(
            if self.brightness(app) == Some(Brightness::Light) {
                with_opacity(CupertinoColors::BLACK.color(), K_PRESSED_OVERLAY_OPACITY)
            } else {
                with_opacity(CupertinoColors::WHITE.color(), K_PRESSED_OVERLAY_OPACITY)
            }
            .into(),
        );
        canvas.draw_circle(center, K_OUTER_RADIUS as f32, &pressed_paint);
    }

    fn draw_fill_gradient(
        canvas: &mut Canvas,
        center: Offset,
        radius: f64,
        top_color: Color,
        bottom_color: Color,
    ) {
        let fill_gradient = LinearGradient::new(vec![top_color, bottom_color])
            .begin(AlignmentGeometry::TOP_CENTER)
            .end(AlignmentGeometry::BOTTOM_CENTER);
        let circle_rect = Rect::from_circle(center, radius);
        let gradient_paint = Paint {
            shader: Some(fill_gradient.create_shader(circle_rect, None)),
            ..Paint::default()
        };
        let mut path = PathBuilder::new();
        path.circle(circle_rect.center(), radius as f32);
        canvas.draw_path(&path.build(), FillRule::NonZero, &gradient_paint);
    }

    fn draw_outer_border(self: Handle<Self>, app: &App, canvas: &mut Canvas, center: Offset) {
        let border_paint = Paint {
            color: self.border_color(app).into(),
            style: PaintStyle::Stroke(Stroke::new(K_BORDER_OUTLINE_STROKE_WIDTH as f32)),
            ..Paint::default()
        };
        canvas.draw_circle(center, K_OUTER_RADIUS as f32, &border_paint);
    }

    fn draw_checkmark(self: Handle<Self>, app: &App, canvas: &mut Canvas, center: Offset) {
        let mut path = PathBuilder::new();
        let mut stroke = Stroke::new(K_CHECKMARK_STROKE_WIDTH as f32);
        stroke.cap = Cap::Round;
        let check_paint = Paint {
            color: self.active_color(app).into(),
            style: PaintStyle::Stroke(stroke),
            ..Paint::default()
        };
        let width = SIZE.width();
        let origin = Offset::new(center.dx() - (width / 2.0), center.dy() - (width / 2.0));
        let start = Offset::new(width * 0.25, width * 0.52);
        let mid = Offset::new(width * 0.46, width * 0.75);
        let end = Offset::new(width * 0.85, width * 0.29);
        path.move_to(Offset::new(
            origin.dx() + start.dx(),
            origin.dy() + start.dy(),
        ));
        path.line_to(Offset::new(origin.dx() + mid.dx(), origin.dy() + mid.dy()));
        canvas.draw_path(&path.clone().build(), FillRule::NonZero, &check_paint);
        path.move_to(Offset::new(origin.dx() + mid.dx(), origin.dy() + mid.dy()));
        path.line_to(Offset::new(origin.dx() + end.dx(), origin.dy() + end.dy()));
        canvas.draw_path(&path.build(), FillRule::NonZero, &check_paint);
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
        let center = (Offset::ZERO & size).center();
        let is_active = self.is_active(app);
        let opacities = if is_active {
            K_DARK_GRADIENT_OPACITIES
        } else {
            K_DISABLED_DARK_GRADIENT_OPACITIES
        };

        if self.checkmark_style(app) {
            if self.value(app).unwrap_or(false) {
                self.draw_checkmark(app, canvas, center);
            }
        } else if self.value(app).unwrap_or(false) {
            let outer_color = self.active_color(app);
            // Draw a gradient in dark mode if the radio is disabled.
            if self.brightness(app) == Some(Brightness::Dark) && !is_active {
                Self::draw_fill_gradient(
                    canvas,
                    center,
                    K_OUTER_RADIUS,
                    with_opacity(outer_color, opacities[0]),
                    with_opacity(outer_color, opacities[1]),
                );
            } else {
                canvas.draw_circle(
                    center,
                    K_OUTER_RADIUS as f32,
                    &Paint::from_color(outer_color.into()),
                );
            }
            // The outer circle's opacity changes when the radio is pressed.
            if self.down_position(app).is_some() {
                self.draw_pressed_overlay(app, canvas, center);
            }
            let inner_paint = Paint::from_color(self.fill_color(app).into());
            canvas.draw_circle(center, K_INNER_RADIUS as f32, &inner_paint);
            // Draw an outer border if the radio is disabled and selected.
            if !is_active {
                self.draw_outer_border(app, canvas, center);
            }
        } else {
            let color = if is_active {
                self.inactive_color(app)
            } else {
                k_disabled_outer_color()
            };
            if self.brightness(app) == Some(Brightness::Dark) {
                Self::draw_fill_gradient(
                    canvas,
                    center,
                    K_OUTER_RADIUS,
                    with_opacity(color, opacities[0]),
                    with_opacity(color, opacities[1]),
                );
            } else {
                canvas.draw_circle(
                    center,
                    K_OUTER_RADIUS as f32,
                    &Paint::from_color(color.into()),
                );
            }
            // The entire circle's opacity changes when the radio is pressed.
            if self.down_position(app).is_some() {
                self.draw_pressed_overlay(app, canvas, center);
            }
            self.draw_outer_border(app, canvas, center);
        }
        if self.is_focused(app) {
            let focus_paint = Paint {
                color: self.focus_color(app).into(),
                style: PaintStyle::Stroke(Stroke::new(K_FOCUS_OUTLINE_STROKE_WIDTH as f32)),
                ..Paint::default()
            };
            canvas.draw_circle(
                center,
                (K_OUTER_RADIUS + K_FOCUS_OUTLINE_STROKE_WIDTH / 2.0) as f32,
                &focus_paint,
            );
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use waft_embedder::valo::{Op, Shader};
    use waft_embedder::{
        KeyData, KeyEventDeviceType, KeyEventType, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind, TargetPlatform,
    };
    use waft_foundation::AppCell;
    use waft_gestures::GestureBinding;
    use waft_painting::PaintingBinding;
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject, RendererBinding,
    };
    use waft_services::{KeyEventManager, LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_test::TestPlatform;
    use waft_widgets::{
        Center, FocusHighlightStrategy, FocusManager, GlobalKey, Row, SizedBox,
        WidgetStateMouseCursor,
    };

    use super::*;
    use crate::app::CupertinoApp;
    use crate::test_support::{build, pump, test_view};
    use crate::theme::CupertinoThemeData;

    /// The test view is 400x300 logical pixels; a centred radio sits at its centre.
    const VIEW_CENTER: Offset = Offset::new(200.0, 150.0);

    const INNER_RADIUS: f32 = 2.975;
    const OUTER_RADIUS: f32 = 7.0;

    /// Dart's `WidgetTester` over a `CupertinoApp` on Android, flutter_test's default platform.
    struct Tester {
        cell: Rc<AppCell>,
        clock: Duration,
        host: Option<Handle<HostState>>,
    }

    /// The app's home, standing in for a second `pumpWidget(CupertinoApp(home: ..))`: a
    /// `CupertinoApp` rebuilt with a new `home` keeps showing the old one here, so a test
    /// swaps the home's content instead, and the app's `theme` is a `CupertinoTheme` around it.
    struct Host {
        key: KeyRef,
        theme: Option<CupertinoThemeData>,
        child: WidgetRef,
    }

    impl Debug for Host {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Host")
        }
    }

    impl StatefulWidget for Host {
        type State = HostState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> HostState {
            HostState {
                state: StateData::new(),
                theme: self.theme.clone(),
                child: self.child.clone(),
            }
        }
    }

    struct HostState {
        state: StateData<Host>,
        theme: Option<CupertinoThemeData>,
        child: WidgetRef,
    }

    impl State for HostState {
        type Widget = Host;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let centered = Center::new()
                .child(app.get(self).child.clone())
                .into_widget();
            match app.get(self).theme.clone() {
                Some(theme) => CupertinoTheme::new(theme, centered).into_widget(),
                None => centered,
            }
        }
    }

    impl Tester {
        fn new() -> Tester {
            let platform = TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_view(test_view());
            let cell = AppCell::with_platform(Rc::new(platform));
            install_fonts(&mut cell.borrow_mut());
            Tester {
                cell,
                clock: Duration::ZERO,
                host: None,
            }
        }

        /// Dart's `pumpWidget(CupertinoApp(theme:, home: Center(child: child)))`.
        fn pump_app(&mut self, theme: Option<CupertinoThemeData>, child: WidgetRef) {
            match self.host {
                Some(host) => {
                    host.set_state(&mut self.cell.borrow_mut(), |host| {
                        host.theme = theme;
                        host.child = child;
                    });
                }
                None => {
                    let key = GlobalKey::new();
                    let home = Host {
                        key: key.clone().into(),
                        theme,
                        child,
                    };
                    build(&self.cell, CupertinoApp::new().home(home).into_widget());
                    self.host = key.current_state::<HostState>(&self.cell.borrow_mut());
                }
            }
            self.pump();
        }

        fn pump(&mut self) {
            self.clock += Duration::from_millis(16);
            pump(&mut self.cell.borrow_mut(), self.clock);
        }

        /// Dart's `pumpAndSettle`: frames until no animation is left.
        fn settle(&mut self) {
            for _ in 0..60 {
                self.pump();
            }
        }

        fn highlight_traditionally(&self) {
            let mut app = self.cell.borrow_mut();
            let manager = FocusManager::instance(&mut app);
            manager.set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        }

        /// The test view is 2x: logical coordinates double into the packet.
        fn send(&mut self, change: PointerChange, kind: PointerDeviceKind, at: Offset) {
            self.clock += Duration::from_millis(10);
            let mut app = self.cell.borrow_mut();
            GestureBinding::instance(&mut app).handle_pointer_data_packet(
                &mut app,
                PointerDataPacket::new(vec![PointerData {
                    change,
                    kind,
                    time_stamp: self.clock,
                    device: 1,
                    pointer_identifier: 1,
                    buttons: if matches!(change, PointerChange::Down | PointerChange::Move) {
                        1
                    } else {
                        0
                    },
                    physical_x: at.dx() * 2.0,
                    physical_y: at.dy() * 2.0,
                    ..PointerData::default()
                }]),
            );
            app.drain_microtasks();
        }

        /// Dart's `tester.startGesture` at the view's centre, then `pump`.
        fn press(&mut self) {
            self.send(PointerChange::Down, PointerDeviceKind::Touch, VIEW_CENTER);
            self.pump();
        }

        fn release(&mut self) {
            self.send(PointerChange::Up, PointerDeviceKind::Touch, VIEW_CENTER);
            self.pump();
        }

        /// Dart's `tester.tap` at the radio's centre.
        fn tap(&mut self) {
            self.send(PointerChange::Down, PointerDeviceKind::Touch, VIEW_CENTER);
            self.send(PointerChange::Up, PointerDeviceKind::Touch, VIEW_CENTER);
            self.pump();
        }

        /// A mouse that arrives over the radio's centre.
        fn hover(&mut self) {
            self.send(PointerChange::Add, PointerDeviceKind::Mouse, VIEW_CENTER);
            self.send(PointerChange::Hover, PointerDeviceKind::Mouse, VIEW_CENTER);
            self.pump();
        }

        fn cursor(&self) -> Option<MouseCursorRef> {
            let mut app = self.cell.borrow_mut();
            RendererBinding::instance(&mut app)
                .mouse_tracker(&app)
                .debug_device_active_cursor(&app, 1)
        }

        /// Dart's `tester.sendKeyEvent`: one key down and up, as the shell delivers it.
        fn send_key_event(&mut self, physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) {
            for event_type in [KeyEventType::Down, KeyEventType::Up] {
                let mut app = self.cell.borrow_mut();
                KeyEventManager::instance(&mut app).handle_key_data(
                    &mut app,
                    KeyData {
                        time_stamp: Duration::ZERO,
                        event_type,
                        device_type: KeyEventDeviceType::Keyboard,
                        physical: physical.usb_hid_usage,
                        logical: logical.key_id,
                        character: None,
                        synthesized: false,
                    },
                );
                app.drain_microtasks();
            }
        }

        fn custom_paint(&self, key: &GlobalKey) -> RenderHandle<RenderCustomPaint> {
            let app = self.cell.borrow();
            let root = key
                .current_context(&app)
                .expect("the radio mounted")
                .find_render_object(&app)
                .expect("the radio has been laid out");
            find::<RenderCustomPaint>(&app, root).expect("the radio's paint")
        }

        /// Dart's `find.byType(CupertinoRadio)` with a `paints` matcher: what the radio's
        /// painter draws, in paint order.
        fn drawn(&self, key: &GlobalKey) -> Vec<Drawn> {
            let custom = self.custom_paint(key);
            let mut app = self.cell.borrow_mut();
            let painter = custom.painter(&app).expect("the radio's painter");
            let size = custom.size(&app);
            let mut canvas = Canvas::new();
            painter.paint(&mut app, &mut canvas, size);
            canvas.build().ops().iter().filter_map(Drawn::of).collect()
        }

        /// The radio's painter, for a colour whose draw valo drops because it is transparent.
        fn painter(&self, key: &GlobalKey) -> Handle<RadioPainter> {
            let custom = self.custom_paint(key);
            let app = self.cell.borrow();
            let painter = custom.painter(&app).expect("the radio's painter");
            painter
                .as_any()
                .downcast_ref::<ToggleablePainterRef<RadioPainter>>()
                .expect("a radio painter")
                .painter()
        }
    }

    /// What the shell does at start-up: the app-wide fonts, which the debug banner needs.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
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

    /// One recorded path, reduced to what Dart's `paints` matchers compare. A draw with a
    /// transparent paint is not recorded at all.
    #[derive(Clone, Debug, PartialEq)]
    enum Drawn {
        /// A filled circle: its radius and its colour as `0xAARRGGBB`.
        Circle(f32, u32),
        /// A stroked circle: its radius, stroke width and colour.
        StrokedCircle(f32, f32, u32),
        /// A fill through a linear gradient: its stop colours, top to bottom.
        Gradient(Vec<u32>),
        /// Any other path (Dart's `path()`): its stroke width, 0 for a fill.
        Path(f32),
    }

    fn argb(color: waft_embedder::valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    impl Drawn {
        fn of(op: &Op) -> Option<Drawn> {
            let Op::DrawPath { path, paint, .. } = op else {
                return None;
            };
            let bounds = path.bounds();
            let is_circle = (bounds.width - bounds.height).abs() < 1e-3;
            let radius = (bounds.width / 2.0 * 1000.0).round() / 1000.0;
            Some(match &paint.style {
                PaintStyle::Fill if paint.shader.is_some() => match &paint.shader {
                    Some(Shader::Linear { stops, .. }) => {
                        Drawn::Gradient(stops.iter().map(|stop| argb(stop.color)).collect())
                    }
                    _ => return None,
                },
                PaintStyle::Fill if is_circle => Drawn::Circle(radius, argb(paint.color)),
                PaintStyle::Stroke(stroke) if is_circle => {
                    Drawn::StrokedCircle(radius, stroke.width, argb(paint.color))
                }
                PaintStyle::Stroke(stroke) => Drawn::Path(stroke.width),
                PaintStyle::Fill => Drawn::Path(0.0),
            })
        }
    }

    fn radio(value: i32) -> CupertinoRadio<i32> {
        CupertinoRadio::new(value)
    }

    fn log_into(log: &Rc<RefCell<Vec<Option<i32>>>>) -> ValueChanged<Option<i32>> {
        let log = Rc::clone(log);
        Rc::new(move |_app, value| log.borrow_mut().push(value))
    }

    fn keyed(radio: CupertinoRadio<i32>, key: &GlobalKey) -> WidgetRef {
        radio.key(key.clone().into()).into_widget()
    }

    fn dark() -> Option<CupertinoThemeData> {
        Some(CupertinoThemeData::new().with_brightness(Brightness::Dark))
    }

    #[test]
    fn a_radio_reports_its_value_when_tapped_unless_selected_or_without_on_changed() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();

        tester.pump_app(
            None,
            keyed(radio(1).group_value(2).on_changed(log_into(&log)), &key),
        );
        tester.tap();
        assert_eq!(*log.borrow(), [Some(1)]);
        log.borrow_mut().clear();

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .on_changed(log_into(&log))
                    .active_color(CupertinoColors::SYSTEM_GREEN.color()),
                &key,
            ),
        );
        tester.tap();
        assert!(log.borrow().is_empty());

        tester.pump_app(None, keyed(radio(1).group_value(2), &key));
        tester.tap();
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn a_disabled_radio_does_not_report_a_tap() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(2)
                    .enabled(false)
                    .on_changed(log_into(&log)),
                &key,
            ),
        );
        tester.tap();
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn a_toggleable_radio_reports_null_when_tapped_while_selected() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(2)
                    .on_changed(log_into(&log))
                    .toggleable(true),
                &key,
            ),
        );
        tester.tap();
        assert_eq!(*log.borrow(), [Some(1)]);
        log.borrow_mut().clear();

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .on_changed(log_into(&log))
                    .toggleable(true),
                &key,
            ),
        );
        tester.tap();
        assert_eq!(*log.borrow(), [None]);
        log.borrow_mut().clear();

        tester.pump_app(
            None,
            keyed(radio(1).on_changed(log_into(&log)).toggleable(true), &key),
        );
        tester.tap();
        assert_eq!(*log.borrow(), [Some(1)]);
    }

    #[test]
    fn a_radio_under_a_radio_group_selects_through_the_group() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let log: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
        let group = |group_value: Option<&str>, log: &Rc<RefCell<Vec<Option<String>>>>| {
            let log = Rc::clone(log);
            let mut group = RadioGroup::<String>::new(
                Rc::new(move |_app, value| log.borrow_mut().push(value)),
                CupertinoRadio::new(String::from("jefferson")).key(key.clone().into()),
            );
            if let Some(group_value) = group_value {
                group = group.group_value(String::from(group_value));
            }
            group.into_widget()
        };

        tester.pump_app(None, group(Some("lafayette"), &log));
        assert_eq!(
            tester.painter(&key).value(&tester.cell.borrow()),
            Some(false)
        );
        tester.tap();
        assert_eq!(*log.borrow(), [Some(String::from("jefferson"))]);

        tester.pump_app(None, group(Some("jefferson"), &log));
        tester.settle();
        assert_eq!(
            tester.painter(&key).value(&tester.cell.borrow()),
            Some(true)
        );
    }

    /// Dart's `StatefulBuilder` around three radios that share `groupValue`.
    struct KeyboardHost {
        focus_node2: FocusNodeRef,
        group_value: Rc<RefCell<Option<i32>>>,
    }

    impl Debug for KeyboardHost {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("KeyboardHost")
        }
    }

    impl StatefulWidget for KeyboardHost {
        type State = KeyboardHostState;

        fn create_state(&self) -> KeyboardHostState {
            KeyboardHostState {
                state: StateData::new(),
            }
        }
    }

    struct KeyboardHostState {
        state: StateData<KeyboardHost>,
    }

    impl State for KeyboardHostState {
        type Widget = KeyboardHost;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let group_value = Rc::clone(&self.widget(app).group_value);
            let focus_node2 = self.widget(app).focus_node2.clone();
            let on_changed = || {
                let group_value = Rc::clone(&group_value);
                Rc::new(move |app: &mut App, new_value: Option<i32>| {
                    *group_value.borrow_mut() = new_value;
                    self.set_state(app, |_| {});
                }) as ValueChanged<Option<i32>>
            };
            let current = *group_value.borrow();
            let with_group = |radio: CupertinoRadio<i32>| match current {
                Some(value) => radio.group_value(value),
                None => radio,
            };
            SizedBox::new()
                .width(200.0)
                .height(100.0)
                .child(Row::new().children(vec![
                    with_group(radio(0).on_changed(on_changed()).autofocus(true)).into_widget(),
                    with_group(radio(1).on_changed(on_changed())).into_widget(),
                    with_group(radio(2).on_changed(on_changed()).focus_node(focus_node2))
                        .into_widget(),
                ]))
                .into_widget()
        }
    }

    #[test]
    fn a_radio_can_be_controlled_by_keyboard_shortcuts() {
        let mut tester = Tester::new();
        tester.highlight_traditionally();
        let group_value = Rc::new(RefCell::new(Some(1)));
        let focus_node2: FocusNodeRef = FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
        focus_node2.set_debug_label(&mut tester.cell.borrow_mut(), Some("radio2".into()));

        tester.pump_app(
            None,
            KeyboardHost {
                focus_node2: focus_node2.clone(),
                group_value: Rc::clone(&group_value),
            }
            .into_widget(),
        );
        tester.settle();

        tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
        tester.settle();
        // On web, radios don't respond to the enter key.
        assert_eq!(
            *group_value.borrow(),
            if K_IS_WEB { Some(1) } else { Some(0) }
        );

        focus_node2.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.settle();

        tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
        tester.settle();
        assert_eq!(*group_value.borrow(), Some(2));
    }

    #[test]
    fn a_checkmark_shows_only_when_use_checkmark_style_is_true_and_selected() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let on_changed: ValueChanged<Option<i32>> = Rc::new(|_app, _value| {});
        let has_path = |drawn: &[Drawn]| drawn.iter().any(|drawn| matches!(drawn, Drawn::Path(_)));

        tester.pump_app(
            None,
            keyed(
                radio(1).group_value(1).on_changed(Rc::clone(&on_changed)),
                &key,
            ),
        );
        tester.settle();
        // Has no checkmark when useCheckmarkStyle is false
        assert!(!has_path(&tester.drawn(&key)));

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(2)
                    .use_checkmark_style(true)
                    .on_changed(Rc::clone(&on_changed)),
                &key,
            ),
        );
        tester.settle();
        // Has no checkmark when group value doesn't match the value
        assert!(!has_path(&tester.drawn(&key)));

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .use_checkmark_style(true)
                    .on_changed(on_changed),
                &key,
            ),
        );
        tester.settle();
        // Draws a path to show the checkmark when toggled on
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Path(K_CHECKMARK_STROKE_WIDTH as f32),
                Drawn::Path(K_CHECKMARK_STROKE_WIDTH as f32)
            ]
        );
    }

    #[test]
    fn a_radio_that_disappears_while_the_pointer_is_down_does_not_crash() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let shown = |show: bool| {
            if show {
                CupertinoRadio::new(true)
                    .key(key.clone().into())
                    .group_value(false)
                    .on_changed(Rc::new(|_app, _value| {}))
                    .into_widget()
            } else {
                SizedBox::new().into_widget()
            }
        };

        tester.pump_app(None, shown(true));
        // Put a pointer down on the screen.
        tester.press();
        // While the pointer is down, the widget disappears.
        tester.pump_app(None, shown(false));
        assert!(key.current_context(&tester.cell.borrow_mut()).is_none());
        // Release pointer after widget disappeared.
        tester.release();
    }

    #[test]
    fn a_radio_can_set_its_inactive_active_and_fill_colors() {
        const INACTIVE_BORDER_COLOR: u32 = 0xffd1d1d6;
        const ACTIVE_COLOR: Color = Color::new(0x0000000A);
        const FILL_COLOR: Color = Color::new(0x0000000B);
        const INACTIVE_COLOR: Color = Color::new(0x0000000C);
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let configured = |group_value: i32| {
            radio(1)
                .group_value(group_value)
                .on_changed(Rc::new(|_app, _value| {}))
                .active_color(ACTIVE_COLOR)
                .fill_color(FILL_COLOR)
                .inactive_color(INACTIVE_COLOR)
        };

        tester.pump_app(None, keyed(configured(2), &key));
        // The colours have no alpha, so valo records no draw of the fill: the painter holds
        // the inactive colour the circle is filled with.
        {
            let painter = tester.painter(&key);
            let app = tester.cell.borrow();
            assert_eq!(painter.inactive_color(&app), INACTIVE_COLOR);
            assert_eq!(painter.value(&app), Some(false));
        }
        assert_eq!(
            tester.drawn(&key),
            [Drawn::StrokedCircle(
                OUTER_RADIUS,
                K_BORDER_OUTLINE_STROKE_WIDTH as f32,
                INACTIVE_BORDER_COLOR
            )],
            "Unselected radio button should use inactive and border colors"
        );

        tester.pump_app(None, keyed(configured(1), &key));
        let painter = tester.painter(&key);
        let app = tester.cell.borrow();
        assert_eq!(painter.active_color(&app), ACTIVE_COLOR);
        assert_eq!(painter.fill_color(&app), FILL_COLOR);
        assert_eq!(
            painter.value(&app),
            Some(true),
            "Selected radio button should use active and fill colors"
        );
    }

    #[test]
    fn a_radio_has_the_default_colors_in_each_state_and_brightness() {
        // Dart pins these with goldens; the painter's colours are what the goldens show.
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let cases: Vec<(Option<CupertinoThemeData>, bool, i32, u32, u32, u32)> = vec![
            (None, true, 1, 0xff007aff, 0xffffffff, 0x00000000),
            (None, true, 2, 0xff007aff, 0xffffffff, 0xffd1d1d6),
            (dark(), true, 1, 0xff3264d7, 0xffdee8f8, 0x00000000),
            (dark(), true, 2, 0xff3264d7, 0xffffffff, 0x40000000),
            (None, false, 1, 0x80ffffff, 0x40000000, 0x40000000),
            (None, false, 2, 0x80ffffff, 0xffffffff, 0x40000000),
            (dark(), false, 1, 0x80ffffff, 0x40ffffff, 0x40000000),
            (dark(), false, 2, 0x80ffffff, 0xffffffff, 0x40000000),
        ];
        for (theme, enabled, group_value, active, fill, border) in cases {
            let mut configured = radio(1).group_value(group_value);
            if enabled {
                configured = configured.on_changed(Rc::new(|_app, _value| {}));
            }
            tester.pump_app(theme, keyed(configured, &key));
            let painter = tester.painter(&key);
            let app = tester.cell.borrow();
            let case = format!("enabled {enabled}, group value {group_value}");
            assert_eq!(painter.active_color(&app).value(), active, "{case}: active");
            assert_eq!(painter.fill_color(&app).value(), fill, "{case}: fill");
            assert_eq!(painter.border_color(&app).value(), border, "{case}: border");
        }
    }

    #[test]
    fn a_disabled_radio_in_dark_mode_paints_its_outer_circle_with_the_disabled_gradient() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        tester.pump_app(dark(), keyed(radio(1).group_value(1), &key));
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Gradient(vec![0x14ffffff, 0x24ffffff]),
                Drawn::Circle(INNER_RADIUS, 0x40ffffff),
                Drawn::StrokedCircle(
                    OUTER_RADIUS,
                    K_BORDER_OUTLINE_STROKE_WIDTH as f32,
                    0x40000000
                ),
            ]
        );
    }

    #[test]
    fn a_radio_is_slightly_darkened_when_pressed_in_light_mode() {
        const ACTIVE_INNER_COLOR: u32 = 0xffffffff;
        const ACTIVE_OUTER_COLOR: u32 = 0xff007aff;
        const INACTIVE_BORDER_COLOR: u32 = 0xffd1d1d6;
        const INACTIVE_OUTER_COLOR: u32 = 0xffffffff;
        const PRESSED_SHADOW_COLOR: u32 = 0x26ffffff;
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let on_changed: ValueChanged<Option<i32>> = Rc::new(|_app, _value| {});

        tester.pump_app(
            None,
            keyed(
                radio(1).group_value(2).on_changed(Rc::clone(&on_changed)),
                &key,
            ),
        );
        tester.press();
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Circle(OUTER_RADIUS, INACTIVE_OUTER_COLOR),
                Drawn::Circle(OUTER_RADIUS, PRESSED_SHADOW_COLOR),
                Drawn::StrokedCircle(
                    OUTER_RADIUS,
                    K_BORDER_OUTLINE_STROKE_WIDTH as f32,
                    INACTIVE_BORDER_COLOR
                ),
            ],
            "Unselected pressed radio button is slightly darkened"
        );
        tester.release();

        tester.pump_app(
            None,
            keyed(radio(2).group_value(2).on_changed(on_changed), &key),
        );
        tester.press();
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Circle(OUTER_RADIUS, ACTIVE_OUTER_COLOR),
                Drawn::Circle(OUTER_RADIUS, PRESSED_SHADOW_COLOR),
                Drawn::Circle(INNER_RADIUS, ACTIVE_INNER_COLOR),
            ],
            "Selected pressed radio button is slightly darkened"
        );
        tester.release();
    }

    #[test]
    fn a_radio_is_slightly_lightened_when_pressed_in_dark_mode() {
        const ACTIVE_INNER_COLOR: u32 = 0xffffffff;
        const ACTIVE_OUTER_COLOR: u32 = 0xff007aff;
        const INACTIVE_BORDER_COLOR: u32 = 0x40000000;
        const PRESSED_SHADOW_COLOR: u32 = 0x26ffffff;
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let on_changed: ValueChanged<Option<i32>> = Rc::new(|_app, _value| {});

        tester.pump_app(
            dark(),
            keyed(
                radio(1).group_value(2).on_changed(Rc::clone(&on_changed)),
                &key,
            ),
        );
        tester.press();
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Gradient(vec![0x24ffffff, 0x4affffff]),
                Drawn::Circle(OUTER_RADIUS, PRESSED_SHADOW_COLOR),
                Drawn::StrokedCircle(
                    OUTER_RADIUS,
                    K_BORDER_OUTLINE_STROKE_WIDTH as f32,
                    INACTIVE_BORDER_COLOR
                ),
            ],
            "Unselected pressed radio button is slightly lightened"
        );
        tester.release();

        tester.pump_app(
            None,
            keyed(radio(2).group_value(2).on_changed(on_changed), &key),
        );
        tester.press();
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Circle(OUTER_RADIUS, ACTIVE_OUTER_COLOR),
                Drawn::Circle(OUTER_RADIUS, PRESSED_SHADOW_COLOR),
                Drawn::Circle(INNER_RADIUS, ACTIVE_INNER_COLOR),
            ],
            "Selected pressed radio button is slightly lightened"
        );
        tester.release();
    }

    fn mount_focused(
        tester: &mut Tester,
        key: &GlobalKey,
        configure: fn(CupertinoRadio<i32>) -> CupertinoRadio<i32>,
    ) -> FocusNodeRef {
        tester.highlight_traditionally();
        let node: FocusNodeRef = FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
        tester.pump_app(
            None,
            keyed(
                configure(
                    radio(1)
                        .group_value(1)
                        .on_changed(Rc::new(|_app, _value| {}))
                        .focus_node(node.clone())
                        .autofocus(true),
                ),
                key,
            ),
        );
        tester.pump();
        node
    }

    #[test]
    fn a_radio_is_focusable_and_has_the_default_focus_color() {
        let default_focus_color = HSLColor::from_color(with_opacity(
            CupertinoColors::ACTIVE_BLUE.color(),
            K_CUPERTINO_FOCUS_COLOR_OPACITY,
        ))
        .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
        .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
        .to_color();
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let node = mount_focused(&mut tester, &key, |radio| radio);

        assert!(node.has_primary_focus(&tester.cell.borrow()));
        let drawn = tester.drawn(&key);
        assert_eq!(
            drawn,
            [
                Drawn::Circle(OUTER_RADIUS, 0xff007aff),
                Drawn::Circle(INNER_RADIUS, 0xffffffff),
                Drawn::StrokedCircle(
                    OUTER_RADIUS + K_FOCUS_OUTLINE_STROKE_WIDTH as f32 / 2.0,
                    K_FOCUS_OUTLINE_STROKE_WIDTH as f32,
                    default_focus_color.value()
                ),
            ],
            "Radio is focusable and shows the default focus color"
        );
    }

    #[test]
    fn a_radio_can_configure_a_focus_color() {
        const FOCUS_COLOR: Color = Color::new(0x0000000A);
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let node = mount_focused(&mut tester, &key, |radio| radio.focus_color(FOCUS_COLOR));

        assert!(node.has_primary_focus(&tester.cell.borrow()));
        // The focus colour has no alpha, so valo records no draw of the outline: the painter
        // holds the colour and the focus it paints the outline for.
        assert_eq!(
            tester.drawn(&key),
            [
                Drawn::Circle(OUTER_RADIUS, 0xff007aff),
                Drawn::Circle(INNER_RADIUS, 0xffffffff),
            ]
        );
        let painter = tester.painter(&key);
        let app = tester.cell.borrow();
        assert!(painter.is_focused(&app));
        assert_eq!(
            painter.focus_color(&app),
            FOCUS_COLOR,
            "Radio configures the color of the focus outline"
        );
    }

    #[test]
    fn a_radio_configures_its_mouse_cursor() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .on_changed(Rc::new(|_app, _value| {}))
                    .mouse_cursor(SystemMouseCursors::FORBIDDEN.into()),
                &key,
            ),
        );
        tester.hover();
        assert_eq!(tester.cursor(), Some(SystemMouseCursors::FORBIDDEN.into()));
    }

    #[test]
    fn the_mouse_cursor_resolves_in_disabled_hovered_and_focused_states() {
        let cursor: MouseCursorRef = WidgetStateMouseCursor::resolve_with(
            |states| {
                if states.contains(&WidgetState::Disabled) {
                    return SystemMouseCursors::FORBIDDEN.into();
                }
                if states.contains(&WidgetState::Focused) {
                    return SystemMouseCursors::BASIC.into();
                }
                SystemMouseCursors::CLICK.into()
            },
            Some("_RadioMouseCursor()"),
        )
        .into();
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        let focus_node: FocusNodeRef = FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
        focus_node.set_debug_label(&mut tester.cell.borrow_mut(), Some("Radio".into()));
        tester.highlight_traditionally();

        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .on_changed(Rc::new(|_app, _value| {}))
                    .mouse_cursor(Rc::clone(&cursor))
                    .focus_node(focus_node.clone()),
                &key,
            ),
        );
        // Test hovered case.
        tester.hover();
        assert_eq!(tester.cursor(), Some(SystemMouseCursors::CLICK.into()));

        // Test focused case.
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump();
        assert_eq!(tester.cursor(), Some(SystemMouseCursors::BASIC.into()));

        // Test disabled case.
        tester.pump_app(
            None,
            keyed(radio(1).group_value(1).mouse_cursor(cursor), &key),
        );
        tester.pump();
        assert_eq!(tester.cursor(), Some(SystemMouseCursors::FORBIDDEN.into()));
    }

    #[test]
    fn a_radio_has_the_default_mouse_cursor() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        tester.pump_app(
            None,
            keyed(
                radio(1)
                    .group_value(1)
                    .on_changed(Rc::new(|_app, _value| {})),
                &key,
            ),
        );
        tester.hover();
        assert_eq!(
            tester.cursor(),
            Some(if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            })
        );
    }

    #[test]
    fn a_cupertino_radio_does_not_crash_at_zero_area() {
        let mut tester = Tester::new();
        let key = GlobalKey::new();
        tester.pump_app(
            None,
            SizedBox::shrink()
                .child(CupertinoRadio::new(false).key(key.clone().into()))
                .into_widget(),
        );
        let context = key
            .current_context(&tester.cell.borrow_mut())
            .expect("the radio mounted");
        let size = context.size(&tester.cell.borrow());
        assert_eq!(size, Some(Size::ZERO));
    }
}

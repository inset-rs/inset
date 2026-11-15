//! Flutter counterpart: `cupertino_ui/checkbox.dart`.
//!
//! [`CupertinoCheckboxState`] is a [`ToggleableStateMixin`], and its painter an arena
//! [`ToggleablePainter`] the state keeps for its lifetime, as Dart's `_painter` field is. The
//! deprecated `inactiveColor`, which nothing reads, is not ported (`PORTING.md`).

use std::fmt::{self, Debug};

use waft_animation::Animation;
use waft_embedder::valo::Cap;
use waft_embedder::{
    Brightness, Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, Radius, Rect,
    Size, Stroke, TargetPlatform,
};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, ValueChanged};
use waft_painting::{
    AlignmentGeometry, AnyBorderSide, AnyColor, BorderRadiusGeometry, BorderSide, Gradient,
    HSLColor, LinearGradient, OutlinedBorder, OutlinedBorderRef, RoundedRectangleBorder,
    ShapeBorder,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    AnyFocusNode, BuildContext, IntoWidget, KeyRef, Semantics, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, ToggleableStateMixin, ToggleableStateMixinData,
    WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::constants::{
    K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS, K_CUPERTINO_FOCUS_COLOR_OPACITY,
    K_CUPERTINO_FOCUS_COLOR_SATURATION, K_MIN_INTERACTIVE_DIMENSION_CUPERTINO,
};
use crate::theme::CupertinoTheme;

// Eyeballed from a checkbox on a physical Macbook Pro running macOS version 14.5.
const K_DISABLED_CHECK_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(
        Color::from_argb(64, 0, 0, 0),
        Color::from_argb(64, 255, 255, 255),
    );
const K_DISABLED_BORDER_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(
        Color::from_argb(13, 0, 0, 0),
        Color::from_argb(13, 0, 0, 0),
    );
const K_DEFAULT_BORDER_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(
        Color::from_argb(255, 209, 209, 214),
        Color::from_argb(50, 128, 128, 128),
    );
const K_PRESSED_OVERLAY_OPACITY: f64 = 0.15;
// In dark mode, the fill color of a checkbox is an opacity gradient of the
// background color.
const K_DARK_GRADIENT_OPACITIES: [f64; 2] = [0.14, 0.29];
const K_DISABLED_DARK_GRADIENT_OPACITIES: [f64; 2] = [0.08, 0.14];

/// Dart's `_kDefaultFillColor`: its light colour is `CupertinoColors.activeBlue`'s.
fn k_default_fill_color() -> AnyColor {
    CupertinoDynamicColor::with_brightness(
        CupertinoColors::ACTIVE_BLUE.color(),
        Color::from_argb(255, 50, 100, 215),
    )
    .into_any()
}

/// Dart's `_kDefaultCheckColor`: its light colour is `CupertinoColors.white`.
fn k_default_check_color() -> AnyColor {
    CupertinoDynamicColor::with_brightness(
        CupertinoColors::WHITE.color(),
        Color::from_argb(255, 222, 232, 248),
    )
    .into_any()
}

/// Dart's deprecated `Color.withOpacity`, deprecated here too: its rounding to an 8-bit alpha
/// is part of the behaviour being ported.
#[allow(deprecated)]
fn with_opacity(color: Color, opacity: f64) -> Color {
    color.with_opacity(opacity)
}

/// A macOS style checkbox.
///
/// The checkbox itself does not maintain any state. Instead, when the state of the checkbox
/// changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most widgets that
/// use a checkbox will listen for the [`on_changed`](Self::on_changed) callback and rebuild the
/// checkbox with a new [`value`](Self::value) to update the visual appearance of the checkbox.
///
/// The checkbox can optionally display three values - true, false, and null - if
/// [`tristate`](Self::tristate) is true. When [`value`](Self::value) is null a dash is
/// displayed. By default [`tristate`](Self::tristate) is false and the checkbox's
/// [`value`](Self::value) must be true or false.
///
/// In the Apple Human Interface Guidelines (HIG), checkboxes are encouraged for use on macOS,
/// but is silent about their use on iOS. If a multi-selection component is needed on iOS, the
/// HIG encourages the developer to use switches ([`CupertinoSwitch`](crate::CupertinoSwitch)
/// in Flutter) instead, or to find a creative custom solution.
///
/// Visually, the checkbox is a square of [`CupertinoCheckbox::WIDTH`] pixels. However, the
/// widget's tap target and layout size depend on the platform:
///   * On desktop devices, the tap target matches the visual size.
///   * On mobile devices, the tap target expands to a square of
///     [`K_MIN_INTERACTIVE_DIMENSION_CUPERTINO`] pixels to meet accessibility guidelines.
///
/// See also:
///
///  * `Checkbox`, the Material Design equivalent.
///  * [`CupertinoSwitch`](crate::CupertinoSwitch), a widget with semantics similar to
///    [`CupertinoCheckbox`].
///  * [`CupertinoSlider`](crate::CupertinoSlider), for selecting a value in a range.
///  * <https://developer.apple.com/design/human-interface-guidelines/components/selection-and-input/toggles/>
#[derive(Clone)]
pub struct CupertinoCheckbox {
    pub key: Option<KeyRef>,
    /// Whether this checkbox is checked.
    ///
    /// When [`tristate`](Self::tristate) is true, a value of null corresponds to the mixed
    /// state. When [`tristate`](Self::tristate) is false, this value must not be null. This is
    /// asserted in debug mode.
    pub value: Option<bool>,
    /// If true, the checkbox's [`value`](Self::value) can be true, false, or null.
    ///
    /// [`CupertinoCheckbox`] displays a dash when its value is null.
    ///
    /// When a tri-state checkbox ([`tristate`](Self::tristate) is true) is tapped, its
    /// [`on_changed`](Self::on_changed) callback will be applied to true if the current value
    /// is false, to null if value is true, and to false if value is null (i.e. it cycles
    /// through false => true => null => false when tapped).
    ///
    /// If tristate is false (the default), [`value`](Self::value) must not be null, and
    /// [`on_changed`](Self::on_changed) will only toggle between true and false.
    pub tristate: bool,
    /// Called when the value of the checkbox should change.
    ///
    /// The checkbox passes the new value to the callback but does not actually change state
    /// until the parent widget rebuilds the checkbox with the new value.
    ///
    /// If this callback is null, the checkbox will be displayed as disabled and will not
    /// respond to input gestures.
    ///
    /// When the checkbox is tapped, if [`tristate`](Self::tristate) is false (the default)
    /// then the [`on_changed`](Self::on_changed) callback will be applied to `!value`. If
    /// [`tristate`](Self::tristate) is true this callback cycle from false to true to null and
    /// back to false again.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
    pub on_changed: Option<ValueChanged<Option<bool>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// When [`value`](Self::value) is null and [`tristate`](Self::tristate) is true,
    /// [`WidgetState::Selected`] is included as a state.
    ///
    /// If null, then [`SystemMouseCursors::BASIC`] is used when this checkbox is disabled.
    /// When the checkbox is enabled, [`SystemMouseCursors::CLICK`] is used on Web, and
    /// [`SystemMouseCursors::BASIC`] is used on other platforms.
    ///
    /// See also:
    ///
    ///  * `WidgetStateMouseCursor`, a mouse cursor that implements `WidgetStateProperty` which
    ///    is used in APIs that need to accept either a mouse cursor or a
    ///    `WidgetStateProperty`.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The color to use when this checkbox is checked.
    ///
    /// If [`fill_color`](Self::fill_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, [`fill_color`](Self::fill_color) will be used instead
    /// of [`active_color`](Self::active_color).
    ///
    /// Defaults to [`CupertinoColors::ACTIVE_BLUE`].
    pub active_color: Option<Color>,
    /// The color used to fill this checkbox.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If [`fill_color`](Self::fill_color) resolves to null for the requested state, then the
    /// fill color falls back to [`active_color`](Self::active_color) if the state includes
    /// [`WidgetState::Selected`], [`CupertinoColors::WHITE`] at 50% opacity if checkbox is
    /// disabled, and [`CupertinoColors::WHITE`] otherwise.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The color to use for the check icon when this checkbox is checked.
    ///
    /// If null, then the value of [`CupertinoColors::WHITE`] is used if the checkbox is
    /// enabled. If the checkbox is disabled, a grey-black color is used.
    pub check_color: Option<Color>,
    /// The color for the checkbox's border shadow when it has the input focus.
    ///
    /// If null, then a paler form of the [`active_color`](Self::active_color) will be used.
    pub focus_color: Option<Color>,
    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed by
    /// this widget. The widget will be focusable even if a `focus_node` is not supplied. If
    /// supplied, the given `focus_node` will be _hosted_ by this widget, but not owned.
    pub focus_node: Option<AnyFocusNode>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// The color and width of the checkbox's border.
    ///
    /// This property can be a `WidgetStateBorderSide` that can specify different border color
    /// and widths depending on the checkbox's state.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///  * [`WidgetState::Error`].
    ///
    /// If this property is not a `WidgetStateBorderSide` and it is non-null, then it is only
    /// rendered when the checkbox's value is false. The difference in interpretation is for
    /// backwards compatibility.
    ///
    /// If this property is null and the checkbox's value is false, then the side defaults to a
    /// one pixel wide grey-black border.
    pub side: Option<AnyBorderSide>,
    /// The shape of the checkbox.
    ///
    /// If this property is null then the shape defaults to a [`RoundedRectangleBorder`] with a
    /// circular corner radius of 4.0.
    pub shape: Option<OutlinedBorderRef>,
    /// The tap target and layout size of the checkbox.
    ///
    /// If this property is null, the tap target size defaults to a square of
    /// [`CupertinoCheckbox::WIDTH`] pixels on desktop devices and
    /// [`K_MIN_INTERACTIVE_DIMENSION_CUPERTINO`] pixels on mobile devices.
    pub tap_target_size: Option<Size>,
    /// The semantic label for the checkbox that will be announced by screen readers.
    ///
    /// This is announced by assistive technologies (e.g TalkBack/VoiceOver).
    ///
    /// This label does not show in the UI.
    pub semantic_label: Option<String>,
}

impl CupertinoCheckbox {
    /// The width of a checkbox widget.
    pub const WIDTH: f64 = 14.0;

    /// Creates a macOS-styled checkbox.
    ///
    /// The checkbox itself does not maintain any state. Instead, when the state of the
    /// checkbox changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most
    /// widgets that use a checkbox will listen for the [`on_changed`](Self::on_changed)
    /// callback and rebuild the checkbox with a new [`value`](Self::value) to update the visual
    /// appearance of the checkbox.
    ///
    /// The following arguments are required:
    ///
    /// * `value`, which determines whether the checkbox is checked. The `value` can only be
    ///   null if [`tristate`](Self::tristate) is true.
    /// * `on_changed`, which is called when the value of the checkbox should change. It can be
    ///   set to null to disable the checkbox.
    pub fn new(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
    ) -> CupertinoCheckbox {
        CupertinoCheckbox {
            key: None,
            value,
            tristate: false,
            on_changed,
            mouse_cursor: None,
            active_color: None,
            fill_color: None,
            check_color: None,
            focus_color: None,
            focus_node: None,
            autofocus: false,
            side: None,
            shape: None,
            tap_target_size: None,
            semantic_label: None,
        }
    }

    /// Dart `CupertinoCheckbox(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoCheckbox {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoCheckbox(tristate:)`.
    pub fn tristate(mut self, tristate: bool) -> CupertinoCheckbox {
        self.tristate = tristate;
        self
    }

    /// Dart `CupertinoCheckbox(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> CupertinoCheckbox {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CupertinoCheckbox(activeColor:)`.
    pub fn active_color(mut self, active_color: Color) -> CupertinoCheckbox {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `CupertinoCheckbox(fillColor:)`.
    pub fn fill_color(
        mut self,
        fill_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CupertinoCheckbox {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `CupertinoCheckbox(checkColor:)`.
    pub fn check_color(mut self, check_color: Color) -> CupertinoCheckbox {
        self.check_color = Some(check_color);
        self
    }

    /// Dart `CupertinoCheckbox(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> CupertinoCheckbox {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `CupertinoCheckbox(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> CupertinoCheckbox {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoCheckbox(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoCheckbox {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoCheckbox(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> CupertinoCheckbox {
        self.side = Some(side.into());
        self
    }

    /// Dart `CupertinoCheckbox(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> CupertinoCheckbox {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `CupertinoCheckbox(tapTargetSize:)`.
    pub fn tap_target_size(mut self, tap_target_size: Size) -> CupertinoCheckbox {
        self.tap_target_size = Some(tap_target_size);
        self
    }

    /// Dart `CupertinoCheckbox(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> CupertinoCheckbox {
        self.semantic_label = Some(semantic_label.into());
        self
    }
}

impl Debug for CupertinoCheckbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoCheckbox")
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

impl StatefulWidget for CupertinoCheckbox {
    type State = CupertinoCheckboxState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.tristate || self.value.is_some());
    }

    fn create_state(&self) -> CupertinoCheckboxState {
        CupertinoCheckboxState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            toggleable: ToggleableStateMixinData::new(),
            painter: None,
            previous_value: None,
            focused: false,
        }
    }
}

/// Dart's `_CupertinoCheckboxState`.
pub struct CupertinoCheckboxState {
    state: StateData<CupertinoCheckbox>,
    ticker_provider: TickerProviderStateMixinData,
    toggleable: ToggleableStateMixinData,
    /// Dart's `final _CheckboxPainter _painter = _CheckboxPainter()`, created in `init_state`
    /// because a painter is an arena object and a state is created without the `App`.
    painter: Option<Handle<CheckboxPainter>>,
    previous_value: Option<bool>,
    #[expect(
        dead_code,
        reason = "Dart's public field, which nothing reads or writes"
    )]
    focused: bool,
}

impl CupertinoCheckboxState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<CheckboxPainter> {
        app.get(self).painter.expect("created in init_state")
    }

    /// Dart's `_defaultFillColor` resolved in `states`: its closure resolves a dynamic colour
    /// against the state's context, which needs the `App`, so the property is this method.
    fn default_fill_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        states: &WidgetStates,
    ) -> Color {
        if states.contains(&WidgetState::Disabled) {
            return with_opacity(CupertinoColors::WHITE.color(), 0.5);
        }
        if states.contains(&WidgetState::Selected) {
            return self.widget(app).active_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(&k_default_fill_color(), app, context).color()
            });
        }
        CupertinoColors::WHITE.color()
    }

    /// Dart's `_defaultCheckColor` resolved in `states`, a method for the reason
    /// [`default_fill_color`](Self::default_fill_color) is.
    fn default_check_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        states: &WidgetStates,
    ) -> Color {
        if states.contains(&WidgetState::Disabled) && states.contains(&WidgetState::Selected) {
            return self.widget(app).check_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(
                    &K_DISABLED_CHECK_COLOR_DYNAMIC.to_any(),
                    app,
                    context,
                )
                .color()
            });
        }
        if states.contains(&WidgetState::Selected) {
            return self.widget(app).check_color.unwrap_or_else(|| {
                CupertinoDynamicColor::resolve(&k_default_check_color(), app, context).color()
            });
        }
        CupertinoColors::WHITE.color()
    }

    /// Dart's `_defaultSide` resolved in `states`, a method for the reason
    /// [`default_fill_color`](Self::default_fill_color) is.
    fn default_side(app: &mut App, context: BuildContext, states: &WidgetStates) -> BorderSide {
        if (states.contains(&WidgetState::Selected) || states.contains(&WidgetState::Focused))
            && !states.contains(&WidgetState::Disabled)
        {
            return BorderSide {
                width: 0.0,
                color: CupertinoColors::TRANSPARENT.color(),
                ..BorderSide::default()
            };
        }
        if states.contains(&WidgetState::Disabled) {
            return BorderSide {
                color: CupertinoDynamicColor::resolve(
                    &K_DISABLED_BORDER_COLOR_DYNAMIC.to_any(),
                    app,
                    context,
                )
                .color(),
                ..BorderSide::default()
            };
        }
        BorderSide {
            color: CupertinoDynamicColor::resolve(
                &K_DEFAULT_BORDER_COLOR_DYNAMIC.to_any(),
                app,
                context,
            )
            .color(),
            ..BorderSide::default()
        }
    }

    /// Dart's `_resolveSide`: `resolve_as` answers Dart's `is WidgetStateBorderSide` itself.
    fn resolve_side(side: &Option<AnyBorderSide>, states: &WidgetStates) -> Option<BorderSide> {
        if side.as_ref().is_some_and(|side| {
            side.extension::<waft_widgets::WidgetStateBorderSide>()
                .is_some()
        }) {
            return <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(side, states)
                .map(|side| *side);
        }
        if !states.contains(&WidgetState::Selected) {
            return side.as_ref().map(|side| **side);
        }
        None
    }

    /// Dart's `HSLColor.fromColor(effectiveActiveColor.withOpacity(...))...toColor()`, the
    /// [`focus_color`](CupertinoCheckbox::focus_color) default.
    fn default_focus_color(active_color: Color) -> Color {
        HSLColor::from_color(with_opacity(active_color, K_CUPERTINO_FOCUS_COLOR_OPACITY))
            .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
            .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
            .to_color()
    }

    /// Dart's `effectiveMouseCursor`, built from the widget's cursor.
    fn effective_mouse_cursor(
        mouse_cursor: Option<MouseCursorRef>,
    ) -> WidgetStatePropertyRef<MouseCursorRef> {
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(move |states: &WidgetStates| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                .unwrap_or_else(|| {
                    if K_IS_WEB && !states.contains(&WidgetState::Disabled) {
                        SystemMouseCursors::CLICK.into()
                    } else {
                        SystemMouseCursors::BASIC.into()
                    }
                })
        })
    }

    /// Dart's `effectiveSize`: the tap target the platform asks for.
    fn effective_size(app: &App, tap_target_size: Option<Size>) -> Size {
        tap_target_size.unwrap_or(match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => {
                Size::square(K_MIN_INTERACTIVE_DIMENSION_CUPERTINO)
            }
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => {
                Size::square(CupertinoCheckbox::WIDTH)
            }
        })
    }
}

impl TickerProviderStateMixin for CupertinoCheckboxState {
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
impl TickerProviderObject for CupertinoCheckboxState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ToggleableStateMixin for CupertinoCheckboxState {
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
}

impl State for CupertinoCheckboxState {
    type Widget = CupertinoCheckbox;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let painter = CheckboxPainter::new(app);
        app.get_mut(self).painter = Some(painter);
        ToggleableStateMixin::init_state(self, app);
        app.get_mut(self).previous_value = self.widget(app).value;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoCheckbox) {
        if old_widget.value != self.widget(app).value {
            app.get_mut(self).previous_value = old_widget.value;
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let painter = self.painter(app);
        ToggleablePainter::dispose(painter, app);
        app.get_mut(self).painter = None;
        app.destroy(painter);
        ToggleableStateMixin::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();

        // Colors need to be resolved in selected and non selected states separately.
        let mut active_states = self.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = self.states(app);
        inactive_states.remove(&WidgetState::Selected);

        let current_states = self.states(app);

        let effective_active_color = match widget
            .fill_color
            .as_ref()
            .and_then(|fill_color| fill_color.resolve(&active_states))
        {
            Some(color) => color,
            None => self.default_fill_color(app, context, &active_states),
        };

        let effective_inactive_color = match widget
            .fill_color
            .as_ref()
            .and_then(|fill_color| fill_color.resolve(&inactive_states))
        {
            Some(color) => color,
            None => self.default_fill_color(app, context, &inactive_states),
        };

        let effective_border_side = match Self::resolve_side(&widget.side, &current_states) {
            Some(side) => side,
            None => Self::default_side(app, context, &current_states),
        };

        let effective_focus_overlay_color = widget
            .focus_color
            .unwrap_or_else(|| Self::default_focus_color(effective_active_color));

        let effective_mouse_cursor = Self::effective_mouse_cursor(widget.mouse_cursor.clone());

        let effective_size = Self::effective_size(app, widget.tap_target_size);

        let check_color = self.default_check_color(app, context, &current_states);
        let brightness = CupertinoTheme::of(app, context).brightness();
        let shape = widget.shape.clone().unwrap_or_else(|| {
            OutlinedBorderRef::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(4.0)),
            ))
        });

        let painter = self.painter(app);
        painter.set_position(app, self.position(app).as_animation());
        painter.set_reaction(app, self.reaction(app).as_animation());
        painter.set_focus_color(app, effective_focus_overlay_color);
        painter.set_down_position(app, self.down_position(app));
        painter.set_is_focused(app, Some(current_states.contains(&WidgetState::Focused)));
        painter.set_is_hovered(app, Some(current_states.contains(&WidgetState::Hovered)));
        painter.set_active_color(app, effective_active_color);
        painter.set_inactive_color(app, effective_inactive_color);
        painter.set_check_color(app, check_color);
        painter.set_value(app, ToggleableStateMixin::value(self, app));
        painter.set_previous_value(app, app.get(self).previous_value);
        painter.set_is_active(app, Some(widget.on_changed.is_some()));
        painter.set_shape(app, shape);
        painter.set_side(app, effective_border_side);
        painter.set_brightness(app, brightness);

        let mut semantics = Semantics::new().checked(widget.value.unwrap_or(false));
        if let Some(semantic_label) = widget.semantic_label.clone() {
            semantics = semantics.label(semantic_label);
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
                effective_size,
                ToggleablePainterRef::new(painter),
            ))
            .into_widget()
    }
}

/// Dart's `_CheckboxPainter`.
struct CheckboxPainter {
    change_notifier: ChangeNotifierData,
    toggleable_painter: ToggleablePainterData,
    check_color: Option<Color>,
    value: Option<bool>,
    previous_value: Option<bool>,
    shape: Option<OutlinedBorderRef>,
    side: Option<BorderSide>,
    brightness: Option<Brightness>,
}

impl CheckboxPainter {
    fn new(app: &mut App) -> Handle<CheckboxPainter> {
        app.create(CheckboxPainter {
            change_notifier: ChangeNotifierData::new(),
            toggleable_painter: ToggleablePainterData::new(),
            check_color: None,
            value: None,
            previous_value: None,
            shape: None,
            side: None,
            brightness: None,
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

    #[expect(
        dead_code,
        reason = "Dart's getter; only the setter is called, from the state's build"
    )]
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

    fn side(self: Handle<Self>, app: &App) -> BorderSide {
        app.get(self).side.expect("set before painting")
    }

    fn set_side(self: Handle<Self>, app: &mut App, value: BorderSide) {
        if app.get(self).side == Some(value) {
            return;
        }
        app.get_mut(self).side = Some(value);
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

    fn outer_rect_at(origin: Offset) -> Rect {
        const SIZE: f64 = CupertinoCheckbox::WIDTH;
        Rect::from_ltwh(origin.dx(), origin.dy(), SIZE, SIZE)
    }

    // The checkbox's border color if value == false, or its fill color when
    // value == true or null.
    fn color_at(self: Handle<Self>, app: &App, value: bool) -> Color {
        if value && self.is_active(app) {
            self.active_color(app)
        } else {
            self.inactive_color(app)
        }
    }

    // White stroke used to paint the check and dash.
    fn create_stroke_paint(self: Handle<Self>, app: &App) -> Paint {
        let mut stroke = Stroke::new(2.0);
        stroke.cap = Cap::Round;
        Paint {
            color: self.check_color(app).into(),
            style: PaintStyle::Stroke(stroke),
            ..Paint::default()
        }
    }

    // Draw a gradient from the top to the bottom of the checkbox.
    fn draw_fill_gradient(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        outer: Rect,
        top_color: Color,
        bottom_color: Color,
    ) {
        let fill_gradient = LinearGradient::new(
            // Eyeballed from a checkbox on a physical Macbook Pro running macOS version 14.5.
            vec![top_color, bottom_color],
        )
        .begin(AlignmentGeometry::TOP_CENTER)
        .end(AlignmentGeometry::BOTTOM_CENTER);
        let gradient_paint = Paint {
            shader: Some(fill_gradient.create_shader(outer, None)),
            ..Paint::default()
        };
        let shape = self.shape(app);
        if shape.prefer_paint_interior() {
            shape.paint_interior(canvas, outer, &gradient_paint, None);
        } else {
            let path = shape.get_outer_path(outer, None);
            canvas.draw_path(path.as_valo(), path.fill_type, &gradient_paint);
        }
    }

    fn draw_box(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        outer: Rect,
        paint: &Paint,
        side: Option<BorderSide>,
        value: bool,
    ) {
        let is_active = self.is_active(app);
        // Draw a gradient in dark mode except when the checkbox is enabled and checked.
        if self.brightness(app) == Some(Brightness::Dark) && !(is_active && value) {
            let color: Color = paint.color.into();
            let opacities = if is_active {
                K_DARK_GRADIENT_OPACITIES
            } else {
                K_DISABLED_DARK_GRADIENT_OPACITIES
            };
            self.draw_fill_gradient(
                app,
                canvas,
                outer,
                with_opacity(color, opacities[0]),
                with_opacity(color, opacities[1]),
            );
        } else if self.shape(app).prefer_paint_interior() {
            self.shape(app).paint_interior(canvas, outer, paint, None);
        } else {
            let path = self.shape(app).get_outer_path(outer, None);
            canvas.draw_path(path.as_valo(), path.fill_type, paint);
        }
        if let Some(side) = side {
            self.shape(app)
                .copy_with(Some(side))
                .paint(canvas, outer, None);
        }
    }

    fn draw_check(canvas: &mut Canvas, origin: Offset, paint: &Paint) {
        let mut path = PathBuilder::new();
        // The ratios for the offsets below were found from looking at the checkbox
        // examples on in the HIG docs. The distance from the needed point to the
        // edge was measured, then divided by the total width.
        const START: Offset = Offset::new(
            CupertinoCheckbox::WIDTH * 0.22,
            CupertinoCheckbox::WIDTH * 0.54,
        );
        const MID: Offset = Offset::new(
            CupertinoCheckbox::WIDTH * 0.40,
            CupertinoCheckbox::WIDTH * 0.75,
        );
        const END: Offset = Offset::new(
            CupertinoCheckbox::WIDTH * 0.78,
            CupertinoCheckbox::WIDTH * 0.25,
        );
        path.move_to(Offset::new(
            origin.dx() + START.dx(),
            origin.dy() + START.dy(),
        ));
        path.line_to(Offset::new(origin.dx() + MID.dx(), origin.dy() + MID.dy()));
        path.move_to(Offset::new(origin.dx() + MID.dx(), origin.dy() + MID.dy()));
        path.line_to(Offset::new(origin.dx() + END.dx(), origin.dy() + END.dy()));
        canvas.draw_path(&path.build(), FillRule::NonZero, paint);
    }

    fn draw_dash(canvas: &mut Canvas, origin: Offset, paint: &Paint) {
        // From measuring the checkbox example in the HIG docs, the dash was found
        // to be half the total width, centered in the middle.
        const START: Offset = Offset::new(
            CupertinoCheckbox::WIDTH * 0.25,
            CupertinoCheckbox::WIDTH * 0.5,
        );
        const END: Offset = Offset::new(
            CupertinoCheckbox::WIDTH * 0.75,
            CupertinoCheckbox::WIDTH * 0.5,
        );
        draw_line(canvas, origin + START, origin + END, paint);
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
        let stroke_paint = self.create_stroke_paint(app);
        let origin = size / 2.0 - Size::square(CupertinoCheckbox::WIDTH) / 2.0;
        let outer = Self::outer_rect_at(origin);
        let value = self.value(app);
        let paint = Paint::from_color(self.color_at(app, value.unwrap_or(true)).into());
        let side = Some(self.side(app));

        match value {
            Some(false) => {
                self.draw_box(app, canvas, outer, &paint, side, value.unwrap_or(true));
            }
            Some(true) => {
                self.draw_box(app, canvas, outer, &paint, side, value.unwrap_or(true));
                Self::draw_check(canvas, origin, &stroke_paint);
            }
            None => {
                self.draw_box(app, canvas, outer, &paint, side, value.unwrap_or(true));
                Self::draw_dash(canvas, origin, &stroke_paint);
            }
        }
        // The checkbox's opacity changes when pressed.
        if self.down_position(app).is_some() {
            let pressed_paint = Paint::from_color(
                if self.brightness(app) == Some(Brightness::Light) {
                    with_opacity(CupertinoColors::BLACK.color(), K_PRESSED_OVERLAY_OPACITY)
                } else {
                    with_opacity(CupertinoColors::WHITE.color(), K_PRESSED_OVERLAY_OPACITY)
                }
                .into(),
            );
            let shape = self.shape(app);
            if shape.prefer_paint_interior() {
                shape.paint_interior(canvas, outer, &pressed_paint, None);
            } else {
                let path = shape.get_outer_path(outer, None);
                canvas.draw_path(path.as_valo(), path.fill_type, &pressed_paint);
            }
        }
        if self.is_focused(app) {
            let focus_outer = outer.inflate(1.0);
            let border_paint = Paint {
                color: self.focus_color(app).into(),
                style: PaintStyle::Stroke(Stroke::new(3.5)),
                ..Paint::default()
            };
            self.draw_box(
                app,
                canvas,
                focus_outer,
                &border_paint,
                side,
                value.unwrap_or(true),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::Duration;

    use waft_embedder::valo::{GradientStop, Matrix, Op, Path, Shader, SpreadMode};
    use waft_embedder::{
        KeyData, KeyEventDeviceType, KeyEventType, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind, RRect,
    };
    use waft_foundation::AppCell;
    use waft_gestures::GestureBinding;
    use waft_painting::{PaintingBinding, draw_drrect};
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject, RendererBinding,
    };
    use waft_services::{KeyEventManager, LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_test::TestPlatform;
    use waft_widgets::{
        Center, FocusHighlightStrategy, FocusManager, FocusNode, FocusNodeLeaf, GlobalKey,
        IntoWidget, SizedBox, WidgetStateMouseCursor,
    };

    use super::*;
    use crate::app::CupertinoApp;
    use crate::test_support::{build, pump, test_view};
    use crate::theme::CupertinoThemeData;

    /// The `StatefulBuilder` every Dart test wraps its checkbox in: it owns the value.
    struct Host {
        key: Option<KeyRef>,
        initial: Option<bool>,
        theme: Option<CupertinoThemeData>,
        focus_node: Option<AnyFocusNode>,
        configure: Rc<dyn Fn(CupertinoCheckbox) -> CupertinoCheckbox>,
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
                checkbox_key: GlobalKey::new(),
            }
        }
    }

    struct HostState {
        state: StateData<Host>,
        value: Option<bool>,
        enabled: bool,
        checkbox_key: GlobalKey,
    }

    impl State for HostState {
        type Widget = Host;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let on_changed: Option<ValueChanged<Option<bool>>> = app.get(self).enabled.then(|| {
                Rc::new(move |app: &mut App, new_value: Option<bool>| {
                    self.set_state(app, |state| state.value = new_value);
                }) as ValueChanged<Option<bool>>
            });
            let mut checkbox = CupertinoCheckbox::new(app.get(self).value, on_changed)
                .key(app.get(self).checkbox_key.clone().into());
            if let Some(focus_node) = self.widget(app).focus_node {
                checkbox = checkbox.focus_node(focus_node);
            }
            let checkbox = (self.widget(app).configure)(checkbox);
            let centered = Center::new().child(checkbox).into_widget();
            match self.widget(app).theme.clone() {
                Some(theme) => CupertinoTheme::new(theme, centered).into_widget(),
                None => centered,
            }
        }
    }

    struct Setup {
        platform: TargetPlatform,
        initial: Option<bool>,
        theme: Option<CupertinoThemeData>,
        focusable: bool,
        in_app: bool,
        configure: Rc<dyn Fn(CupertinoCheckbox) -> CupertinoCheckbox>,
    }

    impl Setup {
        fn new() -> Setup {
            Setup {
                platform: TargetPlatform::IOS,
                initial: Some(false),
                theme: None,
                focusable: false,
                in_app: false,
                configure: Rc::new(|checkbox| checkbox),
            }
        }

        /// Mounts the host as a `CupertinoApp`'s home, under the app's shortcuts.
        fn in_app(mut self) -> Setup {
            self.in_app = true;
            self
        }

        fn on(mut self, platform: TargetPlatform) -> Setup {
            self.platform = platform;
            self
        }

        fn initial(mut self, initial: Option<bool>) -> Setup {
            self.initial = initial;
            self
        }

        fn theme(mut self, theme: CupertinoThemeData) -> Setup {
            self.theme = Some(theme);
            self
        }

        fn dark(self) -> Setup {
            self.theme(CupertinoThemeData::new().with_brightness(Brightness::Dark))
        }

        /// Hands the checkbox a focus node the fixture keeps.
        fn focus_node(mut self) -> Setup {
            self.focusable = true;
            self
        }

        fn configure(
            mut self,
            configure: impl Fn(CupertinoCheckbox) -> CupertinoCheckbox + 'static,
        ) -> Setup {
            self.configure = Rc::new(configure);
            self
        }

        fn mount(self) -> Fixture {
            let platform = TestPlatform::new().on(self.platform).with_view(test_view());
            let cell = AppCell::with_platform(Rc::new(platform));
            let focus_node = self
                .focusable
                .then(|| FocusNode::new(&mut cell.borrow_mut()).as_node());
            let host_key = GlobalKey::new();
            let host = Host {
                key: Some(host_key.clone().into()),
                initial: self.initial,
                theme: self.theme,
                focus_node,
                configure: self.configure,
            };
            let root = if self.in_app {
                install_fonts(&mut cell.borrow_mut());
                CupertinoApp::new().home(host).into_widget()
            } else {
                host.into_widget()
            };
            build(&cell, root);
            let host = host_key
                .current_state::<HostState>(&cell.borrow_mut())
                .expect("the host mounted");
            Fixture {
                cell,
                host,
                focus_node,
                clock: Duration::ZERO,
            }
        }
    }

    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    struct Fixture {
        cell: Rc<AppCell>,
        host: Handle<HostState>,
        focus_node: Option<AnyFocusNode>,
        clock: Duration,
    }

    /// The test view is 400x300 logical pixels.
    const VIEW_CENTER: Offset = Offset::new(200.0, 150.0);

    impl Fixture {
        fn value(&self) -> Option<bool> {
            self.cell.borrow().get(self.host).value
        }

        fn node(&self) -> AnyFocusNode {
            self.focus_node.expect("mounted with Setup::focus_node")
        }

        /// Rebuilds the host with `change` applied, as a Dart test's second `pumpWidget`.
        fn set(&mut self, change: impl FnOnce(&mut HostState)) {
            self.host.set_state(&mut self.cell.borrow_mut(), change);
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

        /// Dart's `tester.sendKeyEvent`: one key down and up, the way the shell delivers it.
        fn send_key_event(&mut self, physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) {
            let mut app = self.cell.borrow_mut();
            for event_type in [KeyEventType::Down, KeyEventType::Up] {
                KeyEventManager::instance(&mut app).handle_key_data(
                    &mut app,
                    KeyData {
                        time_stamp: self.clock,
                        event_type,
                        device_type: KeyEventDeviceType::Keyboard,
                        physical: physical.usb_hid_usage,
                        logical: logical.key_id,
                        character: None,
                        synthesized: false,
                    },
                );
            }
            app.drain_microtasks();
        }

        /// Dart's `tester.tap` at the checkbox's centre.
        fn tap(&mut self) {
            self.send(PointerChange::Down, PointerDeviceKind::Touch, VIEW_CENTER);
            self.send(PointerChange::Up, PointerDeviceKind::Touch, VIEW_CENTER);
            self.pump();
        }

        /// A mouse that arrives over the checkbox's centre.
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

        fn root(&self) -> AnyRenderObject {
            let key = self.cell.borrow().get(self.host).checkbox_key.clone();
            let app = self.cell.borrow();
            key.current_context(&app)
                .expect("the checkbox mounted")
                .find_render_object(&app)
                .expect("the checkbox has been laid out")
        }

        /// The ops the checkbox's painter records, in paint order.
        fn ops(&self) -> Vec<Op> {
            let root = self.root();
            let mut app = self.cell.borrow_mut();
            let custom = find::<RenderCustomPaint>(&app, root).expect("the checkbox's paint");
            let painter = custom.painter(&app).expect("the checkbox's painter");
            let size = custom.size(&app);
            let mut canvas = Canvas::new();
            painter.paint(&mut app, &mut canvas, size);
            canvas.build().ops().to_vec()
        }

        /// What the checkbox's painter draws, in paint order.
        fn drawn(&self) -> Vec<Drawn> {
            self.ops().iter().filter_map(Drawn::of).collect()
        }

        /// The shaders of the draws that paint through one, in paint order.
        fn shaders(&self) -> Vec<Shader> {
            self.ops()
                .iter()
                .filter_map(|op| match op {
                    Op::DrawRect { paint, .. } | Op::DrawPath { paint, .. } => paint.shader.clone(),
                    _ => None,
                })
                .collect()
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

    /// One recorded op, reduced to what Dart's `paints` matchers compare.
    ///
    /// A draw with a transparent paint is not recorded at all, so the zero-width transparent
    /// side of a checked, enabled box (the `rrect()` Dart's matchers skip over between the fill
    /// and the check) has no entry.
    #[derive(Clone, Debug, PartialEq)]
    enum Drawn {
        /// A filled rrect: its colour as `0xAARRGGBB` and its bounds.
        Fill(u32, [f32; 4]),
        /// A filled ring (Dart's `drrect`): its colour and its even-odd path of the outer and
        /// inner rrects.
        Ring(u32, Arc<Path>),
        /// A stroked shape: its colour and its stroke width.
        Stroke(u32, f32),
        /// A fill through a gradient shader: the paint's own colour.
        Gradient(u32),
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
            let (paint, rect, path) = match op {
                Op::DrawRect { paint, rect, .. } => (paint, *rect, None),
                Op::DrawPath {
                    paint,
                    content_bounds,
                    fill_rule,
                    path,
                    ..
                } => (
                    paint,
                    *content_bounds,
                    (*fill_rule == FillRule::EvenOdd).then(|| path.clone()),
                ),
                _ => return None,
            };
            let bounds = [rect.x, rect.y, rect.width, rect.height];
            Some(match (&paint.style, path) {
                (PaintStyle::Stroke(stroke), _) => Drawn::Stroke(argb(paint.color), stroke.width),
                (PaintStyle::Fill, _) if paint.shader.is_some() => {
                    Drawn::Gradient(argb(paint.color))
                }
                (PaintStyle::Fill, Some(path)) => Drawn::Ring(argb(paint.color), path),
                (PaintStyle::Fill, None) => Drawn::Fill(argb(paint.color), bounds),
            })
        }

        fn color(&self) -> u32 {
            match self {
                Drawn::Fill(color, _)
                | Drawn::Ring(color, _)
                | Drawn::Stroke(color, _)
                | Drawn::Gradient(color) => *color,
            }
        }
    }

    /// Dart's `paints..drrect(color:, outer:, inner:)`: the ring `RoundedRectangleBorder.paint`
    /// draws between `outer` and `inner` in `color`.
    fn ring(color: u32, outer: RRect, inner: RRect) -> Drawn {
        let mut canvas = Canvas::new();
        draw_drrect(
            &mut canvas,
            outer,
            inner,
            &Paint::from_color(Color::new(color).into()),
        );
        Drawn::of(&canvas.build().ops()[0]).expect("a ring")
    }

    fn colors(drawn: &[Drawn]) -> Vec<u32> {
        drawn.iter().map(Drawn::color).collect()
    }

    const ACTIVE_BLUE: u32 = 0xff007aff;
    const WHITE: u32 = 0xffffffff;

    #[test]
    fn a_tristate_checkbox_cycles_false_true_null() {
        let mut fixture = Setup::new()
            .initial(None)
            .configure(|checkbox| checkbox.tristate(true))
            .mount();
        assert_eq!(fixture.value(), None);
        for expected in [Some(false), Some(true), None] {
            fixture.tap();
            fixture.settle();
            assert_eq!(fixture.value(), expected);
        }
    }

    #[test]
    fn the_checkbox_can_be_toggled_by_keyboard_shortcuts() {
        let mut fixture = Setup::new()
            .in_app()
            .initial(Some(true))
            .configure(|checkbox| checkbox.autofocus(true))
            .mount();
        fixture.highlight_traditionally();
        fixture.settle();
        fixture.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
        fixture.settle();
        // On web, switches don't respond to the enter key.
        assert_eq!(fixture.value(), Some(K_IS_WEB));
        fixture.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
        fixture.settle();
        assert_eq!(fixture.value(), Some(true));
        fixture.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
        fixture.settle();
        assert_eq!(fixture.value(), Some(false));
        fixture.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
        fixture.settle();
        assert_eq!(fixture.value(), Some(true));
    }

    fn shape_and_side(checkbox: CupertinoCheckbox) -> CupertinoCheckbox {
        checkbox
            .shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(5.0)),
            ))
            .side(BorderSide {
                width: 4.0,
                color: Color::new(0xfff44336),
                ..BorderSide::default()
            })
    }

    #[test]
    fn the_shape_and_side_paint_the_border_ring_on_mobile() {
        for platform in [
            TargetPlatform::IOS,
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
        ] {
            let fixture = Setup::new().on(platform).configure(shape_and_side).mount();
            assert_eq!(
                fixture.drawn()[1],
                ring(
                    0xfff44336,
                    RRect::from_ltrbr(15.0, 15.0, 29.0, 29.0, Radius::circular(5.0)),
                    RRect::from_ltrbr(19.0, 19.0, 25.0, 25.0, Radius::circular(1.0)),
                ),
                "{platform:?}: the 14x14 box centred in the 44x44 tap target"
            );
        }
    }

    #[test]
    fn the_shape_and_side_paint_the_border_ring_on_desktop() {
        for platform in [
            TargetPlatform::MacOS,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
        ] {
            let fixture = Setup::new().on(platform).configure(shape_and_side).mount();
            assert_eq!(
                fixture.drawn()[1],
                ring(
                    0xfff44336,
                    RRect::from_ltrbr(0.0, 0.0, 14.0, 14.0, Radius::circular(5.0)),
                    RRect::from_ltrbr(4.0, 4.0, 10.0, 10.0, Radius::circular(1.0)),
                ),
                "{platform:?}: the tap target is the box"
            );
        }
    }

    #[test]
    fn the_tap_target_size_centres_the_box() {
        let fixture = Setup::new()
            .configure(|checkbox| checkbox.tap_target_size(Size::square(20.0)))
            .mount();
        assert_eq!(
            fixture.drawn()[1],
            ring(
                0xffd1d1d6,
                RRect::from_ltrbr(3.0, 3.0, 17.0, 17.0, Radius::circular(4.0)),
                RRect::from_ltrbr(4.0, 4.0, 16.0, 16.0, Radius::circular(3.0)),
            )
        );
    }

    #[test]
    fn a_mouse_over_the_checkbox_shows_the_configured_cursor() {
        let mut fixture = Setup::new().mount();
        fixture.hover();
        let expected: MouseCursorRef = if K_IS_WEB {
            SystemMouseCursors::CLICK.into()
        } else {
            SystemMouseCursors::BASIC.into()
        };
        assert_eq!(fixture.cursor(), Some(expected));

        fixture.set(|host| host.enabled = false);
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::BASIC.into()));

        let mut fixture = Setup::new()
            .initial(Some(true))
            .configure(|checkbox| checkbox.mouse_cursor(SystemMouseCursors::GRAB.into()))
            .mount();
        fixture.hover();
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::GRAB.into()));
    }

    #[test]
    fn the_mouse_cursor_resolves_in_selected_focused_and_disabled_states() {
        let cursor: MouseCursorRef = WidgetStateMouseCursor::resolve_with(
            |states| {
                if states.contains(&WidgetState::Disabled) {
                    SystemMouseCursors::FORBIDDEN.into()
                } else if states.contains(&WidgetState::Focused) {
                    SystemMouseCursors::GRAB.into()
                } else if states.contains(&WidgetState::Selected) {
                    SystemMouseCursors::CLICK.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                }
            },
            Some("_CheckboxMouseCursor()"),
        )
        .into();
        let mut fixture = Setup::new()
            .focus_node()
            .configure(move |checkbox| checkbox.mouse_cursor(Rc::clone(&cursor)))
            .mount();
        fixture.highlight_traditionally();
        fixture.hover();
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::BASIC.into()));

        fixture.set(|host| host.value = Some(true));
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::CLICK.into()));

        let node = fixture.node();
        node.request_focus(&mut fixture.cell.borrow_mut(), None);
        fixture.pump();
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::GRAB.into()));

        fixture.set(|host| host.enabled = false);
        assert_eq!(fixture.cursor(), Some(SystemMouseCursors::FORBIDDEN.into()));
    }

    fn fill_by_enablement() -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
            Some(Color::new(if states.contains(&WidgetState::Disabled) {
                0xFF000002
            } else {
                0xFF000001
            }))
        })
    }

    #[test]
    fn the_fill_colour_resolves_in_enabled_and_disabled_states() {
        let mut fixture = Setup::new()
            .initial(Some(true))
            .configure(|checkbox| checkbox.fill_color(fill_by_enablement()))
            .mount();
        fixture.settle();
        assert_eq!(fixture.drawn()[0].color(), 0xFF000001);
        fixture.set(|host| host.enabled = false);
        fixture.settle();
        assert_eq!(fixture.drawn()[0].color(), 0xFF000002);
    }

    #[test]
    fn the_fill_colour_takes_precedence_over_the_active_colour() {
        let mut fixture = Setup::new()
            .initial(Some(true))
            .configure(|checkbox| {
                checkbox
                    .fill_color(fill_by_enablement())
                    .active_color(Color::new(0xFF000003))
            })
            .mount();
        fixture.settle();
        assert_eq!(fixture.drawn()[0].color(), 0xFF000001);
        fixture.set(|host| host.enabled = false);
        fixture.settle();
        assert_eq!(fixture.drawn()[0].color(), 0xFF000002);
    }

    #[test]
    fn the_fill_colour_resolves_in_hovered_and_focused_states() {
        let fill = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
            Some(Color::new(if states.contains(&WidgetState::Hovered) {
                0xFF000001
            } else if states.contains(&WidgetState::Focused) {
                0xFF000002
            } else {
                0x00000000
            }))
        });
        let mut fixture = Setup::new()
            .initial(Some(true))
            .focus_node()
            .configure(move |checkbox| checkbox.fill_color(Rc::clone(&fill)))
            .mount();
        fixture.highlight_traditionally();
        let node = fixture.node();
        node.request_focus(&mut fixture.cell.borrow_mut(), None);
        fixture.settle();
        assert!(node.has_primary_focus(&fixture.cell.borrow()));
        assert_eq!(fixture.drawn()[0].color(), 0xFF000002);

        fixture.hover();
        fixture.settle();
        assert_eq!(fixture.drawn()[0].color(), 0xFF000001);
    }

    #[test]
    fn a_focused_checkbox_strokes_its_focus_colour() {
        let default_focus_color =
            argb(CupertinoCheckboxState::default_focus_color(Color::new(ACTIVE_BLUE)).into());
        for (focus_color, expected) in [(None, default_focus_color), (Some(0xffaabbcc), 0xffaabbcc)]
        {
            let mut fixture = Setup::new()
                .initial(Some(true))
                .focus_node()
                .configure(move |checkbox| {
                    let checkbox = checkbox.autofocus(true);
                    match focus_color {
                        Some(color) => checkbox.focus_color(Color::new(color)),
                        None => checkbox,
                    }
                })
                .mount();
            fixture.highlight_traditionally();
            fixture.pump();
            assert!(fixture.node().has_primary_focus(&fixture.cell.borrow()));
            let drawn = fixture.drawn();
            assert_eq!(drawn[0].color(), ACTIVE_BLUE);
            assert_eq!(drawn[1], Drawn::Stroke(WHITE, 2.0), "the check");
            assert_eq!(drawn[2], Drawn::Stroke(expected, 3.5), "{focus_color:?}");
        }
    }

    /// Presses the checkbox's centre and leaves the finger down.
    fn press(fixture: &mut Fixture) {
        fixture.send(PointerChange::Down, PointerDeviceKind::Touch, VIEW_CENTER);
        fixture.pump();
    }

    #[test]
    fn a_pressed_checkbox_takes_an_overlay_without_a_light_theme() {
        const PRESSED: u32 = 0x26ffffff;
        let mut fixture = Setup::new().mount();
        press(&mut fixture);
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), WHITE, "the inactive fill");
        assert!(matches!(drawn[1], Drawn::Ring(..)), "{:?}", drawn[1]);
        assert_eq!(drawn[2].color(), PRESSED);

        let mut fixture = Setup::new().initial(Some(true)).mount();
        press(&mut fixture);
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), ACTIVE_BLUE);
        assert_eq!(drawn[1], Drawn::Stroke(WHITE, 2.0), "the check");
        assert_eq!(drawn[2].color(), PRESSED);
    }

    #[test]
    fn a_light_theme_darkens_a_pressed_checkbox() {
        let mut fixture = Setup::new()
            .theme(CupertinoThemeData::new().with_brightness(Brightness::Light))
            .mount();
        press(&mut fixture);
        assert_eq!(fixture.drawn()[2].color(), 0x26000000);
    }

    #[test]
    fn a_dark_pressed_checkbox_is_lightened() {
        const PRESSED: u32 = 0x26ffffff;
        let mut fixture = Setup::new().dark().mount();
        press(&mut fixture);
        let drawn = fixture.drawn();
        assert_eq!(
            drawn[0],
            Drawn::Gradient(0xff000000),
            "the unchecked dark fill is a gradient"
        );
        assert!(matches!(drawn[1], Drawn::Ring(..)), "{:?}", drawn[1]);
        assert_eq!(drawn[2].color(), PRESSED);

        let mut fixture = Setup::new().dark().initial(Some(true)).mount();
        press(&mut fixture);
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), 0xff3264d7);
        assert_eq!(drawn[1], Drawn::Stroke(0xffdee8f8, 2.0), "the check");
        assert_eq!(drawn[2].color(), PRESSED);
    }

    /// The shader of Dart's `LinearGradient(begin: Alignment.topCenter, end:
    /// Alignment.bottomCenter, colors: [top, bottom])` over the 14x14 box the 44x44 tap target
    /// centres.
    fn dark_fill(top: u32, bottom: u32) -> Shader {
        Shader::Linear {
            start: Offset::new(22.0, 15.0).into(),
            end: Offset::new(22.0, 29.0).into(),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::new(top).into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: Color::new(bottom).into(),
                },
            ],
            spread: SpreadMode::Pad,
            local: Matrix::IDENTITY,
        }
    }

    #[test]
    fn a_dark_checkbox_fills_with_its_gradient_opacities() {
        // White at 0.14 and 0.29.
        let fixture = Setup::new().dark().mount();
        assert_eq!(fixture.shaders(), [dark_fill(0x24ffffff, 0x4affffff)]);

        // The checked, enabled box is filled flat.
        let fixture = Setup::new().dark().initial(Some(true)).mount();
        assert_eq!(fixture.shaders(), []);
    }

    #[test]
    fn a_disabled_dark_checkbox_fills_with_its_disabled_gradient_opacities() {
        // White at 0.08 and 0.14, checked or not.
        for initial in [Some(true), Some(false)] {
            let mut fixture = Setup::new().dark().initial(initial).mount();
            fixture.set(|host| host.enabled = false);
            assert_eq!(
                fixture.shaders(),
                [dark_fill(0x14ffffff, 0x24ffffff)],
                "{initial:?}"
            );
        }
    }

    #[test]
    fn a_disabled_checkbox_paints_its_disabled_defaults() {
        let mut fixture = Setup::new().initial(Some(true)).mount();
        fixture.set(|host| host.enabled = false);
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), 0x80ffffff, "white at half opacity");
        assert_eq!(
            drawn[2],
            Drawn::Stroke(0x40000000, 2.0),
            "the disabled check"
        );

        fixture.set(|host| host.value = Some(false));
        assert_eq!(colors(&fixture.drawn()), [0x80ffffff, 0x0d000000]);
    }

    #[test]
    fn a_mixed_checkbox_draws_a_dash() {
        let fixture = Setup::new()
            .initial(None)
            .configure(|checkbox| checkbox.tristate(true))
            .mount();
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), ACTIVE_BLUE);
        assert_eq!(drawn[1], Drawn::Stroke(WHITE, 2.0), "the dash");
    }

    #[test]
    fn a_checkbox_squeezed_to_nothing_lays_out_at_zero() {
        let cell = crate::test_support::test_cell();
        let key = GlobalKey::new();
        build(
            &cell,
            Center::new()
                .child(
                    SizedBox::shrink().child(
                        CupertinoCheckbox::new(Some(true), Some(Rc::new(|_app: &mut App, _| {})))
                            .key(key.clone().into()),
                    ),
                )
                .into_widget(),
        );
        let app = cell.borrow();
        let render = key
            .current_context(&app)
            .and_then(|context| context.find_render_object(&app))
            .and_then(|object| object.as_box())
            .expect("the checkbox laid out");
        assert_eq!(render.size(&app), Size::ZERO);
    }

    #[test]
    fn unmounting_the_checkbox_disposes_its_painter() {
        let fixture = Setup::new().mount();
        let key = fixture.cell.borrow().get(fixture.host).checkbox_key.clone();
        let state = key
            .current_state::<CupertinoCheckboxState>(&fixture.cell.borrow_mut())
            .expect("the checkbox mounted");
        let painter = fixture.cell.borrow().get(state).painter.expect("created");
        build(&fixture.cell, SizedBox::shrink().into_widget());
        assert!(!fixture.cell.borrow().contains(painter));
    }
}

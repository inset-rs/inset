//! Flutter counterpart: `cupertino/switch.dart`.
//!
//! [`CupertinoSwitchState`] is a [`ToggleableStateMixin`], and its painter an arena
//! [`ToggleablePainter`] the state keeps for its lifetime, as Dart's `_painter` field is. The
//! thumb images wait with painting's `DecorationImage` and the `Semantics(toggled:)` wrapper with
//! accessibility; the deprecated `activeColor` and `trackColor` are not ported (`PORTING.md`).

use std::cell::Cell;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationController, AnimationStatus, Curve, CurvedAnimation, Curves,
};
use waft_embedder::{
    Canvas, ClipOp, Color, Offset, Paint, PaintStyle, RRect, Radius, Rect, Size, Stroke,
    TargetPlatform, TextDirection, lerp_double, rrect_radii_elliptical,
};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, ValueChanged};
use waft_gestures::{
    DragEndDetails, DragStartBehavior, DragStartDetails, DragUpdateDetails, TapDownDetails,
};
use waft_painting::{
    AnyColor, BlurStyle, BorderSide, BoxPainter, BoxShadow, Decoration, FontVariation, HSLColor,
    ImageConfiguration, PaintingBinding, ShapeDecoration, StadiumBorder, TextPainter, TextSpan,
    TextStyle, draw_rrect,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{HapticFeedback, MouseCursor, MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    AnyFocusNode, BuildContext, Directionality, GestureDetector, Icon, IconTheme, IconThemeData,
    IntoWidget, KeyRef, MediaQuery, Opacity, State, StateData, StatefulWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, ToggleableStateMixin, ToggleableStateMixinData,
    WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
    create_local_image_configuration,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::constants::{
    K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS, K_CUPERTINO_FOCUS_COLOR_OPACITY,
    K_CUPERTINO_FOCUS_COLOR_SATURATION,
};
use crate::theme::CupertinoTheme;

// Hand coded defaults eyeballed from an iOS simulator running iOS version 17.5.
const K_DISABLED_OPACITY: f64 = 0.5;
const K_THUMB_RADIUS: f64 = 14.0;
const K_TRACK_HEIGHT: f64 = 31.0;
const K_TRACK_WIDTH: f64 = 51.0;
const K_SWITCH_SIZE: Size = Size::new(59.0, 39.0);
const K_THUMB_EXTENSION_FACTOR: f64 = 7.0;

/// Dart's `_kSwitchBoxShadows`.
fn k_switch_box_shadows() -> [BoxShadow; 2] {
    [
        BoxShadow::new(
            Color::new(0x26000000),
            Offset::new(0.0, 3.0),
            8.0,
            0.0,
            BlurStyle::Normal,
        ),
        BoxShadow::new(
            Color::new(0x0F000000),
            Offset::new(0.0, 3.0),
            1.0,
            0.0,
            BlurStyle::Normal,
        ),
    ]
}

// The drag distance (as a fraction of the track width) beyond which the switch
// must be dragged to commit the state change.
// This threshold is used when the user is dragging to a new state.
const K_DRAG_COMMIT_THRESHOLD: f64 = 0.7;

// The drag distance (as a fraction of the track width) past which the user must
// drag back to reverse a state change that has already been visually committed
// during the drag.
// This threshold is used when the user is dragging back to the original state.
const K_DRAG_REVERSE_THRESHOLD: f64 = 0.2;

// Label sizes and padding taken from xcode inspector.
// See https://github.com/flutter/flutter/issues/4830#issuecomment-528495360.
const K_ON_LABEL_WIDTH: f64 = 1.0;
const K_ON_LABEL_HEIGHT: f64 = 10.0;
const K_ON_LABEL_PADDING_HORIZONTAL: f64 = 11.0;
const K_OFF_LABEL_WIDTH: f64 = 1.0;
const K_OFF_LABEL_PADDING_HORIZONTAL: f64 = 12.0;
const K_OFF_LABEL_RADIUS: f64 = 5.0;
const K_OFF_LABEL_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness_and_contrast(
        // Source: https://github.com/flutter/flutter/pull/39993#discussion_r321946033
        Color::from_argb(255, 179, 179, 179),
        // Source: https://github.com/flutter/flutter/pull/39993#issuecomment-535196665
        Color::from_argb(255, 179, 179, 179),
        // Source: https://github.com/flutter/flutter/pull/127776#discussion_r1244208264
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 255, 255, 255),
    )
    .debug_label("offSwitchLabel");
const K_OFF_LABEL_COLOR: AnyColor = K_OFF_LABEL_COLOR_DYNAMIC.to_any();

/// Dart's deprecated `Color.withOpacity`, deprecated here too: its rounding to an 8-bit alpha
/// is part of the behaviour being ported.
#[allow(deprecated)]
fn with_opacity(color: Color, opacity: f64) -> Color {
    color.with_opacity(opacity)
}

/// An iOS-style switch.
///
/// Used to toggle the on/off state of a single setting.
///
/// The switch itself does not maintain its toggle state. Instead, when the toggle state of
/// the switch changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most
/// widgets that use a switch will listen for the [`on_changed`](Self::on_changed) callback
/// and rebuild the switch with a new [`value`](Self::value) to update the visual appearance
/// of the switch.
///
/// See also:
///
///  * `Switch`, the Material Design equivalent.
///  * <https://developer.apple.com/design/human-interface-guidelines/toggles/>
#[derive(Clone)]
pub struct CupertinoSwitch {
    pub key: Option<KeyRef>,
    /// Whether this switch is on or off.
    pub value: bool,
    /// Called when the user toggles the switch on or off.
    ///
    /// The switch passes the new value to the callback but does not actually change state
    /// until the parent widget rebuilds the switch with the new value.
    ///
    /// If null, the switch will be displayed as disabled, which has a reduced opacity.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
    pub on_changed: Option<ValueChanged<bool>>,
    /// The color to use for the track when the switch is on.
    ///
    /// If null and [`apply_theme`](Self::apply_theme) is false, defaults to
    /// [`CupertinoColors::SYSTEM_GREEN`] in accordance to native iOS behavior. Otherwise,
    /// defaults to `CupertinoThemeData::primary_color`.
    ///
    /// See also:
    ///
    ///  * [`inactive_track_color`](Self::inactive_track_color), the color to use for the
    ///    track when the switch is off.
    pub active_track_color: Option<AnyColor>,
    /// The color to use for the track when the switch is off.
    ///
    /// Defaults to [`CupertinoColors::SECONDARY_SYSTEM_FILL`] when null.
    ///
    /// See also:
    ///
    ///  * [`active_track_color`](Self::active_track_color), the color to use for the track
    ///    when the switch is on.
    pub inactive_track_color: Option<AnyColor>,
    /// The color to use for the thumb when the switch is on.
    ///
    /// If this color is not opaque, it is blended against
    /// `CupertinoThemeData::scaffold_background_color`, so as not to see through the thumb to
    /// the track underneath.
    ///
    /// Defaults to [`CupertinoColors::WHITE`] when null.
    ///
    /// See also:
    ///
    ///  * [`inactive_thumb_color`](Self::inactive_thumb_color), the color to use for the thumb
    ///    when the switch is off.
    pub thumb_color: Option<AnyColor>,
    /// The color to use on the thumb when the switch is off.
    ///
    /// If this color is not opaque, it is blended against
    /// `CupertinoThemeData::scaffold_background_color`, so as not to see through the thumb to
    /// the track underneath.
    ///
    /// If null, defaults to [`thumb_color`](Self::thumb_color). If that is also null,
    /// [`CupertinoColors::WHITE`] is used.
    ///
    /// See also:
    ///
    ///  * [`thumb_color`](Self::thumb_color), the color to use for the thumb when the switch
    ///    is on.
    pub inactive_thumb_color: Option<AnyColor>,
    /// The color to use for the focus highlight for keyboard interactions.
    ///
    /// Defaults to the active track color with an opacity of 0.80, a brightness of 0.69 and a
    /// saturation of 0.835.
    pub focus_color: Option<AnyColor>,
    /// The color to use for the accessibility label when the switch is on.
    ///
    /// Defaults to [`CupertinoColors::WHITE`] when null.
    pub on_label_color: Option<AnyColor>,
    /// The color to use for the accessibility label when the switch is off.
    ///
    /// Defaults to `Color::from_argb(255, 179, 179, 179)` (or
    /// `Color::from_argb(255, 255, 255, 255)` in high contrast) when null.
    pub off_label_color: Option<AnyColor>,
    /// The outline color of this [`CupertinoSwitch`]'s track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// The [`CupertinoSwitch`] track has no outline by default.
    pub track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The outline width of this [`CupertinoSwitch`]'s track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// Since a [`CupertinoSwitch`] has no track outline by default, this parameter is set
    /// only if [`track_outline_color`](Self::track_outline_color) is provided.
    ///
    /// Defaults to 2.0 if a [`track_outline_color`](Self::track_outline_color) is provided.
    pub track_outline_width: Option<WidgetStatePropertyRef<Option<f64>>>,
    /// The icon to use on the thumb of this switch.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the [`CupertinoSwitch`] does not have any icons on the thumb.
    pub thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then `MouseCursor.defer` is used when the switch is disabled. When the switch
    /// is enabled, [`SystemMouseCursors::CLICK`] is used on Web, and `MouseCursor.defer` is
    /// used on other platforms.
    ///
    /// See also:
    ///
    ///  * `WidgetStateMouseCursor`, a mouse cursor that implements `WidgetStateProperty`
    ///    which is used in APIs that need to accept either a mouse cursor or a
    ///    `WidgetStateProperty`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<MouseCursorRef>>,
    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed
    /// by this widget. The widget will be focusable even if a `focus_node` is not supplied.
    /// If supplied, the given `focus_node` will be _hosted_ by this widget, but not owned.
    pub focus_node: Option<AnyFocusNode>,
    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses focus.
    pub on_focus_change: Option<ValueChanged<bool>>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    ///
    /// Defaults to false.
    pub autofocus: bool,
    /// Whether to apply the ambient `CupertinoThemeData`.
    ///
    /// If true, the track uses `CupertinoThemeData::primary_color` for the track when the
    /// switch is on.
    ///
    /// Defaults to `CupertinoThemeData::apply_theme_to_all`.
    pub apply_theme: Option<bool>,
    /// Determines the way that drag start behavior is handled.
    ///
    /// If set to [`DragStartBehavior::Start`], the drag behavior used to move the switch from
    /// on to off will begin at the position where the drag gesture won the arena. If set to
    /// [`DragStartBehavior::Down`] it will begin at the position where a down event was first
    /// detected.
    ///
    /// In general, setting this to [`DragStartBehavior::Start`] will make drag animation
    /// smoother and setting it to [`DragStartBehavior::Down`] will make drag behavior feel
    /// slightly more reactive.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    pub drag_start_behavior: DragStartBehavior,
}

impl CupertinoSwitch {
    /// Creates an iOS-style switch.
    ///
    /// The following arguments are required:
    ///
    /// * `value` determines whether this switch is on or off.
    /// * `on_changed` is called when the user toggles the switch on or off.
    ///
    /// The [`drag_start_behavior`](Self::drag_start_behavior) defaults to
    /// [`DragStartBehavior::Start`].
    pub fn new(value: bool, on_changed: Option<ValueChanged<bool>>) -> CupertinoSwitch {
        CupertinoSwitch {
            key: None,
            value,
            on_changed,
            active_track_color: None,
            inactive_track_color: None,
            thumb_color: None,
            inactive_thumb_color: None,
            focus_color: None,
            on_label_color: None,
            off_label_color: None,
            track_outline_color: None,
            track_outline_width: None,
            thumb_icon: None,
            mouse_cursor: None,
            focus_node: None,
            on_focus_change: None,
            autofocus: false,
            apply_theme: None,
            drag_start_behavior: DragStartBehavior::Start,
        }
    }

    /// Dart `CupertinoSwitch(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSwitch {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSwitch(activeTrackColor:)`.
    pub fn active_track_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.active_track_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(inactiveTrackColor:)`.
    pub fn inactive_track_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.inactive_track_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(thumbColor:)`.
    pub fn thumb_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.thumb_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(inactiveThumbColor:)`.
    pub fn inactive_thumb_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.inactive_thumb_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(focusColor:)`.
    pub fn focus_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.focus_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(onLabelColor:)`.
    pub fn on_label_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.on_label_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(offLabelColor:)`.
    pub fn off_label_color(mut self, color: impl Into<AnyColor>) -> CupertinoSwitch {
        self.off_label_color = Some(color.into());
        self
    }

    /// Dart `CupertinoSwitch(trackOutlineColor:)`.
    pub fn track_outline_color(
        mut self,
        track_outline_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CupertinoSwitch {
        self.track_outline_color = Some(track_outline_color);
        self
    }

    /// Dart `CupertinoSwitch(trackOutlineWidth:)`.
    pub fn track_outline_width(
        mut self,
        track_outline_width: WidgetStatePropertyRef<Option<f64>>,
    ) -> CupertinoSwitch {
        self.track_outline_width = Some(track_outline_width);
        self
    }

    /// Dart `CupertinoSwitch(thumbIcon:)`.
    pub fn thumb_icon(
        mut self,
        thumb_icon: WidgetStatePropertyRef<Option<Rc<Icon>>>,
    ) -> CupertinoSwitch {
        self.thumb_icon = Some(thumb_icon);
        self
    }

    /// Dart `CupertinoSwitch(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<MouseCursorRef>,
    ) -> CupertinoSwitch {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CupertinoSwitch(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> CupertinoSwitch {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoSwitch(onFocusChange:)`.
    pub fn on_focus_change(
        mut self,
        on_focus_change: impl Fn(&mut App, bool) + 'static,
    ) -> CupertinoSwitch {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `CupertinoSwitch(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoSwitch {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoSwitch(applyTheme:)`.
    pub fn apply_theme(mut self, apply_theme: bool) -> CupertinoSwitch {
        self.apply_theme = Some(apply_theme);
        self
    }

    /// Dart `CupertinoSwitch(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> CupertinoSwitch {
        self.drag_start_behavior = drag_start_behavior;
        self
    }
}

impl Debug for CupertinoSwitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoSwitch")
            .field("value", &if self.value { "on" } else { "off" })
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

impl StatefulWidget for CupertinoSwitch {
    type State = CupertinoSwitchState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoSwitchState {
        CupertinoSwitchState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            toggleable: ToggleableStateMixinData::new(),
            painter: None,
            drag_start_position: Offset::ZERO,
            drag_delta: 0.0,
            drag_value: None,
            needs_position_animation: false,
        }
    }
}

/// Dart's `_CupertinoSwitchState`.
pub struct CupertinoSwitchState {
    state: StateData<CupertinoSwitch>,
    ticker_provider: TickerProviderStateMixinData,
    toggleable: ToggleableStateMixinData,
    /// Dart's `final _SwitchPainter _painter = _SwitchPainter()`, created in `init_state`
    /// because a painter is an arena object and a state is created without the `App`.
    painter: Option<Handle<SwitchPainter>>,
    // The global position where the user first touched the screen. This value to
    // calculate the initial drag delta.
    drag_start_position: Offset,
    // The cumulative horizontal drag delta, normalized as a fraction of the
    // track width.
    drag_delta: f64,
    // The transient value of the switch determined by _dragDelta during a
    // drag.
    drag_value: Option<bool>,
    needs_position_animation: bool,
}

impl CupertinoSwitchState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<SwitchPainter> {
        app.get(self).painter.expect("created in init_state")
    }

    /// Dart's `_widgetThumbColor`.
    fn widget_thumb_color(
        self: Handle<Self>,
        app: &App,
    ) -> WidgetStatePropertyRef<Option<AnyColor>> {
        let widget = self.widget(app);
        let (thumb_color, inactive_thumb_color) = (
            widget.thumb_color.clone(),
            widget.inactive_thumb_color.clone(),
        );
        <dyn WidgetStateProperty<Option<AnyColor>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Selected) {
                return thumb_color.clone();
            }
            inactive_thumb_color.clone()
        })
    }

    /// Dart's `_widgetTrackColor`.
    fn widget_track_color(
        self: Handle<Self>,
        app: &App,
    ) -> WidgetStatePropertyRef<Option<AnyColor>> {
        let widget = self.widget(app);
        let (active_track_color, inactive_track_color) = (
            widget.active_track_color.clone(),
            widget.inactive_track_color.clone(),
        );
        <dyn WidgetStateProperty<Option<AnyColor>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Selected) {
                return active_track_color.clone();
            }
            inactive_track_color.clone()
        })
    }

    /// Dart's `_defaultMouseCursor`.
    fn default_mouse_cursor() -> WidgetStatePropertyRef<MouseCursorRef> {
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(|states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return <dyn MouseCursor>::defer();
            }
            if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                <dyn MouseCursor>::defer()
            }
        })
    }

    /// Dart's `_resolveTrackColor`: `resolve_as` answers Dart's `is WidgetStateColor` itself.
    fn resolve_track_color(
        track_color: &Option<AnyColor>,
        states: &WidgetStates,
    ) -> Option<AnyColor> {
        <dyn WidgetStateProperty<Color>>::resolve_as(track_color, states)
    }

    /// Dart's `_resolveThumbColor`: `resolve_as` answers Dart's `is WidgetStateColor` itself.
    fn resolve_thumb_color(
        thumb_color: &Option<AnyColor>,
        states: &WidgetStates,
    ) -> Option<AnyColor> {
        <dyn WidgetStateProperty<Color>>::resolve_as(thumb_color, states)
    }

    /// Dart's `_trackInnerLength`.
    fn track_inner_length() -> f64 {
        const TRACK_INNER_START: f64 = K_TRACK_HEIGHT / 2.0;
        const TRACK_INNER_END: f64 = K_TRACK_WIDTH - TRACK_INNER_START;
        const TRACK_INNER_LENGTH: f64 = TRACK_INNER_END - TRACK_INNER_START;
        TRACK_INNER_LENGTH
    }

    /// The drag delta in the reading direction: Dart's `switch (Directionality.of(context))`.
    fn directional(self: Handle<Self>, app: &mut App, delta: f64) -> f64 {
        let context = self.context(app);
        match Directionality::of(app, context) {
            TextDirection::Rtl => -delta,
            TextDirection::Ltr => delta,
        }
    }

    fn handle_on_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        if self.is_interactive(app) {
            app.get_mut(self).drag_start_position = details.global_position;
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, details: DragStartDetails) {
        if self.is_interactive(app) {
            self.reaction_controller(app).forward(app, None);

            let drag_start_position = app.get(self).drag_start_position;
            if drag_start_position != Offset::ZERO {
                let delta = (details.global_position - drag_start_position).dx() / K_TRACK_WIDTH;
                app.get_mut(self).drag_delta = self.directional(app, delta);
            }

            app.get_mut(self).drag_value = ToggleableStateMixin::value(self, app);
        }
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        if self.is_interactive(app) {
            let delta = details
                .primary_delta
                .expect("a horizontal drag has a primary delta")
                / K_TRACK_WIDTH;
            let delta = self.directional(app, delta);
            app.get_mut(self).drag_delta += delta;

            let widget_value = self.widget(app).value;
            let (drag_delta, drag_value) = {
                let state = app.get(self);
                (state.drag_delta, state.drag_value)
            };
            let value_changed_while_dragging = Some(widget_value) != drag_value;

            let threshold = if value_changed_while_dragging {
                K_DRAG_REVERSE_THRESHOLD
            } else {
                K_DRAG_COMMIT_THRESHOLD
            };
            let effective_threshold = if widget_value { -threshold } else { threshold };

            let new_drag_value = drag_delta >= effective_threshold;

            if drag_value != Some(new_drag_value) {
                Self::emit_vibration(app);

                let position_controller = self.position_controller(app);
                if new_drag_value {
                    position_controller.forward(app, None);
                } else {
                    position_controller.reverse(app, None);
                }

                app.get_mut(self).drag_value = Some(new_drag_value);
            }
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, _details: DragEndDetails) {
        let widget_value = self.widget(app).value;
        if app.get(self).drag_value != Some(widget_value) {
            if let Some(on_changed) = self.widget(app).on_changed.clone() {
                on_changed(app, !widget_value);
            }
            // Wait to finish the animation until widget.value has changed to
            // !widget.value as part of the widget.onChanged call above.
            self.set_state(app, |state| state.needs_position_animation = true);
        }

        let state = app.get_mut(self);
        state.drag_start_position = Offset::ZERO;
        state.drag_delta = 0.0;
        state.drag_value = None;

        self.reaction_controller(app).reverse(app, None);
    }

    fn handle_changed(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        debug_assert!(value.is_some());
        let on_changed = self.widget(app).on_changed.clone();
        debug_assert!(on_changed.is_some());
        if let Some(on_changed) = on_changed {
            on_changed(app, value.expect("a two-state toggle"));
        }
        Self::emit_vibration(app);
    }

    fn emit_vibration(app: &App) {
        match app.platform().target_platform() {
            TargetPlatform::IOS => HapticFeedback::light_impact(app),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {}
        }
    }

    /// Dart's `HSLColor.fromColor(activeColor.withOpacity(...))...toColor()`, the
    /// [`focus_color`](CupertinoSwitch::focus_color) default.
    fn default_focus_color(active_color: Color) -> Color {
        HSLColor::from_color(with_opacity(active_color, K_CUPERTINO_FOCUS_COLOR_OPACITY))
            .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
            .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
            .to_color()
    }
}

impl TickerProviderStateMixin for CupertinoSwitchState {
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
impl TickerProviderObject for CupertinoSwitchState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ToggleableStateMixin for CupertinoSwitchState {
    fn toggleable_data(self: Handle<Self>, app: &App) -> &ToggleableStateMixinData {
        &app.get(self).toggleable
    }

    fn toggleable_data_mut(self: Handle<Self>, app: &mut App) -> &mut ToggleableStateMixinData {
        &mut app.get_mut(self).toggleable
    }

    fn on_changed(self: Handle<Self>, app: &App) -> Option<ValueChanged<Option<bool>>> {
        self.widget(app).on_changed.as_ref()?;
        Some(Rc::new(move |app: &mut App, value| {
            self.handle_changed(app, value)
        }))
    }

    fn tristate(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn value(self: Handle<Self>, app: &App) -> Option<bool> {
        Some(self.widget(app).value)
    }
}

impl State for CupertinoSwitchState {
    type Widget = CupertinoSwitch;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let painter = SwitchPainter::new(app);
        app.get_mut(self).painter = Some(painter);
        ToggleableStateMixin::init_state(self, app);
        let position_controller = self.position_controller(app);
        app.get_mut(position_controller).duration = Some(Duration::from_millis(200));
        let reaction_controller = self.reaction_controller(app);
        app.get_mut(reaction_controller).duration = Some(Duration::from_millis(300));
        let position = app.get_mut(self.position(app));
        position.curve = Curves::ease();
        position.reverse_curve = Some(Rc::new(Curves::ease().flipped()) as Rc<dyn Curve>);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoSwitch) {
        if old_widget.value != self.widget(app).value {
            self.animate_to_value(app);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let painter = self.painter(app);
        painter.dispose(app);
        app.get_mut(self).painter = None;
        app.destroy(painter);
        ToggleableStateMixin::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        if app.get(self).needs_position_animation {
            app.get_mut(self).needs_position_animation = false;
            self.animate_to_value(app);
        }

        let theme = CupertinoTheme::of(app, context);
        let widget = self.widget(app).clone();

        let apply_theme = widget
            .apply_theme
            .unwrap_or_else(|| theme.apply_theme_to_all());
        let active_color = CupertinoDynamicColor::resolve(
            &widget
                .active_track_color
                .clone()
                .or_else(|| apply_theme.then(|| theme.primary_color()))
                .unwrap_or(CupertinoColors::SYSTEM_GREEN),
            app,
            context,
        )
        .color();

        let on_off_label_colors = if MediaQuery::on_off_switch_labels_of(app, context) {
            Some((
                CupertinoDynamicColor::resolve(
                    &widget
                        .on_label_color
                        .clone()
                        .unwrap_or(CupertinoColors::WHITE),
                    app,
                    context,
                )
                .color(),
                CupertinoDynamicColor::resolve(
                    &widget.off_label_color.clone().unwrap_or(K_OFF_LABEL_COLOR),
                    app,
                    context,
                )
                .color(),
            ))
        } else {
            None
        };

        // Colors need to be resolved in selected and non selected states separately
        // so that they can be lerped between.
        let mut active_states = self.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = self.states(app);
        inactive_states.remove(&WidgetState::Selected);

        let widget_thumb_color = self.widget_thumb_color(app);
        let effective_active_thumb_color =
            Self::resolve_thumb_color(&widget.thumb_color, &active_states)
                .or_else(|| widget_thumb_color.resolve(&active_states))
                .unwrap_or(CupertinoColors::WHITE)
                .color();

        let effective_inactive_thumb_color =
            Self::resolve_thumb_color(&widget.inactive_thumb_color, &inactive_states)
                .or_else(|| widget_thumb_color.resolve(&inactive_states))
                .map_or(effective_active_thumb_color, |color| color.color());

        let effective_active_track_color = self
            .widget_track_color(app)
            .resolve(&active_states)
            .map_or(active_color, |color| color.color());

        let effective_active_track_outline_color = widget
            .track_outline_color
            .as_ref()
            .and_then(|color| color.resolve(&active_states));

        let effective_active_track_outline_width = widget
            .track_outline_width
            .as_ref()
            .and_then(|width| width.resolve(&active_states));

        // Dart reads `widget.trackColor`, the deprecated getter for `inactiveTrackColor`.
        let effective_inactive_track_color =
            Self::resolve_track_color(&widget.inactive_track_color, &inactive_states)
                .unwrap_or_else(|| {
                    CupertinoDynamicColor::resolve(
                        &CupertinoColors::SECONDARY_SYSTEM_FILL,
                        app,
                        context,
                    )
                })
                .color();

        let effective_inactive_track_outline_color = widget
            .track_outline_color
            .as_ref()
            .and_then(|color| color.resolve(&inactive_states));

        let effective_inactive_track_outline_width = widget
            .track_outline_width
            .as_ref()
            .and_then(|width| width.resolve(&inactive_states));

        let effective_active_icon = widget
            .thumb_icon
            .as_ref()
            .and_then(|icon| icon.resolve(&active_states));

        let effective_inactive_icon = widget
            .thumb_icon
            .as_ref()
            .and_then(|icon| icon.resolve(&inactive_states));

        let effective_active_icon_color = effective_active_icon
            .as_ref()
            .and_then(|icon| icon.color.as_ref())
            .unwrap_or(&CupertinoColors::BLACK)
            .color();

        let effective_inactive_icon_color = effective_inactive_icon
            .as_ref()
            .and_then(|icon| icon.color.as_ref())
            .unwrap_or(&CupertinoColors::BLACK)
            .color();

        let mut active_pressed_states = active_states;
        active_pressed_states.insert(WidgetState::Pressed);
        let effective_active_pressed_thumb_color =
            Self::resolve_thumb_color(&widget.thumb_color, &active_pressed_states)
                .or_else(|| widget_thumb_color.resolve(&active_pressed_states))
                .unwrap_or(CupertinoColors::WHITE)
                .color();

        let mut inactive_pressed_states = inactive_states;
        inactive_pressed_states.insert(WidgetState::Pressed);
        let effective_inactive_pressed_thumb_color =
            Self::resolve_thumb_color(&widget.thumb_color, &inactive_pressed_states)
                .or_else(|| widget_thumb_color.resolve(&inactive_pressed_states))
                .unwrap_or(CupertinoColors::WHITE)
                .color();

        let effective_mouse_cursor = widget
            .mouse_cursor
            .clone()
            .unwrap_or_else(Self::default_mouse_cursor);

        let focus_color = CupertinoDynamicColor::resolve(
            &widget
                .focus_color
                .clone()
                .unwrap_or_else(|| AnyColor::new(Self::default_focus_color(active_color))),
            app,
            context,
        )
        .color();
        let configuration = create_local_image_configuration(app, context, None);
        let text_direction = Directionality::of(app, context);
        let icon_theme = IconTheme::of(app, context);
        let states = self.states(app);

        let painter = self.painter(app);
        painter.set_position(app, self.position(app).as_animation());
        painter.set_reaction(app, self.reaction(app).as_animation());
        painter.set_reaction_focus_fade(app, self.reaction_focus_fade(app).as_animation());
        painter.set_reaction_hover_fade(app, self.reaction_hover_fade(app).as_animation());
        painter.set_focus_color(app, focus_color);
        painter.set_down_position(app, self.down_position(app));
        painter.set_is_focused(app, Some(states.contains(&WidgetState::Focused)));
        painter.set_is_hovered(app, Some(states.contains(&WidgetState::Hovered)));
        painter.set_active_color(app, effective_active_thumb_color);
        painter.set_inactive_color(app, effective_inactive_thumb_color);
        painter.set_active_pressed_color(app, effective_active_pressed_thumb_color);
        painter.set_on_off_label_colors(app, on_off_label_colors);
        painter.set_inactive_pressed_color(app, effective_inactive_pressed_thumb_color);
        painter.set_active_track_color(app, effective_active_track_color);
        painter.set_active_track_outline_color(app, effective_active_track_outline_color);
        painter.set_active_track_outline_width(app, effective_active_track_outline_width);
        painter.set_inactive_track_color(app, effective_inactive_track_color);
        painter.set_inactive_track_outline_color(app, effective_inactive_track_outline_color);
        painter.set_inactive_track_outline_width(app, effective_inactive_track_outline_width);
        painter.set_configuration(app, configuration);
        painter.set_is_interactive(app, self.is_interactive(app));
        painter.set_track_inner_length(app, Self::track_inner_length());
        painter.set_text_direction(app, text_direction);
        painter.set_active_icon_color(app, effective_active_icon_color);
        painter.set_inactive_icon_color(app, effective_inactive_icon_color);
        painter.set_active_icon(app, effective_active_icon);
        painter.set_inactive_icon(app, effective_inactive_icon);
        painter.set_icon_theme(app, Some(icon_theme));
        painter.set_surface_color(app, theme.scaffold_background_color().color());
        painter.set_position_controller(app, self.position_controller(app));

        let opacity = if ToggleableStateMixin::on_changed(self, app).is_none() {
            K_DISABLED_OPACITY
        } else {
            1.0
        };
        let toggleable = self.build_toggleable(
            app,
            widget.focus_node,
            widget.on_focus_change.clone(),
            widget.autofocus,
            Some(effective_mouse_cursor),
            K_SWITCH_SIZE,
            ToggleablePainterRef::new(painter),
        );
        // `Semantics(toggled: widget.value)` and the detector's `excludeFromSemantics` wait
        // with accessibility.
        GestureDetector::new()
            .on_tap_down(Rc::new(move |app: &mut App, details| {
                self.handle_on_tap_down(app, details)
            }))
            .on_horizontal_drag_start(Rc::new(move |app: &mut App, details| {
                self.handle_drag_start(app, details)
            }))
            .on_horizontal_drag_update(Rc::new(move |app: &mut App, details| {
                self.handle_drag_update(app, details)
            }))
            .on_horizontal_drag_end(Rc::new(move |app: &mut App, details| {
                self.handle_drag_end(app, details)
            }))
            .drag_start_behavior(widget.drag_start_behavior)
            .child(Opacity::new(opacity).child(toggleable))
            .into_widget()
    }
}

/// Dart's `_SwitchPainter`.
struct SwitchPainter {
    change_notifier: ChangeNotifierData,
    toggleable_painter: ToggleablePainterData,
    position_controller: Option<Handle<AnimationController>>,
    color_animation: Option<Handle<CurvedAnimation>>,
    active_icon: Option<Rc<Icon>>,
    inactive_icon: Option<Rc<Icon>>,
    icon_theme: Option<IconThemeData>,
    active_icon_color: Option<Color>,
    inactive_icon_color: Option<Color>,
    active_pressed_color: Option<Color>,
    inactive_pressed_color: Option<Color>,
    active_track_color: Option<Color>,
    active_track_outline_color: Option<Color>,
    inactive_track_outline_color: Option<Color>,
    active_track_outline_width: Option<f64>,
    inactive_track_outline_width: Option<f64>,
    inactive_track_color: Option<Color>,
    configuration: Option<ImageConfiguration>,
    text_direction: Option<TextDirection>,
    surface_color: Option<Color>,
    is_interactive: Option<bool>,
    track_inner_length: Option<f64>,
    on_off_label_colors: Option<(Color, Color)>,
    text_painter: TextPainter,
    cached_thumb_color: Option<Color>,
    cached_thumb_painter: Option<Box<dyn BoxPainter>>,
    /// Shared with the thumb decoration's change callback, which reads it while the painter
    /// is borrowed for painting.
    is_painting: Rc<Cell<bool>>,
    stop_press_animation: bool,
    pressed_thumb_extension: Option<f64>,
}

/// Dart's `Icon?` setters compare by identity: an `Icon` does not override `==`.
fn same_icon(a: &Option<Rc<Icon>>, b: &Option<Rc<Icon>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl SwitchPainter {
    fn new(app: &mut App) -> Handle<SwitchPainter> {
        app.create(SwitchPainter {
            change_notifier: ChangeNotifierData::new(),
            toggleable_painter: ToggleablePainterData::new(),
            position_controller: None,
            color_animation: None,
            active_icon: None,
            inactive_icon: None,
            icon_theme: None,
            active_icon_color: None,
            inactive_icon_color: None,
            active_pressed_color: None,
            inactive_pressed_color: None,
            active_track_color: None,
            active_track_outline_color: None,
            inactive_track_outline_color: None,
            active_track_outline_width: None,
            inactive_track_outline_width: None,
            inactive_track_color: None,
            configuration: None,
            text_direction: None,
            surface_color: None,
            is_interactive: None,
            track_inner_length: None,
            on_off_label_colors: None,
            text_painter: TextPainter::new(),
            cached_thumb_color: None,
            cached_thumb_painter: None,
            is_painting: Rc::new(Cell::new(false)),
            stop_press_animation: false,
            pressed_thumb_extension: None,
        })
    }

    fn position_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .position_controller
            .expect("set before painting")
    }

    fn set_position_controller(
        self: Handle<Self>,
        app: &mut App,
        value: Handle<AnimationController>,
    ) {
        if app.get(self).position_controller == Some(value) {
            return;
        }
        app.get_mut(self).position_controller = Some(value);
        if let Some(color_animation) = app.get_mut(self).color_animation.take() {
            color_animation.dispose(app);
            app.destroy(color_animation);
        }
        let color_animation = CurvedAnimation::create(
            app,
            value.as_animation(),
            Curves::ease_out(),
            Some(Curves::ease_in()),
        );
        app.get_mut(self).color_animation = Some(color_animation);
        self.notify_listeners(app);
    }

    fn active_icon(self: Handle<Self>, app: &App) -> Option<Rc<Icon>> {
        app.get(self).active_icon.clone()
    }

    fn set_active_icon(self: Handle<Self>, app: &mut App, value: Option<Rc<Icon>>) {
        if same_icon(&value, &app.get(self).active_icon) {
            return;
        }
        app.get_mut(self).active_icon = value;
        self.notify_listeners(app);
    }

    fn inactive_icon(self: Handle<Self>, app: &App) -> Option<Rc<Icon>> {
        app.get(self).inactive_icon.clone()
    }

    fn set_inactive_icon(self: Handle<Self>, app: &mut App, value: Option<Rc<Icon>>) {
        if same_icon(&value, &app.get(self).inactive_icon) {
            return;
        }
        app.get_mut(self).inactive_icon = value;
        self.notify_listeners(app);
    }

    fn icon_theme(self: Handle<Self>, app: &App) -> Option<&IconThemeData> {
        app.get(self).icon_theme.as_ref()
    }

    fn set_icon_theme(self: Handle<Self>, app: &mut App, value: Option<IconThemeData>) {
        if value == app.get(self).icon_theme {
            return;
        }
        app.get_mut(self).icon_theme = value;
        self.notify_listeners(app);
    }

    fn active_icon_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .active_icon_color
            .expect("set before painting")
    }

    fn set_active_icon_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).active_icon_color {
            return;
        }
        app.get_mut(self).active_icon_color = Some(value);
        self.notify_listeners(app);
    }

    fn inactive_icon_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .inactive_icon_color
            .expect("set before painting")
    }

    fn set_inactive_icon_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).inactive_icon_color {
            return;
        }
        app.get_mut(self).inactive_icon_color = Some(value);
        self.notify_listeners(app);
    }

    fn active_pressed_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .active_pressed_color
            .expect("set before painting")
    }

    fn set_active_pressed_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).active_pressed_color {
            return;
        }
        app.get_mut(self).active_pressed_color = Some(value);
        self.notify_listeners(app);
    }

    fn inactive_pressed_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .inactive_pressed_color
            .expect("set before painting")
    }

    fn set_inactive_pressed_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).inactive_pressed_color {
            return;
        }
        app.get_mut(self).inactive_pressed_color = Some(value);
        self.notify_listeners(app);
    }

    fn active_track_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .active_track_color
            .expect("set before painting")
    }

    fn set_active_track_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).active_track_color {
            return;
        }
        app.get_mut(self).active_track_color = Some(value);
        self.notify_listeners(app);
    }

    fn active_track_outline_color(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).active_track_outline_color
    }

    fn set_active_track_outline_color(self: Handle<Self>, app: &mut App, value: Option<Color>) {
        if value == app.get(self).active_track_outline_color {
            return;
        }
        app.get_mut(self).active_track_outline_color = value;
        self.notify_listeners(app);
    }

    fn inactive_track_outline_color(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).inactive_track_outline_color
    }

    fn set_inactive_track_outline_color(self: Handle<Self>, app: &mut App, value: Option<Color>) {
        if value == app.get(self).inactive_track_outline_color {
            return;
        }
        app.get_mut(self).inactive_track_outline_color = value;
        self.notify_listeners(app);
    }

    fn active_track_outline_width(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).active_track_outline_width
    }

    fn set_active_track_outline_width(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        if value == app.get(self).active_track_outline_width {
            return;
        }
        app.get_mut(self).active_track_outline_width = value;
        self.notify_listeners(app);
    }

    fn inactive_track_outline_width(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).inactive_track_outline_width
    }

    fn set_inactive_track_outline_width(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        if value == app.get(self).inactive_track_outline_width {
            return;
        }
        app.get_mut(self).inactive_track_outline_width = value;
        self.notify_listeners(app);
    }

    fn inactive_track_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self)
            .inactive_track_color
            .expect("set before painting")
    }

    fn set_inactive_track_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).inactive_track_color {
            return;
        }
        app.get_mut(self).inactive_track_color = Some(value);
        self.notify_listeners(app);
    }

    fn configuration(self: Handle<Self>, app: &App) -> &ImageConfiguration {
        app.get(self)
            .configuration
            .as_ref()
            .expect("set before painting")
    }

    fn set_configuration(self: Handle<Self>, app: &mut App, value: ImageConfiguration) {
        if app.get(self).configuration.as_ref() == Some(&value) {
            return;
        }
        app.get_mut(self).configuration = Some(value);
        self.notify_listeners(app);
    }

    fn text_direction(self: Handle<Self>, app: &App) -> TextDirection {
        app.get(self).text_direction.expect("set before painting")
    }

    fn set_text_direction(self: Handle<Self>, app: &mut App, value: TextDirection) {
        if app.get(self).text_direction == Some(value) {
            return;
        }
        app.get_mut(self).text_direction = Some(value);
        self.notify_listeners(app);
    }

    fn surface_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).surface_color.expect("set before painting")
    }

    fn set_surface_color(self: Handle<Self>, app: &mut App, value: Color) {
        if Some(value) == app.get(self).surface_color {
            return;
        }
        app.get_mut(self).surface_color = Some(value);
        self.notify_listeners(app);
    }

    #[expect(
        dead_code,
        reason = "Dart's getter; only the setter is called, from the state's build"
    )]
    fn is_interactive(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_interactive.expect("set before painting")
    }

    fn set_is_interactive(self: Handle<Self>, app: &mut App, value: bool) {
        if Some(value) == app.get(self).is_interactive {
            return;
        }
        app.get_mut(self).is_interactive = Some(value);
        self.notify_listeners(app);
    }

    fn track_inner_length(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .track_inner_length
            .expect("set before painting")
    }

    fn set_track_inner_length(self: Handle<Self>, app: &mut App, value: f64) {
        if Some(value) == app.get(self).track_inner_length {
            return;
        }
        app.get_mut(self).track_inner_length = Some(value);
        self.notify_listeners(app);
    }

    fn on_off_label_colors(self: Handle<Self>, app: &App) -> Option<(Color, Color)> {
        app.get(self).on_off_label_colors
    }

    fn set_on_off_label_colors(self: Handle<Self>, app: &mut App, value: Option<(Color, Color)>) {
        if value == app.get(self).on_off_label_colors {
            return;
        }
        app.get_mut(self).on_off_label_colors = value;
        self.notify_listeners(app);
    }

    /// Dart's `_createDefaultThumbDecoration`, without the thumb image (see `PORTING.md`).
    fn create_default_thumb_decoration(color: Color) -> ShapeDecoration {
        ShapeDecoration::new(StadiumBorder::new(BorderSide::NONE)).color(color)
    }

    /// Dart's `_handleDecorationChanged`, the thumb decoration's change callback.
    ///
    /// If the image decoration is available synchronously, we'll get called here during
    /// paint. There's no reason to mark ourselves as needing paint if we are already in the
    /// middle of painting. (In fact, doing so would trigger an assert).
    fn handle_decoration_changed(self: Handle<Self>, app: &mut App, is_painting: &Cell<bool>) {
        if !is_painting.get() {
            self.notify_listeners(app);
        }
    }

    /// Computes canvas offset for track's upper left corner.
    fn compute_track_paint_offset(canvas_size: Size) -> Offset {
        let horizontal_offset = (canvas_size.width() - K_TRACK_WIDTH) / 2.0;
        let vertical_offset = (canvas_size.height() - K_TRACK_HEIGHT) / 2.0;

        Offset::new(horizontal_offset, vertical_offset)
    }

    /// Computes canvas offset for thumb's upper left corner as if it were a square.
    fn compute_thumb_paint_offset(
        self: Handle<Self>,
        app: &App,
        track_paint_offset: Offset,
        thumb_size: Size,
        visual_position: f64,
    ) -> Offset {
        // How much the thumb radius extends beyond the track.
        const TRACK_RADIUS: f64 = K_TRACK_HEIGHT / 2.0;
        let additional_thumb_radius = thumb_size.height() / 2.0 - TRACK_RADIUS;

        let pressed_thumb_extension = app
            .get(self)
            .pressed_thumb_extension
            .expect("set at the start of paint");
        let horizontal_progress =
            visual_position * (self.track_inner_length(app) - pressed_thumb_extension);
        let thumb_horizontal_offset =
            track_paint_offset.dx() + TRACK_RADIUS + (pressed_thumb_extension / 2.0)
                - thumb_size.width() / 2.0
                + horizontal_progress;
        let thumb_vertical_offset = track_paint_offset.dy() - additional_thumb_radius;
        Offset::new(thumb_horizontal_offset, thumb_vertical_offset)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_track_with(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        paint: &Paint,
        track_paint_offset: Offset,
        track_outline_color: Option<Color>,
        track_outline_width: Option<f64>,
        track_rect: Rect,
    ) {
        const TRACK_RADIUS: f64 = K_TRACK_HEIGHT / 2.0;
        let track_rrect = RRect::from_rect_and_radius(track_rect, Radius::circular(TRACK_RADIUS));

        draw_rrect(canvas, track_rrect, paint);

        // Paint the track outline.
        if let Some(track_outline_color) = track_outline_color {
            let outline_track_rect = Rect::from_ltwh(
                track_paint_offset.dx() + 1.0,
                track_paint_offset.dy() + 1.0,
                K_TRACK_WIDTH - 2.0,
                K_TRACK_HEIGHT - 2.0,
            );
            let outline_track_rrect =
                RRect::from_rect_and_radius(outline_track_rect, Radius::circular(TRACK_RADIUS));

            let outline_paint = Paint {
                style: PaintStyle::Stroke(Stroke::new(track_outline_width.unwrap_or(2.0) as f32)),
                color: track_outline_color.into(),
                ..Paint::default()
            };

            draw_rrect(canvas, outline_track_rrect, &outline_paint);
        }

        if self.is_focused(app) {
            let focused_outline = track_rrect.inflate(1.75);
            let focused_paint = Paint {
                style: PaintStyle::Stroke(Stroke::new(3.5)),
                color: self.focus_color(app).into(),
                ..Paint::default()
            };
            draw_rrect(canvas, focused_outline, &focused_paint);
        }
        canvas.clip_rrect_radii_elliptical(
            track_rrect.outer_rect(),
            rrect_radii_elliptical(track_rrect),
            ClipOp::Intersect,
            true,
        );
    }

    /// Draws the on (`|`) and off (`O`) labels, when the platform asks for them.
    fn paint_on_off_labels(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        visual_position: f64,
        track_rect: Rect,
    ) {
        let current_reaction_value = self.reaction(app).value(app);
        let Some((on_label_color, off_label_color)) = self.on_off_label_colors(app) else {
            return;
        };

        let left_label_opacity = visual_position * (1.0 - current_reaction_value);
        let right_label_opacity = (1.0 - visual_position) * (1.0 - current_reaction_value);
        let (on_label_opacity, off_label_opacity) = match self.text_direction(app) {
            TextDirection::Ltr => (left_label_opacity, right_label_opacity),
            TextDirection::Rtl => (right_label_opacity, left_label_opacity),
        };

        let (on_label_offset, off_label_offset) = match self.text_direction(app) {
            TextDirection::Ltr => (
                track_rect
                    .center_left()
                    .translate(K_ON_LABEL_PADDING_HORIZONTAL, 0.0),
                track_rect
                    .center_right()
                    .translate(-K_OFF_LABEL_PADDING_HORIZONTAL, 0.0),
            ),
            TextDirection::Rtl => (
                track_rect
                    .center_right()
                    .translate(-K_ON_LABEL_PADDING_HORIZONTAL, 0.0),
                track_rect
                    .center_left()
                    .translate(K_OFF_LABEL_PADDING_HORIZONTAL, 0.0),
            ),
        };

        // Draws '|' label.
        let on_label_rect = Rect::from_center(on_label_offset, K_ON_LABEL_WIDTH, K_ON_LABEL_HEIGHT);
        let on_label_paint = Paint {
            color: with_opacity(on_label_color, on_label_opacity).into(),
            style: PaintStyle::Fill,
            ..Paint::default()
        };
        canvas.draw_rect(on_label_rect, &on_label_paint);

        // Draws 'O' label.
        let off_label_paint = Paint {
            color: with_opacity(off_label_color, off_label_opacity).into(),
            style: PaintStyle::Stroke(Stroke::new(K_OFF_LABEL_WIDTH as f32)),
            ..Paint::default()
        };
        canvas.draw_circle(
            off_label_offset,
            K_OFF_LABEL_RADIUS as f32,
            &off_label_paint,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_thumb_with(
        self: Handle<Self>,
        app: &mut App,
        canvas: &mut Canvas,
        thumb_paint_offset: Offset,
        current_value: f64,
        thumb_color: Color,
        thumb_icon: Option<Rc<Icon>>,
        thumb_size: Size,
    ) {
        let is_painting = Rc::clone(&app.get(self).is_painting);
        let _painting = PaintingGuard::enter(&is_painting);
        self.update_cached_thumb_painter(app, thumb_color);

        self.paint_cupertino_thumb_shadow_and_border(canvas, thumb_paint_offset, thumb_size);

        let configuration =
            self.configuration(app)
                .copy_with(None, None, None, None, Some(thumb_size), None);
        app.get_mut(self)
            .cached_thumb_painter
            .as_mut()
            .expect("created above")
            .paint(canvas, thumb_paint_offset, &configuration);

        if let Some(thumb_icon) = thumb_icon
            && let Some(icon_data) = &thumb_icon.icon
        {
            let icon_color = Color::lerp(
                Some(self.inactive_icon_color(app)),
                Some(self.active_icon_color(app)),
                current_value,
            )
            .expect("lerping non-null colors");
            let icon_size = thumb_icon.size.unwrap_or(16.0);
            let icon_theme = self.icon_theme(app);
            let icon_weight = thumb_icon
                .weight
                .or(icon_theme.and_then(|theme| theme.weight));
            let icon_fill = thumb_icon.fill.or(icon_theme.and_then(|theme| theme.fill));
            let icon_grade = thumb_icon
                .grade
                .or(icon_theme.and_then(|theme| theme.grade));
            let icon_optical_size = thumb_icon
                .optical_size
                .or(icon_theme.and_then(|theme| theme.optical_size));
            let icon_shadows = thumb_icon
                .shadows
                .clone()
                .or_else(|| icon_theme.and_then(|theme| theme.shadows.clone()));

            let font_variations = [
                ("FILL", icon_fill),
                ("wght", icon_weight),
                ("GRAD", icon_grade),
                ("opsz", icon_optical_size),
            ]
            .into_iter()
            .filter_map(|(axis, value)| value.map(|value| FontVariation::new(axis, value)))
            .collect();
            let mut style = TextStyle::new()
                .font_variations(font_variations)
                .color(icon_color)
                .font_size(icon_size)
                .inherit(false);
            if let Some(font_family) = &icon_data.font_family {
                style = style.font_family(font_family.clone());
            }
            if let Some(font_package) = &icon_data.font_package {
                style = style.package(font_package.clone());
            }
            if let Some(icon_shadows) = icon_shadows {
                style = style.shadows(icon_shadows);
            }
            let code_point = char::from_u32(icon_data.code_point)
                .expect("an IconData code point is a Unicode scalar value");
            let text_span = TextSpan::new().text(code_point.to_string()).style(style);
            let additional_horizontal_offset = (thumb_size.width() - icon_size) / 2.0;
            let additional_vertical_offset = (thumb_size.height() - icon_size) / 2.0;
            let offset = thumb_paint_offset
                + Offset::new(additional_horizontal_offset, additional_vertical_offset);
            self.paint_icon(app, canvas, text_span, offset);
        }
    }

    /// The cache check at the top of Dart's `_paintThumbWith`: a fresh box painter whenever
    /// the thumb colour changed.
    fn update_cached_thumb_painter(self: Handle<Self>, app: &mut App, thumb_color: Color) {
        let painter = app.get_mut(self);
        if painter.cached_thumb_painter.is_some() && painter.cached_thumb_color == Some(thumb_color)
        {
            return;
        }
        painter.cached_thumb_color = Some(thumb_color);
        if let Some(mut old) = painter.cached_thumb_painter.take() {
            old.dispose();
        }
        let is_painting = Rc::clone(&painter.is_painting);
        let on_changed: Rc<dyn Fn(&mut App)> =
            Rc::new(move |app: &mut App| self.handle_decoration_changed(app, &is_painting));
        app.get_mut(self).cached_thumb_painter = Some(
            Self::create_default_thumb_decoration(thumb_color).create_box_painter(Some(on_changed)),
        );
    }

    /// Lays out and paints the thumb icon's glyph: the `_textPainter` lines of Dart's
    /// `_paintThumbWith`.
    fn paint_icon(
        self: Handle<Self>,
        app: &mut App,
        canvas: &mut Canvas,
        text: TextSpan,
        offset: Offset,
    ) {
        let text_direction = self.text_direction(app);
        let fonts = PaintingBinding::instance(app).fonts(app);
        let (painter, fonts) = app.get_disjoint_mut(self, fonts);
        painter
            .text_painter
            .set_text_direction(Some(text_direction));
        painter.text_painter.set_text(Some(Rc::new(text)));
        painter.text_painter.layout(fonts, 0.0, f64::INFINITY);
        painter.text_painter.paint(fonts, canvas, offset);
    }

    fn paint_cupertino_thumb_shadow_and_border(
        self: Handle<Self>,
        canvas: &mut Canvas,
        thumb_paint_offset: Offset,
        thumb_size: Size,
    ) {
        let thumb_bounds = RRect::from_ltrbr(
            thumb_paint_offset.dx(),
            thumb_paint_offset.dy(),
            thumb_paint_offset.dx() + thumb_size.width(),
            thumb_paint_offset.dy() + thumb_size.height(),
            Radius::circular(thumb_size.height() / 2.0),
        );
        for shadow in k_switch_box_shadows() {
            draw_rrect(
                canvas,
                thumb_bounds.shift(shadow.offset),
                &shadow.to_paint(),
            );
        }
        draw_rrect(
            canvas,
            thumb_bounds.inflate(0.5),
            &Paint::from_color(Color::new(0x0A000000).into()),
        );
    }

    /// Dart's `_SwitchPainter.dispose`, which ends in `ToggleablePainter`'s.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let painter = app.get_mut(self);
        painter.text_painter.dispose();
        if let Some(mut cached_thumb_painter) = painter.cached_thumb_painter.take() {
            cached_thumb_painter.dispose();
        }
        painter.cached_thumb_color = None;
        let color_animation = painter.color_animation.take();
        if let Some(color_animation) = color_animation {
            color_animation.dispose(app);
            app.destroy(color_animation);
        }
        ToggleablePainter::dispose(self, app);
    }
}

/// Dart's `try { _isPainting = true; … } finally { _isPainting = false; }`.
struct PaintingGuard {
    flag: Rc<Cell<bool>>,
}

impl PaintingGuard {
    fn enter(flag: &Rc<Cell<bool>>) -> PaintingGuard {
        flag.set(true);
        PaintingGuard {
            flag: Rc::clone(flag),
        }
    }
}

impl Drop for PaintingGuard {
    fn drop(&mut self) {
        self.flag.set(false);
    }
}

impl ChangeNotifier for SwitchPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ToggleablePainter for SwitchPainter {
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
        let current_value = self.position(app).value(app);

        let visual_position = match self.text_direction(app) {
            TextDirection::Rtl => 1.0 - current_value,
            TextDirection::Ltr => current_value,
        };
        let reaction = self.reaction(app);
        let stop_press_animation = app.get(self).stop_press_animation;
        app.get_mut(self).stop_press_animation =
            reaction.status(app) == AnimationStatus::Reverse && !stop_press_animation;

        let pressed_thumb_extension = reaction.value(app) * K_THUMB_EXTENSION_FACTOR;
        app.get_mut(self).pressed_thumb_extension = Some(pressed_thumb_extension);
        let thumb_size = Size::new(
            K_THUMB_RADIUS * 2.0 + pressed_thumb_extension,
            K_THUMB_RADIUS * 2.0,
        );

        let color_value = app
            .get(self)
            .color_animation
            .expect("set with the position controller")
            .value(app);
        let track_color = Color::lerp(
            Some(self.inactive_track_color(app)),
            Some(self.active_track_color(app)),
            self.position(app).value(app),
        )
        .expect("lerping non-null colors");
        let track_outline_color = match (
            self.inactive_track_outline_color(app),
            self.active_track_outline_color(app),
        ) {
            (Some(inactive), Some(active)) => {
                Color::lerp(Some(inactive), Some(active), color_value)
            }
            _ => None,
        };
        let track_outline_width = lerp_double(
            self.inactive_track_outline_width(app),
            self.active_track_outline_width(app),
            color_value,
        );

        let position_status = self.position_controller(app).status(app);
        let lerped_thumb_color = if !reaction.is_dismissed(app) {
            Color::lerp(
                Some(self.inactive_pressed_color(app)),
                Some(self.active_pressed_color(app)),
                color_value,
            )
        } else if position_status == AnimationStatus::Forward {
            Color::lerp(
                Some(self.inactive_pressed_color(app)),
                Some(self.active_color(app)),
                color_value,
            )
        } else if position_status == AnimationStatus::Reverse {
            Color::lerp(
                Some(self.inactive_color(app)),
                Some(self.active_pressed_color(app)),
                color_value,
            )
        } else {
            Color::lerp(
                Some(self.inactive_color(app)),
                Some(self.active_color(app)),
                color_value,
            )
        }
        .expect("lerping non-null colors");

        // Blend the thumb color against a `surfaceColor` background in case the
        // thumbColor is not opaque. This way we do not see through the thumb to the
        // track underneath.
        let thumb_color = Color::alpha_blend(lerped_thumb_color, self.surface_color(app));

        let thumb_icon = if current_value < 0.5 {
            self.inactive_icon(app)
        } else {
            self.active_icon(app)
        };

        let paint = Paint::from_color(track_color.into());

        let track_paint_offset = Self::compute_track_paint_offset(size);
        let thumb_paint_offset =
            self.compute_thumb_paint_offset(app, track_paint_offset, thumb_size, visual_position);

        let track_rect = Rect::from_ltwh(
            track_paint_offset.dx(),
            track_paint_offset.dy(),
            K_TRACK_WIDTH,
            K_TRACK_HEIGHT,
        );

        self.paint_track_with(
            app,
            canvas,
            &paint,
            track_paint_offset,
            track_outline_color,
            track_outline_width,
            track_rect,
        );

        self.paint_on_off_labels(app, canvas, visual_position, track_rect);
        self.paint_thumb_with(
            app,
            canvas,
            thumb_paint_offset,
            color_value,
            thumb_color,
            thumb_icon,
            thumb_size,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use waft_embedder::valo::Op;
    use waft_embedder::{
        HapticFeedbackType, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind,
    };
    use waft_foundation::AppCell;
    use waft_gestures::GestureBinding;
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject, RenderOpacity,
        RendererBinding,
    };
    use waft_test::TestPlatform;
    use waft_widgets::{
        Actions, ActivateIntent, Center, FocusHighlightStrategy, FocusManager, FocusNode,
        FocusNodeLeaf, GlobalKey, MediaQueryData, SizedBox, UnfocusDisposition,
    };

    use super::*;
    use crate::icons::{CupertinoIcons, install_cupertino_icon_font};
    use crate::test_support::{build, pump, test_view};

    /// The haptics a test platform played, in order.
    #[derive(Default)]
    struct RecordedHaptics(RefCell<Vec<HapticFeedbackType>>);

    impl waft_embedder::Haptics for RecordedHaptics {
        fn feedback(&self, kind: HapticFeedbackType) {
            self.0.borrow_mut().push(kind);
        }
    }

    /// How the host answers `on_changed`: Dart's `value = newValue`, or a veto.
    type Accept = fn(old: bool, new: bool) -> bool;

    /// The `StatefulBuilder` every Dart test wraps its switch in: it owns the value.
    struct Host {
        key: Option<KeyRef>,
        initial: bool,
        text_direction: TextDirection,
        media: Option<MediaQueryData>,
        theme: Option<crate::theme::CupertinoThemeData>,
        focus_node: Option<AnyFocusNode>,
        accept: Accept,
        configure: Rc<dyn Fn(CupertinoSwitch) -> CupertinoSwitch>,
        changes: Rc<RefCell<Vec<bool>>>,
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
                switch_key: GlobalKey::new(),
            }
        }
    }

    struct HostState {
        state: StateData<Host>,
        value: bool,
        enabled: bool,
        switch_key: GlobalKey,
    }

    impl State for HostState {
        type Widget = Host;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let host = self.widget(app);
            let (accept, changes) = (host.accept, Rc::clone(&host.changes));
            let on_changed: Option<ValueChanged<bool>> = app.get(self).enabled.then(|| {
                Rc::new(move |app: &mut App, new_value: bool| {
                    changes.borrow_mut().push(new_value);
                    self.set_state(app, |state| state.value = accept(state.value, new_value));
                }) as ValueChanged<bool>
            });
            let mut switch = CupertinoSwitch::new(app.get(self).value, on_changed)
                .key(Rc::new(app.get(self).switch_key.clone()));
            if let Some(focus_node) = self.widget(app).focus_node {
                switch = switch.focus_node(focus_node);
            }
            let switch = (self.widget(app).configure)(switch);
            let centered = Center::new().child(switch).into_widget();
            let themed = match self.widget(app).theme.clone() {
                Some(theme) => CupertinoTheme::new(theme, centered).into_widget(),
                None => centered,
            };
            let tree = Directionality::new(self.widget(app).text_direction, themed);
            match self.widget(app).media.clone() {
                Some(data) => MediaQuery::new(data, tree).into_widget(),
                None => tree.into_widget(),
            }
        }
    }

    struct Fixture {
        cell: Rc<AppCell>,
        host: Handle<HostState>,
        focus_node: Option<AnyFocusNode>,
        /// Where the last pointer event was, for the next one's delta.
        last_position: Offset,
        haptics: Rc<RecordedHaptics>,
        changes: Rc<RefCell<Vec<bool>>>,
        clock: Duration,
    }

    struct Setup {
        initial: bool,
        text_direction: TextDirection,
        media: Option<MediaQueryData>,
        theme: Option<crate::theme::CupertinoThemeData>,
        focusable: bool,
        accept: Accept,
        configure: Rc<dyn Fn(CupertinoSwitch) -> CupertinoSwitch>,
    }

    impl Setup {
        fn new() -> Setup {
            Setup {
                initial: false,
                text_direction: TextDirection::Ltr,
                media: None,
                theme: None,
                focusable: false,
                accept: |_old, new| new,
                configure: Rc::new(|switch| switch),
            }
        }

        fn initial(mut self, initial: bool) -> Setup {
            self.initial = initial;
            self
        }

        fn rtl(mut self) -> Setup {
            self.text_direction = TextDirection::Rtl;
            self
        }

        fn media(mut self, media: MediaQueryData) -> Setup {
            self.media = Some(media);
            self
        }

        fn theme(mut self, theme: crate::theme::CupertinoThemeData) -> Setup {
            self.theme = Some(theme);
            self
        }

        /// Hands the switch a focus node the fixture keeps.
        fn focus_node(mut self) -> Setup {
            self.focusable = true;
            self
        }

        fn accept(mut self, accept: Accept) -> Setup {
            self.accept = accept;
            self
        }

        fn configure(
            mut self,
            configure: impl Fn(CupertinoSwitch) -> CupertinoSwitch + 'static,
        ) -> Setup {
            self.configure = Rc::new(configure);
            self
        }

        /// Mounts the host on an iOS test platform that records its haptics.
        fn mount(self) -> Fixture {
            let haptics = Rc::new(RecordedHaptics::default());
            let platform = TestPlatform::new()
                .on(TargetPlatform::IOS)
                .with_view(test_view())
                .with_haptics(Rc::clone(&haptics) as Rc<dyn waft_embedder::Haptics>);
            let cell = AppCell::with_platform(Rc::new(platform));
            install_cupertino_icon_font(&mut cell.borrow_mut());
            let focus_node = self
                .focusable
                .then(|| FocusNode::new(&mut cell.borrow_mut()).as_node());
            let changes: Rc<RefCell<Vec<bool>>> = Rc::default();
            let host_key = GlobalKey::new();
            build(
                &cell,
                Host {
                    key: Some(Rc::new(host_key.clone())),
                    initial: self.initial,
                    text_direction: self.text_direction,
                    media: self.media,
                    theme: self.theme,
                    focus_node,
                    accept: self.accept,
                    configure: self.configure,
                    changes: Rc::clone(&changes),
                }
                .into_widget(),
            );
            let host = host_key
                .current_state::<HostState>(&mut cell.borrow_mut())
                .expect("the host mounted");
            Fixture {
                cell,
                host,
                focus_node,
                last_position: Offset::ZERO,
                haptics,
                changes,
                clock: Duration::ZERO,
            }
        }
    }

    impl Fixture {
        fn value(&self) -> bool {
            self.cell.borrow().get(self.host).value
        }

        fn state(&self) -> Handle<CupertinoSwitchState> {
            let key = self.cell.borrow().get(self.host).switch_key.clone();
            key.current_state::<CupertinoSwitchState>(&mut self.cell.borrow_mut())
                .expect("the switch mounted")
        }

        /// Dart's `position.value` on the switch's state.
        fn position(&self) -> f64 {
            let state = self.state();
            let app = self.cell.borrow();
            state.position(&app).value(&app)
        }

        fn haptics(&self) -> Vec<HapticFeedbackType> {
            self.haptics.0.borrow().clone()
        }

        /// Rebuilds the host with `change` applied, as a Dart test's second `pumpWidget`.
        fn set(&mut self, change: impl FnOnce(&mut HostState)) {
            self.host.set_state(&mut self.cell.borrow_mut(), change);
            self.pump(16);
        }

        fn tick(&mut self, millis: u64) -> Duration {
            self.clock += Duration::from_millis(millis);
            self.clock
        }

        fn pump(&mut self, millis: u64) {
            let at = self.tick(millis);
            pump(&mut self.cell.borrow_mut(), at);
        }

        /// Dart's `pumpAndSettle`: frames until no animation is left.
        fn settle(&mut self) {
            for _ in 0..120 {
                self.pump(16);
            }
        }

        /// The switch's top-left corner in the view: the 400x300 logical view centres the
        /// 59x39 switch.
        fn origin() -> Offset {
            Offset::new((400.0 - 59.0) / 2.0, (300.0 - 39.0) / 2.0)
        }

        fn center() -> Offset {
            Self::origin() + Offset::new(59.0 / 2.0, 39.0 / 2.0)
        }

        /// The test view is 2x: logical coordinates double into the packet.
        fn node(&self) -> AnyFocusNode {
            self.focus_node.expect("mounted with Setup::focus_node")
        }

        fn send(&mut self, change: PointerChange, kind: PointerDeviceKind, at: Offset) {
            let time_stamp = self.tick(10);
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
            self.send(change, PointerDeviceKind::Touch, at);
        }

        /// Dart's `tester.tap` at the switch's centre.
        fn tap(&mut self) {
            self.touch(PointerChange::Down, Self::center());
            self.touch(PointerChange::Up, Self::center());
            self.pump(16);
        }

        /// Dart's `tester.drag`: down at the centre, a move of the default drag slop, then
        /// the rest, then up.
        fn drag(&mut self, dx: f64) {
            const DRAG_SLOP: f64 = 20.0;
            let start = Self::center();
            self.touch(PointerChange::Down, start);
            self.touch(
                PointerChange::Move,
                start + Offset::new(DRAG_SLOP.copysign(dx), 0.0),
            );
            self.touch(PointerChange::Move, start + Offset::new(dx, 0.0));
            self.touch(PointerChange::Up, start + Offset::new(dx, 0.0));
            self.pump(16);
        }

        /// A mouse that arrives over `at`, as a desktop hover does.
        fn hover(&mut self, at: Offset) {
            self.send(PointerChange::Add, PointerDeviceKind::Mouse, at);
            self.send(PointerChange::Hover, PointerDeviceKind::Mouse, at);
            self.pump(16);
        }

        fn first<T: RenderObject>(&self) -> RenderHandle<T> {
            let key = self.cell.borrow().get(self.host).switch_key.clone();
            let mut app = self.cell.borrow_mut();
            let root = key
                .current_context(&mut app)
                .expect("the switch mounted")
                .find_render_object(&app)
                .expect("the switch has been laid out");
            find::<T>(&app, root).expect("a render object of that type under the switch")
        }

        fn opacity(&self) -> f64 {
            self.first::<RenderOpacity>().opacity(&self.cell.borrow())
        }

        /// What the switch's painter draws, in paint order.
        fn drawn(&self) -> Vec<Drawn> {
            let custom = self.first::<RenderCustomPaint>();
            let mut app = self.cell.borrow_mut();
            let painter = custom.painter(&app).expect("the switch's painter");
            let size = custom.size(&app);
            let mut canvas = Canvas::new();
            painter.paint(&mut app, &mut canvas, size);
            canvas.build().ops().iter().filter_map(Drawn::of).collect()
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
    #[derive(Clone, Debug, PartialEq)]
    enum Drawn {
        /// A filled shape: its colour as `0xAARRGGBB` and its bounds.
        Fill(u32, [f32; 4]),
        /// A stroked shape: its colour and its stroke width.
        Stroke(u32, f32),
        /// A blurred shadow.
        Shadow(u32),
        Clip,
        /// A laid-out paragraph: the left and top of its ink.
        Glyphs(f32, f32),
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
            let shape = |paint: &waft_embedder::Paint, bounds: waft_embedder::valo::Rect| {
                match &paint.style {
                    PaintStyle::Stroke(stroke) => Drawn::Stroke(argb(paint.color), stroke.width),
                    PaintStyle::Fill => Drawn::Fill(
                        argb(paint.color),
                        [bounds.x, bounds.y, bounds.width, bounds.height],
                    ),
                }
            };
            match op {
                Op::DrawRect { paint, rect, .. } => Some(shape(paint, *rect)),
                Op::DrawPath {
                    paint,
                    content_bounds,
                    ..
                } => Some(shape(paint, *content_bounds)),
                Op::RRectBlur { paint, .. } => Some(Drawn::Shadow(argb(paint.color))),
                Op::ClipPath { .. } => Some(Drawn::Clip),
                Op::GlyphRun { content_bounds, .. } => {
                    Some(Drawn::Glyphs(content_bounds.x, content_bounds.y))
                }
                _ => None,
            }
        }

        fn color(&self) -> Option<u32> {
            match self {
                Drawn::Fill(color, _) | Drawn::Stroke(color, _) | Drawn::Shadow(color) => {
                    Some(*color)
                }
                Drawn::Clip | Drawn::Glyphs(..) => None,
            }
        }
    }

    fn colors(drawn: &[Drawn]) -> Vec<Option<u32>> {
        drawn.iter().map(Drawn::color).collect()
    }

    const SYSTEM_GREEN: u32 = 0xff34c759;
    const SECONDARY_SYSTEM_FILL: u32 = 0x28787880;
    const WHITE: u32 = 0xffffffff;
    /// The track, the two shadows, the border and the thumb.
    const UNFOCUSED_THUMB: [Option<u32>; 4] = [
        Some(0x26000000),
        Some(0x0f000000),
        Some(0x0a000000),
        Some(WHITE),
    ];

    #[test]
    fn a_tap_toggles_the_switch() {
        let mut fixture = Setup::new()
            .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down))
            .mount();
        assert!(!fixture.value());
        fixture.tap();
        assert!(fixture.value());
    }

    #[test]
    fn an_activate_intent_toggles_the_switch_both_ways() {
        let mut fixture = Setup::new().initial(true).focus_node().mount();
        let node = fixture.node();
        node.request_focus(&mut fixture.cell.borrow_mut(), None);
        fixture.settle();
        // A space key reaches the switch as an `ActivateIntent` through `WidgetsApp`'s
        // shortcuts; the test invokes the intent where the shortcut would.
        for expected in [false, true] {
            let context = node
                .context(&fixture.cell.borrow())
                .expect("the node is attached");
            Actions::invoke(&mut fixture.cell.borrow_mut(), context, &ActivateIntent);
            fixture.settle();
            assert_eq!(fixture.value(), expected);
        }
    }

    #[test]
    fn a_tap_plays_one_light_impact_and_a_rebuild_plays_none() {
        let mut fixture = Setup::new().mount();
        fixture.tap();
        assert_eq!(fixture.haptics(), [HapticFeedbackType::LightImpact]);

        fixture.set(|host| host.value = !host.value);
        fixture.settle();
        assert_eq!(
            fixture.haptics(),
            [HapticFeedbackType::LightImpact],
            "a programmatic value change is silent"
        );
    }

    #[test]
    fn a_drag_past_the_threshold_plays_a_light_impact() {
        let mut fixture = Setup::new()
            .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down))
            .mount();
        fixture.drag(56.0);
        assert!(fixture.value());
        assert_eq!(fixture.haptics(), [HapticFeedbackType::LightImpact]);
    }

    #[test]
    fn a_drag_turns_the_switch_toward_its_direction_only() {
        for behavior in [DragStartBehavior::Start, DragStartBehavior::Down] {
            let mut fixture = Setup::new()
                .configure(move |switch| switch.drag_start_behavior(behavior))
                .mount();
            fixture.drag(-56.0);
            assert!(
                !fixture.value(),
                "{behavior:?}: dragging left of off stays off"
            );
            fixture.drag(56.0);
            assert!(fixture.value(), "{behavior:?}");
            fixture.drag(56.0);
            assert!(
                fixture.value(),
                "{behavior:?}: dragging right of on stays on"
            );
            fixture.drag(-56.0);
            assert!(!fixture.value(), "{behavior:?}");
        }
    }

    #[test]
    fn a_right_to_left_switch_turns_on_toward_the_left() {
        let mut fixture = Setup::new()
            .rtl()
            .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down))
            .mount();
        fixture.drag(56.0);
        assert!(!fixture.value());
        fixture.drag(-56.0);
        assert!(fixture.value());
        fixture.drag(-56.0);
        assert!(fixture.value());
        fixture.drag(56.0);
        assert!(!fixture.value());
    }

    #[test]
    fn a_vetoed_drag_animates_the_thumb_back() {
        let mut fixture = Setup::new()
            .accept(|old, new| old || new)
            .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down))
            .mount();

        // Move a little to the right, not past the middle.
        let center = Fixture::center();
        fixture.touch(PointerChange::Down, center);
        fixture.touch(PointerChange::Move, center + Offset::new(21.0, 0.0));
        fixture.pump(16);
        fixture.touch(PointerChange::Up, center + Offset::new(21.0, 0.0));
        fixture.pump(16);
        assert!(!fixture.value());
        assert_eq!(fixture.position(), 0.0);
        fixture.settle();
        assert_eq!(fixture.position(), 0.0);

        // Move past the middle.
        fixture.touch(PointerChange::Down, center);
        fixture.touch(PointerChange::Move, center + Offset::new(36.0, 0.0));
        fixture.pump(16);
        fixture.touch(PointerChange::Up, center + Offset::new(36.0, 0.0));
        fixture.pump(16);
        assert!(fixture.value());
        fixture.settle();
        assert_eq!(fixture.position(), 1.0);

        // Now move back to the left, the revert animation should play.
        fixture.touch(PointerChange::Down, center);
        fixture.touch(PointerChange::Move, center + Offset::new(-36.0, 0.0));
        fixture.pump(16);
        fixture.touch(PointerChange::Up, center + Offset::new(-36.0, 0.0));
        fixture.pump(16);
        assert!(fixture.value(), "the host vetoed turning off");
        fixture.settle();
        assert_eq!(fixture.position(), 1.0);
        assert_eq!(*fixture.changes.borrow(), [true, false]);
    }

    #[test]
    fn the_thumb_snaps_to_a_side_while_the_finger_is_still_down() {
        let mut fixture = Setup::new()
            .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down))
            .mount();
        let center = Fixture::center();
        // The distance required for a gesture to be considered a drag.
        let activation = waft_gestures::K_TOUCH_SLOP + 0.1;
        let drag_by = |fixture: &mut Fixture, dx: f64| {
            fixture.touch(PointerChange::Down, center);
            fixture.touch(PointerChange::Move, center + Offset::new(activation, 0.0));
            fixture.touch(PointerChange::Move, center);
            fixture.touch(PointerChange::Move, center + Offset::new(dx, 0.0));
            fixture.settle();
            let held = (fixture.position(), fixture.value());
            fixture.touch(PointerChange::Up, center + Offset::new(dx, 0.0));
            fixture.settle();
            (held, (fixture.position(), fixture.value()))
        };

        assert_eq!(drag_by(&mut fixture, 35.0), ((0.0, false), (0.0, false)));
        assert_eq!(drag_by(&mut fixture, 36.0), ((1.0, false), (1.0, true)));
        assert_eq!(drag_by(&mut fixture, -35.0), ((1.0, true), (1.0, true)));
        assert_eq!(drag_by(&mut fixture, -36.0), ((0.0, true), (0.0, false)));
    }

    #[test]
    fn a_disabled_switch_is_translucent_until_it_is_enabled() {
        let mut fixture = Setup::new().mount();
        assert_eq!(fixture.opacity(), 1.0);
        fixture.set(|host| host.enabled = false);
        assert_eq!(fixture.opacity(), 0.5);
        fixture.set(|host| host.enabled = true);
        assert_eq!(fixture.opacity(), 1.0);
    }

    #[test]
    fn the_track_and_thumb_take_their_default_and_given_colours() {
        let fixture = Setup::new().mount();
        let drawn = fixture.drawn();
        assert_eq!(
            drawn[0],
            Drawn::Fill(SECONDARY_SYSTEM_FILL, [4.0, 4.0, 51.0, 31.0]),
            "the off track, centred in the 59x39 switch"
        );
        assert_eq!(colors(&drawn[1..2]), [None], "the track clips the thumb");
        assert_eq!(colors(&drawn[2..]), UNFOCUSED_THUMB);

        let fixture = Setup::new()
            .configure(|switch| {
                switch
                    .inactive_track_color(Color::new(0xFF00FF00))
                    .thumb_color(Color::new(0xFF000000))
            })
            .mount();
        let drawn = fixture.drawn();
        assert_eq!(drawn[0].color(), Some(0xFF00FF00));
        assert_eq!(drawn.last().and_then(Drawn::color), Some(0xFF000000));

        let fixture = Setup::new()
            .initial(true)
            .configure(|switch| switch.active_track_color(Color::new(0xFF00FF00)))
            .mount();
        assert_eq!(fixture.drawn()[0].color(), Some(0xFF00FF00));

        let fixture = Setup::new().initial(true).mount();
        assert_eq!(fixture.drawn()[0].color(), Some(SYSTEM_GREEN));
    }

    #[test]
    fn the_thumb_takes_the_inactive_colour_off_and_the_active_one_on() {
        const ACTIVE: Color = Color::new(0xFF00000A);
        const INACTIVE: Color = Color::new(0xFF00000B);
        let mut fixture = Setup::new()
            .rtl()
            .configure(|switch| {
                switch
                    .drag_start_behavior(DragStartBehavior::Down)
                    .thumb_color(ACTIVE)
                    .inactive_thumb_color(INACTIVE)
            })
            .mount();
        assert_eq!(
            fixture.drawn().last().and_then(Drawn::color),
            Some(0xFF00000B)
        );
        fixture.drag(-56.0);
        fixture.settle();
        assert_eq!(
            fixture.drawn().last().and_then(Drawn::color),
            Some(0xFF00000A)
        );
    }

    /// The on label (`|`) and the off label (`O`) as Dart's `onLabelPaintPattern` and
    /// `offLabelPaintPattern` find them. A label at zero alpha is not recorded at all.
    fn labels(drawn: &[Drawn]) -> (Option<Drawn>, Option<Drawn>) {
        let on = drawn
            .iter()
            .find(|drawn| matches!(drawn, Drawn::Fill(_, [_, _, width, _]) if *width == 1.0))
            .cloned();
        let off = drawn
            .iter()
            .find(|drawn| matches!(drawn, Drawn::Stroke(_, width) if *width == 1.0))
            .cloned();
        (on, off)
    }

    #[test]
    fn the_on_off_labels_cross_fade_with_the_thumb() {
        for (high_contrast, rtl) in [(false, false), (true, false), (false, true)] {
            let media = MediaQueryData::new()
                .on_off_switch_labels(true)
                .high_contrast(high_contrast);
            let setup = Setup::new()
                .media(media)
                .configure(|switch| switch.drag_start_behavior(DragStartBehavior::Down));
            let mut fixture = if rtl { setup.rtl() } else { setup }.mount();
            let off_color = if high_contrast { 0xffffff } else { 0xb3b3b3 };
            let on_x = if rtl { 43.5 } else { 14.5 };
            let case = format!("high contrast {high_contrast}, rtl {rtl}");

            let (on, off) = labels(&fixture.drawn());
            assert_eq!(on, None, "{case}: the on label starts transparent");
            assert_eq!(
                off,
                Some(Drawn::Stroke(0xff000000 | off_color, 1.0)),
                "{case}"
            );

            fixture.tap();
            assert!(fixture.value());
            fixture.pump(60);
            let (on, off) = labels(&fixture.drawn());
            assert_eq!(
                on,
                Some(Drawn::Fill((131 << 24) | 0xffffff, [on_x, 14.5, 1.0, 10.0])),
                "{case}"
            );
            assert_eq!(
                off,
                Some(Drawn::Stroke((124 << 24) | off_color, 1.0)),
                "{case}"
            );

            fixture.settle();
            let (on, off) = labels(&fixture.drawn());
            assert_eq!(
                on,
                Some(Drawn::Fill(0xffffffff, [on_x, 14.5, 1.0, 10.0])),
                "{case}"
            );
            assert_eq!(off, None, "{case}: the off label ends transparent");
        }
    }

    #[test]
    fn the_track_follows_the_theme_only_where_the_switch_applies_it() {
        const AMBER: u32 = 0xFFFFC107;
        let theme = crate::theme::CupertinoThemeData::new()
            .with_primary_color(Color::new(AMBER))
            .with_apply_theme_to_all(true);
        for (apply_theme, expected) in [
            (None, AMBER),
            (Some(true), AMBER),
            (Some(false), SYSTEM_GREEN),
        ] {
            let fixture = Setup::new()
                .initial(true)
                .theme(theme.clone())
                .configure(move |switch| match apply_theme {
                    Some(apply_theme) => switch.apply_theme(apply_theme),
                    None => switch,
                })
                .mount();
            assert_eq!(
                fixture.drawn()[0].color(),
                Some(expected),
                "{apply_theme:?}"
            );
        }
    }

    #[test]
    fn a_dark_platform_resolves_the_dark_track_colours() {
        let dark = MediaQueryData::new().platform_brightness(waft_embedder::Brightness::Dark);
        let fixture = Setup::new().media(dark.clone()).mount();
        assert_eq!(
            fixture.drawn()[0].color(),
            Some(0x51787880),
            "dark secondarySystemFill"
        );
        let fixture = Setup::new().initial(true).media(dark).mount();
        assert_eq!(
            fixture.drawn()[0].color(),
            Some(0xff30d158),
            "dark systemGreen"
        );
    }

    #[test]
    fn a_focused_switch_draws_its_ring_until_it_is_disabled() {
        const FOCUS: Color = Color::new(0xffff0000);
        let mut fixture = Setup::new()
            .initial(true)
            .focus_node()
            .configure(|switch| switch.focus_color(FOCUS).autofocus(true))
            .mount();
        let node = fixture.node();
        {
            let mut app = fixture.cell.borrow_mut();
            let manager = FocusManager::instance(&mut app);
            manager.set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        }
        fixture.settle();
        assert!(node.has_primary_focus(&fixture.cell.borrow()));
        let ring = [Some(SYSTEM_GREEN), Some(0xffff0000), None];
        assert_eq!(
            colors(&fixture.drawn()),
            [&ring[..], &UNFOCUSED_THUMB[..]].concat()
        );

        fixture.set(|host| host.value = false);
        fixture.settle();
        let ring = [Some(SECONDARY_SYSTEM_FILL), Some(0xffff0000), None];
        assert_eq!(
            colors(&fixture.drawn()),
            [&ring[..], &UNFOCUSED_THUMB[..]].concat()
        );

        fixture.set(|host| host.enabled = false);
        fixture.settle();
        assert!(!node.has_primary_focus(&fixture.cell.borrow()));
        let track = [Some(SECONDARY_SYSTEM_FILL), None];
        assert_eq!(
            colors(&fixture.drawn()),
            [&track[..], &UNFOCUSED_THUMB[..]].concat()
        );
    }

    #[test]
    fn on_focus_change_hears_focus_arrive_and_leave() {
        let focused = Rc::new(Cell::new(false));
        let mut fixture = Setup::new()
            .initial(true)
            .focus_node()
            .configure({
                let focused = Rc::clone(&focused);
                move |switch| {
                    let focused = Rc::clone(&focused);
                    switch.on_focus_change(move |_app, value| focused.set(value))
                }
            })
            .mount();
        let node = fixture.node();

        node.request_focus(&mut fixture.cell.borrow_mut(), None);
        fixture.pump(16);
        assert!(focused.get());
        assert!(node.has_focus(&mut fixture.cell.borrow_mut()));

        node.unfocus(&mut fixture.cell.borrow_mut(), UnfocusDisposition::Scope);
        fixture.pump(16);
        assert!(!focused.get());
        assert!(!node.has_focus(&mut fixture.cell.borrow_mut()));
    }

    /// The fill under the outline and the outline itself.
    fn outline(drawn: &[Drawn]) -> (Drawn, Drawn) {
        (drawn[0].clone(), drawn[1].clone())
    }

    #[test]
    fn the_track_outline_colour_and_width_resolve_in_selected_and_disabled_states() {
        let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
            let disabled = states.contains(&WidgetState::Disabled);
            let selected = states.contains(&WidgetState::Selected);
            Some(Color::new(match (disabled, selected) {
                (true, true) => 0xFF000002,
                (true, false) => 0xFF000004,
                (false, true) => 0xFF000001,
                (false, false) => 0xFF000003,
            }))
        });
        let width = <dyn WidgetStateProperty<Option<f64>>>::resolve_with(|states| {
            let disabled = states.contains(&WidgetState::Disabled);
            let selected = states.contains(&WidgetState::Selected);
            Some(match (disabled, selected) {
                (true, true) => 2.0,
                (true, false) => 4.0,
                (false, true) => 1.0,
                (false, false) => 3.0,
            })
        });
        for (enabled, active, expected_color, expected_width) in [
            (false, false, 0xFF000004, 4.0),
            (false, true, 0xFF000002, 2.0),
            (true, false, 0xFF000003, 3.0),
            (true, true, 0xFF000001, 1.0),
        ] {
            let (color, width) = (Rc::clone(&color), Rc::clone(&width));
            let mut fixture = Setup::new()
                .initial(active)
                .configure(move |switch| {
                    switch
                        .track_outline_color(Rc::clone(&color))
                        .track_outline_width(Rc::clone(&width))
                })
                .mount();
            fixture.set(|host| host.enabled = enabled);
            fixture.settle();
            let (track, outline) = outline(&fixture.drawn());
            assert!(matches!(track, Drawn::Fill(..)), "{track:?}");
            assert_eq!(
                outline,
                Drawn::Stroke(expected_color, expected_width),
                "enabled {enabled}, active {active}"
            );
        }
    }

    #[test]
    fn the_track_outline_resolves_in_focused_and_hovered_states() {
        let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
            Some(Color::new(if states.contains(&WidgetState::Hovered) {
                0xFF000001
            } else if states.contains(&WidgetState::Focused) {
                0xFF000002
            } else {
                0x00000000
            }))
        });
        let width = <dyn WidgetStateProperty<Option<f64>>>::resolve_with(|states| {
            Some(if states.contains(&WidgetState::Hovered) {
                4.0
            } else if states.contains(&WidgetState::Focused) {
                6.0
            } else {
                8.0
            })
        });
        let mut fixture = Setup::new()
            .initial(true)
            .rtl()
            .focus_node()
            .configure(move |switch| {
                switch
                    .autofocus(true)
                    .track_outline_color(Rc::clone(&color))
                    .track_outline_width(Rc::clone(&width))
            })
            .mount();
        let node = fixture.node();
        {
            let mut app = fixture.cell.borrow_mut();
            let manager = FocusManager::instance(&mut app);
            manager.set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        }
        fixture.settle();
        assert!(node.has_primary_focus(&fixture.cell.borrow()));
        let drawn = fixture.drawn();
        assert_eq!(drawn[1], Drawn::Stroke(0xFF000002, 6.0));
        assert!(
            matches!(drawn[2], Drawn::Stroke(_, 3.5)),
            "the focus ring follows the outline: {:?}",
            drawn[2]
        );

        fixture.hover(Fixture::center());
        fixture.settle();
        assert_eq!(fixture.drawn()[1], Drawn::Stroke(0xFF000001, 4.0));
    }

    #[test]
    fn a_mouse_over_the_switch_shows_the_cursor_it_resolves() {
        let mut fixture = Setup::new()
            .configure(|switch| {
                switch.mouse_cursor(<dyn WidgetStateProperty<MouseCursorRef>>::all(
                    SystemMouseCursors::FORBIDDEN.into(),
                ))
            })
            .mount();
        let cursor = |fixture: &Fixture| {
            let mut app = fixture.cell.borrow_mut();
            RendererBinding::instance(&mut app)
                .mouse_tracker(&app)
                .debug_device_active_cursor(&app, 1)
        };
        fixture.hover(Fixture::center() + Offset::new(51.0, 31.0));
        assert_eq!(cursor(&fixture), Some(SystemMouseCursors::BASIC.into()));
        fixture.send(
            PointerChange::Hover,
            PointerDeviceKind::Mouse,
            Fixture::center(),
        );
        fixture.pump(16);
        assert_eq!(cursor(&fixture), Some(SystemMouseCursors::FORBIDDEN.into()));
    }

    #[test]
    fn the_thumb_shows_the_icon_for_its_side_only() {
        let icon = |active: bool| {
            move |switch: CupertinoSwitch| {
                let glyph = Rc::new(Icon::new(Some(CupertinoIcons::clear())));
                switch.thumb_icon(<dyn WidgetStateProperty<Option<Rc<Icon>>>>::resolve_with(
                    move |states| {
                        (states.contains(&WidgetState::Selected) == active)
                            .then(|| Rc::clone(&glyph))
                    },
                ))
            }
        };
        let glyphs = |fixture: &Fixture| {
            fixture
                .drawn()
                .into_iter()
                .filter(|drawn| matches!(drawn, Drawn::Glyphs(..)))
                .count()
        };
        let fixture = Setup::new().initial(true).configure(icon(true)).mount();
        assert_eq!(glyphs(&fixture), 1, "the active icon shows when on");
        let fixture = Setup::new().initial(false).configure(icon(false)).mount();
        assert_eq!(glyphs(&fixture), 1, "the inactive icon shows when off");
        let fixture = Setup::new().initial(false).configure(icon(true)).mount();
        assert_eq!(
            glyphs(&fixture),
            0,
            "the active icon does not show when off"
        );
        let fixture = Setup::new().initial(true).configure(icon(false)).mount();
        assert_eq!(
            glyphs(&fixture),
            0,
            "the inactive icon does not show when on"
        );
    }

    #[test]
    fn a_switch_squeezed_to_nothing_lays_out_at_zero() {
        let cell = crate::test_support::test_cell();
        let key = GlobalKey::new();
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    SizedBox::shrink().child(
                        CupertinoSwitch::new(false, Some(Rc::new(|_app: &mut App, _| {})))
                            .key(Rc::new(key.clone())),
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
            .expect("the switch laid out");
        assert_eq!(render.size(&app), Size::ZERO);
    }

    #[test]
    fn unmounting_the_switch_disposes_its_painter_and_animations() {
        let fixture = Setup::new().mount();
        let state = fixture.state();
        let painter = fixture.cell.borrow().get(state).painter.expect("created");
        build(&fixture.cell, SizedBox::shrink().into_widget());
        let app = fixture.cell.borrow();
        assert!(!app.contains(painter));
    }
}

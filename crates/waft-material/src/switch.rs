//! Flutter counterpart: material_ui's `switch.dart`.
//!
//! [`Switch`] builds a private `_MaterialSwitch`, whose state is a [`ToggleableStateMixin`]
//! and whose painter an arena [`ToggleablePainter`] the state keeps for its lifetime, as
//! Dart's `_painter` field is. `Switch::adaptive` on iOS and macOS is the same switch with the
//! Cupertino configuration and defaults, not a `CupertinoSwitch`. The thumb images wait with
//! painting's `DecorationImage`.

use std::cell::Cell;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationController, AnimationStatus, ConstantTween, Cubic, Curve,
    CurveTween, CurvedAnimation, Curves, Tween, TweenSequence, TweenSequenceItem,
};
use waft_cupertino::{CupertinoColors, CupertinoDynamicColor};
use waft_embedder::{
    Brightness, Canvas, ClipOp, Color, Offset, Paint, PaintStyle, RRect, Radius, Rect, Size,
    Stroke, TargetPlatform, TextDirection, lerp_double, rrect_radii_elliptical,
};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, ValueChanged};
use waft_gestures::{DragEndDetails, DragStartBehavior, DragStartDetails, DragUpdateDetails};
use waft_painting::{
    AnyColor, BlurStyle, BorderSide, BoxPainter, BoxShadow, Decoration, EdgeInsetsGeometry,
    FontVariation, HSLColor, ImageConfiguration, PaintingBinding, ShapeDecoration, StadiumBorder,
    TextPainter, TextSpan, TextStyle, draw_rrect,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    AnyFocusNode, BuildContext, Directionality, GestureDetector, Icon, IconTheme, IconThemeData,
    IntoWidget, KeyRef, Opacity, Semantics, State, StateData, StatefulWidget, StatelessWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, ToggleablePainter,
    ToggleablePainterData, ToggleablePainterRef, ToggleableStateMixin, ToggleableStateMixinData,
    WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
    WidgetStates, create_local_image_configuration,
};

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::constants::{
    K_MIN_INTERACTIVE_DIMENSION, K_RADIAL_REACTION_ALPHA, K_RADIAL_REACTION_DURATION,
    K_RADIAL_REACTION_RADIUS,
};
use crate::debug::debug_check_has_material;
use crate::generated::switch_defaults_m3::{SwitchConfigM3, switch_defaults_m3};
use crate::shadows::k_elevation_to_shadow;
use crate::switch_theme::{SwitchTheme, SwitchThemeData};
use crate::theme::Theme;
use crate::theme_data::{Adaptation, MaterialTapTargetSize, ThemeData};

/// Dart's deprecated `Color.withOpacity`, deprecated here too: its rounding to an 8-bit alpha
/// is part of the behaviour being ported.
#[allow(deprecated)]
fn with_opacity(color: Color, opacity: f64) -> Color {
    color.with_opacity(opacity)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwitchType {
    Material,
    Adaptive,
}

/// A Material Design switch.
///
/// Used to toggle the on/off state of a single setting.
///
/// The switch itself does not maintain any state. Instead, when the state of the switch
/// changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most widgets that
/// use a switch will listen for the [`on_changed`](Self::on_changed) callback and rebuild the
/// switch with a new [`value`](Self::value) to update the visual appearance of the switch.
///
/// If the [`on_changed`](Self::on_changed) callback is null, then the switch will be disabled
/// (it will not respond to input). A disabled switch's thumb and track are rendered in shades
/// of grey by default. The default appearance of a disabled switch can be overridden with
/// [`inactive_thumb_color`](Self::inactive_thumb_color) and
/// [`inactive_track_color`](Self::inactive_track_color).
///
/// Requires one of its ancestors to be a `Material` widget.
///
/// Material Design 3 provides the option to add icons on the thumb of the [`Switch`]. If
/// `ThemeData::use_material3` is set to true, users can use
/// [`thumb_icon`](Self::thumb_icon) to add optional Icons based on the different
/// [`WidgetState`]s of the [`Switch`].
///
/// See also:
///
///  * `SwitchListTile`, which combines this widget with a `ListTile` so that you can give the
///    switch a label.
///  * `Checkbox`, another widget with similar semantics.
///  * `Radio`, for selecting among a set of explicit values.
///  * `Slider`, for selecting a value in a range.
///  * [`WidgetStateProperty`], an interface for objects that "resolve" to different values
///    depending on a widget's material state.
///  * <https://material.io/design/components/selection-controls.html#switches>
#[derive(Clone)]
pub struct Switch {
    pub key: Option<KeyRef>,
    /// Whether this switch is on or off.
    pub value: bool,
    /// Called when the user toggles the switch on or off.
    ///
    /// The switch passes the new value to the callback but does not actually change state
    /// until the parent widget rebuilds the switch with the new value.
    ///
    /// If null, the switch will be displayed as disabled.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
    pub on_changed: Option<ValueChanged<bool>>,
    /// The color to use when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary`.
    ///
    /// If [`thumb_color`](Self::thumb_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, it will be used instead of this color.
    #[deprecated(
        note = "Use activeThumbColor or activeTrackColor instead. This feature was deprecated after v3.31.0-2.0.pre."
    )]
    pub active_color: Option<Color>,
    /// The color to use when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary`.
    ///
    /// If [`thumb_color`](Self::thumb_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, it will be used instead of this color.
    pub active_thumb_color: Option<Color>,
    /// The color to use on the track when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary` with the opacity set at 50%.
    ///
    /// If [`track_color`](Self::track_color) returns a non-null color in the
    /// [`WidgetState::Selected`] state, it will be used instead of this color.
    pub active_track_color: Option<Color>,
    /// The color to use on the thumb when this switch is off.
    ///
    /// Defaults to the colors described in the Material design specification.
    ///
    /// If [`thumb_color`](Self::thumb_color) returns a non-null color in the default state,
    /// it will be used instead of this color.
    pub inactive_thumb_color: Option<Color>,
    /// The color to use on the track when this switch is off.
    ///
    /// Defaults to the colors described in the Material design specification.
    ///
    /// If [`track_color`](Self::track_color) returns a non-null color in the default state,
    /// it will be used instead of this color.
    pub inactive_track_color: Option<Color>,
    /// The color of this [`Switch`]'s thumb.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_thumb_color`](Self::active_thumb_color) is used in
    /// the selected state and [`inactive_thumb_color`](Self::inactive_thumb_color) in the
    /// default state. If that is also null, then the value of
    /// [`SwitchThemeData::thumb_color`] is used. If that is also null, then the following
    /// colors are used:
    ///
    /// | State    | Light theme              | Dark theme               |
    /// |----------|--------------------------|--------------------------|
    /// | Default  | `Colors.grey.shade50`    | `Colors.grey.shade400`   |
    /// | Selected | `ColorScheme::secondary` | `ColorScheme::secondary` |
    /// | Disabled | `Colors.grey.shade400`   | `Colors.grey.shade800`   |
    pub thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The color of this [`Switch`]'s track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_track_color`](Self::active_track_color) is used in
    /// the selected state and [`inactive_track_color`](Self::inactive_track_color) in the
    /// default state. If that is also null, then the value of
    /// [`SwitchThemeData::track_color`] is used. If that is also null, then the following
    /// colors are used:
    ///
    /// | State    | Light theme                          | Dark theme                           |
    /// |----------|--------------------------------------|--------------------------------------|
    /// | Default  | `Color(0x52000000)`                  | `Colors.white30`                     |
    /// | Selected | `active_thumb_color` with alpha 0x80 | `active_thumb_color` with alpha 0x80 |
    /// | Disabled | `Colors.black12`                     | `Colors.white10`                     |
    pub track_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The outline color of this [`Switch`]'s track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// In Material 3, the outline color defaults to transparent in the selected state and
    /// `ColorScheme::outline` in the unselected state. In Material 2, the [`Switch`] track has
    /// no outline by default.
    pub track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The outline width of this [`Switch`]'s track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// Defaults to 2.0.
    pub track_outline_width: Option<WidgetStatePropertyRef<Option<f64>>>,
    /// The icon to use on the thumb of this switch
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`SwitchThemeData::thumb_icon`] is used. If this is also
    /// null, then the [`Switch`] does not have any icons on the thumb.
    pub thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,
    /// Configures the minimum size of the tap target.
    ///
    /// If null, then the value of [`SwitchThemeData::material_tap_target_size`] is used. If
    /// that is also null, then the value of `ThemeData::material_tap_target_size` is used.
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    switch_type: SwitchType,
    /// Whether to apply the ambient `CupertinoThemeData`.
    ///
    /// If true, the track uses `CupertinoThemeData::primary_color` for the track when the
    /// switch is on.
    ///
    /// Defaults to `CupertinoThemeData::apply_theme_to_all`.
    pub apply_cupertino_theme: Option<bool>,
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
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`SwitchThemeData::mouse_cursor`] is used. If that is also
    /// null, then [`WidgetStateMouseCursor::clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The color for the button's `Material` when it has the input focus.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Focused`] state, it will be used instead.
    ///
    /// If null, then the value of [`SwitchThemeData::overlay_color`] is used in the focused
    /// state. If that is also null, then the value of `ThemeData::focus_color` is used.
    pub focus_color: Option<Color>,
    /// The color for the button's `Material` when a pointer is hovering over it.
    ///
    /// If [`overlay_color`](Self::overlay_color) returns a non-null color in the
    /// [`WidgetState::Hovered`] state, it will be used instead.
    ///
    /// If null, then the value of [`SwitchThemeData::overlay_color`] is used in the hovered
    /// state. If that is also null, then the value of `ThemeData::hover_color` is used.
    pub hover_color: Option<Color>,
    /// The color for the switch's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///
    /// If null, then the value of [`active_thumb_color`](Self::active_thumb_color) with alpha
    /// [`K_RADIAL_REACTION_ALPHA`], [`focus_color`](Self::focus_color) and
    /// [`hover_color`](Self::hover_color) is used in the pressed, focused and hovered state.
    /// If that is also null, the value of [`SwitchThemeData::overlay_color`] is used. If that
    /// is also null, then the value of `ColorScheme::secondary` with alpha
    /// [`K_RADIAL_REACTION_ALPHA`], `ThemeData::focus_color` and `ThemeData::hover_color` is
    /// used in the pressed, focused and hovered state.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of [`SwitchThemeData::splash_radius`] is used. If that is also
    /// null, then [`K_RADIAL_REACTION_RADIUS`] is used.
    pub splash_radius: Option<f64>,
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
    /// The amount of space to surround the child inside the bounds of the [`Switch`].
    ///
    /// Defaults to horizontal padding of 4 pixels. If `ThemeData::use_material3` is false,
    /// then there is no padding by default.
    pub padding: Option<EdgeInsetsGeometry>,
}

impl Switch {
    /// Creates a Material Design switch.
    ///
    /// The switch itself does not maintain any state. Instead, when the state of the switch
    /// changes, the widget calls the [`on_changed`](Self::on_changed) callback. Most widgets
    /// that use a switch will listen for the [`on_changed`](Self::on_changed) callback and
    /// rebuild the switch with a new [`value`](Self::value) to update the visual appearance of
    /// the switch.
    ///
    /// The following arguments are required:
    ///
    /// * `value` determines whether this switch is on or off.
    /// * `on_changed` is called when the user toggles the switch on or off.
    ///
    /// Dart's `Switch` has no `applyCupertinoTheme` parameter: its `applyCupertinoTheme` is
    /// false, so an [`apply_cupertino_theme`](Self::apply_cupertino_theme) set on it is ignored
    /// ([`effective_apply_cupertino_theme`](Self::effective_apply_cupertino_theme)).
    pub fn new(value: bool, on_changed: Option<ValueChanged<bool>>) -> Switch {
        Switch::with_type(value, on_changed, SwitchType::Material)
    }

    /// Creates an adaptive [`Switch`] based on whether the target platform is iOS or macOS,
    /// following Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// Creates a switch that looks and feels native when the `ThemeData::platform` is iOS or
    /// macOS, otherwise a Material Design switch is created.
    ///
    /// To provide a custom switch theme that's only used by this factory constructor, pass a
    /// custom `Adaptation<SwitchThemeData>` class to the `adaptations` parameter of
    /// [`ThemeData`]. This can be useful in situations where you don't want the overall
    /// `ThemeData::switch_theme` to apply when this adaptive constructor is used.
    ///
    /// The target platform is based on the current `Theme`: `ThemeData::platform`.
    pub fn adaptive(value: bool, on_changed: Option<ValueChanged<bool>>) -> Switch {
        Switch::with_type(value, on_changed, SwitchType::Adaptive)
    }

    #[allow(deprecated)]
    fn with_type(
        value: bool,
        on_changed: Option<ValueChanged<bool>>,
        switch_type: SwitchType,
    ) -> Switch {
        Switch {
            key: None,
            value,
            on_changed,
            active_color: None,
            active_thumb_color: None,
            active_track_color: None,
            inactive_thumb_color: None,
            inactive_track_color: None,
            thumb_color: None,
            track_color: None,
            track_outline_color: None,
            track_outline_width: None,
            thumb_icon: None,
            material_tap_target_size: None,
            switch_type,
            apply_cupertino_theme: None,
            drag_start_behavior: DragStartBehavior::Start,
            mouse_cursor: None,
            focus_color: None,
            hover_color: None,
            overlay_color: None,
            splash_radius: None,
            focus_node: None,
            on_focus_change: None,
            autofocus: false,
            padding: None,
        }
    }

    /// Dart `Switch(key:)`.
    pub fn key(mut self, key: KeyRef) -> Switch {
        self.key = Some(key);
        self
    }

    /// Dart `Switch(activeColor:)`.
    #[deprecated(
        note = "Use activeThumbColor instead. This feature was deprecated after v3.31.0-2.0.pre."
    )]
    #[allow(deprecated)]
    pub fn active_color(mut self, active_color: Color) -> Switch {
        self.active_color = Some(active_color);
        self
    }

    /// Dart `Switch(activeThumbColor:)`.
    pub fn active_thumb_color(mut self, active_thumb_color: Color) -> Switch {
        self.active_thumb_color = Some(active_thumb_color);
        self
    }

    /// Dart `Switch(activeTrackColor:)`.
    pub fn active_track_color(mut self, active_track_color: Color) -> Switch {
        self.active_track_color = Some(active_track_color);
        self
    }

    /// Dart `Switch(inactiveThumbColor:)`.
    pub fn inactive_thumb_color(mut self, inactive_thumb_color: Color) -> Switch {
        self.inactive_thumb_color = Some(inactive_thumb_color);
        self
    }

    /// Dart `Switch(inactiveTrackColor:)`.
    pub fn inactive_track_color(mut self, inactive_track_color: Color) -> Switch {
        self.inactive_track_color = Some(inactive_track_color);
        self
    }

    /// Dart `Switch(thumbColor:)`.
    pub fn thumb_color(mut self, thumb_color: WidgetStatePropertyRef<Option<Color>>) -> Switch {
        self.thumb_color = Some(thumb_color);
        self
    }

    /// Dart `Switch(trackColor:)`.
    pub fn track_color(mut self, track_color: WidgetStatePropertyRef<Option<Color>>) -> Switch {
        self.track_color = Some(track_color);
        self
    }

    /// Dart `Switch(trackOutlineColor:)`.
    pub fn track_outline_color(
        mut self,
        track_outline_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> Switch {
        self.track_outline_color = Some(track_outline_color);
        self
    }

    /// Dart `Switch(trackOutlineWidth:)`.
    pub fn track_outline_width(
        mut self,
        track_outline_width: WidgetStatePropertyRef<Option<f64>>,
    ) -> Switch {
        self.track_outline_width = Some(track_outline_width);
        self
    }

    /// Dart `Switch(thumbIcon:)`.
    pub fn thumb_icon(mut self, thumb_icon: WidgetStatePropertyRef<Option<Rc<Icon>>>) -> Switch {
        self.thumb_icon = Some(thumb_icon);
        self
    }

    /// Dart `Switch(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> Switch {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `Switch.adaptive(applyCupertinoTheme:)`.
    pub fn apply_cupertino_theme(mut self, apply_cupertino_theme: bool) -> Switch {
        self.apply_cupertino_theme = Some(apply_cupertino_theme);
        self
    }

    /// Dart `Switch(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> Switch {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `Switch(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> Switch {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `Switch(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> Switch {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `Switch(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> Switch {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `Switch(overlayColor:)`.
    pub fn overlay_color(mut self, overlay_color: WidgetStatePropertyRef<Option<Color>>) -> Switch {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `Switch(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> Switch {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `Switch(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> Switch {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Switch(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: impl Fn(&mut App, bool) + 'static) -> Switch {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `Switch(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Switch {
        self.autofocus = autofocus;
        self
    }

    /// Dart `Switch(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Switch {
        self.padding = Some(padding.into());
        self
    }

    /// Dart's `applyCupertinoTheme` field: what
    /// [`apply_cupertino_theme`](Self::apply_cupertino_theme) was given, and false for a switch
    /// made with [`Switch::new`], whose constructor sets it so.
    pub fn effective_apply_cupertino_theme(&self) -> Option<bool> {
        match self.switch_type {
            SwitchType::Material => Some(false),
            SwitchType::Adaptive => self.apply_cupertino_theme,
        }
    }

    /// Dart's `_getSwitchSize`.
    fn get_switch_size(&self, app: &mut App, context: BuildContext) -> Size {
        let theme = Theme::of(app, context);
        let mut switch_theme = SwitchTheme::of(app, context);
        let defaults = switch_defaults(app, context, theme.use_material3);
        if self.switch_type == SwitchType::Adaptive {
            let switch_adaptation = theme
                .get_adaptation::<SwitchThemeData>()
                .unwrap_or(&SwitchThemeAdaptation);
            switch_theme = switch_adaptation.adapt(&theme, switch_theme);
        }
        let switch_config = switch_config(app, context, theme.use_material3);

        let effective_material_tap_target_size = self
            .material_tap_target_size
            .or(switch_theme.material_tap_target_size)
            .unwrap_or(theme.material_tap_target_size);
        let effective_padding = self
            .padding
            .or(switch_theme.padding)
            .or(defaults.padding)
            .expect("the defaults give a padding");
        match effective_material_tap_target_size {
            MaterialTapTargetSize::Padded => Size::new(
                switch_config.switch_width() + effective_padding.horizontal(),
                switch_config.switch_height() + effective_padding.vertical(),
            ),
            MaterialTapTargetSize::ShrinkWrap => Size::new(
                switch_config.switch_width() + effective_padding.horizontal(),
                switch_config.switch_height_collapsed() + effective_padding.vertical(),
            ),
        }
    }
}

/// Dart's `theme.useMaterial3 ? _SwitchConfigM3(context) : _SwitchConfigM2()`.
fn switch_config(
    app: &mut App,
    context: BuildContext,
    use_material3: bool,
) -> Box<dyn SwitchConfig> {
    if use_material3 {
        Box::new(SwitchConfigM3::new(app, context))
    } else {
        Box::new(SwitchConfigM2::new())
    }
}

/// Dart's `theme.useMaterial3 ? _SwitchDefaultsM3(context) : _SwitchDefaultsM2(context)`.
fn switch_defaults(app: &mut App, context: BuildContext, use_material3: bool) -> SwitchThemeData {
    if use_material3 {
        switch_defaults_m3(app, context)
    } else {
        switch_defaults_m2(app, context)
    }
}

impl Debug for Switch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Switch")
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

impl StatelessWidget for Switch {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    #[allow(deprecated)]
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut effective_active_thumb_color = None;
        let mut effective_active_track_color = None;
        match self.switch_type {
            SwitchType::Material => {
                effective_active_thumb_color = self.active_color;
            }
            SwitchType::Adaptive => match Theme::of(app, context).platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => {
                    effective_active_thumb_color = self.active_color;
                }
                TargetPlatform::IOS | TargetPlatform::MacOS => {
                    effective_active_track_color = self.active_color;
                }
            },
        }
        MaterialSwitch {
            value: self.value,
            on_changed: self.on_changed.clone(),
            size: self.get_switch_size(app, context),
            active_thumb_color: self.active_thumb_color.or(effective_active_thumb_color),
            active_track_color: self.active_track_color.or(effective_active_track_color),
            inactive_thumb_color: self.inactive_thumb_color,
            inactive_track_color: self.inactive_track_color,
            thumb_color: self.thumb_color.clone(),
            track_color: self.track_color.clone(),
            track_outline_color: self.track_outline_color.clone(),
            track_outline_width: self.track_outline_width.clone(),
            thumb_icon: self.thumb_icon.clone(),
            material_tap_target_size: self.material_tap_target_size,
            drag_start_behavior: self.drag_start_behavior,
            mouse_cursor: self.mouse_cursor.clone(),
            focus_color: self.focus_color,
            hover_color: self.hover_color,
            overlay_color: self.overlay_color.clone(),
            splash_radius: self.splash_radius,
            focus_node: self.focus_node,
            on_focus_change: self.on_focus_change.clone(),
            autofocus: self.autofocus,
            apply_cupertino_theme: self.effective_apply_cupertino_theme(),
            switch_type: self.switch_type,
        }
        .into_widget()
    }
}

/// Dart's `_MaterialSwitch`.
#[derive(Clone)]
struct MaterialSwitch {
    value: bool,
    on_changed: Option<ValueChanged<bool>>,
    active_thumb_color: Option<Color>,
    active_track_color: Option<Color>,
    inactive_thumb_color: Option<Color>,
    inactive_track_color: Option<Color>,
    thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    track_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    track_outline_width: Option<WidgetStatePropertyRef<Option<f64>>>,
    thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,
    #[expect(
        dead_code,
        reason = "Dart's field; the size Switch works out from it is what the state reads"
    )]
    material_tap_target_size: Option<MaterialTapTargetSize>,
    drag_start_behavior: DragStartBehavior,
    mouse_cursor: Option<MouseCursorRef>,
    focus_color: Option<Color>,
    hover_color: Option<Color>,
    overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    splash_radius: Option<f64>,
    focus_node: Option<AnyFocusNode>,
    on_focus_change: Option<ValueChanged<bool>>,
    autofocus: bool,
    size: Size,
    apply_cupertino_theme: Option<bool>,
    switch_type: SwitchType,
}

impl Debug for MaterialSwitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_MaterialSwitch")
            .field("value", &self.value)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MaterialSwitch {
    type State = MaterialSwitchState;

    fn create_state(&self) -> MaterialSwitchState {
        MaterialSwitchState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            toggleable: ToggleableStateMixinData::new(),
            painter: None,
            needs_position_animation: false,
            is_cupertino: false,
        }
    }
}

/// Dart's `_MaterialSwitchState`.
struct MaterialSwitchState {
    state: StateData<MaterialSwitch>,
    ticker_provider: TickerProviderStateMixinData,
    toggleable: ToggleableStateMixinData,
    /// Dart's `final _SwitchPainter _painter = _SwitchPainter()`, created in `init_state`
    /// because a painter is an arena object and a state is created without the `App`.
    painter: Option<Handle<SwitchPainter>>,
    needs_position_animation: bool,
    is_cupertino: bool,
}

impl MaterialSwitchState {
    fn painter(self: Handle<Self>, app: &App) -> Handle<SwitchPainter> {
        app.get(self).painter.expect("created in init_state")
    }

    fn update_curve(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let position = self.position(app);
        if Theme::of(app, context).use_material3 {
            let position = app.get_mut(position);
            position.curve = Curves::ease_out_back();
            position.reverse_curve = Some(Rc::new(Curves::ease_out_back().flipped()));
        } else {
            let position = app.get_mut(position);
            position.curve = Curves::ease_in();
            position.reverse_curve = Some(Curves::ease_out());
        }
    }

    /// Dart's `_widgetThumbColor`.
    fn widget_thumb_color(widget: &MaterialSwitch) -> WidgetStatePropertyRef<Option<Color>> {
        let (active_thumb_color, inactive_thumb_color) =
            (widget.active_thumb_color, widget.inactive_thumb_color);
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return inactive_thumb_color;
            }
            if states.contains(&WidgetState::Selected) {
                return active_thumb_color;
            }
            inactive_thumb_color
        })
    }

    /// Dart's `_widgetTrackColor`.
    fn widget_track_color(widget: &MaterialSwitch) -> WidgetStatePropertyRef<Option<Color>> {
        let (active_track_color, inactive_track_color) =
            (widget.active_track_color, widget.inactive_track_color);
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Selected) {
                return active_track_color;
            }
            inactive_track_color
        })
    }

    /// Dart's `_trackInnerLength`.
    fn track_inner_length(self: Handle<Self>, app: &mut App) -> f64 {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let config = match self.widget(app).switch_type {
            SwitchType::Adaptive => match theme.platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => switch_config(app, context, theme.use_material3),
                TargetPlatform::IOS | TargetPlatform::MacOS => {
                    Box::new(SwitchConfigCupertino::new(app, context)) as Box<dyn SwitchConfig>
                }
            },
            SwitchType::Material => switch_config(app, context, theme.use_material3),
        };
        let track_inner_start = config.track_height() / 2.0;
        let track_inner_end = config.track_width() - track_inner_start;
        track_inner_end - track_inner_start
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        if self.is_interactive(app) {
            self.reaction_controller(app).forward(app, None);
        }
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        if self.is_interactive(app) {
            let position = app.get_mut(self.position(app));
            position.curve = Curves::linear();
            position.reverse_curve = None;
            let delta = details
                .primary_delta
                .expect("a horizontal drag has a primary delta")
                / self.track_inner_length(app);
            let context = self.context(app);
            let delta = match Directionality::of(app, context) {
                TextDirection::Rtl => -delta,
                TextDirection::Ltr => delta,
            };
            let position_controller = self.position_controller(app);
            let value = position_controller.value(app);
            position_controller.set_value(app, value + delta);
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, _details: DragEndDetails) {
        let widget_value = self.widget(app).value;
        if (self.position(app).value(app) >= 0.5) != widget_value {
            if let Some(on_changed) = self.widget(app).on_changed.clone() {
                on_changed(app, !widget_value);
            }
            // Wait with finishing the animation until widget.value has changed to
            // !widget.value as part of the widget.onChanged call above.
            self.set_state(app, |state| state.needs_position_animation = true);
        } else {
            self.animate_to_value(app);
        }
        self.reaction_controller(app).reverse(app, None);
    }

    fn handle_changed(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        debug_assert!(value.is_some());
        let on_changed = self.widget(app).on_changed.clone();
        debug_assert!(on_changed.is_some());
        if let Some(on_changed) = on_changed {
            on_changed(app, value.expect("a two-state toggle"));
        }
    }
}

/// `property?.resolve(states)` for a nullable property of a nullable value.
fn resolve<T>(
    property: &Option<WidgetStatePropertyRef<Option<T>>>,
    states: &WidgetStates,
) -> Option<T> {
    property
        .as_ref()
        .and_then(|property| property.resolve(states))
}

impl TickerProviderStateMixin for MaterialSwitchState {
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
impl TickerProviderObject for MaterialSwitchState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ToggleableStateMixin for MaterialSwitchState {
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

    fn reaction_animation_duration(self: Handle<Self>, _app: &App) -> Option<Duration> {
        Some(K_RADIAL_REACTION_DURATION)
    }
}

impl State for MaterialSwitchState {
    type Widget = MaterialSwitch;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let painter = SwitchPainter::new(app);
        app.get_mut(self).painter = Some(painter);
        ToggleableStateMixin::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MaterialSwitch) {
        if old_widget.value != self.widget(app).value {
            // During a drag we may have modified the curve, reset it if its possible
            // to do without visual discontinuation.
            let position_value = self.position(app).value(app);
            if position_value == 0.0 || position_value == 1.0 {
                match self.widget(app).switch_type {
                    SwitchType::Adaptive => {
                        let context = self.context(app);
                        match Theme::of(app, context).platform {
                            TargetPlatform::Android
                            | TargetPlatform::Fuchsia
                            | TargetPlatform::Linux
                            | TargetPlatform::Windows => self.update_curve(app),
                            TargetPlatform::IOS | TargetPlatform::MacOS => {
                                let position = app.get_mut(self.position(app));
                                position.curve = Curves::linear();
                                position.reverse_curve = Some(Curves::linear());
                            }
                        }
                    }
                    SwitchType::Material => self.update_curve(app),
                }
            }
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
        debug_assert!(debug_check_has_material(app, context));

        if app.get(self).needs_position_animation {
            app.get_mut(self).needs_position_animation = false;
            self.animate_to_value(app);
        }

        let theme = Theme::of(app, context);
        let mut switch_theme = SwitchTheme::of(app, context);
        let cupertino_primary_color = theme
            .cupertino_override_theme
            .as_ref()
            .and_then(|override_theme| override_theme.primary_color.as_ref())
            .map(AnyColor::color)
            .unwrap_or(theme.color_scheme.primary);
        let widget = self.widget(app).clone();

        let switch_config: Box<dyn SwitchConfig>;
        let defaults: SwitchThemeData;
        let mut apply_cupertino_theme = false;
        let mut disabled_opacity = 1.0;
        match widget.switch_type {
            SwitchType::Material => {
                switch_config = self::switch_config(app, context, theme.use_material3);
                defaults = switch_defaults(app, context, theme.use_material3);
            }
            SwitchType::Adaptive => {
                let switch_adaptation = theme
                    .get_adaptation::<SwitchThemeData>()
                    .unwrap_or(&SwitchThemeAdaptation);
                switch_theme = switch_adaptation.adapt(&theme, switch_theme);
                match theme.platform {
                    TargetPlatform::Android
                    | TargetPlatform::Fuchsia
                    | TargetPlatform::Linux
                    | TargetPlatform::Windows => {
                        switch_config = self::switch_config(app, context, theme.use_material3);
                        defaults = switch_defaults(app, context, theme.use_material3);
                    }
                    TargetPlatform::IOS | TargetPlatform::MacOS => {
                        app.get_mut(self).is_cupertino = true;
                        apply_cupertino_theme = widget
                            .apply_cupertino_theme
                            .or_else(|| {
                                theme
                                    .cupertino_override_theme
                                    .as_ref()
                                    .and_then(|override_theme| override_theme.apply_theme_to_all)
                            })
                            .unwrap_or(false);
                        disabled_opacity = 0.5;
                        switch_config = Box::new(SwitchConfigCupertino::new(app, context));
                        defaults = switch_defaults_cupertino(app, context);
                        let reaction_controller = self.reaction_controller(app);
                        app.get_mut(reaction_controller).duration =
                            Some(Duration::from_millis(200));
                    }
                }
            }
        }

        let position_controller = self.position_controller(app);
        app.get_mut(position_controller).duration =
            Some(Duration::from_millis(switch_config.toggle_duration()));

        // Colors need to be resolved in selected and non selected states separately
        // so that they can be lerped between.
        let mut active_states = self.states(app);
        active_states.insert(WidgetState::Selected);
        let mut inactive_states = self.states(app);
        inactive_states.remove(&WidgetState::Selected);

        let widget_thumb_color = Self::widget_thumb_color(&widget);
        let widget_track_color = Self::widget_track_color(&widget);

        let active_thumb_color = resolve(&widget.thumb_color, &active_states)
            .or_else(|| widget_thumb_color.resolve(&active_states))
            .or_else(|| resolve(&switch_theme.thumb_color, &active_states));
        let effective_active_thumb_color = active_thumb_color
            .or_else(|| resolve(&defaults.thumb_color, &active_states))
            .expect("the defaults give a thumb color");
        let inactive_thumb_color = resolve(&widget.thumb_color, &inactive_states)
            .or_else(|| widget_thumb_color.resolve(&inactive_states))
            .or_else(|| resolve(&switch_theme.thumb_color, &inactive_states));
        let effective_inactive_thumb_color = inactive_thumb_color
            .or_else(|| resolve(&defaults.thumb_color, &inactive_states))
            .expect("the defaults give a thumb color");
        let effective_active_track_color = resolve(&widget.track_color, &active_states)
            .or_else(|| widget_track_color.resolve(&active_states))
            .or_else(|| {
                if apply_cupertino_theme {
                    Some(cupertino_primary_color)
                } else {
                    resolve(&switch_theme.track_color, &active_states)
                }
            })
            .or_else(|| {
                widget_thumb_color
                    .resolve(&active_states)
                    .map(|color| color.with_alpha(0x80))
            })
            .or_else(|| resolve(&defaults.track_color, &active_states))
            .expect("the defaults give a track color");
        let effective_active_track_outline_color =
            resolve(&widget.track_outline_color, &active_states)
                .or_else(|| resolve(&switch_theme.track_outline_color, &active_states))
                .or_else(|| {
                    defaults
                        .track_outline_color
                        .as_ref()
                        .expect("the defaults give a track outline color")
                        .resolve(&active_states)
                });
        let effective_active_track_outline_width =
            resolve(&widget.track_outline_width, &active_states)
                .or_else(|| resolve(&switch_theme.track_outline_width, &active_states))
                .or_else(|| resolve(&defaults.track_outline_width, &active_states));

        let effective_inactive_track_color = resolve(&widget.track_color, &inactive_states)
            .or_else(|| widget_track_color.resolve(&inactive_states))
            .or_else(|| resolve(&switch_theme.track_color, &inactive_states))
            .or_else(|| resolve(&defaults.track_color, &inactive_states))
            .expect("the defaults give a track color");
        let effective_inactive_track_outline_color =
            resolve(&widget.track_outline_color, &inactive_states)
                .or_else(|| resolve(&switch_theme.track_outline_color, &inactive_states))
                .or_else(|| resolve(&defaults.track_outline_color, &inactive_states));
        let effective_inactive_track_outline_width =
            resolve(&widget.track_outline_width, &inactive_states)
                .or_else(|| resolve(&switch_theme.track_outline_width, &inactive_states))
                .or_else(|| resolve(&defaults.track_outline_width, &inactive_states));

        let effective_active_icon = resolve(&widget.thumb_icon, &active_states)
            .or_else(|| resolve(&switch_theme.thumb_icon, &active_states));
        let effective_inactive_icon = resolve(&widget.thumb_icon, &inactive_states)
            .or_else(|| resolve(&switch_theme.thumb_icon, &inactive_states));

        let effective_active_icon_color = effective_active_icon
            .as_ref()
            .and_then(|icon| icon.color.as_ref())
            .map(AnyColor::color)
            .unwrap_or_else(|| switch_config.icon_color().resolve(&active_states));
        let effective_inactive_icon_color = effective_inactive_icon
            .as_ref()
            .and_then(|icon| icon.color.as_ref())
            .map(AnyColor::color)
            .unwrap_or_else(|| switch_config.icon_color().resolve(&inactive_states));

        let mut focused_states = self.states(app);
        focused_states.insert(WidgetState::Focused);
        let effective_focus_overlay_color = resolve(&widget.overlay_color, &focused_states)
            .or(widget.focus_color)
            .or_else(|| resolve(&switch_theme.overlay_color, &focused_states))
            .or_else(|| {
                apply_cupertino_theme.then(|| {
                    HSLColor::from_color(with_opacity(cupertino_primary_color, 0.80))
                        .with_lightness(0.69)
                        .with_saturation(0.835)
                        .to_color()
                })
            })
            .or_else(|| resolve(&defaults.overlay_color, &focused_states))
            .expect("the defaults give a focus overlay color");

        let mut hovered_states = self.states(app);
        hovered_states.insert(WidgetState::Hovered);
        let effective_hover_overlay_color = resolve(&widget.overlay_color, &hovered_states)
            .or(widget.hover_color)
            .or_else(|| resolve(&switch_theme.overlay_color, &hovered_states))
            .or_else(|| resolve(&defaults.overlay_color, &hovered_states))
            .expect("the defaults give a hover overlay color");

        let mut active_pressed_states = active_states;
        active_pressed_states.insert(WidgetState::Pressed);
        let effective_active_pressed_thumb_color =
            resolve(&widget.thumb_color, &active_pressed_states)
                .or_else(|| widget_thumb_color.resolve(&active_pressed_states))
                .or_else(|| resolve(&switch_theme.thumb_color, &active_pressed_states))
                .or_else(|| resolve(&defaults.thumb_color, &active_pressed_states))
                .expect("the defaults give a thumb color");
        let effective_active_pressed_overlay_color =
            resolve(&widget.overlay_color, &active_pressed_states)
                .or_else(|| resolve(&switch_theme.overlay_color, &active_pressed_states))
                .or_else(|| {
                    active_thumb_color.map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA))
                })
                .or_else(|| resolve(&defaults.overlay_color, &active_pressed_states))
                .expect("the defaults give a pressed overlay color");

        let mut inactive_pressed_states = inactive_states;
        inactive_pressed_states.insert(WidgetState::Pressed);
        let effective_inactive_pressed_thumb_color =
            resolve(&widget.thumb_color, &inactive_pressed_states)
                .or_else(|| widget_thumb_color.resolve(&inactive_pressed_states))
                .or_else(|| resolve(&switch_theme.thumb_color, &inactive_pressed_states))
                .or_else(|| resolve(&defaults.thumb_color, &inactive_pressed_states))
                .expect("the defaults give a thumb color");
        let effective_inactive_pressed_overlay_color =
            resolve(&widget.overlay_color, &inactive_pressed_states)
                .or_else(|| resolve(&switch_theme.overlay_color, &inactive_pressed_states))
                .or_else(|| {
                    inactive_thumb_color.map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA))
                })
                .or_else(|| resolve(&defaults.overlay_color, &inactive_pressed_states))
                .expect("the defaults give a pressed overlay color");

        let effective_mouse_cursor = {
            let mouse_cursor = widget.mouse_cursor.clone();
            let theme_mouse_cursor = switch_theme.mouse_cursor.clone();
            let default_mouse_cursor = defaults.mouse_cursor.clone();
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(move |states: &WidgetStates| {
                <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                    .or_else(|| resolve(&theme_mouse_cursor, states))
                    .or_else(|| resolve(&default_mouse_cursor, states))
                    .expect("the defaults give a mouse cursor")
            })
        };

        // Dart also takes the thumb radius with an icon when there is an inactive thumb
        // image; the images wait with `DecorationImage`.
        let effective_active_thumb_radius = if effective_active_icon.is_none() {
            switch_config.active_thumb_radius()
        } else {
            switch_config.thumb_radius_with_icon()
        };
        let effective_inactive_thumb_radius = if effective_inactive_icon.is_none() {
            switch_config.inactive_thumb_radius()
        } else {
            switch_config.thumb_radius_with_icon()
        };
        let effective_splash_radius = widget
            .splash_radius
            .or(switch_theme.splash_radius)
            .or(defaults.splash_radius)
            .expect("the defaults give a splash radius");

        let configuration = create_local_image_configuration(app, context, None);
        let track_inner_length = self.track_inner_length(app);
        let text_direction = Directionality::of(app, context);
        let icon_theme = IconTheme::of(app, context);
        let states = self.states(app);
        let is_cupertino = app.get(self).is_cupertino;

        let painter = self.painter(app);
        painter.set_position(app, self.position(app).as_animation());
        painter.set_reaction(app, self.reaction(app).as_animation());
        painter.set_reaction_focus_fade(app, self.reaction_focus_fade(app).as_animation());
        painter.set_reaction_hover_fade(app, self.reaction_hover_fade(app).as_animation());
        painter.set_inactive_reaction_color(app, effective_inactive_pressed_overlay_color);
        painter.set_reaction_color(app, effective_active_pressed_overlay_color);
        painter.set_hover_color(app, effective_hover_overlay_color);
        painter.set_focus_color(app, effective_focus_overlay_color);
        painter.set_splash_radius(app, effective_splash_radius);
        painter.set_down_position(app, self.down_position(app));
        painter.set_is_focused(app, Some(states.contains(&WidgetState::Focused)));
        painter.set_is_hovered(app, Some(states.contains(&WidgetState::Hovered)));
        painter.set_active_color(app, effective_active_thumb_color);
        painter.set_inactive_color(app, effective_inactive_thumb_color);
        painter.set_active_pressed_color(app, effective_active_pressed_thumb_color);
        painter.set_inactive_pressed_color(app, effective_inactive_pressed_thumb_color);
        painter.set_active_track_color(app, effective_active_track_color);
        painter.set_active_track_outline_color(app, effective_active_track_outline_color);
        painter.set_active_track_outline_width(app, effective_active_track_outline_width);
        painter.set_inactive_track_color(app, effective_inactive_track_color);
        painter.set_inactive_track_outline_color(app, effective_inactive_track_outline_color);
        painter.set_inactive_track_outline_width(app, effective_inactive_track_outline_width);
        painter.set_configuration(app, configuration);
        painter.set_is_interactive(app, self.is_interactive(app));
        painter.set_track_inner_length(app, track_inner_length);
        painter.set_text_direction(app, text_direction);
        painter.set_surface_color(app, theme.color_scheme.surface);
        painter.set_inactive_thumb_radius(app, effective_inactive_thumb_radius);
        painter.set_active_thumb_radius(app, effective_active_thumb_radius);
        painter.set_pressed_thumb_radius(app, switch_config.pressed_thumb_radius());
        painter.set_thumb_offset(app, switch_config.thumb_offset());
        painter.set_track_height(app, switch_config.track_height());
        painter.set_track_width(app, switch_config.track_width());
        painter.set_active_icon_color(app, effective_active_icon_color);
        painter.set_inactive_icon_color(app, effective_inactive_icon_color);
        painter.set_active_icon(app, effective_active_icon);
        painter.set_inactive_icon(app, effective_inactive_icon);
        painter.set_icon_theme(app, Some(icon_theme));
        painter.set_thumb_shadow(app, switch_config.thumb_shadow());
        painter.set_transitional_thumb_size(app, switch_config.transitional_thumb_size());
        painter.set_position_controller(app, position_controller);
        painter.set_is_cupertino(app, is_cupertino);

        let opacity = if ToggleableStateMixin::on_changed(self, app).is_none() {
            disabled_opacity
        } else {
            1.0
        };
        let toggleable = self.build_toggleable(
            app,
            widget.focus_node,
            widget.on_focus_change.clone(),
            widget.autofocus,
            Some(effective_mouse_cursor),
            widget.size,
            ToggleablePainterRef::new(painter),
        );
        // The detector's `excludeFromSemantics` waits with the semantics tree.
        let detector = GestureDetector::new()
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
            .child(Opacity::new(opacity).child(toggleable));
        Semantics::new()
            .toggled(widget.value)
            .child(detector)
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
    active_thumb_radius: Option<f64>,
    inactive_thumb_radius: Option<f64>,
    pressed_thumb_radius: Option<f64>,
    thumb_offset: Option<f64>,
    transitional_thumb_size: Option<Size>,
    track_height: Option<f64>,
    track_width: Option<f64>,
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
    is_cupertino: Option<bool>,
    thumb_shadow: Option<Vec<BoxShadow>>,
    text_painter: TextPainter,
    cached_thumb_color: Option<Color>,
    cached_thumb_painter: Option<Box<dyn BoxPainter>>,
    /// Shared with the thumb decoration's change callback, which reads it while the painter
    /// is borrowed for painting.
    is_painting: Rc<Cell<bool>>,
    stop_press_animation: bool,
    pressed_inactive_thumb_radius: Option<f64>,
    pressed_active_thumb_radius: Option<f64>,
    pressed_thumb_extension: f64,
}

/// Dart's `Icon?` setters compare by identity: an `Icon` does not override `==`.
fn same_icon(a: &Option<Rc<Icon>>, b: &Option<Rc<Icon>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// A `late` painter field a setter writes before the first paint, read as Dart's `!`.
macro_rules! painter_field {
    ($field:ident, $setter:ident, $ty:ty) => {
        fn $field(self: Handle<Self>, app: &App) -> $ty {
            app.get(self).$field.expect("set before painting")
        }

        fn $setter(self: Handle<Self>, app: &mut App, value: $ty) {
            if Some(value) == app.get(self).$field {
                return;
            }
            app.get_mut(self).$field = Some(value);
            self.notify_listeners(app);
        }
    };
}

/// A nullable painter field.
macro_rules! nullable_painter_field {
    ($field:ident, $setter:ident, $ty:ty) => {
        fn $field(self: Handle<Self>, app: &App) -> Option<$ty> {
            app.get(self).$field
        }

        fn $setter(self: Handle<Self>, app: &mut App, value: Option<$ty>) {
            if value == app.get(self).$field {
                return;
            }
            app.get_mut(self).$field = value;
            self.notify_listeners(app);
        }
    };
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
            active_thumb_radius: None,
            inactive_thumb_radius: None,
            pressed_thumb_radius: None,
            thumb_offset: None,
            transitional_thumb_size: None,
            track_height: None,
            track_width: None,
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
            is_cupertino: None,
            thumb_shadow: None,
            text_painter: TextPainter::new(),
            cached_thumb_color: None,
            cached_thumb_painter: None,
            is_painting: Rc::new(Cell::new(false)),
            stop_press_animation: false,
            pressed_inactive_thumb_radius: None,
            pressed_active_thumb_radius: None,
            pressed_thumb_extension: 0.0,
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

    painter_field!(active_icon_color, set_active_icon_color, Color);
    painter_field!(inactive_icon_color, set_inactive_icon_color, Color);
    painter_field!(active_pressed_color, set_active_pressed_color, Color);
    painter_field!(inactive_pressed_color, set_inactive_pressed_color, Color);
    painter_field!(active_thumb_radius, set_active_thumb_radius, f64);
    painter_field!(inactive_thumb_radius, set_inactive_thumb_radius, f64);
    painter_field!(pressed_thumb_radius, set_pressed_thumb_radius, f64);
    nullable_painter_field!(thumb_offset, set_thumb_offset, f64);
    painter_field!(transitional_thumb_size, set_transitional_thumb_size, Size);
    painter_field!(track_height, set_track_height, f64);
    painter_field!(track_width, set_track_width, f64);
    painter_field!(active_track_color, set_active_track_color, Color);
    nullable_painter_field!(
        active_track_outline_color,
        set_active_track_outline_color,
        Color
    );
    nullable_painter_field!(
        inactive_track_outline_color,
        set_inactive_track_outline_color,
        Color
    );
    nullable_painter_field!(
        active_track_outline_width,
        set_active_track_outline_width,
        f64
    );
    nullable_painter_field!(
        inactive_track_outline_width,
        set_inactive_track_outline_width,
        f64
    );
    painter_field!(inactive_track_color, set_inactive_track_color, Color);

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

    painter_field!(text_direction, set_text_direction, TextDirection);
    painter_field!(surface_color, set_surface_color, Color);
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

    painter_field!(track_inner_length, set_track_inner_length, f64);
    painter_field!(is_cupertino, set_is_cupertino, bool);

    fn thumb_shadow(self: Handle<Self>, app: &App) -> Option<&[BoxShadow]> {
        app.get(self).thumb_shadow.as_deref()
    }

    fn set_thumb_shadow(self: Handle<Self>, app: &mut App, value: Option<Vec<BoxShadow>>) {
        if value == app.get(self).thumb_shadow {
            return;
        }
        app.get_mut(self).thumb_shadow = value;
        self.notify_listeners(app);
    }

    /// Dart's `_createDefaultThumbDecoration`, without the thumb image (see `PORTING.md`).
    fn create_default_thumb_decoration(
        self: Handle<Self>,
        app: &App,
        color: Color,
    ) -> ShapeDecoration {
        let decoration = ShapeDecoration::new(StadiumBorder::new(BorderSide::NONE)).color(color);
        match self.thumb_shadow(app) {
            Some(thumb_shadow) if !self.is_cupertino(app) => {
                decoration.shadows(thumb_shadow.to_vec())
            }
            _ => decoration,
        }
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

    /// Dart's local `thumbSizeAnimation(isForward).value`: the size sequence evaluated at the
    /// position controller, with the tweens it makes freed once it has been read.
    fn thumb_size_animation_value(
        self: Handle<Self>,
        app: &mut App,
        is_forward: bool,
        inactive_thumb_size: Size,
        active_thumb_size: Size,
    ) -> Size {
        let transitional_thumb_size = self.transitional_thumb_size(app);
        let cubic = |a, b, c, d| Rc::new(Cubic::new(a, b, c, d)) as Rc<dyn Curve>;
        let mut owned = Vec::new();
        let mut tween = |app: &mut App, begin: Size, end: Size, curve: Rc<dyn Curve>| {
            let tween = Tween::new(app, Some(begin), Some(end));
            let curve = CurveTween::new(app, curve);
            owned.extend([tween.id(), curve.id()]);
            Arc::new(tween.chain(curve)) as Arc<dyn Animatable<Size>>
        };
        let thumb_size_sequence = if is_forward {
            let first = tween(
                app,
                inactive_thumb_size,
                transitional_thumb_size,
                cubic(0.31, 0.00, 0.56, 1.00),
            );
            let second = tween(
                app,
                transitional_thumb_size,
                active_thumb_size,
                cubic(0.20, 0.00, 0.00, 1.00),
            );
            let constant = ConstantTween::new(app, active_thumb_size);
            owned.push(constant.id());
            vec![
                TweenSequenceItem::new(first, 11.0),
                TweenSequenceItem::new(second, 72.0),
                TweenSequenceItem::new(Arc::new(constant), 17.0),
            ]
        } else {
            let constant = ConstantTween::new(app, inactive_thumb_size);
            let first = tween(
                app,
                inactive_thumb_size,
                transitional_thumb_size,
                Rc::new(cubic(0.20, 0.00, 0.00, 1.00).flipped()),
            );
            let second = tween(
                app,
                transitional_thumb_size,
                active_thumb_size,
                Rc::new(cubic(0.31, 0.00, 0.56, 1.00).flipped()),
            );
            owned.push(constant.id());
            vec![
                TweenSequenceItem::new(Arc::new(constant), 17.0),
                TweenSequenceItem::new(first, 72.0),
                TweenSequenceItem::new(second, 11.0),
            ]
        };

        let position_controller = self.position_controller(app);
        let value = TweenSequence::new(thumb_size_sequence)
            .evaluate(app, position_controller.as_animation());
        for id in owned {
            app.destroy(id);
        }
        value
    }

    /// Computes canvas offset for track's upper left corner
    fn compute_track_paint_offset(
        canvas_size: Size,
        track_width: f64,
        track_height: f64,
    ) -> Offset {
        let horizontal_offset = (canvas_size.width() - track_width) / 2.0;
        let vertical_offset = (canvas_size.height() - track_height) / 2.0;

        Offset::new(horizontal_offset, vertical_offset)
    }

    /// Computes canvas offset for thumb's upper left corner as if it were a
    /// square
    fn compute_thumb_paint_offset(
        self: Handle<Self>,
        app: &App,
        track_paint_offset: Offset,
        thumb_size: Size,
        visual_position: f64,
    ) -> Offset {
        // How much thumb radius extends beyond the track
        let track_radius = self.track_height(app) / 2.0;
        let additional_thumb_radius = thumb_size.height() / 2.0 - track_radius;

        let pressed_thumb_extension = app.get(self).pressed_thumb_extension;
        let horizontal_progress =
            visual_position * (self.track_inner_length(app) - pressed_thumb_extension);
        let thumb_horizontal_offset =
            track_paint_offset.dx() + track_radius + (pressed_thumb_extension / 2.0)
                - thumb_size.width() / 2.0
                + horizontal_progress;
        let thumb_vertical_offset = track_paint_offset.dy() - additional_thumb_radius;
        Offset::new(thumb_horizontal_offset, thumb_vertical_offset)
    }

    fn paint_track_with(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        paint: &Paint,
        track_paint_offset: Offset,
        track_outline_color: Option<Color>,
        track_outline_width: Option<f64>,
    ) {
        let track_width = self.track_width(app);
        let track_height = self.track_height(app);
        let track_rect = Rect::from_ltwh(
            track_paint_offset.dx(),
            track_paint_offset.dy(),
            track_width,
            track_height,
        );
        let track_radius = track_height / 2.0;
        let track_rrect = RRect::from_rect_and_radius(track_rect, Radius::circular(track_radius));

        draw_rrect(canvas, track_rrect, paint);

        // paint track outline
        if let Some(track_outline_color) = track_outline_color {
            let outline_track_rect = Rect::from_ltwh(
                track_paint_offset.dx() + 1.0,
                track_paint_offset.dy() + 1.0,
                track_width - 2.0,
                track_height - 2.0,
            );
            let outline_track_rrect =
                RRect::from_rect_and_radius(outline_track_rect, Radius::circular(track_radius));

            let outline_paint = Paint {
                style: PaintStyle::Stroke(Stroke::new(track_outline_width.unwrap_or(2.0) as f32)),
                color: track_outline_color.into(),
                ..Paint::default()
            };

            draw_rrect(canvas, outline_track_rrect, &outline_paint);
        }

        if self.is_cupertino(app) {
            if self.is_focused(app) {
                let focused_outline = track_rrect.inflate(1.75);
                let focused_paint = Paint {
                    style: PaintStyle::Stroke(Stroke::new(K_CUPERTINO_FOCUS_TRACK_OUTLINE as f32)),
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
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_thumb_with(
        self: Handle<Self>,
        app: &mut App,
        thumb_paint_offset: Offset,
        canvas: &mut Canvas,
        current_value: f64,
        thumb_color: Color,
        thumb_icon: Option<Rc<Icon>>,
        thumb_size: Size,
    ) {
        let is_painting = Rc::clone(&app.get(self).is_painting);
        let _painting = PaintingGuard::enter(&is_painting);
        self.update_cached_thumb_painter(app, thumb_color);

        if self.is_cupertino(app) {
            self.paint_cupertino_thumb_shadow_and_border(
                app,
                canvas,
                thumb_paint_offset,
                thumb_size,
            );
        }

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
            let icon_size = thumb_icon.size.unwrap_or(SwitchConfigM3::ICON_SIZE);
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
        let painter = app.get(self);
        if painter.cached_thumb_painter.is_some() && painter.cached_thumb_color == Some(thumb_color)
        {
            return;
        }
        let decoration = self.create_default_thumb_decoration(app, thumb_color);
        let painter = app.get_mut(self);
        painter.cached_thumb_color = Some(thumb_color);
        if let Some(mut old) = painter.cached_thumb_painter.take() {
            old.dispose();
        }
        let is_painting = Rc::clone(&painter.is_painting);
        let on_changed: Rc<dyn Fn(&mut App)> =
            Rc::new(move |app: &mut App| self.handle_decoration_changed(app, &is_painting));
        app.get_mut(self).cached_thumb_painter =
            Some(decoration.create_box_painter(Some(on_changed)));
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
        app: &App,
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
        if let Some(thumb_shadow) = self.thumb_shadow(app) {
            for shadow in thumb_shadow {
                draw_rrect(
                    canvas,
                    thumb_bounds.shift(shadow.offset),
                    &shadow.to_paint(),
                );
            }
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

        let is_cupertino = self.is_cupertino(app);
        let inactive_thumb_radius = self.inactive_thumb_radius(app);
        let active_thumb_radius = self.active_thumb_radius(app);
        let pressed_thumb_radius = self.pressed_thumb_radius(app);

        // To get the thumb radius when the press ends, the value can be any number
        // between activeThumbRadius/inactiveThumbRadius and pressedThumbRadius.
        if !app.get(self).stop_press_animation {
            let reaction_value = reaction.value(app);
            let pressed_thumb_extension = if is_cupertino {
                reaction_value * 7.0
            } else {
                0.0
            };
            let painter = app.get_mut(self);
            painter.pressed_thumb_extension = pressed_thumb_extension;
            if reaction.is_completed(app) {
                // This happens when the thumb is dragged instead of being tapped.
                let painter = app.get_mut(self);
                painter.pressed_inactive_thumb_radius = lerp_double(
                    Some(inactive_thumb_radius),
                    Some(pressed_thumb_radius),
                    reaction_value,
                );
                painter.pressed_active_thumb_radius = lerp_double(
                    Some(active_thumb_radius),
                    Some(pressed_thumb_radius),
                    reaction_value,
                );
            }
            let painter = app.get_mut(self);
            if current_value == 0.0 {
                painter.pressed_inactive_thumb_radius = lerp_double(
                    Some(inactive_thumb_radius),
                    Some(pressed_thumb_radius),
                    reaction_value,
                );
                painter.pressed_active_thumb_radius = Some(active_thumb_radius);
            }
            if current_value == 1.0 {
                painter.pressed_active_thumb_radius = lerp_double(
                    Some(active_thumb_radius),
                    Some(pressed_thumb_radius),
                    reaction_value,
                );
                painter.pressed_inactive_thumb_radius = Some(inactive_thumb_radius);
            }
        }
        let painter = app.get(self);
        let pressed_thumb_extension = painter.pressed_thumb_extension;
        let effective_pressed_inactive_thumb_radius = painter
            .pressed_inactive_thumb_radius
            .unwrap_or(inactive_thumb_radius);
        let effective_pressed_active_thumb_radius = painter
            .pressed_active_thumb_radius
            .unwrap_or(active_thumb_radius);
        let inactive_thumb_size = if is_cupertino {
            Size::new(
                effective_pressed_inactive_thumb_radius * 2.0 + pressed_thumb_extension,
                effective_pressed_inactive_thumb_radius * 2.0,
            )
        } else {
            Size::from_radius(effective_pressed_inactive_thumb_radius)
        };
        let active_thumb_size = if is_cupertino {
            Size::new(
                effective_pressed_active_thumb_radius * 2.0 + pressed_thumb_extension,
                effective_pressed_active_thumb_radius * 2.0,
            )
        } else {
            Size::from_radius(effective_pressed_active_thumb_radius)
        };

        let position = self.position(app);
        let position_forward_or_dismissed =
            position.is_dismissed(app) || position.status(app) == AnimationStatus::Forward;
        // Dart's two Cupertino lerp branches are the same.
        #[allow(clippy::if_same_then_else)]
        let thumb_size = if is_cupertino {
            if reaction.is_completed(app) {
                Size::new(
                    effective_pressed_inactive_thumb_radius * 2.0 + pressed_thumb_extension,
                    effective_pressed_inactive_thumb_radius * 2.0,
                )
            } else if position_forward_or_dismissed {
                Size::lerp(
                    Some(inactive_thumb_size),
                    Some(active_thumb_size),
                    position.value(app),
                )
                .expect("lerping non-null sizes")
            } else {
                Size::lerp(
                    Some(inactive_thumb_size),
                    Some(active_thumb_size),
                    position.value(app),
                )
                .expect("lerping non-null sizes")
            }
        } else if reaction.is_completed(app) {
            Size::from_radius(pressed_thumb_radius)
        } else if position_forward_or_dismissed {
            self.thumb_size_animation_value(app, true, inactive_thumb_size, active_thumb_size)
        } else {
            self.thumb_size_animation_value(app, false, inactive_thumb_size, active_thumb_size)
        };

        // The thumb contracts slightly during the animation in Material 2.
        let inset = match self.thumb_offset(app) {
            None => 0.0,
            Some(thumb_offset) => 1.0 - (current_value - thumb_offset).abs() * 2.0,
        };
        let thumb_size = Size::new(thumb_size.width() - inset, thumb_size.height() - inset);

        let color_value = app
            .get(self)
            .color_animation
            .expect("set with the position controller")
            .value(app);
        let track_color = Color::lerp(
            Some(self.inactive_track_color(app)),
            Some(self.active_track_color(app)),
            color_value,
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

        let track_paint_offset =
            Self::compute_track_paint_offset(size, self.track_width(app), self.track_height(app));
        let thumb_paint_offset =
            self.compute_thumb_paint_offset(app, track_paint_offset, thumb_size, visual_position);
        let radial_reaction_origin = Offset::new(
            thumb_paint_offset.dx() + thumb_size.height() / 2.0,
            size.height() / 2.0,
        );

        self.paint_track_with(
            app,
            canvas,
            &paint,
            track_paint_offset,
            track_outline_color,
            track_outline_width,
        );
        self.paint_radial_reaction(app, canvas, Offset::ZERO, radial_reaction_origin);
        self.paint_thumb_with(
            app,
            thumb_paint_offset,
            canvas,
            color_value,
            thumb_color,
            thumb_icon,
            thumb_size,
        );
    }
}

/// Dart's `_SwitchThemeAdaptation`.
#[derive(Debug)]
struct SwitchThemeAdaptation;

impl Adaptation<SwitchThemeData> for SwitchThemeAdaptation {
    fn adapt(&self, theme: &ThemeData, default_value: SwitchThemeData) -> SwitchThemeData {
        match theme.platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => default_value,
            TargetPlatform::IOS | TargetPlatform::MacOS => SwitchThemeData::new(),
        }
    }
}

/// Dart's `mixin _SwitchConfig`: the sizes, shadows and icon colours of one look of the
/// switch.
pub(crate) trait SwitchConfig {
    fn track_height(&self) -> f64;
    fn track_width(&self) -> f64;
    fn switch_width(&self) -> f64;
    fn switch_height(&self) -> f64;
    fn switch_height_collapsed(&self) -> f64;
    fn active_thumb_radius(&self) -> f64;
    fn inactive_thumb_radius(&self) -> f64;
    fn pressed_thumb_radius(&self) -> f64;
    fn thumb_radius_with_icon(&self) -> f64;
    fn thumb_shadow(&self) -> Option<Vec<BoxShadow>>;
    fn icon_color(&self) -> WidgetStatePropertyRef<Color>;
    fn thumb_offset(&self) -> Option<f64>;
    fn transitional_thumb_size(&self) -> Size;
    /// In milliseconds.
    fn toggle_duration(&self) -> u64;
    fn switch_min_size(&self) -> Size;
}

/// Hand coded defaults for iOS/macOS Switch: Dart's `_SwitchDefaultsCupertino(context)`, the
/// [`SwitchThemeData`] whose fields are the values of that subclass's getters.
fn switch_defaults_cupertino(app: &mut App, context: BuildContext) -> SwitchThemeData {
    let system_green =
        CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_GREEN, app, context).color();
    let secondary_system_fill =
        CupertinoDynamicColor::resolve(&CupertinoColors::SECONDARY_SYSTEM_FILL, app, context)
            .color();
    SwitchThemeData::new()
        .mouse_cursor(<dyn WidgetStateProperty<_>>::resolve_with(
            |states: &WidgetStates| -> Option<MouseCursorRef> {
                if states.contains(&WidgetState::Disabled) {
                    return Some(SystemMouseCursors::BASIC.into());
                }
                Some(if K_IS_WEB {
                    SystemMouseCursors::CLICK.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                })
            },
        ))
        .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::WHITE,
        )))
        .track_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    return Some(system_green);
                }
                Some(secondary_system_fill)
            },
        ))
        .track_outline_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::TRANSPARENT,
        )))
        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Focused) {
                    return Some(
                        HSLColor::from_color(with_opacity(system_green, 0.80))
                            .with_lightness(0.69)
                            .with_saturation(0.835)
                            .to_color(),
                    );
                }
                Some(Colors::TRANSPARENT)
            },
        ))
        .splash_radius(0.0)
}

const K_CUPERTINO_FOCUS_TRACK_OUTLINE: f64 = 3.5;

/// Dart's `_SwitchConfigCupertino`.
struct SwitchConfigCupertino {
    colors: ColorScheme,
}

impl SwitchConfigCupertino {
    /// Dart's `_SwitchConfigCupertino(context)`.
    fn new(app: &mut App, context: BuildContext) -> SwitchConfigCupertino {
        SwitchConfigCupertino {
            colors: Theme::of(app, context).color_scheme.clone(),
        }
    }
}

impl SwitchConfig for SwitchConfigCupertino {
    fn icon_color(&self) -> WidgetStatePropertyRef<Color> {
        let colors = self.colors.clone();
        <dyn WidgetStateProperty<Color>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return with_opacity(colors.on_surface, 0.38);
            }
            colors.on_primary_container()
        })
    }

    fn active_thumb_radius(&self) -> f64 {
        14.0
    }

    fn inactive_thumb_radius(&self) -> f64 {
        14.0
    }

    fn pressed_thumb_radius(&self) -> f64 {
        14.0
    }

    fn switch_height(&self) -> f64 {
        self.switch_min_size().height() + 8.0
    }

    fn switch_height_collapsed(&self) -> f64 {
        self.switch_min_size().height()
    }

    fn switch_width(&self) -> f64 {
        60.0
    }

    fn thumb_radius_with_icon(&self) -> f64 {
        14.0
    }

    fn thumb_shadow(&self) -> Option<Vec<BoxShadow>> {
        Some(vec![
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
        ])
    }

    fn track_height(&self) -> f64 {
        31.0
    }

    fn track_width(&self) -> f64 {
        51.0
    }

    // The thumb size at the middle of the track. Hand coded default based on the animation specs.
    fn transitional_thumb_size(&self) -> Size {
        Size::new(28.0, 28.0)
    }

    // Hand coded default by comparing with [CupertinoSwitch].
    fn toggle_duration(&self) -> u64 {
        140
    }

    // Hand coded default based on the animation specs.
    fn thumb_offset(&self) -> Option<f64> {
        None
    }

    fn switch_min_size(&self) -> Size {
        Size::square(K_MIN_INTERACTIVE_DIMENSION - 8.0)
    }
}

/// Hand coded defaults based on Material Design 2: Dart's `_SwitchConfigM2`.
struct SwitchConfigM2;

impl SwitchConfigM2 {
    /// Dart's `_SwitchConfigM2()`.
    fn new() -> SwitchConfigM2 {
        SwitchConfigM2
    }
}

impl SwitchConfig for SwitchConfigM2 {
    fn active_thumb_radius(&self) -> f64 {
        10.0
    }

    fn icon_color(&self) -> WidgetStatePropertyRef<Color> {
        <dyn WidgetStateProperty<Color>>::all(Colors::TRANSPARENT)
    }

    fn inactive_thumb_radius(&self) -> f64 {
        10.0
    }

    fn pressed_thumb_radius(&self) -> f64 {
        10.0
    }

    fn switch_height(&self) -> f64 {
        self.switch_min_size().height() + 8.0
    }

    fn switch_height_collapsed(&self) -> f64 {
        self.switch_min_size().height()
    }

    fn switch_width(&self) -> f64 {
        self.track_width() - 2.0 * (self.track_height() / 2.0) + self.switch_min_size().width()
    }

    fn thumb_radius_with_icon(&self) -> f64 {
        10.0
    }

    fn thumb_shadow(&self) -> Option<Vec<BoxShadow>> {
        k_elevation_to_shadow(1).map(<[BoxShadow]>::to_vec)
    }

    fn track_height(&self) -> f64 {
        14.0
    }

    fn track_width(&self) -> f64 {
        33.0
    }

    fn thumb_offset(&self) -> Option<f64> {
        Some(0.5)
    }

    fn transitional_thumb_size(&self) -> Size {
        Size::new(20.0, 20.0)
    }

    fn toggle_duration(&self) -> u64 {
        200
    }

    fn switch_min_size(&self) -> Size {
        Size::square(K_MIN_INTERACTIVE_DIMENSION - 8.0)
    }
}

/// Dart's `_SwitchDefaultsM2(context)`: the [`SwitchThemeData`] whose fields are the values of
/// that subclass's getters.
fn switch_defaults_m2(app: &mut App, context: BuildContext) -> SwitchThemeData {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let is_dark = theme.brightness() == Brightness::Dark;

    let thumb_color = {
        let secondary = colors.secondary;
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return Some(if is_dark {
                    Colors::GREY.shade800()
                } else {
                    Colors::GREY.shade400()
                });
            }
            if states.contains(&WidgetState::Selected) {
                return Some(secondary);
            }
            Some(if is_dark {
                Colors::GREY.shade400()
            } else {
                Colors::GREY.shade50()
            })
        })
    };

    let track_color = {
        let secondary = colors.secondary;
        const BLACK32: Color = Color::new(0x52000000); // Black with 32% opacity
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return Some(if is_dark {
                    Colors::WHITE10
                } else {
                    Colors::BLACK12
                });
            }
            if states.contains(&WidgetState::Selected) {
                let active_color = secondary;
                return Some(active_color.with_alpha(0x80));
            }
            Some(if is_dark { Colors::WHITE30 } else { BLACK32 })
        })
    };

    let overlay_color = {
        let thumb_color = Rc::clone(&thumb_color);
        let (hover_color, focus_color) = (theme.hover_color, theme.focus_color);
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Pressed) {
                return thumb_color
                    .resolve(states)
                    .map(|color| color.with_alpha(K_RADIAL_REACTION_ALPHA));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(hover_color);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(focus_color);
            }
            None
        })
    };

    SwitchThemeData::new()
        .thumb_color(thumb_color)
        .track_color(track_color)
        .track_outline_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::TRANSPARENT,
        )))
        .material_tap_target_size(theme.material_tap_target_size)
        .mouse_cursor(<dyn WidgetStateProperty<_>>::resolve_with(
            |states: &WidgetStates| Some(WidgetStateMouseCursor::clickable().resolve(states)),
        ))
        .overlay_color(overlay_color)
        .splash_radius(K_RADIAL_REACTION_RADIUS)
        .padding(EdgeInsetsGeometry::ZERO)
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/switch_test.dart`.
    //!
    //! Dart's `paints` matcher records every draw; this recorder does not record a draw whose
    //! colour is fully transparent, so the expectations below leave out Dart's
    //! `rrect(color: Color(0x00000000))` entries (the transparent track outline).

    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::PointerChange;
    use waft_embedder::valo::Op;
    use waft_painting::AlignmentGeometry;
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject, RenderOpacity,
    };
    use waft_scheduler::SchedulerBinding;
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        Align, AnyElement, Center, Container, FocusHighlightStrategy, FocusNode, FocusNodeLeaf,
        IconData, MouseRegion, StateSetter, StatefulBuilder, StatefulElement, UnfocusDisposition,
    };

    use super::*;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::{TestPointer, WidgetTester, with_main_thread_stack};
    use crate::theme_data::AdaptationRef;
    use crate::tooltip::tests::{m2, m3, pump_app, tester, tester_on};

    /// One recorded op, reduced to what Dart's `paints` matchers compare.
    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Drawn {
        /// A filled shape (`rrect`, `circle`): its colour as `0xAARRGGBB` and its bounds.
        Fill(u32, Rect),
        /// A stroked shape: its colour, its stroke width and the bounds of its path.
        Stroke(u32, f32, Rect),
        /// A blurred shadow.
        Shadow(u32),
        Clip,
        /// A laid-out paragraph: the left and top of its ink.
        Glyphs(f64, f64),
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
            match op {
                Op::DrawPath { path, paint, .. } => Some(match &paint.style {
                    PaintStyle::Stroke(stroke) => {
                        Drawn::Stroke(argb(paint.color), stroke.width, rect_of(path.bounds()))
                    }
                    PaintStyle::Fill => Drawn::Fill(argb(paint.color), rect_of(path.bounds())),
                }),
                Op::DrawRect { paint, rect, .. } => {
                    Some(Drawn::Fill(argb(paint.color), rect_of(*rect)))
                }
                Op::RRectBlur { paint, .. } => Some(Drawn::Shadow(argb(paint.color))),
                Op::ClipPath { .. } => Some(Drawn::Clip),
                Op::GlyphRun { content_bounds, .. } => Some(Drawn::Glyphs(
                    f64::from(content_bounds.x),
                    f64::from(content_bounds.y),
                )),
                _ => None,
            }
        }

        pub(crate) fn color(&self) -> Option<u32> {
            match self {
                Drawn::Fill(color, _) | Drawn::Stroke(color, _, _) | Drawn::Shadow(color) => {
                    Some(*color)
                }
                Drawn::Clip | Drawn::Glyphs(..) => None,
            }
        }
    }

    /// The colours of what was drawn, clips and glyphs left out.
    pub(crate) fn colors(drawn: &[Drawn]) -> Vec<u32> {
        drawn.iter().filter_map(Drawn::color).collect()
    }

    pub(crate) fn c(color: Color) -> u32 {
        color.to_argb32()
    }

    fn ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect::from_ltrb(left, top, right, bottom)
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

    fn switch_element(tester: &WidgetTester) -> AnyElement {
        *tester.find_by_type::<Switch>().first().expect("a Switch")
    }

    /// Dart's `tester.getRect(find.byType(Switch))`.
    pub(crate) fn switch_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(switch_element(tester)))
    }

    /// What the switch's painter draws, in paint order: Dart's `paints` matcher on the
    /// switch's `Material`.
    pub(crate) fn drawn(tester: &WidgetTester) -> Vec<Drawn> {
        let render = tester.render_box_of(switch_element(tester)).as_object();
        let mut app = tester.cell.borrow_mut();
        let custom = find::<RenderCustomPaint>(&app, render).expect("the switch's CustomPaint");
        let painter = custom.painter(&app).expect("the switch's painter");
        let size = custom.size(&app);
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, size);
        canvas.build().ops().iter().filter_map(Drawn::of).collect()
    }

    /// The opacity the switch is painted with.
    fn opacity(tester: &WidgetTester) -> f64 {
        let render = tester.render_box_of(switch_element(tester)).as_object();
        let app = tester.cell.borrow();
        find::<RenderOpacity>(&app, render)
            .expect("the switch's Opacity")
            .opacity(&app)
    }

    /// Dart's `tester.state<ToggleableStateMixin>(find.byWidgetPredicate(... '_MaterialSwitch'))`.
    fn state(tester: &WidgetTester) -> Handle<MaterialSwitchState> {
        let element = *tester
            .find_by_type::<MaterialSwitch>()
            .first()
            .expect("a _MaterialSwitch");
        let app = tester.cell.borrow();
        element
            .downcast::<StatefulElement<MaterialSwitch>>(&app)
            .expect("a stateful element")
            .state(&app)
    }

    /// Dart's `state.position.value`.
    fn position(tester: &WidgetTester) -> f64 {
        let state = state(tester);
        let app = tester.cell.borrow();
        state.position(&app).value(&app)
    }

    /// Dart's `tester.hasRunningAnimations`.
    fn has_running_animations(tester: &WidgetTester) -> bool {
        SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut()) > 0
    }

    fn colors_of(app: &App, theme: fn(&App) -> ThemeData) -> ColorScheme {
        theme(app).color_scheme
    }

    /// The value a Dart test's `StatefulBuilder` owns.
    type Shared = Rc<Cell<bool>>;

    fn shared(value: bool) -> Shared {
        Rc::new(Cell::new(value))
    }

    /// Dart's `StatefulBuilder(builder: (context, setState) => make(value, onChanged))`, whose
    /// `onChanged`, when `enabled`, sets the value in `setState`.
    fn host(
        value: &Shared,
        enabled: bool,
        make: impl Fn(bool, Option<ValueChanged<bool>>) -> Switch + 'static,
    ) -> WidgetRef {
        let value = Rc::clone(value);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let on_changed = enabled.then(|| {
                let value = Rc::clone(&value);
                Rc::new(move |app: &mut App, new_value: bool| {
                    let value = Rc::clone(&value);
                    set_state(app, &mut move |_app| value.set(new_value));
                }) as ValueChanged<bool>
            });
            make(value.get(), on_changed).into_widget()
        })
        .into_widget()
    }

    /// `Directionality(textDirection: direction, child: Material(child: Center(child: child)))`.
    fn in_material(direction: TextDirection, child: WidgetRef) -> WidgetRef {
        Directionality::new(direction, Material::new().child(Center::new().child(child)))
            .into_widget()
    }

    /// An `onChanged` that ignores its value: Dart's `(_) {}`.
    pub(crate) fn ignore() -> Option<ValueChanged<bool>> {
        Some(Rc::new(|_app: &mut App, _value: bool| {}))
    }

    /// `MaterialApp(theme: theme, home: Directionality(direction, Material(Center(host))))`
    /// around a switch that owns `value`.
    fn pump_host(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        direction: TextDirection,
        value: &Shared,
        enabled: bool,
        make: impl Fn(bool, Option<ValueChanged<bool>>) -> Switch + 'static,
    ) {
        let home = in_material(direction, host(value, enabled, make));
        pump_app(tester, theme, move |_app| home);
    }

    /// `MaterialApp(theme: theme, home: Material(Center(switch)))`.
    fn pump_switch(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        switch: Switch,
    ) {
        let home = in_material(TextDirection::Ltr, switch.into_widget());
        pump_app(tester, theme, move |_app| home);
    }

    pub(crate) fn center(tester: &WidgetTester) -> Offset {
        switch_rect(tester).center()
    }

    /// Dart's `tester.tap(find.byType(Switch))`.
    pub(crate) fn tap(tester: &mut WidgetTester) {
        let at = center(tester);
        tester.tap(at, 1);
    }

    /// Dart's `tester.drag(find.byType(Switch), Offset(dx, 0.0))`.
    fn drag(tester: &mut WidgetTester, dx: f64) {
        let at = center(tester);
        tester.drag_from(at, Offset::new(dx, 0.0), 1);
    }

    /// Dart's `tester.press(find.byType(Switch))`: a pointer down at the centre, kept down.
    pub(crate) fn press(tester: &mut WidgetTester) -> TestPointer {
        let at = center(tester);
        let mut touch = WidgetTester::touch(2);
        tester.down(&mut touch, at);
        touch
    }

    /// Dart's `createGesture(kind: mouse)`, `addPointer()` and `moveTo(center)`.
    pub(crate) fn hover(tester: &mut WidgetTester) -> TestPointer {
        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        let at = center(tester);
        tester.move_to(&mut mouse, at, false);
        mouse
    }

    fn focus_node(tester: &WidgetTester) -> AnyFocusNode {
        FocusNode::new(&mut tester.cell.borrow_mut()).as_node()
    }

    fn adaptive_on(platform: TargetPlatform) -> impl FnOnce(&App) -> ThemeData {
        move |app| ThemeData::builder().platform(platform).build(app)
    }

    const MOBILE: [TargetPlatform; 2] = [TargetPlatform::Android, TargetPlatform::Fuchsia];
    const CUPERTINO_PLATFORMS: [TargetPlatform; 2] = [TargetPlatform::IOS, TargetPlatform::MacOS];
    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    pub(crate) const M2_SHADOWS: [u32; 3] = [0x33000000, 0x24000000, 0x1f000000];
    const CUPERTINO_SHADOWS_AND_BORDER: [u32; 3] = [0x26000000, 0x0f000000, 0x0a000000];
    const M2_TRACK: Rect = Rect::from_ltrb(13.0, 17.0, 46.0, 31.0);
    const M3_TRACK: Rect = Rect::from_ltrb(4.0, 8.0, 56.0, 40.0);
    const M3_OUTLINE: Rect = Rect::from_ltrb(5.0, 9.0, 55.0, 39.0);

    /// An M2 switch's draws: the track, the reaction circles in `between`, the thumb's three
    /// shadows and the thumb.
    fn m2_sequence(track: u32, between: &[u32], thumb: u32) -> Vec<u32> {
        [&[track][..], between, &M2_SHADOWS[..], &[thumb][..]].concat()
    }

    #[test]
    fn a_tap_toggles_the_switch() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            assert!(!value.get());
            tap(&mut tester);
            assert!(value.get());
        });
    }

    #[test]
    fn the_switch_size_follows_the_theme_material_tap_target_size() {
        with_main_thread_stack(|| {
            // switch width = trackWidth - 2 * trackRadius + _kSwitchMinSize
            // M2 width = 33 - 2 * 7 + 40
            // M3 width = 52 - 2 * 16 + 40
            for (theme, padded, shrink_wrapped) in [
                (
                    m3 as fn(&App) -> ThemeData,
                    Size::new(60.0, 48.0),
                    Size::new(60.0, 40.0),
                ),
                (m2, Size::new(59.0, 48.0), Size::new(59.0, 40.0)),
            ] {
                for (size, expected) in [
                    (MaterialTapTargetSize::Padded, padded),
                    (MaterialTapTargetSize::ShrinkWrap, shrink_wrapped),
                ] {
                    let mut tester = tester();
                    pump_switch(
                        &mut tester,
                        move |app| {
                            theme(app)
                                .copy_with()
                                .material_tap_target_size(size)
                                .build()
                        },
                        Switch::new(true, ignore()).drag_start_behavior(DragStartBehavior::Down),
                    );
                    assert_eq!(switch_rect(&tester).size(), expected, "{size:?}");
                }
            }
        });
    }

    #[test]
    fn a_drag_toggles_the_switch_toward_its_direction() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            assert!(!value.get());
            drag(&mut tester, -30.0);
            assert!(!value.get());
            drag(&mut tester, 30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, 30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, -30.0);
            assert!(!value.get());
        });
    }

    #[test]
    fn a_drag_toggles_the_switch_with_either_drag_start_behavior() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            assert!(!value.get());
            drag(&mut tester, -30.0);
            assert!(!value.get());
            drag(&mut tester, 30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, 30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, -30.0);
            assert!(!value.get());

            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                Switch::new,
            );
            tester.pump_and_settle();
            let start = center(&tester);
            for (dx, before_up, after_up) in [
                (20.0, false, true),
                (20.0, true, true),
                (-20.0, true, false),
            ] {
                let mut gesture = WidgetTester::touch(3);
                tester.down(&mut gesture, start);
                // We have to execute the drag in two frames because the first update will
                // just set the start position.
                tester.move_to(&mut gesture, start + Offset::new(dx, 0.0), true);
                tester.move_to(&mut gesture, start + Offset::new(2.0 * dx, 0.0), true);
                assert_eq!(value.get(), before_up, "{dx}");
                tester.up(&gesture);
                assert_eq!(value.get(), after_up, "{dx}");
                tester.pump(Duration::ZERO);
            }
        });
    }

    #[test]
    fn a_right_to_left_switch_turns_on_toward_the_left() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Rtl,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            drag(&mut tester, 30.0);
            assert!(!value.get());
            drag(&mut tester, -30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, -30.0);
            assert!(value.get());
            tester.pump(Duration::ZERO);
            drag(&mut tester, 30.0);
            assert!(!value.get());
        });
    }

    #[test]
    fn an_m2_switch_has_its_default_colors_when_enabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m2,
                TextDirection::Rtl,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x52000000, M2_TRACK));
            assert_eq!(
                colors(&painted),
                m2_sequence(0x52000000, &[], c(Colors::GREY.shade50())),
                "Inactive enabled switch should match these colors"
            );
            drag(&mut tester, -30.0);
            tester.pump(Duration::ZERO);
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x802196f3, M2_TRACK));
            assert_eq!(
                colors(&painted),
                m2_sequence(0x802196f3, &[], 0xff2196f3),
                "Active enabled switch should match these colors"
            );
        });
    }

    /// Dart's 'Material3 - Switch has default colors when enabled', and the same case through
    /// `Switch.adaptive` on the test's Android platform.
    #[test]
    fn an_m3_switch_has_its_default_colors_when_enabled() {
        with_main_thread_stack(|| {
            for adaptive in [false, true] {
                let mut tester = tester();
                let colors_ = colors_of(&tester.cell.borrow(), m3);
                let value = shared(false);
                pump_host(
                    &mut tester,
                    m3,
                    TextDirection::Rtl,
                    &value,
                    true,
                    move |value, on_changed| {
                        let switch = if adaptive {
                            Switch::adaptive(value, on_changed)
                        } else {
                            Switch::new(value, on_changed)
                        };
                        switch.drag_start_behavior(DragStartBehavior::Down)
                    },
                );
                assert_eq!(
                    drawn(&tester),
                    [
                        Drawn::Fill(c(colors_.surface_container_highest()), M3_TRACK),
                        Drawn::Stroke(c(colors_.outline()), 2.0, M3_OUTLINE),
                        Drawn::Fill(c(colors_.outline()), ltrb(32.0, 16.0, 48.0, 32.0)),
                    ],
                    "Inactive enabled switch should match these colors"
                );
                drag(&mut tester, -30.0);
                tester.pump(Duration::ZERO);
                let painted = drawn(&tester);
                assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
                assert_eq!(
                    colors(&painted),
                    [c(colors_.primary), c(colors_.on_primary)],
                    "Active enabled switch should match these colors"
                );
            }
        });
    }

    #[test]
    fn an_m2_switch_has_its_default_colors_when_disabled() {
        with_main_thread_stack(|| {
            for value in [false, true] {
                let mut tester = tester();
                let home = Directionality::new(
                    TextDirection::Rtl,
                    Material::new().child(Center::new().child(Switch::new(value, None))),
                )
                .into_widget();
                pump_app(&mut tester, m2, move |_app| home);
                let painted = drawn(&tester);
                assert_eq!(painted[0], Drawn::Fill(c(Colors::BLACK12), M2_TRACK));
                assert_eq!(
                    colors(&painted),
                    m2_sequence(c(Colors::BLACK12), &[], c(Colors::GREY.shade400())),
                    "value {value}"
                );
            }
        });
    }

    #[test]
    fn an_m3_switch_has_its_default_colors_when_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            let home = in_material(TextDirection::Rtl, Switch::new(false, None).into_widget());
            pump_app(&mut tester, m3, move |_app| home);
            let painted = drawn(&tester);
            assert_eq!(
                painted,
                [
                    Drawn::Fill(
                        c(with_opacity(colors_.surface_container_highest(), 0.12)),
                        M3_TRACK
                    ),
                    Drawn::Stroke(c(with_opacity(colors_.on_surface, 0.12)), 2.0, M3_OUTLINE),
                    Drawn::Fill(
                        c(Color::alpha_blend(
                            with_opacity(colors_.on_surface, 0.38),
                            colors_.surface
                        )),
                        painted[2].clone().rect(),
                    ),
                ],
                "Inactive disabled switch should match these colors"
            );

            let mut tester = crate::tooltip::tests::tester();
            let home = in_material(TextDirection::Rtl, Switch::new(true, None).into_widget());
            pump_app(&mut tester, m3, move |_app| home);
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(with_opacity(colors_.on_surface, 0.12)), M3_TRACK)
            );
            assert_eq!(
                colors(&painted),
                [
                    c(with_opacity(colors_.on_surface, 0.12)),
                    c(colors_.surface)
                ],
                "Active disabled switch should match these colors"
            );
        });
    }

    /// The reaction circles drawn: Dart's `paints..circle()`, told from an `rrect` by being
    /// a square of the splash radius's diameter centred on the thumb's row.
    pub(crate) fn circles(painted: &[Drawn], radius: f64) -> Vec<u32> {
        painted
            .iter()
            .filter_map(|drawn| match drawn {
                Drawn::Fill(color, rect)
                    if (rect.width() - 2.0 * radius).abs() < 1e-6
                        && (rect.height() - 2.0 * radius).abs() < 1e-6 =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_default_overlay_resolves_the_focused_then_the_hovered_state() {
        with_main_thread_stack(|| {
            for (theme, use_material3) in [(m2 as fn(&App) -> ThemeData, false), (m3, true)] {
                let mut tester = tester();
                let data = theme(&tester.cell.borrow());
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let node = focus_node(&tester);
                let home = Scaffold::new()
                    .body(Switch::new(true, ignore()).focus_node(node))
                    .into_widget();
                pump_app(&mut tester, theme, move |_app| home);

                // Focused.
                node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump_and_settle();
                let (focused, hovered) = if use_material3 {
                    (
                        with_opacity(data.color_scheme.primary, 0.1),
                        with_opacity(data.color_scheme.primary, 0.08),
                    )
                } else {
                    (data.focus_color, data.hover_color)
                };
                let radius = if use_material3 {
                    20.0
                } else {
                    K_RADIAL_REACTION_RADIUS
                };
                assert_eq!(
                    circles(&drawn(&tester), radius),
                    [c(focused)],
                    "M3 {use_material3}"
                );

                // On both hovered and focused, the overlay color should show hovered overlay color.
                hover(&mut tester);
                tester.pump_and_settle();
                assert_eq!(
                    circles(&drawn(&tester), radius),
                    [c(hovered)],
                    "M3 {use_material3}"
                );
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn an_m2_switch_takes_the_colors_it_is_given() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m2,
                TextDirection::Rtl,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed)
                        .drag_start_behavior(DragStartBehavior::Down)
                        .active_color(Colors::RED.shade500())
                        .active_track_color(Colors::GREEN.shade500())
                        .inactive_thumb_color(Colors::YELLOW.shade500())
                        .inactive_track_color(Colors::BLUE.shade500())
                },
            );
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(Colors::BLUE.shade500()), M2_TRACK)
            );
            assert_eq!(
                colors(&painted),
                m2_sequence(
                    c(Colors::BLUE.shade500()),
                    &[],
                    c(Colors::YELLOW.shade500())
                )
            );
            drag(&mut tester, -30.0);
            tester.pump(Duration::ZERO);
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(Colors::GREEN.shade500()), M2_TRACK)
            );
            assert_eq!(
                colors(&painted),
                m2_sequence(c(Colors::GREEN.shade500()), &[], c(Colors::RED.shade500()))
            );
        });
    }

    #[test]
    #[allow(deprecated)]
    fn an_m3_switch_takes_the_colors_it_is_given() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Rtl,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed)
                        .drag_start_behavior(DragStartBehavior::Down)
                        .active_color(Colors::RED.shade500())
                        .active_track_color(Colors::GREEN.shade500())
                        .inactive_thumb_color(Colors::YELLOW.shade500())
                        .inactive_track_color(Colors::BLUE.shade500())
                },
            );
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(Colors::BLUE.shade500()), M3_TRACK)
            );
            assert!(
                matches!(painted[1], Drawn::Stroke(color, ..) if color == c(colors_.outline()))
            );
            assert_eq!(painted[2].color(), Some(c(Colors::YELLOW.shade500())));
            drag(&mut tester, -30.0);
            tester.pump(Duration::ZERO);
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(Colors::GREEN.shade500()), M3_TRACK)
            );
            assert_eq!(
                colors(&painted),
                [c(Colors::GREEN.shade500()), c(Colors::RED.shade500())]
            );
        });
    }

    #[test]
    fn a_drag_ends_after_its_animation_completes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = shared(false);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed).drag_start_behavior(DragStartBehavior::Down)
                },
            );
            assert!(!value.get());
            let switch_rect = switch_rect(&tester);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, switch_rect.center_left());
            tester.pump(Duration::ZERO);
            let to = gesture.position + Offset::new(switch_rect.width(), 0.0);
            tester.move_to(&mut gesture, to, true);
            tester.pump(Duration::ZERO);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(200));
            assert!(value.get());
            assert!(!has_running_animations(&tester));
        });
    }

    #[test]
    fn a_vetoed_drag_animates_the_thumb_back() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(false));
            let home = {
                let value = Rc::clone(&value);
                in_material(
                    TextDirection::Ltr,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let shared = Rc::clone(&value);
                        let on_changed: ValueChanged<bool> =
                            Rc::new(move |app: &mut App, new_value: bool| {
                                let shared = Rc::clone(&shared);
                                set_state(app, &mut move |_app| {
                                    shared.set(shared.get() || new_value)
                                });
                            });
                        Switch::new(value.get(), Some(on_changed))
                            .drag_start_behavior(DragStartBehavior::Down)
                            .into_widget()
                    })
                    .into_widget(),
                )
            };
            pump_app(&mut tester, m3, move |_app| home);
            let slop = waft_gestures::K_TOUCH_SLOP;

            // Move a little to the right, not past the middle.
            let start = center(&tester);
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, start);
            tester.move_to(&mut gesture, start + Offset::new(slop + 0.1, 0.0), true);
            tester.pump(Duration::ZERO);
            let at = gesture.position + Offset::new(-slop + 5.1, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump(Duration::ZERO);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);
            assert!(!value.get());
            assert!(position(&tester) < 0.5);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert!(!value.get());
            assert_eq!(position(&tester), 0.0);

            // Move past the middle.
            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, start);
            tester.move_to(&mut gesture, start + Offset::new(slop + 0.1, 0.0), true);
            tester.pump(Duration::ZERO);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);
            assert!(value.get());
            assert!(position(&tester) > 0.5);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert!(value.get());
            assert_eq!(position(&tester), 1.0);

            // Now move back to the left, the revert animation should play.
            let mut gesture = WidgetTester::touch(3);
            tester.down(&mut gesture, start);
            tester.move_to(&mut gesture, start + Offset::new(-slop - 0.1, 0.0), true);
            tester.pump(Duration::ZERO);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);
            assert!(value.get());
            assert!(position(&tester) < 0.5);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle();
            assert!(value.get());
            assert_eq!(position(&tester), 1.0);
        });
    }

    #[test]
    #[allow(deprecated)]
    fn an_adaptive_switch_is_a_material_switch_that_toggles_on_every_platform() {
        with_main_thread_stack(|| {
            const ACTIVE_TRACK_COLOR: Color = Color::new(0xffff1200);
            const INACTIVE_TRACK_COLOR: Color = Color::new(0xffff12ff);
            const THUMB_COLOR: Color = Color::new(0xffffff00);
            const FOCUS_COLOR: Color = Color::new(0xff00ff00);
            for platform in ALL_PLATFORMS {
                let mut tester = tester_on(platform);
                let value = shared(false);
                let home = host(&value, true, |value, on_changed| {
                    Switch::adaptive(value, on_changed)
                        .active_color(ACTIVE_TRACK_COLOR)
                        .inactive_track_color(INACTIVE_TRACK_COLOR)
                        .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                            THUMB_COLOR,
                        )))
                        .focus_color(FOCUS_COLOR)
                });
                let home = Material::new()
                    .child(Center::new().child(home))
                    .into_widget();
                pump_app(&mut tester, adaptive_on(platform), move |_app| home);
                assert_eq!(tester.find_by_type::<Switch>().len(), 1, "on {platform:?}");
                assert!(
                    tester
                        .find_by_type::<waft_cupertino::CupertinoSwitch>()
                        .is_empty()
                );
                tester.widget(switch_element(&tester), |switch: &Switch| {
                    assert_eq!(switch.active_color, Some(ACTIVE_TRACK_COLOR));
                    assert_eq!(switch.inactive_track_color, Some(INACTIVE_TRACK_COLOR));
                    assert_eq!(
                        switch
                            .thumb_color
                            .as_ref()
                            .and_then(|color| color.resolve(&WidgetStates::new())),
                        Some(THUMB_COLOR)
                    );
                    assert_eq!(switch.focus_color, Some(FOCUS_COLOR));
                });
                assert!(!value.get(), "on {platform:?}");
                tap(&mut tester);
                assert!(value.get(), "on {platform:?}");
            }
        });
    }

    /// Dart's `buildAdaptiveSwitch`.
    #[derive(Clone, Default)]
    struct AdaptiveSwitch {
        disabled: bool,
        off: bool,
        autofocus: bool,
        focus_node: Option<AnyFocusNode>,
        focus_color: Option<Color>,
        overall_switch_theme_data: Option<SwitchThemeData>,
        switch_theme_data: Option<SwitchThemeData>,
        switch_theme_adaptation: bool,
    }

    /// The test's `_SwitchThemeAdaptation`.
    #[derive(Debug)]
    struct TestSwitchThemeAdaptation;

    impl Adaptation<SwitchThemeData> for TestSwitchThemeAdaptation {
        fn adapt(&self, theme: &ThemeData, default_value: SwitchThemeData) -> SwitchThemeData {
            match theme.platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Windows
                | TargetPlatform::Linux => default_value,
                TargetPlatform::IOS | TargetPlatform::MacOS => SwitchThemeData::new()
                    .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                        Colors::LIGHT_GREEN.primary(),
                    )))
                    .track_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                        Colors::DEEP_PURPLE.primary(),
                    ))),
            }
        }
    }

    impl AdaptiveSwitch {
        fn pump(self, tester: &mut WidgetTester, platform: TargetPlatform) {
            let mut adaptive_switch =
                Switch::adaptive(!self.off, if self.disabled { None } else { ignore() })
                    .autofocus(self.autofocus);
            if let Some(focus_node) = self.focus_node {
                adaptive_switch = adaptive_switch.focus_node(focus_node);
            }
            if let Some(focus_color) = self.focus_color {
                adaptive_switch = adaptive_switch.focus_color(focus_color);
            }
            let child = match self.switch_theme_data {
                None => adaptive_switch.into_widget(),
                Some(data) => SwitchTheme::new(data, adaptive_switch).into_widget(),
            };
            let home = Material::new()
                .child(Center::new().child(child))
                .into_widget();
            let (overall, adaptation) =
                (self.overall_switch_theme_data, self.switch_theme_adaptation);
            pump_app(
                tester,
                move |app| {
                    let mut builder = ThemeData::builder().platform(platform);
                    if let Some(overall) = overall {
                        builder = builder.switch_theme(overall);
                    }
                    if adaptation {
                        builder = builder
                            .adaptations(vec![AdaptationRef::new(TestSwitchThemeAdaptation)]);
                    }
                    builder.build(app)
                },
                move |_app| home,
            );
        }
    }

    #[test]
    fn an_adaptive_cupertino_switch_shows_the_default_mouse_cursor() {
        with_main_thread_stack(|| {
            let platform_cursor: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            // One tester for both platforms, as in Dart: on macOS the size is read on the first
            // frame of the theme animation from iOS, which still has iOS's padded tap target.
            let mut tester = tester_on(TargetPlatform::IOS);
            for platform in CUPERTINO_PLATFORMS {
                AdaptiveSwitch {
                    off: true,
                    ..AdaptiveSwitch::default()
                }
                .pump(&mut tester, platform);
                assert_eq!(switch_rect(&tester).size(), Size::new(60.0, 48.0));
                let mut mouse = WidgetTester::mouse(1, 0);
                let at = center(&tester);
                tester.add_pointer(&mut mouse, at);
                tester.pump(Duration::ZERO);
                tester.move_to(&mut mouse, at, false);
                assert_eq!(Some(tester.active_cursor(1)), Some(platform_cursor.clone()));

                AdaptiveSwitch::default().pump(&mut tester, platform);
                assert_eq!(Some(tester.active_cursor(1)), Some(platform_cursor.clone()));

                // Test disabled switch.
                AdaptiveSwitch {
                    disabled: true,
                    off: true,
                    ..AdaptiveSwitch::default()
                }
                .pump(&mut tester, platform);
                assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
                tester.send(&mouse, PointerChange::Remove);
                tester.pump(Duration::ZERO);
            }
        });
    }

    #[test]
    fn an_adaptive_cupertino_switch_has_its_default_thumb_and_track_colors_and_size() {
        with_main_thread_stack(|| {
            const THUMB_COLOR: u32 = 0xffffffff;
            const INACTIVE_TRACK_COLOR: u32 = 0x28787880; // Default inactive track color.
            const ACTIVE_TRACK_COLOR: u32 = 0xff34c759; // Default active track color.
            for platform in CUPERTINO_PLATFORMS {
                // Switches have same sizes on both platform but they are more compact on macOS.
                let (track, inactive_thumb, active_thumb) = if platform == TargetPlatform::IOS {
                    (
                        ltrb(4.5, 8.5, 55.5, 39.5),
                        ltrb(6.0, 10.0, 34.0, 38.0),
                        ltrb(26.0, 10.0, 54.0, 38.0),
                    )
                } else {
                    (
                        ltrb(4.5, 4.5, 55.5, 35.5),
                        ltrb(6.0, 6.0, 34.0, 34.0),
                        ltrb(26.0, 6.0, 54.0, 34.0),
                    )
                };
                for (disabled, off, expected_opacity) in [
                    (false, true, 1.0),
                    (false, false, 1.0),
                    (true, true, 0.5),
                    (true, false, 0.5),
                ] {
                    let mut tester = tester_on(platform);
                    AdaptiveSwitch {
                        disabled,
                        off,
                        ..AdaptiveSwitch::default()
                    }
                    .pump(&mut tester, platform);
                    tester.pump(Duration::ZERO);
                    let (track_color, thumb) = if off {
                        (INACTIVE_TRACK_COLOR, inactive_thumb)
                    } else {
                        (ACTIVE_TRACK_COLOR, active_thumb)
                    };
                    assert_eq!(
                        drawn(&tester),
                        [
                            Drawn::Fill(track_color, track),
                            Drawn::Clip,
                            Drawn::Shadow(CUPERTINO_SHADOWS_AND_BORDER[0]),
                            Drawn::Shadow(CUPERTINO_SHADOWS_AND_BORDER[1]),
                            Drawn::Fill(CUPERTINO_SHADOWS_AND_BORDER[2], thumb.inflate(0.5)),
                            Drawn::Fill(THUMB_COLOR, thumb),
                        ],
                        "{platform:?}, disabled {disabled}, off {off}"
                    );
                    assert_eq!(opacity(&tester), expected_opacity);
                }
            }
        });
    }

    #[test]
    fn an_adaptive_cupertino_switch_ignores_the_switch_themes() {
        with_main_thread_stack(|| {
            const DEFAULT_THUMB_COLOR: u32 = 0xffffffff;
            const DEFAULT_INACTIVE_TRACK_COLOR: u32 = 0x28787880;
            const DEFAULT_ACTIVE_TRACK_COLOR: u32 = 0xff34c759;
            let updated_thumb_color = Colors::RED.primary();
            let updated_track_color = Colors::GREEN.primary();
            let theme = SwitchThemeData::new()
                .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                    updated_thumb_color,
                )))
                .track_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                    updated_track_color,
                )));
            // Dart's two cases: the theme in `ThemeData.switchTheme`, and in a `SwitchTheme`.
            for overall in [true, false] {
                let with_theme = |switch: AdaptiveSwitch| {
                    if overall {
                        AdaptiveSwitch {
                            overall_switch_theme_data: Some(theme.clone()),
                            ..switch
                        }
                    } else {
                        AdaptiveSwitch {
                            switch_theme_data: Some(theme.clone()),
                            ..switch
                        }
                    }
                };
                for platform in CUPERTINO_PLATFORMS {
                    for (disabled, off, track) in [
                        (false, false, DEFAULT_ACTIVE_TRACK_COLOR),
                        (false, true, DEFAULT_INACTIVE_TRACK_COLOR),
                        (true, true, DEFAULT_INACTIVE_TRACK_COLOR),
                    ] {
                        let mut tester = tester_on(platform);
                        with_theme(AdaptiveSwitch {
                            disabled,
                            off,
                            ..AdaptiveSwitch::default()
                        })
                        .pump(&mut tester, platform);
                        assert_eq!(
                            colors(&drawn(&tester)),
                            [
                                &[track][..],
                                &CUPERTINO_SHADOWS_AND_BORDER[..],
                                &[DEFAULT_THUMB_COLOR][..]
                            ]
                            .concat(),
                            "overall {overall}, {platform:?}, disabled {disabled}, off {off}"
                        );
                    }
                }

                let mut tester = tester_on(TargetPlatform::Android);
                with_theme(AdaptiveSwitch::default()).pump(&mut tester, TargetPlatform::Android);
                assert_eq!(
                    colors(&drawn(&tester)),
                    [c(updated_track_color), c(updated_thumb_color)],
                    "Switch.adaptive is affected by SwitchTheme on other platforms"
                );
            }
        });
    }

    #[test]
    fn an_adaptation_overrides_the_adaptive_switch_theme_on_ios_and_macos_only() {
        with_main_thread_stack(|| {
            let switch_theme_data = SwitchThemeData::new()
                .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                    Colors::YELLOW.primary(),
                )))
                .track_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                    Colors::BROWN.primary(),
                )));
            for platform in ALL_PLATFORMS {
                let mut tester = tester_on(platform);
                AdaptiveSwitch {
                    switch_theme_data: Some(switch_theme_data.clone()),
                    switch_theme_adaptation: true,
                    ..AdaptiveSwitch::default()
                }
                .pump(&mut tester, platform);
                let painted = colors(&drawn(&tester));
                if CUPERTINO_PLATFORMS.contains(&platform) {
                    assert_eq!(
                        painted,
                        [
                            &[c(Colors::DEEP_PURPLE.primary())][..],
                            &CUPERTINO_SHADOWS_AND_BORDER[..],
                            &[c(Colors::LIGHT_GREEN.primary())][..]
                        ]
                        .concat(),
                        "{platform:?}"
                    );
                } else {
                    // Other platforms should not be affected by the adaptive switch theme.
                    assert_eq!(
                        painted,
                        [c(Colors::BROWN.primary()), c(Colors::YELLOW.primary())],
                        "{platform:?}"
                    );
                }
            }
        });
    }

    #[test]
    fn an_adaptive_cupertino_switch_has_its_default_focus_color() {
        with_main_thread_stack(|| {
            for (focus_color, ring) in [
                (None, 0xcc6ef28f),
                (Some(Colors::RED.primary()), c(Colors::RED.primary())),
            ] {
                let mut tester = tester_on(TargetPlatform::MacOS);
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let node = focus_node(&tester);
                AdaptiveSwitch {
                    autofocus: true,
                    focus_node: Some(node),
                    focus_color,
                    ..AdaptiveSwitch::default()
                }
                .pump(&mut tester, TargetPlatform::MacOS);
                tester.pump_and_settle();
                assert!(node.has_primary_focus(&tester.cell.borrow()));
                let painted = drawn(&tester);
                assert_eq!(painted[0].color(), Some(0xff34c759)); // Track color
                assert!(
                    matches!(painted[1], Drawn::Stroke(color, width, _) if color == ring && width == 3.5),
                    "{:?}",
                    painted[1]
                ); // Focused outline
                assert_eq!(painted.last().and_then(Drawn::color), Some(0xffffffff)); // Thumb color
            }
        });
    }

    #[test]
    fn an_m3_switch_is_focusable_and_has_its_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let value = shared(true);
            let orange = Colors::ORANGE.shade500();
            for enabled in [true, true, false] {
                if !enabled {
                    value.set(false);
                }
                pump_host(
                    &mut tester,
                    m3,
                    TextDirection::Ltr,
                    &value,
                    enabled,
                    move |value, on_changed| {
                        Switch::new(value, on_changed)
                            .focus_color(orange)
                            .autofocus(true)
                            .focus_node(node)
                    },
                );
                tester.pump_and_settle();
                let painted = drawn(&tester);
                if !enabled {
                    // Check what happens when disabled: inactive disabled switch.
                    assert!(!node.has_primary_focus(&tester.cell.borrow()));
                    assert_eq!(
                        colors(&painted),
                        [
                            c(with_opacity(colors_.surface_container_highest(), 0.12)),
                            c(with_opacity(colors_.on_surface, 0.12)),
                            c(Color::alpha_blend(
                                with_opacity(colors_.on_surface, 0.38),
                                colors_.surface
                            )),
                        ]
                    );
                    break;
                }
                assert!(node.has_primary_focus(&tester.cell.borrow()));
                if value.get() {
                    // active, enabled switch
                    assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
                    assert_eq!(circles(&painted, 20.0), [c(orange)]);
                    // Check the false value: inactive enabled switch
                    value.set(false);
                } else {
                    assert_eq!(
                        painted[0],
                        Drawn::Fill(c(colors_.surface_container_highest()), M3_TRACK)
                    );
                    assert_eq!(
                        painted[1],
                        Drawn::Stroke(c(colors_.outline()), 2.0, M3_OUTLINE)
                    );
                    assert_eq!(circles(&painted, 20.0), [c(orange)]);
                }
            }
        });
    }

    #[test]
    fn the_splash_radius_sizes_the_focus_circle() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 30.0;
            pump_switch(
                &mut tester,
                m3,
                Switch::new(true, ignore())
                    .focus_color(Colors::ORANGE.shade500())
                    .autofocus(true)
                    .splash_radius(SPLASH_RADIUS),
            );
            tester.pump_and_settle();
            assert_eq!(
                circles(&drawn(&tester), SPLASH_RADIUS),
                [c(Colors::ORANGE.shade500())]
            );
        });
    }

    #[test]
    fn an_m2_switch_can_be_hovered_and_has_its_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = shared(true);
            let orange = Colors::ORANGE.shade500();
            let make = move |value, on_changed| Switch::new(value, on_changed).hover_color(orange);
            pump_host(&mut tester, m2, TextDirection::Ltr, &value, true, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x802196f3, M2_TRACK));
            assert_eq!(colors(&painted), m2_sequence(0x802196f3, &[], 0xff2196f3));

            // Start hovering
            hover(&mut tester);
            pump_host(&mut tester, m2, TextDirection::Ltr, &value, true, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x802196f3, M2_TRACK));
            assert_eq!(
                colors(&painted),
                m2_sequence(0x802196f3, &[c(orange)], 0xff2196f3)
            );

            // Check what happens when disabled.
            pump_host(&mut tester, m2, TextDirection::Ltr, &value, false, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x1f000000, M2_TRACK));
            assert_eq!(colors(&painted), m2_sequence(0x1f000000, &[], 0xffbdbdbd));
        });
    }

    #[test]
    fn an_m3_switch_can_be_hovered_and_has_its_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = shared(true);
            let orange = Colors::ORANGE.shade500();
            let make = move |value, on_changed| Switch::new(value, on_changed).hover_color(orange);

            // active enabled switch
            pump_host(&mut tester, m3, TextDirection::Ltr, &value, true, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
            assert_eq!(
                colors(&painted),
                [c(colors_.primary), c(colors_.on_primary)]
            );

            // Start hovering
            hover(&mut tester);
            pump_host(&mut tester, m3, TextDirection::Ltr, &value, true, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
            assert_eq!(circles(&painted, 20.0), [c(orange)]);

            // Check what happens for disabled active switch
            pump_host(&mut tester, m3, TextDirection::Ltr, &value, false, make);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(
                painted[0],
                Drawn::Fill(c(with_opacity(colors_.on_surface, 0.12)), M3_TRACK)
            );
            assert_eq!(
                painted.last().and_then(Drawn::color),
                Some(c(colors_.surface))
            );
        });
    }

    #[test]
    fn the_enter_and_space_keys_toggle_the_switch() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = shared(true);
            pump_host(
                &mut tester,
                m3,
                TextDirection::Ltr,
                &value,
                true,
                |value, on_changed| {
                    Switch::new(value, on_changed)
                        .focus_color(Colors::ORANGE.shade500())
                        .autofocus(true)
                },
            );
            tester.pump_and_settle();
            tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            // On web, switches don't respond to the enter key.
            assert_eq!(value.get(), K_IS_WEB);
            tester.send_key_event(PhysicalKeyboardKey::ENTER, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            assert!(value.get());
            tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
            tester.pump_and_settle();
            assert!(!value.get());
            tester.send_key_event(PhysicalKeyboardKey::SPACE, LogicalKeyboardKey::SPACE);
            tester.pump_and_settle();
            assert!(value.get());
        });
    }

    #[test]
    fn a_hovered_switch_shows_its_mouse_cursor() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pump = |tester: &mut WidgetTester, switch: Switch| {
                let home = Scaffold::new()
                    .body(
                        Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                            Material::new().child(
                                MouseRegion::new()
                                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                                    .child(switch),
                            ),
                        ),
                    )
                    .into_widget();
                pump_app(tester, m3, move |_app| home);
            };
            // Test Switch.adaptive() constructor
            pump(
                &mut tester,
                Switch::adaptive(true, ignore()).mouse_cursor(SystemMouseCursors::TEXT),
            );
            let mut mouse = WidgetTester::mouse(1, 0);
            let at = center(&tester);
            tester.add_pointer(&mut mouse, at);
            tester.pump(Duration::ZERO);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test Switch() constructor
            pump(
                &mut tester,
                Switch::new(true, ignore()).mouse_cursor(SystemMouseCursors::TEXT),
            );
            let at = center(&tester);
            tester.move_to(&mut mouse, at, false);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            pump(&mut tester, Switch::new(true, ignore()));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::CLICK.into());

            // Test default cursor when disabled
            pump(&mut tester, Switch::new(true, None));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());

            tester.pump_and_settle();
        });
    }

    #[test]
    fn an_m3_thumb_color_resolves_in_the_selected_and_disabled_states() {
        with_main_thread_stack(|| {
            const ACTIVE_ENABLED: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED: Color = Color::new(0xFF000004);
            let thumb_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                Some(if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        ACTIVE_DISABLED
                    } else {
                        INACTIVE_DISABLED
                    }
                } else if states.contains(&WidgetState::Selected) {
                    ACTIVE_ENABLED
                } else {
                    INACTIVE_ENABLED
                })
            });
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            for (enabled, active, track, thumb) in [
                (
                    false,
                    false,
                    with_opacity(colors_.surface_container_highest(), 0.12),
                    INACTIVE_DISABLED,
                ),
                (
                    false,
                    true,
                    with_opacity(colors_.on_surface, 0.12),
                    ACTIVE_DISABLED,
                ),
                (
                    true,
                    false,
                    colors_.surface_container_highest(),
                    INACTIVE_ENABLED,
                ),
                (true, true, colors_.primary, ACTIVE_ENABLED),
            ] {
                let home = in_material(
                    TextDirection::Rtl,
                    Switch::new(active, if enabled { ignore() } else { None })
                        .thumb_color(Rc::clone(&thumb_color))
                        .into_widget(),
                );
                pump_app(&mut tester, m3, move |_app| home);
                tester.pump_and_settle();
                let painted = drawn(&tester);
                assert_eq!(
                    painted[0],
                    Drawn::Fill(c(track), M3_TRACK),
                    "{enabled} {active}"
                );
                assert_eq!(painted.last().and_then(Drawn::color), Some(c(thumb)));
            }
        });
    }

    #[test]
    fn an_m3_thumb_color_resolves_in_the_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            const HOVERED: Color = Color::new(0xFF000001);
            const FOCUSED: Color = Color::new(0xFF000002);
            let mut tester = tester();
            let colors_ = colors_of(&tester.cell.borrow(), m3);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let home = in_material(
                TextDirection::Rtl,
                Switch::new(true, ignore())
                    .focus_node(node)
                    .autofocus(true)
                    .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            Some(if states.contains(&WidgetState::Hovered) {
                                HOVERED
                            } else if states.contains(&WidgetState::Focused) {
                                FOCUSED
                            } else {
                                Colors::TRANSPARENT
                            })
                        },
                    ))
                    .into_widget(),
            );
            pump_app(&mut tester, m3, move |_app| home);
            tester.pump_and_settle();
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
            assert_eq!(
                circles(&painted, 20.0),
                [c(with_opacity(colors_.primary, 0.1))]
            );
            assert_eq!(painted.last().and_then(Drawn::color), Some(c(FOCUSED)));

            // Start hovering
            hover(&mut tester);
            tester.pump_and_settle();
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(c(colors_.primary), M3_TRACK));
            assert_eq!(
                circles(&painted, 20.0),
                [c(with_opacity(colors_.primary, 0.08))]
            );
            assert_eq!(painted.last().and_then(Drawn::color), Some(c(HOVERED)));
        });
    }

    #[test]
    fn the_overlay_color_resolves_in_the_pressed_focused_and_hovered_states() {
        with_main_thread_stack(|| {
            const ACTIVE_THUMB: Color = Color::new(0xFF000000);
            const INACTIVE_THUMB: Color = Color::new(0xFF000010);
            const ACTIVE_PRESSED_OVERLAY: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY: Color = Color::new(0xFF000003);
            const FOCUS_OVERLAY: Color = Color::new(0xFF000004);
            const HOVER: Color = Color::new(0xFF000005);
            const FOCUS: Color = Color::new(0xFF000006);
            const SPLASH_RADIUS: f64 = 24.0;
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let pump =
                |tester: &mut WidgetTester, active: bool, focused: bool, use_overlay: bool| {
                    let mut switch = Switch::new(active, ignore())
                        .focus_node(node)
                        .autofocus(focused)
                        .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                Some(if states.contains(&WidgetState::Selected) {
                                    ACTIVE_THUMB
                                } else {
                                    INACTIVE_THUMB
                                })
                            },
                        ))
                        .hover_color(HOVER)
                        .focus_color(FOCUS)
                        .splash_radius(SPLASH_RADIUS);
                    if use_overlay {
                        switch = switch.overlay_color(
                            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                                if states.contains(&WidgetState::Pressed) {
                                    if states.contains(&WidgetState::Selected) {
                                        return Some(ACTIVE_PRESSED_OVERLAY);
                                    }
                                    return Some(INACTIVE_PRESSED_OVERLAY);
                                }
                                if states.contains(&WidgetState::Hovered) {
                                    return Some(HOVER_OVERLAY);
                                }
                                if states.contains(&WidgetState::Focused) {
                                    return Some(FOCUS_OVERLAY);
                                }
                                None
                            }),
                        );
                    }
                    let home = Scaffold::new().body(switch).into_widget();
                    pump_app(tester, m3, move |_app| home);
                };

            for (active, use_overlay, expected) in [
                (
                    false,
                    false,
                    INACTIVE_THUMB.with_alpha(K_RADIAL_REACTION_ALPHA),
                ),
                (
                    true,
                    false,
                    ACTIVE_THUMB.with_alpha(K_RADIAL_REACTION_ALPHA),
                ),
                (false, true, INACTIVE_PRESSED_OVERLAY),
                (true, true, ACTIVE_PRESSED_OVERLAY),
            ] {
                pump(&mut tester, active, false, use_overlay);
                let touch = press(&mut tester);
                tester.pump_and_settle();
                assert_eq!(
                    circles(&drawn(&tester), SPLASH_RADIUS),
                    [c(expected)],
                    "active {active}, overlay {use_overlay}"
                );
                tester.up(&touch);
                tester.pump_and_settle();
            }

            pump(&mut tester, false, true, true);
            tester.pump_and_settle();
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            assert_eq!(circles(&drawn(&tester), SPLASH_RADIUS), [c(FOCUS_OVERLAY)]);

            // Start hovering
            hover(&mut tester);
            tester.pump_and_settle();
            assert_eq!(circles(&drawn(&tester), SPLASH_RADIUS), [c(HOVER_OVERLAY)]);
        });
    }

    #[test]
    fn an_m3_switch_animates_for_300_milliseconds_and_its_thumb_bounces() {
        with_main_thread_stack(|| {
            for bounce in [false, true] {
                let mut tester = tester();
                let value = shared(false);
                let direction = if bounce {
                    TextDirection::Ltr
                } else {
                    TextDirection::Rtl
                };
                pump_host(&mut tester, m3, direction, &value, true, Switch::new);
                assert!(!value.get());
                let switch_rect = switch_rect(&tester);
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, switch_rect.center_left());
                tester.pump(Duration::ZERO);
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                if bounce {
                    // The value on y axis is greater than 1 when t > 0.375
                    // 300 * 0.375 = 112.5
                    tester.pump(Duration::from_millis(113));
                    assert!(has_running_animations(&tester));
                    assert!(position(&tester) > 1.0, "{}", position(&tester));
                } else {
                    tester.pump(Duration::from_millis(200)); // M2 animation duration
                    assert!(has_running_animations(&tester));
                    tester.pump(Duration::from_millis(101));
                    assert!(!has_running_animations(&tester));
                }
            }
        });
    }

    #[test]
    fn an_m3_thumb_shows_its_pressed_color() {
        with_main_thread_stack(|| {
            for platform in MOBILE.into_iter().chain([TargetPlatform::IOS]) {
                let colors_ = colors_of(&tester_on(platform).cell.borrow(), m3);
                for (enabled, value, track, outline, thumb) in [
                    (
                        true,
                        true,
                        colors_.primary,
                        None,
                        Drawn::Fill(c(colors_.primary_container()), ltrb(26.0, 10.0, 54.0, 38.0)),
                    ),
                    (
                        true,
                        false,
                        colors_.surface_container_highest(),
                        Some(colors_.outline()),
                        Drawn::Fill(c(colors_.on_surface_variant()), ltrb(6.0, 10.0, 34.0, 38.0)),
                    ),
                    (
                        false,
                        true,
                        with_opacity(colors_.on_surface, 0.12),
                        None,
                        Drawn::Fill(c(colors_.surface), ltrb(28.0, 12.0, 52.0, 36.0)),
                    ),
                    (
                        false,
                        false,
                        with_opacity(colors_.surface_container_highest(), 0.12),
                        Some(with_opacity(colors_.on_surface, 0.12)),
                        Drawn::Fill(
                            c(Color::alpha_blend(
                                with_opacity(colors_.on_surface, 0.38),
                                colors_.surface,
                            )),
                            ltrb(12.0, 16.0, 28.0, 32.0),
                        ),
                    ),
                ] {
                    let mut tester = tester_on(platform);
                    let shared = shared(value);
                    pump_host(
                        &mut tester,
                        m3,
                        TextDirection::Ltr,
                        &shared,
                        enabled,
                        Switch::new,
                    );
                    let _touch = press(&mut tester);
                    tester.pump_and_settle();
                    let painted = drawn(&tester);
                    let case = format!("{platform:?}, enabled {enabled}, value {value}");
                    assert_eq!(painted[0].color(), Some(c(track)), "{case}");
                    match outline {
                        Some(outline) => {
                            assert!(
                                matches!(painted[1], Drawn::Stroke(color, ..) if color == c(outline)),
                                "{case}: {:?}",
                                painted[1]
                            );
                        }
                        // The transparent outline is not recorded.
                        None => assert!(
                            !painted
                                .iter()
                                .any(|drawn| matches!(drawn, Drawn::Stroke(..))),
                            "{case}"
                        ),
                    }
                    assert_eq!(painted.last(), Some(&thumb), "{case}");
                }
            }
        });
    }

    #[test]
    fn the_track_outline_color_and_width_resolve_in_the_selected_and_disabled_states() {
        with_main_thread_stack(|| {
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
            let mut tester = tester();
            for (enabled, active, expected_color, expected_width) in [
                (false, false, 0xFF000004, 4.0),
                (false, true, 0xFF000002, 2.0),
                (true, false, 0xFF000003, 3.0),
                (true, true, 0xFF000001, 1.0),
            ] {
                let home = in_material(
                    TextDirection::Rtl,
                    Switch::new(active, if enabled { ignore() } else { None })
                        .track_outline_color(Rc::clone(&color))
                        .track_outline_width(Rc::clone(&width))
                        .into_widget(),
                );
                pump_app(&mut tester, m3, move |_app| home);
                tester.pump_and_settle();
                let painted = drawn(&tester);
                assert!(matches!(painted[0], Drawn::Fill(..)), "{:?}", painted[0]);
                assert!(
                    matches!(painted[1], Drawn::Stroke(color, width, _)
                        if color == expected_color && f64::from(width) == expected_width),
                    "enabled {enabled}, active {active}: {:?}",
                    painted[1]
                );
            }
        });
    }

    #[test]
    fn the_track_outline_resolves_in_the_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let home = in_material(
                TextDirection::Rtl,
                Switch::new(true, ignore())
                    .focus_node(node)
                    .autofocus(true)
                    .track_outline_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            Some(Color::new(if states.contains(&WidgetState::Hovered) {
                                0xFF000001
                            } else if states.contains(&WidgetState::Focused) {
                                0xFF000002
                            } else {
                                0x00000000
                            }))
                        },
                    ))
                    .track_outline_width(<dyn WidgetStateProperty<Option<f64>>>::resolve_with(
                        |states| {
                            Some(if states.contains(&WidgetState::Hovered) {
                                4.0
                            } else if states.contains(&WidgetState::Focused) {
                                6.0
                            } else {
                                8.0
                            })
                        },
                    ))
                    .into_widget(),
            );
            pump_app(&mut tester, m3, move |_app| home);
            tester.pump_and_settle();
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            assert!(matches!(
                drawn(&tester)[1],
                Drawn::Stroke(0xFF000002, 6.0, _)
            ));

            // Start hovering
            hover(&mut tester);
            tester.pump_and_settle();
            assert!(matches!(
                drawn(&tester)[1],
                Drawn::Stroke(0xFF000001, 4.0, _)
            ));
        });
    }

    #[test]
    fn an_m3_thumb_shows_the_icon_for_its_side_only() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            crate::icons::install_material_icon_font(&mut tester.cell.borrow_mut());
            let theme = |app: &App| {
                ThemeData::builder()
                    .color_scheme_seed(Color::new(0xff6750a4))
                    .brightness(Brightness::Light)
                    .build(app)
            };
            let mut pump =
                |active: bool, active_icon: Option<IconData>, inactive_icon: Option<IconData>| {
                    let (active_icon, inactive_icon) = (
                        active_icon.map(|icon| Rc::new(Icon::new(Some(icon)))),
                        inactive_icon.map(|icon| Rc::new(Icon::new(Some(icon)))),
                    );
                    let home = Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(Center::new().child(
                            Switch::new(active, ignore()).thumb_icon(<dyn WidgetStateProperty<
                                Option<Rc<Icon>>,
                            >>::resolve_with(
                                move |states| {
                                    if states.contains(&WidgetState::Selected) {
                                        return active_icon.clone();
                                    }
                                    inactive_icon.clone()
                                },
                            )),
                        )),
                    )
                    .into_widget();
                    pump_app(&mut tester, theme, move |_app| home);
                    tester.pump_and_settle();
                    drawn(&tester)
                        .into_iter()
                        .filter_map(|drawn| match drawn {
                            Drawn::Glyphs(left, top) => Some((left, top)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                };
            // The glyph's ink is inside the 16x16 icon square Dart paints its paragraph at.
            let inside = |glyphs: &[(f64, f64)], left: f64, top: f64| matches!(glyphs, [(x, y)] if *x >= left && *x < left + 16.0 && *y >= top && *y < top + 16.0);

            // active icon shows when switch is on.
            let glyphs = pump(true, Some(Icons::close()), None);
            assert!(inside(&glyphs, 32.0, 16.0), "{glyphs:?}");
            // inactive icon shows when switch is off.
            let glyphs = pump(false, None, Some(Icons::close()));
            assert!(inside(&glyphs, 12.0, 16.0), "{glyphs:?}");
            // active icon doesn't show when switch is off.
            assert_eq!(pump(false, Some(Icons::check()), None), []);
            // inactive icon doesn't show when switch is on.
            assert_eq!(pump(true, None, Some(Icons::check())), []);
            // without icon
            assert_eq!(pump(false, None, None), []);
        });
    }

    #[test]
    fn an_adaptive_cupertino_switch_is_focusable_and_has_its_focus_color() {
        with_main_thread_stack(|| {
            const FOCUS_COLOR: u32 = 0xffff0000;
            let mut tester = tester_on(TargetPlatform::IOS);
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let value = shared(true);
            let pump = |tester: &mut WidgetTester, enabled: bool| {
                let home = Material::new()
                    .child(host(&value, enabled, move |value, on_changed| {
                        Switch::adaptive(value, on_changed)
                            .focus_color(Color::new(FOCUS_COLOR))
                            .focus_node(node)
                            .autofocus(true)
                    }))
                    .into_widget();
                let home = Center::new().child(home).into_widget();
                pump_app(tester, adaptive_on(TargetPlatform::IOS), move |_app| home);
                tester.pump_and_settle();
            };
            let thumb = [0x26000000, 0x0f000000, 0x0a000000, 0xffffffff];

            pump(&mut tester, true);
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(
                colors(&painted),
                [&[0xff34c759, FOCUS_COLOR][..], &thumb[..]].concat()
            );
            assert_eq!(painted[2], Drawn::Clip);

            // Check the false value.
            value.set(false);
            pump(&mut tester, true);
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(
                colors(&painted),
                [&[0x28787880, FOCUS_COLOR][..], &thumb[..]].concat()
            );
            assert_eq!(painted[2], Drawn::Clip);

            // Check what happens when disabled.
            pump(&mut tester, false);
            assert!(!node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(colors(&painted), [&[0x28787880][..], &thumb[..]].concat());
            assert_eq!(painted[1], Drawn::Clip);
        });
    }

    #[test]
    fn on_focus_change_hears_focus_arrive_and_leave() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let node = focus_node(&tester);
            let focused = Rc::new(Cell::new(false));
            let switch = {
                let focused = Rc::clone(&focused);
                Switch::new(true, ignore())
                    .focus_node(node)
                    .on_focus_change(move |_app, value| focused.set(value))
            };
            pump_switch(&mut tester, m3, switch);

            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(focused.get());
            assert!(node.has_focus(&mut tester.cell.borrow_mut()));

            node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
            tester.pump(Duration::ZERO);
            assert!(!focused.get());
            assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    #[test]
    fn the_padding_is_respected() {
        with_main_thread_stack(|| {
            for (padding, expected) in [
                (None, Size::new(60.0, 48.0)),
                (Some(EdgeInsetsGeometry::ZERO), Size::new(52.0, 48.0)),
                (Some(EdgeInsetsGeometry::all(4.0)), Size::new(60.0, 56.0)),
            ] {
                let mut tester = tester();
                let mut switch = Switch::new(true, ignore());
                if let Some(padding) = padding {
                    switch = switch.padding(padding);
                }
                pump_switch(&mut tester, m3, switch);
                assert_eq!(switch_rect(&tester).size(), expected, "{padding:?}");
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn the_active_thumb_color_wins_over_the_active_color() {
        with_main_thread_stack(|| {
            const ACTIVE_COLOR: Color = Color::new(0xffff0000);
            const ACTIVE_THUMB_COLOR: Color = Color::new(0xff00ff00);
            const ACTIVE_TRACK_COLOR: Color = Color::new(0xff0000ff);
            for (theme, use_material3) in [(m2 as fn(&App) -> ThemeData, false), (m3, true)] {
                let mut tester = tester();
                let value = shared(true);
                pump_host(
                    &mut tester,
                    theme,
                    TextDirection::Rtl,
                    &value,
                    true,
                    |_value, on_changed| {
                        Switch::new(true, on_changed)
                            .drag_start_behavior(DragStartBehavior::Down)
                            .active_color(ACTIVE_COLOR)
                            .active_thumb_color(ACTIVE_THUMB_COLOR)
                            .active_track_color(ACTIVE_TRACK_COLOR)
                    },
                );
                tester.widget(switch_element(&tester), |switch: &Switch| {
                    assert_eq!(switch.active_thumb_color, Some(ACTIVE_THUMB_COLOR));
                });
                let painted = drawn(&tester);
                if use_material3 {
                    assert_eq!(painted[0], Drawn::Fill(c(ACTIVE_TRACK_COLOR), M3_TRACK));
                    assert_eq!(
                        colors(&painted),
                        [c(ACTIVE_TRACK_COLOR), c(ACTIVE_THUMB_COLOR)]
                    );
                } else {
                    assert_eq!(painted[0], Drawn::Fill(c(ACTIVE_TRACK_COLOR), M2_TRACK));
                    assert_eq!(
                        colors(&painted),
                        m2_sequence(c(ACTIVE_TRACK_COLOR), &[], c(ACTIVE_THUMB_COLOR))
                    );
                }
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn an_adaptive_switch_takes_its_active_thumb_color_on_every_platform() {
        with_main_thread_stack(|| {
            const ACTIVE_COLOR: Color = Color::new(0xffff0000);
            const ACTIVE_THUMB_COLOR: Color = Color::new(0xff00ff00);
            const ACTIVE_TRACK_COLOR: Color = Color::new(0xff0000ff);
            for use_material3 in [false, true] {
                for platform in ALL_PLATFORMS {
                    let mut tester = tester_on(platform);
                    let value = shared(true);
                    pump_host(
                        &mut tester,
                        move |app| {
                            ThemeData::builder()
                                .platform(platform)
                                .use_material3(use_material3)
                                .build(app)
                        },
                        TextDirection::Rtl,
                        &value,
                        true,
                        |_value, on_changed| {
                            Switch::adaptive(true, on_changed)
                                .drag_start_behavior(DragStartBehavior::Down)
                                .active_color(ACTIVE_COLOR)
                                .active_thumb_color(ACTIVE_THUMB_COLOR)
                                .active_track_color(ACTIVE_TRACK_COLOR)
                        },
                    );
                    let compact = !MOBILE.contains(&platform) && platform != TargetPlatform::IOS;
                    let (track, sequence) = if CUPERTINO_PLATFORMS.contains(&platform) {
                        let (left, right) = if use_material3 {
                            (4.5, 55.5)
                        } else {
                            (4.0, 55.0)
                        };
                        let (top, bottom) = if compact { (4.5, 35.5) } else { (8.5, 39.5) };
                        (
                            ltrb(left, top, right, bottom),
                            [
                                &[c(ACTIVE_TRACK_COLOR)][..],
                                &CUPERTINO_SHADOWS_AND_BORDER[..],
                                &[c(ACTIVE_THUMB_COLOR)][..],
                            ]
                            .concat(),
                        )
                    } else if use_material3 {
                        (
                            if compact {
                                ltrb(4.0, 4.0, 56.0, 36.0)
                            } else {
                                M3_TRACK
                            },
                            vec![c(ACTIVE_TRACK_COLOR), c(ACTIVE_THUMB_COLOR)],
                        )
                    } else {
                        (
                            if compact {
                                ltrb(13.0, 13.0, 46.0, 27.0)
                            } else {
                                M2_TRACK
                            },
                            m2_sequence(c(ACTIVE_TRACK_COLOR), &[], c(ACTIVE_THUMB_COLOR)),
                        )
                    };
                    let painted = drawn(&tester);
                    let case = format!("M3 {use_material3}, {platform:?}");
                    assert_eq!(
                        painted[0],
                        Drawn::Fill(c(ACTIVE_TRACK_COLOR), track),
                        "{case}"
                    );
                    assert_eq!(colors(&painted), sequence, "{case}");
                }
            }
        });
    }

    /// Not a Dart case: `Switch.adaptive(applyCupertinoTheme:)` and the Cupertino override
    /// theme's `applyThemeToAll` give the track the Cupertino primary colour on iOS.
    #[test]
    fn an_adaptive_cupertino_switch_applies_the_cupertino_theme_where_asked() {
        with_main_thread_stack(|| {
            const PRIMARY: Color = Color::new(0xffffc107);
            for (apply_to_all, apply, expected) in [
                (false, None, 0xff34c759),
                (false, Some(true), c(PRIMARY)),
                (true, None, c(PRIMARY)),
                (true, Some(false), 0xff34c759),
            ] {
                let mut tester = tester_on(TargetPlatform::IOS);
                let mut switch = Switch::adaptive(true, ignore());
                if let Some(apply) = apply {
                    switch = switch.apply_cupertino_theme(apply);
                }
                pump_switch(
                    &mut tester,
                    move |app| {
                        ThemeData::builder()
                            .platform(TargetPlatform::IOS)
                            .cupertino_override_theme(waft_cupertino::NoDefaultCupertinoThemeData {
                                primary_color: Some(PRIMARY.into()),
                                apply_theme_to_all: Some(apply_to_all),
                                ..waft_cupertino::NoDefaultCupertinoThemeData::default()
                            })
                            .build(app)
                    },
                    switch,
                );
                assert_eq!(
                    drawn(&tester)[0].color(),
                    Some(expected),
                    "apply to all {apply_to_all}, apply {apply:?}"
                );
            }
        });
    }

    #[test]
    fn a_switch_that_disappears_while_a_pointer_is_down_does_not_crash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_switch(&mut tester, m3, Switch::new(true, ignore()));
            let at = center(&tester);
            // Put a pointer down on the screen.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, at);
            tester.pump(Duration::ZERO);
            // While the pointer is down, the widget disappears.
            let home = in_material(TextDirection::Ltr, Container::new().into_widget());
            pump_app(&mut tester, m3, move |_app| home);
            assert!(tester.find_by_type::<Switch>().is_empty());
            // Release pointer after widget disappeared.
            tester.up(&gesture);
        });
    }

    /// Dart's two `Switch does not throw when first painted …` cases: an `Opacity(0)` lays out
    /// the switch without painting it, and a toggle makes it visible.
    fn first_painted_host(
        value: &Shared,
        visible: &Shared,
        set_state_slot: &Rc<RefCell<Option<StateSetter>>>,
        interactive: bool,
    ) -> WidgetRef {
        let (value, visible, slot) = (
            Rc::clone(value),
            Rc::clone(visible),
            Rc::clone(set_state_slot),
        );
        let home = StatefulBuilder::new(move |_app, _context, set_state| {
            *slot.borrow_mut() = Some(Rc::clone(&set_state));
            let on_changed: ValueChanged<bool> = {
                let (value, visible) = (Rc::clone(&value), Rc::clone(&visible));
                Rc::new(move |app: &mut App, new_value: bool| {
                    if interactive {
                        let (value, visible) = (Rc::clone(&value), Rc::clone(&visible));
                        set_state(app, &mut move |_app| {
                            value.set(new_value);
                            visible.set(true);
                        });
                    }
                })
            };
            Opacity::new(if visible.get() { 1.0 } else { 0.0 })
                .child(Switch::adaptive(value.get(), Some(on_changed)))
                .into_widget()
        });
        Material::new()
            .child(Center::new().child(home))
            .into_widget()
    }

    #[test]
    fn a_switch_first_painted_while_its_press_reaction_reverses_does_not_panic() {
        with_main_thread_stack(|| {
            for platform in ALL_PLATFORMS {
                let mut tester = tester_on(platform);
                let (value, visible) = (shared(false), shared(false));
                let slot = Rc::new(RefCell::new(None));
                let home = first_painted_host(&value, &visible, &slot, true);
                pump_app(&mut tester, adaptive_on(platform), move |_app| home);

                // Press and hold so that the press reaction starts.
                let at = center(&tester);
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, at);
                tester.pump(waft_gestures::K_PRESS_TIMEOUT);
                tester.pump(Duration::from_millis(50));

                // Releasing the pointer reverses the press reaction and toggles the switch,
                // which makes it visible.
                tester.up(&gesture);
                let state = state(&tester);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(
                        state.reaction(&app).status(&app),
                        AnimationStatus::Reverse,
                        "{platform:?}"
                    );
                }
                assert!(value.get());

                // Paint the switch for the first time.
                tester.pump(Duration::ZERO);
                drawn(&tester);
            }
        });
    }

    #[test]
    fn a_switch_first_painted_during_its_toggle_animation_does_not_panic() {
        with_main_thread_stack(|| {
            for platform in ALL_PLATFORMS {
                let mut tester = tester_on(platform);
                let (value, visible) = (shared(false), shared(false));
                let slot: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
                let home = first_painted_host(&value, &visible, &slot, false);
                pump_app(&mut tester, adaptive_on(platform), move |_app| home);
                let set_state = slot.borrow().clone().expect("the builder ran");

                // Start the toggle animation while the switch has never been painted.
                {
                    let value = Rc::clone(&value);
                    set_state(&mut tester.cell.borrow_mut(), &mut move |_app| {
                        value.set(true)
                    });
                }
                tester.pump(Duration::ZERO);
                tester.pump(Duration::ZERO); // First tick of the toggle animation.
                tester.pump(Duration::from_millis(50));
                let position = position(&tester);
                assert!(position > 0.0 && position < 1.0, "{platform:?}: {position}");

                // Paint the switch for the first time, in the middle of the toggle animation.
                {
                    let visible = Rc::clone(&visible);
                    set_state(&mut tester.cell.borrow_mut(), &mut move |_app| {
                        visible.set(true)
                    });
                }
                tester.pump(Duration::ZERO);
                drawn(&tester);
            }
        });
    }

    #[test]
    fn an_m2_switch_is_focusable_and_has_its_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            let value = shared(true);
            let orange = Colors::ORANGE.shade500();
            let pump = |tester: &mut WidgetTester, enabled: bool| {
                pump_host(
                    tester,
                    m2,
                    TextDirection::Ltr,
                    &value,
                    enabled,
                    move |value, on_changed| {
                        Switch::new(value, on_changed)
                            .focus_color(orange)
                            .autofocus(true)
                            .focus_node(node)
                    },
                );
                tester.pump_and_settle();
            };

            pump(&mut tester, true);
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x802196f3, M2_TRACK));
            assert_eq!(
                colors(&painted),
                m2_sequence(0x802196f3, &[c(orange)], 0xff2196f3)
            );

            // Check the false value.
            value.set(false);
            pump(&mut tester, true);
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x52000000, M2_TRACK));
            assert_eq!(
                colors(&painted),
                m2_sequence(0x52000000, &[c(orange)], 0xfffafafa)
            );

            // Check what happens when disabled.
            pump(&mut tester, false);
            assert!(!node.has_primary_focus(&tester.cell.borrow()));
            let painted = drawn(&tester);
            assert_eq!(painted[0], Drawn::Fill(0x1f000000, M2_TRACK));
            assert_eq!(colors(&painted), m2_sequence(0x1f000000, &[], 0xffbdbdbd));
        });
    }

    /// Regression test for https://github.com/flutter/flutter/issues/61247.
    #[test]
    fn a_material_switch_keeps_its_render_object_when_disabled_mid_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (value, enabled) = (shared(true), shared(true));
            let slot: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
            let home = {
                let (value, enabled, slot) =
                    (Rc::clone(&value), Rc::clone(&enabled), Rc::clone(&slot));
                StatefulBuilder::new(move |_app, _context, set_state| {
                    *slot.borrow_mut() = Some(Rc::clone(&set_state));
                    let on_changed = enabled.get().then(|| {
                        let value = Rc::clone(&value);
                        Rc::new(move |app: &mut App, new_value: bool| {
                            let value = Rc::clone(&value);
                            set_state(app, &mut move |_app| value.set(new_value));
                        }) as ValueChanged<bool>
                    });
                    Material::new()
                        .child(Center::new().child(Switch::new(value.get(), on_changed)))
                        .into_widget()
                })
                .into_widget()
            };
            let home = Directionality::new(TextDirection::Ltr, home).into_widget();
            pump_app(&mut tester, m3, move |_app| home);
            let old_switch_state = state(&tester);
            let set_state = slot.borrow().clone().expect("the builder ran");

            {
                let value = Rc::clone(&value);
                set_state(&mut tester.cell.borrow_mut(), &mut move |_app| {
                    value.set(false)
                });
            }
            tester.pump(Duration::ZERO);
            // Disable the switch when the implicit animation begins.
            {
                let enabled = Rc::clone(&enabled);
                set_state(&mut tester.cell.borrow_mut(), &mut move |_app| {
                    enabled.set(false)
                });
            }
            tester.pump(Duration::ZERO);

            let updated_switch_state = state(&tester);
            let app = tester.cell.borrow();
            assert!(!updated_switch_state.is_interactive(&app));
            assert_eq!(updated_switch_state, old_switch_state);
            let status = updated_switch_state.position(&app).status(&app);
            assert!(!status.is_completed(), "{status:?}");
            assert!(!status.is_dismissed(), "{status:?}");
        });
    }

    #[test]
    fn an_m2_thumb_color_resolves_in_the_selected_and_disabled_states() {
        with_main_thread_stack(|| {
            const ACTIVE_ENABLED: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED: Color = Color::new(0xFF000004);
            let thumb_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                Some(if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        ACTIVE_DISABLED
                    } else {
                        INACTIVE_DISABLED
                    }
                } else if states.contains(&WidgetState::Selected) {
                    ACTIVE_ENABLED
                } else {
                    INACTIVE_ENABLED
                })
            });
            let mut tester = tester();
            for (index, (enabled, active, track, thumb, reason)) in [
                (
                    false,
                    false,
                    c(Colors::BLACK12),
                    INACTIVE_DISABLED,
                    "Inactive disabled switch should default track and custom thumb color",
                ),
                (
                    false,
                    true,
                    c(Colors::BLACK12),
                    ACTIVE_DISABLED,
                    "Active disabled switch should match these colors",
                ),
                (
                    true,
                    false,
                    0x52000000, // Black with 32% opacity,
                    INACTIVE_ENABLED,
                    "Inactive enabled switch should match these colors",
                ),
                (
                    false,
                    false,
                    c(Colors::BLACK12),
                    INACTIVE_DISABLED,
                    "Inactive disabled switch should match these colors",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let home = in_material(
                    TextDirection::Rtl,
                    Switch::new(active, if enabled { ignore() } else { None })
                        .thumb_color(Rc::clone(&thumb_color))
                        .into_widget(),
                );
                pump_app(&mut tester, m2, move |_app| home);
                if index > 0 {
                    tester.pump_and_settle();
                }
                let painted = drawn(&tester);
                assert_eq!(painted[0], Drawn::Fill(track, M2_TRACK), "{reason}");
                assert_eq!(
                    colors(&painted),
                    m2_sequence(track, &[], c(thumb)),
                    "{reason}"
                );
            }
        });
    }

    #[test]
    fn an_m2_thumb_color_resolves_in_the_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            const HOVERED: Color = Color::new(0xFF000001);
            const FOCUSED: Color = Color::new(0xFF000002);
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let node = focus_node(&tester);
            pump_switch(
                &mut tester,
                m2,
                Switch::new(true, ignore())
                    .focus_node(node)
                    .autofocus(true)
                    .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            Some(if states.contains(&WidgetState::Hovered) {
                                HOVERED
                            } else if states.contains(&WidgetState::Focused) {
                                FOCUSED
                            } else {
                                Colors::TRANSPARENT
                            })
                        },
                    )),
            );
            tester.pump_and_settle();
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            // The track, the radial reaction, the thumb's three shadows and the thumb.
            let assert_painted = |tester: &WidgetTester, thumb: Color| {
                let painted = drawn(tester);
                assert_eq!(painted[0], Drawn::Fill(0x802196f3, M2_TRACK));
                let reactions = circles(&painted, K_RADIAL_REACTION_RADIUS);
                assert_eq!(reactions.len(), 1, "{painted:?}");
                assert_eq!(
                    colors(&painted),
                    m2_sequence(0x802196f3, &reactions, c(thumb))
                );
            };
            assert_painted(&tester, FOCUSED);

            // Start hovering
            hover(&mut tester);
            tester.pump_and_settle();
            assert_painted(&tester, HOVERED);
        });
    }

    /// Dart's 'Material2 - Track color resolves in active/enabled states' and its Material 3
    /// twin.
    #[test]
    fn the_track_color_resolves_in_the_selected_and_disabled_states() {
        with_main_thread_stack(|| {
            const ACTIVE_ENABLED: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED: Color = Color::new(0xFF000004);
            let track_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                Some(if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        ACTIVE_DISABLED
                    } else {
                        INACTIVE_DISABLED
                    }
                } else if states.contains(&WidgetState::Selected) {
                    ACTIVE_ENABLED
                } else {
                    INACTIVE_ENABLED
                })
            });
            for (theme, use_material3) in [(m2 as fn(&App) -> ThemeData, false), (m3, true)] {
                let (direction, track, last) = if use_material3 {
                    (
                        TextDirection::Rtl,
                        M3_TRACK,
                        (true, true, ACTIVE_ENABLED, "Active enabled switch"),
                    )
                } else {
                    (
                        TextDirection::Ltr,
                        M2_TRACK,
                        (false, false, INACTIVE_DISABLED, "Inactive disabled switch"),
                    )
                };
                let mut tester = tester();
                for (index, (enabled, active, expected, reason)) in [
                    (
                        false,
                        false,
                        INACTIVE_DISABLED,
                        "Inactive disabled switch track",
                    ),
                    (false, true, ACTIVE_DISABLED, "Active disabled switch"),
                    (true, false, INACTIVE_ENABLED, "Inactive enabled switch"),
                    last,
                ]
                .into_iter()
                .enumerate()
                {
                    let home = in_material(
                        direction,
                        Switch::new(active, if enabled { ignore() } else { None })
                            .track_color(Rc::clone(&track_color))
                            .into_widget(),
                    );
                    pump_app(&mut tester, theme, move |_app| home);
                    if index > 0 {
                        tester.pump_and_settle();
                    }
                    assert_eq!(
                        drawn(&tester)[0],
                        Drawn::Fill(c(expected), track),
                        "M3 {use_material3}: {reason}"
                    );
                }
            }
        });
    }

    /// Dart's 'Material2 - Switch track color resolves in hovered/focused states' and its
    /// Material 3 twin.
    #[test]
    fn the_track_color_resolves_in_the_hovered_and_focused_states() {
        with_main_thread_stack(|| {
            const HOVERED: Color = Color::new(0xFF000001);
            const FOCUSED: Color = Color::new(0xFF000002);
            for (theme, track) in [(m2 as fn(&App) -> ThemeData, M2_TRACK), (m3, M3_TRACK)] {
                let mut tester = tester();
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let node = focus_node(&tester);
                let home = in_material(
                    TextDirection::Rtl,
                    Switch::new(true, ignore())
                        .focus_node(node)
                        .autofocus(true)
                        .track_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                Some(if states.contains(&WidgetState::Hovered) {
                                    HOVERED
                                } else if states.contains(&WidgetState::Focused) {
                                    FOCUSED
                                } else {
                                    Colors::TRANSPARENT
                                })
                            },
                        ))
                        .into_widget(),
                );
                pump_app(&mut tester, theme, move |_app| home);
                tester.pump_and_settle();
                assert!(node.has_primary_focus(&tester.cell.borrow()));
                assert_eq!(drawn(&tester)[0], Drawn::Fill(c(FOCUSED), track));

                // Start hovering
                hover(&mut tester);
                tester.pump_and_settle();
                assert_eq!(drawn(&tester)[0], Drawn::Fill(c(HOVERED), track));
            }
        });
    }

    /// Dart's 'Material2 - Switch thumb color is blended against surface color' and its
    /// Material 3 twin.
    #[test]
    fn the_thumb_color_is_blended_against_the_surface_color() {
        with_main_thread_stack(|| {
            let active_disabled_thumb_color = with_opacity(Colors::BLUE.primary(), 0.60);
            for (theme, use_material3) in [(m2 as fn(&App) -> ThemeData, false), (m3, true)] {
                let mut tester = tester();
                let colors_ = colors_of(&tester.cell.borrow(), theme);
                let home = in_material(
                    TextDirection::Rtl,
                    Switch::new(true, None)
                        .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            move |states| {
                                Some(if states.contains(&WidgetState::Disabled) {
                                    active_disabled_thumb_color
                                } else {
                                    Colors::BLACK
                                })
                            },
                        ))
                        .into_widget(),
                );
                pump_app(&mut tester, theme, move |_app| home);
                let expected_thumb_color = c(Color::alpha_blend(
                    active_disabled_thumb_color,
                    colors_.surface,
                ));
                let painted = drawn(&tester);
                let reason =
                    "Active disabled thumb color should be blended on top of surface color";
                if use_material3 {
                    assert_eq!(
                        painted[0],
                        Drawn::Fill(c(with_opacity(colors_.on_surface, 0.12)), M3_TRACK),
                        "{reason}"
                    );
                    assert_eq!(
                        painted.last().and_then(Drawn::color),
                        Some(expected_thumb_color),
                        "{reason}"
                    );
                } else {
                    assert_eq!(
                        painted[0],
                        Drawn::Fill(c(Colors::BLACK12), M2_TRACK),
                        "{reason}"
                    );
                    assert_eq!(
                        colors(&painted),
                        m2_sequence(c(Colors::BLACK12), &[], expected_thumb_color),
                        "{reason}"
                    );
                }
            }
        });
    }

    #[test]
    fn a_disabled_switch_shows_its_tooltip() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const LONG_PRESS_TOOLTIP: &str = "long press tooltip";
            const TAP_TOOLTIP: &str = "tap tooltip";
            let frame = |tester: &mut WidgetTester, tooltip: crate::tooltip::Tooltip| {
                tester.pump_widget(
                    crate::app::MaterialApp::new()
                        .home(Material::new().child(tooltip.child(Switch::new(true, None))))
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

    impl Drawn {
        fn rect(self) -> Rect {
            match self {
                Drawn::Fill(_, rect) | Drawn::Stroke(_, _, rect) => rect,
                _ => panic!("{self:?} has no rect"),
            }
        }
    }
}

//! Flutter counterpart: material_ui's `button.dart`.
//!
//! The `button_theme.dart` that Dart's file imports for its documentation waits with
//! `material_button.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Clip, Color, Offset, Size, TextBaseline};
use waft_foundation::{App, Handle, Listener, ValueChanged};
use waft_painting::{
    Alignment, EdgeInsets, EdgeInsetsGeometry, OutlinedBorderRef, RoundedRectangleBorder,
    ShapeBorderRef, TextStyle, force_to_point,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, PaintingContext, RenderBox, RenderBoxBase, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderShiftedBox,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Center, ConstrainedBox, FocusNodeRef, IconTheme, IconThemeData, IntoWidget,
    KeyRef, Padding, RenderObjectWidget, Semantics, SingleChildRenderObjectWidget, State,
    StateData, StatefulWidget, WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStates,
};

use crate::constants::{K_MIN_INTERACTIVE_DIMENSION, K_THEME_CHANGE_DURATION};
use crate::ink_well::InkWell;
use crate::material::{Material, MaterialType};
use crate::material_state_mixin::{MaterialStateMixin, MaterialStateMixinData};
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Creates a button based on `Semantics`, [`Material`], and [`InkWell`]
/// widgets.
///
/// This class does not use the current `Theme` or `ButtonTheme` to
/// compute default values for unspecified parameters. It's intended to
/// be used for custom Material buttons that optionally incorporate defaults
/// from the themes or from app-specific sources.
///
/// This class is planned to be deprecated in a future release, see
/// `ButtonStyleButton`, the base class of `ElevatedButton`, `FilledButton`,
/// `OutlinedButton` and `TextButton`.
///
/// See also:
///
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `FilledButton`, a filled button that doesn't elevate when pressed.
///  * `FilledButton.tonal`, a filled button variant that uses a secondary fill color.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * `TextButton`, a button with no outline or fill color.
pub struct RawMaterialButton {
    pub key: Option<KeyRef>,

    /// Called when the button is tapped or otherwise activated.
    ///
    /// If this callback and [`on_long_press`](Self::on_long_press) are null, then the button
    /// will be disabled.
    ///
    /// See also:
    ///
    ///  * [`enabled`](Self::enabled), which is true if the button is enabled.
    pub on_pressed: Option<Listener>,

    /// Called when the button is long-pressed.
    ///
    /// If this callback and [`on_pressed`](Self::on_pressed) are null, then the button will be
    /// disabled.
    ///
    /// See also:
    ///
    ///  * [`enabled`](Self::enabled), which is true if the button is enabled.
    pub on_long_press: Option<Listener>,

    /// Called by the underlying [`InkWell`] widget's `InkWell.on_highlight_changed`
    /// callback.
    ///
    /// If [`on_pressed`](Self::on_pressed) changes from null to non-null while a gesture is
    /// ongoing, this can fire during the build phase (in which case calling
    /// [`State::set_state`] is not allowed).
    pub on_highlight_changed: Option<ValueChanged<bool>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// button.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Defines the default text style, with [`Material::text_style`], for the
    /// button's [`child`](Self::child).
    ///
    /// If `TextStyle.color` is a `WidgetStateProperty<Color>`, `WidgetStateProperty::resolve`
    /// is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    pub text_style: Option<TextStyle>,

    /// The color of the button's [`Material`].
    pub fill_color: Option<Color>,

    /// The color for the button's [`Material`] when it has the input focus.
    pub focus_color: Option<Color>,

    /// The color for the button's [`Material`] when a pointer is hovering over it.
    pub hover_color: Option<Color>,

    /// The highlight color for the button's [`InkWell`].
    pub highlight_color: Option<Color>,

    /// The splash color for the button's [`InkWell`].
    pub splash_color: Option<Color>,

    /// The elevation for the button's [`Material`] when the button
    /// is [`enabled`](Self::enabled) but not pressed.
    ///
    /// Defaults to 2.0. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`highlight_elevation`](Self::highlight_elevation), the default elevation.
    ///  * [`hover_elevation`](Self::hover_elevation), the elevation when a pointer is
    ///    hovering over the button.
    ///  * [`focus_elevation`](Self::focus_elevation), the elevation when the button is
    ///    focused.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    pub elevation: f64,

    /// The elevation for the button's [`Material`] when the button
    /// is [`enabled`](Self::enabled) and a pointer is hovering over it.
    ///
    /// Defaults to 4.0. The value is always non-negative.
    ///
    /// If the button is [`enabled`](Self::enabled), and being pressed (in the highlighted
    /// state), then the [`highlight_elevation`](Self::highlight_elevation) take precedence
    /// over the [`hover_elevation`](Self::hover_elevation).
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`focus_elevation`](Self::focus_elevation), the elevation when the button is
    ///    focused.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    pub hover_elevation: f64,

    /// The elevation for the button's [`Material`] when the button
    /// is [`enabled`](Self::enabled) and has the input focus.
    ///
    /// Defaults to 4.0. The value is always non-negative.
    ///
    /// If the button is [`enabled`](Self::enabled), and being pressed (in the highlighted
    /// state), or a mouse cursor is hovering over the button, then the
    /// [`hover_elevation`](Self::hover_elevation) and
    /// [`highlight_elevation`](Self::highlight_elevation) take precedence over the
    /// [`focus_elevation`](Self::focus_elevation).
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`hover_elevation`](Self::hover_elevation), the elevation when a pointer is
    ///    hovering over the button.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    pub focus_elevation: f64,

    /// The elevation for the button's [`Material`] when the button
    /// is [`enabled`](Self::enabled) and pressed.
    ///
    /// Defaults to 8.0. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`hover_elevation`](Self::hover_elevation), the elevation when a pointer is
    ///    hovering over the button.
    ///  * [`focus_elevation`](Self::focus_elevation), the elevation when the button is
    ///    focused.
    ///  * [`disabled_elevation`](Self::disabled_elevation), the elevation when the button is
    ///    disabled.
    pub highlight_elevation: f64,

    /// The elevation for the button's [`Material`] when the button
    /// is not [`enabled`](Self::enabled).
    ///
    /// Defaults to 0.0. The value is always non-negative.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), the default elevation.
    ///  * [`hover_elevation`](Self::hover_elevation), the elevation when a pointer is
    ///    hovering over the button.
    ///  * [`focus_elevation`](Self::focus_elevation), the elevation when the button is
    ///    focused.
    ///  * [`highlight_elevation`](Self::highlight_elevation), the elevation when the button
    ///    is pressed.
    pub disabled_elevation: f64,

    /// The internal padding for the button's [`child`](Self::child).
    pub padding: EdgeInsetsGeometry,

    /// Defines how compact the button's layout will be.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the [`visual_density`](Self::visual_density)
    ///    for all widgets within a `Theme`.
    pub visual_density: VisualDensity,

    /// Defines the button's size.
    ///
    /// Typically used to constrain the button's minimum size.
    pub constraints: BoxConstraints,

    /// The shape of the button's [`Material`].
    ///
    /// The button's highlight and splash are clipped to this shape. If the
    /// button has an elevation, then its drop shadow is defined by this shape.
    ///
    /// If [`shape`](Self::shape) is a [`WidgetStateOutlinedBorder`](waft_widgets::WidgetStateOutlinedBorder),
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    /// * [`WidgetState::Pressed`].
    /// * [`WidgetState::Hovered`].
    /// * [`WidgetState::Focused`].
    /// * [`WidgetState::Disabled`].
    pub shape: ShapeBorderRef,

    /// Defines the duration of animated changes for [`shape`](Self::shape) and
    /// [`elevation`](Self::elevation).
    ///
    /// The default value is [`K_THEME_CHANGE_DURATION`].
    pub animation_duration: Duration,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,

    /// An optional focus node to use as the focus node for this widget.
    ///
    /// See `Focus::focus_node`.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    ///
    /// See `Focus::autofocus`.
    pub autofocus: bool,

    /// Configures the minimum size of the tap target.
    ///
    /// Defaults to [`MaterialTapTargetSize::Padded`].
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    pub material_tap_target_size: MaterialTapTargetSize,

    /// Typically the button's label.
    pub child: Option<WidgetRef>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: bool,
}

impl RawMaterialButton {
    /// Create a button based on `Semantics`, [`Material`], and [`InkWell`] widgets.
    ///
    /// The [`elevation`](Self::elevation), [`focus_elevation`](Self::focus_elevation),
    /// [`hover_elevation`](Self::hover_elevation),
    /// [`highlight_elevation`](Self::highlight_elevation), and
    /// [`disabled_elevation`](Self::disabled_elevation) parameters must be non-negative.
    pub fn new(on_pressed: Option<Listener>) -> RawMaterialButton {
        RawMaterialButton {
            key: None,
            on_pressed,
            on_long_press: None,
            on_highlight_changed: None,
            mouse_cursor: None,
            text_style: None,
            fill_color: None,
            focus_color: None,
            hover_color: None,
            highlight_color: None,
            splash_color: None,
            elevation: 2.0,
            focus_elevation: 4.0,
            hover_elevation: 4.0,
            highlight_elevation: 8.0,
            disabled_elevation: 0.0,
            padding: EdgeInsetsGeometry::ZERO,
            visual_density: VisualDensity::STANDARD,
            constraints: BoxConstraints::new().min_width(88.0).min_height(36.0),
            shape: ShapeBorderRef::new(RoundedRectangleBorder::default()),
            animation_duration: K_THEME_CHANGE_DURATION,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            material_tap_target_size: MaterialTapTargetSize::Padded,
            child: None,
            enable_feedback: true,
        }
    }

    /// Dart `RawMaterialButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawMaterialButton {
        self.key = Some(key);
        self
    }

    /// Dart `RawMaterialButton(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: Listener) -> RawMaterialButton {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `RawMaterialButton(onHighlightChanged:)`.
    pub fn on_highlight_changed(
        mut self,
        on_highlight_changed: ValueChanged<bool>,
    ) -> RawMaterialButton {
        self.on_highlight_changed = Some(on_highlight_changed);
        self
    }

    /// Dart `RawMaterialButton(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> RawMaterialButton {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `RawMaterialButton(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> RawMaterialButton {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `RawMaterialButton(fillColor:)`.
    pub fn fill_color(mut self, fill_color: Color) -> RawMaterialButton {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `RawMaterialButton(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> RawMaterialButton {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `RawMaterialButton(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> RawMaterialButton {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `RawMaterialButton(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> RawMaterialButton {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `RawMaterialButton(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> RawMaterialButton {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `RawMaterialButton(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> RawMaterialButton {
        debug_assert!(elevation >= 0.0);
        self.elevation = elevation;
        self
    }

    /// Dart `RawMaterialButton(focusElevation:)`.
    pub fn focus_elevation(mut self, focus_elevation: f64) -> RawMaterialButton {
        debug_assert!(focus_elevation >= 0.0);
        self.focus_elevation = focus_elevation;
        self
    }

    /// Dart `RawMaterialButton(hoverElevation:)`.
    pub fn hover_elevation(mut self, hover_elevation: f64) -> RawMaterialButton {
        debug_assert!(hover_elevation >= 0.0);
        self.hover_elevation = hover_elevation;
        self
    }

    /// Dart `RawMaterialButton(highlightElevation:)`.
    pub fn highlight_elevation(mut self, highlight_elevation: f64) -> RawMaterialButton {
        debug_assert!(highlight_elevation >= 0.0);
        self.highlight_elevation = highlight_elevation;
        self
    }

    /// Dart `RawMaterialButton(disabledElevation:)`.
    pub fn disabled_elevation(mut self, disabled_elevation: f64) -> RawMaterialButton {
        debug_assert!(disabled_elevation >= 0.0);
        self.disabled_elevation = disabled_elevation;
        self
    }

    /// Dart `RawMaterialButton(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> RawMaterialButton {
        self.padding = padding;
        self
    }

    /// Dart `RawMaterialButton(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> RawMaterialButton {
        self.visual_density = visual_density;
        self
    }

    /// Dart `RawMaterialButton(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> RawMaterialButton {
        self.constraints = constraints;
        self
    }

    /// Dart `RawMaterialButton(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> RawMaterialButton {
        self.shape = shape.into();
        self
    }

    /// Dart `RawMaterialButton(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> RawMaterialButton {
        self.animation_duration = animation_duration;
        self
    }

    /// Dart `RawMaterialButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> RawMaterialButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `RawMaterialButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> RawMaterialButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `RawMaterialButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> RawMaterialButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `RawMaterialButton(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> RawMaterialButton {
        self.material_tap_target_size = material_tap_target_size;
        self
    }

    /// Dart `RawMaterialButton(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RawMaterialButton {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `RawMaterialButton(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> RawMaterialButton {
        self.enable_feedback = enable_feedback;
        self
    }

    /// Whether the button is enabled or disabled.
    ///
    /// Buttons are disabled by default. To enable a button, set its
    /// [`on_pressed`](Self::on_pressed) or [`on_long_press`](Self::on_long_press) properties
    /// to a non-null value.
    pub fn enabled(&self) -> bool {
        self.on_pressed.is_some() || self.on_long_press.is_some()
    }
}

impl fmt::Debug for RawMaterialButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawMaterialButton")
            .field("enabled", &self.enabled())
            .field("fill_color", &self.fill_color)
            .field("elevation", &self.elevation)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RawMaterialButton {
    type State = RawMaterialButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawMaterialButtonState {
        RawMaterialButtonState {
            state: StateData::new(),
            material_state: MaterialStateMixinData::new(),
        }
    }
}

/// Dart's `_RawMaterialButtonState`.
pub struct RawMaterialButtonState {
    state: StateData<RawMaterialButton>,
    material_state: MaterialStateMixinData,
}

impl RawMaterialButtonState {
    fn effective_elevation(self: Handle<Self>, app: &App) -> f64 {
        let widget = self.widget(app);
        // These conditionals are in order of precedence, so be careful about
        // reorganizing them.
        if self.is_disabled(app) {
            return widget.disabled_elevation;
        }
        if self.is_pressed(app) {
            return widget.highlight_elevation;
        }
        if self.is_hovered(app) {
            return widget.hover_elevation;
        }
        if self.is_focused(app) {
            return widget.focus_elevation;
        }
        widget.elevation
    }
}

/// Dart's `WidgetStateProperty.resolveAs<ShapeBorder?>(shape, states)`: the border a
/// [`WidgetStateOutlinedBorder`](waft_widgets::WidgetStateOutlinedBorder) resolves to, and any
/// other border itself.
fn resolve_shape(shape: &ShapeBorderRef, states: &WidgetStates) -> Option<ShapeBorderRef> {
    let Some(outlined) = shape.as_outlined_border_ref() else {
        return Some(shape.clone());
    };
    let resolved =
        <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_as(&Some(outlined), states);
    resolved.map(ShapeBorderRef::from)
}

impl MaterialStateMixin for RawMaterialButtonState {
    fn material_state_data(self: Handle<Self>, app: &App) -> &MaterialStateMixinData {
        &app.get(self).material_state
    }

    fn material_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut MaterialStateMixinData {
        &mut app.get_mut(self).material_state
    }
}

impl State for RawMaterialButtonState {
    type Widget = RawMaterialButton;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let enabled = self.widget(app).enabled();
        self.set_material_state(app, WidgetState::Disabled, !enabled);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &RawMaterialButton) {
        let enabled = self.widget(app).enabled();
        self.set_material_state(app, WidgetState::Disabled, !enabled);
        // If the button is disabled while a press gesture is currently ongoing,
        // InkWell makes a call to handleHighlightChanged. This causes an exception
        // because it calls setState in the middle of a build. To preempt this, we
        // manually update pressed to false when this situation occurs.
        if self.is_disabled(app) && self.is_pressed(app) {
            self.remove_material_state(app, WidgetState::Pressed);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // Dart reads the theme where it builds the `Material`; `widget` below borrows `app`.
        let theme = Theme::of(app, context);
        let widget = self.widget(app);
        let states = self.material_states(app);
        let effective_text_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &widget
                .text_style
                .as_ref()
                .and_then(|text_style| text_style.color.clone()),
            states,
        );
        let effective_shape = resolve_shape(&widget.shape, states);
        let density_adjustment = widget.visual_density.base_size_adjustment();
        let effective_constraints = widget
            .visual_density
            .effective_constraints(widget.constraints);
        let mouse_cursor: MouseCursorRef = widget
            .mouse_cursor
            .clone()
            .unwrap_or_else(|| Rc::new(WidgetStateMouseCursor::adaptive_clickable()));
        let effective_mouse_cursor =
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states);
        let padding = widget
            .padding
            .add(EdgeInsetsGeometry::Insets(EdgeInsets::only(
                density_adjustment.dx(),
                density_adjustment.dy(),
                density_adjustment.dx(),
                density_adjustment.dy(),
            )))
            .clamp(&EdgeInsetsGeometry::ZERO, &EdgeInsetsGeometry::INFINITY);

        let mut center = Center::new().width_factor(1.0).height_factor(1.0);
        if let Some(child) = &widget.child {
            center = center.child(child.clone());
        }
        let mut icon_theme_data = IconThemeData::new();
        if let Some(color) = effective_text_color.clone() {
            icon_theme_data = icon_theme_data.color(color);
        }
        let mut ink_well =
            InkWell::new()
                .can_request_focus(widget.enabled())
                .on_focus_change(self.update_material_state(WidgetState::Focused, None))
                .autofocus(widget.autofocus)
                .on_highlight_changed(self.update_material_state(
                    WidgetState::Pressed,
                    widget.on_highlight_changed.clone(),
                ))
                .on_hover(self.update_material_state(WidgetState::Hovered, None))
                .enable_feedback(widget.enable_feedback)
                .mouse_cursor(effective_mouse_cursor)
                .child(IconTheme::merge(
                    None,
                    icon_theme_data,
                    Padding::new(padding).child(center),
                ));
        if let Some(focus_node) = widget.focus_node.clone() {
            ink_well = ink_well.focus_node(focus_node);
        }
        if let Some(splash_color) = widget.splash_color {
            ink_well = ink_well.splash_color(splash_color);
        }
        if let Some(highlight_color) = widget.highlight_color {
            ink_well = ink_well.highlight_color(highlight_color);
        }
        if let Some(focus_color) = widget.focus_color {
            ink_well = ink_well.focus_color(focus_color);
        }
        if let Some(hover_color) = widget.hover_color {
            ink_well = ink_well.hover_color(hover_color);
        }
        if let Some(on_pressed) = widget.on_pressed.clone() {
            ink_well = ink_well.on_tap(on_pressed);
        }
        if let Some(on_long_press) = widget.on_long_press.clone() {
            ink_well = ink_well.on_long_press(on_long_press);
        }
        if let Some(effective_shape) = &effective_shape {
            ink_well = ink_well.custom_border(effective_shape.clone());
        }

        let mut material = Material::new()
            .elevation(self.effective_elevation(app))
            .r#type(if widget.fill_color.is_none() {
                MaterialType::Transparency
            } else {
                MaterialType::Button
            })
            .animation_duration(widget.animation_duration)
            .clip_behavior(widget.clip_behavior)
            .child(ink_well);
        if let Some(text_style) = &widget.text_style {
            let mut text_style = text_style.copy_with();
            if let Some(color) = effective_text_color {
                text_style = text_style.color(color);
            }
            material = material.text_style(text_style);
        }
        if let Some(effective_shape) = effective_shape {
            material = material.shape(effective_shape);
        }
        if let Some(fill_color) = widget.fill_color {
            material = material.color(fill_color);
        }
        // For compatibility during the M3 migration the default shadow needs to be passed.
        if theme.use_material3 {
            material = material.shadow_color(theme.shadow_color);
        }
        let result = ConstrainedBox::new(effective_constraints).child(material);

        let min_size = match widget.material_tap_target_size {
            MaterialTapTargetSize::Padded => {
                let min_size = Size::new(
                    K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dx(),
                    K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dy(),
                );
                debug_assert!(min_size.width() >= 0.0);
                debug_assert!(min_size.height() >= 0.0);
                min_size
            }
            MaterialTapTargetSize::ShrinkWrap => Size::ZERO,
        };

        Semantics::new()
            .container(true)
            .button(true)
            .enabled(widget.enabled())
            .child(InputPadding {
                child: Some(result.into_widget()),
                min_size,
            })
            .into_widget()
    }
}

/// A widget to pad the area around a `MaterialButton`'s inner [`Material`].
///
/// Redirect taps that occur in the padded area around the child to the center
/// of the child. This increases the size of the button and the button's
/// "tap target", but not its material or its ink splashes.
///
/// Dart's `_InputPadding`.
#[derive(Debug)]
struct InputPadding {
    child: Option<WidgetRef>,
    min_size: Size,
}

impl RenderObjectWidget for InputPadding {
    type RenderObject = RenderInputPadding;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderInputPadding::new(app, self.min_size, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderInputPadding>,
    ) {
        render_object.set_min_size(app, self.min_size);
    }
}

impl SingleChildRenderObjectWidget for InputPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderInputPadding`.
struct RenderInputPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    min_size: Size,
}

impl RenderInputPadding {
    fn new(
        app: &mut App,
        min_size: Size,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderInputPadding> {
        let this = RenderHandle::new_box(
            app,
            RenderInputPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                min_size,
            },
        );
        this.set_child(app, child);
        this
    }

    fn min_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).min_size
    }

    fn set_min_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.min_size(app) == value {
            return;
        }
        self.get_mut(app).min_size = value;
        self.mark_needs_layout(app);
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        if let Some(child) = self.child(app) {
            let min_size = self.min_size(app);
            let child_size = layout_child(app, child, constraints);
            let width = child_size.width().max(min_size.width());
            let height = child_size.height().max(min_size.height());
            return constraints.constrain(Size::new(width, height));
        }
        Size::ZERO
    }
}

impl RenderObjectWithChildMixin for RenderInputPadding {
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

impl RenderShiftedBox for RenderInputPadding {}

impl RenderObject for RenderInputPadding {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            let child_size = child.size(app);
            child.parent_data_of_mut::<BoxParentData>(app).offset =
                Alignment::CENTER.along_offset(size - child_size);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
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
}

impl RenderBox for RenderInputPadding {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, constraints, baseline)?;
        let child_size = child.get_dry_layout(app, constraints);
        let size = self.get_dry_layout(app, constraints);
        Some(result + Alignment::CENTER.along_offset(size - child_size).dy())
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if RenderBoxBase::hit_test(self, app, result, position) {
            return true;
        }
        let child = self.child(app).expect("a hit on the padding needs a child");
        let center = child.size(app).center(Offset::ZERO);
        result.add_with_raw_transform(Some(force_to_point(center)), center, |result, position| {
            debug_assert!(
                (position.dx() - center.dx()).abs() <= center.dx().abs() * f32::EPSILON as f64
                    && (position.dy() - center.dy()).abs()
                        <= center.dy().abs() * f32::EPSILON as f64,
                "the forced transform maps every hit to the child's center"
            );
            child.hit_test(app, result, center)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::Cell;

    use waft_embedder::{Rect, TargetPlatform, TextDirection};
    use waft_gestures::{GestureBinding, HitTestResult};
    use waft_rendering::BoxHitTestEntry;
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Actions, ActivateIntent, Builder, ColoredBox, Column, CrossAxisAlignment, Directionality,
        FocusHighlightStrategy, FocusManager, FocusNode, FocusScope, GlobalKey, MouseRegion,
        SizedBox,
    };

    use super::*;
    use crate::test_support::{Drawn, PaintInspection, WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    fn noop() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    /// `Theme(data: ThemeData(useMaterial3: false), child: Directionality(ltr, child))`.
    fn m2_ltr<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        let data = ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow());
        Theme::new(data, Directionality::new(TextDirection::Ltr, child)).into_widget()
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    fn center_of_view() -> Offset {
        WidgetTester::SIZE.center(Offset::ZERO)
    }

    fn set_highlight_strategy(tester: &WidgetTester, strategy: FocusHighlightStrategy) {
        let mut app = tester.cell.borrow_mut();
        FocusManager::instance(&mut app).set_highlight_strategy(&mut app, strategy);
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    /// Dart's `Text(text)` as its tests lay it out, in the Ahem font whose glyphs are 14
    /// pixels square. The test platform has no Ahem, so a box of its size stands in, which a
    /// pointer hits as it hits a paragraph.
    fn label(text: &str) -> SizedBox {
        SizedBox::new()
            .width(14.0 * text.chars().count() as f64)
            .height(14.0)
            .child(ColoredBox::new(Color::new(0x00000000)))
    }

    fn red_square() -> SizedBox {
        SizedBox::new()
            .width(100.0)
            .height(100.0)
            .child(ColoredBox::new(Color::new(0xffff0000)))
    }

    /// The colors of the rects the first material's ink features draw.
    fn ink_rect_colors(tester: &WidgetTester) -> Vec<u32> {
        tester
            .ink_drawn()
            .into_iter()
            .filter_map(|drawn| match drawn {
                Drawn::Rect(color, _) => Some(color),
                _ => None,
            })
            .collect()
    }

    /// The colors and bounds of the circles (paths) the first material's ink features draw.
    fn ink_circles(tester: &WidgetTester) -> Vec<(u32, Rect)> {
        tester
            .ink_drawn()
            .into_iter()
            .filter_map(|drawn| match drawn {
                Drawn::Path(color, bounds) => Some((color, bounds)),
                _ => None,
            })
            .collect()
    }

    /// Whether the first material draws a circle of the color `rgb`, whatever its alpha.
    fn has_splash(tester: &WidgetTester, rgb: u32) -> bool {
        ink_circles(tester)
            .iter()
            .any(|(color, _)| color & 0x00ff_ffff == rgb)
    }

    /// Dart's `finder.hitTestable()` for the render object of `key`'s widget: it is on the path
    /// of a hit test at its own center.
    fn is_hit_testable(tester: &WidgetTester, key: &GlobalKey) -> bool {
        let render = tester.render_object(key);
        let render_box = render.as_box().expect("a render box");
        let mut app = tester.cell.borrow_mut();
        let center =
            render_box.local_to_global(&app, render_box.size(&app).center(Offset::ZERO), None);
        let view = app.platform().views().into_iter().next().expect("one view");
        let mut result = HitTestResult::new();
        GestureBinding::instance(&mut app).hit_test_in_view(
            &mut app,
            &mut result,
            center,
            view.id(),
        );
        result.path().iter().any(|entry| {
            (entry.target() as &dyn Any)
                .downcast_ref::<BoxHitTestEntry>()
                .is_some_and(|hit| hit.target().as_object() == render)
        })
    }

    fn size_of(tester: &WidgetTester, key: &GlobalKey) -> Size {
        tester
            .render_object(key)
            .as_box()
            .expect("a render box")
            .size(&tester.cell.borrow())
    }

    fn rect_of(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render_box = tester.render_object(key).as_box().expect("a render box");
        let app = tester.cell.borrow();
        let origin = render_box.local_to_global(&app, Offset::ZERO, None);
        origin & render_box.size(&app)
    }

    #[test]
    fn raw_material_button_responds_when_tapped() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pressed = Rc::new(Cell::new(false));
            let splash_color = Color::new(0xff00ff00);
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(Some(Listener::new({
                        let pressed = Rc::clone(&pressed);
                        move |_app| pressed.set(true)
                    })))
                    .splash_color(splash_color)
                    .child(label("BUTTON")),
                ),
            ));

            tester.tap(center_of_view(), 1);
            tester.pump(Duration::from_millis(10));
            // valo records no circle of radius 0, which the splash has until its first
            // frame after this one; by then it has begun to fade.
            tester.pump(Duration::from_millis(16));

            assert!(has_splash(&tester, 0x00ff00));

            tester.pump_and_settle();

            assert!(pressed.get());
        });
    }

    /// Dart sends the shortcuts' keys; here the intent is invoked at a context under the
    /// button, where the focused `Shortcuts` would invoke it.
    #[test]
    fn raw_material_button_responds_to_shortcut_when_activated() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pressed = Rc::new(Cell::new(false));
            let inner: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let focus_node = focus_node(&tester);
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(Some(Listener::new({
                        let pressed = Rc::clone(&pressed);
                        move |_app| pressed.set(true)
                    })))
                    .splash_color(Color::new(0xff00ff00))
                    .focus_node(focus_node.clone())
                    .child(Builder::new({
                        let inner = Rc::clone(&inner);
                        move |_app, context| {
                            inner.set(Some(context));
                            SizedBox::shrink().into_widget()
                        }
                    })),
                ),
            ));
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            let context = inner.get().expect("built");

            for _ in 0..2 {
                pressed.set(false);
                Actions::invoke(&mut tester.cell.borrow_mut(), context, &ActivateIntent);
                tester.pump(Duration::from_millis(10));
                // valo records no circle of radius 0, which the splash has until its first
                // frame after this one.
                tester.pump(Duration::from_millis(16));
                assert!(has_splash(&tester, 0x00ff00));
                tester.pump_and_settle();
                assert!(pressed.get());
            }
        });
    }

    #[test]
    fn material_tap_target_size_padded_expands_hit_test_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pressed = Rc::new(Cell::new(0));
            tester.pump_widget(ltr(RawMaterialButton::new(Some(Listener::new({
                let pressed = Rc::clone(&pressed);
                move |_app| pressed.set(pressed.get() + 1)
            })))
            .constraints(BoxConstraints::tight(Size::new(10.0, 10.0)))
            .material_tap_target_size(MaterialTapTargetSize::Padded)
            .child(label("+"))));

            tester.tap(Offset::new(40.0, 400.0), 1);

            assert_eq!(pressed.get(), 1);
        });
    }

    fn splash_case(at: impl Fn(Offset) -> Offset) -> Vec<(u32, Rect)> {
        let mut tester = tester();
        let splash_color = Color::new(0xAA0000FF);
        tester.pump_widget(m2_ltr(
            &tester,
            Center::new().child(
                RawMaterialButton::new(noop())
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .fill_color(Color::new(0xFFEF5350))
                    .highlight_color(Color::new(0xAAFF0000))
                    .splash_color(splash_color)
                    .child(SizedBox::new()),
            ),
        ));
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, at(center_of_view()));
        tester.pump(Duration::ZERO); // start gesture
        tester.pump(Duration::from_millis(200)); // wait for splash to be well under way
        let circles = ink_circles(&tester);
        tester.up(&gesture);
        circles
    }

    #[test]
    fn ink_splash_from_center_tap_originates_in_correct_location() {
        with_main_thread_stack(|| {
            let circles = splash_case(|center| center);
            // centered in material button.
            assert!(
                circles.iter().any(|(color, bounds)| *color == 0xAA0000FF
                    && bounds.center() == Offset::new(44.0, 18.0)),
                "{circles:?}"
            );
        });
    }

    #[test]
    fn ink_splash_from_tap_above_material_originates_in_correct_location() {
        with_main_thread_stack(|| {
            // The top center of the button's 88x36 material, which is centered in the view.
            let circles = splash_case(|center| center - Offset::new(0.0, 18.0));
            // paints above material
            assert!(
                circles.iter().any(|(color, bounds)| *color == 0xAA0000FF
                    && bounds.center() == Offset::new(44.0, 0.0)),
                "{circles:?}"
            );
        });
    }

    /// Not in Flutter's file, whose version of this case taps where the root's tight
    /// constraints put the button anyway: a tap in the padding around a 10x10 material reaches
    /// it, and the semantics case's 48x48 size is the padding's. Like Dart's, the redirect does not
    /// check the position is in bounds.
    #[test]
    fn a_tap_in_the_padding_around_the_material_reaches_the_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let pressed = Rc::new(Cell::new(0));
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(Some(Listener::new({
                        let pressed = Rc::clone(&pressed);
                        move |_app| pressed.set(pressed.get() + 1)
                    })))
                    .key(key.clone().into())
                    .constraints(BoxConstraints::tight(Size::new(10.0, 10.0)))
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .child(label("+")),
                ),
            ));
            assert_eq!(size_of(&tester, &key), Size::new(48.0, 48.0));

            tester.tap(center_of_view() + Offset::new(20.0, 0.0), 1);
            assert_eq!(pressed.get(), 1);
        });
    }

    #[test]
    fn off_center_child_is_hit_testable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(m2_ltr(
                &tester,
                Column::new().children([RawMaterialButton::new(noop())
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .child(
                        SizedBox::square(Some(400.0)).child(
                            Column::new()
                                .main_axis_alignment(waft_rendering::MainAxisAlignment::End)
                                .children([SizedBox::new()
                                    .height(50.0)
                                    .width(400.0)
                                    .child(label("Material").key(key.clone().into()))
                                    .into_widget()]),
                        ),
                    )
                    .into_widget()]),
            ));
            assert!(is_hit_testable(&tester, &key));
        });
    }

    #[test]
    fn smaller_child_is_hit_testable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(m2_ltr(
                &tester,
                Column::new().children([RawMaterialButton::new(noop())
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .child(
                        SizedBox::new()
                            .key(key.clone().into())
                            .width(8.0)
                            .height(8.0)
                            .child(ColoredBox::new(Color::new(0xFFAABBCC))),
                    )
                    .into_widget()]),
            ));
            assert!(is_hit_testable(&tester, &key));
        });
    }

    #[test]
    fn raw_material_button_can_be_expanded_by_parent_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(m2_ltr(
                &tester,
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children([RawMaterialButton::new(noop())
                        .key(key.clone().into())
                        .child(SizedBox::new())
                        .into_widget()]),
            ));

            assert_eq!(size_of(&tester, &key), Size::new(800.0, 48.0));
        });
    }

    #[test]
    fn raw_material_button_handles_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let focus_color = Color::new(0xff00ff00);

            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(noop())
                        .focus_node(focus_node.clone())
                        .focus_color(focus_color)
                        .child(red_square()),
                ),
            ));
            assert!(!ink_rect_colors(&tester).contains(&0xff00ff00));

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            assert!(ink_rect_colors(&tester).contains(&0xff00ff00));
        });
    }

    #[test]
    fn raw_material_button_loses_focus_when_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(noop())
                        .autofocus(true)
                        .focus_node(focus_node.clone())
                        .child(red_square()),
                ),
            ));

            tester.pump(Duration::ZERO);
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(None)
                        .focus_node(focus_node.clone())
                        .child(red_square()),
                ),
            ));

            tester.pump(Duration::ZERO);
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn disabled_raw_material_button_cant_be_traversed_to() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node1 = focus_node(&tester);
            let focus_node2 = focus_node(&tester);
            tester.pump_widget(m2_ltr(
                &tester,
                FocusScope::new(
                    Center::new().child(
                        Column::new().children([
                            RawMaterialButton::new(noop())
                                .autofocus(true)
                                .focus_node(focus_node1.clone())
                                .child(red_square())
                                .into_widget(),
                            RawMaterialButton::new(None)
                                .autofocus(true)
                                .focus_node(focus_node2.clone())
                                .child(red_square())
                                .into_widget(),
                        ]),
                    ),
                ),
            ));
            tester.pump(Duration::ZERO);

            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));

            assert!(!focus_node1.next_focus(&mut tester.cell.borrow_mut()));
            tester.pump(Duration::ZERO);

            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn raw_material_button_handles_hover() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let hover_color = Color::new(0xff00ff00);

            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    RawMaterialButton::new(noop())
                        .key(key.clone().into())
                        .hover_color(hover_color)
                        .hover_elevation(10.5)
                        .child(red_square()),
                ),
            ));
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            assert!(!ink_rect_colors(&tester).contains(&0xff00ff00));

            tester.move_to(&mut mouse, tester.center_of(&key), false);
            tester.pump_and_settle();

            assert!(ink_rect_colors(&tester).contains(&0xff00ff00));
        });
    }

    #[test]
    fn raw_material_button_on_pressed_and_on_long_press_callbacks_are_correctly_called_when_non_null()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let was_pressed = Rc::new(Cell::new(false));
            let was_pressed_listener = || {
                let was_pressed = Rc::clone(&was_pressed);
                Listener::new(move |_app| was_pressed.set(true))
            };
            // Returns the button's `enabled`, which Dart reads off `tester.widget`.
            let build_frame = |tester: &mut WidgetTester,
                               on_pressed: Option<Listener>,
                               on_long_press: Option<Listener>|
             -> bool {
                let mut button = RawMaterialButton::new(on_pressed)
                    .key(key.clone().into())
                    .child(label("button"));
                button.on_long_press = on_long_press;
                let enabled = button.enabled();
                tester.pump_widget(ltr(button));
                enabled
            };

            // onPressed not null, onLongPress null.
            was_pressed.set(false);
            assert!(build_frame(&mut tester, Some(was_pressed_listener()), None));
            tester.tap(tester.center_of(&key), 1);
            assert!(was_pressed.get());

            // onPressed null, onLongPress not null.
            was_pressed.set(false);
            assert!(build_frame(&mut tester, None, Some(was_pressed_listener())));
            tester.long_press(tester.center_of(&key), 2);
            assert!(was_pressed.get());

            // onPressed null, onLongPress null.
            assert!(!build_frame(&mut tester, None, None));
        });
    }

    #[test]
    fn raw_material_button_on_pressed_and_on_long_press_callbacks_are_distinctly_recognized() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let did_press_button = Rc::new(Cell::new(false));
            let did_long_press_button = Rc::new(Cell::new(false));
            tester.pump_widget(ltr(RawMaterialButton::new(Some(Listener::new({
                let did_press_button = Rc::clone(&did_press_button);
                move |_app| did_press_button.set(true)
            })))
            .key(key.clone().into())
            .on_long_press(Listener::new({
                let did_long_press_button = Rc::clone(&did_long_press_button);
                move |_app| did_long_press_button.set(true)
            }))
            .child(label("button"))));

            assert!(!did_press_button.get());
            tester.tap(tester.center_of(&key), 1);
            assert!(did_press_button.get());

            assert!(!did_long_press_button.get());
            tester.long_press(tester.center_of(&key), 2);
            assert!(did_long_press_button.get());
        });
    }

    #[test]
    fn raw_material_button_responds_to_density_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let child_key = GlobalKey::new();

            let build_test =
                |tester: &mut WidgetTester, visual_density: VisualDensity, use_text: bool| {
                    let child: WidgetRef = if use_text {
                        label("Text").key(child_key.clone().into()).into_widget()
                    } else {
                        SizedBox::new()
                            .key(child_key.clone().into())
                            .width(100.0)
                            .height(100.0)
                            .child(ColoredBox::new(Color::new(0xffff0000)))
                            .into_widget()
                    };
                    let data = ThemeData::builder()
                        .use_material3(false)
                        .build(&tester.cell.borrow());
                    tester.pump_widget(
                        Theme::new(
                            data,
                            Directionality::new(
                                TextDirection::Rtl,
                                Center::new().child(
                                    RawMaterialButton::new(noop())
                                        .visual_density(visual_density)
                                        .key(key.clone().into())
                                        .child(child),
                                ),
                            ),
                        )
                        .into_widget(),
                    );
                };

            build_test(&mut tester, VisualDensity::STANDARD, false);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(100.0, 100.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(350.0, 250.0, 450.0, 350.0)
            );

            build_test(&mut tester, VisualDensity::new(3.0, 3.0), false);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(124.0, 124.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(350.0, 250.0, 450.0, 350.0)
            );

            build_test(&mut tester, VisualDensity::new(-3.0, -3.0), false);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(100.0, 100.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(350.0, 250.0, 450.0, 350.0)
            );

            build_test(&mut tester, VisualDensity::STANDARD, true);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(88.0, 48.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );

            build_test(&mut tester, VisualDensity::new(3.0, 3.0), true);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(100.0, 60.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );

            build_test(&mut tester, VisualDensity::new(-3.0, -3.0), true);
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, &key), Size::new(76.0, 36.0));
            assert_eq!(
                rect_of(&tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );
        });
    }

    fn active_cursor(tester: &WidgetTester) -> MouseCursorRef {
        let mut app = tester.cell.borrow_mut();
        let tracker = waft_rendering::RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, 1)
            .expect("the mouse is over the view")
    }

    #[test]
    fn raw_material_button_changes_mouse_cursor_as_expected_when_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let in_region = |child: RawMaterialButton| {
                ltr(MouseRegion::new()
                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                    .child(child))
            };
            tester.pump_widget(in_region(
                RawMaterialButton::new(noop()).mouse_cursor(SystemMouseCursors::TEXT.into()),
            ));
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.pump(Duration::ZERO);

            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            assert!(*active_cursor(&tester) == *text);

            // Test default cursor
            tester.pump_widget(in_region(RawMaterialButton::new(noop())));
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            assert!(*active_cursor(&tester) == *basic);

            // Test default cursor when disabled
            tester.pump_widget(in_region(RawMaterialButton::new(None)));
            assert!(*active_cursor(&tester) == *basic);
        });
    }

    /// Not in Flutter's file: `resolve_shape` stands for its `resolveAs<ShapeBorder?>`.
    #[test]
    fn a_widget_state_outlined_border_resolves_to_the_border_of_the_states() {
        let enabled = RoundedRectangleBorder::default();
        let disabled = RoundedRectangleBorder {
            border_radius: waft_painting::BorderRadius::circular(8.0).into(),
            ..RoundedRectangleBorder::default()
        };
        let property = waft_widgets::WidgetStateOutlinedBorder::resolve_with({
            move |states| {
                let border = if states.contains(&WidgetState::Disabled) {
                    disabled
                } else {
                    enabled
                };
                Some(OutlinedBorderRef::new(border))
            }
        });

        let resolved = |shape: &ShapeBorderRef, states: &[WidgetState]| {
            resolve_shape(shape, &states.iter().copied().collect()).expect("a border")
        };

        let property = ShapeBorderRef::new(property);
        assert!(resolved(&property, &[]).eq_shape(&enabled));
        assert!(resolved(&property, &[WidgetState::Disabled]).eq_shape(&disabled));
        // A border that is not a property is its own value.
        let disabled = ShapeBorderRef::new(disabled);
        assert_eq!(resolved(&disabled, &[]), disabled);
    }

    #[test]
    fn raw_material_button_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(m2_ltr(
                &tester,
                Center::new().child(
                    SizedBox::shrink()
                        .child(RawMaterialButton::new(noop()).key(key.clone().into())),
                ),
            ));
            assert_eq!(size_of(&tester, &key), Size::ZERO);
        });
    }
}

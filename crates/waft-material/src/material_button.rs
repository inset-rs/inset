//! Flutter counterpart: material_ui's `material_button.dart`.

use std::fmt;
use std::time::Duration;

use waft_embedder::{Brightness, Clip, Color};
use waft_foundation::{App, Listener, ValueChanged};
use waft_painting::EdgeInsetsGeometry;
use waft_services::MouseCursorRef;
use waft_widgets::{BuildContext, FocusNodeRef, IntoWidget, KeyRef, StatelessWidget, WidgetRef};

use crate::button::RawMaterialButton;
use crate::button_theme::{ButtonTextTheme, ButtonTheme};
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// A Material Design button.
///
/// To create a custom Material button consider using `TextButton`, `ElevatedButton`, or
/// `OutlinedButton`.
///
/// The [`elevation`](Self::elevation), [`hover_elevation`](Self::hover_elevation),
/// [`focus_elevation`](Self::focus_elevation), [`highlight_elevation`](Self::highlight_elevation)
/// and [`disabled_elevation`](Self::disabled_elevation) arguments must be non-negative, if
/// specified.
pub struct MaterialButton {
    pub key: Option<KeyRef>,
    /// The callback that is called when the button is tapped or otherwise activated.
    ///
    /// If this callback and [`on_long_press`](Self::on_long_press) are null, then the button
    /// will be disabled.
    pub on_pressed: Option<Listener>,
    /// The callback that is called when the button is long-pressed.
    pub on_long_press: Option<Listener>,
    /// Called by the underlying `InkWell` widget's `onHighlightChanged` callback.
    pub on_highlight_changed: Option<ValueChanged<bool>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the button.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// Defines the button's base colors, and the defaults for the button's minimum size,
    /// internal padding, and shape.
    pub text_theme: Option<ButtonTextTheme>,
    /// The color to use for this button's text.
    pub text_color: Option<Color>,
    /// The color to use for this button's text when the button is disabled.
    pub disabled_text_color: Option<Color>,
    /// The button's fill color while it is in its default (unpressed, enabled) state.
    pub color: Option<Color>,
    /// The fill color of the button when the button is disabled.
    pub disabled_color: Option<Color>,
    /// The splash color of the button's `InkWell`.
    pub splash_color: Option<Color>,
    /// The fill color of the button's `Material` when it has the input focus.
    pub focus_color: Option<Color>,
    /// The fill color of the button's `Material` when a pointer is hovering over it.
    pub hover_color: Option<Color>,
    /// The highlight color of the button's `InkWell`.
    pub highlight_color: Option<Color>,
    /// The z-coordinate at which to place this button relative to its parent.
    pub elevation: Option<f64>,
    /// The elevation for the button's `Material` when the button is enabled and a pointer is
    /// hovering over it.
    pub hover_elevation: Option<f64>,
    /// The elevation for the button's `Material` when the button is enabled and has the input
    /// focus.
    pub focus_elevation: Option<f64>,
    /// The elevation for the button's `Material` relative to its parent when the button is
    /// enabled and pressed.
    pub highlight_elevation: Option<f64>,
    /// The elevation for the button's `Material` relative to its parent when the button is not
    /// enabled.
    pub disabled_elevation: Option<f64>,
    /// The theme brightness to use for this button.
    pub color_brightness: Option<Brightness>,
    /// The button's label.
    pub child: Option<WidgetRef>,
    /// The internal padding for the button's child.
    pub padding: Option<EdgeInsetsGeometry>,
    /// Defines how compact the button's layout will be.
    pub visual_density: Option<VisualDensity>,
    /// The shape of the button's `Material`.
    pub shape: Option<waft_painting::ShapeBorderRef>,
    /// How to clip the button's `Material`.
    pub clip_behavior: Clip,
    /// The focus node of the button.
    pub focus_node: Option<FocusNodeRef>,
    /// Whether the button will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,
    /// The duration of animated changes for shape and elevation.
    pub animation_duration: Option<Duration>,
    /// Configures the minimum size of the tap target.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// The smallest horizontal extent that the button will occupy.
    pub min_width: Option<f64>,
    /// The vertical extent of the button.
    pub height: Option<f64>,
    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    pub enable_feedback: bool,
}

impl MaterialButton {
    /// Creates a Material Design button; Dart's optional named arguments are the setters.
    pub fn new(on_pressed: Option<Listener>) -> MaterialButton {
        MaterialButton {
            key: None,
            on_pressed,
            on_long_press: None,
            on_highlight_changed: None,
            mouse_cursor: None,
            text_theme: None,
            text_color: None,
            disabled_text_color: None,
            color: None,
            disabled_color: None,
            splash_color: None,
            focus_color: None,
            hover_color: None,
            highlight_color: None,
            elevation: None,
            hover_elevation: None,
            focus_elevation: None,
            highlight_elevation: None,
            disabled_elevation: None,
            color_brightness: None,
            child: None,
            padding: None,
            visual_density: None,
            shape: None,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            animation_duration: None,
            material_tap_target_size: None,
            min_width: None,
            height: None,
            enable_feedback: true,
        }
    }

    /// Whether the button is enabled or disabled.
    pub fn enabled(&self) -> bool {
        self.on_pressed.is_some() || self.on_long_press.is_some()
    }

    /// Dart `MaterialButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> MaterialButton {
        self.key = Some(key);
        self
    }

    /// Dart `MaterialButton(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: Listener) -> MaterialButton {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `MaterialButton(onHighlightChanged:)`.
    pub fn on_highlight_changed(
        mut self,
        on_highlight_changed: ValueChanged<bool>,
    ) -> MaterialButton {
        self.on_highlight_changed = Some(on_highlight_changed);
        self
    }

    /// Dart `MaterialButton(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> MaterialButton {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `MaterialButton(textTheme:)`.
    pub fn text_theme(mut self, text_theme: ButtonTextTheme) -> MaterialButton {
        self.text_theme = Some(text_theme);
        self
    }

    /// Dart `MaterialButton(textColor:)`.
    pub fn text_color(mut self, text_color: Color) -> MaterialButton {
        self.text_color = Some(text_color);
        self
    }

    /// Dart `MaterialButton(disabledTextColor:)`.
    pub fn disabled_text_color(mut self, disabled_text_color: Color) -> MaterialButton {
        self.disabled_text_color = Some(disabled_text_color);
        self
    }

    /// Dart `MaterialButton(color:)`.
    pub fn color(mut self, color: Color) -> MaterialButton {
        self.color = Some(color);
        self
    }

    /// Dart `MaterialButton(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> MaterialButton {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `MaterialButton(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> MaterialButton {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `MaterialButton(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> MaterialButton {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `MaterialButton(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> MaterialButton {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `MaterialButton(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> MaterialButton {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `MaterialButton(colorBrightness:)`.
    pub fn color_brightness(mut self, color_brightness: Brightness) -> MaterialButton {
        self.color_brightness = Some(color_brightness);
        self
    }

    /// Dart `MaterialButton(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> MaterialButton {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `MaterialButton(focusElevation:)`.
    pub fn focus_elevation(mut self, focus_elevation: f64) -> MaterialButton {
        debug_assert!(focus_elevation >= 0.0);
        self.focus_elevation = Some(focus_elevation);
        self
    }

    /// Dart `MaterialButton(hoverElevation:)`.
    pub fn hover_elevation(mut self, hover_elevation: f64) -> MaterialButton {
        debug_assert!(hover_elevation >= 0.0);
        self.hover_elevation = Some(hover_elevation);
        self
    }

    /// Dart `MaterialButton(highlightElevation:)`.
    pub fn highlight_elevation(mut self, highlight_elevation: f64) -> MaterialButton {
        debug_assert!(highlight_elevation >= 0.0);
        self.highlight_elevation = Some(highlight_elevation);
        self
    }

    /// Dart `MaterialButton(disabledElevation:)`.
    pub fn disabled_elevation(mut self, disabled_elevation: f64) -> MaterialButton {
        debug_assert!(disabled_elevation >= 0.0);
        self.disabled_elevation = Some(disabled_elevation);
        self
    }

    /// Dart `MaterialButton(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> MaterialButton {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `MaterialButton(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> MaterialButton {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `MaterialButton(shape:)`.
    pub fn shape(mut self, shape: impl Into<waft_painting::ShapeBorderRef>) -> MaterialButton {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `MaterialButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> MaterialButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `MaterialButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> MaterialButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `MaterialButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> MaterialButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `MaterialButton(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> MaterialButton {
        self.animation_duration = Some(animation_duration);
        self
    }

    /// Dart `MaterialButton(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> MaterialButton {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `MaterialButton(minWidth:)`.
    pub fn min_width(mut self, min_width: f64) -> MaterialButton {
        self.min_width = Some(min_width);
        self
    }

    /// Dart `MaterialButton(height:)`.
    pub fn height(mut self, height: f64) -> MaterialButton {
        self.height = Some(height);
        self
    }

    /// Dart `MaterialButton(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> MaterialButton {
        self.enable_feedback = enable_feedback;
        self
    }

    /// Dart `MaterialButton(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MaterialButton {
        self.child = Some(child.into_widget());
        self
    }
}

impl fmt::Debug for MaterialButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialButton")
            .field("enabled", &self.enabled())
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for MaterialButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let button_theme = ButtonTheme::of(app, context);

        let text_style = theme
            .text_theme
            .label_large
            .clone()
            .expect("the text theme has a label large")
            .copy_with()
            .color(button_theme.get_text_color(self));

        let mut constraints = button_theme.get_constraints(self);
        if let Some(min_width) = self.min_width {
            constraints = constraints.min_width(min_width);
        }
        if let Some(height) = self.height {
            constraints = constraints.min_height(height);
        }

        let mut button = RawMaterialButton::new(self.on_pressed.clone())
            .enable_feedback(self.enable_feedback)
            .text_style(text_style)
            .focus_color(
                self.focus_color
                    .unwrap_or_else(|| button_theme.get_focus_color(self)),
            )
            .hover_color(
                self.hover_color
                    .unwrap_or_else(|| button_theme.get_hover_color(self)),
            )
            .highlight_color(self.highlight_color.unwrap_or(theme.highlight_color))
            .splash_color(self.splash_color.unwrap_or(theme.splash_color))
            .elevation(button_theme.get_elevation(self))
            .focus_elevation(button_theme.get_focus_elevation(self))
            .hover_elevation(button_theme.get_hover_elevation(self))
            .highlight_elevation(button_theme.get_highlight_elevation(self))
            .padding(button_theme.get_padding(self))
            .visual_density(self.visual_density.unwrap_or(theme.visual_density))
            .constraints(constraints)
            .shape(button_theme.get_shape(self))
            .clip_behavior(self.clip_behavior)
            .autofocus(self.autofocus)
            .animation_duration(button_theme.get_animation_duration(self))
            .material_tap_target_size(
                self.material_tap_target_size
                    .unwrap_or(theme.material_tap_target_size),
            )
            .disabled_elevation(self.disabled_elevation.unwrap_or(0.0));

        if let Some(on_long_press) = self.on_long_press.clone() {
            button = button.on_long_press(on_long_press);
        }
        if let Some(on_highlight_changed) = self.on_highlight_changed.clone() {
            button = button.on_highlight_changed(on_highlight_changed);
        }
        if let Some(mouse_cursor) = self.mouse_cursor.clone() {
            button = button.mouse_cursor(mouse_cursor);
        }
        if let Some(fill_color) = button_theme.get_fill_color(self) {
            button = button.fill_color(fill_color);
        }
        if let Some(focus_node) = self.focus_node.clone() {
            button = button.focus_node(focus_node);
        }
        if let Some(child) = self.child.clone() {
            button = button.child(child);
        }
        button.into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::{Clip, FontWeight, Offset, Radius, Rect, Size, TargetPlatform};
    use waft_foundation::{K_IS_WEB, ValueKey};
    use waft_painting::{
        BorderRadius, BorderSide, EdgeInsets, RoundedRectangleBorder, ShapeBorder,
    };
    use waft_rendering::testing::{CanvasMethod, PaintCall, assert_paints, count_calls};
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};
    use waft_widgets::{
        Center, Container, Directionality, FocusHighlightStrategy, FocusNode, IntoWidget,
        MouseRegion, SizedBox, Text, TextDirection,
    };

    use super::*;
    use crate::ink_well::InkWell;
    use crate::material::{Material, MaterialType};
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    /// Dart's `ThemeData(useMaterial3: false)` on the test platform.
    fn theme(app: &App) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .platform(TargetPlatform::Android)
            .build(app)
    }

    fn noop() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn button(on_pressed: bool) -> MaterialButton {
        MaterialButton::new(on_pressed.then(|| Listener::new(|_app| {}))).child(Text::new("button"))
    }

    /// Dart's `pumpWidget(Directionality(textDirection: ltr, child: button))`: no `Material`
    /// above it, so the tester's first ink features are the button's own.
    fn pump_bare(tester: &mut WidgetTester, button: MaterialButton) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Directionality::new(TextDirection::Ltr, button).into_widget(),
        ));
    }

    /// Dart's `pumpWidget(Theme(data: ThemeData(useMaterial3: false), child: Directionality(
    /// textDirection: ltr, child: child)))`.
    fn pump_themed<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(
            Theme::new(theme, Directionality::new(TextDirection::Ltr, child)).into_widget(),
        );
    }

    fn button_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget(
            find::descendant(
                find::by_type::<MaterialButton>(),
                find::by_type::<Material>(),
            ),
            read,
        )
    }

    fn button_elevation(tester: &WidgetTester) -> f64 {
        button_material(tester, |material| material.elevation)
    }

    fn raw_button_disabled_elevation(tester: &WidgetTester) -> f64 {
        tester.widget(
            find::descendant(
                find::by_type::<MaterialButton>(),
                find::by_type::<RawMaterialButton>(),
            ),
            |button: &RawMaterialButton| button.disabled_elevation,
        )
    }

    fn enabled(tester: &WidgetTester) -> bool {
        tester.widget(
            find::by_type::<MaterialButton>(),
            |button: &MaterialButton| button.enabled(),
        )
    }

    /// The `Material` checks every step of `MaterialButton defaults` repeats.
    fn assert_default_material(tester: &WidgetTester, elevation: f64, text_color: Color) {
        let shape = RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(2.0)).into(),
        );
        button_material(tester, |material| {
            assert_eq!(material.animation_duration, Duration::from_millis(200));
            assert!(material.border_on_foreground);
            assert_eq!(material.border_radius, None);
            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.color, None);
            assert_eq!(material.elevation, elevation);
            assert_eq!(material.shadow_color, None);
            assert!(material.shape.as_deref() == Some(&shape as &dyn ShapeBorder));
            let text_style = material.text_style.as_ref().expect("a text style");
            assert_eq!(
                text_style.color.as_ref().map(|color| color.color()),
                Some(text_color)
            );
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.font_size, Some(14.0));
            assert_eq!(text_style.font_weight, Some(FontWeight::W500));
            assert_eq!(material.r#type, MaterialType::Transparency);
        });
    }

    #[test]
    fn material_button_defaults() {
        test_widgets(|tester| {
            // Enabled MaterialButton
            pump_themed(tester, button(true));
            assert_default_material(tester, 2.0, Color::new(0xdd000000));

            let center = tester.get_center(find::by_type::<MaterialButton>());
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            tester.pump_and_settle();

            // Only elevation changes when enabled and pressed.
            assert_default_material(tester, 8.0, Color::new(0xdd000000));

            // Disabled MaterialButton
            pump_themed(tester, button(false));
            assert_default_material(tester, 0.0, Color::new(0x61000000));

            // Finish gesture to release resources.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn does_material_button_work_with_hover() {
        test_widgets(|tester| {
            let hover_color = Color::new(0xff001122);
            pump_bare(tester, button(true).hover_color(hover_color));

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(
                &mut gesture,
                tester.get_center(find::by_type::<MaterialButton>()),
                false,
            );
            tester.pump_and_settle();

            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(hover_color))],
            );
        });
    }

    #[test]
    fn does_material_button_work_with_focus() {
        test_widgets(|tester| {
            let focus_color = Color::new(0xff001122);
            let focus_node: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            pump_bare(
                tester,
                button(true)
                    .focus_color(focus_color)
                    .focus_node(focus_node.clone()),
            );

            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(focus_color))],
            );
        });
    }

    #[test]
    fn material_button_elevation_and_colors_have_proper_precedence() {
        test_widgets(|tester| {
            let elevation = 10.0;
            let focus_elevation = 11.0;
            let hover_elevation = 12.0;
            let highlight_elevation = 13.0;
            let focus_color = Color::new(0xff001122);
            let hover_color = Color::new(0xff112233);
            let highlight_color = Color::new(0xff223344);

            let focus_node: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            pump_bare(
                tester,
                button(true)
                    .focus_color(focus_color)
                    .hover_color(hover_color)
                    .highlight_color(highlight_color)
                    .elevation(elevation)
                    .focus_elevation(focus_elevation)
                    .hover_elevation(hover_elevation)
                    .highlight_elevation(highlight_elevation)
                    .focus_node(focus_node.clone()),
            );
            tester.pump_and_settle();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);

            // Base elevation
            assert_eq!(button_elevation(tester), elevation);

            // Focus elevation overrides base
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(focus_color))],
            );
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_eq!(button_elevation(tester), focus_elevation);

            // Hover elevation overrides focus
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(
                &mut gesture,
                tester.get_center(find::by_type::<MaterialButton>()),
                false,
            );
            tester.pump_and_settle();
            assert_paints(
                tester.ink_list().ops(),
                &[
                    PaintCall::rect(Some(focus_color)),
                    PaintCall::rect(Some(hover_color)),
                ],
            );
            assert_eq!(button_elevation(tester), hover_elevation);
            tester.remove_pointer(&gesture);

            // Highlight elevation overrides hover
            let mut gesture2 = TestPointer::touch(2);
            tester.down(
                &mut gesture2,
                tester.get_center(find::by_type::<MaterialButton>()),
            );
            tester.pump_and_settle();
            assert_paints(
                tester.ink_list().ops(),
                &[
                    PaintCall::rect(Some(focus_color)),
                    PaintCall::rect(Some(highlight_color)),
                ],
            );
            assert_eq!(button_elevation(tester), highlight_elevation);
            tester.up(&gesture2);
        });
    }

    #[test]
    fn material_button_s_disabled_color_takes_precedence_over_its_default_disabled_color() {
        test_widgets(|tester| {
            pump_bare(tester, button(false).disabled_color(Color::new(0xff00ff00)));
            assert_eq!(
                button_material(tester, |material| material.color),
                Some(Color::new(0xff00ff00))
            );
        });
    }

    #[test]
    fn material_button_gets_focus_when_autofocus_is_set() {
        test_widgets(|tester| {
            let focus_node: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            let pump = |tester: &mut WidgetTester, autofocus: bool| {
                let theme = ThemeData::builder().build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            MaterialButton::new(noop())
                                .autofocus(autofocus)
                                .focus_node(focus_node.clone())
                                .child(
                                    Container::new()
                                        .width(100.0)
                                        .height(100.0)
                                        .color(Color::new(0xffff0000)),
                                ),
                        )
                        .into_widget(),
                ));
            };

            pump(tester, false);
            tester.pump();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));

            pump(tester, true);
            tester.pump();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn material_button_on_pressed_and_on_long_press_callbacks_are_correctly_called_when_non_null() {
        test_widgets(|tester| {
            let build_frame = |on_pressed: Option<Listener>, on_long_press: Option<Listener>| {
                let mut button = MaterialButton::new(on_pressed).child(Text::new("button"));
                if let Some(on_long_press) = on_long_press {
                    button = button.on_long_press(on_long_press);
                }
                Directionality::new(TextDirection::Ltr, button)
            };
            let was_pressed = Rc::new(Cell::new(false));
            let press = || {
                let was_pressed = Rc::clone(&was_pressed);
                Listener::new(move |_app| was_pressed.set(true))
            };

            // onPressed not null, onLongPress null.
            pump_themed(tester, build_frame(Some(press()), None));
            assert!(enabled(tester));
            tester.tap(find::by_type::<MaterialButton>());
            assert!(was_pressed.get());

            // onPressed null, onLongPress not null.
            was_pressed.set(false);
            pump_themed(tester, build_frame(None, Some(press())));
            assert!(enabled(tester));
            tester.long_press(find::by_type::<MaterialButton>());
            assert!(was_pressed.get());

            // onPressed null, onLongPress null.
            pump_themed(tester, build_frame(None, None));
            assert!(!enabled(tester));
        });
    }

    #[test]
    fn material_button_on_pressed_and_on_long_press_callbacks_are_distinctly_recognized() {
        test_widgets(|tester| {
            let did_press_button = Rc::new(Cell::new(false));
            let did_long_press_button = Rc::new(Cell::new(false));
            let pressed = Rc::clone(&did_press_button);
            let long_pressed = Rc::clone(&did_long_press_button);
            pump_themed(
                tester,
                MaterialButton::new(Some(Listener::new(move |_app| pressed.set(true))))
                    .on_long_press(Listener::new(move |_app| long_pressed.set(true)))
                    .child(Text::new("button")),
            );

            assert!(enabled(tester));

            assert!(!did_press_button.get());
            tester.tap(find::by_type::<MaterialButton>());
            assert!(did_press_button.get());

            assert!(!did_long_press_button.get());
            tester.long_press(find::by_type::<MaterialButton>());
            assert!(did_long_press_button.get());
        });
    }

    #[test]
    fn material_button_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            let in_region = |button: MaterialButton| {
                MouseRegion::new()
                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                    .child(button)
            };
            pump_themed(
                tester,
                in_region(
                    MaterialButton::new(noop()).mouse_cursor(SystemMouseCursors::TEXT.into()),
                ),
            );

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump();

            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            assert!(*tester.active_cursor(1) == *text);

            // Test default cursor
            pump_themed(tester, in_region(MaterialButton::new(noop())));
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert!(*tester.active_cursor(1) == *expected);

            // Test default cursor when disabled
            pump_themed(tester, in_region(MaterialButton::new(None)));
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            assert!(*tester.active_cursor(1) == *basic);
        });
    }

    /// Dart matches the clip's path against the button's rounded rect; the recorded clip keeps
    /// no path, so this only checks that one is there.
    #[test]
    fn material_button_with_explicit_splash_color_and_highlight_color() {
        test_widgets(|tester| {
            let direct_splash_color = Color::new(0xFF000011);
            let direct_highlight_color = Color::new(0xFF000011);
            let themed = |tester: &WidgetTester,
                          highlight_color: Option<Color>,
                          splash_color: Option<Color>,
                          button_widget: WidgetRef| {
                let mut builder = ThemeData::builder()
                    .use_material3(false)
                    .platform(TargetPlatform::Android)
                    .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap);
                if let Some(highlight_color) = highlight_color {
                    builder = builder.highlight_color(highlight_color);
                }
                if let Some(splash_color) = splash_color {
                    builder = builder.splash_color(splash_color);
                }
                Directionality::new(
                    TextDirection::Ltr,
                    Theme::new(builder.build(&tester.cell.borrow()), button_widget),
                )
                .into_widget()
            };
            let ink_well_material = |tester: &WidgetTester| {
                tester.nearest_material_ops(tester.element(find::by_type::<InkWell>()))
            };

            let button_widget = Center::new()
                .child(
                    MaterialButton::new(noop())
                        .splash_color(direct_splash_color)
                        .highlight_color(direct_highlight_color)
                        .clip_behavior(Clip::AntiAlias),
                )
                .into_widget();
            let widget = themed(tester, None, None, button_widget);
            tester.pump_widget(widget);

            let center = tester.get_center(find::by_type::<MaterialButton>());
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            tester.pump(); // start gesture
            tester.pump_by(Duration::from_millis(200)); // wait for splash to be well under way

            // Painter is translated to the center by the Center widget and not
            // the Material widget.
            let expected_clip_rect = Rect::from_ltrb(0.0, 0.0, 88.0, 36.0);
            assert_eq!(
                tester.get_size(find::by_type::<MaterialButton>()),
                expected_clip_rect.size()
            );
            assert_paints(
                &ink_well_material(tester),
                &[
                    PaintCall::Clip,
                    PaintCall::circle(Some(direct_splash_color)),
                    PaintCall::rect(Some(direct_highlight_color)),
                ],
            );

            let theme_splash_color1 = Color::new(0xFF001100);
            let theme_highlight_color1 = Color::new(0xFF001100);

            let button_widget = Center::new()
                .child(MaterialButton::new(noop()).clip_behavior(Clip::AntiAlias))
                .into_widget();
            let widget = themed(
                tester,
                Some(theme_highlight_color1),
                Some(theme_splash_color1),
                button_widget.clone(),
            );
            tester.pump_widget(widget);

            assert_paints(
                &ink_well_material(tester),
                &[
                    PaintCall::Clip,
                    PaintCall::circle(Some(theme_splash_color1)),
                    PaintCall::rect(Some(theme_highlight_color1)),
                ],
            );

            let theme_splash_color2 = Color::new(0xFF002200);
            let theme_highlight_color2 = Color::new(0xFF002200);

            // same widget, so does not get updated because of us
            let widget = themed(
                tester,
                Some(theme_highlight_color2),
                Some(theme_splash_color2),
                button_widget,
            );
            tester.pump_widget(widget);

            assert_paints(
                &ink_well_material(tester),
                &[
                    PaintCall::circle(Some(theme_splash_color2)),
                    PaintCall::rect(Some(theme_highlight_color2)),
                ],
            );

            tester.up(&gesture);
        });
    }

    #[test]
    fn material_button_has_no_clip_by_default() {
        test_widgets(|tester| {
            let button_key = KeyRef::from(ValueKey::new("button"));
            let theme = ThemeData::builder()
                .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                .build(&tester.cell.borrow());
            tester.pump_widget(
                Directionality::new(
                    TextDirection::Ltr,
                    Theme::new(
                        theme,
                        Center::new().child(MaterialButton::new(noop()).key(button_key.clone())),
                    ),
                )
                .into_widget(),
            );

            let render = tester.render_object(find::by_key(button_key));
            assert_eq!(
                count_calls(&tester.painted_ops_deep(render), CanvasMethod::Clip),
                0
            );
        });
    }

    #[test]
    fn material_button_min_width_and_height_parameters() {
        test_widgets(|tester| {
            let build_frame = |min_width: Option<f64>,
                               height: Option<f64>,
                               padding: EdgeInsets,
                               child: Option<WidgetRef>| {
                let mut button = MaterialButton::new(None)
                    .padding(padding)
                    .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap);
                if let Some(min_width) = min_width {
                    button = button.min_width(min_width);
                }
                if let Some(height) = height {
                    button = button.height(height);
                }
                if let Some(child) = child {
                    button = button.child(child);
                }
                Center::new().child(button)
            };
            let eight = || Some(SizedBox::new().width(8.0).height(8.0).into_widget());
            let cases = [
                (
                    Some(8.0),
                    Some(24.0),
                    EdgeInsets::ZERO,
                    None,
                    Size::new(8.0, 24.0),
                ),
                // Default minHeight constraint is 36, see RawMaterialButton.
                (
                    Some(8.0),
                    None,
                    EdgeInsets::ZERO,
                    None,
                    Size::new(8.0, 36.0),
                ),
                // Default minWidth constraint is 88, see RawMaterialButton.
                (
                    None,
                    Some(8.0),
                    EdgeInsets::ZERO,
                    None,
                    Size::new(88.0, 8.0),
                ),
                (None, None, EdgeInsets::ZERO, None, Size::new(88.0, 36.0)),
                (
                    None,
                    None,
                    EdgeInsets::all(4.0),
                    None,
                    Size::new(88.0, 36.0),
                ),
                // Size is defined by the padding.
                (
                    Some(0.0),
                    Some(0.0),
                    EdgeInsets::all(4.0),
                    None,
                    Size::new(8.0, 8.0),
                ),
                // Size is defined by the padded child.
                (
                    Some(0.0),
                    Some(0.0),
                    EdgeInsets::all(4.0),
                    eight(),
                    Size::new(16.0, 16.0),
                ),
                // Size is defined by the minWidth, height constraints.
                (
                    Some(18.0),
                    Some(18.0),
                    EdgeInsets::all(4.0),
                    eight(),
                    Size::new(18.0, 18.0),
                ),
            ];
            for (min_width, height, padding, child, expected) in cases {
                pump_themed(tester, build_frame(min_width, height, padding, child));
                assert_eq!(tester.get_size(find::by_type::<MaterialButton>()), expected);
            }
        });
    }

    #[test]
    fn material_button_size_is_configurable_by_theme_data_material_tap_target_size() {
        test_widgets(|tester| {
            let pump = |tester: &mut WidgetTester, key: KeyRef, size: MaterialTapTargetSize| {
                let theme = ThemeData::builder()
                    .material_tap_target_size(size)
                    .build(&tester.cell.borrow());
                tester.pump_widget(
                    Theme::new(
                        theme,
                        Directionality::new(
                            TextDirection::Ltr,
                            Center::new().child(
                                MaterialButton::new(noop())
                                    .key(key)
                                    .child(SizedBox::new().width(50.0).height(8.0)),
                            ),
                        ),
                    )
                    .into_widget(),
                );
            };

            let key1 = KeyRef::from(ValueKey::new("key1"));
            pump(tester, key1.clone(), MaterialTapTargetSize::Padded);
            assert_eq!(tester.get_size(find::by_key(key1)), Size::new(88.0, 48.0));

            let key2 = KeyRef::from(ValueKey::new("key2"));
            pump(tester, key2.clone(), MaterialTapTargetSize::ShrinkWrap);
            assert_eq!(tester.get_size(find::by_key(key2)), Size::new(88.0, 36.0));
        });
    }

    #[test]
    fn material_button_shape_overrides_button_theme_shape() {
        test_widgets(|tester| {
            let shape = waft_painting::StadiumBorder::new(BorderSide::NONE);
            pump_bare(tester, button(true).shape(shape));
            assert!(button_material(tester, |material| {
                material.shape.as_deref() == Some(&shape as &dyn ShapeBorder)
            }));
        });
    }

    #[test]
    fn material_button_responds_to_density_changes() {
        test_widgets(|tester| {
            let key = KeyRef::from(ValueKey::new("test"));
            let child_key = KeyRef::from(ValueKey::new("test child"));
            let build_test =
                |tester: &mut WidgetTester, visual_density: VisualDensity, use_text: bool| {
                    let child = if use_text {
                        Text::new("Text").key(child_key.clone()).into_widget()
                    } else {
                        Container::new()
                            .key(child_key.clone())
                            .width(100.0)
                            .height(100.0)
                            .color(Color::new(0xffff0000))
                            .into_widget()
                    };
                    let theme = theme(&tester.cell.borrow());
                    tester.pump_widget(material_app(
                        theme,
                        Directionality::new(
                            TextDirection::Rtl,
                            Center::new().child(
                                MaterialButton::new(noop())
                                    .visual_density(visual_density)
                                    .key(key.clone())
                                    .child(child),
                            ),
                        )
                        .into_widget(),
                    ));
                };
            let cases = [
                (
                    VisualDensity::STANDARD,
                    false,
                    Size::new(132.0, 100.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::new(3.0, 3.0),
                    false,
                    Size::new(156.0, 124.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::new(-3.0, -3.0),
                    false,
                    Size::new(108.0, 100.0),
                    Rect::from_ltrb(350.0, 250.0, 450.0, 350.0),
                ),
                (
                    VisualDensity::STANDARD,
                    true,
                    Size::new(88.0, 48.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
                (
                    VisualDensity::new(3.0, 3.0),
                    true,
                    Size::new(112.0, 60.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
                (
                    VisualDensity::new(-3.0, -3.0),
                    true,
                    Size::new(76.0, 36.0),
                    Rect::from_ltrb(372.0, 293.0, 428.0, 307.0),
                ),
            ];
            for (visual_density, use_text, size, child_rect) in cases {
                build_test(tester, visual_density, use_text);
                tester.pump_and_settle();
                assert_eq!(tester.get_size(find::by_key(key.clone())), size);
                assert_eq!(tester.get_rect(find::by_key(child_key.clone())), child_rect);
            }
        });
    }

    #[test]
    fn disabled_elevation_is_passed_to_raw_material_button() {
        test_widgets(|tester| {
            let disabled_elevation = 16.0;
            pump_bare(tester, button(false).disabled_elevation(disabled_elevation));
            assert_eq!(raw_button_disabled_elevation(tester), disabled_elevation);
        });
    }

    #[test]
    fn material_button_disabled_elevation_defaults_to_0_0_when_not_provided() {
        test_widgets(|tester| {
            pump_bare(tester, button(false));
            assert_eq!(raw_button_disabled_elevation(tester), 0.0);
        });
    }

    #[test]
    fn material_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        SizedBox::shrink().child(MaterialButton::new(noop()).child(Text::new("X"))),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<MaterialButton>()),
                Size::ZERO
            );
        });
    }
}

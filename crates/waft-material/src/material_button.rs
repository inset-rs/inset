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

    use waft_embedder::{Clip, Offset, Size, TargetPlatform};
    use waft_foundation::ValueKey;
    use waft_rendering::testing::{PaintCall, assert_paints};
    use waft_widgets::testing::{TestPointer, find, test_widgets};
    use waft_widgets::{
        Center, Directionality, FocusHighlightStrategy, FocusManager, FocusNode, IntoWidget,
        SizedBox, Text, TextDirection,
    };

    use crate::material::Material;

    use super::*;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    fn theme(app: &App) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .platform(TargetPlatform::Android)
            .build(app)
    }

    fn pump(tester: &mut waft_widgets::testing::WidgetTester, button: MaterialButton) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(Material::new().child(button)),
            )
            .into_widget(),
        ));
    }

    fn raw_button(tester: &waft_widgets::testing::WidgetTester) -> RawMaterialButton {
        tester.widget(
            find::descendant(
                find::by_type::<MaterialButton>(),
                find::by_type::<RawMaterialButton>(),
            ),
            |button: &RawMaterialButton| RawMaterialButton {
                key: button.key.clone(),
                on_pressed: button.on_pressed.clone(),
                on_long_press: button.on_long_press.clone(),
                on_highlight_changed: button.on_highlight_changed.clone(),
                mouse_cursor: button.mouse_cursor.clone(),
                text_style: button.text_style.clone(),
                fill_color: button.fill_color,
                focus_color: button.focus_color,
                hover_color: button.hover_color,
                highlight_color: button.highlight_color,
                splash_color: button.splash_color,
                elevation: button.elevation,
                hover_elevation: button.hover_elevation,
                focus_elevation: button.focus_elevation,
                highlight_elevation: button.highlight_elevation,
                disabled_elevation: button.disabled_elevation,
                padding: button.padding,
                visual_density: button.visual_density,
                constraints: button.constraints,
                shape: button.shape.clone(),
                animation_duration: button.animation_duration,
                clip_behavior: button.clip_behavior,
                focus_node: button.focus_node.clone(),
                autofocus: button.autofocus,
                material_tap_target_size: button.material_tap_target_size,
                child: button.child.clone(),
                enable_feedback: button.enable_feedback,
            },
        )
    }

    #[test]
    fn min_width_and_height_are_the_button_s_minimum_size() {
        test_widgets(|tester| {
            let cases = [
                (Some(8.0), Some(24.0), Size::new(8.0, 24.0)),
                (Some(8.0), None, Size::new(8.0, 36.0)),
                (None, Some(8.0), Size::new(88.0, 8.0)),
                (None, None, Size::new(88.0, 36.0)),
            ];
            for (min_width, height, expected) in cases {
                let mut button = MaterialButton::new(None)
                    .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                    .padding(waft_painting::EdgeInsets::ZERO);
                if let Some(min_width) = min_width {
                    button = button.min_width(min_width);
                }
                if let Some(height) = height {
                    button = button.height(height);
                }
                pump(tester, button);
                assert_eq!(tester.get_size(find::by_type::<MaterialButton>()), expected);
            }
        });
    }

    #[test]
    fn the_shape_overrides_the_button_theme_shape() {
        test_widgets(|tester| {
            pump(
                tester,
                MaterialButton::new(None).shape(waft_painting::StadiumBorder::new(
                    waft_painting::BorderSide::NONE,
                )),
            );
            let button = raw_button(tester);
            assert!(
                button
                    .shape
                    .as_any()
                    .downcast_ref::<waft_painting::StadiumBorder>()
                    .is_some()
            );
        });
    }

    #[test]
    fn the_disabled_elevation_is_passed_and_defaults_to_zero() {
        test_widgets(|tester| {
            pump(tester, MaterialButton::new(None).disabled_elevation(16.0));
            assert_eq!(raw_button(tester).disabled_elevation, 16.0);

            pump(tester, MaterialButton::new(None));
            assert_eq!(raw_button(tester).disabled_elevation, 0.0);
        });
    }

    #[test]
    fn the_disabled_color_takes_precedence() {
        test_widgets(|tester| {
            pump(
                tester,
                MaterialButton::new(None).disabled_color(Color::new(0xFF00FF00)),
            );
            let material = tester.widget(
                find::descendant(
                    find::by_type::<MaterialButton>(),
                    find::by_type::<Material>(),
                )
                .first(),
                |material: &Material| (material.color, material.r#type),
            );
            assert_eq!(material.0, Some(Color::new(0xFF00FF00)));
        });
    }

    #[test]
    fn the_button_has_no_clip_by_default() {
        test_widgets(|tester| {
            pump(tester, MaterialButton::new(None));
            assert_eq!(raw_button(tester).clip_behavior, Clip::None);
        });
    }

    #[test]
    fn tapping_and_long_pressing_reach_their_own_callbacks() {
        test_widgets(|tester| {
            let tapped = Rc::new(Cell::new(false));
            let long_pressed = Rc::new(Cell::new(false));
            let tapped_seen = Rc::clone(&tapped);
            let long_seen = Rc::clone(&long_pressed);
            pump(
                tester,
                MaterialButton::new(Some(Listener::new(move |_app| tapped_seen.set(true))))
                    .on_long_press(Listener::new(move |_app| long_seen.set(true)))
                    .child(Text::new("button")),
            );

            tester.tap(find::by_type::<MaterialButton>());
            assert!(tapped.get());
            assert!(!long_pressed.get());

            tapped.set(false);
            tester.long_press(find::by_type::<MaterialButton>());
            assert!(!tapped.get(), "a long press is not a tap");
            assert!(long_pressed.get());
        });
    }

    #[test]
    fn the_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = theme(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(SizedBox::shrink().child(
                        MaterialButton::new(Some(Listener::new(|_app| {}))).child(Text::new("X")),
                    ))
                    .into_widget(),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<MaterialButton>()),
                Size::ZERO
            );
        });
    }

    /// Dart's `pumpWidget(Directionality(textDirection: ltr, child: button))`: no `Material`
    /// above it, so the tester's first ink features are the button's own.
    fn pump_bare(tester: &mut waft_widgets::testing::WidgetTester, button: MaterialButton) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Directionality::new(TextDirection::Ltr, button).into_widget(),
        ));
    }

    fn button(on_pressed: bool) -> MaterialButton {
        MaterialButton::new(on_pressed.then(|| Listener::new(|_app| {}))).child(Text::new("button"))
    }

    fn button_elevation(tester: &waft_widgets::testing::WidgetTester) -> f64 {
        tester.widget(
            find::descendant(
                find::by_type::<MaterialButton>(),
                find::by_type::<Material>(),
            )
            .first(),
            |material: &Material| material.elevation,
        )
    }

    #[test]
    fn a_hovered_button_paints_its_hover_color() {
        test_widgets(|tester| {
            let hover_color = Color::new(0xff001122);
            pump_bare(tester, button(true).hover_color(hover_color));

            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(
                &mut mouse,
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
    fn a_focused_button_paints_its_focus_color() {
        test_widgets(|tester| {
            let focus_color = Color::new(0xff001122);
            let focus_node: FocusNodeRef =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased();
            {
                let mut app = tester.cell.borrow_mut();
                FocusManager::instance(&mut app)
                    .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
            }
            pump_bare(
                tester,
                button(true)
                    .focus_color(focus_color)
                    .focus_node(focus_node.clone()),
            );
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();

            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(focus_color))],
            );
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn elevation_and_colors_follow_focus_then_hover_then_press() {
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
            {
                let mut app = tester.cell.borrow_mut();
                FocusManager::instance(&mut app)
                    .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
            }

            // Base elevation.
            assert_eq!(button_elevation(tester), elevation);

            // Focus elevation overrides base.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(focus_color))],
            );
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_eq!(button_elevation(tester), focus_elevation);

            // Hover elevation overrides focus.
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(
                &mut mouse,
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
            tester.remove_pointer(&mouse);

            // Highlight elevation overrides hover.
            let mut touch = TestPointer::touch(2);
            tester.down(
                &mut touch,
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
            tester.up(&touch);
        });
    }

    #[test]
    fn the_tap_target_size_follows_the_theme() {
        test_widgets(|tester| {
            let padded = KeyRef::from(ValueKey::new("padded"));
            let theme = ThemeData::builder()
                .use_material3(false)
                .platform(TargetPlatform::Android)
                .material_tap_target_size(MaterialTapTargetSize::Padded)
                .build(&tester.cell.borrow());
            tester.pump_widget(
                Theme::new(
                    theme,
                    Directionality::new(
                        TextDirection::Ltr,
                        Center::new().child(
                            button(true)
                                .key(padded.clone())
                                .child(SizedBox::new().width(50.0).height(8.0)),
                        ),
                    ),
                )
                .into_widget(),
            );
            assert_eq!(tester.get_size(find::by_key(padded)), Size::new(88.0, 48.0));

            let shrink_wrap = KeyRef::from(ValueKey::new("shrinkWrap"));
            let theme = ThemeData::builder()
                .use_material3(false)
                .platform(TargetPlatform::Android)
                .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                .build(&tester.cell.borrow());
            tester.pump_widget(
                Theme::new(
                    theme,
                    Directionality::new(
                        TextDirection::Ltr,
                        Center::new().child(
                            button(true)
                                .key(shrink_wrap.clone())
                                .child(SizedBox::new().width(50.0).height(8.0)),
                        ),
                    ),
                )
                .into_widget(),
            );
            assert_eq!(
                tester.get_size(find::by_key(shrink_wrap)),
                Size::new(88.0, 36.0)
            );
        });
    }
}

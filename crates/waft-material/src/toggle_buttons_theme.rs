//! Flutter counterpart: material_ui's `toggle_buttons_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyColor, BorderRadius, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Defines default property values for descendant `ToggleButtons` widgets.
///
/// Descendant widgets obtain the current [`ToggleButtonsThemeData`] object using
/// [`ToggleButtonsTheme::of`]. Instances of [`ToggleButtonsThemeData`] can be customized
/// with [`ToggleButtonsThemeData::copy_with`].
///
/// Typically a [`ToggleButtonsThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::toggle_buttons_theme`.
///
/// All [`ToggleButtonsThemeData`] properties are `None` by default. When null, the
/// `ToggleButtons` will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults based on the overall `Theme`'s `ThemeData::text_theme` and
/// `ThemeData::color_scheme`. See the individual `ToggleButtons` properties for details.
///
/// See also:
///
///  * [`ToggleButtonsTheme`], which describes the actual configuration of a
///    toggle buttons theme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToggleButtonsThemeData {
    /// The default text style for `ToggleButtons::children`.
    ///
    /// `TextStyle::color` will be ignored and substituted by [`color`](Self::color),
    /// [`selected_color`](Self::selected_color) or [`disabled_color`](Self::disabled_color)
    /// depending on whether the buttons are active, selected, or disabled.
    pub text_style: Option<TextStyle>,

    /// Defines the button's size.
    ///
    /// Typically used to constrain the button's minimum size.
    pub constraints: Option<BoxConstraints>,

    /// The color for descendant `Text` and `Icon` widgets if the button is enabled and not selected.
    pub color: Option<Color>,

    /// The color for descendant `Text` and `Icon` widgets if the button is selected.
    pub selected_color: Option<Color>,

    /// The color for descendant `Text` and `Icon` widgets if the button is disabled.
    pub disabled_color: Option<Color>,

    /// The fill color for selected toggle buttons.
    ///
    /// It can be a `WidgetStateColor`, which then resolves in the `WidgetState::Selected` and
    /// `WidgetState::Disabled` states.
    pub fill_color: Option<AnyColor>,

    /// The color to use for filling the button when the button has input focus.
    pub focus_color: Option<Color>,

    /// The highlight color for the button's `InkWell`.
    pub highlight_color: Option<Color>,

    /// The color to use for filling the button when the button has a pointer hovering over it.
    pub hover_color: Option<Color>,

    /// The splash color for the button's `InkWell`.
    pub splash_color: Option<Color>,

    /// The border color to display when the toggle button is enabled.
    pub border_color: Option<Color>,

    /// The border color to display when the toggle button is selected.
    pub selected_border_color: Option<Color>,

    /// The border color to display when the toggle button is disabled.
    pub disabled_border_color: Option<Color>,

    /// The radii of the border's corners.
    pub border_radius: Option<BorderRadius>,

    /// The width of the border surrounding each toggle button.
    ///
    /// This applies to both the greater surrounding border, as well as the
    /// borders dividing each toggle button.
    ///
    /// To render a hairline border (one physical pixel), set border_width to 0.0.
    /// See [`BorderSide::width`] for more details on hairline borders.
    pub border_width: Option<f64>,
}

impl ToggleButtonsThemeData {
    /// Creates the set of color and border properties used to configure `ToggleButtons`.
    pub fn new() -> ToggleButtonsThemeData {
        ToggleButtonsThemeData::default()
    }

    /// Dart `ToggleButtonsThemeData(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> ToggleButtonsThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `ToggleButtonsThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> ToggleButtonsThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `ToggleButtonsThemeData(color:)`.
    pub fn color(mut self, color: Color) -> ToggleButtonsThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `ToggleButtonsThemeData(selectedColor:)`.
    pub fn selected_color(mut self, selected_color: Color) -> ToggleButtonsThemeData {
        self.selected_color = Some(selected_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ToggleButtonsThemeData {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(fillColor:)`.
    pub fn fill_color(mut self, fill_color: impl Into<AnyColor>) -> ToggleButtonsThemeData {
        self.fill_color = Some(fill_color.into());
        self
    }

    /// Dart `ToggleButtonsThemeData(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> ToggleButtonsThemeData {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ToggleButtonsThemeData {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> ToggleButtonsThemeData {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ToggleButtonsThemeData {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(borderColor:)`.
    pub fn border_color(mut self, border_color: Color) -> ToggleButtonsThemeData {
        self.border_color = Some(border_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(selectedBorderColor:)`.
    pub fn selected_border_color(mut self, selected_border_color: Color) -> ToggleButtonsThemeData {
        self.selected_border_color = Some(selected_border_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(disabledBorderColor:)`.
    pub fn disabled_border_color(mut self, disabled_border_color: Color) -> ToggleButtonsThemeData {
        self.disabled_border_color = Some(disabled_border_color);
        self
    }

    /// Dart `ToggleButtonsThemeData(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> ToggleButtonsThemeData {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `ToggleButtonsThemeData(borderWidth:)`.
    pub fn border_width(mut self, border_width: f64) -> ToggleButtonsThemeData {
        self.border_width = Some(border_width);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().color(color)`).
    pub fn copy_with(&self) -> ToggleButtonsThemeData {
        self.clone()
    }

    /// Linearly interpolate between two toggle buttons themes.
    pub fn lerp(
        a: Option<&ToggleButtonsThemeData>,
        b: Option<&ToggleButtonsThemeData>,
        t: f64,
    ) -> Option<ToggleButtonsThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ToggleButtonsThemeData {
            text_style: TextStyle::lerp(
                a.and_then(|a| a.text_style.as_ref()),
                b.and_then(|b| b.text_style.as_ref()),
                t,
            ),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
            color: Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
            selected_color: Color::lerp(
                a.and_then(|a| a.selected_color),
                b.and_then(|b| b.selected_color),
                t,
            ),
            disabled_color: Color::lerp(
                a.and_then(|a| a.disabled_color),
                b.and_then(|b| b.disabled_color),
                t,
            ),
            fill_color: AnyColor::lerp(
                a.and_then(|a| a.fill_color.as_ref()),
                b.and_then(|b| b.fill_color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            focus_color: Color::lerp(
                a.and_then(|a| a.focus_color),
                b.and_then(|b| b.focus_color),
                t,
            ),
            highlight_color: Color::lerp(
                a.and_then(|a| a.highlight_color),
                b.and_then(|b| b.highlight_color),
                t,
            ),
            hover_color: Color::lerp(
                a.and_then(|a| a.hover_color),
                b.and_then(|b| b.hover_color),
                t,
            ),
            splash_color: Color::lerp(
                a.and_then(|a| a.splash_color),
                b.and_then(|b| b.splash_color),
                t,
            ),
            border_color: Color::lerp(
                a.and_then(|a| a.border_color),
                b.and_then(|b| b.border_color),
                t,
            ),
            selected_border_color: Color::lerp(
                a.and_then(|a| a.selected_border_color),
                b.and_then(|b| b.selected_border_color),
                t,
            ),
            disabled_border_color: Color::lerp(
                a.and_then(|a| a.disabled_border_color),
                b.and_then(|b| b.disabled_border_color),
                t,
            ),
            border_radius: BorderRadius::lerp(
                a.and_then(|a| a.border_radius),
                b.and_then(|b| b.border_radius),
                t,
            ),
            border_width: lerp_double(
                a.and_then(|a| a.border_width),
                b.and_then(|b| b.border_width),
                t,
            ),
        })
    }
}

/// An inherited widget that defines color and border parameters for `ToggleButtons` in this
/// widget's subtree.
///
/// Values specified here are used for `ToggleButtons` properties that are not given an
/// explicit non-null value.
#[derive(Debug)]
pub struct ToggleButtonsTheme {
    pub key: Option<KeyRef>,
    /// Defines the color and border properties for descendant `ToggleButtons` widgets.
    pub data: ToggleButtonsThemeData,
    pub child: WidgetRef,
}

impl ToggleButtonsTheme {
    /// Creates a toggle buttons theme that controls the color and border parameters for
    /// `ToggleButtons`.
    pub fn new<K>(data: ToggleButtonsThemeData, child: impl IntoWidget<K>) -> ToggleButtonsTheme {
        ToggleButtonsTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ToggleButtonsTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ToggleButtonsTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`ToggleButtonsThemeData`] from the closest ancestor
    /// [`ToggleButtonsTheme`] widget, or `ThemeData::toggle_buttons_theme` when there is none.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ToggleButtonsTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ToggleButtonsThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ToggleButtonsTheme>(app) {
            Some(toggle_buttons_theme) => toggle_buttons_theme.data.clone(),
            None => Theme::of(app, context).toggle_buttons_theme.clone(),
        }
    }
}

impl InheritedTheme for ToggleButtonsTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ToggleButtonsTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ToggleButtonsTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ToggleButtonsTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/toggle_buttons_theme_test.dart`; the `debugFillProperties` cases wait
    //! for diagnostics, and `hashCode` for `Hash`.

    use waft_embedder::TextBaseline;
    use waft_painting::TextDirection;
    use waft_rendering::RenderBox;
    use waft_widgets::{Center, Directionality, Icon, Row, Text, WidgetState, WidgetStateColor};
    use waft_widgets::{SizedBox, WidgetStates};

    use waft_widgets::testing::test_widgets;

    use super::*;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::{Material, MaterialType};
    use crate::theme_data::MaterialTapTargetSize;
    use crate::toggle_buttons::ToggleButtons;
    use crate::toggle_buttons::tests::{
        button_material, button_rect, button_text_style, check_ink_colors, default_theme,
        icon_theme_color, material_color, paints_physical_model_then_strokes,
        select_toggle_button_objects, style_color, texts, three_icons,
    };

    /// `Material(child: boilerplate(child: ToggleButtonsTheme(data:, child:)))`.
    fn themed(data: ToggleButtonsThemeData, buttons: ToggleButtons) -> WidgetRef {
        Material::new()
            .child(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(ToggleButtonsTheme::new(data, buttons)),
            ))
            .into_widget()
    }

    fn row_with_check() -> Vec<WidgetRef> {
        vec![
            // This Row is used like this to test for both TextStyle
            // and IconTheme for Text and Icon widgets respectively.
            Row::new()
                .children([
                    Text::new("First child").into_widget(),
                    Icon::new(Some(Icons::check())).into_widget(),
                ])
                .into_widget(),
        ]
    }

    // 'ToggleButtonsThemeData copyWith, ==, hashCode basics'
    #[test]
    fn copy_with_and_equality_basics() {
        assert_eq!(
            ToggleButtonsThemeData::new(),
            ToggleButtonsThemeData::new().copy_with()
        );
    }

    // 'ToggleButtonsThemeData lerp special cases'
    #[test]
    fn lerp_special_cases() {
        assert!(ToggleButtonsThemeData::lerp(None, None, 0.0).is_none());
        let data = ToggleButtonsThemeData::new();
        assert_eq!(
            ToggleButtonsThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // 'ToggleButtonsThemeData defaults'
    #[test]
    fn theme_data_has_no_values_by_default() {
        let theme_data = ToggleButtonsThemeData::new();
        assert_eq!(theme_data.text_style, None);
        assert_eq!(theme_data.constraints, None);
        assert_eq!(theme_data.color, None);
        assert_eq!(theme_data.selected_color, None);
        assert_eq!(theme_data.disabled_color, None);
        assert_eq!(theme_data.fill_color, None);
        assert_eq!(theme_data.focus_color, None);
        assert_eq!(theme_data.highlight_color, None);
        assert_eq!(theme_data.hover_color, None);
        assert_eq!(theme_data.splash_color, None);
        assert_eq!(theme_data.border_color, None);
        assert_eq!(theme_data.selected_border_color, None);
        assert_eq!(theme_data.disabled_border_color, None);
        assert_eq!(theme_data.border_radius, None);
        assert_eq!(theme_data.border_width, None);

        let theme = ToggleButtonsTheme::new(ToggleButtonsThemeData::new(), SizedBox::new());
        assert_eq!(theme.data, ToggleButtonsThemeData::new());
    }

    // 'Theme text style, except color, is applied'
    #[test]
    fn the_theme_text_style_is_applied_except_its_color() {
        test_widgets(|tester| {
            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().text_style(
                    TextStyle::new()
                        .color(Colors::ORANGE.primary())
                        .text_baseline(TextBaseline::Ideographic)
                        .font_size(20.0),
                ),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![false, true])
                    .on_pressed(|_app, _index| {}),
            ));

            for text in ["First child", "Second child"] {
                let text_style = button_text_style(tester, text);
                assert_eq!(text_style.text_baseline, Some(TextBaseline::Ideographic));
                assert_eq!(text_style.font_size, Some(20.0));
                assert_ne!(style_color(&text_style), Some(Colors::ORANGE.primary()));
            }
        });
    }

    // 'Custom BoxConstraints'
    #[test]
    fn the_theme_box_constraints_size_the_buttons() {
        test_widgets(|tester| {
            // Test for minimum constraints
            tester.pump_widget(themed(
                ToggleButtonsThemeData::new()
                    .constraints(BoxConstraints::new().min_width(50.0).min_height(60.0)),
                ToggleButtons::new(three_icons(), vec![false; 3])
                    .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                    .on_pressed(|_app, _index| {}),
            ));
            for index in 0..3 {
                let rect = button_rect(tester, index);
                assert_eq!(rect.width(), 50.0);
                assert_eq!(rect.height(), 60.0);
            }

            // Test for maximum constraints
            tester.pump_widget(themed(
                ToggleButtonsThemeData::new()
                    .constraints(BoxConstraints::new().max_width(20.0).max_height(10.0)),
                ToggleButtons::new(three_icons(), vec![false; 3])
                    .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                    .on_pressed(|_app, _index| {}),
            ));
            for index in 0..3 {
                let rect = button_rect(tester, index);
                assert_eq!(rect.width(), 20.0);
                assert_eq!(rect.height(), 10.0);
            }
        });
    }

    // 'Theme text/icon colors for enabled, selected and disabled states'
    #[test]
    fn the_theme_text_and_icon_colors_follow_the_state() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let check = Icons::check();
            let enabled_color = Colors::LIME.primary();
            let selected_color = Colors::GREEN.primary();
            let disabled_color = Colors::YELLOW.primary();

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new(),
                ToggleButtons::new(row_with_check(), vec![false])
                    .color(enabled_color)
                    .on_pressed(|_app, _index| {}),
            ));
            // Custom theme enabled color
            assert_ne!(theme.color_scheme.on_surface(), enabled_color);
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(enabled_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(enabled_color));

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().selected_color(selected_color),
                ToggleButtons::new(row_with_check(), vec![true])
                    .color(enabled_color)
                    .on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            // Custom theme selected color
            assert_ne!(theme.color_scheme.primary(), selected_color);
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(selected_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(selected_color));

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().disabled_color(disabled_color),
                ToggleButtons::new(row_with_check(), vec![false]).color(enabled_color),
            ));
            tester.pump_and_settle();
            // Custom theme disabled color
            assert_ne!(theme.disabled_color, disabled_color);
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(disabled_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(disabled_color));
        });
    }

    // 'Theme button fillColor'
    #[test]
    fn the_theme_fill_color_fills_a_selected_button() {
        test_widgets(|tester| {
            let custom_fill_color = Colors::GREEN.primary();
            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().fill_color(custom_fill_color),
                ToggleButtons::new(texts(&["First child"]), vec![true])
                    .on_pressed(|_app, _index| {}),
            ));

            let (color, material_type) =
                button_material(tester, None, |material| (material.color, material.r#type));
            assert_eq!(color, Some(custom_fill_color));
            assert_eq!(material_type, MaterialType::Button);
        });
    }

    // 'Custom Theme button fillColor in different states'
    #[test]
    fn a_widget_state_theme_fill_color_resolves_in_the_button_states() {
        test_widgets(|tester| {
            let enabled_fill_color = Colors::GREEN.primary();
            let selected_fill_color = Colors::BLUE.primary();
            let disabled_fill_color = Colors::YELLOW.primary();
            let get_color = move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    selected_fill_color
                } else if states.contains(&WidgetState::Disabled) {
                    disabled_fill_color
                } else {
                    enabled_fill_color
                }
            };

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().fill_color(WidgetStateColor::resolve_with(get_color)),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![true, false])
                    .on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            assert_eq!(
                material_color(tester, "First child"),
                Some(selected_fill_color)
            );
            assert_eq!(
                material_color(tester, "Second child"),
                Some(enabled_fill_color)
            );

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new().fill_color(WidgetStateColor::resolve_with(get_color)),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![true, false]),
            ));
            assert_eq!(
                material_color(tester, "First child"),
                Some(disabled_fill_color)
            );
            assert_eq!(
                material_color(tester, "Second child"),
                Some(disabled_fill_color)
            );
        });
    }

    // 'Theme InkWell colors'
    #[test]
    fn the_theme_ink_colors_are_used() {
        test_widgets(|tester| {
            let splash_color = Color::new(0xff4caf50);
            let highlight_color = Color::new(0xffcddc39);
            let hover_color = Color::new(0xffffeb3b);
            let focus_color = Color::new(0xffffff00);
            check_ink_colors(
                tester,
                |focus_node| {
                    themed(
                        ToggleButtonsThemeData::new()
                            .splash_color(splash_color)
                            .highlight_color(highlight_color)
                            .hover_color(hover_color)
                            .focus_color(focus_color),
                        ToggleButtons::new(texts(&["First child"]), vec![true])
                            .on_pressed(|_app, _index| {})
                            .focus_nodes(vec![focus_node.erase()]),
                    )
                },
                [hover_color, splash_color, focus_color],
                true,
            );
        });
    }

    // 'Theme border width and border colors for enabled, selected and disabled states'
    #[test]
    fn the_theme_border_width_and_colors_follow_the_state() {
        test_widgets(|tester| {
            let border_color = Color::new(0xff4caf50);
            let selected_border_color = Color::new(0xffcddc39);
            let disabled_border_color = Color::new(0xffffeb3b);
            const CUSTOM_WIDTH: f64 = 2.0;
            let button = |selected: bool, enabled: bool| {
                let buttons = ToggleButtons::new(texts(&["First child"]), vec![selected]);
                if enabled {
                    buttons.on_pressed(|_app, _index| {})
                } else {
                    buttons
                }
            };

            // The physical model's path is dropped from the display list when the fill has no
            // effect, as it has for an unselected button.
            tester.pump_widget(themed(
                ToggleButtonsThemeData::new()
                    .border_color(border_color)
                    .border_width(CUSTOM_WIDTH),
                button(false, true),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                false,
                &[(border_color, CUSTOM_WIDTH)]
            ));

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new()
                    .selected_border_color(selected_border_color)
                    .border_width(CUSTOM_WIDTH),
                button(true, true),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                true,
                &[(selected_border_color, CUSTOM_WIDTH)]
            ));

            tester.pump_widget(themed(
                ToggleButtonsThemeData::new()
                    .disabled_border_color(disabled_border_color)
                    .border_width(CUSTOM_WIDTH),
                button(false, false),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                false,
                &[(disabled_border_color, CUSTOM_WIDTH)]
            ));
        });
    }
}

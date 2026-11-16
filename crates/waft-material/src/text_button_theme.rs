//! Flutter counterpart: material_ui's `text_button_theme.dart`.

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;

/// A [`ButtonStyle`] that overrides the default appearance of
/// `TextButton`s when it's used with [`TextButtonTheme`] or with the
/// overall `Theme`'s `ThemeData::text_button_theme`.
///
/// The [`style`](Self::style)'s properties override `TextButton`'s default style,
/// i.e. the [`ButtonStyle`] returned by `TextButton::default_style_of`. Only
/// the style's non-null property values or resolved non-null
/// `WidgetStateProperty` values are used.
///
/// See also:
///
///  * [`TextButtonTheme`], the theme which is configured with this class.
///  * `TextButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for text buttons.
///  * `TextButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `TextButton`'s defaults.
///  * `WidgetStateProperty::resolve`, "resolve" a material state property
///    to a simple value based on a set of `WidgetState`s.
///  * `ThemeData::text_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `TextButton`s below the overall `Theme`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextButtonThemeData {
    /// Overrides for `TextButton`'s default style.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty`
    /// values override the [`ButtonStyle`] returned by
    /// `TextButton::default_style_of`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    pub style: Option<ButtonStyle>,
}

impl TextButtonThemeData {
    /// Creates a [`TextButtonThemeData`].
    ///
    /// The [`style`](Self::style) may be null.
    pub fn new() -> TextButtonThemeData {
        TextButtonThemeData::default()
    }

    /// Dart `TextButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> TextButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Linearly interpolate between two text button themes.
    pub fn lerp(
        a: Option<&TextButtonThemeData>,
        b: Option<&TextButtonThemeData>,
        t: f64,
    ) -> Option<TextButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(TextButtonThemeData {
            style: ButtonStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
        })
    }
}

/// Overrides the default [`ButtonStyle`] of its `TextButton` descendants.
///
/// See also:
///
///  * [`TextButtonThemeData`], which is used to configure this theme.
///  * `TextButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for text buttons.
///  * `TextButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `TextButton`'s defaults.
///  * `ThemeData::text_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `TextButton`s below the overall `Theme`.
#[derive(Debug)]
pub struct TextButtonTheme {
    pub key: Option<KeyRef>,
    /// The configuration of this theme.
    pub data: TextButtonThemeData,
    pub child: WidgetRef,
}

impl TextButtonTheme {
    /// Create a [`TextButtonTheme`].
    pub fn new<K>(data: TextButtonThemeData, child: impl IntoWidget<K>) -> TextButtonTheme {
        TextButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `TextButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextButtonTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`TextButtonThemeData`] from the closest ancestor
    /// [`TextButtonTheme`].
    ///
    /// If there is no enclosing [`TextButtonTheme`] widget, then
    /// `ThemeData::text_button_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = TextButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> TextButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<TextButtonTheme>(app) {
            Some(button_theme) => button_theme.data.clone(),
            None => Theme::of(app, context).text_button_theme.clone(),
        }
    }
}

impl InheritedTheme for TextButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        TextButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for TextButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TextButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]
    use crate::drawer_theme::tests::assert_near;

    use std::time::Duration;

    use waft_embedder::{Color, FontWeight, Radius, Size};
    use waft_foundation::Listener;
    use waft_painting::{
        AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder,
        StadiumBorder, TextStyle,
    };
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        Align, Builder, Center, GlobalKey, Icon, KeyedSubtree, Text, WidgetState,
        WidgetStateProperty, WidgetStates,
    };

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::button_style::{ButtonLayerBuilder, ButtonStyle};
    use crate::button_style_button::IconAlignment;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::Material;
    use crate::test_support::material_app;
    use crate::text_button::TextButton;
    use crate::theme_data::{MaterialTapTargetSize, ThemeData};

    fn pressed() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    /// Reads the button's `Material`, the last one in the tree.
    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let element = *tester
            .find_by_type::<Material>()
            .last()
            .expect("the button's Material");
        tester.widget(element, read)
    }

    fn ink_well<R>(tester: &WidgetTester, read: impl FnOnce(&InkWell) -> R) -> R {
        let element = *tester.find_by_type::<InkWell>().last().expect("an InkWell");
        tester.widget(element, read)
    }

    fn align_of_label(tester: &WidgetTester) -> AlignmentGeometry {
        let label = *tester.find_text("button").first().expect("the label");
        let align = tester.ancestor_of_type::<Align>(label);
        tester.widget(align, |align: &Align| align.alignment)
    }

    #[test]
    fn text_button_theme_lerp_special_cases() {
        assert_eq!(TextButtonThemeData::lerp(None, None, 0.0), None);
        let data = TextButtonThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(
            TextButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data.clone())
        );
    }

    fn no_theme_case(tester: &mut WidgetTester, use_material3: bool) {
        let color_scheme = ColorScheme::light();
        let theme = ThemeData::from(color_scheme.clone())
            .use_material3(use_material3)
            .build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new()
                .child(TextButton::new(pressed(), Some(Text::new("button"))))
                .into_widget(),
        ));

        material(tester, |material| {
            assert_eq!(material.animation_duration, Duration::from_millis(200));
            assert!(material.border_radius.is_none());
            assert_eq!(material.color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 0.0);
            assert_eq!(
                material.shadow_color,
                Some(if use_material3 {
                    Colors::TRANSPARENT
                } else {
                    Colors::BLACK
                })
            );
            let shape = material.shape.as_deref().expect("a shape");
            if use_material3 {
                assert!(shape.eq_shape(&StadiumBorder::new(BorderSide::NONE)));
            } else {
                assert!(shape.eq_shape(&RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(4.0)).into(),
                )));
            }
            let text_style = material.text_style.as_ref().expect("a text style");
            assert_eq!(
                text_style.color.as_deref().copied(),
                Some(color_scheme.primary)
            );
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.font_size, Some(14.0));
            assert_eq!(text_style.font_weight, Some(FontWeight::W500));
        });
        assert!(align_of_label(tester) == AlignmentGeometry::CENTER);
    }

    #[test]
    fn material_3_without_a_text_button_theme_the_defaults_apply() {
        test_widgets(|tester| no_theme_case(tester, true));
    }

    #[test]
    fn material_2_without_a_text_button_theme_the_defaults_apply() {
        test_widgets(|tester| no_theme_case(tester, false));
    }

    // The group '[Theme, TextTheme, TextButton style overrides]'.
    mod style_overrides {
        use super::*;

        const FOREGROUND_COLOR: Color = Color::new(0xff000001);
        const BACKGROUND_COLOR: Color = Color::new(0xff000002);
        const DISABLED_COLOR: Color = Color::new(0xff000003);
        const SHADOW_COLOR: Color = Color::new(0xff000004);
        const ELEVATION: f64 = 3.0;

        fn style_side() -> BorderSide {
            BorderSide {
                color: Colors::GREEN.primary(),
                width: 2.0,
                ..BorderSide::default()
            }
        }

        fn shape() -> RoundedRectangleBorder {
            RoundedRectangleBorder::new(
                style_side(),
                BorderRadius::all(Radius::circular(2.0)).into(),
            )
        }

        thread_local! {
            static BACKGROUND_KEY: GlobalKey = GlobalKey::new();
            static FOREGROUND_KEY: GlobalKey = GlobalKey::new();
        }

        fn background_key() -> GlobalKey {
            BACKGROUND_KEY.with(GlobalKey::clone)
        }

        fn foreground_key() -> GlobalKey {
            FOREGROUND_KEY.with(GlobalKey::clone)
        }

        fn background_builder(
            _app: &mut App,
            _context: BuildContext,
            _states: &WidgetStates,
            child: Option<WidgetRef>,
        ) -> WidgetRef {
            KeyedSubtree::new(child.expect("the button's child"))
                .key(background_key().into())
                .into_widget()
        }

        fn foreground_builder(
            _app: &mut App,
            _context: BuildContext,
            _states: &WidgetStates,
            child: Option<WidgetRef>,
        ) -> WidgetRef {
            KeyedSubtree::new(child.expect("the button's child"))
                .key(foreground_key().into())
                .into_widget()
        }

        fn style() -> ButtonStyle {
            TextButton::style_from()
                .foreground_color(FOREGROUND_COLOR)
                .disabled_foreground_color(DISABLED_COLOR)
                .background_color(BACKGROUND_COLOR)
                .disabled_background_color(DISABLED_COLOR)
                .shadow_color(SHADOW_COLOR)
                .elevation(ELEVATION)
                .text_style(TextStyle::new().font_size(12.0))
                .padding(EdgeInsetsGeometry::all(3.0))
                .minimum_size(Size::new(200.0, 200.0))
                .side(style_side())
                .shape(shape())
                .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
                .disabled_mouse_cursor(SystemMouseCursors::GRAB.into())
                .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                .animation_duration(Duration::from_millis(25))
                .enable_feedback(false)
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .background_builder(ButtonLayerBuilder::function(background_builder))
                .foreground_builder(ButtonLayerBuilder::function(foreground_builder))
                .build()
        }

        fn build_frame(
            tester: &WidgetTester,
            button_style: Option<ButtonStyle>,
            theme_style: Option<ButtonStyle>,
            overall_style: Option<ButtonStyle>,
        ) -> WidgetRef {
            let child = Builder::new(move |_app, _context| {
                let mut button = TextButton::new(pressed(), Some(Text::new("button")));
                if let Some(button_style) = &button_style {
                    button = button.style(button_style.clone());
                }
                button.into_widget()
            });
            let mut overall = TextButtonThemeData::new();
            if let Some(overall_style) = overall_style {
                overall = overall.style(overall_style);
            }
            let theme = ThemeData::from(ColorScheme::light())
                .build(&tester.cell.borrow())
                .copy_with()
                .text_button_theme(overall)
                .build();
            // If the TextButtonTheme widget is present, it's used
            // instead of the Theme's ThemeData.outlinedButtonTheme.
            let body = match theme_style {
                None => child.into_widget(),
                Some(theme_style) => {
                    TextButtonTheme::new(TextButtonThemeData::new().style(theme_style), child)
                        .into_widget()
                }
            };
            material_app(
                theme,
                Material::new()
                    .child(Center::new().child(body))
                    .into_widget(),
            )
        }

        fn check_button(tester: &WidgetTester) {
            let enabled = WidgetStates::new();
            let disabled: WidgetStates = [WidgetState::Disabled].into_iter().collect();
            let hovered: WidgetStates = [WidgetState::Hovered].into_iter().collect();
            let focused: WidgetStates = [WidgetState::Focused].into_iter().collect();
            material(tester, |material| {
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(text_style.color.as_deref().copied(), Some(FOREGROUND_COLOR));
                assert_eq!(text_style.font_size, Some(12.0));
                assert_eq!(material.color, Some(BACKGROUND_COLOR));
                assert_eq!(material.shadow_color, Some(SHADOW_COLOR));
                assert_eq!(material.elevation, ELEVATION);
                assert!(material.border_radius.is_none());
                assert!(
                    material
                        .shape
                        .as_deref()
                        .expect("a shape")
                        .eq_shape(&shape())
                );
                assert_eq!(material.animation_duration, Duration::from_millis(25));
            });
            ink_well(tester, |ink_well| {
                let mouse_cursor = ink_well
                    .ink_response
                    .mouse_cursor
                    .clone()
                    .expect("a mouse cursor");
                let resolve = |states: &WidgetStates| {
                    <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, states)
                };
                assert!(resolve(&enabled) == MouseCursorRef::from(SystemMouseCursors::TEXT));
                assert!(resolve(&disabled) == MouseCursorRef::from(SystemMouseCursors::GRAB));
                let overlay_color = ink_well
                    .ink_response
                    .overlay_color
                    .clone()
                    .expect("an overlay color");
                assert_eq!(
                    overlay_color.resolve(&hovered),
                    Some(FOREGROUND_COLOR.with_opacity(0.08))
                );
                assert_eq!(
                    overlay_color.resolve(&focused),
                    Some(FOREGROUND_COLOR.with_opacity(0.1))
                );
                assert!(!ink_well.ink_response.enable_feedback);
            });
            let button = *tester
                .find_by_type::<TextButton>()
                .first()
                .expect("an TextButton");
            let size = tester.render_box_of(button).size(&tester.cell.borrow());
            assert_eq!(size, Size::new(200.0, 200.0));
            assert!(align_of_label(tester) == AlignmentGeometry::CENTER_LEFT);
            let is_below = |key: GlobalKey, ancestor: waft_widgets::AnyElement| {
                let keyed = key
                    .current_context(&tester.cell.borrow_mut())
                    .expect("the keyed subtree is mounted");
                tester.ancestors(keyed).contains(&ancestor)
            };
            let material = *tester
                .find_by_type::<Material>()
                .last()
                .expect("a Material");
            let ink_well = *tester.find_by_type::<InkWell>().last().expect("an InkWell");
            assert!(is_below(background_key(), material));
            assert!(is_below(foreground_key(), ink_well));
        }

        /// What a case gives one of the three styles: Dart's `null`, `const ButtonStyle()` or
        /// [`style`].
        #[derive(Clone, Copy)]
        enum Given {
            Null,
            Empty,
            Style,
        }

        impl Given {
            fn style(self) -> Option<ButtonStyle> {
                match self {
                    Given::Null => None,
                    Given::Empty => Some(ButtonStyle::new()),
                    Given::Style => Some(style()),
                }
            }
        }

        fn case(button_style: Given, theme_style: Given, overall_style: Given) {
            test_widgets(|tester| {
                let frame = build_frame(
                    tester,
                    button_style.style(),
                    theme_style.style(),
                    overall_style.style(),
                );
                tester.pump_widget(frame);
                tester.pump_and_settle(); // allow the animations to finish
                check_button(tester);
            });
        }

        #[test]
        fn the_button_style_overrides_the_defaults() {
            case(Given::Style, Given::Null, Given::Null);
        }

        #[test]
        fn the_button_theme_style_overrides_the_defaults() {
            case(Given::Null, Given::Style, Given::Null);
        }

        #[test]
        fn the_overall_themes_button_theme_style_overrides_the_defaults() {
            case(Given::Null, Given::Null, Given::Style);
        }

        // Same as the previous tests with empty ButtonStyle's instead of null.

        #[test]
        fn the_button_style_overrides_the_defaults_over_empty_theme_and_overall_styles() {
            case(Given::Style, Given::Empty, Given::Empty);
        }

        #[test]
        fn the_button_theme_style_overrides_the_defaults_over_empty_button_and_overall_styles() {
            case(Given::Empty, Given::Style, Given::Empty);
        }

        #[test]
        fn the_overall_theme_style_overrides_the_defaults_with_no_theme_and_an_empty_button_style()
        {
            case(Given::Empty, Given::Null, Given::Style);
        }
    }

    fn shadow_color_case(tester: &mut WidgetTester, use_material3: bool) {
        let color_scheme = ColorScheme::light();
        let shadow_color = Color::new(0xff000001);
        let overridden_color = Color::new(0xff000002);

        let frame = |tester: &WidgetTester,
                     overall_shadow_color: Option<Color>,
                     theme_shadow_color: Option<Color>,
                     button_shadow_color: Option<Color>| {
            let theme = if use_material3 {
                let mut scheme = color_scheme.copy_with();
                if let Some(overall_shadow_color) = overall_shadow_color {
                    scheme = scheme.with_shadow(overall_shadow_color);
                }
                ThemeData::from(scheme).build(&tester.cell.borrow())
            } else {
                let theme = ThemeData::from(color_scheme.clone())
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                match overall_shadow_color {
                    Some(overall_shadow_color) => {
                        theme.copy_with().shadow_color(overall_shadow_color).build()
                    }
                    None => theme,
                }
            };
            let style_with = |color: Option<Color>| {
                let mut style = TextButton::style_from();
                if let Some(color) = color {
                    style = style.shadow_color(color);
                }
                style.build()
            };
            let button_style = style_with(button_shadow_color);
            material_app(
                theme,
                Material::new()
                    .child(Center::new().child(TextButtonTheme::new(
                        TextButtonThemeData::new().style(style_with(theme_shadow_color)),
                        Builder::new(move |_app, _context| {
                            TextButton::new(pressed(), Some(Text::new("button")))
                                .style(button_style.clone())
                                .into_widget()
                        }),
                    )))
                    .into_widget(),
            )
        };
        let mut pump = |overall: Option<Color>, theme: Option<Color>, button: Option<Color>| {
            let frame = frame(tester, overall, theme, button);
            tester.pump_widget(frame);
            tester.pump_and_settle(); // theme animation
            material(tester, |material| material.shadow_color)
        };

        assert_eq!(
            pump(None, None, None),
            Some(if use_material3 {
                Colors::TRANSPARENT
            } else {
                Colors::BLACK
            })
        );
        assert_eq!(
            pump(Some(shadow_color), None, None),
            Some(if use_material3 {
                Colors::TRANSPARENT
            } else {
                shadow_color
            })
        );
        assert_eq!(pump(None, Some(shadow_color), None), Some(shadow_color));
        assert_eq!(pump(None, None, Some(shadow_color)), Some(shadow_color));
        assert_eq!(
            pump(Some(overridden_color), Some(shadow_color), None),
            Some(shadow_color)
        );
        assert_eq!(
            pump(None, Some(overridden_color), Some(shadow_color)),
            Some(shadow_color)
        );
    }

    #[test]
    fn material_3_the_shadow_color_comes_from_the_button_then_its_theme_and_never_the_scheme() {
        test_widgets(|tester| shadow_color_case(tester, true));
    }

    #[test]
    fn material_2_the_shadow_color_comes_from_the_button_then_its_theme_then_the_theme() {
        test_widgets(|tester| shadow_color_case(tester, false));
    }

    #[test]
    fn the_icon_follows_the_themes_style_icon_alignment() {
        test_widgets(|tester| {
            let mut case = |icon_alignment: Option<IconAlignment>| {
                let mut style = ButtonStyle::new();
                if let Some(icon_alignment) = icon_alignment {
                    style = style.icon_alignment(icon_alignment);
                }
                let theme = ThemeData::builder()
                    .text_button_theme(TextButtonThemeData::new().style(style))
                    .build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            TextButton::icon(pressed(), Text::new("button"))
                                .icon(Icon::new(Some(Icons::add()))),
                        )
                        .into_widget(),
                ));
                tester.pump_and_settle();
                let button = *tester
                    .find_by_type::<Material>()
                    .last()
                    .expect("a Material");
                let icon = *tester.find_by_type::<Icon>().first().expect("an Icon");
                (
                    tester.global_rect(tester.render_box_of(button)),
                    tester.global_rect(tester.render_box_of(icon)),
                )
            };

            let (button, icon) = case(None);
            assert_eq!(button.left, icon.left - 12.0);

            let (button, icon) = case(Some(IconAlignment::End));
            assert_near(button.right, icon.right + 16.0);
        });
    }

    #[test]
    fn the_icon_takes_the_themes_foreground_color_over_the_default_icon_color() {
        // Regression test for https://github.com/flutter/flutter/issues/162839.
        test_widgets(|tester| {
            let foreground_color = Color::new(0xFFFFA500);
            let theme = ThemeData::builder()
                .text_button_theme(
                    TextButtonThemeData::new().style(
                        TextButton::style_from()
                            .foreground_color(foreground_color)
                            .build(),
                    ),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Center::new().child(
                            TextButton::icon(pressed(), Text::new("Button"))
                                .icon(Icon::new(Some(Icons::add()))),
                        ),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                tester.icon_style().color.as_deref().copied(),
                Some(foreground_color)
            );
        });
    }
}

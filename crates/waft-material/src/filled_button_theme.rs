//! Flutter counterpart: material_ui's `filled_button_theme.dart`.

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;

/// A [`ButtonStyle`] that overrides the default appearance of
/// `FilledButton`s when it's used with [`FilledButtonTheme`] or with the
/// overall `Theme`'s `ThemeData::filled_button_theme`.
///
/// The [`style`](Self::style)'s properties override `FilledButton`'s default style,
/// i.e. the [`ButtonStyle`] returned by `FilledButton::default_style_of`. Only
/// the style's non-null property values or resolved non-null
/// `WidgetStateProperty` values are used.
///
/// See also:
///
///  * [`FilledButtonTheme`], the theme which is configured with this class.
///  * `FilledButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for text buttons.
///  * `FilledButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `FilledButton`'s defaults.
///  * `WidgetStateProperty::resolve`, "resolve" a material state property
///    to a simple value based on a set of `WidgetState`s.
///  * `ThemeData::filled_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `FilledButton`s below the overall `Theme`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilledButtonThemeData {
    /// Overrides for `FilledButton`'s default style.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty`
    /// values override the [`ButtonStyle`] returned by
    /// `FilledButton::default_style_of`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    pub style: Option<ButtonStyle>,
}

impl FilledButtonThemeData {
    /// Creates an [`FilledButtonThemeData`].
    ///
    /// The [`style`](Self::style) may be null.
    pub fn new() -> FilledButtonThemeData {
        FilledButtonThemeData::default()
    }

    /// Dart `FilledButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> FilledButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Linearly interpolate between two filled button themes.
    pub fn lerp(
        a: Option<&FilledButtonThemeData>,
        b: Option<&FilledButtonThemeData>,
        t: f64,
    ) -> Option<FilledButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(FilledButtonThemeData {
            style: ButtonStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
        })
    }
}

/// Overrides the default [`ButtonStyle`] of its `FilledButton` descendants.
///
/// See also:
///
///  * [`FilledButtonThemeData`], which is used to configure this theme.
///  * `FilledButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for filled buttons.
///  * `FilledButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `FilledButton`'s defaults.
///  * `ThemeData::filled_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `FilledButton`s below the overall `Theme`.
#[derive(Debug)]
pub struct FilledButtonTheme {
    pub key: Option<KeyRef>,
    /// The configuration of this theme.
    pub data: FilledButtonThemeData,
    pub child: WidgetRef,
}

impl FilledButtonTheme {
    /// Create a [`FilledButtonTheme`].
    pub fn new<K>(data: FilledButtonThemeData, child: impl IntoWidget<K>) -> FilledButtonTheme {
        FilledButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `FilledButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> FilledButtonTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`FilledButtonThemeData`] from the closest ancestor [`FilledButtonTheme`].
    ///
    /// If there is no enclosing [`FilledButtonTheme`] widget, then
    /// `ThemeData::filled_button_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = FilledButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> FilledButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<FilledButtonTheme>(app) {
            Some(button_theme) => button_theme.data.clone(),
            None => Theme::of(app, context).filled_button_theme.clone(),
        }
    }
}

impl InheritedTheme for FilledButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        FilledButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for FilledButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &FilledButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Color, FontWeight, Radius, Size, TargetPlatform};
    use waft_foundation::Listener;
    use waft_painting::{
        Alignment, AlignmentGeometry, BorderRadius, BorderSide, BorderStyle, EdgeInsetsGeometry,
        OutlinedBorder, RoundedRectangleBorder, ShapeBorder, StadiumBorder, TextStyle,
    };
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        Align, AnyElement, Builder, Center, Column, Icon, IconTheme, SizedBox, WidgetState,
        WidgetStateProperty, WidgetStates, downcast_widget,
    };

    use super::*;
    use crate::button_style_button::IconAlignment;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::filled_button::FilledButton;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::Material;
    use crate::test_support::{WidgetTester, ahem_text, material_app, with_main_thread_stack};
    use crate::theme_data::{MaterialTapTargetSize, ThemeData};

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        crate::icons::install_material_icon_font(&mut tester.cell.borrow_mut());
        tester
    }

    fn pressed() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn text(text: &str) -> WidgetRef {
        ahem_text(text).into_widget()
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    fn is_inside<W: 'static>(tester: &WidgetTester, element: AnyElement) -> bool {
        let ancestors = tester.ancestors(element);
        let app = tester.cell.borrow();
        ancestors
            .into_iter()
            .any(|ancestor| downcast_widget::<W>(&**ancestor.widget(&app)).is_some())
    }

    /// The first `W` below an `FilledButton`.
    fn in_button<W: 'static>(tester: &WidgetTester) -> AnyElement {
        tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| is_inside::<FilledButton>(tester, *element))
            .expect("a descendant of the button")
    }

    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget::<Material, R>(in_button::<Material>(tester), read)
    }

    fn text_color(material: &Material) -> Option<Color> {
        material
            .text_style
            .as_ref()
            .and_then(|text_style| text_style.color.as_ref())
            .map(|color| color.color())
    }

    // filled_button_theme_test.dart 'FilledButtonThemeData lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_a_theme_with_itself_is_that_theme() {
        assert_eq!(FilledButtonThemeData::lerp(None, None, 0.0), None);
        let data = FilledButtonThemeData::new();
        assert_eq!(
            FilledButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // filled_button_theme_test.dart 'Passing no FilledButtonTheme returns defaults'.
    #[test]
    fn without_a_button_theme_the_button_shows_its_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let color_scheme = ColorScheme::light();
            let theme = ThemeData::from(color_scheme.clone()).build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(FilledButton::new(pressed(), Some(text("button"))))
                    .into_widget(),
            ));

            material(&tester, |material| {
                assert_eq!(material.animation_duration, Duration::from_millis(200));
                assert!(material.border_radius.is_none());
                assert_eq!(material.color, Some(color_scheme.primary));
                assert_eq!(material.elevation, 0.0);
                assert_eq!(material.shadow_color, Some(Color::new(0xff000000)));
                let shape = material.shape.as_deref().expect("a shape");
                assert!(shape.eq_shape(&StadiumBorder::default()));
                assert_eq!(text_color(material), Some(color_scheme.on_primary));
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
                assert_eq!(text_style.font_size, Some(14.0));
                assert_eq!(text_style.font_weight, Some(FontWeight::W500));
            });
            let align = in_button::<Align>(&tester);
            tester.widget::<Align, _>(align, |align| {
                assert!(align.alignment == AlignmentGeometry::from(Alignment::CENTER));
            });
        });
    }

    const FOREGROUND_COLOR: Color = Color::new(0xff000001);
    const BACKGROUND_COLOR: Color = Color::new(0xff000002);
    const DISABLED_FOREGROUND_COLOR: Color = Color::new(0xff000003);
    const DISABLED_BACKGROUND_COLOR: Color = Color::new(0xff000004);
    const SHADOW_COLOR: Color = Color::new(0xff000005);
    const ELEVATION: f64 = 1.0;
    const MINIMUM_SIZE: Size = Size::new(200.0, 200.0);
    const ANIMATION_DURATION: Duration = Duration::from_millis(25);
    const ENABLE_FEEDBACK: bool = false;
    const ALIGNMENT: AlignmentGeometry = AlignmentGeometry::CENTER_LEFT;

    fn side() -> BorderSide {
        BorderSide::new(
            Colors::GREEN.primary(),
            2.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        )
    }

    fn shape() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(side(), BorderRadius::all(Radius::circular(2.0)).into())
    }

    /// The style of the group '[Theme, TextTheme, FilledButton style overrides]'.
    fn override_style() -> ButtonStyle {
        let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
        let grab: MouseCursorRef = SystemMouseCursors::GRAB.into();
        FilledButton::style_from()
            .foreground_color(FOREGROUND_COLOR)
            .disabled_foreground_color(DISABLED_FOREGROUND_COLOR)
            .background_color(BACKGROUND_COLOR)
            .disabled_background_color(DISABLED_BACKGROUND_COLOR)
            .shadow_color(SHADOW_COLOR)
            .elevation(ELEVATION)
            .text_style(TextStyle::new().font_size(12.0))
            .padding(EdgeInsetsGeometry::all(3.0))
            .minimum_size(MINIMUM_SIZE)
            .side(side())
            .shape(Box::new(shape()) as Box<dyn OutlinedBorder>)
            .enabled_mouse_cursor(text)
            .disabled_mouse_cursor(grab)
            .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
            .animation_duration(ANIMATION_DURATION)
            .enable_feedback(ENABLE_FEEDBACK)
            .alignment(ALIGNMENT)
            .build()
    }

    fn override_frame(
        tester: &WidgetTester,
        button_style: Option<ButtonStyle>,
        theme_style: Option<ButtonStyle>,
        overall_style: Option<ButtonStyle>,
    ) -> WidgetRef {
        let child = Builder::new(move |_app, _context| {
            let mut button = FilledButton::new(pressed(), Some(text("button")));
            if let Some(button_style) = &button_style {
                button = button.style(button_style.clone());
            }
            button.into_widget()
        })
        .into_widget();
        let mut overall = FilledButtonThemeData::new();
        overall.style = overall_style;
        let theme = ThemeData::from(ColorScheme::light())
            .build(&tester.cell.borrow())
            .copy_with()
            .filled_button_theme(overall)
            .build();
        material_app(
            theme,
            Center::new()
                // If the FilledButtonTheme widget is present, it's used
                // instead of the Theme's ThemeData.FilledButtonTheme.
                .child(match theme_style {
                    None => child,
                    Some(theme_style) => FilledButtonTheme::new(
                        FilledButtonThemeData::new().style(theme_style),
                        child,
                    )
                    .into_widget(),
                })
                .into_widget(),
        )
    }

    fn check_button(tester: &WidgetTester) {
        material(tester, |material| {
            assert_eq!(text_color(material), Some(FOREGROUND_COLOR));
            assert_eq!(
                material
                    .text_style
                    .as_ref()
                    .and_then(|text_style| text_style.font_size),
                Some(12.0)
            );
            assert_eq!(material.color, Some(BACKGROUND_COLOR));
            assert_eq!(material.shadow_color, Some(SHADOW_COLOR));
            assert_eq!(material.elevation, ELEVATION);
            assert!(material.border_radius.is_none());
            assert!(material.shape.as_deref() == Some(&shape() as &dyn ShapeBorder));
            assert_eq!(material.animation_duration, ANIMATION_DURATION);
        });
        tester.widget::<InkWell, _>(in_button::<InkWell>(tester), |ink_well| {
            let response = &ink_well.ink_response;
            let cursor = response.mouse_cursor.clone().expect("a mouse cursor");
            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            let grab: MouseCursorRef = SystemMouseCursors::GRAB.into();
            assert!(
                *<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&cursor, &states(&[]))
                    == *text
            );
            assert!(
                *<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                    &cursor,
                    &states(&[WidgetState::Disabled])
                ) == *grab
            );
            let overlay = response.overlay_color.as_ref().expect("an overlay color");
            assert_eq!(
                overlay.resolve(&states(&[WidgetState::Hovered])),
                Some(FOREGROUND_COLOR.with_opacity(0.08))
            );
            assert_eq!(
                overlay.resolve(&states(&[WidgetState::Focused])),
                Some(FOREGROUND_COLOR.with_opacity(0.1))
            );
            assert_eq!(
                overlay.resolve(&states(&[WidgetState::Pressed])),
                Some(FOREGROUND_COLOR.with_opacity(0.1))
            );
            assert_eq!(response.enable_feedback, ENABLE_FEEDBACK);
        });
        let button = tester.find_by_type::<FilledButton>()[0];
        let size = tester.render_box_of(button).size(&tester.cell.borrow());
        assert_eq!(size, Size::new(200.0, 200.0));
        let align = in_button::<Align>(tester);
        tester.widget::<Align, _>(align, |align| assert!(align.alignment == ALIGNMENT));
    }

    // filled_button_theme_test.dart group '[Theme, TextTheme, FilledButton style
    // overrides]': each of its six cases.
    #[test]
    fn the_button_style_the_button_theme_and_the_overall_theme_each_override_the_defaults() {
        with_main_thread_stack(|| {
            let empty = || Some(ButtonStyle::new());
            // Each case is [button style, button theme style, overall theme style].
            let cases: [[Option<ButtonStyle>; 3]; 6] = [
                // Button style overrides defaults.
                [Some(override_style()), None, None],
                // Button theme style overrides defaults.
                [None, Some(override_style()), None],
                // Overall Theme button theme style overrides defaults.
                [None, None, Some(override_style())],
                // Same as the previous tests with empty ButtonStyle's instead of null.
                // Button style overrides defaults, empty theme and overall styles.
                [Some(override_style()), empty(), empty()],
                // Button theme style overrides defaults, empty button and overall styles.
                [empty(), Some(override_style()), empty()],
                // Overall Theme button theme style overrides defaults, null theme and empty
                // overall style.
                [empty(), None, Some(override_style())],
            ];
            for [button_style, theme_style, overall_style] in cases {
                let mut tester = tester();
                let frame = override_frame(&tester, button_style, theme_style, overall_style);
                tester.pump_widget(frame);
                tester.pump_and_settle(); // allow the animations to finish
                check_button(&tester);
            }
        });
    }

    // filled_button_theme_test.dart 'FilledButton respects Theme shadowColor'.
    #[test]
    fn the_shadow_color_comes_from_the_button_then_its_theme() {
        with_main_thread_stack(|| {
            let shadow_color = Color::new(0xff000001);
            let overridden_color = Color::new(0xff000002);
            let mut tester = tester();
            let build_frame = |tester: &WidgetTester,
                               overall_shadow_color: Option<Color>,
                               theme_shadow_color: Option<Color>,
                               button_shadow_color: Option<Color>| {
                let theme = ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow());
                let theme = match overall_shadow_color {
                    Some(overall) => theme.copy_with().shadow_color(overall).build(),
                    None => theme,
                };
                let style_from = |color: Option<Color>| {
                    let mut style = FilledButton::style_from();
                    if let Some(color) = color {
                        style = style.shadow_color(color);
                    }
                    style.build()
                };
                let button_style = style_from(button_shadow_color);
                material_app(
                    theme,
                    Center::new()
                        .child(FilledButtonTheme::new(
                            FilledButtonThemeData::new().style(style_from(theme_shadow_color)),
                            Builder::new(move |_app, _context| {
                                FilledButton::new(pressed(), Some(text("button")))
                                    .style(button_style.clone())
                                    .into_widget()
                            }),
                        ))
                        .into_widget(),
                )
            };
            let shadow = |tester: &WidgetTester| material(tester, |material| material.shadow_color);

            let frame = build_frame(&tester, None, None, None);
            tester.pump_widget(frame);
            assert_eq!(shadow(&tester), Some(Colors::BLACK)); //default

            for (overall, theme, button) in [
                (None, Some(shadow_color), None),
                (None, None, Some(shadow_color)),
                (Some(overridden_color), Some(shadow_color), None),
                (None, Some(overridden_color), Some(shadow_color)),
            ] {
                let frame = build_frame(&tester, overall, theme, button);
                tester.pump_widget(frame);
                tester.pump_and_settle(); // theme animation
                assert_eq!(shadow(&tester), Some(shadow_color));
            }
        });
    }

    fn icon_button_frame(theme: ThemeData, tonal: bool) -> WidgetRef {
        let button = if tonal {
            FilledButton::tonal_icon(pressed(), text("button"))
        } else {
            FilledButton::icon(pressed(), text("button"))
        }
        .icon(Icon::new(Some(Icons::add())));
        material_app(theme, Center::new().child(button).into_widget())
    }

    // filled_button_theme_test.dart 'FilledButton.icon respects FilledButtonTheme
    // ButtonStyle.iconAlignment' and 'Filled tonal button icon respects FilledButtonTheme
    // ButtonStyle.iconAlignment'.
    #[test]
    fn an_icon_button_takes_its_icon_alignment_from_the_theme() {
        with_main_thread_stack(|| {
            for tonal in [false, true] {
                let mut tester = tester();
                let build = |tester: &WidgetTester, icon_alignment: Option<IconAlignment>| {
                    let mut style = ButtonStyle::new();
                    style.icon_alignment = icon_alignment;
                    let theme = ThemeData::builder()
                        .filled_button_theme(FilledButtonThemeData::new().style(style))
                        .build(&tester.cell.borrow());
                    icon_button_frame(theme, tonal)
                };
                let rects = |tester: &WidgetTester| {
                    let material =
                        tester.global_rect(tester.render_box_of(in_button::<Material>(tester)));
                    let icon =
                        tester.global_rect(tester.render_box_of(tester.find_by_type::<Icon>()[0]));
                    (material, icon)
                };

                let frame = build(&tester, None);
                tester.pump_widget(frame);
                let (button, icon) = rects(&tester);
                assert_eq!(button.left, icon.left - 16.0);

                let frame = build(&tester, Some(IconAlignment::End));
                tester.pump_widget(frame);
                tester.pump_and_settle();
                let (button, icon) = rects(&tester);
                assert_eq!(button.right, icon.right + 24.0);
            }
        });
    }

    // filled_button_theme_test.dart 'FilledButton icon uses provided FilledButtonTheme
    // foregroundColor over default icon color'.
    #[test]
    fn each_icon_takes_the_theme_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let foreground_color = Color::new(0xFFFFA500);
            let seen = Rc::new(RefCell::new(Vec::new()));
            let probe = || {
                let seen = Rc::clone(&seen);
                Builder::new(move |app, context| {
                    seen.borrow_mut()
                        .push(IconTheme::of(app, context).color.map(|color| color.color()));
                    SizedBox::new().width(18.0).height(18.0).into_widget()
                })
                .into_widget()
            };
            let theme = ThemeData::builder()
                .filled_button_theme(
                    FilledButtonThemeData::new().style(
                        FilledButton::style_from()
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
                            Column::new().children([
                                FilledButton::icon(pressed(), text("Button"))
                                    .icon(probe())
                                    .into_widget(),
                                FilledButton::icon(pressed(), text("Button"))
                                    .icon(probe())
                                    .into_widget(),
                            ]),
                        ),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                *seen.borrow(),
                vec![Some(foreground_color), Some(foreground_color)]
            );
        });
    }
}

//! Flutter counterpart: material_ui's `elevated_button_theme.dart`.

use waft_foundation::App;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef, downcast_widget,
};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;
use crate::theme_data::StyleVariant;

/// A [`ButtonStyle`] that overrides the default appearance of
/// `ElevatedButton`s when it's used with [`ElevatedButtonTheme`] or with the
/// overall `Theme`'s `ThemeData::elevated_button_theme`.
///
/// The [`style`](Self::style)'s properties override `ElevatedButton`'s default style,
/// i.e. the [`ButtonStyle`] returned by `ElevatedButton::default_style_of`. Only
/// the style's non-null property values or resolved non-null
/// `WidgetStateProperty` values are used.
///
/// See also:
///
///  * [`ElevatedButtonTheme`], the theme which is configured with this class.
///  * `ElevatedButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for text buttons.
///  * `ElevatedButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `ElevatedButton`'s defaults.
///  * `WidgetStateProperty::resolve`, "resolve" a material state property
///    to a simple value based on a set of `WidgetState`s.
///  * `ThemeData::elevated_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `ElevatedButton`s below the overall `Theme`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElevatedButtonThemeData {
    /// Overrides for `ElevatedButton`'s default style.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty`
    /// values override the [`ButtonStyle`] returned by
    /// `ElevatedButton::default_style_of`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    pub style: Option<ButtonStyle>,

    /// The style variant of Material Design used by `ElevatedButton`.
    ///
    /// Set this to [`StyleVariant::Material3Expressive`] to opt elevated buttons
    /// into the Material 3 Expressive style.
    ///
    /// If null, the variant is inherited from the closest enclosing
    /// [`ElevatedButtonTheme`] that specifies one, and then from
    /// `ThemeData::elevated_button_theme`. If none of them specify a variant,
    /// `ElevatedButton` uses [`StyleVariant::Material3`]. To keep a subtree on
    /// Material 3 when an ancestor opts into Material 3 Expressive, set this to
    /// [`StyleVariant::Material3`].
    ///
    /// An enclosing [`ElevatedButtonTheme`] takes precedence over
    /// `ThemeData::elevated_button_theme`, even when a closer `Theme` specifies a
    /// different variant. This matches [`ElevatedButtonTheme::of`], which only uses
    /// `ThemeData::elevated_button_theme` when there is no enclosing
    /// [`ElevatedButtonTheme`]. To change the variant of a subtree, wrap it in an
    /// [`ElevatedButtonTheme`].
    ///
    /// See also:
    ///
    ///  * [`ElevatedButtonTheme::variant_of`], which returns the variant that
    ///    applies at a given location.
    pub variant: Option<StyleVariant>,
}

impl ElevatedButtonThemeData {
    /// Creates an [`ElevatedButtonThemeData`].
    ///
    /// The [`style`](Self::style) may be null.
    pub fn new() -> ElevatedButtonThemeData {
        ElevatedButtonThemeData::default()
    }

    /// Dart `ElevatedButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> ElevatedButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Dart `ElevatedButtonThemeData(variant:)`.
    pub fn variant(mut self, variant: StyleVariant) -> ElevatedButtonThemeData {
        self.variant = Some(variant);
        self
    }

    /// Linearly interpolate between two elevated button themes.
    pub fn lerp(
        a: Option<&ElevatedButtonThemeData>,
        b: Option<&ElevatedButtonThemeData>,
        t: f64,
    ) -> Option<ElevatedButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ElevatedButtonThemeData {
            style: ButtonStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
            variant: if t < 0.5 {
                a.and_then(|a| a.variant)
            } else {
                b.and_then(|b| b.variant)
            },
        })
    }
}

/// Overrides the default [`ButtonStyle`] of its `ElevatedButton` descendants.
///
/// See also:
///
///  * [`ElevatedButtonThemeData`], which is used to configure this theme.
///  * `ElevatedButton::default_style_of`, which returns the default [`ButtonStyle`]
///    for elevated buttons.
///  * `ElevatedButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `ElevatedButton`'s defaults.
///  * `ThemeData::elevated_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `ElevatedButton`s below the overall `Theme`.
#[derive(Debug)]
pub struct ElevatedButtonTheme {
    pub key: Option<KeyRef>,
    /// The configuration of this theme.
    pub data: ElevatedButtonThemeData,
    pub child: WidgetRef,
}

impl ElevatedButtonTheme {
    /// Create a [`ElevatedButtonTheme`].
    pub fn new<K>(data: ElevatedButtonThemeData, child: impl IntoWidget<K>) -> ElevatedButtonTheme {
        ElevatedButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ElevatedButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ElevatedButtonTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`ElevatedButtonThemeData`] from the closest ancestor
    /// [`ElevatedButtonTheme`].
    ///
    /// If there is no enclosing [`ElevatedButtonTheme`] widget, then
    /// `ThemeData::elevated_button_theme` is used.
    ///
    /// The returned [`ElevatedButtonThemeData::variant`] is the value specified by
    /// that theme, which can be null even when an ancestor specifies a variant.
    /// Use [`variant_of`](Self::variant_of) to get the variant that applies at the given
    /// `context`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ElevatedButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ElevatedButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ElevatedButtonTheme>(app) {
            Some(button_theme) => button_theme.data.clone(),
            None => Theme::of(app, context).elevated_button_theme.clone(),
        }
    }

    /// Returns the [`StyleVariant`] that `ElevatedButton`s use at the given
    /// `context`.
    ///
    /// The closest enclosing [`ElevatedButtonTheme`] whose
    /// [`ElevatedButtonThemeData::variant`] is non-null determines the variant, so
    /// an [`ElevatedButtonTheme`] that doesn't specify a variant inherits it from
    /// its ancestors. If none of the enclosing [`ElevatedButtonTheme`]s specify a
    /// variant, the variant of `ThemeData::elevated_button_theme` is used. If that
    /// is also null, [`StyleVariant::Material3`] is returned.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let variant = ElevatedButtonTheme::variant_of(app, context);
    /// ```
    pub fn variant_of(app: &mut App, context: BuildContext) -> StyleVariant {
        let mut element =
            context.get_element_for_inherited_widget_of_exact_type::<ElevatedButtonTheme>(app);
        while let Some(current) = element {
            // Depend on every theme that is visited, so that a change to any of them
            // rebuilds the dependent.
            let widget = context.depend_on_inherited_element(app, current, None);
            let variant = downcast_widget::<ElevatedButtonTheme>(&*widget)
                .expect("found by type")
                .data
                .variant;
            if let Some(variant) = variant {
                return variant;
            }
            // Calling get_element_for_inherited_widget_of_exact_type on the element itself
            // would return the same element, so continue the search from its parent.
            let mut parent = None;
            current.visit_ancestor_elements(app, &mut |ancestor| {
                parent = Some(ancestor);
                false
            });
            element = parent.and_then(|parent| {
                parent.get_element_for_inherited_widget_of_exact_type::<ElevatedButtonTheme>(app)
            });
        }
        Theme::of(app, context)
            .elevated_button_theme
            .variant
            .unwrap_or(StyleVariant::Material3)
    }
}

impl InheritedTheme for ElevatedButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ElevatedButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ElevatedButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ElevatedButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Color, FontWeight, Radius, Size, TargetPlatform};
    use waft_foundation::{Listener, ValueNotifier};
    use waft_painting::{
        Alignment, AlignmentGeometry, BorderRadius, BorderSide, BorderStyle, EdgeInsetsGeometry,
        OutlinedBorder, RoundedRectangleBorder, ShapeBorder, StadiumBorder, TextStyle,
    };
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        Align, AnyElement, Builder, Center, Icon, IconTheme, KeyRef, Row, SizedBox,
        ValueListenableBuilder, WidgetState, WidgetStateProperty, WidgetStatePropertyAll,
        WidgetStates,
    };

    use super::*;
    use crate::button_style::{ButtonShapeVariant, ButtonSizeVariant};
    use crate::button_style_button::IconAlignment;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::elevated_button::ElevatedButton;
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

    /// The first `W` below an `ElevatedButton`.
    fn in_button<W: 'static>(tester: &WidgetTester) -> AnyElement {
        tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| is_inside::<ElevatedButton>(tester, *element))
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

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(radius)).into(),
        )
    }

    // elevated_button_theme_test.dart 'ElevatedButtonThemeData supports Material style
    // variants'.
    #[test]
    fn the_theme_data_carries_a_variant_that_lerps_at_the_midpoint() {
        let material3 = ElevatedButtonThemeData::new().variant(StyleVariant::Material3);
        let expressive = ElevatedButtonThemeData::new().variant(StyleVariant::Material3Expressive);

        assert_eq!(material3.variant, Some(StyleVariant::Material3));
        assert_eq!(expressive.variant, Some(StyleVariant::Material3Expressive));
        assert_eq!(
            ElevatedButtonThemeData::lerp(Some(&material3), Some(&expressive), 0.49),
            Some(material3.clone())
        );
        assert_eq!(
            ElevatedButtonThemeData::lerp(Some(&material3), Some(&expressive), 0.5),
            Some(expressive.clone())
        );
        assert_ne!(material3, expressive);
    }

    // elevated_button_theme_test.dart 'ElevatedButtonThemeData hashCode, diagnostics, and lerp
    // include variant', without the hash code (deferred) and with `Debug` for the diagnostics.
    #[test]
    fn the_debug_description_and_lerp_include_the_variant() {
        let expressive = ElevatedButtonThemeData::new().variant(StyleVariant::Material3Expressive);
        assert!(format!("{expressive:?}").contains("Material3Expressive"));
        assert_eq!(
            ElevatedButtonThemeData::lerp(None, Some(&expressive), 0.4).and_then(|t| t.variant),
            None
        );
        assert_eq!(
            ElevatedButtonThemeData::lerp(None, Some(&expressive), 0.6).and_then(|t| t.variant),
            Some(StyleVariant::Material3Expressive)
        );
    }

    // elevated_button_theme_test.dart 'ElevatedButtonThemeData lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_a_theme_with_itself_is_that_theme() {
        assert_eq!(ElevatedButtonThemeData::lerp(None, None, 0.0), None);
        let data = ElevatedButtonThemeData::new();
        assert_eq!(
            ElevatedButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // elevated_button_theme_test.dart 'Material3: Passing no ElevatedButtonTheme returns
    // defaults' and 'Material2: Passing no ElevatedButtonTheme returns defaults'.
    #[test]
    fn without_a_button_theme_the_button_shows_its_defaults() {
        with_main_thread_stack(|| {
            for use_material3 in [true, false] {
                let mut tester = tester();
                let color_scheme = ColorScheme::light();
                let theme = ThemeData::from(color_scheme.clone())
                    .use_material3(use_material3)
                    .build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(ElevatedButton::new(pressed(), Some(text("button"))))
                        .into_widget(),
                ));

                material(&tester, |material| {
                    assert_eq!(material.animation_duration, Duration::from_millis(200));
                    assert!(material.border_radius.is_none());
                    let shape = material.shape.as_deref().expect("a shape");
                    if use_material3 {
                        assert_eq!(material.color, Some(color_scheme.surface));
                        assert_eq!(material.elevation, 1.0);
                        assert_eq!(material.shadow_color, Some(color_scheme.shadow()));
                        assert!(shape.eq_shape(&StadiumBorder::default()));
                        assert_eq!(text_color(material), Some(color_scheme.primary));
                    } else {
                        assert_eq!(material.color, Some(color_scheme.primary));
                        assert_eq!(material.elevation, 2.0);
                        assert_eq!(material.shadow_color, Some(Color::new(0xff000000)));
                        assert!(shape.eq_shape(&rounded(4.0)));
                        assert_eq!(text_color(material), Some(color_scheme.on_primary));
                    }
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
                    assert_eq!(text_style.font_size, Some(14.0));
                    assert_eq!(text_style.font_weight, Some(FontWeight::W500));
                });
                let align = in_button::<Align>(&tester);
                tester.widget::<Align, _>(align, |align| {
                    assert!(align.alignment == AlignmentGeometry::from(Alignment::CENTER));
                });
            }
        });
    }

    const FOREGROUND_COLOR: Color = Color::new(0xff000001);
    const BACKGROUND_COLOR: Color = Color::new(0xff000002);
    const DISABLED_COLOR: Color = Color::new(0xff000003);
    const SHADOW_COLOR: Color = Color::new(0xff000004);
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

    /// The style of the group '[Theme, TextTheme, ElevatedButton style overrides]'.
    fn override_style() -> ButtonStyle {
        let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
        let grab: MouseCursorRef = SystemMouseCursors::GRAB.into();
        ElevatedButton::style_from()
            .foreground_color(FOREGROUND_COLOR)
            .disabled_foreground_color(DISABLED_COLOR)
            .background_color(BACKGROUND_COLOR)
            .disabled_background_color(DISABLED_COLOR)
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
            let mut button = ElevatedButton::new(pressed(), Some(text("button")));
            if let Some(button_style) = &button_style {
                button = button.style(button_style.clone());
            }
            button.into_widget()
        })
        .into_widget();
        let mut overall = ElevatedButtonThemeData::new();
        overall.style = overall_style;
        let theme = ThemeData::from(ColorScheme::light())
            .use_material3(false)
            .build(&tester.cell.borrow())
            .copy_with()
            .elevated_button_theme(overall)
            .build();
        material_app(
            theme,
            Center::new()
                // If the ElevatedButtonTheme widget is present, it's used
                // instead of the Theme's ThemeData.ElevatedButtonTheme.
                .child(match theme_style {
                    None => child,
                    Some(theme_style) => ElevatedButtonTheme::new(
                        ElevatedButtonThemeData::new().style(theme_style),
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
        let button = tester.find_by_type::<ElevatedButton>()[0];
        let size = tester.render_box_of(button).size(&tester.cell.borrow());
        assert_eq!(size, Size::new(200.0, 200.0));
        let align = in_button::<Align>(tester);
        tester.widget::<Align, _>(align, |align| assert!(align.alignment == ALIGNMENT));
    }

    // elevated_button_theme_test.dart group '[Theme, TextTheme, ElevatedButton style
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

    // elevated_button_theme_test.dart 'Material3 - ElevatedButton respects Theme shadowColor'
    // and 'Material2 - ElevatedButton respects Theme shadowColor'.
    #[test]
    fn the_shadow_color_comes_from_the_button_then_its_theme_then_the_overall_theme() {
        with_main_thread_stack(|| {
            let shadow_color = Color::new(0xff000001);
            let overridden_color = Color::new(0xff000002);
            for use_material3 in [true, false] {
                let mut tester = tester();
                let build_frame =
                    |tester: &WidgetTester,
                     overall_shadow_color: Option<Color>,
                     theme_shadow_color: Option<Color>,
                     button_shadow_color: Option<Color>| {
                        let theme = if use_material3 {
                            let mut color_scheme = ColorScheme::light();
                            if let Some(overall) = overall_shadow_color {
                                color_scheme = color_scheme.copy_with().with_shadow(overall);
                            }
                            ThemeData::from(color_scheme).build(&tester.cell.borrow())
                        } else {
                            let theme = ThemeData::from(ColorScheme::light())
                                .use_material3(false)
                                .build(&tester.cell.borrow());
                            match overall_shadow_color {
                                Some(overall) => theme.copy_with().shadow_color(overall).build(),
                                None => theme,
                            }
                        };
                        let style_from = |color: Option<Color>| {
                            let mut style = ElevatedButton::style_from();
                            if let Some(color) = color {
                                style = style.shadow_color(color);
                            }
                            style.build()
                        };
                        let button_style = style_from(button_shadow_color);
                        material_app(
                            theme,
                            Center::new()
                                .child(ElevatedButtonTheme::new(
                                    ElevatedButtonThemeData::new()
                                        .style(style_from(theme_shadow_color)),
                                    Builder::new(move |_app, _context| {
                                        ElevatedButton::new(pressed(), Some(text("button")))
                                            .style(button_style.clone())
                                            .into_widget()
                                    }),
                                ))
                                .into_widget(),
                        )
                    };
                let shadow =
                    |tester: &WidgetTester| material(tester, |material| material.shadow_color);

                let frame = build_frame(&tester, None, None, None);
                tester.pump_widget(frame);
                assert_eq!(shadow(&tester), Some(Colors::BLACK)); //default

                for (overall, theme, button) in [
                    (Some(shadow_color), None, None),
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
            }
        });
    }

    fn icon_button_frame(tester: &WidgetTester, theme: ThemeData) -> WidgetRef {
        let _ = tester;
        material_app(
            theme,
            Center::new()
                .child(
                    ElevatedButton::icon(pressed(), text("button"))
                        .icon(Icon::new(Some(Icons::add()))),
                )
                .into_widget(),
        )
    }

    // elevated_button_theme_test.dart 'ElevatedButton.icon respects ElevatedButtonTheme
    // ButtonStyle.iconAlignment'.
    #[test]
    fn an_icon_button_takes_its_icon_alignment_from_the_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build = |tester: &WidgetTester, icon_alignment: Option<IconAlignment>| {
                let mut style = ButtonStyle::new();
                style.icon_alignment = icon_alignment;
                let theme = ThemeData::builder()
                    .elevated_button_theme(ElevatedButtonThemeData::new().style(style))
                    .build(&tester.cell.borrow());
                icon_button_frame(tester, theme)
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
        });
    }

    // elevated_button_theme_test.dart 'ElevatedButton icon uses provided ElevatedButtonTheme
    // foregroundColor over default icon color'.
    #[test]
    fn an_icon_takes_the_theme_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let foreground_color = Color::new(0xFFFFA500);
            let icon_color = Rc::new(Cell::new(None));
            let probe = Builder::new({
                let icon_color = Rc::clone(&icon_color);
                move |app, context| {
                    icon_color.set(IconTheme::of(app, context).color.map(|color| color.color()));
                    SizedBox::new().width(18.0).height(18.0).into_widget()
                }
            });
            let theme = ThemeData::builder()
                .elevated_button_theme(
                    ElevatedButtonThemeData::new().style(
                        ElevatedButton::style_from()
                            .foreground_color(foreground_color)
                            .build(),
                    ),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Center::new()
                            .child(ElevatedButton::icon(pressed(), text("Button")).icon(probe)),
                    )
                    .into_widget(),
            ));
            assert_eq!(icon_color.get(), Some(foreground_color));
        });
    }

    // A square elevated button uses a RoundedRectangleBorder in Material 3
    // Expressive. Material 3 ignores shapeVariant and uses a StadiumBorder.
    fn square_button() -> WidgetRef {
        ElevatedButton::new(pressed(), Some(text("Button")))
            .key(button_key())
            .style(ButtonStyle::new().shape_variant(ButtonShapeVariant::Square))
            .into_widget()
    }

    fn button_key() -> KeyRef {
        waft_foundation::ValueKey::new("button".to_owned()).into()
    }

    fn custom_style() -> ButtonStyle {
        ButtonStyle::new().foreground_color(Rc::new(WidgetStatePropertyAll::new(Some(Color::new(
            0xffff0000,
        )))))
    }

    fn is_stadium(tester: &WidgetTester) -> bool {
        material(tester, |material| {
            material
                .shape
                .as_ref()
                .is_some_and(|shape| shape.as_any().downcast_ref::<StadiumBorder>().is_some())
        })
    }

    fn is_rounded_rectangle(tester: &WidgetTester) -> bool {
        material(tester, |material| {
            material.shape.as_ref().is_some_and(|shape| {
                shape
                    .as_any()
                    .downcast_ref::<RoundedRectangleBorder>()
                    .is_some()
            })
        })
    }

    /// Builds the square button and reports the variant that applies to it.
    fn button_with_variant(variant: &Rc<Cell<Option<StyleVariant>>>) -> WidgetRef {
        let variant = Rc::clone(variant);
        Builder::new(move |app, context| {
            variant.set(Some(ElevatedButtonTheme::variant_of(app, context)));
            square_button()
        })
        .into_widget()
    }

    fn variant_data(variant: StyleVariant) -> ElevatedButtonThemeData {
        ElevatedButtonThemeData::new().variant(variant)
    }

    fn variant_theme(tester: &WidgetTester, variant: StyleVariant) -> ThemeData {
        ThemeData::builder()
            .elevated_button_theme(variant_data(variant))
            .build(&tester.cell.borrow())
    }

    fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'returns
    // material3 when no theme specifies a variant'.
    #[test]
    fn variant_of_is_material_3_when_no_theme_names_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(Cell::new(None));
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        ElevatedButtonThemeData::new().style(custom_style()),
                        button_with_variant(&variant),
                    ))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3));
            assert!(is_stadium(&tester));
        });
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'returns the
    // variant of ThemeData.elevatedButtonTheme'.
    #[test]
    fn variant_of_falls_back_to_the_theme_data_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(Cell::new(None));
            let theme = variant_theme(&tester, StyleVariant::Material3Expressive);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(button_with_variant(&variant))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'a nested
    // ElevatedButtonTheme without a variant inherits it from an ElevatedButtonTheme', 'an
    // ElevatedButtonTheme without a variant inherits it from ThemeData.elevatedButtonTheme' and
    // 'an explicit variant overrides the variant of the ancestors'.
    #[test]
    fn variant_of_takes_the_nearest_theme_that_names_one() {
        with_main_thread_stack(|| {
            // Nested in an expressive ElevatedButtonTheme.
            let mut tester = tester();
            let variant = Rc::new(Cell::new(None));
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        variant_data(StyleVariant::Material3Expressive),
                        ElevatedButtonTheme::new(
                            ElevatedButtonThemeData::new().style(custom_style()),
                            button_with_variant(&variant),
                        ),
                    ))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));

            // Under an expressive ThemeData.
            let mut tester = self::tester();
            let variant = Rc::new(Cell::new(None));
            let theme = variant_theme(&tester, StyleVariant::Material3Expressive);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        ElevatedButtonThemeData::new().style(custom_style()),
                        button_with_variant(&variant),
                    ))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));

            // An explicit Material 3 below an expressive ancestor.
            let mut tester = self::tester();
            let variant = Rc::new(Cell::new(None));
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        variant_data(StyleVariant::Material3Expressive),
                        ElevatedButtonTheme::new(
                            ElevatedButtonThemeData::new()
                                .style(custom_style())
                                .variant(StyleVariant::Material3),
                            button_with_variant(&variant),
                        ),
                    ))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3));
            assert!(is_stadium(&tester));
        });
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'an enclosing
    // ElevatedButtonTheme takes precedence over a closer Theme'.
    #[test]
    fn an_enclosing_button_theme_wins_over_a_closer_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(Cell::new(None));
            let theme = default_theme(&tester);
            let closer = variant_theme(&tester, StyleVariant::Material3);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        variant_data(StyleVariant::Material3Expressive),
                        Theme::new(closer, button_with_variant(&variant)),
                    ))
                    .into_widget(),
            ));
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'ElevatedButton
    // is rebuilt when an ElevatedButtonTheme above the closest one changes'.
    #[test]
    fn the_button_rebuilds_when_a_farther_theme_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = tester
                .cell
                .borrow_mut()
                .create(ValueNotifier::new(Some(StyleVariant::Material3Expressive)));
            // The subtree is made once, so it is only rebuilt through its
            // dependency on the outer ElevatedButtonTheme.
            let child = ElevatedButtonTheme::new(
                ElevatedButtonThemeData::new().style(custom_style()),
                square_button(),
            )
            .into_widget();
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        ValueListenableBuilder::new(
                            Rc::new(variant),
                            |_app, _context, value: &Option<StyleVariant>, child| {
                                let mut data = ElevatedButtonThemeData::new();
                                data.variant = *value;
                                ElevatedButtonTheme::new(data, child.expect("a child").clone())
                                    .into_widget()
                            },
                        )
                        .child(child),
                    )
                    .into_widget(),
            ));
            assert!(is_rounded_rectangle(&tester));

            variant.set_value(&mut tester.cell.borrow_mut(), Some(StyleVariant::Material3));
            tester.pump_and_settle();
            assert!(is_stadium(&tester));
        });
    }

    // elevated_button_theme_test.dart group 'ElevatedButtonTheme.variantOf', 'ElevatedButton.icon
    // uses the inherited variant for the icon-label gap'.
    #[test]
    fn an_icon_button_gap_follows_the_inherited_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = default_theme(&tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(ElevatedButtonTheme::new(
                        variant_data(StyleVariant::Material3Expressive),
                        ElevatedButtonTheme::new(
                            ElevatedButtonThemeData::new().style(custom_style()),
                            ElevatedButton::icon(pressed(), text("Button"))
                                .icon(Icon::new(Some(Icons::add())))
                                .key(button_key())
                                .style(ButtonStyle::new().size_variant(ButtonSizeVariant::Large)),
                        ),
                    ))
                    .into_widget(),
            ));

            // Large Material 3 Expressive buttons have a 12 pixel gap between the
            // icon and the label. Material 3 uses 8 pixels.
            let row = in_button::<Row>(&tester);
            assert_eq!(tester.widget::<Row, _>(row, |row| row.spacing), 12.0);
        });
    }
}

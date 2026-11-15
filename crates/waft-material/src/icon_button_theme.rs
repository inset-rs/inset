//! Flutter counterpart: material_ui's `icon_button_theme.dart`.

use waft_foundation::App;
use waft_widgets::{
    AnyElement, BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    downcast_widget,
};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;
use crate::theme_data::StyleVariant;

/// A [`ButtonStyle`] that overrides the default appearance of
/// `IconButton`s when it's used with the `IconButton`, the [`IconButtonTheme`] or the
/// overall `Theme`'s `ThemeData::icon_button_theme`.
///
/// The `IconButton` will be affected by [`IconButtonTheme`] and [`IconButtonThemeData`]
/// only if `ThemeData::use_material3` is set to true; otherwise, `IconTheme` will be used.
///
/// The [`style`](Self::style)'s properties override `IconButton`'s default style. Only
/// the style's non-null property values or resolved non-null
/// `WidgetStateProperty` values are used.
///
/// See also:
///
///  * [`IconButtonTheme`], the theme which is configured with this class.
///  * `IconButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `IconButton`'s defaults.
///  * `WidgetStateProperty::resolve`, "resolve" a material state property
///    to a simple value based on a set of `WidgetState`s.
///  * `ThemeData::icon_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `IconButton`s below the overall `Theme`.
#[derive(Clone, Debug, Default)]
pub struct IconButtonThemeData {
    /// Overrides for `IconButton`'s default style if `ThemeData::use_material3`
    /// is set to true.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty`
    /// values override the default [`ButtonStyle`] in `IconButton`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    pub style: Option<ButtonStyle>,

    /// The style variant of Material Design used by `IconButton`.
    ///
    /// Set this to [`StyleVariant::Material3Expressive`] to opt icon buttons into
    /// the Material 3 Expressive style.
    ///
    /// If null, the variant is inherited from the closest enclosing
    /// [`IconButtonTheme`] that specifies one, and then from
    /// `ThemeData::icon_button_theme`. If none of them specify a variant,
    /// `IconButton` uses [`StyleVariant::Material3`]. To keep a subtree on
    /// Material 3 when an ancestor opts into Material 3 Expressive, set this to
    /// [`StyleVariant::Material3`].
    ///
    /// An enclosing [`IconButtonTheme`] takes precedence over
    /// `ThemeData::icon_button_theme`, even when a closer `Theme` specifies a
    /// different variant. This matches [`IconButtonTheme::of`], which only uses
    /// `ThemeData::icon_button_theme` when there is no enclosing [`IconButtonTheme`].
    /// To change the variant of a subtree, wrap it in an [`IconButtonTheme`].
    ///
    /// See also:
    ///
    ///  * [`IconButtonTheme::variant_of`], which returns the variant that applies
    ///    at a given location.
    pub variant: Option<StyleVariant>,
}

impl IconButtonThemeData {
    /// Creates a [`IconButtonThemeData`].
    ///
    /// The [`style`](Self::style) may be null.
    pub fn new() -> IconButtonThemeData {
        IconButtonThemeData::default()
    }

    /// Dart `IconButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> IconButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Dart `IconButtonThemeData(variant:)`.
    pub fn variant(mut self, variant: StyleVariant) -> IconButtonThemeData {
        self.variant = Some(variant);
        self
    }

    /// Linearly interpolate between two icon button themes.
    pub fn lerp(
        a: Option<&IconButtonThemeData>,
        b: Option<&IconButtonThemeData>,
        t: f64,
    ) -> Option<IconButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(IconButtonThemeData {
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

impl PartialEq for IconButtonThemeData {
    fn eq(&self, other: &IconButtonThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.style == self.style && other.variant == self.variant
    }
}

/// Overrides the default [`ButtonStyle`] of its `IconButton` descendants.
///
/// See also:
///
///  * [`IconButtonThemeData`], which is used to configure this theme.
///  * `IconButton::style_from`, which converts simple values into a
///    [`ButtonStyle`] that's consistent with `IconButton`'s defaults.
///  * `ThemeData::icon_button_theme`, which can be used to override the default
///    [`ButtonStyle`] for `IconButton`s below the overall `Theme`.
#[derive(Debug)]
pub struct IconButtonTheme {
    pub key: Option<KeyRef>,
    /// The configuration of this theme.
    pub data: IconButtonThemeData,
    pub child: WidgetRef,
}

impl IconButtonTheme {
    /// Create a [`IconButtonTheme`].
    pub fn new<K>(data: IconButtonThemeData, child: impl IntoWidget<K>) -> IconButtonTheme {
        IconButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `IconButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> IconButtonTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`IconButtonThemeData`] from the closest ancestor [`IconButtonTheme`].
    ///
    /// If there is no enclosing [`IconButtonTheme`] widget, then
    /// `ThemeData::icon_button_theme` is used.
    ///
    /// The returned [`IconButtonThemeData::variant`] is the value specified by
    /// that theme, which can be null even when an ancestor specifies a variant.
    /// Use [`variant_of`](Self::variant_of) to get the variant that applies at the given
    /// `context`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = IconButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> IconButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<IconButtonTheme>(app) {
            Some(button_theme) => button_theme.data.clone(),
            None => Theme::of(app, context).icon_button_theme,
        }
    }

    /// Returns the [`StyleVariant`] that `IconButton`s use at the given `context`.
    ///
    /// The closest enclosing [`IconButtonTheme`] whose
    /// [`IconButtonThemeData::variant`] is non-null determines the variant, so an
    /// [`IconButtonTheme`] that doesn't specify a variant inherits it from its
    /// ancestors. If none of the enclosing [`IconButtonTheme`]s specify a variant,
    /// the variant of `ThemeData::icon_button_theme` is used. If that is also null,
    /// [`StyleVariant::Material3`] is returned.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let variant = IconButtonTheme::variant_of(app, context);
    /// ```
    pub fn variant_of(app: &mut App, context: BuildContext) -> StyleVariant {
        let mut element =
            context.get_element_for_inherited_widget_of_exact_type::<IconButtonTheme>(app);
        while let Some(found) = element {
            // Depend on every theme that is visited, so that a change to any of them
            // rebuilds the dependent.
            context.depend_on_inherited_element(app, found, None);
            let variant = downcast_widget::<IconButtonTheme>(&**found.widget(app))
                .expect("found as an IconButtonTheme")
                .data
                .variant;
            if let Some(variant) = variant {
                return variant;
            }
            // Calling get_element_for_inherited_widget_of_exact_type on the element itself
            // would return the same element, so continue the search from its parent.
            let mut parent: Option<AnyElement> = None;
            found.visit_ancestor_elements(app, &mut |ancestor| {
                parent = Some(ancestor);
                false
            });
            element = parent.and_then(|parent| {
                parent.get_element_for_inherited_widget_of_exact_type::<IconButtonTheme>(app)
            });
        }
        Theme::of(app, context)
            .icon_button_theme
            .variant
            .unwrap_or(StyleVariant::Material3)
    }
}

impl InheritedTheme for IconButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        IconButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for IconButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &IconButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Color, Offset, Radius, Size, TargetPlatform};
    use waft_foundation::{App, Listener, ValueNotifier};
    use waft_painting::{
        Alignment, AlignmentGeometry, BorderRadius, BorderSide, BorderStyle, EdgeInsets,
        OutlinedBorder, RoundedRectangleBorder, ShapeBorder, StadiumBorder,
    };
    use waft_services::{
        LogicalKeyboardKey, MouseCursorRef, PhysicalKeyboardKey, SystemMouseCursors,
    };
    use waft_widgets::{
        Align, AnyElement, Builder, Center, Icon, KeyRef, ValueListenableBuilder, WidgetState,
        WidgetStateProperty, WidgetStatePropertyAll, WidgetStates,
    };

    use super::*;
    use crate::button_style::{ButtonShapeVariant, ButtonSizeVariant, IconButtonWidthVariant};
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::{Material, MaterialType};
    use crate::test_support::{Drawn, WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::{MaterialTapTargetSize, ThemeData};

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    fn noop() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    fn m3_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::from(ColorScheme::light()).build(&tester.cell.borrow())
    }

    fn icon_button_element(tester: &WidgetTester) -> AnyElement {
        tester.find_by_type::<IconButton>()[0]
    }

    /// The first `W` below the `IconButton`: Dart's `find.descendant(of: find.byType(IconButton),
    /// matching: find.byType(W))`.
    fn below_button<W: 'static, R>(tester: &WidgetTester, read: impl FnOnce(&W) -> R) -> R {
        let button = icon_button_element(tester);
        let found = tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| tester.ancestors(*element).contains(&button))
            .expect("a widget of that type below the IconButton");
        tester.widget(found, read)
    }

    fn button_size(tester: &WidgetTester) -> Size {
        let render = tester.render_box_of(icon_button_element(tester));
        render.size(&tester.cell.borrow())
    }

    fn icon_alignment(tester: &WidgetTester) -> AlignmentGeometry {
        let icon = tester.find_by_type::<Icon>()[0];
        let align = tester.ancestor_of_type::<Align>(icon);
        tester.widget(align, |align: &Align| align.alignment)
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    // 'IconButtonThemeData lerp special cases'; a value has no identity, so the second
    // expectation compares by value.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_theme_is_that_theme() {
        assert!(IconButtonThemeData::lerp(None, None, 0.0).is_none());
        let data = IconButtonThemeData::new();
        assert_eq!(
            IconButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // 'IconButtonThemeData supports Material 3 Expressive variant'
    #[test]
    fn the_theme_data_keeps_the_expressive_variant() {
        let data = IconButtonThemeData::new().variant(StyleVariant::Material3Expressive);
        assert_eq!(data.variant, Some(StyleVariant::Material3Expressive));
    }

    // 'IconButtonThemeData variant defaults to null'
    #[test]
    fn the_variant_defaults_to_none() {
        assert!(IconButtonThemeData::new().variant.is_none());
    }

    // icon_button_test.dart 'IconButtonThemeData': 'equality'
    #[test]
    fn theme_data_with_the_same_variants_are_equal() {
        let style = |size, width, shape| {
            IconButtonThemeData::new().style(
                ButtonStyle::new()
                    .size_variant(size)
                    .icon_button_width(width)
                    .shape_variant(shape),
            )
        };
        let a = style(
            ButtonSizeVariant::Small,
            IconButtonWidthVariant::Standard,
            ButtonShapeVariant::Round,
        );
        let b = style(
            ButtonSizeVariant::Small,
            IconButtonWidthVariant::Standard,
            ButtonShapeVariant::Round,
        );
        let c = style(
            ButtonSizeVariant::Large,
            IconButtonWidthVariant::Wide,
            ButtonShapeVariant::Square,
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // icon_button_test.dart 'IconButtonThemeData': 'lerp'
    #[test]
    fn lerp_switches_the_variants_halfway() {
        let a = IconButtonThemeData::new().style(
            ButtonStyle::new()
                .size_variant(ButtonSizeVariant::Small)
                .icon_button_width(IconButtonWidthVariant::Narrow)
                .shape_variant(ButtonShapeVariant::Round),
        );
        let b = IconButtonThemeData::new().style(
            ButtonStyle::new()
                .size_variant(ButtonSizeVariant::Large)
                .icon_button_width(IconButtonWidthVariant::Wide)
                .shape_variant(ButtonShapeVariant::Square),
        );
        let at = |t: f64| {
            IconButtonThemeData::lerp(Some(&a), Some(&b), t)
                .and_then(|data| data.style)
                .expect("a style")
        };
        assert_eq!(at(0.0).size_variant, Some(ButtonSizeVariant::Small));
        assert_eq!(at(0.4).size_variant, Some(ButtonSizeVariant::Small));
        assert_eq!(at(0.5).size_variant, Some(ButtonSizeVariant::Large));
        assert_eq!(at(1.0).size_variant, Some(ButtonSizeVariant::Large));
        assert_eq!(
            at(0.4).icon_button_width,
            Some(IconButtonWidthVariant::Narrow)
        );
        assert_eq!(
            at(0.5).icon_button_width,
            Some(IconButtonWidthVariant::Wide)
        );
        assert_eq!(at(0.4).shape_variant, Some(ButtonShapeVariant::Round));
        assert_eq!(at(0.5).shape_variant, Some(ButtonShapeVariant::Square));
    }

    // 'IconButton supports Material 3 Expressive style variants'; Dart's `Scaffold` adds
    // nothing the button reads.
    #[test]
    fn a_disabled_button_builds_under_an_expressive_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::builder()
                .use_material3(true)
                .build(&tester.cell.borrow());
            let widget = material_app(
                theme,
                IconButtonTheme::new(
                    IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                    IconButton::new(None, Icon::new(Some(Icons::ac_unit()))),
                )
                .into_widget(),
            );
            tester.pump_widget(widget);
        });
    }

    // 'Passing no IconButtonTheme returns defaults'
    #[test]
    fn without_a_theme_the_button_has_its_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = material_app(
                m3_theme(&tester),
                Center::new()
                    .child(IconButton::new(noop(), Icon::new(Some(Icons::ac_unit()))))
                    .into_widget(),
            );
            tester.pump_widget(widget);

            below_button(&tester, |material: &Material| {
                assert_eq!(material.animation_duration, Duration::from_millis(200));
                assert!(material.border_radius.is_none());
                assert_eq!(material.color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 0.0);
                assert_eq!(material.shadow_color, Some(Colors::TRANSPARENT));
                assert!(
                    material.shape.as_deref()
                        == Some(&StadiumBorder::default() as &dyn ShapeBorder)
                );
                assert!(material.text_style.is_none());
                assert_eq!(material.r#type, MaterialType::Button);
            });
            assert!(icon_alignment(&tester) == AlignmentGeometry::CENTER);
        });
    }

    const FOREGROUND_COLOR: Color = Color::new(0xff000001);
    const DISABLED_FOREGROUND_COLOR: Color = Color::new(0xff000002);
    const BACKGROUND_COLOR: Color = Color::new(0xff000003);
    const SHADOW_COLOR: Color = Color::new(0xff000004);
    const ELEVATION: f64 = 3.0;
    const MINIMUM_SIZE: Size = Size::new(200.0, 200.0);
    const ANIMATION_DURATION: Duration = Duration::from_millis(25);
    const ENABLE_FEEDBACK: bool = false;

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

    /// The `style` of Dart's '[Theme, IconTheme, IconButton style overrides]' group.
    fn override_style() -> ButtonStyle {
        IconButton::style_from()
            .foreground_color(FOREGROUND_COLOR)
            .disabled_foreground_color(DISABLED_FOREGROUND_COLOR)
            .background_color(BACKGROUND_COLOR)
            .shadow_color(SHADOW_COLOR)
            .elevation(ELEVATION)
            .padding(EdgeInsets::all(3.0).into())
            .minimum_size(MINIMUM_SIZE)
            .side(side())
            .shape(Box::new(shape()) as Box<dyn OutlinedBorder>)
            .enabled_mouse_cursor(SystemMouseCursors::TEXT.into())
            .disabled_mouse_cursor(SystemMouseCursors::GRAB.into())
            .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
            .animation_duration(ANIMATION_DURATION)
            .enable_feedback(ENABLE_FEEDBACK)
            .alignment(Alignment::CENTER_LEFT.into())
            .build()
    }

    /// Dart's `buildFrame` of that group.
    fn override_frame(
        tester: &WidgetTester,
        button_style: Option<ButtonStyle>,
        theme_style: Option<ButtonStyle>,
        overall_style: Option<ButtonStyle>,
    ) -> WidgetRef {
        let child = Builder::new(move |_app, _context| {
            let button = IconButton::new(noop(), Icon::new(Some(Icons::ac_unit())));
            match &button_style {
                Some(style) => button.style(style.clone()),
                None => button,
            }
            .into_widget()
        })
        .into_widget();
        let mut overall = IconButtonThemeData::new();
        overall.style = overall_style;
        let theme = m3_theme(tester)
            .copy_with()
            .icon_button_theme(overall)
            .build();
        // If the IconButtonTheme widget is present, it's used
        // instead of the Theme's ThemeData.iconButtonTheme.
        let body = match theme_style {
            None => child,
            Some(theme_style) => {
                IconButtonTheme::new(IconButtonThemeData::new().style(theme_style), child)
                    .into_widget()
            }
        };
        material_app(theme, Center::new().child(body).into_widget())
    }

    /// Dart's `checkButton`.
    fn check_button(tester: &WidgetTester) {
        below_button(tester, |material: &Material| {
            assert!(material.text_style.is_none());
            assert_eq!(material.color, Some(BACKGROUND_COLOR));
            assert_eq!(material.shadow_color, Some(SHADOW_COLOR));
            assert_eq!(material.elevation, ELEVATION);
            assert!(material.border_radius.is_none());
            assert!(material.shape.as_deref() == Some(&shape() as &dyn ShapeBorder));
            assert_eq!(material.animation_duration, ANIMATION_DURATION);
        });
        below_button(tester, |ink_well: &InkWell| {
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
        assert_eq!(button_size(tester), Size::new(200.0, 200.0));
        assert!(icon_alignment(tester) == Alignment::CENTER_LEFT.into());
    }

    fn check_override(
        button_style: Option<ButtonStyle>,
        theme_style: Option<ButtonStyle>,
        overall_style: Option<ButtonStyle>,
    ) {
        let mut tester = tester();
        let widget = override_frame(&tester, button_style, theme_style, overall_style);
        tester.pump_widget(widget);
        tester.pump_and_settle(); // allow the animations to finish
        check_button(&tester);
    }

    // '[Theme, IconTheme, IconButton style overrides]': 'Button style overrides defaults'
    #[test]
    fn the_button_style_overrides_the_defaults() {
        with_main_thread_stack(|| check_override(Some(override_style()), None, None));
    }

    // 'Button theme style overrides defaults'
    #[test]
    fn the_button_theme_style_overrides_the_defaults() {
        with_main_thread_stack(|| check_override(None, Some(override_style()), None));
    }

    // 'Overall Theme button theme style overrides defaults'
    #[test]
    fn the_overall_theme_style_overrides_the_defaults() {
        with_main_thread_stack(|| check_override(None, None, Some(override_style())));
    }

    // 'Button style overrides defaults, empty theme and overall styles'
    #[test]
    fn the_button_style_overrides_empty_theme_and_overall_styles() {
        with_main_thread_stack(|| {
            check_override(
                Some(override_style()),
                Some(ButtonStyle::new()),
                Some(ButtonStyle::new()),
            )
        });
    }

    // 'Button theme style overrides defaults, empty button and overall styles'
    #[test]
    fn the_button_theme_style_overrides_empty_button_and_overall_styles() {
        with_main_thread_stack(|| {
            check_override(
                Some(ButtonStyle::new()),
                Some(override_style()),
                Some(ButtonStyle::new()),
            )
        });
    }

    // 'Overall Theme button theme style overrides defaults, null theme and empty overall
    // style'
    #[test]
    fn the_overall_theme_style_overrides_an_empty_button_style() {
        with_main_thread_stack(|| {
            check_override(Some(ButtonStyle::new()), None, Some(override_style()))
        });
    }

    // 'Theme shadowColor'
    #[test]
    fn the_shadow_color_comes_from_the_button_then_its_theme_not_the_overall_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let shadow_color = Color::new(0xff000001);
            let overridden_color = Color::new(0xff000002);
            let build = |tester: &mut WidgetTester,
                         overall_shadow_color: Option<Color>,
                         theme_shadow_color: Option<Color>,
                         button_shadow_color: Option<Color>| {
                let mut theme = m3_theme(tester).copy_with();
                if let Some(color) = overall_shadow_color {
                    theme = theme.shadow_color(color);
                }
                let mut theme_style = IconButton::style_from();
                if let Some(color) = theme_shadow_color {
                    theme_style = theme_style.shadow_color(color);
                }
                let theme_style = theme_style.build();
                let widget = material_app(
                    theme.build(),
                    Center::new()
                        .child(IconButtonTheme::new(
                            IconButtonThemeData::new().style(theme_style),
                            Builder::new(move |_app, _context| {
                                let mut style = IconButton::style_from();
                                if let Some(color) = button_shadow_color {
                                    style = style.shadow_color(color);
                                }
                                IconButton::new(noop(), Icon::new(Some(Icons::add())))
                                    .style(style.build())
                                    .into_widget()
                            }),
                        ))
                        .into_widget(),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle(); // theme animation
                below_button(tester, |material: &Material| material.shadow_color)
            };

            assert_eq!(
                build(&mut tester, None, None, None),
                Some(Colors::TRANSPARENT)
            ); //default
            assert_eq!(
                build(&mut tester, Some(shadow_color), None, None),
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(
                build(&mut tester, None, Some(shadow_color), None),
                Some(shadow_color)
            );
            assert_eq!(
                build(&mut tester, None, None, Some(shadow_color)),
                Some(shadow_color)
            );
            assert_eq!(
                build(
                    &mut tester,
                    Some(overridden_color),
                    Some(shadow_color),
                    None
                ),
                Some(shadow_color)
            );
            assert_eq!(
                build(
                    &mut tester,
                    None,
                    Some(overridden_color),
                    Some(shadow_color)
                ),
                Some(shadow_color)
            );
        });
    }

    // 'IconButtonTheme IconButton.styleFrom overlayColor overrides default overlay color'
    #[test]
    fn the_theme_style_from_overlay_color_overrides_the_default_overlay() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let overlay_color = Color::new(0xffff0000);
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().style(
                            IconButton::style_from()
                                .overlay_color(overlay_color)
                                .build(),
                        ),
                        IconButton::new(noop(), Icon::new(Some(Icons::add()))),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            let rects = |tester: &WidgetTester| -> Vec<u32> {
                tester
                    .ink_drawn()
                    .into_iter()
                    .filter_map(|drawn| match drawn {
                        Drawn::Rect(color, _) => Some(color),
                        _ => None,
                    })
                    .collect()
            };

            // Hovered.
            let render = tester.render_box_of(icon_button_element(&tester));
            let center = tester.global_rect(render).center();
            let mut mouse = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(&mut mouse, center, false);
            tester.pump_and_settle();
            assert_eq!(
                rects(&tester),
                vec![overlay_color.with_opacity(0.08).value()]
            );

            // Highlighted (pressed).
            tester.send(&mouse, waft_embedder::PointerChange::Down);
            tester.pump_and_settle();
            assert_eq!(
                rects(&tester),
                vec![
                    overlay_color.with_opacity(0.08).value(),
                    overlay_color.with_opacity(0.1).value()
                ]
            );
            // Remove pressed and hovered states,
            tester.up(&mouse);
            tester.pump_and_settle();
            tester.move_to(&mut mouse, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Focused.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();
            assert_eq!(
                rects(&tester),
                vec![overlay_color.with_opacity(0.1).value()]
            );
        });
    }

    const BUTTON_KEY: &str = "button";

    /// Dart's `squareButton`, keyed.
    fn square_button(key: &KeyRef) -> IconButton {
        IconButton::new(noop(), Icon::new(Some(Icons::add())))
            .key(key.clone())
            .style(ButtonStyle::new().shape_variant(ButtonShapeVariant::Square))
    }

    fn custom_style() -> ButtonStyle {
        ButtonStyle::new().foreground_color(Rc::new(WidgetStatePropertyAll::new(Some(Color::new(
            0xffff0000,
        )))))
    }

    fn is_stadium(tester: &WidgetTester) -> bool {
        below_button(tester, |material: &Material| {
            material
                .shape
                .as_ref()
                .is_some_and(|shape| shape.as_any().is::<StadiumBorder>())
        })
    }

    fn is_rounded_rectangle(tester: &WidgetTester) -> bool {
        below_button(tester, |material: &Material| {
            material
                .shape
                .as_ref()
                .is_some_and(|shape| shape.as_any().is::<RoundedRectangleBorder>())
        })
    }

    /// Dart's `buttonWithVariant`: the square button, reporting the variant at its place.
    fn button_with_variant(
        key: &KeyRef,
        variant: &Rc<std::cell::Cell<Option<StyleVariant>>>,
    ) -> WidgetRef {
        let key = key.clone();
        let variant = Rc::clone(variant);
        Builder::new(move |app: &mut App, context| {
            variant.set(Some(IconButtonTheme::variant_of(app, context)));
            square_button(&key).into_widget()
        })
        .into_widget()
    }

    fn key() -> KeyRef {
        waft_foundation::ValueKey::new(BUTTON_KEY.to_owned()).into()
    }

    fn expressive_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .icon_button_theme(
                IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
            )
            .build(&tester.cell.borrow())
    }

    // 'IconButtonTheme.variantOf': 'returns material3 when no theme specifies a variant'
    #[test]
    fn variant_of_is_material3_when_no_theme_specifies_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().style(custom_style()),
                        button_with_variant(&key(), &variant),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3));
            assert!(is_stadium(&tester));
        });
    }

    // 'returns the variant of ThemeData.iconButtonTheme'
    #[test]
    fn variant_of_reads_the_theme_data_icon_button_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let widget = material_app(
                expressive_theme(&tester),
                Center::new()
                    .child(button_with_variant(&key(), &variant))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // 'a nested IconButtonTheme without a variant inherits it from an IconButtonTheme'
    #[test]
    fn a_nested_theme_without_a_variant_inherits_the_outer_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                        IconButtonTheme::new(
                            IconButtonThemeData::new().style(custom_style()),
                            button_with_variant(&key(), &variant),
                        ),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // 'an IconButtonTheme without a variant inherits it from ThemeData.iconButtonTheme'
    #[test]
    fn a_theme_without_a_variant_inherits_the_theme_data_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let widget = material_app(
                expressive_theme(&tester),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().style(custom_style()),
                        button_with_variant(&key(), &variant),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // 'an explicit variant overrides the variant of the ancestors'
    #[test]
    fn an_explicit_variant_overrides_the_ancestors() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                        IconButtonTheme::new(
                            IconButtonThemeData::new()
                                .style(custom_style())
                                .variant(StyleVariant::Material3),
                            button_with_variant(&key(), &variant),
                        ),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3));
            assert!(is_stadium(&tester));
        });
    }

    // 'an enclosing IconButtonTheme takes precedence over a closer Theme'
    #[test]
    fn an_enclosing_icon_button_theme_precedes_a_closer_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = Rc::new(std::cell::Cell::new(None));
            let closer = ThemeData::builder()
                .icon_button_theme(IconButtonThemeData::new().variant(StyleVariant::Material3))
                .build(&tester.cell.borrow());
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(IconButtonTheme::new(
                        IconButtonThemeData::new().variant(StyleVariant::Material3Expressive),
                        Theme::new(closer, button_with_variant(&key(), &variant)),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert_eq!(variant.get(), Some(StyleVariant::Material3Expressive));
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // 'IconButton is rebuilt when an IconButtonTheme above the closest one changes'
    #[test]
    fn the_button_rebuilds_when_a_farther_theme_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let variant = tester
                .cell
                .borrow_mut()
                .create(ValueNotifier::new(Some(StyleVariant::Material3Expressive)));
            // The subtree is made once, so it is only rebuilt through its
            // dependency on the outer IconButtonTheme.
            let child = IconButtonTheme::new(
                IconButtonThemeData::new().style(custom_style()),
                square_button(&key()),
            )
            .into_widget();
            let widget = material_app(
                ThemeData::builder().build(&tester.cell.borrow()),
                Center::new()
                    .child(
                        ValueListenableBuilder::new(
                            Rc::new(variant),
                            |_app, _context, value: &Option<StyleVariant>, child| {
                                let mut data = IconButtonThemeData::new();
                                data.variant = *value;
                                IconButtonTheme::new(data, child.expect("a child").clone())
                                    .into_widget()
                            },
                        )
                        .child(child),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert!(is_rounded_rectangle(&tester));

            variant.set_value(&mut tester.cell.borrow_mut(), Some(StyleVariant::Material3));
            tester.pump_and_settle();
            assert!(is_stadium(&tester));
        });
    }

    // 'IconButton in a ListTile uses the inherited variant'
    #[test]
    fn an_icon_button_in_a_list_tile_uses_the_inherited_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = material_app(
                expressive_theme(&tester),
                Material::new()
                    .child(
                        crate::list_tile::ListTile::new()
                            .title(waft_widgets::Text::new("Title"))
                            .trailing(square_button(&key())),
                    )
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert!(is_rounded_rectangle(&tester));
        });
    }

    // 'IconButton in an AppBar with a custom foregroundColor uses the inherited variant'
    #[test]
    fn an_icon_button_in_an_app_bar_with_a_foreground_color_uses_the_inherited_variant() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = material_app(
                expressive_theme(&tester),
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new()
                            .foreground_color(Color::new(0xffff0000))
                            .actions(vec![square_button(&key()).into_widget()]),
                    ))
                    .into_widget(),
            );
            tester.pump_widget(widget);
            assert!(is_rounded_rectangle(&tester));
        });
    }
}

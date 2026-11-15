//! Flutter counterpart: material_ui's `dropdown_menu_theme.dart`.

use std::fmt::{self, Debug};

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::TextStyle;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::input_decorator::InputDecorationThemeData;
use crate::menu_style::MenuStyle;
use crate::theme::Theme;

/// Overrides the default values of visual properties for descendant `DropdownMenu` widgets.
///
/// Descendant widgets obtain the current [`DropdownMenuThemeData`] object with
/// [`DropdownMenuTheme::of`]. Instances of [`DropdownMenuThemeData`] can be customized with
/// [`DropdownMenuThemeData::copy_with`].
///
/// Typically a [`DropdownMenuTheme`] is specified as part of the overall `Theme` with
/// `ThemeData::dropdown_menu_theme`.
///
/// All [`DropdownMenuThemeData`] properties are null by default. When null, the
/// `DropdownMenu` computes its own default values, typically based on the overall theme's
/// `ThemeData::color_scheme`, `ThemeData::text_theme`, and `ThemeData::icon_theme`.
#[derive(Clone, Default)]
pub struct DropdownMenuThemeData {
    /// Overrides the default value for `DropdownMenu::text_style`.
    pub text_style: Option<TextStyle>,

    /// The input decoration theme for the `TextField`s in a `DropdownMenu`.
    ///
    /// If this is null, the `DropdownMenu` provides its own defaults.
    pub input_decoration_theme: Option<InputDecorationThemeData>,

    /// Overrides the menu's default style in a `DropdownMenu`.
    ///
    /// Any values not set in the [`MenuStyle`] will use the menu default for that property.
    pub menu_style: Option<MenuStyle>,

    /// The color used for disabled `DropdownMenu`.
    ///
    /// If this is null, the `DropdownMenu` uses `ColorScheme::on_surface` with 0.38 opacity.
    pub disabled_color: Option<Color>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `DropdownMenuThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait DropdownMenuThemeDataGetters for DropdownMenuThemeData, fields: dropdown_menu_theme_data {
        text_style: Option<TextStyle>,
        input_decoration_theme: Option<InputDecorationThemeData>,
        menu_style: Option<MenuStyle>,
        disabled_color: Option<Color>,
    }
}

impl DropdownMenuThemeData {
    /// Creates a [`DropdownMenuThemeData`] that can be used to override default properties in
    /// a [`DropdownMenuTheme`] widget.
    pub fn new() -> DropdownMenuThemeData {
        DropdownMenuThemeData::default()
    }

    /// Dart `DropdownMenuThemeData(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> DropdownMenuThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `DropdownMenuThemeData(inputDecorationTheme:)`.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> DropdownMenuThemeData {
        self.input_decoration_theme = Some(input_decoration_theme.into());
        self
    }

    /// Dart `DropdownMenuThemeData(menuStyle:)`.
    pub fn menu_style(mut self, menu_style: MenuStyle) -> DropdownMenuThemeData {
        self.menu_style = Some(menu_style);
        self
    }

    /// Dart `DropdownMenuThemeData(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> DropdownMenuThemeData {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the new values.
    pub fn copy_with(&self) -> DropdownMenuThemeData {
        self.clone()
    }

    /// Linearly interpolates between two dropdown menu themes.
    pub fn lerp(
        a: Option<&DropdownMenuThemeData>,
        b: Option<&DropdownMenuThemeData>,
        t: f64,
    ) -> DropdownMenuThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        DropdownMenuThemeData {
            text_style: TextStyle::lerp(
                a.and_then(|a| a.text_style.as_ref()),
                b.and_then(|b| b.text_style.as_ref()),
                t,
            ),
            input_decoration_theme: if t < 0.5 {
                a.and_then(|a| a.input_decoration_theme.clone())
            } else {
                b.and_then(|b| b.input_decoration_theme.clone())
            },
            menu_style: MenuStyle::lerp(
                a.and_then(|a| a.menu_style.as_ref()),
                b.and_then(|b| b.menu_style.as_ref()),
                t,
            ),
            disabled_color: Color::lerp(
                a.and_then(|a| a.disabled_color),
                b.and_then(|b| b.disabled_color),
                t,
            ),
        }
    }
}

impl PartialEq for DropdownMenuThemeData {
    fn eq(&self, other: &DropdownMenuThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.text_style == self.text_style
            && other.input_decoration_theme == self.input_decoration_theme
            && other.menu_style == self.menu_style
            && other.disabled_color == self.disabled_color
    }
}

impl Debug for DropdownMenuThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("DropdownMenuThemeData");
        if let Some(text_style) = &self.text_style {
            properties.field("textStyle", text_style);
        }
        if self.input_decoration_theme.is_some() {
            properties.field("inputDecorationThemeData", &"InputDecorationThemeData");
        }
        if let Some(menu_style) = &self.menu_style {
            properties.field("menuStyle", menu_style);
        }
        if let Some(disabled_color) = &self.disabled_color {
            properties.field("disabledColor", disabled_color);
        }
        properties.finish()
    }
}

/// An inherited widget that defines the visual properties for `DropdownMenu`s in this
/// widget's subtree.
///
/// Values specified here are used for `DropdownMenu` properties that are not given an
/// explicit non-null value.
#[derive(Debug)]
pub struct DropdownMenuTheme {
    pub key: Option<KeyRef>,
    /// Specifies the visual properties used by descendant `DropdownMenu` widgets.
    pub data: DropdownMenuThemeData,
    pub child: WidgetRef,
}

impl DropdownMenuTheme {
    /// Creates a [`DropdownMenuTheme`] that controls visual parameters for descendant
    /// `DropdownMenu`s.
    pub fn new<K>(data: DropdownMenuThemeData, child: impl IntoWidget<K>) -> DropdownMenuTheme {
        DropdownMenuTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `DropdownMenuTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownMenuTheme {
        self.key = Some(key);
        self
    }

    /// Returns the closest instance of this class's [`data`](Self::data) value that encloses
    /// the given context.
    ///
    /// If there is no ancestor, it returns `ThemeData::dropdown_menu_theme`. Applications can
    /// assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = DropdownMenuTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> DropdownMenuThemeData {
        DropdownMenuTheme::maybe_of(app, context)
            .unwrap_or_else(|| Theme::of(app, context).dropdown_menu_theme.clone())
    }

    /// Returns the closest instance of this class's [`data`](Self::data) value that encloses
    /// the given context, if any.
    ///
    /// Use this function if you want to allow situations where no [`DropdownMenuTheme`] is in
    /// scope. Prefer using [`DropdownMenuTheme::of`] in situations where a
    /// [`DropdownMenuThemeData`] is expected to be non-null.
    ///
    /// If there is no [`DropdownMenuTheme`] in scope, then this function will return null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = DropdownMenuTheme::maybe_of(app, context);
    /// if theme.is_none() {
    ///   // Do something else instead.
    /// }
    /// ```
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<DropdownMenuThemeData> {
        context
            .depend_on_inherited_widget_of_exact_type::<DropdownMenuTheme>(app)
            .map(|theme| theme.data.clone())
    }
}

impl InheritedTheme for DropdownMenuTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DropdownMenuTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for DropdownMenuTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DropdownMenuTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use waft_widgets::WidgetStatePropertyAll;

    use super::*;
    use crate::colors::Colors;
    use crate::input_decorator::InputDecorationTheme;

    // dropdown_menu_theme_test.dart 'DropdownMenuThemeData copyWith, ==, hashCode basics'.
    #[test]
    fn a_copy_equals_its_original_and_a_copy_with_every_field_equals_a_theme_made_with_them() {
        assert_eq!(
            DropdownMenuThemeData::new(),
            DropdownMenuThemeData::new().copy_with()
        );

        let custom =
            DropdownMenuThemeData::new()
                .menu_style(MenuStyle::new().background_color(Rc::new(
                    WidgetStatePropertyAll::new(Some(Colors::GREEN.primary())),
                )))
                .input_decoration_theme(InputDecorationTheme::new().with_filled(true))
                .text_style(TextStyle::new().font_size(25.0));
        let copy = DropdownMenuThemeData::new()
            .copy_with()
            .menu_style(custom.menu_style.clone().expect("set above"))
            .input_decoration_theme(custom.input_decoration_theme.clone().expect("set above"))
            .text_style(custom.text_style.clone().expect("set above"));
        assert_eq!(copy, custom);
    }

    // dropdown_menu_theme_test.dart 'DropdownMenuThemeData lerp special cases'; a value has
    // no identity, so the second expectation compares by value.
    #[test]
    fn lerp_of_nothing_is_the_empty_theme_and_of_one_theme_with_itself_is_that_theme() {
        assert_eq!(
            DropdownMenuThemeData::lerp(None, None, 0.0),
            DropdownMenuThemeData::new()
        );
        let data = DropdownMenuThemeData::new();
        assert_eq!(
            DropdownMenuThemeData::lerp(Some(&data), Some(&data), 0.5),
            data
        );
    }

    // ---- the widget cases, on a `DropdownMenu` ----

    use waft_embedder::{Color, Radius};
    use waft_painting::{
        BorderRadiusGeometry, BorderSide, OutlinedBorderRef, RoundedRectangleBorder, Shadow,
        ShapeBorder, TextDecoration,
    };
    use waft_widgets::{AnyElement, EditableText};

    use crate::dropdown_menu::{DropdownMenu, DropdownMenuEntry};
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::menu_anchor::{MenuAnchor, MenuItemButton, MenuPanel};
    use crate::scaffold::Scaffold;
    use crate::shadows::k_elevation_to_shadow;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::tester;

    fn entries() -> Vec<DropdownMenuEntry<i32>> {
        vec![
            DropdownMenuEntry::new(0, "Item 0"),
            DropdownMenuEntry::new(1, "Item 1"),
            DropdownMenuEntry::new(2, "Item 2"),
        ]
    }

    fn shadows(elevation: i32) -> Vec<Shadow> {
        k_elevation_to_shadow(elevation)
            .expect("a defined elevation")
            .iter()
            .copied()
            .map(Shadow::from)
            .collect()
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(radius)),
        )
    }

    fn all<T: Clone + std::fmt::Debug + 'static>(
        value: T,
    ) -> Rc<WidgetStatePropertyAll<Option<T>>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    /// A theme's or a widget's `textStyle`, `menuStyle` and `inputDecorationTheme` of one of
    /// the cases.
    struct Styles {
        text_style: TextStyle,
        menu_style: MenuStyle,
        input_decoration_theme: InputDecorationThemeData,
    }

    /// The values the cases give, and what each expects to see.
    #[derive(Clone)]
    struct Expected {
        color: waft_painting::AnyColor,
        background_color: waft_painting::AnyColor,
        font_size: f64,
        elevation_shadows: i32,
        decoration: TextDecoration,
        word_spacing: f64,
        menu_background: Color,
        menu_shadow: Color,
        menu_surface_tint: Color,
        menu_elevation: f64,
        menu_radius: f64,
        fill_color: waft_painting::AnyColor,
    }

    impl Expected {
        fn styles(&self) -> Styles {
            let shape: OutlinedBorderRef = OutlinedBorderRef::new(rounded(self.menu_radius));
            Styles {
                text_style: TextStyle::new()
                    .color(self.color.clone())
                    .background_color(self.background_color.clone())
                    .font_size(self.font_size)
                    .shadows(shadows(self.elevation_shadows))
                    .decoration(self.decoration)
                    .word_spacing(self.word_spacing),
                menu_style: MenuStyle::new()
                    .background_color(all(self.menu_background))
                    .shadow_color(all(self.menu_shadow))
                    .surface_tint_color(all(self.menu_surface_tint))
                    .elevation(all(self.menu_elevation))
                    .shape(all(shape)),
                input_decoration_theme: InputDecorationTheme::new()
                    .with_filled(true)
                    .with_fill_color(self.fill_color.clone())
                    .into(),
            }
        }

        fn theme_data(&self) -> DropdownMenuThemeData {
            let styles = self.styles();
            DropdownMenuThemeData::new()
                .text_style(styles.text_style)
                .menu_style(styles.menu_style)
                .input_decoration_theme(styles.input_decoration_theme)
        }
    }

    /// Dart's `global`.
    fn global() -> Expected {
        Expected {
            color: Colors::ORANGE.into(),
            background_color: Colors::INDIGO.into(),
            font_size: 30.0,
            elevation_shadows: 1,
            decoration: TextDecoration::UNDERLINE,
            word_spacing: 2.0,
            menu_background: Colors::GREY.primary(),
            menu_shadow: Colors::BROWN.primary(),
            menu_surface_tint: Colors::AMBER_ACCENT.primary(),
            menu_elevation: 10.0,
            menu_radius: 10.0,
            fill_color: Colors::LIGHT_GREEN.into(),
        }
    }

    /// Dart's `dropdownMenuTheme`.
    fn local() -> Expected {
        Expected {
            color: Colors::RED.into(),
            background_color: Colors::ORANGE.into(),
            font_size: 27.0,
            elevation_shadows: 2,
            decoration: TextDecoration::LINE_THROUGH,
            word_spacing: 5.0,
            menu_background: Colors::YELLOW.primary(),
            menu_shadow: Colors::GREEN.primary(),
            menu_surface_tint: Colors::TEAL.primary(),
            menu_elevation: 15.0,
            menu_radius: 8.0,
            fill_color: Colors::BLUE.into(),
        }
    }

    /// The widget's own values in 'Widget parameters overrides …'.
    fn parameters() -> Expected {
        Expected {
            color: Colors::PINK.into(),
            background_color: Colors::CYAN.into(),
            font_size: 32.0,
            elevation_shadows: 3,
            decoration: TextDecoration::OVERLINE,
            word_spacing: 3.0,
            menu_background: Colors::LIME_ACCENT.primary(),
            menu_shadow: Colors::DEEP_ORANGE_ACCENT.primary(),
            menu_surface_tint: Colors::LIGHT_BLUE.primary(),
            menu_elevation: 21.0,
            menu_radius: 15.0,
            fill_color: Colors::DEEP_PURPLE.into(),
        }
    }

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    /// Dart's `find.widgetWithIcon(IconButton, Icons.arrow_drop_down).first`.
    fn arrow_drop_down_button(tester: &WidgetTester) -> AnyElement {
        let icon = tester.find_by_icon(&Icons::arrow_drop_down());
        tester.ancestor_of_type::<IconButton>(icon)
    }

    /// Dart's `getMenuMaterial`.
    fn menu_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let panel = only(tester.find_by_type::<MenuPanel>());
        let material = tester.descendants_of_type::<Material>(panel)[0];
        tester.widget::<Material, R>(material, read)
    }

    /// Dart's `getButtonMaterial`: the `Material` of the last `MenuItemButton` showing `label`.
    fn button_material<R>(
        tester: &WidgetTester,
        label: &str,
        read: impl FnOnce(&Material) -> R,
    ) -> R {
        let text = *tester.find_text(label).last().expect("the label");
        let button = tester.ancestor_of_type::<MenuItemButton>(text);
        let material = only(tester.descendants_of_type::<Material>(button));
        tester.widget::<Material, R>(material, read)
    }

    fn shape_is(material: &Material, expected: &dyn ShapeBorder) -> bool {
        material
            .shape
            .as_ref()
            .is_some_and(|shape| shape.eq_shape(expected))
    }

    fn tap(tester: &mut WidgetTester, element: AnyElement) {
        let render = tester.render_box_of(element);
        let center = tester.get_rect(render).center();
        tester.tap(center, 1);
    }

    /// What the cases but the first expect of the field, the menu and its first item.
    fn assert_the_menu_shows(tester: &mut WidgetTester, theme: &ThemeData, expected: &Expected) {
        let editable = only(tester.find_by_type::<EditableText>());
        tester.widget(editable, |editable: &EditableText| {
            assert_eq!(editable.style.color, Some(expected.color.clone()));
            assert_eq!(
                editable.style.background_color,
                Some(expected.background_color.clone())
            );
            assert_eq!(editable.style.font_size, Some(expected.font_size));
            assert_eq!(
                editable.style.shadows,
                Some(shadows(expected.elevation_shadows))
            );
            assert_eq!(editable.style.decoration, Some(expected.decoration));
            assert_eq!(editable.style.word_spacing, Some(expected.word_spacing));
        });

        let text_field = only(tester.find_by_type::<TextField>());
        tester.widget(text_field, |text_field: &TextField| {
            let decoration = text_field.decoration.as_ref().expect("a decoration");
            assert_eq!(decoration.filled, Some(true));
            assert_eq!(decoration.fill_color, Some(expected.fill_color.clone()));
        });

        let button = arrow_drop_down_button(tester);
        tap(tester, button);
        tester.pump(std::time::Duration::ZERO);
        assert_eq!(tester.find_by_type::<MenuAnchor>().len(), 1);

        menu_material(tester, |material| {
            assert_eq!(material.color, Some(expected.menu_background));
            assert_eq!(material.shadow_color, Some(expected.menu_shadow));
            assert_eq!(
                material.surface_tint_color,
                Some(expected.menu_surface_tint)
            );
            assert_eq!(material.elevation, expected.menu_elevation);
            assert!(shape_is(material, &rounded(expected.menu_radius)));
        });

        button_material(tester, "Item 0", |material| {
            assert_eq!(material.color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 0.0);
            assert!(shape_is(material, &RoundedRectangleBorder::default()));
            let text_style = material.text_style.as_ref().expect("a text style");
            assert_eq!(text_style.color, Some(theme.color_scheme.on_surface.into()));
        });
    }

    fn centered(menu: DropdownMenu<i32>) -> WidgetRef {
        Scaffold::new()
            .body(waft_widgets::Center::new().child(menu))
            .into_widget()
    }

    // dropdown_menu_theme_test.dart 'With no other configuration, defaults are used'.
    #[test]
    fn with_no_other_configuration_the_defaults_are_used() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                centered(DropdownMenu::new(entries())),
            ));

            let label_large = theme.text_theme.label_large.clone().expect("a label large");
            let editable = only(tester.find_by_type::<EditableText>());
            tester.widget(editable, |editable: &EditableText| {
                assert_eq!(editable.style.color, label_large.color);
                assert_eq!(editable.style.background, label_large.background);
                assert_eq!(editable.style.shadows, label_large.shadows);
                assert_eq!(editable.style.decoration, label_large.decoration);
                assert_eq!(editable.style.word_spacing, label_large.word_spacing);
            });

            let text_field = only(tester.find_by_type::<TextField>());
            tester.widget(text_field, |text_field: &TextField| {
                let decoration = text_field.decoration.as_ref().expect("a decoration");
                assert!(decoration.border.as_ref().is_some_and(|border| {
                    border.eq_shape(&crate::input_border::OutlineInputBorder::new())
                }));
            });

            let button = arrow_drop_down_button(&tester);
            tap(&mut tester, button);
            tester.pump(std::time::Duration::ZERO);
            assert_eq!(tester.find_by_type::<MenuAnchor>().len(), 1);

            menu_material(&tester, |material| {
                assert_eq!(material.color, Some(theme.color_scheme.surface_container()));
                assert_eq!(material.shadow_color, Some(theme.color_scheme.shadow()));
                assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 3.0);
                assert!(shape_is(material, &rounded(4.0)));
            });

            button_material(&tester, "Item 0", |material| {
                assert_eq!(material.color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 0.0);
                assert!(shape_is(material, &RoundedRectangleBorder::default()));
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(text_style.color, Some(theme.color_scheme.on_surface.into()));
            });
        });
    }

    // dropdown_menu_theme_test.dart 'ThemeData.dropdownMenuTheme overrides defaults'.
    #[test]
    fn the_theme_data_dropdown_menu_theme_overrides_the_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expected = global();
            let theme = ThemeData::builder()
                .dropdown_menu_theme(expected.theme_data())
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                centered(DropdownMenu::new(entries())),
            ));
            assert_the_menu_shows(&mut tester, &theme, &expected);
        });
    }

    // dropdown_menu_theme_test.dart 'DropdownMenuTheme overrides ThemeData and defaults'.
    #[test]
    fn a_dropdown_menu_theme_overrides_the_theme_data_and_the_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expected = local();
            let theme = ThemeData::builder()
                .dropdown_menu_theme(global().theme_data())
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                DropdownMenuTheme::new(
                    expected.theme_data(),
                    centered(DropdownMenu::new(entries())),
                )
                .into_widget(),
            ));
            assert_the_menu_shows(&mut tester, &theme, &expected);
        });
    }

    // dropdown_menu_theme_test.dart 'Widget parameters overrides DropdownMenuTheme, ThemeData
    // and defaults'.
    #[test]
    fn the_widget_parameters_override_the_themes_and_the_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expected = parameters();
            let theme = ThemeData::builder()
                .dropdown_menu_theme(global().theme_data())
                .build(&tester.cell.borrow());
            let styles = expected.styles();
            tester.pump_widget(material_app(
                theme.clone(),
                DropdownMenuTheme::new(
                    local().theme_data(),
                    centered(
                        DropdownMenu::new(entries())
                            .text_style(styles.text_style)
                            .menu_style(styles.menu_style)
                            .input_decoration_theme(styles.input_decoration_theme),
                    ),
                )
                .into_widget(),
            ));
            assert_the_menu_shows(&mut tester, &theme, &expected);
        });
    }

    // dropdown_menu_theme_test.dart 'DropdownMenuThemeData.menuStyle.disabledColor is being
    // applied when the menu is disabled'.
    #[test]
    fn the_theme_disabled_color_colours_a_disabled_menu() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = ThemeData::builder()
                .dropdown_menu_theme(
                    DropdownMenuThemeData::new().disabled_color(Colors::GREY.primary()),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                centered(
                    DropdownMenu::new(entries())
                        .enabled(false)
                        .initial_selection(0),
                ),
            ));

            // make sure the displaying text has grey color
            let editable = only(tester.find_by_type::<EditableText>());
            tester.widget(editable, |editable: &EditableText| {
                assert_eq!(editable.style.color, Some(Colors::GREY.primary().into()));
            });
        });
    }
}

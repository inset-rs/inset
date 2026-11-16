//! Flutter counterpart: material_ui's `menu_bar_theme.dart`.

use std::ops::Deref;

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::menu_style::MenuStyle;
use crate::menu_theme::MenuThemeData;
use crate::theme::Theme;

/// A data class that [`MenuBarTheme`] uses to define the visual properties of
/// `MenuBar` widgets.
///
/// This class defines the visual properties of `MenuBar` widgets themselves,
/// but not their submenus. Those properties are defined by [`MenuThemeData`] or
/// `MenuButtonThemeData` instead.
///
/// Descendant widgets obtain the current [`MenuBarThemeData`] object using
/// [`MenuBarTheme::of`].
///
/// Typically, a [`MenuBarThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::menu_bar_theme`. Otherwise, `MenuTheme` can be used to
/// configure its own widget subtree.
///
/// All [`MenuBarThemeData`] properties are `null` by default. If any of these
/// properties are null, the menu bar will provide its own defaults.
///
/// Dart's `MenuBarThemeData extends MenuThemeData`: it holds the [`MenuThemeData`] it
/// derefs to, whose `submenu_icon` its constructor never sets.
///
/// See also:
///
/// * [`MenuThemeData`], which describes the theme for the submenus of a
///   `MenuBar`.
/// * `MenuButtonThemeData`, which describes the theme for the `MenuItemButton`s
///   in a menu.
/// * `ThemeData`, which describes the overall theme for the application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuBarThemeData {
    menu_theme_data: MenuThemeData,
}

impl MenuBarThemeData {
    /// Creates a const set of properties used to configure `MenuTheme`.
    pub fn new() -> MenuBarThemeData {
        MenuBarThemeData::default()
    }

    /// Dart `MenuBarThemeData(style:)`.
    pub fn style(mut self, style: MenuStyle) -> MenuBarThemeData {
        self.menu_theme_data.style = Some(style);
        self
    }

    /// Linearly interpolate between two `MenuBar` themes.
    pub fn lerp(
        a: Option<&MenuBarThemeData>,
        b: Option<&MenuBarThemeData>,
        t: f64,
    ) -> Option<MenuBarThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        let mut data = MenuBarThemeData::new();
        if let Some(style) = MenuStyle::lerp(
            a.and_then(|a| a.style.as_ref()),
            b.and_then(|b| b.style.as_ref()),
            t,
        ) {
            data = data.style(style);
        }
        Some(data)
    }
}

impl Deref for MenuBarThemeData {
    type Target = MenuThemeData;

    fn deref(&self) -> &MenuThemeData {
        &self.menu_theme_data
    }
}

/// An inherited widget that defines the configuration for the `MenuBar` widgets
/// in this widget's descendants.
///
/// This class defines the visual properties of `MenuBar` widgets themselves,
/// but not their submenus. Those properties are defined by `MenuTheme` or
/// `MenuButtonTheme` instead.
///
/// Values specified here are used for `MenuBar`'s properties that are not given
/// an explicit non-null value.
///
/// See also:
/// * [`MenuStyle`], a configuration object that holds attributes of a menu, and
///   is used by this theme to define those attributes.
/// * `MenuTheme`, which does the same thing for the menus created by a
///   `SubmenuButton` or `MenuAnchor`.
/// * `MenuButtonTheme`, which does the same thing for the `MenuItemButton`s
///   inside of the menus.
/// * `SubmenuButton`, a button that manages a submenu that uses these
///   properties.
/// * `MenuBar`, a widget that creates a menu bar that can use `SubmenuButton`s.
#[derive(Debug)]
pub struct MenuBarTheme {
    pub key: Option<KeyRef>,
    /// The properties to set for `MenuBar` in this widget's descendants.
    pub data: MenuBarThemeData,
    pub child: WidgetRef,
}

impl MenuBarTheme {
    /// Creates a theme that controls the configurations for `MenuBar` and
    /// `MenuItemButton` in its widget subtree.
    pub fn new<K>(data: MenuBarThemeData, child: impl IntoWidget<K>) -> MenuBarTheme {
        MenuBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `MenuBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuBarTheme {
        self.key = Some(key);
        self
    }

    /// Returns the closest instance of this class's [`data`](Self::data) value that encloses
    /// the given context. If there is no ancestor, it returns
    /// `ThemeData::menu_bar_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = MenuBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> MenuBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<MenuBarTheme>(app) {
            Some(menu_bar_theme) => menu_bar_theme.data.clone(),
            None => Theme::of(app, context).menu_bar_theme.clone(),
        }
    }
}

impl InheritedTheme for MenuBarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        MenuBarTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for MenuBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuBarTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {

    use waft_embedder::Rect;
    use waft_rendering::BoxConstraints;
    use waft_widgets::{Axis, Builder, Column, ConstrainedBox, Expanded};

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::menu_anchor::MenuBar;
    use crate::menu_theme::MenuTheme;
    use crate::menu_theme::tests::{
        ThemeTestMenus, assert_constructor_parameters_override_theme_parameters, green_menu_style,
        menu_panels, overriding_bar, panel_material, red_stadium_menu_style, test_menu,
        theme_test_menus,
    };
    use crate::test_support::material_app;
    use crate::tooltip::tests::m2;

    /// The bar of `menu_bar_theme_test.dart`'s two widget cases: `MenuTheme(menu_style)` over
    /// `MenuBarTheme(bar_style)` over `Column([bar, Expanded(Placeholder())])`.
    fn pump_themed_bar(
        tester: &mut WidgetTester,
        menu_theme: MenuThemeData,
        bar_theme: MenuBarThemeData,
        bar: impl Fn() -> WidgetRef + 'static,
    ) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                m2(&app),
                Material::new()
                    .child(Builder::new(move |_app, _context| {
                        MenuTheme::new(
                            menu_theme.clone(),
                            MenuBarTheme::new(
                                bar_theme.clone(),
                                Column::new().children(vec![
                                    bar(),
                                    Expanded::new(ConstrainedBox::new(BoxConstraints::expand(
                                        None, None,
                                    )))
                                    .into_widget(),
                                ]),
                            ),
                        )
                        .into_widget()
                    }))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    // menu_bar_theme_test.dart 'MenuBarThemeData lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_theme_with_itself_is_that_theme() {
        assert!(MenuBarThemeData::lerp(None, None, 0.0).is_none());
        let data = MenuBarThemeData::new();
        assert_eq!(
            MenuBarThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // menu_bar_theme_test.dart 'theme is honored'.
    #[test]
    fn a_menu_bar_theme_styles_the_bar_and_a_menu_theme_the_submenus() {
        test_widgets(|tester| {
            pump_themed_bar(
                tester,
                MenuThemeData::new().style(green_menu_style()),
                MenuBarThemeData::new().style(red_stadium_menu_style()),
                || MenuBar::new(theme_test_menus(ThemeTestMenus::default())).into_widget(),
            );

            // Open a test menu.
            tester.tap(find::text(test_menu::MAIN_MENU_1));
            tester.pump();
            assert_eq!(
                tester.get_rect(menu_panels(Axis::Horizontal).first()),
                Rect::from_ltrb(228.0, 0.0, 572.0, 68.0)
            );
            panel_material(tester, Axis::Horizontal, |material| {
                assert_eq!(material.elevation, 15.0);
                assert_eq!(material.color, Some(Colors::RED.primary()));
            });

            assert_eq!(
                tester.get_rect(menu_panels(Axis::Vertical)),
                Rect::from_ltrb(346.0, 58.0, 560.0, 218.0)
            );
            panel_material(tester, Axis::Vertical, |material| {
                assert_eq!(material.elevation, 20.0);
                assert_eq!(material.color, Some(Colors::GREEN.primary()));
            });
        });
    }

    // menu_bar_theme_test.dart 'Constructor parameters override theme parameters'.
    #[test]
    fn a_menu_bars_and_its_buttons_own_styles_override_the_themes() {
        test_widgets(|tester| {
            pump_themed_bar(
                tester,
                MenuThemeData::new().style(green_menu_style()),
                MenuBarThemeData::new().style(red_stadium_menu_style()),
                overriding_bar,
            );

            // Open a test menu.
            tester.tap(find::text(test_menu::MAIN_MENU_1));
            tester.pump();

            assert_constructor_parameters_override_theme_parameters(tester);
        });
    }
}

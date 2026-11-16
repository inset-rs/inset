//! Flutter counterpart: material_ui's `menu_theme.dart`.

use std::fmt::{self, Debug};

use waft_foundation::App;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStatePropertyRef,
};

use crate::menu_style::MenuStyle;
use crate::theme::Theme;

/// Defines the configuration of the submenus created by the `SubmenuButton`,
/// `MenuBar`, or `MenuAnchor` widgets.
///
/// Descendant widgets obtain the current [`MenuThemeData`] object using
/// [`MenuTheme::of`].
///
/// Typically, a [`MenuThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::menu_theme`. Otherwise, [`MenuTheme`] can be used to configure
/// its own widget subtree.
///
/// All [`MenuThemeData`] properties are `null` by default. If any of these
/// properties are null, the menu bar will provide its own defaults.
///
/// See also:
///
/// * `ThemeData`, which describes the overall theme for the application.
/// * `MenuBarThemeData`, which describes the theme for the menu bar itself in a
///   `MenuBar` widget.
#[derive(Clone, Default)]
pub struct MenuThemeData {
    /// The [`MenuStyle`] of a `SubmenuButton` menu.
    ///
    /// Any values not set in the [`MenuStyle`] will use the menu default for that
    /// property.
    pub style: Option<MenuStyle>,

    /// If provided, the widget replaces the default `SubmenuButton` arrow icon.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    pub submenu_icon: Option<WidgetStatePropertyRef<Option<WidgetRef>>>,
}

impl MenuThemeData {
    /// Creates a const set of properties used to configure [`MenuTheme`].
    pub fn new() -> MenuThemeData {
        MenuThemeData::default()
    }

    /// Dart `MenuThemeData(style:)`.
    pub fn style(mut self, style: MenuStyle) -> MenuThemeData {
        self.style = Some(style);
        self
    }

    /// Dart `MenuThemeData(submenuIcon:)`.
    pub fn submenu_icon(
        mut self,
        submenu_icon: WidgetStatePropertyRef<Option<WidgetRef>>,
    ) -> MenuThemeData {
        self.submenu_icon = Some(submenu_icon);
        self
    }

    /// Linearly interpolate between two menu button themes.
    pub fn lerp(
        a: Option<&MenuThemeData>,
        b: Option<&MenuThemeData>,
        t: f64,
    ) -> Option<MenuThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(MenuThemeData {
            style: MenuStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
            submenu_icon: if t < 0.5 {
                a.and_then(|a| a.submenu_icon.clone())
            } else {
                b.and_then(|b| b.submenu_icon.clone())
            },
        })
    }
}

impl PartialEq for MenuThemeData {
    fn eq(&self, other: &MenuThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.style == self.style && other.submenu_icon == self.submenu_icon
    }
}

impl Debug for MenuThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("MenuThemeData");
        if let Some(style) = &self.style {
            properties.field("style", style);
        }
        if let Some(submenu_icon) = &self.submenu_icon {
            properties.field("submenuIcon", submenu_icon);
        }
        properties.finish()
    }
}

/// An inherited widget that defines the configuration in this widget's
/// descendants for menus created by the `SubmenuButton`, `MenuBar`, or
/// `MenuAnchor` widgets.
///
/// Values specified here are used for `SubmenuButton`'s menu properties that
/// are not given an explicit non-null value.
///
/// See also:
///
/// * [`MenuThemeData`], a configuration object that holds attributes of a menu
///   used by this theme.
/// * `MenuBarTheme`, which does the same thing for the `MenuBar` widget.
/// * `MenuBar`, a widget that manages `MenuItemButton`s.
/// * `MenuAnchor`, a widget that creates a region that has a submenu.
/// * `MenuItemButton`, a widget that is a selectable item in a menu bar menu.
/// * `SubmenuButton`, a widget that specifies an item with a cascading submenu
///   in a `MenuBar` menu.
#[derive(Debug)]
pub struct MenuTheme {
    pub key: Option<KeyRef>,
    /// The properties for `MenuBar` and `MenuItemButton` in this widget's
    /// descendants.
    pub data: MenuThemeData,
    pub child: WidgetRef,
}

impl MenuTheme {
    /// Creates a const theme that controls the configurations for the menus
    /// created by the `SubmenuButton` or `MenuAnchor` widgets.
    pub fn new<K>(data: MenuThemeData, child: impl IntoWidget<K>) -> MenuTheme {
        MenuTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `MenuTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuTheme {
        self.key = Some(key);
        self
    }

    /// Returns the closest instance of this class's [`data`](Self::data) value that encloses
    /// the given context. If there is no ancestor, it returns
    /// `ThemeData::menu_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = MenuTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> MenuThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<MenuTheme>(app) {
            Some(menu_theme) => menu_theme.data.clone(),
            None => Theme::of(app, context).menu_theme.clone(),
        }
    }
}

impl InheritedTheme for MenuTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        MenuTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for MenuTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::rc::Rc;

    use waft_embedder::{Color, Rect};
    use waft_painting::{
        BeveledRectangleBorder, EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorder, StadiumBorder,
    };
    use waft_rendering::BoxConstraints;
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        AnyElement, Axis, Builder, Column, ConstrainedBox, DefaultTextStyle, Expanded, Icon,
        IconData, Text, WidgetState, WidgetStateProperty, WidgetStatePropertyAll, WidgetStates,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::button_style::ButtonStyle;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::menu_anchor::{MenuBar, MenuItemButton, MenuPanel, SubmenuButton};
    use crate::menu_bar_theme::{MenuBarTheme, MenuBarThemeData};
    use crate::test_support::material_app;
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m2;

    /// The labels of the menus these tests build: Dart's `TestMenu` of `menu_theme_test.dart`
    /// and `menu_bar_theme_test.dart`.
    pub(crate) mod test_menu {
        pub(crate) const MAIN_MENU_0: &str = "Menu 0";
        pub(crate) const MAIN_MENU_1: &str = "Menu 1";
        pub(crate) const MAIN_MENU_2: &str = "Menu 2";
        pub(crate) const SUB_MENU_00: &str = "Sub Menu 00";
        pub(crate) const SUB_MENU_10: &str = "Sub Menu 10";
        pub(crate) const SUB_MENU_11: &str = "Sub Menu 11";
        pub(crate) const SUB_MENU_12: &str = "Sub Menu 12";
        pub(crate) const SUB_MENU_20: &str = "Sub Menu 20";
        pub(crate) const SUB_SUB_MENU_110: &str = "Sub Sub Menu 110";
        pub(crate) const SUB_SUB_MENU_111: &str = "Sub Sub Menu 111";
        pub(crate) const SUB_SUB_MENU_112: &str = "Sub Sub Menu 112";
        pub(crate) const SUB_SUB_MENU_113: &str = "Sub Sub Menu 113";
    }

    /// Dart's `createTestMenus` styling arguments of `menu_theme_test.dart`.
    #[derive(Default)]
    pub(crate) struct ThemeTestMenus {
        pub(crate) item_overlay: Option<Color>,
        pub(crate) item_background: Option<Color>,
        pub(crate) item_foreground: Option<Color>,
        pub(crate) item_padding: Option<EdgeInsetsGeometry>,
        pub(crate) menu_background: Option<Color>,
        pub(crate) menu_padding: Option<EdgeInsetsGeometry>,
        pub(crate) menu_shape: Option<OutlinedBorderRef>,
        pub(crate) menu_elevation: Option<f64>,
        pub(crate) item_shape: Option<OutlinedBorderRef>,
    }

    fn all<T: Clone + std::fmt::Debug + PartialEq + 'static>(
        value: T,
    ) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    /// Dart's `createTestMenus(onPressed: onPressed, ...)`, with an `onPressed` that does
    /// nothing.
    pub(crate) fn theme_test_menus(options: ThemeTestMenus) -> Vec<WidgetRef> {
        use test_menu::*;
        let mut menu_style = MenuStyle::new();
        menu_style.padding = options.menu_padding.map(all);
        menu_style.background_color = options.menu_background.map(all);
        menu_style.elevation = options.menu_elevation.map(all);
        menu_style.shape = options.menu_shape.map(all);
        let mut item_style = ButtonStyle::new();
        item_style.padding = options.item_padding.map(all);
        item_style.shape = options.item_shape.map(all);
        item_style.foreground_color = options.item_foreground.map(all);
        item_style.background_color = options.item_background.map(all);
        item_style.overlay_color = options.item_overlay.map(all);
        let item = |label: &str| {
            MenuItemButton::new()
                .on_pressed(waft_foundation::Listener::new(|_app| {}))
                .child(Text::new(label))
                .into_widget()
        };
        vec![
            SubmenuButton::new(vec![item(SUB_MENU_00)], Some(Text::new(MAIN_MENU_0))).into_widget(),
            SubmenuButton::new(
                vec![
                    MenuItemButton::new()
                        .on_pressed(waft_foundation::Listener::new(|_app| {}))
                        .style(item_style)
                        .child(Text::new(SUB_MENU_10))
                        .into_widget(),
                    SubmenuButton::new(
                        vec![
                            item(SUB_SUB_MENU_110),
                            item(SUB_SUB_MENU_111),
                            item(SUB_SUB_MENU_112),
                            item(SUB_SUB_MENU_113),
                        ],
                        Some(Text::new(SUB_MENU_11)),
                    )
                    .into_widget(),
                    item(SUB_MENU_12),
                ],
                Some(Text::new(MAIN_MENU_1)),
            )
            .menu_style(menu_style)
            .into_widget(),
            SubmenuButton::new(
                // Always disabled.
                vec![
                    MenuItemButton::new()
                        .child(Text::new(SUB_MENU_20))
                        .into_widget(),
                ],
                Some(Text::new(MAIN_MENU_2)),
            )
            .into_widget(),
        ]
    }

    pub(crate) fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    /// Dart's `findMenuPanels(orientation)`.
    pub(crate) fn menu_panels(tester: &WidgetTester, orientation: Axis) -> Vec<AnyElement> {
        tester
            .find_by_type::<MenuPanel>()
            .into_iter()
            .filter(|panel| {
                tester.widget(*panel, |panel: &MenuPanel| panel.orientation) == orientation
            })
            .collect()
    }

    /// The first `Material` under the only panel of `orientation`, read through `read`.
    pub(crate) fn panel_material<R>(
        tester: &WidgetTester,
        orientation: Axis,
        read: impl FnOnce(&Material) -> R,
    ) -> R {
        let panel = menu_panels(tester, orientation)[0];
        let material = tester.descendants_of_type::<Material>(panel)[0];
        tester.widget(material, read)
    }

    /// Dart's `tester.getRect(finder)` of `element`.
    pub(crate) fn rect(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.tap(find.text(text))`; a menu is in an `OverlayPortal`, which the kit's
    /// onstage finders do not see, so this finds every mounted `Text`.
    pub(crate) fn tap_text(tester: &mut WidgetTester, text: &str) {
        let center = rect(tester, only(tester.find_text(text))).center();
        tester.tap(center, 1);
    }

    /// Dart's `find.ancestor(of: find.text(text), matching: find.byType(W)).first`.
    pub(crate) fn ancestor_of_text<W: 'static>(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.ancestor_of_type::<W>(only(tester.find_text(text)))
    }

    fn shape_is(material: &Material, expected: &dyn ShapeBorder) -> bool {
        material
            .shape
            .as_ref()
            .is_some_and(|shape| shape.eq_shape(expected))
    }

    /// The bar of `menu_theme_test.dart`'s two widget cases: `MenuBarTheme(bar_style)` over
    /// `MenuTheme(menu_style)` over `Column([bar, Expanded(Placeholder())])`.
    fn pump_themed_bar(
        tester: &mut WidgetTester,
        bar_theme: MenuBarThemeData,
        menu_theme: MenuThemeData,
        bar: impl Fn() -> WidgetRef + 'static,
    ) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                m2(&app),
                Material::new()
                    .child(Builder::new(move |_app, _context| {
                        MenuBarTheme::new(
                            bar_theme.clone(),
                            MenuTheme::new(
                                menu_theme.clone(),
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

    /// The menu theme of both widget cases.
    pub(crate) fn red_stadium_menu_style() -> MenuStyle {
        MenuStyle::new()
            .background_color(all(Colors::RED.primary()))
            .elevation(all(15.0))
            .shape(all(OutlinedBorderRef::new(StadiumBorder::default())))
            .padding(all(EdgeInsetsGeometry::directional(10.0, 10.0, 10.0, 10.0)))
    }

    /// The menu bar theme of both widget cases.
    pub(crate) fn green_menu_style() -> MenuStyle {
        MenuStyle::new()
            .background_color(all(Colors::GREEN.primary()))
            .elevation(all(20.0))
    }

    /// The `MenuBar` of the 'Constructor parameters override theme parameters' cases.
    pub(crate) fn overriding_bar() -> WidgetRef {
        MenuBar::new(theme_test_menus(ThemeTestMenus {
            menu_background: Some(Colors::CYAN.primary()),
            menu_elevation: Some(18.0),
            menu_padding: Some(EdgeInsetsGeometry::directional(14.0, 14.0, 14.0, 14.0)),
            menu_shape: Some(OutlinedBorderRef::new(BeveledRectangleBorder::default())),
            item_background: Some(Colors::AMBER.primary()),
            item_foreground: Some(Colors::GREY.primary()),
            item_overlay: Some(Colors::BLUE_GREY.primary()),
            item_padding: Some(EdgeInsetsGeometry::directional(11.0, 11.0, 11.0, 11.0)),
            item_shape: Some(OutlinedBorderRef::new(StadiumBorder::default())),
        }))
        .style(
            MenuStyle::new()
                .background_color(all(Colors::BLUE.primary()))
                .elevation(all(10.0))
                .padding(all(EdgeInsetsGeometry::directional(12.0, 12.0, 12.0, 12.0))),
        )
        .into_widget()
    }

    /// The asserts the 'Constructor parameters override theme parameters' cases share.
    pub(crate) fn assert_constructor_parameters_override_theme_parameters(tester: &WidgetTester) {
        use test_menu::*;
        assert_eq!(
            rect(tester, menu_panels(tester, Axis::Horizontal)[0]),
            Rect::from_ltrb(226.0, 0.0, 574.0, 72.0)
        );
        panel_material(tester, Axis::Horizontal, |material| {
            assert_eq!(material.elevation, 10.0);
            assert_eq!(material.color, Some(Colors::BLUE.primary()));
        });

        assert_eq!(
            rect(tester, only(menu_panels(tester, Axis::Vertical))),
            Rect::from_ltrb(332.0, 60.0, 574.0, 232.0)
        );
        panel_material(tester, Axis::Vertical, |material| {
            assert_eq!(material.elevation, 18.0);
            assert_eq!(material.color, Some(Colors::CYAN.primary()));
            assert!(shape_is(material, &BeveledRectangleBorder::default()));
        });

        let submenu_panel = *menu_panels(tester, Axis::Vertical)
            .last()
            .expect("a submenu panel");
        let menu_item = tester.descendants_of_type::<MenuItemButton>(submenu_panel)[0];
        assert_eq!(
            rect(tester, menu_item),
            Rect::from_ltrb(346.0, 74.0, 560.0, 122.0)
        );
        let menu_item_material = ancestor_of_text::<Material>(tester, SUB_MENU_10);
        tester.widget(menu_item_material, |material: &Material| {
            assert_eq!(material.color, Some(Colors::AMBER.primary()));
            assert_eq!(material.elevation, 0.0);
            assert!(shape_is(material, &StadiumBorder::default()));
        });
        let label_style = ancestor_of_text::<DefaultTextStyle>(tester, SUB_MENU_10);
        tester.widget(label_style, |style: &DefaultTextStyle| {
            assert_eq!(style.style.color, Some(Colors::GREY.primary().into()));
        });
        let text_button = ancestor_of_text::<TextButton>(tester, SUB_MENU_10);
        let overlay = tester.widget(text_button, |button: &TextButton| {
            crate::button_style_button::ButtonStyleButton::button_style_button_data(button)
                .style
                .as_ref()
                .and_then(|style| style.overlay_color.clone())
        });
        let hovered: WidgetStates = [WidgetState::Hovered].into_iter().collect();
        assert_eq!(
            overlay.and_then(|overlay| overlay.resolve(&hovered)),
            Some(Colors::BLUE_GREY.primary())
        );
    }

    // menu_theme_test.dart 'MenuThemeData defaults'.
    #[test]
    fn a_new_menu_theme_data_sets_nothing() {
        let menu_theme_data = MenuThemeData::new();
        assert!(menu_theme_data.style.is_none());
        assert!(menu_theme_data.submenu_icon.is_none());
    }

    // menu_theme_test.dart 'MenuThemeData lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_theme_with_itself_is_that_theme() {
        assert!(MenuThemeData::lerp(None, None, 0.0).is_none());
        let data = MenuThemeData::new();
        assert_eq!(
            MenuThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // menu_theme_test.dart 'theme is honored'.
    #[test]
    fn a_menu_theme_styles_the_submenus_and_a_menu_bar_theme_the_bar() {
        test_widgets(|tester| {
            pump_themed_bar(
                tester,
                MenuBarThemeData::new().style(green_menu_style()),
                MenuThemeData::new().style(red_stadium_menu_style()),
                || MenuBar::new(theme_test_menus(ThemeTestMenus::default())).into_widget(),
            );

            // Open a test menu.
            tap_text(tester, test_menu::MAIN_MENU_1);
            tester.pump();
            assert_eq!(
                rect(tester, menu_panels(tester, Axis::Horizontal)[0]),
                Rect::from_ltrb(234.0, 0.0, 566.0, 48.0)
            );
            panel_material(tester, Axis::Horizontal, |material| {
                assert_eq!(material.elevation, 20.0);
                assert_eq!(material.color, Some(Colors::GREEN.primary()));
            });

            assert_eq!(
                rect(tester, only(menu_panels(tester, Axis::Vertical))),
                Rect::from_ltrb(336.0, 48.0, 570.0, 212.0)
            );
            panel_material(tester, Axis::Vertical, |material| {
                assert_eq!(material.elevation, 15.0);
                assert_eq!(material.color, Some(Colors::RED.primary()));
            });
        });
    }

    // menu_theme_test.dart 'Constructor parameters override theme parameters'.
    #[test]
    fn a_menu_bars_and_its_buttons_own_styles_override_the_themes() {
        test_widgets(|tester| {
            pump_themed_bar(
                tester,
                MenuBarThemeData::new().style(green_menu_style()),
                MenuThemeData::new().style(red_stadium_menu_style()),
                overriding_bar,
            );

            // Open a test menu.
            tap_text(tester, test_menu::MAIN_MENU_1);
            tester.pump();

            assert_constructor_parameters_override_theme_parameters(tester);
        });
    }

    /// Dart's `tester.sendKeyEvent(LogicalKeyboardKey.arrowDown)`.
    fn send_arrow_down(tester: &mut WidgetTester) {
        tester.send_key_event(
            PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_DOWN,
        );
    }

    fn count_icon(tester: &WidgetTester, icon: &IconData) -> usize {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    // menu_theme_test.dart 'SubmenuButton.submenuIcon updates default arrow icon'.
    #[test]
    fn a_menu_themes_submenu_icon_resolves_against_the_buttons_states() {
        test_widgets(|tester| {
            use test_menu::*;
            let controller = waft_widgets::MenuController::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyMenuController>();
            let disabled_icon = Icons::close_fullscreen();
            let hovered_icon = Icons::ac_unit();
            let focused_icon = Icons::zoom_out();
            let default_icon = Icons::minimize();
            let icons = (
                disabled_icon.clone(),
                hovered_icon.clone(),
                focused_icon.clone(),
                default_icon.clone(),
            );
            let submenu_icon: WidgetStatePropertyRef<Option<WidgetRef>> =
                <dyn WidgetStateProperty<Option<WidgetRef>>>::resolve_with(move |states| {
                    let (disabled, hovered, focused, default) = icons.clone();
                    if states.contains(&WidgetState::Disabled) {
                        return Some(Icon::new(Some(disabled)).into_widget());
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(Icon::new(Some(hovered)).into_widget());
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(Icon::new(Some(focused)).into_widget());
                    }
                    Some(Icon::new(Some(default)).into_widget())
                });

            let build_menu = |tester: &mut WidgetTester,
                              icon: Option<WidgetStatePropertyRef<Option<WidgetRef>>>,
                              enabled: bool| {
                let root = {
                    let app = tester.cell.borrow();
                    let mut menu_theme = MenuThemeData::new();
                    menu_theme.submenu_icon = icon;
                    let menu_children = if enabled {
                        vec![
                            MenuItemButton::new()
                                .child(Text::new(MAIN_MENU_0))
                                .into_widget(),
                        ]
                    } else {
                        Vec::new()
                    };
                    material_app(
                        ThemeData::builder().menu_theme(menu_theme).build(&app),
                        Material::new()
                            .child(
                                MenuBar::new(vec![
                                    SubmenuButton::new(
                                        vec![
                                            SubmenuButton::new(
                                                menu_children,
                                                Some(Text::new(SUB_SUB_MENU_110)),
                                            )
                                            .into_widget(),
                                        ],
                                        Some(Text::new(SUB_MENU_00)),
                                    )
                                    .into_widget(),
                                ])
                                .controller(controller.clone()),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);
            };

            build_menu(tester, None, true);
            tap_text(tester, SUB_MENU_00);
            tester.pump();

            assert_eq!(count_icon(tester, &Icons::arrow_right()), 1);

            controller.close(&mut tester.cell.borrow_mut());
            tester.pump();

            build_menu(tester, Some(Rc::clone(&submenu_icon)), true);
            tester.pump_and_settle();

            tap_text(tester, SUB_MENU_00);
            tester.pump();
            assert_eq!(count_icon(tester, &default_icon), 1);

            send_arrow_down(tester);
            tester.pump();
            assert_eq!(count_icon(tester, &focused_icon), 1);

            controller.close(&mut tester.cell.borrow_mut());
            tester.pump();

            tap_text(tester, SUB_MENU_00);
            tester.pump();
            // Dart's `hoverOver`: a new mouse moved over the text, then a settle.
            let at = rect(tester, only(tester.find_text(SUB_SUB_MENU_110))).center();
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            tester.pump();
            assert_eq!(count_icon(tester, &hovered_icon), 1);

            controller.close(&mut tester.cell.borrow_mut());
            tester.pump();

            build_menu(tester, Some(submenu_icon), false);
            tap_text(tester, SUB_MENU_00);
            tester.pump();
            assert_eq!(count_icon(tester, &disabled_icon), 1);
        });
    }
}

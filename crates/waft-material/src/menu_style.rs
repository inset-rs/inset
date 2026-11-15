//! Flutter counterpart: material_ui's `menu_style.dart`.
//!
//! A state-dependent field holds a [`WidgetStatePropertyRef`] of `Option<T>`, as
//! [`ButtonStyle`](crate::ButtonStyle)'s do.

use std::fmt::{self, Debug};

use waft_embedder::{Color, Size, lerp_double};
use waft_painting::{AlignmentGeometry, BorderSide, EdgeInsetsGeometry, OutlinedBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{WidgetStateBorderSide, WidgetStateProperty, WidgetStatePropertyRef};

use crate::theme_data::VisualDensity;

/// The visual properties that menus have in common.
///
/// Menus created by `MenuBar` and `MenuAnchor` and their themes have a
/// [`MenuStyle`] property which defines the visual properties whose default
/// values are to be overridden. The default values are defined by the
/// individual menu widgets and are typically based on overall theme's
/// `ThemeData::color_scheme` and `ThemeData::text_theme`.
///
/// All of the [`MenuStyle`] properties are null by default.
///
/// Many of the [`MenuStyle`] properties are [`WidgetStateProperty`] objects which
/// resolve to different values depending on the menu's state. For example the
/// [`Color`] properties are defined with `WidgetStateProperty<Option<Color>>` and can
/// resolve to different colors depending on if the menu is pressed, hovered,
/// focused, disabled, etc.
///
/// These properties can override the default value for just one state or all of
/// them. For example to create a `SubmenuButton` whose background color is the
/// color scheme’s primary color with 50% opacity, but only when the menu is
/// pressed, one could write:
///
/// ```text
/// SubmenuButton::new(menu_children, Text::new("Fly me to the moon")).menu_style(
///     MenuStyle::new().background_color(
///         <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
///             if states.contains(&WidgetState::Focused) {
///                 return Some(primary.with_values(Some(0.5), None, None, None, None));
///             }
///             None // Use the component's default.
///         }),
///     ),
/// )
/// ```
///
/// In this case the background color for all other menu states would fall back
/// to the `SubmenuButton`'s default values. To unconditionally set the menu's
/// [`background_color`](Self::background_color) for all states one could write:
///
/// ```text
/// SubmenuButton::new(menu_children, Text::new("Let me play among the stars")).menu_style(
///     MenuStyle::new().background_color(Rc::new(WidgetStatePropertyAll::new(Some(
///         Colors::GREEN.primary(),
///     )))),
/// )
/// ```
///
/// To configure all of the application's menus in the same way, specify the
/// overall theme's `menu_theme`:
///
/// ```text
/// MaterialApp::new().theme(ThemeData::new().menu_theme(MenuThemeData::new().style(
///     MenuStyle::new().background_color(Rc::new(WidgetStatePropertyAll::new(Some(
///         Colors::RED.primary(),
///     )))),
/// )))
/// ```
///
/// See also:
///
/// * `MenuAnchor`, a widget which hosts cascading menus.
/// * `MenuBar`, a widget which defines a menu bar of buttons hosting cascading
///   menus.
/// * `MenuButtonTheme`, the theme for `SubmenuButton`s and `MenuItemButton`s.
/// * [`ButtonStyle`](crate::ButtonStyle), a similar configuration object for button styles.
#[derive(Clone, Default)]
pub struct MenuStyle {
    /// The menu's background fill color.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The shadow color of the menu's `Material`.
    ///
    /// The material's elevation shadow can be difficult to see for dark themes,
    /// so by default the menu classes add a semi-transparent overlay to indicate
    /// elevation. See `ThemeData::apply_elevation_overlay_color`.
    pub shadow_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The surface tint color of the menu's `Material`.
    ///
    /// See `Material::surface_tint_color` for more details.
    pub surface_tint_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The elevation of the menu's `Material`.
    pub elevation: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// The padding between the menu's boundary and its child.
    pub padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,

    /// The minimum size of the menu itself.
    ///
    /// This value must be less than or equal to [`maximum_size`](Self::maximum_size).
    pub minimum_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The menu's size.
    ///
    /// This size is still constrained by the style's [`minimum_size`](Self::minimum_size) and
    /// [`maximum_size`](Self::maximum_size). Fixed size dimensions whose value is
    /// [`f64::INFINITY`] are ignored.
    ///
    /// To specify menus with a fixed width and the default height use
    /// `fixed_size(Size::from_width(320.0))`. Similarly, to specify a fixed height and the
    /// default width use `fixed_size(Size::from_height(100.0))`.
    pub fixed_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The maximum size of the menu itself.
    ///
    /// A `Size::INFINITE` or null value for this property means that the menu's
    /// maximum size is not constrained.
    ///
    /// This value must be greater than or equal to [`minimum_size`](Self::minimum_size).
    pub maximum_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The color and weight of the menu's outline.
    ///
    /// This value is combined with [`shape`](Self::shape) to create a shape decorated with an
    /// outline.
    pub side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,

    /// The shape of the menu's underlying `Material`.
    ///
    /// This shape is combined with [`side`](Self::side) to create a shape decorated with an
    /// outline.
    pub shape: Option<WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>>>,

    /// The cursor for a mouse pointer when it enters or is hovering over this
    /// menu's `InkWell`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Defines how compact the menu's layout will be.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the
    ///    [`visual_density`](Self::visual_density) for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,

    /// Determines the desired alignment of the submenu when opened relative to
    /// the button that opens it.
    ///
    /// If there isn't sufficient space to open the menu with the given alignment,
    /// and there's space on the other side of the button, then the alignment is
    /// swapped to it's opposite (1 becomes -1, etc.), and the menu will try to
    /// appear on the other side of the button. If there isn't enough space there
    /// either, then the menu will be pushed as far over as necessary to display
    /// as much of itself as possible, possibly overlapping the parent button.
    pub alignment: Option<AlignmentGeometry>,
}

/// The fluent setters of Dart's named `MenuStyle` arguments, which `copyWith` shares.
macro_rules! menu_style_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `MenuStyle(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> MenuStyle {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl MenuStyle {
    /// Create a [`MenuStyle`].
    pub fn new() -> MenuStyle {
        MenuStyle::default()
    }

    menu_style_setters!(
        background_color: WidgetStatePropertyRef<Option<Color>> => "backgroundColor",
        shadow_color: WidgetStatePropertyRef<Option<Color>> => "shadowColor",
        surface_tint_color: WidgetStatePropertyRef<Option<Color>> => "surfaceTintColor",
        elevation: WidgetStatePropertyRef<Option<f64>> => "elevation",
        padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>> => "padding",
        minimum_size: WidgetStatePropertyRef<Option<Size>> => "minimumSize",
        fixed_size: WidgetStatePropertyRef<Option<Size>> => "fixedSize",
        maximum_size: WidgetStatePropertyRef<Option<Size>> => "maximumSize",
        side: WidgetStatePropertyRef<Option<BorderSide>> => "side",
        shape: WidgetStatePropertyRef<Option<Box<dyn OutlinedBorder>>> => "shape",
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> => "mouseCursor",
        visual_density: VisualDensity => "visualDensity",
        alignment: AlignmentGeometry => "alignment",
    );

    /// Returns a copy of this MenuStyle with the given fields replaced with
    /// the new values.
    ///
    /// Chain setters: `style.copy_with().elevation(e)`.
    pub fn copy_with(&self) -> MenuStyle {
        self.clone()
    }

    /// Returns a copy of this MenuStyle where the non-null fields in `style`
    /// have replaced the corresponding null fields in this MenuStyle.
    ///
    /// In other words, `style` is used to fill in unspecified (null) fields
    /// this MenuStyle.
    pub fn merge(&self, style: Option<&MenuStyle>) -> MenuStyle {
        let Some(style) = style else {
            return self.clone();
        };
        MenuStyle {
            background_color: self
                .background_color
                .clone()
                .or_else(|| style.background_color.clone()),
            shadow_color: self
                .shadow_color
                .clone()
                .or_else(|| style.shadow_color.clone()),
            surface_tint_color: self
                .surface_tint_color
                .clone()
                .or_else(|| style.surface_tint_color.clone()),
            elevation: self.elevation.clone().or_else(|| style.elevation.clone()),
            padding: self.padding.clone().or_else(|| style.padding.clone()),
            minimum_size: self
                .minimum_size
                .clone()
                .or_else(|| style.minimum_size.clone()),
            fixed_size: self.fixed_size.clone().or_else(|| style.fixed_size.clone()),
            maximum_size: self
                .maximum_size
                .clone()
                .or_else(|| style.maximum_size.clone()),
            side: self.side.clone().or_else(|| style.side.clone()),
            shape: self.shape.clone().or_else(|| style.shape.clone()),
            mouse_cursor: self
                .mouse_cursor
                .clone()
                .or_else(|| style.mouse_cursor.clone()),
            visual_density: self.visual_density.or(style.visual_density),
            alignment: self.alignment.or(style.alignment),
        }
    }

    /// Linearly interpolate between two [`MenuStyle`]s.
    pub fn lerp(a: Option<&MenuStyle>, b: Option<&MenuStyle>, t: f64) -> Option<MenuStyle> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(MenuStyle {
            background_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.background_color.clone()),
                b.and_then(|b| b.background_color.clone()),
                t,
                Color::lerp,
            ),
            shadow_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.shadow_color.clone()),
                b.and_then(|b| b.shadow_color.clone()),
                t,
                Color::lerp,
            ),
            surface_tint_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.surface_tint_color.clone()),
                b.and_then(|b| b.surface_tint_color.clone()),
                t,
                Color::lerp,
            ),
            elevation: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.elevation.clone()),
                b.and_then(|b| b.elevation.clone()),
                t,
                lerp_double,
            ),
            padding: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.padding.clone()),
                b.and_then(|b| b.padding.clone()),
                t,
                EdgeInsetsGeometry::lerp,
            ),
            minimum_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.minimum_size.clone()),
                b.and_then(|b| b.minimum_size.clone()),
                t,
                Size::lerp,
            ),
            fixed_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.fixed_size.clone()),
                b.and_then(|b| b.fixed_size.clone()),
                t,
                Size::lerp,
            ),
            maximum_size: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.maximum_size.clone()),
                b.and_then(|b| b.maximum_size.clone()),
                t,
                Size::lerp,
            ),
            side: WidgetStateBorderSide::lerp(
                a.and_then(|a| a.side.clone()),
                b.and_then(|b| b.side.clone()),
                t,
            ),
            shape: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.shape.clone()),
                b.and_then(|b| b.shape.clone()),
                t,
                |a, b, t| <dyn OutlinedBorder>::lerp(a.as_deref(), b.as_deref(), t),
            ),
            mouse_cursor: if t < 0.5 {
                a.and_then(|a| a.mouse_cursor.clone())
            } else {
                b.and_then(|b| b.mouse_cursor.clone())
            },
            visual_density: if t < 0.5 {
                a.and_then(|a| a.visual_density)
            } else {
                b.and_then(|b| b.visual_density)
            },
            alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.alignment),
                b.and_then(|b| b.alignment),
                t,
            ),
        })
    }
}

impl PartialEq for MenuStyle {
    fn eq(&self, other: &MenuStyle) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.background_color == self.background_color
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.elevation == self.elevation
            && other.padding == self.padding
            && other.minimum_size == self.minimum_size
            && other.fixed_size == self.fixed_size
            && other.maximum_size == self.maximum_size
            && other.side == self.side
            && other.shape == self.shape
            && other.mouse_cursor == self.mouse_cursor
            && other.visual_density == self.visual_density
            && other.alignment == self.alignment
    }
}

impl Debug for MenuStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("MenuStyle");
        macro_rules! property {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = &self.$field {
                        properties.field(stringify!($field), value);
                    }
                )*
            };
        }
        property!(
            background_color,
            shadow_color,
            surface_tint_color,
            elevation,
            padding,
            minimum_size,
            fixed_size,
            maximum_size,
            side,
            shape,
            mouse_cursor,
            visual_density,
            alignment,
        );
        properties.finish()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::Rect;
    use waft_rendering::BoxConstraints;
    use waft_widgets::{
        AnyElement, Column, ConstrainedBox, Expanded, IntoWidget, Text, WidgetRef,
        WidgetStatePropertyAll,
    };

    use super::*;
    use crate::material::Material;
    use crate::menu_anchor::{MenuBar, MenuItemButton, MenuPanel, SubmenuButton};
    use crate::menu_bar_theme::{MenuBarTheme, MenuBarThemeData};
    use crate::menu_theme::tests::{only, rect, tap_text};
    use crate::menu_theme::{MenuTheme, MenuThemeData};
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, tester};

    fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    /// Dart's `createTestMenus(onPressed: (TestMenu menu) {})` of `menu_style_test.dart`.
    fn test_menus() -> Vec<WidgetRef> {
        let item = |label: &str| {
            MenuItemButton::new()
                .on_pressed(waft_foundation::Listener::new(|_app| {}))
                .child(Text::new(label))
                .into_widget()
        };
        vec![
            SubmenuButton::new(
                vec![
                    item("Sub Menu 00"),
                    item("Sub Menu 01"),
                    item("Sub Menu 02"),
                ],
                Some(Text::new("Menu 0")),
            )
            .into_widget(),
            SubmenuButton::new(
                vec![
                    item("Sub Menu 10"),
                    SubmenuButton::new(
                        vec![
                            item("Sub Sub Menu 110"),
                            item("Sub Sub Menu 111"),
                            item("Sub Sub Menu 112"),
                            item("Sub Sub Menu 113"),
                        ],
                        Some(Text::new("Sub Menu 11")),
                    )
                    .into_widget(),
                    item("Sub Menu 12"),
                ],
                Some(Text::new("Menu 1")),
            )
            .into_widget(),
            SubmenuButton::new(
                // Always disabled.
                vec![
                    MenuItemButton::new()
                        .child(Text::new("Sub Menu 20"))
                        .into_widget(),
                ],
                Some(Text::new("Menu 2")),
            )
            .into_widget(),
        ]
    }

    /// `MaterialApp(theme:, home: Material(child: Column([MenuBarTheme(bar_style,
    /// MenuTheme(menu_style, MenuBar(test_menus))), Expanded(Placeholder())])))`.
    fn pump_themed_bar(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&waft_foundation::App) -> ThemeData,
        bar_style: MenuStyle,
        menu_style: MenuStyle,
    ) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                theme(&app),
                Material::new()
                    .child(Column::new().children(vec![
                        MenuBarTheme::new(
                            MenuBarThemeData::new().style(bar_style),
                            MenuTheme::new(
                                MenuThemeData::new().style(menu_style),
                                MenuBar::new(test_menus()),
                            ),
                        )
                        .into_widget(),
                        Expanded::new(ConstrainedBox::new(BoxConstraints::expand(None, None)))
                            .into_widget(),
                    ]))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// Dart's `findMenuPanels()`.
    fn menu_panels(tester: &WidgetTester) -> Vec<AnyElement> {
        tester.find_by_type::<MenuPanel>()
    }

    /// Opens the first menu: Dart's "Have to open a menu initially to start things going."
    fn open_first_menu(tester: &mut WidgetTester) {
        tap_text(tester, "Menu 0");
        tester.pump(Duration::ZERO);
    }

    // menu_style_test.dart 'MenuStyle lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_style_with_itself_is_that_style() {
        assert!(MenuStyle::lerp(None, None, 0.0).is_none());
        let data = MenuStyle::new();
        assert_eq!(MenuStyle::lerp(Some(&data), Some(&data), 0.5), Some(data));
    }

    // menu_style_test.dart 'fixedSize affects geometry'.
    #[test]
    fn a_fixed_size_sizes_the_menu_bar_and_its_menus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_themed_bar(
                &mut tester,
                m3,
                MenuStyle::new().fixed_size(all(waft_embedder::Size::new(600.0, 60.0))),
                MenuStyle::new().fixed_size(all(waft_embedder::Size::new(100.0, 100.0))),
            );
            open_first_menu(&mut tester);

            // MenuBarTheme affects MenuBar.
            let panels = menu_panels(&tester);
            assert_eq!(
                rect(&tester, panels[0]),
                Rect::from_ltrb(100.0, 0.0, 700.0, 60.0)
            );
            assert_eq!(
                rect(&tester, panels[0]).size(),
                waft_embedder::Size::new(600.0, 60.0)
            );

            // MenuTheme affects menus.
            assert_eq!(
                rect(&tester, panels[1]),
                Rect::from_ltrb(104.0, 54.0, 204.0, 154.0)
            );
            assert_eq!(
                rect(&tester, panels[1]).size(),
                waft_embedder::Size::new(100.0, 100.0)
            );
        });
    }

    // menu_style_test.dart 'maximumSize affects geometry'.
    #[test]
    fn a_maximum_size_limits_the_menu_bar_and_its_menus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_themed_bar(
                &mut tester,
                m3,
                MenuStyle::new().maximum_size(all(waft_embedder::Size::new(250.0, 40.0))),
                MenuStyle::new().maximum_size(all(waft_embedder::Size::new(100.0, 100.0))),
            );
            open_first_menu(&mut tester);

            // MenuBarTheme affects MenuBar.
            let panels = menu_panels(&tester);
            assert_eq!(
                rect(&tester, panels[0]),
                Rect::from_ltrb(275.0, 0.0, 525.0, 40.0)
            );
            assert_eq!(
                rect(&tester, panels[0]).size(),
                waft_embedder::Size::new(250.0, 40.0)
            );

            // MenuTheme affects menus.
            assert_eq!(
                rect(&tester, panels[1]),
                Rect::from_ltrb(279.0, 44.0, 379.0, 144.0)
            );
            assert_eq!(
                rect(&tester, panels[1]).size(),
                waft_embedder::Size::new(100.0, 100.0)
            );
        });
    }

    // menu_style_test.dart 'minimumSize affects geometry'.
    #[test]
    fn a_minimum_size_grows_the_menu_bar_and_its_menus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_themed_bar(
                &mut tester,
                m3,
                MenuStyle::new().minimum_size(all(waft_embedder::Size::new(400.0, 60.0))),
                MenuStyle::new().minimum_size(all(waft_embedder::Size::new(300.0, 300.0))),
            );
            open_first_menu(&mut tester);

            // MenuBarTheme affects MenuBar.
            let panels = menu_panels(&tester);
            assert_eq!(
                rect(&tester, panels[0]),
                Rect::from_ltrb(200.0, 0.0, 600.0, 60.0)
            );
            assert_eq!(
                rect(&tester, panels[0]).size(),
                waft_embedder::Size::new(400.0, 60.0)
            );

            // MenuTheme affects menus.
            assert_eq!(
                rect(&tester, panels[1]),
                Rect::from_ltrb(204.0, 54.0, 504.0, 354.0)
            );
            assert_eq!(
                rect(&tester, panels[1]).size(),
                waft_embedder::Size::new(300.0, 300.0)
            );
        });
    }

    // menu_style_test.dart 'visual density'.
    #[test]
    fn a_menu_styles_visual_density_spaces_the_bar_and_its_menus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_themed_bar(
                &mut tester,
                m2,
                MenuStyle::new().visual_density(VisualDensity::new(1.5, -1.5)),
                MenuStyle::new().visual_density(VisualDensity::new(0.5, -0.5)),
            );
            tester.pump(Duration::ZERO);

            let bar = only(tester.find_by_type::<MenuBar>());
            assert_eq!(rect(&tester, bar), Rect::from_ltrb(228.0, 0.0, 572.0, 48.0));

            // Open and make sure things are the right size.
            tap_text(&mut tester, "Menu 1");
            tester.pump(Duration::ZERO);

            assert_eq!(rect(&tester, bar), Rect::from_ltrb(228.0, 0.0, 572.0, 48.0));
            assert_eq!(
                rect(&tester, only(tester.find_text("Sub Menu 10"))),
                Rect::from_ltrb(372.0, 70.0, 565.0, 84.0)
            );
            let label = only(tester.find_text("Sub Menu 10"));
            let materials: Vec<AnyElement> = tester
                .ancestors(label)
                .into_iter()
                .filter(|ancestor| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<Material>(&**ancestor.widget(&app)).is_some()
                })
                .collect();
            assert_eq!(
                rect(&tester, materials[1]),
                Rect::from_ltrb(352.0, 48.0, 585.0, 190.0)
            );
        });
    }
}

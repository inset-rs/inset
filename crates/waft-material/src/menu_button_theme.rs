//! Flutter counterpart: material_ui's `menu_button_theme.dart`.

use waft_foundation::App;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::button_style::ButtonStyle;
use crate::theme::Theme;

/// A [`ButtonStyle`] theme that overrides the default appearance of
/// `SubmenuButton`s and `MenuItemButton`s.
///
/// Descendant widgets obtain the current [`MenuButtonThemeData`] object
/// using [`MenuButtonTheme::of`].
///
/// A [`MenuButtonThemeData`] is often specified as part of the
/// overall `Theme` with `ThemeData::menu_button_theme`.
///
/// The [`style`](Self::style)'s properties override `MenuItemButton`'s and `SubmenuButton`'s
/// default style, i.e. the [`ButtonStyle`] returned by
/// `MenuItemButton::default_style_of` and `SubmenuButton::default_style_of`. Only the
/// style's non-null property values or resolved non-null
/// `WidgetStateProperty` values are used.
///
/// See also:
///
/// * [`MenuButtonTheme`], the theme which is configured with this class.
/// * `MenuTheme`, the theme used to configure the look of the menus these
///   buttons reside in.
/// * `MenuItemButton::default_style_of` and `SubmenuButton::default_style_of` which
///   return the default [`ButtonStyle`]s for menu buttons.
/// * `MenuItemButton::style_from` and `SubmenuButton::style_from`, which converts
///   simple values into a [`ButtonStyle`] that's consistent with their respective
///   defaults.
/// * `WidgetStateProperty::resolve`, "resolve" a material state property to a
///   simple value based on a set of `WidgetState`s.
/// * `ThemeData::menu_button_theme`, which can be used to override the default
///   [`ButtonStyle`] for `MenuItemButton`s and `SubmenuButton`s below the overall
///   `Theme`.
/// * `MenuAnchor`, a widget which hosts cascading menus.
/// * `MenuBar`, a widget which defines a menu bar of buttons hosting cascading
///   menus.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuButtonThemeData {
    /// Overrides for `SubmenuButton` and `MenuItemButton`'s default style.
    ///
    /// Non-null properties or non-null resolved `WidgetStateProperty` values
    /// override the [`ButtonStyle`] returned by `SubmenuButton::default_style_of` or
    /// `MenuItemButton::default_style_of`.
    ///
    /// If [`style`](Self::style) is null, then this theme doesn't override anything.
    pub style: Option<ButtonStyle>,
}

impl MenuButtonThemeData {
    /// Creates a [`MenuButtonThemeData`].
    ///
    /// The [`style`](Self::style) may be null.
    pub fn new() -> MenuButtonThemeData {
        MenuButtonThemeData::default()
    }

    /// Dart `MenuButtonThemeData(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> MenuButtonThemeData {
        self.style = Some(style);
        self
    }

    /// Linearly interpolate between two menu button themes.
    pub fn lerp(
        a: Option<&MenuButtonThemeData>,
        b: Option<&MenuButtonThemeData>,
        t: f64,
    ) -> Option<MenuButtonThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(MenuButtonThemeData {
            style: ButtonStyle::lerp(
                a.and_then(|a| a.style.as_ref()),
                b.and_then(|b| b.style.as_ref()),
                t,
            ),
        })
    }
}

/// Overrides the default [`ButtonStyle`] of its `MenuItemButton` and
/// `SubmenuButton` descendants.
///
/// See also:
///
/// * [`MenuButtonThemeData`], which is used to configure this theme.
/// * `MenuTheme`, the theme used to configure the look of the menus themselves.
/// * `MenuItemButton::default_style_of` and `SubmenuButton::default_style_of` which
///   return the default [`ButtonStyle`]s for menu buttons.
/// * `MenuItemButton::style_from` and `SubmenuButton::style_from`, which converts
///   simple values into a [`ButtonStyle`] that's consistent with their respective
///   defaults.
/// * `ThemeData::menu_button_theme`, which can be used to override the default
///   [`ButtonStyle`] for `MenuItemButton`s and `SubmenuButton`s below the overall
///   `Theme`.
#[derive(Debug)]
pub struct MenuButtonTheme {
    pub key: Option<KeyRef>,
    /// The configuration of this theme.
    pub data: MenuButtonThemeData,
    pub child: WidgetRef,
}

impl MenuButtonTheme {
    /// Create a [`MenuButtonTheme`].
    pub fn new<K>(data: MenuButtonThemeData, child: impl IntoWidget<K>) -> MenuButtonTheme {
        MenuButtonTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `MenuButtonTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuButtonTheme {
        self.key = Some(key);
        self
    }

    /// The closest instance of this class that encloses the given context.
    ///
    /// If there is no enclosing [`MenuButtonTheme`] widget, then
    /// `ThemeData::menu_button_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = MenuButtonTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> MenuButtonThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<MenuButtonTheme>(app) {
            Some(button_theme) => button_theme.data.clone(),
            None => Theme::of(app, context).menu_button_theme.clone(),
        }
    }
}

impl InheritedTheme for MenuButtonTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        MenuButtonTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for MenuButtonTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuButtonTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // menu_button_theme_test.dart 'MenuButtonThemeData lerp special cases'.
    #[test]
    fn lerp_of_nothing_is_nothing_and_of_one_theme_with_itself_is_that_theme() {
        assert!(MenuButtonThemeData::lerp(None, None, 0.0).is_none());
        let data = MenuButtonThemeData::new();
        assert_eq!(
            MenuButtonThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }
}

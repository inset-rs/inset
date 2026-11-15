//! Flutter counterpart: material_ui's `navigation_drawer_theme.dart`.

use waft_embedder::{Color, Size, lerp_double};
use waft_foundation::App;
use waft_painting::{ShapeBorder, TextStyle};
use waft_widgets::{
    BuildContext, IconThemeData, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `NavigationDrawer`
/// widgets.
///
/// Descendant widgets obtain the current [`NavigationDrawerThemeData`] object
/// using [`NavigationDrawerTheme::of`]. Instances of [`NavigationDrawerThemeData`]
/// can be customized with [`NavigationDrawerThemeData::copy_with`].
///
/// Typically a [`NavigationDrawerThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::navigation_drawer_theme`. Alternatively, a
/// [`NavigationDrawerTheme`] inherited widget can be used to theme `NavigationDrawer`s
/// in a subtree of widgets.
///
/// All [`NavigationDrawerThemeData`] properties are `null` by default.
/// When null, the `NavigationDrawer` will provide its own defaults based on the
/// overall `Theme`'s textTheme and colorScheme. See the individual
/// `NavigationDrawer` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct NavigationDrawerThemeData {
    /// Overrides the default height of `NavigationDrawerDestination`.
    pub tile_height: Option<f64>,

    /// Overrides the default value of `NavigationDrawer::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `NavigationDrawer::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value of `NavigationDrawer::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value of `NavigationDrawer::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of `NavigationDrawer`'s selection indicator.
    pub indicator_color: Option<Color>,

    /// Overrides the default shape of the `NavigationDrawer`'s selection indicator.
    pub indicator_shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default size of the `NavigationDrawer`'s selection indicator.
    pub indicator_size: Option<Size>,

    /// The style to merge with the default text style for
    /// `NavigationDestination` labels.
    ///
    /// You can use this to specify a different style when the label is selected.
    pub label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The theme to merge with the default icon theme for
    /// `NavigationDestination` icons.
    ///
    /// You can use this to specify a different icon theme when the icon is
    /// selected.
    pub icon_theme: Option<WidgetStatePropertyRef<Option<IconThemeData>>>,
}

impl NavigationDrawerThemeData {
    /// Creates a theme that can be used for `ThemeData::navigation_drawer_theme` and
    /// [`NavigationDrawerTheme`]; Dart's optional named arguments are the fluent setters.
    pub fn new() -> NavigationDrawerThemeData {
        NavigationDrawerThemeData::default()
    }

    /// Dart `NavigationDrawerThemeData(tileHeight:)`.
    pub fn tile_height(mut self, tile_height: f64) -> NavigationDrawerThemeData {
        self.tile_height = Some(tile_height);
        self
    }

    /// Dart `NavigationDrawerThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationDrawerThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationDrawerThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationDrawerThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationDrawerThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> NavigationDrawerThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `NavigationDrawerThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> NavigationDrawerThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `NavigationDrawerThemeData(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationDrawerThemeData {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationDrawerThemeData(indicatorShape:)`.
    pub fn indicator_shape(
        mut self,
        indicator_shape: impl Into<Box<dyn ShapeBorder>>,
    ) -> NavigationDrawerThemeData {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationDrawerThemeData(indicatorSize:)`.
    pub fn indicator_size(mut self, indicator_size: Size) -> NavigationDrawerThemeData {
        self.indicator_size = Some(indicator_size);
        self
    }

    /// Dart `NavigationDrawerThemeData(labelTextStyle:)`.
    pub fn label_text_style(
        mut self,
        label_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> NavigationDrawerThemeData {
        self.label_text_style = Some(label_text_style);
        self
    }

    /// Dart `NavigationDrawerThemeData(iconTheme:)`.
    pub fn icon_theme(
        mut self,
        icon_theme: WidgetStatePropertyRef<Option<IconThemeData>>,
    ) -> NavigationDrawerThemeData {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().tile_height(48.0)`).
    pub fn copy_with(&self) -> NavigationDrawerThemeData {
        self.clone()
    }

    /// Linearly interpolate between two navigation rail themes.
    ///
    /// If both arguments are null then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&NavigationDrawerThemeData>,
        b: Option<&NavigationDrawerThemeData>,
        t: f64,
    ) -> Option<NavigationDrawerThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(NavigationDrawerThemeData {
            tile_height: lerp_double(
                a.and_then(|a| a.tile_height),
                b.and_then(|b| b.tile_height),
                t,
            ),
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            indicator_color: Color::lerp(
                a.and_then(|a| a.indicator_color),
                b.and_then(|b| b.indicator_color),
                t,
            ),
            indicator_shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.indicator_shape.as_deref()),
                b.and_then(|b| b.indicator_shape.as_deref()),
                t,
            ),
            // Dart lerps `a`'s size with itself.
            indicator_size: Size::lerp(
                a.and_then(|a| a.indicator_size),
                a.and_then(|a| a.indicator_size),
                t,
            ),
            label_text_style: <dyn WidgetStateProperty<Option<TextStyle>>>::lerp_optional(
                a.and_then(|a| a.label_text_style.clone()),
                b.and_then(|b| b.label_text_style.clone()),
                t,
                |a: Option<TextStyle>, b: Option<TextStyle>, t| {
                    TextStyle::lerp(a.as_ref(), b.as_ref(), t)
                },
            ),
            icon_theme: <dyn WidgetStateProperty<Option<IconThemeData>>>::lerp_optional(
                a.and_then(|a| a.icon_theme.clone()),
                b.and_then(|b| b.icon_theme.clone()),
                t,
                |a: Option<IconThemeData>, b: Option<IconThemeData>, t| {
                    Some(IconThemeData::lerp(a.as_ref(), b.as_ref(), t))
                },
            ),
        })
    }
}

impl PartialEq for NavigationDrawerThemeData {
    fn eq(&self, other: &NavigationDrawerThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.tile_height == self.tile_height
            && other.background_color == self.background_color
            && other.elevation == self.elevation
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.indicator_color == self.indicator_color
            && other.indicator_shape == self.indicator_shape
            && other.indicator_size == self.indicator_size
            && other.label_text_style == self.label_text_style
            && other.icon_theme == self.icon_theme
    }
}

/// An inherited widget that defines visual properties for `NavigationDrawer`s and
/// `NavigationDestination`s in this widget's subtree.
///
/// Values specified here are used for `NavigationDrawer` properties that are not
/// given an explicit non-null value.
///
/// See also:
///
///  * `ThemeData::navigation_drawer_theme`, which describes the
///    [`NavigationDrawerThemeData`] in the overall theme for the application.
#[derive(Debug)]
pub struct NavigationDrawerTheme {
    pub key: Option<KeyRef>,
    /// Specifies the background color, label text style, icon theme, and label
    /// type values for descendant `NavigationDrawer` widgets.
    pub data: NavigationDrawerThemeData,
    pub child: WidgetRef,
}

impl NavigationDrawerTheme {
    /// Creates a navigation rail theme that controls the
    /// [`NavigationDrawerThemeData`] properties for a `NavigationDrawer`.
    pub fn new<K>(
        data: NavigationDrawerThemeData,
        child: impl IntoWidget<K>,
    ) -> NavigationDrawerTheme {
        NavigationDrawerTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `NavigationDrawerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationDrawerTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`NavigationDrawerThemeData`] from the closest
    /// ancestor [`NavigationDrawerTheme`].
    ///
    /// If there is no enclosing [`NavigationDrawerTheme`] widget, then
    /// `ThemeData::navigation_drawer_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = NavigationDrawerTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> NavigationDrawerThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<NavigationDrawerTheme>(app) {
            Some(navigation_drawer_theme) => navigation_drawer_theme.data.clone(),
            None => Theme::of(app, context).navigation_drawer_theme.clone(),
        }
    }
}

impl InheritedTheme for NavigationDrawerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        NavigationDrawerTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for NavigationDrawerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &NavigationDrawerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

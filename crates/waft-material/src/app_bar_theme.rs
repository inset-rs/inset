//! Flutter counterpart: material_ui's `app_bar_theme.dart`.

use waft_embedder::{Color, SystemUiOverlayStyle, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_widgets::{
    BuildContext, IconThemeData, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, SizedBox,
    WidgetRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `AppBar` widgets.
///
/// Descendant widgets obtain the current [`AppBarThemeData`] object with
/// [`AppBarTheme::of`]. Instances of [`AppBarThemeData`] can be customized
/// with [`AppBarThemeData::copy_with`].
///
/// Typically an [`AppBarThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::app_bar_theme`.
///
/// All [`AppBarTheme`] properties are `null` by default. When null, the
/// `AppBar` constructor provides defaults.
///
/// Dart's `AppBarTheme` constructor takes either the properties or a [`data`](Self::data);
/// here the properties are the `with_*` setters and [`with_data`](Self::with_data) the other,
/// and each setter that breaks Dart's constructor assert (a `data` together with any property)
/// fails in a debug build. The getters are methods, as Dart's, hence the `with_` prefix.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug)]
pub struct AppBarTheme {
    pub key: Option<KeyRef>,
    data: Option<AppBarThemeData>,
    background_color: Option<Color>,
    foreground_color: Option<Color>,
    elevation: Option<f64>,
    scrolled_under_elevation: Option<f64>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    shape: Option<ShapeBorderRef>,
    icon_theme: Option<IconThemeData>,
    actions_icon_theme: Option<IconThemeData>,
    center_title: Option<bool>,
    title_spacing: Option<f64>,
    leading_width: Option<f64>,
    toolbar_height: Option<f64>,
    toolbar_text_style: Option<TextStyle>,
    title_text_style: Option<TextStyle>,
    system_overlay_style: Option<SystemUiOverlayStyle>,
    actions_padding: Option<EdgeInsetsGeometry>,
    child: WidgetRef,
}

#[allow(deprecated)]
impl AppBarTheme {
    /// Creates a theme that can be used for `ThemeData::app_bar_theme`.
    pub fn new() -> AppBarTheme {
        AppBarTheme {
            key: None,
            data: None,
            background_color: None,
            foreground_color: None,
            elevation: None,
            scrolled_under_elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            shape: None,
            icon_theme: None,
            actions_icon_theme: None,
            center_title: None,
            title_spacing: None,
            leading_width: None,
            toolbar_height: None,
            toolbar_text_style: None,
            title_text_style: None,
            system_overlay_style: None,
            actions_padding: None,
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart `AppBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> AppBarTheme {
        self.key = Some(key);
        self
    }

    /// Dart `AppBarTheme(child:)`; the default is an empty `SizedBox`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AppBarTheme {
        self.child = child.into_widget();
        self
    }

    /// Dart `AppBarTheme(data:)`.
    pub fn with_data(mut self, data: AppBarThemeData) -> AppBarTheme {
        debug_assert!(self.has_no_property());
        self.data = Some(data);
        self
    }

    /// Dart `AppBarTheme(color:)`: the same as [`with_background_color`](Self::with_background_color).
    #[deprecated(
        note = "Use backgroundColor instead. This feature was deprecated after v3.33.0-0.2.pre."
    )]
    pub fn with_color(self, color: Color) -> AppBarTheme {
        self.with_background_color(color)
    }

    /// Dart `AppBarTheme(backgroundColor:)`.
    pub fn with_background_color(mut self, background_color: Color) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.background_color = Some(background_color);
        self
    }

    /// Dart `AppBarTheme(foregroundColor:)`.
    pub fn with_foreground_color(mut self, foreground_color: Color) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `AppBarTheme(elevation:)`.
    pub fn with_elevation(mut self, elevation: f64) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.elevation = Some(elevation);
        self
    }

    /// Dart `AppBarTheme(scrolledUnderElevation:)`.
    pub fn with_scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    /// Dart `AppBarTheme(shadowColor:)`.
    pub fn with_shadow_color(mut self, shadow_color: Color) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `AppBarTheme(surfaceTintColor:)`.
    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `AppBarTheme(shape:)`.
    pub fn with_shape(mut self, shape: impl Into<ShapeBorderRef>) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.shape = Some(shape.into());
        self
    }

    /// Dart `AppBarTheme(iconTheme:)`.
    pub fn with_icon_theme(mut self, icon_theme: IconThemeData) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `AppBarTheme(actionsIconTheme:)`.
    pub fn with_actions_icon_theme(mut self, actions_icon_theme: IconThemeData) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.actions_icon_theme = Some(actions_icon_theme);
        self
    }

    /// Dart `AppBarTheme(centerTitle:)`.
    pub fn with_center_title(mut self, center_title: bool) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.center_title = Some(center_title);
        self
    }

    /// Dart `AppBarTheme(titleSpacing:)`.
    pub fn with_title_spacing(mut self, title_spacing: f64) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.title_spacing = Some(title_spacing);
        self
    }

    /// Dart `AppBarTheme(leadingWidth:)`.
    pub fn with_leading_width(mut self, leading_width: f64) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.leading_width = Some(leading_width);
        self
    }

    /// Dart `AppBarTheme(toolbarHeight:)`.
    pub fn with_toolbar_height(mut self, toolbar_height: f64) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.toolbar_height = Some(toolbar_height);
        self
    }

    /// Dart `AppBarTheme(toolbarTextStyle:)`.
    pub fn with_toolbar_text_style(mut self, toolbar_text_style: TextStyle) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.toolbar_text_style = Some(toolbar_text_style);
        self
    }

    /// Dart `AppBarTheme(titleTextStyle:)`.
    pub fn with_title_text_style(mut self, title_text_style: TextStyle) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `AppBarTheme(systemOverlayStyle:)`.
    pub fn with_system_overlay_style(
        mut self,
        system_overlay_style: SystemUiOverlayStyle,
    ) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.system_overlay_style = Some(system_overlay_style);
        self
    }

    /// Dart `AppBarTheme(actionsPadding:)`.
    pub fn with_actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> AppBarTheme {
        debug_assert!(self.data.is_none());
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Overrides the default value of `AppBar::background_color` in all
    /// descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`foreground_color`](Self::foreground_color), which overrides the default value of
    ///    `AppBar::foreground_color` in all descendant `AppBar` widgets.
    pub fn background_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.background_color,
            None => self.background_color,
        }
    }

    /// Overrides the default value of `AppBar::foreground_color` in all
    /// descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`background_color`](Self::background_color), which overrides the default value of
    ///    `AppBar::background_color` in all descendant `AppBar` widgets.
    pub fn foreground_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.foreground_color,
            None => self.foreground_color,
        }
    }

    /// Overrides the default value of `AppBar::elevation` in all
    /// descendant `AppBar` widgets.
    pub fn elevation(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.elevation,
            None => self.elevation,
        }
    }

    /// Overrides the default value of `AppBar::scrolled_under_elevation` in all
    /// descendant `AppBar` widgets.
    pub fn scrolled_under_elevation(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.scrolled_under_elevation,
            None => self.scrolled_under_elevation,
        }
    }

    /// Overrides the default value of `AppBar::shadow_color` in all
    /// descendant `AppBar` widgets.
    pub fn shadow_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.shadow_color,
            None => self.shadow_color,
        }
    }

    /// Overrides the default value of `AppBar::surface_tint_color` in all
    /// descendant `AppBar` widgets.
    pub fn surface_tint_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.surface_tint_color,
            None => self.surface_tint_color,
        }
    }

    /// Overrides the default value of `AppBar::shape` in all
    /// descendant `AppBar` widgets.
    pub fn shape(&self) -> Option<&ShapeBorderRef> {
        match &self.data {
            Some(data) => data.shape.as_ref(),
            None => self.shape.as_ref(),
        }
    }

    /// Overrides the default value of `AppBar::icon_theme` in all
    /// descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`actions_icon_theme`](Self::actions_icon_theme), which overrides the default value of
    ///    `AppBar::actions_icon_theme` in all descendant `AppBar` widgets.
    ///  * [`foreground_color`](Self::foreground_color), which overrides the default value
    ///    `AppBar::foreground_color` in all descendant `AppBar` widgets.
    pub fn icon_theme(&self) -> Option<&IconThemeData> {
        match &self.data {
            Some(data) => data.icon_theme.as_ref(),
            None => self.icon_theme.as_ref(),
        }
    }

    /// Overrides the default value of `AppBar::actions_icon_theme` in all
    /// descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`icon_theme`](Self::icon_theme), which overrides the default value of
    ///    `AppBar::icon_theme` in all descendant `AppBar` widgets.
    ///  * [`foreground_color`](Self::foreground_color), which overrides the default value
    ///    `AppBar::foreground_color` in all descendant `AppBar` widgets.
    pub fn actions_icon_theme(&self) -> Option<&IconThemeData> {
        match &self.data {
            Some(data) => data.actions_icon_theme.as_ref(),
            None => self.actions_icon_theme.as_ref(),
        }
    }

    /// Overrides the default value of `AppBar::center_title`
    /// property in all descendant `AppBar` widgets.
    pub fn center_title(&self) -> Option<bool> {
        match &self.data {
            Some(data) => data.center_title,
            None => self.center_title,
        }
    }

    /// Overrides the default value of the obsolete `AppBar::title_spacing`
    /// property in all descendant `AppBar` widgets.
    ///
    /// If null, `AppBar` uses default value of `NavigationToolbar::K_MIDDLE_SPACING`.
    pub fn title_spacing(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.title_spacing,
            None => self.title_spacing,
        }
    }

    /// Overrides the default value of the `AppBar::leading_width`
    /// property in all descendant `AppBar` widgets.
    pub fn leading_width(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.leading_width,
            None => self.leading_width,
        }
    }

    /// Overrides the default value of the `AppBar::toolbar_height`
    /// property in all descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * `AppBar::preferred_height_for`, which computes the overall
    ///    height of an AppBar widget, taking this value into account.
    pub fn toolbar_height(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.toolbar_height,
            None => self.toolbar_height,
        }
    }

    /// Overrides the default value of the obsolete `AppBar::toolbar_text_style`
    /// property in all descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`title_text_style`](Self::title_text_style), which overrides the default of `AppBar::title_text_style`
    ///    in all descendant `AppBar` widgets.
    pub fn toolbar_text_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.toolbar_text_style.as_ref(),
            None => self.toolbar_text_style.as_ref(),
        }
    }

    /// Overrides the default value of `AppBar::title_text_style`
    /// property in all descendant `AppBar` widgets.
    ///
    /// See also:
    ///
    ///  * [`toolbar_text_style`](Self::toolbar_text_style), which overrides the default of `AppBar::toolbar_text_style`
    ///    in all descendant `AppBar` widgets.
    pub fn title_text_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.title_text_style.as_ref(),
            None => self.title_text_style.as_ref(),
        }
    }

    /// Overrides the default value of `AppBar::system_overlay_style`
    /// property in all descendant `AppBar` widgets.
    pub fn system_overlay_style(&self) -> Option<SystemUiOverlayStyle> {
        match &self.data {
            Some(data) => data.system_overlay_style,
            None => self.system_overlay_style,
        }
    }

    /// Overrides the default value of `AppBar::actions_padding`
    /// property in all descendant `AppBar` widgets.
    pub fn actions_padding(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.actions_padding,
            None => self.actions_padding,
        }
    }

    /// The properties used for all descendant `AppBar` widgets.
    pub fn data(&self) -> AppBarThemeData {
        self.data.clone().unwrap_or_else(|| AppBarThemeData {
            background_color: self.background_color,
            foreground_color: self.foreground_color,
            elevation: self.elevation,
            scrolled_under_elevation: self.scrolled_under_elevation,
            shadow_color: self.shadow_color,
            surface_tint_color: self.surface_tint_color,
            shape: self.shape.clone(),
            icon_theme: self.icon_theme.clone(),
            actions_icon_theme: self.actions_icon_theme.clone(),
            center_title: self.center_title,
            title_spacing: self.title_spacing,
            leading_width: self.leading_width,
            toolbar_height: self.toolbar_height,
            toolbar_text_style: self.toolbar_text_style.clone(),
            title_text_style: self.title_text_style.clone(),
            system_overlay_style: self.system_overlay_style,
            actions_padding: self.actions_padding,
        })
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`AppBarThemeData::copy_with`] method instead.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`theme.copy_with().with_elevation(2.0)`). As in Dart, the copy carries neither the
    /// key nor the child, and holds the properties, never a [`data`](Self::data).
    pub fn copy_with(&self) -> AppBarTheme {
        AppBarTheme {
            key: None,
            data: None,
            background_color: self.background_color(),
            foreground_color: self.foreground_color(),
            elevation: self.elevation(),
            scrolled_under_elevation: self.scrolled_under_elevation(),
            shadow_color: self.shadow_color(),
            surface_tint_color: self.surface_tint_color(),
            shape: self.shape().cloned(),
            icon_theme: self.icon_theme().cloned(),
            actions_icon_theme: self.actions_icon_theme().cloned(),
            center_title: self.center_title(),
            title_spacing: self.title_spacing(),
            leading_width: self.leading_width(),
            toolbar_height: self.toolbar_height(),
            toolbar_text_style: self.toolbar_text_style().cloned(),
            title_text_style: self.title_text_style().cloned(),
            system_overlay_style: self.system_overlay_style(),
            actions_padding: self.actions_padding(),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Retrieves the [`AppBarThemeData`] from the closest ancestor [`AppBarTheme`].
    ///
    /// If there is no enclosing [`AppBarTheme`] widget, then
    /// `ThemeData::app_bar_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = AppBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> AppBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<AppBarTheme>(app) {
            Some(app_bar_theme) => app_bar_theme.data(),
            None => Theme::of(app, context).app_bar_theme.clone(),
        }
    }

    /// Linearly interpolate between two AppBar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: Option<&AppBarTheme>, b: Option<&AppBarTheme>, t: f64) -> AppBarTheme {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        AppBarTheme {
            key: None,
            data: None,
            background_color: Color::lerp(
                a.and_then(|a| a.background_color()),
                b.and_then(|b| b.background_color()),
                t,
            ),
            foreground_color: Color::lerp(
                a.and_then(|a| a.foreground_color()),
                b.and_then(|b| b.foreground_color()),
                t,
            ),
            elevation: lerp_double(
                a.and_then(|a| a.elevation()),
                b.and_then(|b| b.elevation()),
                t,
            ),
            scrolled_under_elevation: lerp_double(
                a.and_then(|a| a.scrolled_under_elevation()),
                b.and_then(|b| b.scrolled_under_elevation()),
                t,
            ),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color()),
                b.and_then(|b| b.shadow_color()),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color()),
                b.and_then(|b| b.surface_tint_color()),
                t,
            ),
            shape: ShapeBorderRef::lerp(a.and_then(|a| a.shape()), b.and_then(|b| b.shape()), t),
            icon_theme: Some(IconThemeData::lerp(
                a.and_then(|a| a.icon_theme()),
                b.and_then(|b| b.icon_theme()),
                t,
            )),
            actions_icon_theme: Some(IconThemeData::lerp(
                a.and_then(|a| a.actions_icon_theme()),
                b.and_then(|b| b.actions_icon_theme()),
                t,
            )),
            center_title: if t < 0.5 {
                a.and_then(|a| a.center_title())
            } else {
                b.and_then(|b| b.center_title())
            },
            title_spacing: lerp_double(
                a.and_then(|a| a.title_spacing()),
                b.and_then(|b| b.title_spacing()),
                t,
            ),
            leading_width: lerp_double(
                a.and_then(|a| a.leading_width()),
                b.and_then(|b| b.leading_width()),
                t,
            ),
            toolbar_height: lerp_double(
                a.and_then(|a| a.toolbar_height()),
                b.and_then(|b| b.toolbar_height()),
                t,
            ),
            toolbar_text_style: TextStyle::lerp(
                a.and_then(|a| a.toolbar_text_style()),
                b.and_then(|b| b.toolbar_text_style()),
                t,
            ),
            title_text_style: TextStyle::lerp(
                a.and_then(|a| a.title_text_style()),
                b.and_then(|b| b.title_text_style()),
                t,
            ),
            system_overlay_style: if t < 0.5 {
                a.and_then(|a| a.system_overlay_style())
            } else {
                b.and_then(|b| b.system_overlay_style())
            },
            actions_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.actions_padding()),
                b.and_then(|b| b.actions_padding()),
                t,
            ),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart's constructor assert: a `data` takes the place of every property.
    fn has_no_property(&self) -> bool {
        self.background_color.is_none()
            && self.foreground_color.is_none()
            && self.elevation.is_none()
            && self.scrolled_under_elevation.is_none()
            && self.shadow_color.is_none()
            && self.surface_tint_color.is_none()
            && self.shape.is_none()
            && self.icon_theme.is_none()
            && self.actions_icon_theme.is_none()
            && self.center_title.is_none()
            && self.title_spacing.is_none()
            && self.leading_width.is_none()
            && self.toolbar_height.is_none()
            && self.toolbar_text_style.is_none()
            && self.title_text_style.is_none()
            && self.system_overlay_style.is_none()
            && self.actions_padding.is_none()
    }
}

impl Default for AppBarTheme {
    fn default() -> AppBarTheme {
        AppBarTheme::new()
    }
}

impl InheritedTheme for AppBarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        AppBarTheme::new()
            .with_data(self.data())
            .child(child)
            .into_widget()
    }
}

impl InheritedWidget for AppBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &AppBarTheme) -> bool {
        self.data() != old_widget.data()
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// `ThemeData(appBarTheme:)` takes Dart's `Object?`, either an [`AppBarThemeData`] or an
/// [`AppBarTheme`] whose [`data`](AppBarTheme::data) is used.
impl From<AppBarTheme> for AppBarThemeData {
    fn from(app_bar_theme: AppBarTheme) -> AppBarThemeData {
        app_bar_theme.data()
    }
}

/// Defines default property values for descendant `AppBar` widgets.
///
/// Descendant widgets obtain the current [`AppBarThemeData`] object using
/// [`AppBarTheme::of`]. Instances of [`AppBarThemeData`] can be
/// customized with [`AppBarThemeData::copy_with`].
///
/// Typically an [`AppBarThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::app_bar_theme`.
///
/// All [`AppBarThemeData`] properties are `null` by default. When null, the `AppBar`
/// will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults. See the individual `AppBar` properties for details.
///
/// See also:
///
///  * `AppBar`, which is the widget that this theme configures.
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppBarThemeData {
    /// Overrides the default value of `AppBar::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `AppBar::foreground_color`.
    pub foreground_color: Option<Color>,

    /// Overrides the default value of `AppBar::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value of `AppBar::scrolled_under_elevation`.
    pub scrolled_under_elevation: Option<f64>,

    /// Overrides the default value of `AppBar::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value of `AppBar::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of `AppBar::shape`.
    pub shape: Option<ShapeBorderRef>,

    /// Overrides the default value of `AppBar::icon_theme`.
    pub icon_theme: Option<IconThemeData>,

    /// Overrides the default value of `AppBar::actions_icon_theme`.
    pub actions_icon_theme: Option<IconThemeData>,

    /// Overrides the default value of `AppBar::center_title`.
    pub center_title: Option<bool>,

    /// Overrides the default value of `AppBar::title_spacing`.
    pub title_spacing: Option<f64>,

    /// Overrides the default value of `AppBar::leading_width`.
    pub leading_width: Option<f64>,

    /// Overrides the default value of `AppBar::toolbar_height`.
    pub toolbar_height: Option<f64>,

    /// Overrides the default value of `AppBar::toolbar_text_style`.
    pub toolbar_text_style: Option<TextStyle>,

    /// Overrides the default value of `AppBar::title_text_style`.
    pub title_text_style: Option<TextStyle>,

    /// Overrides the default value of `AppBar::system_overlay_style`.
    pub system_overlay_style: Option<SystemUiOverlayStyle>,

    /// Overrides the default value of `AppBar::actions_padding`.
    pub actions_padding: Option<EdgeInsetsGeometry>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `AppBarThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait AppBarThemeDataGetters for AppBarThemeData, fields: app_bar_theme_data {
        background_color: Option<Color>,
        foreground_color: Option<Color>,
        elevation: Option<f64>,
        scrolled_under_elevation: Option<f64>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        shape: Option<ShapeBorderRef>,
        icon_theme: Option<IconThemeData>,
        actions_icon_theme: Option<IconThemeData>,
        center_title: Option<bool>,
        title_spacing: Option<f64>,
        leading_width: Option<f64>,
        toolbar_height: Option<f64>,
        toolbar_text_style: Option<TextStyle>,
        title_text_style: Option<TextStyle>,
        system_overlay_style: Option<SystemUiOverlayStyle>,
        actions_padding: Option<EdgeInsetsGeometry>,
    }
}

#[allow(deprecated)]
impl AppBarThemeData {
    /// Creates an app bar theme that can be used with `ThemeData::app_bar_theme`; Dart's optional
    /// named arguments are the fluent setters.
    pub fn new() -> AppBarThemeData {
        AppBarThemeData::default()
    }

    /// Dart `AppBarThemeData(color:)` and `AppBarThemeData.copyWith(color:)`: the same as
    /// [`background_color`](Self::background_color). Dart's constructor ignores it, which
    /// the one fluent setter of both cannot tell apart.
    #[deprecated(
        note = "Use backgroundColor instead. This feature was deprecated after v3.33.0-0.2.pre."
    )]
    pub fn color(self, color: Color) -> AppBarThemeData {
        self.background_color(color)
    }

    /// Dart `AppBarThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> AppBarThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `AppBarThemeData(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> AppBarThemeData {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `AppBarThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> AppBarThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `AppBarThemeData(scrolledUnderElevation:)`.
    pub fn scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> AppBarThemeData {
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    /// Dart `AppBarThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> AppBarThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `AppBarThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> AppBarThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `AppBarThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> AppBarThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `AppBarThemeData(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> AppBarThemeData {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `AppBarThemeData(actionsIconTheme:)`.
    pub fn actions_icon_theme(mut self, actions_icon_theme: IconThemeData) -> AppBarThemeData {
        self.actions_icon_theme = Some(actions_icon_theme);
        self
    }

    /// Dart `AppBarThemeData(centerTitle:)`.
    pub fn center_title(mut self, center_title: bool) -> AppBarThemeData {
        self.center_title = Some(center_title);
        self
    }

    /// Dart `AppBarThemeData(titleSpacing:)`.
    pub fn title_spacing(mut self, title_spacing: f64) -> AppBarThemeData {
        self.title_spacing = Some(title_spacing);
        self
    }

    /// Dart `AppBarThemeData(leadingWidth:)`.
    pub fn leading_width(mut self, leading_width: f64) -> AppBarThemeData {
        self.leading_width = Some(leading_width);
        self
    }

    /// Dart `AppBarThemeData(toolbarHeight:)`.
    pub fn toolbar_height(mut self, toolbar_height: f64) -> AppBarThemeData {
        self.toolbar_height = Some(toolbar_height);
        self
    }

    /// Dart `AppBarThemeData(toolbarTextStyle:)`.
    pub fn toolbar_text_style(mut self, toolbar_text_style: TextStyle) -> AppBarThemeData {
        self.toolbar_text_style = Some(toolbar_text_style);
        self
    }

    /// Dart `AppBarThemeData(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> AppBarThemeData {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `AppBarThemeData(systemOverlayStyle:)`.
    pub fn system_overlay_style(
        mut self,
        system_overlay_style: SystemUiOverlayStyle,
    ) -> AppBarThemeData {
        self.system_overlay_style = Some(system_overlay_style);
        self
    }

    /// Dart `AppBarThemeData(actionsPadding:)`.
    pub fn actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> AppBarThemeData {
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> AppBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two app bar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: &AppBarThemeData, b: &AppBarThemeData, t: f64) -> AppBarThemeData {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        AppBarThemeData {
            background_color: Color::lerp(a.background_color, b.background_color, t),
            foreground_color: Color::lerp(a.foreground_color, b.foreground_color, t),
            elevation: lerp_double(a.elevation, b.elevation, t),
            scrolled_under_elevation: lerp_double(
                a.scrolled_under_elevation,
                b.scrolled_under_elevation,
                t,
            ),
            shadow_color: Color::lerp(a.shadow_color, b.shadow_color, t),
            surface_tint_color: Color::lerp(a.surface_tint_color, b.surface_tint_color, t),
            shape: ShapeBorderRef::lerp(a.shape.as_ref(), b.shape.as_ref(), t),
            icon_theme: Some(IconThemeData::lerp(
                a.icon_theme.as_ref(),
                b.icon_theme.as_ref(),
                t,
            )),
            actions_icon_theme: Some(IconThemeData::lerp(
                a.actions_icon_theme.as_ref(),
                b.actions_icon_theme.as_ref(),
                t,
            )),
            center_title: if t < 0.5 {
                a.center_title
            } else {
                b.center_title
            },
            title_spacing: lerp_double(a.title_spacing, b.title_spacing, t),
            leading_width: lerp_double(a.leading_width, b.leading_width, t),
            toolbar_height: lerp_double(a.toolbar_height, b.toolbar_height, t),
            toolbar_text_style: TextStyle::lerp(
                a.toolbar_text_style.as_ref(),
                b.toolbar_text_style.as_ref(),
                t,
            ),
            title_text_style: TextStyle::lerp(
                a.title_text_style.as_ref(),
                b.title_text_style.as_ref(),
                t,
            ),
            system_overlay_style: if t < 0.5 {
                a.system_overlay_style
            } else {
                b.system_overlay_style
            },
            actions_padding: EdgeInsetsGeometry::lerp(a.actions_padding, b.actions_padding, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::ShapeBorder;
    use waft_painting::{BorderRadiusGeometry, BorderSide, FontStyle, RoundedRectangleBorder};

    use super::*;
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    /// Dart's `appBarTheme` constant at the top of `app_bar_theme_test.dart`.
    fn app_bar_theme_data() -> AppBarThemeData {
        AppBarThemeData::new()
            .background_color(Color::new(0xff00ff00))
            .foreground_color(Color::new(0xff00ffff))
            .elevation(4.0)
            .scrolled_under_elevation(6.0)
            .shadow_color(Color::new(0xff1212ff))
            .shape(rounded(14.0))
            .icon_theme(IconThemeData::new().color(Color::new(0xffff0000)))
            .actions_icon_theme(IconThemeData::new().color(Color::new(0xff0000ff)))
            .center_title(false)
            .title_spacing(10.0)
            .title_text_style(
                TextStyle::new()
                    .font_size(22.0)
                    .font_style(FontStyle::Italic),
            )
    }

    /// `AppBarTheme` with the same properties as [`app_bar_theme_data`], by the `with_*` setters.
    fn app_bar_theme_of_properties() -> AppBarTheme {
        AppBarTheme::new()
            .with_background_color(Color::new(0xff00ff00))
            .with_foreground_color(Color::new(0xff00ffff))
            .with_elevation(4.0)
            .with_scrolled_under_elevation(6.0)
            .with_shadow_color(Color::new(0xff1212ff))
            .with_shape(rounded(14.0))
            .with_icon_theme(IconThemeData::new().color(Color::new(0xffff0000)))
            .with_actions_icon_theme(IconThemeData::new().color(Color::new(0xff0000ff)))
            .with_center_title(false)
            .with_title_spacing(10.0)
            .with_title_text_style(
                TextStyle::new()
                    .font_size(22.0)
                    .font_style(FontStyle::Italic),
            )
    }

    #[test]
    fn app_bar_theme_data_copy_with_equality_basics() {
        assert_eq!(AppBarThemeData::new(), AppBarThemeData::new().copy_with());

        let theme = app_bar_theme_data();
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(
            theme.copy_with().leading_width(40.0).leading_width,
            Some(40.0)
        );
        assert_eq!(theme.copy_with().leading_width(40.0).elevation, Some(4.0));
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(theme.copy_with().shape(rounded(9.0)), theme);
    }

    #[test]
    fn app_bar_theme_data_null_fields_by_default() {
        let theme = AppBarThemeData::new();
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.foreground_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.scrolled_under_elevation, None);
        assert_eq!(theme.shadow_color, None);
        assert_eq!(theme.surface_tint_color, None);
        assert!(theme.shape.is_none());
        assert_eq!(theme.icon_theme, None);
        assert_eq!(theme.actions_icon_theme, None);
        assert_eq!(theme.center_title, None);
        assert_eq!(theme.title_spacing, None);
        assert_eq!(theme.leading_width, None);
        assert_eq!(theme.toolbar_height, None);
        assert_eq!(theme.toolbar_text_style, None);
        assert_eq!(theme.title_text_style, None);
        assert_eq!(theme.system_overlay_style, None);
        assert_eq!(theme.actions_padding, None);
    }

    #[test]
    #[allow(deprecated)]
    fn app_bar_theme_data_color_is_the_background_color() {
        let theme = AppBarThemeData::new().color(Colors::RED.primary());

        assert_eq!(theme.background_color, Some(Colors::RED.primary()));
        assert_eq!(
            app_bar_theme_data()
                .copy_with()
                .color(Colors::RED.primary())
                .background_color,
            Some(Colors::RED.primary())
        );
    }

    #[test]
    fn app_bar_theme_lerp_special_cases() {
        let data = AppBarTheme::new();
        assert_eq!(
            AppBarTheme::lerp(Some(&data), Some(&data), 0.5).data(),
            data.data()
        );

        let data = app_bar_theme_data();
        assert_eq!(AppBarThemeData::lerp(&data, &data, 0.5), data);
    }

    #[test]
    fn app_bar_theme_data_lerps_each_field() {
        let a = AppBarThemeData::new()
            .background_color(Colors::BLACK)
            .elevation(2.0)
            .shape(rounded(4.0))
            .icon_theme(IconThemeData::new().size(10.0))
            .center_title(true)
            .title_spacing(10.0)
            .toolbar_height(40.0)
            .title_text_style(TextStyle::new().font_size(10.0))
            .system_overlay_style(SystemUiOverlayStyle::LIGHT)
            .actions_padding(EdgeInsetsGeometry::all(10.0));
        let b = AppBarThemeData::new()
            .background_color(Colors::WHITE)
            .foreground_color(Colors::RED.primary())
            .elevation(6.0)
            .shape(rounded(8.0))
            .icon_theme(IconThemeData::new().size(30.0))
            .center_title(false)
            .title_spacing(30.0)
            .toolbar_height(80.0)
            .title_text_style(TextStyle::new().font_size(20.0))
            .system_overlay_style(SystemUiOverlayStyle::DARK)
            .actions_padding(EdgeInsetsGeometry::all(30.0));

        let halfway = AppBarThemeData::lerp(&a, &b, 0.5);

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.foreground_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(
            halfway.shape.as_deref(),
            Some(&rounded(6.0) as &dyn ShapeBorder)
        );
        assert_eq!(halfway.icon_theme, Some(IconThemeData::new().size(20.0)));
        assert_eq!(halfway.center_title, Some(false));
        assert_eq!(halfway.title_spacing, Some(20.0));
        assert_eq!(halfway.toolbar_height, Some(60.0));
        assert_eq!(
            halfway.title_text_style.and_then(|style| style.font_size),
            Some(15.0)
        );
        assert_eq!(
            halfway.system_overlay_style,
            Some(SystemUiOverlayStyle::DARK)
        );
        assert_eq!(halfway.actions_padding, Some(EdgeInsetsGeometry::all(20.0)));

        let early = AppBarThemeData::lerp(&a, &b, 0.25);
        assert_eq!(early.center_title, Some(true));
        assert_eq!(
            early.system_overlay_style,
            Some(SystemUiOverlayStyle::LIGHT)
        );
    }

    #[test]
    fn app_bar_theme_data_lerp_gives_an_icon_theme_even_when_neither_side_has_one() {
        let lerped = AppBarThemeData::lerp(&AppBarThemeData::new(), &AppBarThemeData::new(), 0.5);

        assert_eq!(lerped.icon_theme, Some(IconThemeData::new()));
        assert_eq!(lerped.actions_icon_theme, Some(IconThemeData::new()));
    }

    #[test]
    fn app_bar_theme_getters_read_the_properties_it_was_given() {
        let theme = app_bar_theme_of_properties();

        assert_eq!(theme.background_color(), Some(Color::new(0xff00ff00)));
        assert_eq!(theme.foreground_color(), Some(Color::new(0xff00ffff)));
        assert_eq!(theme.elevation(), Some(4.0));
        assert_eq!(theme.scrolled_under_elevation(), Some(6.0));
        assert_eq!(theme.shadow_color(), Some(Color::new(0xff1212ff)));
        assert_eq!(theme.surface_tint_color(), None);
        assert_eq!(theme.shape(), Some(&ShapeBorderRef::new(rounded(14.0))));
        assert_eq!(
            theme.icon_theme(),
            Some(&IconThemeData::new().color(Color::new(0xffff0000)))
        );
        assert_eq!(theme.center_title(), Some(false));
        assert_eq!(theme.title_spacing(), Some(10.0));
        assert_eq!(theme.system_overlay_style(), None);
        assert_eq!(theme.data(), app_bar_theme_data());
    }

    #[test]
    fn app_bar_theme_getters_read_its_data_when_it_has_one() {
        let theme = AppBarTheme::new().with_data(app_bar_theme_data());

        assert_eq!(theme.background_color(), Some(Color::new(0xff00ff00)));
        assert_eq!(theme.elevation(), Some(4.0));
        assert_eq!(theme.shape(), Some(&ShapeBorderRef::new(rounded(14.0))));
        assert_eq!(theme.center_title(), Some(false));
        assert_eq!(theme.data(), app_bar_theme_data());
    }

    #[test]
    #[allow(deprecated)]
    fn app_bar_theme_color_is_the_background_color() {
        assert_eq!(
            AppBarTheme::new()
                .with_color(Colors::RED.primary())
                .background_color(),
            Some(Colors::RED.primary())
        );
    }

    #[test]
    fn app_bar_theme_copy_with_keeps_the_properties_it_is_not_given() {
        let theme = app_bar_theme_of_properties();

        let copy = theme.copy_with().with_elevation(8.0);

        assert_eq!(copy.elevation(), Some(8.0));
        assert_eq!(copy.background_color(), Some(Color::new(0xff00ff00)));
        assert_eq!(copy.data(), app_bar_theme_data().copy_with().elevation(8.0));
        assert_eq!(
            AppBarTheme::new()
                .with_data(app_bar_theme_data())
                .copy_with()
                .with_actions_padding(EdgeInsetsGeometry::all(2.0))
                .data(),
            app_bar_theme_data().actions_padding(EdgeInsetsGeometry::all(2.0))
        );
    }

    #[test]
    #[should_panic]
    fn app_bar_theme_asserts_when_data_is_given_after_a_property() {
        AppBarTheme::new()
            .with_elevation(1.0)
            .with_data(AppBarThemeData::new());
    }

    #[test]
    #[should_panic]
    fn app_bar_theme_asserts_when_a_property_is_given_after_data() {
        AppBarTheme::new()
            .with_data(AppBarThemeData::new())
            .with_elevation(1.0);
    }

    #[test]
    fn of_returns_the_theme_data_app_bar_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = app_bar_theme_data();
            let theme_data_app_bar = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .app_bar_theme(theme_data_app_bar.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                AppBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_app_bar_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = app_bar_theme_data();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .app_bar_theme(AppBarThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(data, app_bar_theme_of_properties().child(child)).into_widget()
                },
                AppBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_app_bar_theme_is_kept_under_another_app_bar_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();

            let seen = theme_seen_through_capture(
                &cell,
                |probe| {
                    AppBarTheme::new()
                        .with_elevation(1.0)
                        .child(probe)
                        .into_widget()
                },
                |wrapped| {
                    AppBarTheme::new()
                        .with_data(AppBarThemeData::new().elevation(2.0))
                        .child(wrapped)
                        .into_widget()
                },
                AppBarTheme::of,
            );

            assert_eq!(seen.elevation, Some(1.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let old = AppBarTheme::new().with_elevation(1.0);

        assert!(
            !AppBarTheme::new()
                .with_data(AppBarThemeData::new().elevation(1.0))
                .update_should_notify(&old)
        );
        assert!(
            AppBarTheme::new()
                .with_elevation(2.0)
                .update_should_notify(&old)
        );
    }

    #[test]
    fn theme_data_holds_the_app_bar_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.app_bar_theme, AppBarThemeData::new());

            let raised = ThemeData::builder()
                .app_bar_theme(AppBarThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.app_bar_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .app_bar_theme(raised.app_bar_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .app_bar_theme(AppBarThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5).app_bar_theme.elevation,
                Some(4.0)
            );
        });
    }

    #[test]
    fn theme_data_takes_the_data_of_an_app_bar_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();

            let built = ThemeData::builder()
                .app_bar_theme(app_bar_theme_of_properties())
                .build(&app);
            let copied = ThemeData::new(&app)
                .copy_with()
                .app_bar_theme(app_bar_theme_of_properties())
                .build();

            assert_eq!(built.app_bar_theme, app_bar_theme_data());
            assert_eq!(copied.app_bar_theme, app_bar_theme_data());
        });
    }
}

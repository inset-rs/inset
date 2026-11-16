//! Flutter counterpart: material_ui's `tab_bar_theme.dart`.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{
    AnyColor, BorderRadius, DecorationRef, EdgeInsetsGeometry, TextScaler, TextStyle,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, SizedBox, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::tabs::{TabAlignment, TabBarIndicatorSize, TabIndicatorAnimation};
use crate::theme::Theme;

/// Defines default property values for descendant `TabBar` widgets.
///
/// Descendant widgets obtain the current [`TabBarThemeData`] object using
/// [`TabBarTheme::of`]. Instances of [`TabBarThemeData`] can be customized
/// with [`TabBarThemeData::copy_with`].
///
/// Typically a [`TabBarThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::tab_bar_theme`.
///
/// Dart's `TabBarTheme` constructor takes either the properties or a [`data`](Self::data);
/// here the properties are the `with_*` setters and [`with_data`](Self::with_data) the other,
/// and each setter that breaks Dart's constructor assert (a `data` together with any property)
/// fails in a debug build. The getters are methods, as Dart's, hence the `with_` prefix.
///
/// See also:
///
///  * [`TabBarThemeData`], which describes the actual configuration of a tab
///    bar theme.
#[derive(Clone, Debug)]
pub struct TabBarTheme {
    pub key: Option<KeyRef>,
    data: Option<TabBarThemeData>,
    indicator: Option<DecorationRef>,
    indicator_color: Option<Color>,
    indicator_size: Option<TabBarIndicatorSize>,
    divider_color: Option<Color>,
    divider_height: Option<f64>,
    label_color: Option<AnyColor>,
    label_padding: Option<EdgeInsetsGeometry>,
    label_style: Option<TextStyle>,
    unselected_label_color: Option<Color>,
    unselected_label_style: Option<TextStyle>,
    overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
    mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
    tab_alignment: Option<TabAlignment>,
    text_scaler: Option<TextScaler>,
    indicator_animation: Option<TabIndicatorAnimation>,
    child: WidgetRef,
}

impl TabBarTheme {
    /// Creates a tab bar theme that can be used with `ThemeData::tab_bar_theme`.
    pub fn new() -> TabBarTheme {
        TabBarTheme {
            key: None,
            data: None,
            indicator: None,
            indicator_color: None,
            indicator_size: None,
            divider_color: None,
            divider_height: None,
            label_color: None,
            label_padding: None,
            label_style: None,
            unselected_label_color: None,
            unselected_label_style: None,
            overlay_color: None,
            splash_factory: None,
            mouse_cursor: None,
            tab_alignment: None,
            text_scaler: None,
            indicator_animation: None,
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart `TabBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> TabBarTheme {
        self.key = Some(key);
        self
    }

    /// Dart `TabBarTheme(child:)`; the default is an empty `SizedBox`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> TabBarTheme {
        self.child = child.into_widget();
        self
    }

    /// Dart `TabBarTheme(data:)`.
    pub fn with_data(mut self, data: TabBarThemeData) -> TabBarTheme {
        debug_assert!(self.has_no_property());
        self.data = Some(data);
        self
    }

    /// Dart `TabBarTheme(indicator:)`.
    pub fn with_indicator(mut self, indicator: impl Into<DecorationRef>) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.indicator = Some(indicator.into());
        self
    }

    /// Dart `TabBarTheme(indicatorColor:)`.
    pub fn with_indicator_color(mut self, indicator_color: Color) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `TabBarTheme(indicatorSize:)`.
    pub fn with_indicator_size(mut self, indicator_size: TabBarIndicatorSize) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.indicator_size = Some(indicator_size);
        self
    }

    /// Dart `TabBarTheme(dividerColor:)`.
    pub fn with_divider_color(mut self, divider_color: Color) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `TabBarTheme(dividerHeight:)`.
    pub fn with_divider_height(mut self, divider_height: f64) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.divider_height = Some(divider_height);
        self
    }

    /// Dart `TabBarTheme(labelColor:)`.
    pub fn with_label_color(mut self, label_color: impl Into<AnyColor>) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.label_color = Some(label_color.into());
        self
    }

    /// Dart `TabBarTheme(labelPadding:)`.
    pub fn with_label_padding(
        mut self,
        label_padding: impl Into<EdgeInsetsGeometry>,
    ) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Dart `TabBarTheme(labelStyle:)`.
    pub fn with_label_style(mut self, label_style: TextStyle) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.label_style = Some(label_style);
        self
    }

    /// Dart `TabBarTheme(unselectedLabelColor:)`.
    pub fn with_unselected_label_color(mut self, unselected_label_color: Color) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.unselected_label_color = Some(unselected_label_color);
        self
    }

    /// Dart `TabBarTheme(unselectedLabelStyle:)`.
    pub fn with_unselected_label_style(mut self, unselected_label_style: TextStyle) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    /// Dart `TabBarTheme(overlayColor:)`.
    pub fn with_overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `TabBarTheme(splashFactory:)`.
    pub fn with_splash_factory(
        mut self,
        splash_factory: InteractiveInkFeatureFactoryRef,
    ) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.splash_factory = Some(splash_factory);
        self
    }

    /// Dart `TabBarTheme(mouseCursor:)`.
    pub fn with_mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `TabBarTheme(tabAlignment:)`.
    pub fn with_tab_alignment(mut self, tab_alignment: TabAlignment) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.tab_alignment = Some(tab_alignment);
        self
    }

    /// Dart `TabBarTheme(textScaler:)`.
    pub fn with_text_scaler(mut self, text_scaler: TextScaler) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `TabBarTheme(indicatorAnimation:)`.
    pub fn with_indicator_animation(
        mut self,
        indicator_animation: TabIndicatorAnimation,
    ) -> TabBarTheme {
        debug_assert!(self.data.is_none());
        self.indicator_animation = Some(indicator_animation);
        self
    }

    /// Overrides the default value for `TabBar::indicator`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::indicator`] property in [`data`](Self::data) instead.
    pub fn indicator(&self) -> Option<&DecorationRef> {
        match &self.data {
            Some(data) => data.indicator.as_ref(),
            None => self.indicator.as_ref(),
        }
    }

    /// Overrides the default value for `TabBar::indicator_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::indicator_color`] property in [`data`](Self::data) instead.
    pub fn indicator_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.indicator_color,
            None => self.indicator_color,
        }
    }

    /// Overrides the default value for `TabBar::indicator_size`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::indicator_size`] property in [`data`](Self::data) instead.
    pub fn indicator_size(&self) -> Option<TabBarIndicatorSize> {
        match &self.data {
            Some(data) => data.indicator_size,
            None => self.indicator_size,
        }
    }

    /// Overrides the default value for `TabBar::divider_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::divider_color`] property in [`data`](Self::data) instead.
    pub fn divider_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.divider_color,
            None => self.divider_color,
        }
    }

    /// Overrides the default value for `TabBar::divider_height`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::divider_height`] property in [`data`](Self::data) instead.
    pub fn divider_height(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.divider_height,
            None => self.divider_height,
        }
    }

    /// Overrides the default value for `TabBar::label_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::label_color`] property in [`data`](Self::data) instead.
    pub fn label_color(&self) -> Option<&AnyColor> {
        match &self.data {
            Some(data) => data.label_color.as_ref(),
            None => self.label_color.as_ref(),
        }
    }

    /// Overrides the default value for `TabBar::label_padding`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::label_padding`] property in [`data`](Self::data) instead.
    pub fn label_padding(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.label_padding,
            None => self.label_padding,
        }
    }

    /// Overrides the default value for `TabBar::label_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::label_style`] property in [`data`](Self::data) instead.
    pub fn label_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.label_style.as_ref(),
            None => self.label_style.as_ref(),
        }
    }

    /// Overrides the default value for `TabBar::unselected_label_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::unselected_label_color`] property in [`data`](Self::data) instead.
    pub fn unselected_label_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.unselected_label_color,
            None => self.unselected_label_color,
        }
    }

    /// Overrides the default value for `TabBar::unselected_label_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::unselected_label_style`] property in [`data`](Self::data) instead.
    pub fn unselected_label_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.unselected_label_style.as_ref(),
            None => self.unselected_label_style.as_ref(),
        }
    }

    /// Overrides the default value for `TabBar::overlay_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::overlay_color`] property in [`data`](Self::data) instead.
    pub fn overlay_color(&self) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        match &self.data {
            Some(data) => data.overlay_color.clone(),
            None => self.overlay_color.clone(),
        }
    }

    /// Overrides the default value for `TabBar::splash_factory`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::splash_factory`] property in [`data`](Self::data) instead.
    pub fn splash_factory(&self) -> Option<InteractiveInkFeatureFactoryRef> {
        match &self.data {
            Some(data) => data.splash_factory.clone(),
            None => self.splash_factory.clone(),
        }
    }

    /// Overrides the default value of `TabBar::mouse_cursor`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::mouse_cursor`] property in [`data`](Self::data) instead.
    pub fn mouse_cursor(&self) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        match &self.data {
            Some(data) => data.mouse_cursor.clone(),
            None => self.mouse_cursor.clone(),
        }
    }

    /// Overrides the default value for `TabBar::tab_alignment`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::tab_alignment`] property in [`data`](Self::data) instead.
    pub fn tab_alignment(&self) -> Option<TabAlignment> {
        match &self.data {
            Some(data) => data.tab_alignment,
            None => self.tab_alignment,
        }
    }

    /// Overrides the default value for `TabBar::text_scaler`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::text_scaler`] property in [`data`](Self::data) instead.
    pub fn text_scaler(&self) -> Option<&TextScaler> {
        match &self.data {
            Some(data) => data.text_scaler.as_ref(),
            None => self.text_scaler.as_ref(),
        }
    }

    /// Overrides the default value for `TabBar::indicator_animation`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::indicator_animation`] property in [`data`](Self::data) instead.
    pub fn indicator_animation(&self) -> Option<TabIndicatorAnimation> {
        match &self.data {
            Some(data) => data.indicator_animation,
            None => self.indicator_animation,
        }
    }

    /// The properties used for all descendant `TabBar` widgets.
    pub fn data(&self) -> TabBarThemeData {
        self.data.clone().unwrap_or_else(|| TabBarThemeData {
            indicator: self.indicator.clone(),
            indicator_color: self.indicator_color,
            indicator_size: self.indicator_size,
            divider_color: self.divider_color,
            divider_height: self.divider_height,
            label_color: self.label_color.clone(),
            label_padding: self.label_padding,
            label_style: self.label_style.clone(),
            unselected_label_color: self.unselected_label_color,
            unselected_label_style: self.unselected_label_style.clone(),
            overlay_color: self.overlay_color.clone(),
            splash_factory: self.splash_factory.clone(),
            mouse_cursor: self.mouse_cursor.clone(),
            tab_alignment: self.tab_alignment,
            text_scaler: self.text_scaler.clone(),
            indicator_animation: self.indicator_animation,
            splash_border_radius: None,
        })
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::copy_with`] instead.
    ///
    /// Dart's named arguments are the `with_*` setters on the copy
    /// (`theme.copy_with().with_divider_height(2.0)`). As in Dart, the copy carries neither the
    /// key nor the child, and holds the properties, never a [`data`](Self::data).
    pub fn copy_with(&self) -> TabBarTheme {
        TabBarTheme {
            key: None,
            data: None,
            indicator: self.indicator().cloned(),
            indicator_color: self.indicator_color(),
            indicator_size: self.indicator_size(),
            divider_color: self.divider_color(),
            divider_height: self.divider_height(),
            label_color: self.label_color().cloned(),
            label_padding: self.label_padding(),
            label_style: self.label_style().cloned(),
            unselected_label_color: self.unselected_label_color(),
            unselected_label_style: self.unselected_label_style().cloned(),
            overlay_color: self.overlay_color(),
            splash_factory: self.splash_factory(),
            mouse_cursor: self.mouse_cursor(),
            tab_alignment: self.tab_alignment(),
            text_scaler: self.text_scaler().cloned(),
            indicator_animation: self.indicator_animation(),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`TabBarTheme`] ancestor.
    /// If there is no ancestor, it returns `ThemeData::tab_bar_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = TabBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> TabBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<TabBarTheme>(app) {
            Some(tab_bar_theme) => tab_bar_theme.data(),
            None => Theme::of(app, context).tab_bar_theme.clone(),
        }
    }

    /// Linearly interpolate between two tab bar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`TabBarThemeData::lerp`] instead.
    pub fn lerp(a: &TabBarTheme, b: &TabBarTheme, t: f64) -> TabBarTheme {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        TabBarTheme {
            key: None,
            data: None,
            indicator: DecorationRef::lerp(a.indicator(), b.indicator(), t),
            indicator_color: Color::lerp(a.indicator_color(), b.indicator_color(), t),
            indicator_size: if t < 0.5 {
                a.indicator_size()
            } else {
                b.indicator_size()
            },
            divider_color: Color::lerp(a.divider_color(), b.divider_color(), t),
            divider_height: if t < 0.5 {
                a.divider_height()
            } else {
                b.divider_height()
            },
            label_color: AnyColor::lerp(a.label_color(), b.label_color(), t).map(AnyColor::new),
            label_padding: EdgeInsetsGeometry::lerp(a.label_padding(), b.label_padding(), t),
            label_style: TextStyle::lerp(a.label_style(), b.label_style(), t),
            unselected_label_color: Color::lerp(
                a.unselected_label_color(),
                b.unselected_label_color(),
                t,
            ),
            unselected_label_style: TextStyle::lerp(
                a.unselected_label_style(),
                b.unselected_label_style(),
                t,
            ),
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.overlay_color(),
                b.overlay_color(),
                t,
                Color::lerp,
            ),
            splash_factory: if t < 0.5 {
                a.splash_factory()
            } else {
                b.splash_factory()
            },
            mouse_cursor: if t < 0.5 {
                a.mouse_cursor()
            } else {
                b.mouse_cursor()
            },
            tab_alignment: if t < 0.5 {
                a.tab_alignment()
            } else {
                b.tab_alignment()
            },
            text_scaler: if t < 0.5 {
                a.text_scaler().cloned()
            } else {
                b.text_scaler().cloned()
            },
            indicator_animation: if t < 0.5 {
                a.indicator_animation()
            } else {
                b.indicator_animation()
            },
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart's constructor assert: a `data` takes the place of every property.
    fn has_no_property(&self) -> bool {
        self.indicator.is_none()
            && self.indicator_color.is_none()
            && self.indicator_size.is_none()
            && self.divider_color.is_none()
            && self.divider_height.is_none()
            && self.label_color.is_none()
            && self.label_padding.is_none()
            && self.label_style.is_none()
            && self.unselected_label_color.is_none()
            && self.unselected_label_style.is_none()
            && self.overlay_color.is_none()
            && self.splash_factory.is_none()
            && self.mouse_cursor.is_none()
            && self.tab_alignment.is_none()
            && self.text_scaler.is_none()
            && self.indicator_animation.is_none()
    }
}

impl Default for TabBarTheme {
    fn default() -> TabBarTheme {
        TabBarTheme::new()
    }
}

impl InheritedTheme for TabBarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        TabBarTheme::new()
            .with_data(self.data())
            .child(child)
            .into_widget()
    }
}

impl InheritedWidget for TabBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TabBarTheme) -> bool {
        self.data() != old_widget.data()
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Defines default property values for descendant `TabBar` widgets.
///
/// Descendant widgets obtain the current [`TabBarThemeData`] object using
/// [`TabBarTheme::of`]. Instances of [`TabBarThemeData`] can be customized
/// with [`TabBarThemeData::copy_with`].
///
/// Typically a [`TabBarThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::tab_bar_theme`.
///
/// All [`TabBarThemeData`] properties are `null` by default. When null, the `TabBar`
/// will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults. See the individual `TabBar` properties for details.
///
/// See also:
///
///  * `TabBar`, which displays a row of tabs.
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct TabBarThemeData {
    /// Overrides the default value for `TabBar::indicator`.
    pub indicator: Option<DecorationRef>,

    /// Overrides the default value for `TabBar::indicator_color`.
    pub indicator_color: Option<Color>,

    /// Overrides the default value for `TabBar::indicator_size`.
    pub indicator_size: Option<TabBarIndicatorSize>,

    /// Overrides the default value for `TabBar::divider_color`.
    pub divider_color: Option<Color>,

    /// Overrides the default value for `TabBar::divider_height`.
    pub divider_height: Option<f64>,

    /// Overrides the default value for `TabBar::label_color`.
    ///
    /// If [`label_color`](Self::label_color) is a `WidgetStateColor`, then the effective color will
    /// depend on the `WidgetState::Selected` state, i.e. if the `Tab` is
    /// selected or not. In case of unselected state, this `WidgetStateColor`'s
    /// resolved color will be used even if `TabBar::unselected_label_color` or
    /// [`unselected_label_color`](Self::unselected_label_color) is non-null.
    pub label_color: Option<AnyColor>,

    /// Overrides the default value for `TabBar::label_padding`.
    ///
    /// If there are few tabs with both icon and text and few
    /// tabs with only icon or text, this padding is vertically
    /// adjusted to provide uniform padding to all tabs.
    pub label_padding: Option<EdgeInsetsGeometry>,

    /// Overrides the default value for `TabBar::label_style`.
    pub label_style: Option<TextStyle>,

    /// Overrides the default value for `TabBar::unselected_label_color`.
    pub unselected_label_color: Option<Color>,

    /// Overrides the default value for `TabBar::unselected_label_style`.
    pub unselected_label_style: Option<TextStyle>,

    /// Overrides the default value for `TabBar::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value for `TabBar::splash_factory`.
    pub splash_factory: Option<InteractiveInkFeatureFactoryRef>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// individual tab widgets.
    ///
    /// If `TabBar::mouse_cursor` is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Selected`.
    ///
    /// If specified, overrides the default value of `TabBar::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Overrides the default value for `TabBar::tab_alignment`.
    pub tab_alignment: Option<TabAlignment>,

    /// Overrides the default value for `TabBar::text_scaler`.
    pub text_scaler: Option<TextScaler>,

    /// Overrides the default value for `TabBar::indicator_animation`.
    pub indicator_animation: Option<TabIndicatorAnimation>,

    /// Defines the clipping radius of splashes that extend outside the bounds of the tab.
    pub splash_border_radius: Option<BorderRadius>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `TabBarThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait TabBarThemeDataGetters for TabBarThemeData, fields: tab_bar_theme_data {
        indicator: Option<DecorationRef>,
        indicator_color: Option<Color>,
        indicator_size: Option<TabBarIndicatorSize>,
        divider_color: Option<Color>,
        divider_height: Option<f64>,
        label_color: Option<AnyColor>,
        label_padding: Option<EdgeInsetsGeometry>,
        label_style: Option<TextStyle>,
        unselected_label_color: Option<Color>,
        unselected_label_style: Option<TextStyle>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        splash_factory: Option<InteractiveInkFeatureFactoryRef>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        tab_alignment: Option<TabAlignment>,
        text_scaler: Option<TextScaler>,
        indicator_animation: Option<TabIndicatorAnimation>,
        splash_border_radius: Option<BorderRadius>,
    }
}

impl TabBarThemeData {
    /// Creates a tab bar theme that can be used with `ThemeData::tab_bar_theme`; Dart's
    /// named arguments are the fluent setters.
    pub fn new() -> TabBarThemeData {
        TabBarThemeData::default()
    }

    /// Dart `TabBarThemeData(indicator:)`.
    pub fn indicator(mut self, indicator: impl Into<DecorationRef>) -> TabBarThemeData {
        self.indicator = Some(indicator.into());
        self
    }

    /// Dart `TabBarThemeData(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> TabBarThemeData {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `TabBarThemeData(indicatorSize:)`.
    pub fn indicator_size(mut self, indicator_size: TabBarIndicatorSize) -> TabBarThemeData {
        self.indicator_size = Some(indicator_size);
        self
    }

    /// Dart `TabBarThemeData(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> TabBarThemeData {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `TabBarThemeData(dividerHeight:)`.
    pub fn divider_height(mut self, divider_height: f64) -> TabBarThemeData {
        self.divider_height = Some(divider_height);
        self
    }

    /// Dart `TabBarThemeData(labelColor:)`.
    pub fn label_color(mut self, label_color: impl Into<AnyColor>) -> TabBarThemeData {
        self.label_color = Some(label_color.into());
        self
    }

    /// Dart `TabBarThemeData(labelPadding:)`.
    pub fn label_padding(
        mut self,
        label_padding: impl Into<EdgeInsetsGeometry>,
    ) -> TabBarThemeData {
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Dart `TabBarThemeData(labelStyle:)`.
    pub fn label_style(mut self, label_style: TextStyle) -> TabBarThemeData {
        self.label_style = Some(label_style);
        self
    }

    /// Dart `TabBarThemeData(unselectedLabelColor:)`.
    pub fn unselected_label_color(mut self, unselected_label_color: Color) -> TabBarThemeData {
        self.unselected_label_color = Some(unselected_label_color);
        self
    }

    /// Dart `TabBarThemeData(unselectedLabelStyle:)`.
    pub fn unselected_label_style(mut self, unselected_label_style: TextStyle) -> TabBarThemeData {
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    /// Dart `TabBarThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> TabBarThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `TabBarThemeData(splashFactory:)`.
    pub fn splash_factory(
        mut self,
        splash_factory: InteractiveInkFeatureFactoryRef,
    ) -> TabBarThemeData {
        self.splash_factory = Some(splash_factory);
        self
    }

    /// Dart `TabBarThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> TabBarThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `TabBarThemeData(tabAlignment:)`.
    pub fn tab_alignment(mut self, tab_alignment: TabAlignment) -> TabBarThemeData {
        self.tab_alignment = Some(tab_alignment);
        self
    }

    /// Dart `TabBarThemeData(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> TabBarThemeData {
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `TabBarThemeData(indicatorAnimation:)`.
    pub fn indicator_animation(
        mut self,
        indicator_animation: TabIndicatorAnimation,
    ) -> TabBarThemeData {
        self.indicator_animation = Some(indicator_animation);
        self
    }

    /// Dart `TabBarThemeData(splashBorderRadius:)`.
    pub fn splash_border_radius(mut self, splash_border_radius: BorderRadius) -> TabBarThemeData {
        self.splash_border_radius = Some(splash_border_radius);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().divider_height(2.0)`).
    pub fn copy_with(&self) -> TabBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two tab bar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: &TabBarThemeData, b: &TabBarThemeData, t: f64) -> TabBarThemeData {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        TabBarThemeData {
            indicator: DecorationRef::lerp(a.indicator.as_ref(), b.indicator.as_ref(), t),
            indicator_color: Color::lerp(a.indicator_color, b.indicator_color, t),
            indicator_size: if t < 0.5 {
                a.indicator_size
            } else {
                b.indicator_size
            },
            divider_color: Color::lerp(a.divider_color, b.divider_color, t),
            divider_height: if t < 0.5 {
                a.divider_height
            } else {
                b.divider_height
            },
            label_color: AnyColor::lerp(a.label_color.as_ref(), b.label_color.as_ref(), t)
                .map(AnyColor::new),
            label_padding: EdgeInsetsGeometry::lerp(a.label_padding, b.label_padding, t),
            label_style: TextStyle::lerp(a.label_style.as_ref(), b.label_style.as_ref(), t),
            unselected_label_color: Color::lerp(
                a.unselected_label_color,
                b.unselected_label_color,
                t,
            ),
            unselected_label_style: TextStyle::lerp(
                a.unselected_label_style.as_ref(),
                b.unselected_label_style.as_ref(),
                t,
            ),
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.overlay_color.clone(),
                b.overlay_color.clone(),
                t,
                Color::lerp,
            ),
            splash_factory: if t < 0.5 {
                a.splash_factory.clone()
            } else {
                b.splash_factory.clone()
            },
            mouse_cursor: if t < 0.5 {
                a.mouse_cursor.clone()
            } else {
                b.mouse_cursor.clone()
            },
            tab_alignment: if t < 0.5 {
                a.tab_alignment
            } else {
                b.tab_alignment
            },
            text_scaler: if t < 0.5 {
                a.text_scaler.clone()
            } else {
                b.text_scaler.clone()
            },
            indicator_animation: if t < 0.5 {
                a.indicator_animation
            } else {
                b.indicator_animation
            },
            // Dart lerps `a.splashBorderRadius` with itself.
            splash_border_radius: BorderRadius::lerp(
                a.splash_border_radius,
                a.splash_border_radius,
                t,
            ),
        }
    }
}

impl PartialEq for TabBarThemeData {
    fn eq(&self, other: &TabBarThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.indicator == self.indicator
            && other.indicator_color == self.indicator_color
            && other.indicator_size == self.indicator_size
            && other.divider_color == self.divider_color
            && other.divider_height == self.divider_height
            && other.label_color == self.label_color
            && other.label_padding == self.label_padding
            && other.label_style == self.label_style
            && other.unselected_label_color == self.unselected_label_color
            && other.unselected_label_style == self.unselected_label_style
            && other.overlay_color == self.overlay_color
            && other.splash_factory == self.splash_factory
            && other.mouse_cursor == self.mouse_cursor
            && other.tab_alignment == self.tab_alignment
            && other.text_scaler == self.text_scaler
            && other.indicator_animation == self.indicator_animation
            && other.splash_border_radius == self.splash_border_radius
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/tab_bar_theme_test.dart`: the two cases that build no `TabBar` here,
    //! those that build one in `tab_bar_cases`; the `debugFillProperties` cases wait for
    //! diagnostics, `hashCode` for `Hash`, and the golden cases are not ported. The cases
    //! below the first two are not in Dart: they pin what Dart's `TabBarTheme`, `of` and `lerp`
    //! do, which its file checks only through a `TabBar`.

    use waft_painting::{BorderSide, BorderStyle, BoxDecoration, EdgeInsets};
    use waft_widgets::WidgetStateColor;

    use waft_widgets::testing::test_widgets;

    use super::*;
    use crate::colors::Colors;
    use crate::drawer_theme::tests::probe;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::ink_ripple::InkRipple;
    use crate::tab_indicator::UnderlineTabIndicator;
    use crate::theme_data::ThemeData;

    #[test]
    fn tab_bar_theme_data_copy_with_equality_and_defaults() {
        assert_eq!(TabBarThemeData::new(), TabBarThemeData::new().copy_with());

        let theme = TabBarThemeData::new();
        assert!(theme.indicator.is_none());
        assert_eq!(theme.indicator_color, None);
        assert_eq!(theme.indicator_size, None);
        assert_eq!(theme.divider_color, None);
        assert_eq!(theme.divider_height, None);
        assert_eq!(theme.label_color, None);
        assert_eq!(theme.label_padding, None);
        assert_eq!(theme.label_style, None);
        assert_eq!(theme.unselected_label_color, None);
        assert_eq!(theme.unselected_label_style, None);
        assert!(theme.overlay_color.is_none());
        assert!(theme.splash_factory.is_none());
        assert!(theme.mouse_cursor.is_none());
        assert_eq!(theme.tab_alignment, None);
        assert_eq!(theme.text_scaler, None);
        assert_eq!(theme.indicator_animation, None);
    }

    #[test]
    fn tab_bar_theme_data_lerp_of_a_theme_with_itself_is_that_theme() {
        let theme = TabBarThemeData::new().divider_height(3.0);
        assert_eq!(TabBarThemeData::lerp(&theme, &theme, 0.5), theme);
    }

    #[test]
    fn tab_bar_theme_data_copies_compare_by_their_fields() {
        let theme = TabBarThemeData::new()
            .indicator_color(Colors::RED.primary())
            .indicator_size(TabBarIndicatorSize::Label)
            .divider_height(2.0)
            .label_color(Colors::BLACK)
            .label_padding(EdgeInsets::all(4.0))
            .tab_alignment(TabAlignment::Center)
            .indicator_animation(TabIndicatorAnimation::Elastic)
            .splash_border_radius(BorderRadius::circular(8.0))
            .splash_factory(InkRipple::splash_factory());

        assert_eq!(theme.copy_with(), theme);
        assert_eq!(
            theme.copy_with().divider_height(5.0).divider_height,
            Some(5.0)
        );
        assert_eq!(
            theme.copy_with().divider_height(5.0).indicator_color,
            theme.indicator_color
        );
        assert_ne!(theme.copy_with().divider_height(5.0), theme);
        assert_ne!(theme.copy_with().tab_alignment(TabAlignment::Fill), theme);
        // Dart compares a `resolveWith` property and a splash factory by identity.
        let overlay = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|_| None);
        let with_overlay = theme.copy_with().overlay_color(overlay.clone());
        assert_eq!(with_overlay.copy_with(), with_overlay);
        assert_ne!(
            with_overlay,
            theme.copy_with().overlay_color(
                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|_| None)
            )
        );
        // A `BoxDecoration` compares by its fields, as Dart's `==` does.
        let decorated = theme
            .copy_with()
            .indicator(BoxDecoration::new().color(Colors::BLUE.primary()));
        assert_eq!(decorated.copy_with(), decorated);
        // An `UnderlineTabIndicator` does not override `==`: a copy shares the one indicator
        // and is equal, and another indicator of the same fields is not.
        let underlined = theme.copy_with().indicator(UnderlineTabIndicator::new());
        assert_eq!(underlined.copy_with(), underlined);
        assert_eq!(underlined.clone(), underlined);
        assert_ne!(
            theme.copy_with().indicator(UnderlineTabIndicator::new()),
            underlined
        );
    }

    #[test]
    fn tab_bar_theme_data_lerps_each_field() {
        let a = TabBarThemeData::new()
            .indicator_color(Colors::BLACK)
            .indicator_size(TabBarIndicatorSize::Tab)
            .divider_color(Colors::BLACK)
            .divider_height(1.0)
            .label_color(Colors::BLACK)
            .label_padding(EdgeInsets::all(10.0))
            .unselected_label_color(Colors::BLACK)
            .tab_alignment(TabAlignment::Start)
            .indicator_animation(TabIndicatorAnimation::Linear)
            .splash_border_radius(BorderRadius::circular(4.0));
        let b = TabBarThemeData::new()
            .indicator_color(Colors::WHITE)
            .indicator_size(TabBarIndicatorSize::Label)
            .divider_color(Colors::WHITE)
            .divider_height(3.0)
            .label_color(Colors::WHITE)
            .label_padding(EdgeInsets::all(30.0))
            .unselected_label_color(Colors::WHITE)
            .tab_alignment(TabAlignment::Center)
            .indicator_animation(TabIndicatorAnimation::Elastic)
            .splash_border_radius(BorderRadius::circular(12.0));

        let mid_gray = Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.25);
        let before = TabBarThemeData::lerp(&a, &b, 0.25);
        assert_eq!(before.indicator_color, mid_gray);
        assert_eq!(before.indicator_size, Some(TabBarIndicatorSize::Tab));
        assert_eq!(before.divider_color, mid_gray);
        assert_eq!(before.divider_height, Some(1.0));
        assert_eq!(before.label_color, mid_gray.map(AnyColor::new));
        assert_eq!(before.label_padding, Some(EdgeInsets::all(15.0).into()));
        assert_eq!(before.unselected_label_color, mid_gray);
        assert_eq!(before.tab_alignment, Some(TabAlignment::Start));
        assert_eq!(
            before.indicator_animation,
            Some(TabIndicatorAnimation::Linear)
        );

        let after = TabBarThemeData::lerp(&a, &b, 0.75);
        assert_eq!(after.indicator_size, Some(TabBarIndicatorSize::Label));
        assert_eq!(after.divider_height, Some(3.0));
        assert_eq!(after.tab_alignment, Some(TabAlignment::Center));
        assert_eq!(
            after.indicator_animation,
            Some(TabIndicatorAnimation::Elastic)
        );
        // Dart lerps the first theme's splash border radius with itself.
        assert_eq!(after.splash_border_radius, a.splash_border_radius);
    }

    #[test]
    fn tab_bar_theme_data_lerp_drops_a_widget_state_label_color() {
        let a = TabBarThemeData::new()
            .label_color(WidgetStateColor::resolve_with(|_| Colors::RED.primary()));
        let b = TabBarThemeData::new().label_color(Colors::RED.primary());
        let lerped = TabBarThemeData::lerp(&a, &b, 0.5)
            .label_color
            .expect("both themes have a label color");
        assert!(!lerped.has_extension());
    }

    #[test]
    fn tab_bar_theme_reads_its_properties_or_its_data() {
        let indicator = UnderlineTabIndicator::new().border_side(BorderSide::new(
            Colors::RED.primary(),
            3.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        ));
        let by_properties = TabBarTheme::new()
            .with_indicator(indicator.clone())
            .with_indicator_color(Colors::RED.primary())
            .with_indicator_size(TabBarIndicatorSize::Label)
            .with_divider_color(Colors::BLUE.primary())
            .with_divider_height(2.0)
            .with_label_color(Colors::BLACK)
            .with_label_padding(EdgeInsets::all(4.0))
            .with_unselected_label_color(Colors::WHITE)
            .with_tab_alignment(TabAlignment::Center)
            .with_text_scaler(TextScaler::linear(2.0))
            .with_indicator_animation(TabIndicatorAnimation::Elastic);
        let data = by_properties.data();
        let by_data = TabBarTheme::new().with_data(data.copy_with());

        assert!(data.indicator.is_some());
        assert_eq!(data.indicator_color, Some(Colors::RED.primary()));
        assert_eq!(data.indicator_size, Some(TabBarIndicatorSize::Label));
        assert_eq!(data.divider_color, Some(Colors::BLUE.primary()));
        assert_eq!(data.divider_height, Some(2.0));
        assert_eq!(data.label_color, Some(AnyColor::new(Colors::BLACK)));
        assert_eq!(data.label_padding, Some(EdgeInsets::all(4.0).into()));
        assert_eq!(data.unselected_label_color, Some(Colors::WHITE));
        assert_eq!(data.tab_alignment, Some(TabAlignment::Center));
        assert_eq!(data.text_scaler, Some(TextScaler::linear(2.0)));
        assert_eq!(
            data.indicator_animation,
            Some(TabIndicatorAnimation::Elastic)
        );
        // `TabBarTheme` has no splash border radius of its own.
        assert_eq!(data.splash_border_radius, None);

        assert_eq!(by_data.divider_height(), by_properties.divider_height());
        assert_eq!(by_data.tab_alignment(), by_properties.tab_alignment());
        assert_eq!(by_data.label_color(), by_properties.label_color());
        assert_eq!(by_data.text_scaler(), by_properties.text_scaler());
        assert!(by_data.indicator().is_some());
        assert_eq!(
            by_data
                .copy_with()
                .with_divider_height(5.0)
                .divider_height(),
            Some(5.0)
        );
        assert_eq!(
            by_data.copy_with().with_divider_height(5.0).tab_alignment(),
            Some(TabAlignment::Center)
        );
    }

    #[test]
    fn tab_bar_theme_lerps_through_its_getters() {
        let a = TabBarTheme::new().with_divider_color(Colors::BLACK);
        let b = TabBarTheme::new().with_data(TabBarThemeData::new().divider_color(Colors::WHITE));

        assert_eq!(
            TabBarTheme::lerp(&a, &b, 0.5).divider_color(),
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            TabBarTheme::lerp(&a, &a, 0.5).divider_color(),
            Some(Colors::BLACK)
        );
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |height: f64| {
            TabBarTheme::new().with_data(TabBarThemeData::new().divider_height(height))
        };
        let old = theme(1.0);

        assert!(!theme(1.0).update_should_notify(&old));
        assert!(theme(2.0).update_should_notify(&old));
    }

    #[test]
    fn of_returns_theme_data_tab_bar_theme_without_a_tab_bar_theme() {
        test_widgets(|tester| {
            let expected = TabBarThemeData::new().divider_height(7.0);
            let theme_data_tab_bar = expected.clone();

            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .tab_bar_theme(theme_data_tab_bar.clone())
                        .build(app);
                    crate::theme::Theme::new(data, child).into_widget()
                },
                TabBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_tab_bar_theme_over_theme_data() {
        test_widgets(|tester| {
            let local = TabBarThemeData::new().divider_height(2.0);
            let expected = local.clone();

            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .tab_bar_theme(TabBarThemeData::new().divider_height(7.0))
                        .build(app);
                    crate::theme::Theme::new(
                        data,
                        TabBarTheme::new().with_data(local.clone()).child(child),
                    )
                    .into_widget()
                },
                TabBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_tab_bar_theme_is_kept_under_another_tab_bar_theme() {
        test_widgets(|tester| {
            let seen = theme_seen_through_capture(
                tester,
                |probe| {
                    TabBarTheme::new()
                        .with_data(TabBarThemeData::new().divider_height(1.0))
                        .child(probe)
                        .into_widget()
                },
                |wrapped| {
                    TabBarTheme::new()
                        .with_data(TabBarThemeData::new().divider_height(2.0))
                        .child(wrapped)
                        .into_widget()
                },
                TabBarTheme::of,
            );

            assert_eq!(seen.divider_height, Some(1.0));
        });
    }

    #[test]
    fn theme_data_holds_the_tab_bar_theme() {
        test_widgets(|tester| {
            let app = tester.cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.tab_bar_theme, TabBarThemeData::new());

            let thick = ThemeData::builder()
                .tab_bar_theme(TabBarThemeData::new().divider_height(4.0))
                .build(&app);
            assert_ne!(theme, thick);
            assert_eq!(
                theme
                    .copy_with()
                    .tab_bar_theme(thick.tab_bar_theme.clone())
                    .build(),
                thick
            );

            let thin = ThemeData::builder()
                .tab_bar_theme(TabBarThemeData::new().divider_height(2.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&thin, &thick, 0.75)
                    .tab_bar_theme
                    .divider_height,
                Some(4.0)
            );
        });
    }

    // ---- the cases that build a `TabBar` ----

    mod tab_bar_cases {
        //! tab_bar_theme_test.dart's cases that build a `TabBar`; the four golden cases are
        //! not ported.
        use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

        use std::panic::{AssertUnwindSafe, catch_unwind};

        use waft_embedder::{
            Color, FontStyle, FontWeight, Offset, RRect, Radius, Rect, Size, TextDirection,
        };

        use waft_foundation::Ref;
        use waft_painting::{
            AlignmentGeometry, AnyColor, EdgeInsets, EdgeInsetsGeometry, TextScaler, TextStyle,
        };
        use waft_rendering::RenderBox as _;
        use waft_services::SystemMouseCursors;
        use waft_widgets::{
            AnyElement, Container, DefaultWidgetsLocalizations, GlobalKey, Icon, IconTheme,
            IntoWidget, Localizations, MediaQuery, MediaQueryData, PreferredSizeWidgetRef,
            RepaintBoundary, SizedBox, WidgetRef, WidgetState, WidgetStateColor,
            WidgetStateMouseCursor, WidgetStateProperty,
        };

        use super::super::{TabBarTheme, TabBarThemeData};
        use crate::app_bar::AppBar;
        use crate::colors::Colors;
        use crate::constants::K_TAB_LABEL_PADDING;
        use crate::icons::Icons;
        use crate::material::Material;
        use crate::material_localizations::DefaultMaterialLocalizations;
        use crate::scaffold::Scaffold;
        use crate::tab_controller::{DefaultTabController, TabController};
        use crate::tabs::tests::{
            close, first_render_box, is_line, only, painted_lines, rect_of, size_of, text,
        };
        use crate::tabs::{Tab, TabAlignment, TabBar, TabBarIndicatorSize, TabIndicatorAnimation};
        use crate::test_support::{PaintInspection, material_app};
        use crate::theme_data::ThemeData;
        use waft_widgets::testing::TestVSync;

        const TAB_1_TEXT: &str = "tab 1";

        /// A recorded paint's colour as `0xAARRGGBB`.
        fn argb32(color: waft_embedder::valo::Color) -> u32 {
            let channel = |value: f32| (value * 255.0).round() as u32;
            (channel(color.a) << 24)
                | (channel(color.r) << 16)
                | (channel(color.g) << 8)
                | channel(color.b)
        }
        const TAB_2_TEXT: &str = "tab 2";

        /// Dart's `_tabs`.
        fn tabs() -> Vec<WidgetRef> {
            vec![
                Tab::new()
                    .text(TAB_1_TEXT)
                    .icon(Icon::new(Some(Icons::looks_one())))
                    .into_widget(),
                Tab::new()
                    .text(TAB_2_TEXT)
                    .icon(Icon::new(Some(Icons::looks_two())))
                    .into_widget(),
                Tab::new()
                    .text("tab 3")
                    .icon(Icon::new(Some(Icons::looks_3())))
                    .into_widget(),
            ]
        }

        /// Dart's `_sizedTabs`, with their keys.
        fn sized_tabs() -> (Vec<WidgetRef>, [GlobalKey; 2]) {
            let keys = [GlobalKey::new(), GlobalKey::new()];
            let tabs = keys
                .iter()
                .map(|key| {
                    SizedBox::new()
                        .key(key.clone().into())
                        .width(100.0)
                        .height(50.0)
                        .into_widget()
                })
                .collect();
            (tabs, keys)
        }

        fn controller(tester: &WidgetTester, length: usize) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(&mut app, 0, None, length, TestVSync)
        }

        /// Dart's `buildTabBar` arguments; its `useMaterial3` defaults to false, as `bool`'s.
        #[derive(Default)]
        struct Build {
            local_tab_bar_theme: Option<TabBarThemeData>,
            tab_bar_theme: Option<TabBarThemeData>,
            secondary_tab_bar: bool,
            tabs: Option<Vec<WidgetRef>>,
            is_scrollable: bool,
            use_material3: bool,
            font_family: Option<&'static str>,
            label_style: Option<TextStyle>,
            unselected_label_style: Option<TextStyle>,
        }

        impl Build {
            /// Dart's `buildTabBar(..)`.
            fn build(self, tester: &WidgetTester) -> WidgetRef {
                let tabs = self.tabs.unwrap_or_else(tabs);
                let controller = controller(tester, tabs.len());
                let mut tab_bar = if self.secondary_tab_bar {
                    TabBar::secondary(tabs)
                } else {
                    TabBar::new(tabs)
                }
                .is_scrollable(self.is_scrollable)
                .controller(controller);
                if let Some(label_style) = self.label_style {
                    tab_bar = tab_bar.label_style(label_style);
                }
                if let Some(unselected_label_style) = self.unselected_label_style {
                    tab_bar = tab_bar.unselected_label_style(unselected_label_style);
                }
                let tab_bar = match self.local_tab_bar_theme {
                    Some(local) => TabBarTheme::new()
                        .with_data(local)
                        .child(tab_bar)
                        .into_widget(),
                    None => tab_bar.into_widget(),
                };
                let theme = {
                    let app = tester.cell.borrow();
                    let mut builder = ThemeData::builder().use_material3(self.use_material3);
                    if let Some(tab_bar_theme) = self.tab_bar_theme {
                        builder = builder.tab_bar_theme(tab_bar_theme);
                    }
                    if let Some(font_family) = self.font_family {
                        builder = builder.font_family(font_family);
                    }
                    builder.build(&app)
                };
                material_app(
                    theme,
                    Scaffold::new()
                        .body(RepaintBoundary::new().child(tab_bar))
                        .into_widget(),
                )
            }
        }

        /// Dart's `ThemeData(tabBarTheme:, useMaterial3:)`.
        fn theme_with(
            tester: &WidgetTester,
            tab_bar_theme: Option<TabBarThemeData>,
            use_material3: bool,
        ) -> ThemeData {
            let app = tester.cell.borrow();
            let mut builder = ThemeData::builder().use_material3(use_material3);
            if let Some(tab_bar_theme) = tab_bar_theme {
                builder = builder.tab_bar_theme(tab_bar_theme);
            }
            builder.build(&app)
        }

        /// Dart's `_getText(tester, text).text.style!`.
        fn text_style(tester: &WidgetTester, value: &str) -> TextStyle {
            let render = tester.render_box_of(text(tester, value));
            let app = tester.cell.borrow();
            let paragraph = render
                .as_object()
                .downcast::<waft_rendering::RenderParagraph>(&app)
                .expect("a Text renders a paragraph");
            paragraph.text(&app).style().cloned().expect("a style")
        }

        fn color_of(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(AnyColor::color)
        }

        /// Dart's `_getIcon(tester, icon).text.style!.color`.
        fn icon_color(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Option<Color> {
            let element = tester.find_by_icon(icon);
            color_of(&tester.glyph_style(element))
        }

        /// Dart's `IconTheme.of(tester.element(find.text(text))).color`.
        fn icon_theme_color(tester: &WidgetTester, value: &str) -> Option<Color> {
            let element = text(tester, value);
            let mut app = tester.cell.borrow_mut();
            IconTheme::of(&mut app, element)
                .color
                .as_ref()
                .map(AnyColor::color)
        }

        fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
            tester.get_rect(tester.render_object(key).as_box().expect("a box"))
        }

        fn tab_rects(tester: &WidgetTester) -> (Rect, Rect) {
            let tabs: Vec<AnyElement> = tester.find_by_type::<Tab>();
            (
                rect_of(tester, tabs[0]),
                rect_of(tester, *tabs.last().expect("a tab")),
            )
        }

        fn label_padding() -> EdgeInsets {
            EdgeInsetsGeometry::from(K_TAB_LABEL_PADDING).resolve(Some(TextDirection::Ltr))
        }

        /// The render object of the innermost `CustomPaint` under the tab bar with a painter:
        /// the indicator's.
        ///
        /// Dart's `paints` matcher on the tab bar records through the layers a scrollable tab
        /// bar's viewport pushes; `WidgetTester::pictures` reads the tab bar's own pictures
        /// only, so the cases ask the indicator's `CustomPaint` for what it paints there.
        fn indicator_paint(tester: &WidgetTester) -> waft_rendering::AnyRenderObject {
            let tab_bar = only::<TabBar>(tester);
            let paint = tester
                .descendants_of_type::<waft_widgets::CustomPaint>(tab_bar)
                .into_iter()
                .rfind(|element| {
                    tester.widget(*element, |paint: &waft_widgets::CustomPaint| {
                        paint.painter.is_some()
                    })
                })
                .expect("the indicator's CustomPaint");
            tester.render_box_of(paint).as_object()
        }

        /// Whether the tab bar paints a line in `color`, `stroke_width` wide when given, and
        /// its indicator then an rrect in `then_rrect_color` or a line in `then_line_color`:
        /// Dart's `paints..line(..)..rrect(..)` / `..line(..)`.
        fn paints_line_then(
            tester: &WidgetTester,
            color: Color,
            stroke_width: Option<f64>,
            then_rrect_color: Option<Color>,
            then_line_color: Option<Color>,
        ) -> bool {
            use waft_embedder::valo::{Op, PaintStyle};
            let render = first_render_box::<TabBar>(tester).as_object();
            let ops = tester.painted_ops(render);
            let is_line = |op: &Op, expected: Color, width: Option<f64>| match op {
                Op::DrawPath { path, paint, .. } => match &paint.style {
                    PaintStyle::Stroke(stroke) => {
                        path.elements().count() == 2
                            && argb32(paint.color) == expected.to_argb32()
                            && width.is_none_or(|width| close(f64::from(stroke.width), width, 1e-4))
                    }
                    _ => false,
                },
                _ => false,
            };
            let Some(line) = ops.iter().position(|op| is_line(op, color, stroke_width)) else {
                return false;
            };
            let is_then = |op: &Op| {
                then_line_color.is_some_and(|expected| is_line(op, expected, None))
                    || then_rrect_color.is_some_and(|expected| match op {
                        Op::DrawPath { path, paint, .. } => {
                            !matches!(paint.style, PaintStyle::Stroke(_))
                                && path.as_round_rect().is_some()
                                && argb32(paint.color) == expected.to_argb32()
                        }
                        _ => false,
                    })
            };
            // The indicator paints after the divider: later in the tab bar's own pictures, or
            // in the scroll view's layer under the divider's.
            ops[line + 1..].iter().any(is_then)
                || tester
                    .painted_ops(indicator_paint(tester))
                    .iter()
                    .any(is_then)
        }

        /// Whether the tab bar paints a filled rrect in `color` (Dart's `paints..rrect(color:)`).
        fn paints_rrect_color(tester: &WidgetTester, color: Color) -> bool {
            use waft_embedder::valo::{Op, PaintStyle};
            let render = first_render_box::<TabBar>(tester).as_object();
            tester.painted_ops(render).iter().any(|op| match op {
                Op::DrawPath { path, paint, .. } => {
                    !matches!(paint.style, PaintStyle::Stroke(_))
                        && path.as_round_rect().is_some()
                        && argb32(paint.color) == color.to_argb32()
                }
                _ => false,
            })
        }

        // 'Local TabBarTheme overrides defaults'.
        #[test]
        fn a_local_tab_bar_theme_overrides_the_defaults() {
            test_widgets(|tester| {
                let indicator_color = Colors::GREEN.primary();
                const DIVIDER_COLOR: Color = Color::new(0xff000001);
                const DIVIDER_HEIGHT: f64 = 20.5;
                const LABEL_COLOR: Color = Color::new(0xff000002);
                const UNSELECTED_LABEL_COLOR: Color = Color::new(0xff654321);

                let tab_bar_theme = TabBarThemeData::new()
                    .indicator_color(indicator_color)
                    .divider_color(DIVIDER_COLOR)
                    .divider_height(DIVIDER_HEIGHT)
                    .label_color(LABEL_COLOR)
                    .label_style(TextStyle::new().font_size(32.0))
                    .unselected_label_color(UNSELECTED_LABEL_COLOR)
                    .unselected_label_style(TextStyle::new().font_weight(FontWeight::BOLD));

                // Test default label color and label styles.
                let widget = Build {
                    use_material3: true,
                    local_tab_bar_theme: Some(tab_bar_theme),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                let selected_label = text_style(tester, TAB_1_TEXT);
                assert_eq!(color_of(&selected_label), Some(LABEL_COLOR));
                assert_eq!(selected_label.font_size, Some(32.0));
                let unselected_label = text_style(tester, TAB_2_TEXT);
                assert_eq!(color_of(&unselected_label), Some(UNSELECTED_LABEL_COLOR));
                assert_eq!(unselected_label.font_weight, Some(FontWeight::BOLD));

                assert!(paints_line_then(
                    tester,
                    DIVIDER_COLOR,
                    Some(DIVIDER_HEIGHT),
                    Some(indicator_color),
                    None
                ));
            });
        }

        /// The default label padding checks of 'Tab bar defaults' (Material 3).
        fn check_default_label_padding(
            tester: &WidgetTester,
            keys: &[GlobalKey; 2],
            tab_start_offset: f64,
        ) {
            const INDICATOR_WEIGHT: f64 = 2.0;
            let padding = label_padding();
            let tab_bar = rect_of(tester, only::<TabBar>(tester));
            let tab_one_rect = key_rect(tester, &keys[0]);
            let tab_two_rect = key_rect(tester, &keys[1]);

            // Verify tabOne coordinates.
            assert_eq!(tab_one_rect.left, padding.left + tab_start_offset);
            assert_eq!(tab_one_rect.top, padding.top);
            assert_eq!(
                tab_one_rect.bottom,
                tab_bar.bottom - padding.bottom - INDICATOR_WEIGHT
            );

            // Verify tabTwo coordinates.
            if tab_start_offset > 0.0 {
                let tab_two_right = tab_start_offset
                    + padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert_eq!(tab_two_rect.right, tab_two_right);
            } else {
                assert_eq!(tab_two_rect.right, tab_bar.width() - padding.right);
            }
            assert_eq!(tab_two_rect.top, padding.top);
            assert_eq!(
                tab_two_rect.bottom,
                tab_bar.bottom - padding.bottom - INDICATOR_WEIGHT
            );

            // Verify tabOne and tabTwo are separated by right padding of tabOne and left padding of tabTwo.
            assert_eq!(
                tab_one_rect.right,
                tab_two_rect.left - padding.left - padding.right
            );
        }

        fn material3_defaults(secondary: bool) {
            test_widgets(|tester| {
                // Test default label color and label styles.
                let widget = Build {
                    secondary_tab_bar: secondary,
                    use_material3: true,
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                let theme = theme_with(tester, None, true);
                let title_small = theme.text_theme.title_small.clone().expect("titleSmall");
                let selected_label = text_style(tester, TAB_1_TEXT);
                assert_eq!(selected_label.font_family, title_small.font_family);
                assert_eq!(selected_label.font_size, Some(14.0));
                let selected_color = if secondary {
                    theme.color_scheme.on_surface
                } else {
                    theme.color_scheme.primary
                };
                assert_eq!(color_of(&selected_label), Some(selected_color));
                let unselected_label = text_style(tester, TAB_2_TEXT);
                assert_eq!(unselected_label.font_family, title_small.font_family);
                assert_eq!(unselected_label.font_size, Some(14.0));
                assert_eq!(
                    color_of(&unselected_label),
                    Some(theme.color_scheme.on_surface_variant())
                );

                // Test default labelPadding. Dart's primary case builds this one without
                // `useMaterial3`; the `MaterialApp`'s theme animation keeps Material 3 in the
                // frame the case measures.
                let (tabs, keys) = sized_tabs();
                let widget = Build {
                    secondary_tab_bar: secondary,
                    tabs: Some(tabs),
                    is_scrollable: true,
                    use_material3: secondary,
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                check_default_label_padding(tester, &keys, 52.0);

                // Test default indicator & divider color.
                let (then_rrect, then_line) = if secondary {
                    (None, Some(theme.color_scheme.primary))
                } else {
                    (Some(theme.color_scheme.primary), None)
                };
                assert!(paints_line_then(
                    tester,
                    theme.color_scheme.outline_variant(),
                    Some(1.0),
                    then_rrect,
                    then_line
                ));
            });
        }

        // 'Tab bar defaults (primary)'.
        #[test]
        fn material3_primary_tab_bar_defaults() {
            material3_defaults(false);
        }

        // 'Tab bar defaults (secondary)'.
        #[test]
        fn material3_secondary_tab_bar_defaults() {
            material3_defaults(true);
        }

        // 'Tab bar theme overrides label color (selected)'.
        #[test]
        fn the_theme_label_color_colors_the_selected_label_and_icon() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(TabBarThemeData::new().label_color(Colors::BLACK)),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                assert_eq!(
                    color_of(&text_style(tester, TAB_1_TEXT)),
                    Some(Colors::BLACK)
                );
                assert_eq!(icon_color(tester, &Icons::looks_one()), Some(Colors::BLACK));
            });
        }

        // 'Tab bar theme overrides label padding'.
        #[test]
        fn the_theme_label_padding_insets_the_tabs() {
            test_widgets(|tester| {
                const TOP_PADDING: f64 = 10.0;
                const BOTTOM_PADDING: f64 = 7.0;
                const RIGHT_PADDING: f64 = 13.0;
                const LEFT_PADDING: f64 = 16.0;
                const INDICATOR_WEIGHT: f64 = 2.0; // default value

                let (tabs, keys) = sized_tabs();
                let widget = Build {
                    tab_bar_theme: Some(TabBarThemeData::new().label_padding(
                        EdgeInsets::from_ltrb(
                            LEFT_PADDING,
                            TOP_PADDING,
                            RIGHT_PADDING,
                            BOTTOM_PADDING,
                        ),
                    )),
                    tabs: Some(tabs),
                    is_scrollable: true,
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                let tab_bar = rect_of(tester, only::<TabBar>(tester));
                let tab_one_rect = key_rect(tester, &keys[0]);
                let tab_two_rect = key_rect(tester, &keys[1]);

                // verify coordinates of tabOne
                assert_eq!(tab_one_rect.left, LEFT_PADDING);
                assert_eq!(tab_one_rect.top, TOP_PADDING);
                assert_eq!(
                    tab_one_rect.bottom,
                    tab_bar.bottom - BOTTOM_PADDING - INDICATOR_WEIGHT
                );

                // verify coordinates of tabTwo
                assert_eq!(tab_two_rect.right, tab_bar.width() - RIGHT_PADDING);
                assert_eq!(tab_two_rect.top, TOP_PADDING);
                assert_eq!(
                    tab_two_rect.bottom,
                    tab_bar.bottom - BOTTOM_PADDING - INDICATOR_WEIGHT
                );

                // verify tabOne and tabTwo are separated by right padding of tabOne and left padding of tabTwo
                assert_eq!(
                    tab_one_rect.right,
                    tab_two_rect.left - LEFT_PADDING - RIGHT_PADDING
                );
            });
        }

        // 'Tab bar theme overrides label styles'.
        #[test]
        fn the_theme_label_styles_style_the_labels() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new()
                            .label_style(TextStyle::new().font_family("foobar"))
                            .unselected_label_style(TextStyle::new().font_family("baz")),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                assert_eq!(
                    text_style(tester, TAB_1_TEXT).font_family.as_deref(),
                    Some("foobar")
                );
                assert_eq!(
                    text_style(tester, TAB_2_TEXT).font_family.as_deref(),
                    Some("baz")
                );
            });
        }

        // 'Tab bar theme with just label style specified'.
        #[test]
        fn a_theme_with_only_a_label_style_leaves_the_unselected_label_to_the_defaults() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new().label_style(TextStyle::new().font_family("foobar")),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                assert_eq!(
                    text_style(tester, TAB_1_TEXT).font_family.as_deref(),
                    Some("foobar")
                );
                let unselected_label = text_style(tester, TAB_2_TEXT);
                assert_eq!(unselected_label.font_family.as_deref(), Some("Roboto"));
                assert_eq!(unselected_label.font_size, Some(14.0));
                assert_eq!(
                    color_of(&unselected_label),
                    Some(Colors::WHITE.with_alpha(0xB2))
                );
            });
        }

        // 'Tab bar label styles override theme label styles'.
        #[test]
        fn the_tab_bar_label_styles_override_the_theme_label_styles() {
            test_widgets(|tester| {
                let theme = theme_with(
                    tester,
                    Some(
                        TabBarThemeData::new()
                            .label_style(TextStyle::new().font_family("3"))
                            .unselected_label_style(TextStyle::new().font_family("4")),
                    ),
                    true,
                );
                let controller = controller(tester, 3);
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            TabBar::new(tabs())
                                .controller(controller)
                                .label_style(TextStyle::new().font_family("1"))
                                .unselected_label_style(TextStyle::new().font_family("2")),
                        )
                        .into_widget(),
                ));

                assert_eq!(
                    text_style(tester, TAB_1_TEXT).font_family.as_deref(),
                    Some("1")
                );
                assert_eq!(
                    text_style(tester, TAB_2_TEXT).font_family.as_deref(),
                    Some("2")
                );
            });
        }

        fn label_padding_overrides_theme(use_material3: bool) {
            test_widgets(|tester| {
                const TAB_START_OFFSET: f64 = 52.0;
                const VERTICAL_PADDING: f64 = 10.0;
                const HORIZONTAL_PADDING: f64 = 10.0;
                const HORIZONTAL_THEME_PADDING: f64 = 20.0;
                const INDICATOR_WEIGHT: f64 = 2.0; // default value

                let theme = theme_with(
                    tester,
                    Some(
                        TabBarThemeData::new()
                            .label_padding(EdgeInsetsGeometry::symmetric(20.0, 20.0)),
                    ),
                    use_material3,
                );
                let (tabs, keys) = sized_tabs();
                let controller = controller(tester, tabs.len());
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            RepaintBoundary::new().child(
                                TabBar::new(tabs)
                                    .is_scrollable(true)
                                    .controller(controller)
                                    .label_padding(EdgeInsetsGeometry::symmetric(
                                        VERTICAL_PADDING,
                                        HORIZONTAL_PADDING,
                                    )),
                            ),
                        )
                        .into_widget(),
                ));

                let tab_bar = rect_of(tester, only::<TabBar>(tester));
                let tab_one_rect = key_rect(tester, &keys[0]);
                let tab_two_rect = key_rect(tester, &keys[1]);

                // verify coordinates of tabOne
                let start = if use_material3 { TAB_START_OFFSET } else { 0.0 };
                assert_eq!(tab_one_rect.left, HORIZONTAL_PADDING + start);
                assert_eq!(tab_one_rect.top, VERTICAL_PADDING);
                assert_eq!(
                    tab_one_rect.bottom,
                    tab_bar.bottom - VERTICAL_PADDING - INDICATOR_WEIGHT
                );

                // verify coordinates of tabTwo
                if use_material3 {
                    assert_eq!(
                        tab_two_rect.right,
                        TAB_START_OFFSET
                            + HORIZONTAL_THEME_PADDING
                            + tab_one_rect.width()
                            + tab_two_rect.width()
                            + (HORIZONTAL_THEME_PADDING / 2.0)
                    );
                } else {
                    assert_eq!(tab_two_rect.right, tab_bar.width() - HORIZONTAL_PADDING);
                }
                assert_eq!(tab_two_rect.top, VERTICAL_PADDING);
                assert_eq!(
                    tab_two_rect.bottom,
                    tab_bar.bottom - VERTICAL_PADDING - INDICATOR_WEIGHT
                );

                // verify tabOne and tabTwo are separated by 2x horizontalPadding
                assert_eq!(
                    tab_one_rect.right,
                    tab_two_rect.left - (2.0 * HORIZONTAL_PADDING)
                );
            });
        }

        // 'Material2 - Tab bar label padding overrides theme label padding'.
        #[test]
        fn material2_the_tab_bar_label_padding_overrides_the_themes() {
            label_padding_overrides_theme(false);
        }

        // 'Material3 - Tab bar label padding overrides theme label padding'.
        #[test]
        fn material3_the_tab_bar_label_padding_overrides_the_themes() {
            label_padding_overrides_theme(true);
        }

        // 'Tab bar theme overrides label color (unselected)'.
        #[test]
        fn the_theme_unselected_label_color_colors_the_unselected_label_and_icon() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new().unselected_label_color(Colors::BLACK),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                assert_eq!(
                    color_of(&text_style(tester, TAB_2_TEXT)),
                    Some(Colors::BLACK)
                );
                assert_eq!(icon_color(tester, &Icons::looks_two()), Some(Colors::BLACK));
            });
        }

        // 'Tab bar theme overrides tab mouse cursor'.
        #[test]
        fn the_theme_mouse_cursor_is_the_tabs_cursor() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new().mouse_cursor(<dyn WidgetStateProperty<
                            Option<waft_services::MouseCursorRef>,
                        >>::resolve_with(
                            |states| Some(WidgetStateMouseCursor::textable().resolve(states)),
                        )),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                let tab_bar = tester.ancestor_of_type::<TabBar>(text(tester, TAB_1_TEXT));
                let center = rect_of(tester, tab_bar).center();
                let mut gesture = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();
                assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
            });
        }

        /// Whether pumping the `Build` that `make` gives panics, on a fresh tester: Dart's
        /// `tester.takeException()` after `pumpWidget`.
        fn pump_panics(make: impl FnOnce() -> Build + Send) -> bool {
            let mut panicked = false;
            test_widgets(|tester| {
                let widget = make().build(tester);
                panicked = catch_unwind(AssertUnwindSafe(|| tester.pump_widget(widget))).is_err();
            });
            panicked
        }

        // 'TabAlignment.fill from TabBarTheme only supports non-scrollable tab bar'.
        #[test]
        fn a_theme_tab_alignment_fill_is_only_for_fixed_tab_bars() {
            fn theme() -> Option<TabBarThemeData> {
                Some(TabBarThemeData::new().tab_alignment(TabAlignment::Fill))
            }

            // Test TabAlignment.fill from TabBarTheme with non-scrollable tab bar.
            assert!(!pump_panics(|| Build {
                tab_bar_theme: theme(),
                ..Build::default()
            }));

            // Test TabAlignment.fill from TabBarTheme with scrollable tab bar.
            assert!(pump_panics(|| Build {
                tab_bar_theme: theme(),
                is_scrollable: true,
                ..Build::default()
            }));
        }

        // 'TabAlignment.start & TabAlignment.startOffset from TabBarTheme only supports scrollable tab bar'.
        #[test]
        fn a_theme_tab_alignment_start_or_start_offset_is_only_for_scrollable_tab_bars() {
            for tab_alignment in [TabAlignment::Start, TabAlignment::StartOffset] {
                let theme = move || Some(TabBarThemeData::new().tab_alignment(tab_alignment));

                // With a scrollable tab bar.
                assert!(!pump_panics(move || Build {
                    tab_bar_theme: theme(),
                    is_scrollable: true,
                    ..Build::default()
                }));

                // With a non-scrollable tab bar.
                assert!(pump_panics(move || Build {
                    tab_bar_theme: theme(),
                    ..Build::default()
                }));
            }
        }

        /// Dart's `MaterialApp(theme:, home: Scaffold(body: Container(alignment: topLeft, child:
        /// tab_bar)))` with four tabs.
        fn four_tabs_top_left(
            tester: &WidgetTester,
            theme: ThemeData,
            secondary: bool,
        ) -> WidgetRef {
            let tabs = (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
            let controller = controller(tester, 4);
            let tab_bar = if secondary {
                TabBar::secondary(tabs)
            } else {
                TabBar::new(tabs)
            }
            .controller(controller);
            material_app(
                theme,
                Scaffold::new()
                    .body(
                        Container::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(tab_bar),
                    )
                    .into_widget(),
            )
        }

        fn indicator_size_provides_the_indicator(
            use_material3: bool,
            secondary: bool,
            indicator_size: TabBarIndicatorSize,
            p1_x: f64,
            p2_x: f64,
        ) {
            test_widgets(|tester| {
                let theme = theme_with(
                    tester,
                    Some(TabBarThemeData::new().indicator_size(indicator_size)),
                    use_material3,
                );
                let widget = four_tabs_top_left(tester, theme.clone(), secondary);
                tester.pump_widget(widget);

                let tab_bar_box = first_render_box::<TabBar>(tester);
                assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 48.0);

                const INDICATOR_WEIGHT: f64 = 2.0;
                const INDICATOR_Y: f64 = 48.0 - (INDICATOR_WEIGHT / 2.0);

                let lines = painted_lines(tester, tab_bar_box.as_object());
                #[allow(deprecated)]
                let indicator_color = if use_material3 {
                    theme.color_scheme.primary
                } else {
                    theme.indicator_color
                };
                let indicator = lines.iter().position(|line| {
                    is_line(
                        line,
                        indicator_color,
                        Some(INDICATOR_WEIGHT),
                        Offset::new(p1_x, INDICATOR_Y),
                        Offset::new(p2_x, INDICATOR_Y),
                    )
                });
                assert!(indicator.is_some(), "{lines:?}");
                if use_material3 {
                    // Divider.
                    assert!(lines[..indicator.expect("checked")].iter().any(|line| {
                        line.color.to_argb32() == theme.color_scheme.outline_variant().to_argb32()
                            && close(line.stroke_width, 1.0, 1e-4)
                    }));
                }
            });
        }

        // 'TabBarTheme.indicatorSize provides correct tab indicator (primary)'.
        #[test]
        fn material3_the_theme_indicator_size_tab_gives_a_tab_wide_indicator() {
            indicator_size_provides_the_indicator(
                true,
                false,
                TabBarIndicatorSize::Tab,
                1.0,
                199.0,
            );
        }

        // 'TabBarTheme.indicatorSize provides correct tab indicator (secondary)'.
        #[test]
        fn material3_the_theme_indicator_size_label_gives_a_label_wide_secondary_indicator() {
            indicator_size_provides_the_indicator(
                true,
                true,
                TabBarIndicatorSize::Label,
                65.75,
                134.25,
            );
        }

        /// Dart's `MaterialApp(theme:, home: Scaffold(appBar: AppBar(bottom: tab_bar)))`.
        fn app_bar_tab_bar(theme: ThemeData, tab_bar: TabBar) -> WidgetRef {
            material_app(
                theme,
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(
                        AppBar::new().bottom(PreferredSizeWidgetRef::new(tab_bar)),
                    ))
                    .into_widget(),
            )
        }

        fn three_tabs() -> Vec<WidgetRef> {
            ["Tab 1", "Tab 2", "Tab 3"]
                .map(|label| Tab::new().text(label).into_widget())
                .into()
        }

        // 'TabBar divider can use TabBarTheme.dividerColor & TabBarTheme.dividerHeight'.
        #[test]
        fn the_divider_takes_the_theme_divider_color_and_height() {
            test_widgets(|tester| {
                const DIVIDER_COLOR: Color = Color::new(0xff00ff00);
                const DIVIDER_HEIGHT: f64 = 10.0;
                let theme = theme_with(
                    tester,
                    Some(
                        TabBarThemeData::new()
                            .divider_color(DIVIDER_COLOR)
                            .divider_height(DIVIDER_HEIGHT),
                    ),
                    true,
                );
                let controller = controller(tester, 3);
                tester.pump_widget(app_bar_tab_bar(
                    theme,
                    TabBar::new(three_tabs()).controller(controller),
                ));

                // Test divider color.
                let lines = painted_lines(tester, first_render_box::<TabBar>(tester).as_object());
                assert!(
                    lines
                        .iter()
                        .any(|line| line.color.to_argb32() == DIVIDER_COLOR.to_argb32()
                            && close(line.stroke_width, DIVIDER_HEIGHT, 1e-4))
                );
            });
        }

        // 'dividerColor & dividerHeight overrides TabBarTheme.dividerColor'.
        #[test]
        fn the_tab_bar_divider_color_and_height_override_the_themes() {
            test_widgets(|tester| {
                const DIVIDER_COLOR: Color = Color::new(0xff0000ff);
                const DIVIDER_HEIGHT: f64 = 8.0;
                let theme = theme_with(
                    tester,
                    Some(
                        TabBarThemeData::new()
                            .divider_color(Colors::PINK.primary())
                            .divider_height(5.0),
                    ),
                    true,
                );
                let controller = controller(tester, 3);
                tester.pump_widget(app_bar_tab_bar(
                    theme,
                    TabBar::new(three_tabs())
                        .divider_color(DIVIDER_COLOR)
                        .divider_height(DIVIDER_HEIGHT)
                        .controller(controller),
                ));

                // Test divider color.
                let lines = painted_lines(tester, first_render_box::<TabBar>(tester).as_object());
                assert!(
                    lines
                        .iter()
                        .any(|line| line.color.to_argb32() == DIVIDER_COLOR.to_argb32()
                            && close(line.stroke_width, DIVIDER_HEIGHT, 1e-4))
                );
            });
        }

        fn two_tabs() -> Vec<WidgetRef> {
            ["Tab 1", "Tab 3"]
                .map(|label| Tab::new().text(label).into_widget())
                .into()
        }

        /// The checks of a centered fixed tab bar 800 wide.
        fn check_centered(tester: &WidgetTester) {
            const AVAILABLE_WIDTH: f64 = 800.0;
            let padding = label_padding();
            let (tab_one_rect, tab_two_rect) = tab_rects(tester);
            let tab_one_left = (AVAILABLE_WIDTH / 2.0) - tab_one_rect.width() - padding.left;
            assert_eq!(tab_one_rect.left, tab_one_left);
            let tab_two_right = (AVAILABLE_WIDTH / 2.0) + tab_two_rect.width() + padding.right;
            assert_eq!(tab_two_rect.right, tab_two_right);
        }

        /// The checks of a scrollable tab bar aligned to its start.
        fn check_start(tester: &WidgetTester) {
            let padding = label_padding();
            let (tab_one_rect, tab_two_rect) = tab_rects(tester);
            assert_eq!(tab_one_rect.left, padding.left);
            let tab_two_right =
                padding.horizontal() + tab_one_rect.width() + padding.left + tab_two_rect.width();
            assert_eq!(tab_two_rect.right, tab_two_right);
        }

        fn respects_theme_tab_alignment(use_material3: bool) {
            test_widgets(|tester| {
                // Test non-scrollable tab bar.
                let theme = theme_with(
                    tester,
                    Some(TabBarThemeData::new().tab_alignment(TabAlignment::Center)),
                    use_material3,
                );
                let controller1 = controller(tester, 2);
                tester.pump_widget(app_bar_tab_bar(
                    theme,
                    TabBar::new(two_tabs()).controller(controller1),
                ));
                check_centered(tester);

                if use_material3 {
                    // Test scrollable tab bar.
                    let theme = theme_with(
                        tester,
                        Some(TabBarThemeData::new().tab_alignment(TabAlignment::Start)),
                        true,
                    );
                    let controller2 = controller(tester, 2);
                    tester.pump_widget(app_bar_tab_bar(
                        theme,
                        TabBar::new(two_tabs())
                            .is_scrollable(true)
                            .controller(controller2),
                    ));
                    tester.pump_and_settle();
                    check_start(tester);
                }
            });
        }

        // 'TabBar respects TabBarTheme.tabAlignment'.
        #[test]
        fn material3_the_tab_bar_takes_the_theme_tab_alignment() {
            respects_theme_tab_alignment(true);
        }

        // 'TabBar respects TabBarTheme.tabAlignment' (Material 2).
        #[test]
        fn material2_the_tab_bar_takes_the_theme_tab_alignment() {
            respects_theme_tab_alignment(false);
        }

        fn tab_alignment_overrides_theme(use_material3: bool) {
            test_widgets(|tester| {
                // Test non-scrollable tab bar.
                let theme = theme_with(
                    tester,
                    Some(TabBarThemeData::new().tab_alignment(TabAlignment::Fill)),
                    use_material3,
                );
                let controller1 = controller(tester, 2);
                tester.pump_widget(app_bar_tab_bar(
                    theme,
                    TabBar::new(two_tabs())
                        .tab_alignment(TabAlignment::Center)
                        .controller(controller1),
                ));
                check_centered(tester);

                if use_material3 {
                    // Test scrollable tab bar.
                    let theme = theme_with(
                        tester,
                        Some(TabBarThemeData::new().tab_alignment(TabAlignment::Center)),
                        true,
                    );
                    let controller2 = controller(tester, 2);
                    tester.pump_widget(app_bar_tab_bar(
                        theme,
                        TabBar::new(two_tabs())
                            .is_scrollable(true)
                            .tab_alignment(TabAlignment::Start)
                            .controller(controller2),
                    ));
                    tester.pump_and_settle();
                    check_start(tester);
                }
            });
        }

        // 'TabBar.tabAlignment overrides TabBarTheme.tabAlignment'.
        #[test]
        fn material3_the_tab_bar_tab_alignment_overrides_the_themes() {
            tab_alignment_overrides_theme(true);
        }

        // 'TabBar.tabAlignment overrides TabBarTheme.tabAlignment' (Material 2).
        #[test]
        fn material2_the_tab_bar_tab_alignment_overrides_the_themes() {
            tab_alignment_overrides_theme(false);
        }

        /// The selected and unselected tabs' icon colors, text colors and font styles.
        fn label_colors(
            tester: &WidgetTester,
        ) -> [(Option<Color>, Option<Color>, Option<FontStyle>); 2] {
            [TAB_1_TEXT, TAB_2_TEXT].map(|label| {
                let style = text_style(tester, label);
                (
                    icon_theme_color(tester, label),
                    color_of(&style),
                    style.font_style,
                )
            })
        }

        const STYLE_COLOR: Color = Color::new(0xff0000ff);
        const UNSELECTED_STYLE_COLOR: Color = Color::new(0x950000ff);

        fn label_style() -> TextStyle {
            TextStyle::new()
                .color(STYLE_COLOR)
                .font_style(FontStyle::Italic)
        }

        fn unselected_label_style() -> TextStyle {
            TextStyle::new()
                .color(UNSELECTED_STYLE_COLOR)
                .font_style(FontStyle::Italic)
        }

        fn styled_by(
            color: Color,
            unselected_color: Color,
        ) -> [(Option<Color>, Option<Color>, Option<FontStyle>); 2] {
            [
                (Some(color), Some(color), Some(FontStyle::Italic)),
                (
                    Some(unselected_color),
                    Some(unselected_color),
                    Some(FontStyle::Italic),
                ),
            ]
        }

        // 'TabBar labels use colors from TabBarTheme.labelStyle & TabBarTheme.unselectedLabelStyle'.
        #[test]
        fn the_labels_take_their_colors_from_the_theme_label_styles() {
            test_widgets(|tester| {
                // Test tab bar with TabBarTheme labelStyle & unselectedLabelStyle.
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new()
                            .label_style(label_style())
                            .unselected_label_style(unselected_label_style()),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                // Selected tab should use labelStyle color; unselected unselectedLabelStyle.
                assert_eq!(
                    label_colors(tester),
                    styled_by(STYLE_COLOR, UNSELECTED_STYLE_COLOR)
                );
            });
        }

        const LABEL_COLOR: Color = Color::new(0xfff00000);
        const UNSELECTED_LABEL_COLOR: Color = Color::new(0x95ff0000);

        // "TabBarTheme's labelColor & unselectedLabelColor override labelStyle & unselectedLabelStyle colors".
        #[test]
        fn the_theme_label_colors_override_the_theme_label_style_colors() {
            test_widgets(|tester| {
                let styles = || {
                    TabBarThemeData::new()
                        .label_style(label_style())
                        .unselected_label_style(unselected_label_style())
                };
                let widget = Build {
                    tab_bar_theme: Some(styles()),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                // Test tab bar with TabBarTheme labelStyle & unselectedLabelStyle. Dart pumps a
                // bar without the theme here and still expects the theme's colors, which its
                // first pump's controller and tabs keep; this pumps the themed bar again.
                let widget = Build {
                    tab_bar_theme: Some(styles()),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(
                    label_colors(tester),
                    styled_by(STYLE_COLOR, UNSELECTED_STYLE_COLOR)
                );

                // Update the TabBarTheme with labelColor & unselectedLabelColor.
                let widget = Build {
                    tab_bar_theme: Some(
                        styles()
                            .label_color(LABEL_COLOR)
                            .unselected_label_color(UNSELECTED_LABEL_COLOR),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(
                    label_colors(tester),
                    styled_by(LABEL_COLOR, UNSELECTED_LABEL_COLOR)
                );
            });
        }

        // "TabBarTheme's labelColor & unselectedLabelColor override TabBar.labelStyle & TabBar.unselectedLabelStyle colors".
        #[test]
        fn the_theme_label_colors_override_the_tab_bar_label_style_colors() {
            test_widgets(|tester| {
                let build = |tester: &WidgetTester, tab_bar_theme: Option<TabBarThemeData>| {
                    let theme = theme_with(tester, tab_bar_theme, true);
                    material_app(
                        theme,
                        Material::new()
                            .child(DefaultTabController::new(
                                2,
                                TabBar::new(
                                    [TAB_1_TEXT, TAB_2_TEXT]
                                        .map(|label| Tab::new().text(label).into_widget()),
                                )
                                .label_style(label_style())
                                .unselected_label_style(unselected_label_style()),
                            ))
                            .into_widget(),
                    )
                };

                // Test tab bar with [TabBar.labelStyle] & [TabBar.unselectedLabelStyle].
                let widget = build(tester, None);
                tester.pump_widget(widget);
                assert_eq!(
                    label_colors(tester),
                    styled_by(STYLE_COLOR, UNSELECTED_STYLE_COLOR)
                );

                // Add TabBarTheme with labelColor & unselectedLabelColor.
                let widget = build(
                    tester,
                    Some(
                        TabBarThemeData::new()
                            .label_color(LABEL_COLOR)
                            .unselected_label_color(UNSELECTED_LABEL_COLOR),
                    ),
                );
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(
                    label_colors(tester),
                    styled_by(LABEL_COLOR, UNSELECTED_LABEL_COLOR)
                );
            });
        }

        fn material2_defaults(secondary: bool) {
            test_widgets(|tester| {
                // Test default label color and label styles.
                let widget = Build {
                    secondary_tab_bar: secondary,
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);

                let theme = theme_with(tester, None, false);
                let selected_label = text_style(tester, TAB_1_TEXT);
                assert_eq!(selected_label.font_family.as_deref(), Some("Roboto"));
                assert_eq!(selected_label.font_size, Some(14.0));
                assert_eq!(color_of(&selected_label), Some(Colors::WHITE));
                let unselected_label = text_style(tester, TAB_2_TEXT);
                assert_eq!(unselected_label.font_family.as_deref(), Some("Roboto"));
                assert_eq!(unselected_label.font_size, Some(14.0));
                assert_eq!(
                    color_of(&unselected_label),
                    Some(Colors::WHITE.with_alpha(0xB2))
                );

                // Test default labelPadding.
                let (tabs, keys) = sized_tabs();
                let widget = Build {
                    tabs: Some(tabs),
                    is_scrollable: true,
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                check_default_label_padding(tester, &keys, 0.0);

                // Test default indicator color.
                #[allow(deprecated)]
                let indicator_color = theme.indicator_color;
                let lines = painted_lines(tester, indicator_paint(tester));
                assert!(
                    lines
                        .iter()
                        .any(|line| line.color.to_argb32() == indicator_color.to_argb32())
                );
            });
        }

        // 'Tab bar defaults (primary)' (Material 2).
        #[test]
        fn material2_primary_tab_bar_defaults() {
            material2_defaults(false);
        }

        // 'Tab bar defaults (secondary)' (Material 2).
        #[test]
        fn material2_secondary_tab_bar_defaults() {
            material2_defaults(true);
        }

        // 'TabBarTheme.indicatorSize provides correct tab indicator (primary)' (Material 2).
        #[test]
        fn material2_the_theme_indicator_size_tab_gives_a_tab_wide_indicator() {
            indicator_size_provides_the_indicator(
                false,
                false,
                TabBarIndicatorSize::Tab,
                1.0,
                199.0,
            );
        }

        // 'TabBarTheme.indicatorSize provides correct tab indicator (secondary)' (Material 2).
        #[test]
        fn material2_the_theme_indicator_size_label_gives_a_label_wide_secondary_indicator() {
            indicator_size_provides_the_indicator(
                false,
                true,
                TabBarIndicatorSize::Label,
                66.0,
                134.0,
            );
        }

        // 'Material3 - TabBar indicator respects TabBarTheme.indicatorColor'.
        #[test]
        fn material3_the_indicator_takes_the_theme_indicator_color() {
            test_widgets(|tester| {
                const TAB_BAR_THEME_INDICATOR_COLOR: Color = Color::new(0xffff0000);
                let controller = controller(tester, 4);
                let build = |theme: ThemeData| {
                    let tabs =
                        (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
                    material_app(
                        theme,
                        Material::new()
                            .child(
                                Container::new()
                                    .alignment(AlignmentGeometry::TOP_LEFT)
                                    .child(TabBar::new(tabs).controller(controller)),
                            )
                            .into_widget(),
                    )
                };

                let theme = theme_with(tester, None, true);
                tester.pump_widget(build.clone()(theme.clone()));
                assert!(paints_rrect_color(tester, theme.color_scheme.primary));

                let theme = theme_with(
                    tester,
                    Some(TabBarThemeData::new().indicator_color(TAB_BAR_THEME_INDICATOR_COLOR)),
                    true,
                );
                tester.pump_widget(build(theme));
                tester.pump_and_settle();
                assert!(paints_rrect_color(tester, TAB_BAR_THEME_INDICATOR_COLOR));
            });
        }

        // 'Material2 - TabBar indicator respects TabBarTheme.indicatorColor'.
        #[test]
        fn material2_the_indicator_takes_the_theme_indicator_color() {
            test_widgets(|tester| {
                const TAB_BAR_THEME_INDICATOR_COLOR: Color = Color::new(0xffffff00);
                let controller = controller(tester, 4);
                let theme = theme_with(
                    tester,
                    Some(TabBarThemeData::new().indicator_color(TAB_BAR_THEME_INDICATOR_COLOR)),
                    false,
                );
                let tabs =
                    (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(
                            Container::new()
                                .alignment(AlignmentGeometry::TOP_LEFT)
                                .child(TabBar::new(tabs).controller(controller)),
                        )
                        .into_widget(),
                ));

                let lines = painted_lines(tester, first_render_box::<TabBar>(tester).as_object());
                assert!(lines.iter().any(
                    |line| line.color.to_argb32() == TAB_BAR_THEME_INDICATOR_COLOR.to_argb32()
                ));
            });
        }

        const SELECTED_STATE_COLOR: Color = Color::new(0xff00ff00);
        const UNSELECTED_STATE_COLOR: Color = Color::new(0xffff0000);

        fn state_label_color() -> WidgetStateColor {
            WidgetStateColor::resolve_with(|states| {
                if states.contains(&WidgetState::Selected) {
                    return SELECTED_STATE_COLOR;
                }
                UNSELECTED_STATE_COLOR
            })
        }

        /// The selected and unselected tabs' icon and text colors.
        fn state_colors(tester: &WidgetTester) -> [Option<Color>; 4] {
            [
                icon_theme_color(tester, TAB_1_TEXT),
                icon_theme_color(tester, TAB_2_TEXT),
                color_of(&text_style(tester, TAB_1_TEXT)),
                color_of(&text_style(tester, TAB_2_TEXT)),
            ]
        }

        fn colored(selected: Color, unselected: Color) -> [Option<Color>; 4] {
            [
                Some(selected),
                Some(unselected),
                Some(selected),
                Some(unselected),
            ]
        }

        // 'TabBarTheme.labelColor resolves material states'.
        #[test]
        fn a_theme_widget_state_label_color_resolves_per_selection() {
            test_widgets(|tester| {
                let widget = Build {
                    tab_bar_theme: Some(TabBarThemeData::new().label_color(state_label_color())),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(
                    state_colors(tester),
                    colored(SELECTED_STATE_COLOR, UNSELECTED_STATE_COLOR)
                );
            });
        }

        // 'TabBarTheme.labelColor & TabBarTheme.unselectedLabelColor override material state TabBarTheme.labelColor'.
        #[test]
        fn plain_theme_label_colors_replace_a_widget_state_label_color() {
            test_widgets(|tester| {
                const SELECTED_COLOR: Color = Color::new(0xff00ffff);
                const UNSELECTED_COLOR: Color = Color::new(0xffff12ff);

                // Test material state label color.
                let widget = Build {
                    tab_bar_theme: Some(TabBarThemeData::new().label_color(state_label_color())),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(
                    state_colors(tester),
                    colored(SELECTED_STATE_COLOR, UNSELECTED_STATE_COLOR)
                );

                // Test labelColor & unselectedLabelColor override material state labelColor.
                let widget = Build {
                    tab_bar_theme: Some(
                        TabBarThemeData::new()
                            .label_color(SELECTED_COLOR)
                            .unselected_label_color(UNSELECTED_COLOR),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(
                    state_colors(tester),
                    colored(SELECTED_COLOR, UNSELECTED_COLOR)
                );
            });
        }

        // 'TabBarTheme.textScaler overrides tab label text scale, textScaleFactor = noScaling, 1.75, 2.0'.
        #[test]
        fn the_theme_text_scaler_overrides_the_ambient_one() {
            test_widgets(|tester| {
                let build = |tester: &WidgetTester, text_scaler: TextScaler| {
                    let theme = theme_with(
                        tester,
                        Some(TabBarThemeData::new().text_scaler(text_scaler)),
                        true,
                    );
                    material_app(
                        theme,
                        MediaQuery::new(
                            MediaQueryData::new().text_scaler(TextScaler::linear(3.0)),
                            DefaultTabController::new(
                                2,
                                Scaffold::new().app_bar(PreferredSizeWidgetRef::new(
                                    AppBar::new().bottom(PreferredSizeWidgetRef::new(TabBar::new(
                                        ["Tab 1", "Tab 2"]
                                            .map(|label| Tab::new().text(label).into_widget()),
                                    ))),
                                )),
                            ),
                        )
                        .into_widget(),
                    )
                };

                let widget = build(tester, TextScaler::NO_SCALING);
                tester.pump_widget(widget);
                assert_eq!(
                    size_of(tester, text(tester, "Tab 1")),
                    Size::new(70.5, 20.0)
                );

                let widget = build(tester, TextScaler::linear(1.75));
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(
                    size_of(tester, text(tester, "Tab 1")),
                    Size::new(123.0, 35.0)
                );

                let widget = build(tester, TextScaler::linear(2.0));
                tester.pump_widget(widget);
                tester.pump_and_settle();
                assert_eq!(
                    size_of(tester, text(tester, "Tab 1")),
                    Size::new(140.5, 40.0)
                );
            });
        }

        /// Whether the indicator's `CustomPaint` paints `rrect`.
        fn indicator_paints_rrect(tester: &WidgetTester, rrect: RRect) -> bool {
            waft_rendering::testing::paints(
                &tester.painted_ops(first_render_box::<TabBar>(tester).as_object()),
                &[waft_rendering::testing::PaintCall::rrect_at(rrect, None)],
            )
            .is_ok()
        }

        fn indicator_rrect(left: f64, right: f64, bottom: f64) -> RRect {
            RRect::from_ltrb_and_corners(
                left,
                bottom - 3.0,
                right,
                bottom,
                Radius::circular(3.0),
                Radius::circular(3.0),
                Radius::ZERO,
                Radius::ZERO,
            )
        }

        // 'TabBarTheme indicatorAnimation can customize tab indicator animation'.
        #[test]
        fn the_theme_indicator_animation_moves_the_indicator_linearly_or_elastically() {
            test_widgets(|tester| {
                const INDICATOR_WIDTH: f64 = 50.0;
                let controller = controller(tester, 4);
                let build = |tester: &WidgetTester, indicator_animation: TabIndicatorAnimation| {
                    let theme = theme_with(
                        tester,
                        Some(TabBarThemeData::new().indicator_animation(indicator_animation)),
                        true,
                    );
                    let tabs = (0..4).map(|index| {
                        Tab::new()
                            .key(waft_foundation::ValueKey::new(index).into())
                            .child(SizedBox::new().width(INDICATOR_WIDTH))
                            .into_widget()
                    });
                    material_app(
                        theme,
                        Material::new()
                            .child(
                                Container::new()
                                    .alignment(AlignmentGeometry::TOP_LEFT)
                                    .child(TabBar::new(tabs).controller(controller.clone())),
                            )
                            .into_widget(),
                    )
                };

                // Test tab indicator animation with TabIndicatorAnimation.linear.
                let widget = build(tester, TabIndicatorAnimation::Linear);
                tester.pump_widget(widget);

                // Idle at tab 0.
                assert!(indicator_paints_rrect(
                    tester,
                    indicator_rrect(75.0, 125.0, 48.0)
                ));

                // Start moving tab indicator.
                controller.set_offset(&mut tester.cell.borrow_mut(), 0.2);
                tester.pump();
                assert!(indicator_paints_rrect(
                    tester,
                    indicator_rrect(115.0, 165.0, 48.0)
                ));

                // Reset tab controller offset.
                controller.set_offset(&mut tester.cell.borrow_mut(), 0.0);

                // Test tab indicator animation with TabIndicatorAnimation.elastic.
                let widget = build(tester, TabIndicatorAnimation::Elastic);
                tester.pump_widget(widget);
                tester.pump_and_settle();

                let height = first_render_box::<TabBar>(tester)
                    .size(&tester.cell.borrow())
                    .height();
                let elastic = |from: Rect, to: Rect, progress: f64| {
                    let left_fraction = 1.0 - ((progress * std::f64::consts::PI) / 2.0).cos();
                    let right_fraction = ((progress * std::f64::consts::PI) / 2.0).sin();
                    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
                    indicator_rrect(
                        lerp(from.left, to.left, left_fraction),
                        lerp(from.right, to.right, right_fraction),
                        height,
                    )
                };

                // Idle at tab 0.
                let from_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                assert!(indicator_paints_rrect(
                    tester,
                    elastic(from_rect, from_rect, 0.0)
                ));

                // Start moving tab indicator.
                controller.set_offset(&mut tester.cell.borrow_mut(), 0.2);
                tester.pump();
                let to_rect = Rect::from_ltrb(275.0, 0.0, 325.0, 48.0);
                assert!(indicator_paints_rrect(
                    tester,
                    elastic(from_rect, to_rect, 0.2)
                ));
            });
        }

        // 'Tab bar inherits fontFamily from theme when labelStyle and unselectedLabelStyle are specified'.
        #[test]
        fn the_labels_inherit_the_theme_font_family_unless_their_styles_do_not_inherit() {
            test_widgets(|tester| {
                const FONT_FAMILY: &str = "TestFont";
                let families = |tester: &WidgetTester| {
                    [TAB_1_TEXT, TAB_2_TEXT].map(|label| text_style(tester, label).font_family)
                };
                let some = || [Some(FONT_FAMILY.to_owned()), Some(FONT_FAMILY.to_owned())];

                let widget = Build {
                    font_family: Some(FONT_FAMILY),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(families(tester), some());

                let widget = Build {
                    font_family: Some(FONT_FAMILY),
                    local_tab_bar_theme: Some(
                        TabBarThemeData::new()
                            .label_style(TextStyle::new())
                            .unselected_label_style(TextStyle::new()),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(families(tester), some());

                let widget = Build {
                    font_family: Some(FONT_FAMILY),
                    label_style: Some(TextStyle::new()),
                    unselected_label_style: Some(TextStyle::new()),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(families(tester), some());

                // if inherit is false, the fontFamily should not be applied
                let widget = Build {
                    font_family: Some(FONT_FAMILY),
                    local_tab_bar_theme: Some(
                        TabBarThemeData::new()
                            .label_style(TextStyle::new().inherit(false))
                            .unselected_label_style(TextStyle::new().inherit(false)),
                    ),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(families(tester), [None, None]);

                let widget = Build {
                    font_family: Some(FONT_FAMILY),
                    label_style: Some(TextStyle::new().inherit(false)),
                    unselected_label_style: Some(TextStyle::new().inherit(false)),
                    ..Build::default()
                }
                .build(tester);
                tester.pump_widget(widget);
                assert_eq!(families(tester), [None, None]);
            });
        }

        // 'TabBar inherits splashBorderRadius from theme'.
        #[test]
        fn the_tab_bar_takes_the_theme_splash_border_radius() {
            test_widgets(|tester| {
                use crate::material::RenderInkFeatures;

                const HOVER_COLOR: Color = Color::new(0xfff44336);
                const RADIUS: f64 = 20.0;
                tester.pump_widget(
                    Material::new()
                        .child(
                            Localizations::new(
                                waft_embedder::Locale::new("en").country_code("US"),
                                vec![
                                    DefaultMaterialLocalizations::delegate(),
                                    DefaultWidgetsLocalizations::delegate(),
                                ],
                            )
                            .child(DefaultTabController::new(
                                1,
                                TabBarTheme::new()
                                    .with_data(TabBarThemeData::new().splash_border_radius(
                                        waft_painting::BorderRadius::all(Radius::circular(RADIUS)),
                                    ))
                                    .child(
                                        TabBar::new([Tab::new()
                                            .child(waft_widgets::Text::new(""))
                                            .into_widget()])
                                        .overlay_color(
                                            <dyn WidgetStateProperty<Option<Color>>>::from_map(
                                                vec![
                                                    (
                                                        WidgetState::Hovered.into(),
                                                        Some(HOVER_COLOR),
                                                    ),
                                                    (WidgetState::any(), Some(Colors::BLACK54)),
                                                ],
                                            ),
                                        ),
                                    ),
                            )),
                        )
                        .into_widget(),
                );
                let mut gesture = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                let center = rect_of(tester, only::<Tab>(tester)).center();
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();

                let ink_well = only::<crate::ink_well::InkWell>(tester);
                let ink_well_rect = rect_of(tester, ink_well);
                let material = {
                    let app = tester.cell.borrow();
                    Material::of(&app, ink_well)
                };
                let render = tester
                    .all::<RenderInkFeatures>()
                    .into_iter()
                    .find(|render| render.as_object().id() == material.id())
                    .expect("the material's render object");
                let mut canvas = waft_embedder::Canvas::new();
                render.paint_ink_features(&mut tester.cell.borrow_mut(), &mut canvas, Offset::ZERO);
                let expected = RRect::from_rect_and_radius(ink_well_rect, Radius::circular(RADIUS));
                let painted = waft_embedder::flattened_ops(&canvas.build())
                    .into_iter()
                    .any(|op| match op {
                        waft_embedder::valo::Op::DrawPath { path, paint, .. } => {
                            argb32(paint.color) == HOVER_COLOR.to_argb32()
                                && path.as_round_rect().is_some_and(|(rect, radii)| {
                                    close(f64::from(rect.x), expected.left, 1e-4)
                                        && close(f64::from(rect.y), expected.top, 1e-4)
                                        && close(
                                            f64::from(rect.x + rect.width),
                                            expected.right,
                                            1e-4,
                                        )
                                        && close(
                                            f64::from(rect.y + rect.height),
                                            expected.bottom,
                                            1e-4,
                                        )
                                        && radii == [[RADIUS as f32; 2]; 4]
                                })
                        }
                        _ => false,
                    });
                assert!(painted);
            });
        }
    }
}

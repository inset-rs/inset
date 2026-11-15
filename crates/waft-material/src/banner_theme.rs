//! Flutter counterpart: material_ui's `banner_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, TextStyle};
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Defines the visual properties of `MaterialBanner` widgets.
///
/// Descendant widgets obtain the current [`MaterialBannerThemeData`] object using
/// [`MaterialBannerTheme::of`]. Instances of [`MaterialBannerThemeData`]
/// can be customized with [`MaterialBannerThemeData::copy_with`].
///
/// Typically a [`MaterialBannerThemeData`] is specified as part of the overall
/// `Theme` with `ThemeData::banner_theme`.
///
/// All [`MaterialBannerThemeData`] properties are `null` by default. When null,
/// the `MaterialBanner` will provide its own defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialBannerThemeData {
    /// The background color of a `MaterialBanner`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `MaterialBanner::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of `MaterialBanner::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value of `MaterialBanner::divider_color`.
    pub divider_color: Option<Color>,

    /// Used to configure the `DefaultTextStyle` for the `MaterialBanner::content`
    /// widget.
    pub content_text_style: Option<TextStyle>,

    /// Default value for `MaterialBanner::elevation`.
    ///
    /// If null, MaterialBanner uses a default of 0.0.
    pub elevation: Option<f64>,

    /// The amount of space by which to inset `MaterialBanner::content`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The amount of space by which to inset `MaterialBanner::leading`.
    pub leading_padding: Option<EdgeInsetsGeometry>,
}

impl MaterialBannerThemeData {
    /// Creates a theme that can be used for [`MaterialBannerTheme`] or
    /// `ThemeData::banner_theme`; Dart's optional named arguments are the fluent setters.
    pub fn new() -> MaterialBannerThemeData {
        MaterialBannerThemeData::default()
    }

    /// Dart `MaterialBannerThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> MaterialBannerThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `MaterialBannerThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> MaterialBannerThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `MaterialBannerThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> MaterialBannerThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `MaterialBannerThemeData(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> MaterialBannerThemeData {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `MaterialBannerThemeData(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> MaterialBannerThemeData {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `MaterialBannerThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> MaterialBannerThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `MaterialBannerThemeData(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> MaterialBannerThemeData {
        self.padding = Some(padding);
        self
    }

    /// Dart `MaterialBannerThemeData(leadingPadding:)`.
    pub fn leading_padding(
        mut self,
        leading_padding: EdgeInsetsGeometry,
    ) -> MaterialBannerThemeData {
        self.leading_padding = Some(leading_padding);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> MaterialBannerThemeData {
        self.clone()
    }

    /// Linearly interpolate between two Banner themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&MaterialBannerThemeData>,
        b: Option<&MaterialBannerThemeData>,
        t: f64,
    ) -> MaterialBannerThemeData {
        MaterialBannerThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            divider_color: Color::lerp(
                a.and_then(|a| a.divider_color),
                b.and_then(|b| b.divider_color),
                t,
            ),
            content_text_style: TextStyle::lerp(
                a.and_then(|a| a.content_text_style.as_ref()),
                b.and_then(|b| b.content_text_style.as_ref()),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            leading_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.leading_padding),
                b.and_then(|b| b.leading_padding),
                t,
            ),
        }
    }
}

/// An inherited widget that defines the configuration for
/// `MaterialBanner`s in this widget's subtree.
///
/// Values specified here are used for `MaterialBanner` properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct MaterialBannerTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `MaterialBanner` widgets.
    pub data: Option<MaterialBannerThemeData>,
    pub child: WidgetRef,
}

impl MaterialBannerTheme {
    /// Creates a banner theme that controls the configurations for
    /// `MaterialBanner`s in its widget subtree.
    pub fn new<K>(child: impl IntoWidget<K>) -> MaterialBannerTheme {
        MaterialBannerTheme {
            key: None,
            data: None,
            child: child.into_widget(),
        }
    }

    /// Dart `MaterialBannerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> MaterialBannerTheme {
        self.key = Some(key);
        self
    }

    /// Dart `MaterialBannerTheme(data:)`.
    pub fn data(mut self, data: MaterialBannerThemeData) -> MaterialBannerTheme {
        self.data = Some(data);
        self
    }

    /// The closest instance of this class's [`data`](Self::data) value that encloses the given
    /// context.
    ///
    /// If there is no ancestor, it returns `ThemeData::banner_theme`. Applications
    /// can assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = MaterialBannerTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> MaterialBannerThemeData {
        context
            .depend_on_inherited_widget_of_exact_type::<MaterialBannerTheme>(app)
            .and_then(|banner_theme| banner_theme.data.clone())
            .unwrap_or_else(|| Theme::of(app, context).banner_theme)
    }
}

impl InheritedTheme for MaterialBannerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        let mut wrapped = MaterialBannerTheme::new(child);
        wrapped.data = self.data.clone();
        wrapped.into_widget()
    }
}

impl InheritedWidget for MaterialBannerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MaterialBannerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_widgets::SizedBox;

    use super::*;
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    #[test]
    fn material_banner_theme_data_copy_with_equality_basics() {
        assert_eq!(
            MaterialBannerThemeData::new(),
            MaterialBannerThemeData::new().copy_with()
        );

        let theme = MaterialBannerThemeData::new()
            .elevation(4.0)
            .padding(EdgeInsetsGeometry::all(20.0));
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(
            theme.copy_with().divider_color(Colors::RED.primary()),
            theme.clone().divider_color(Colors::RED.primary())
        );
        assert_eq!(
            theme
                .copy_with()
                .divider_color(Colors::RED.primary())
                .elevation,
            Some(4.0)
        );
        assert_ne!(theme.copy_with().elevation(5.0), theme);
    }

    #[test]
    fn material_banner_theme_data_null_fields_by_default() {
        let banner_theme = MaterialBannerThemeData::new();
        assert_eq!(banner_theme.background_color, None);
        assert_eq!(banner_theme.surface_tint_color, None);
        assert_eq!(banner_theme.shadow_color, None);
        assert_eq!(banner_theme.divider_color, None);
        assert_eq!(banner_theme.content_text_style, None);
        assert_eq!(banner_theme.elevation, None);
        assert_eq!(banner_theme.padding, None);
        assert_eq!(banner_theme.leading_padding, None);
    }

    #[test]
    fn material_banner_theme_data_lerp_of_nothing_is_an_empty_theme() {
        assert_eq!(
            MaterialBannerThemeData::lerp(None, None, 0.0),
            MaterialBannerThemeData::new()
        );
    }

    #[test]
    fn material_banner_theme_data_lerps_each_field() {
        let a = MaterialBannerThemeData::new()
            .background_color(Colors::BLACK)
            .content_text_style(TextStyle::new().font_size(10.0))
            .elevation(2.0)
            .padding(EdgeInsetsGeometry::all(10.0));
        let b = MaterialBannerThemeData::new()
            .background_color(Colors::WHITE)
            .shadow_color(Colors::RED.primary())
            .content_text_style(TextStyle::new().font_size(20.0))
            .elevation(6.0)
            .padding(EdgeInsetsGeometry::all(30.0))
            .leading_padding(EdgeInsetsGeometry::only(8.0, 0.0, 0.0, 0.0));

        let halfway = MaterialBannerThemeData::lerp(Some(&a), Some(&b), 0.5);

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.shadow_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(
            halfway.content_text_style.and_then(|style| style.font_size),
            Some(15.0)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.padding, Some(EdgeInsetsGeometry::all(20.0)));
        assert_eq!(
            halfway.leading_padding,
            EdgeInsetsGeometry::lerp(
                None,
                Some(EdgeInsetsGeometry::only(8.0, 0.0, 0.0, 0.0)),
                0.5
            )
        );
    }

    #[test]
    fn of_returns_the_theme_data_banner_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = MaterialBannerThemeData::new()
                .elevation(3.0)
                .divider_color(Colors::RED.primary());
            let theme_data_banner = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .banner_theme(theme_data_banner.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                MaterialBannerTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_banner_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_banner = MaterialBannerThemeData::new().elevation(7.0);
            let expected = widget_banner.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .banner_theme(MaterialBannerThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(
                        data,
                        MaterialBannerTheme::new(child).data(widget_banner.clone()),
                    )
                    .into_widget()
                },
                MaterialBannerTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_falls_back_to_theme_data_when_the_banner_theme_has_no_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = MaterialBannerThemeData::new().elevation(3.0);
            let theme_data_banner = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .banner_theme(theme_data_banner.clone())
                        .build(app);
                    Theme::new(data, MaterialBannerTheme::new(child)).into_widget()
                },
                MaterialBannerTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_banner_theme_is_kept_under_another_banner_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let captured = MaterialBannerThemeData::new().elevation(1.0);
            let shadowing = MaterialBannerThemeData::new().elevation(2.0);

            let seen = theme_seen_through_capture(
                &cell,
                |probe| {
                    MaterialBannerTheme::new(probe)
                        .data(captured.clone())
                        .into_widget()
                },
                |wrapped| {
                    MaterialBannerTheme::new(wrapped)
                        .data(shadowing.clone())
                        .into_widget()
                },
                MaterialBannerTheme::of,
            );

            assert_eq!(seen.elevation, Some(1.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: Option<MaterialBannerThemeData>| {
            let theme = MaterialBannerTheme::new(SizedBox::shrink());
            match data {
                Some(data) => theme.data(data),
                None => theme,
            }
        };
        let old = theme(Some(MaterialBannerThemeData::new().elevation(1.0)));

        assert!(
            !theme(Some(MaterialBannerThemeData::new().elevation(1.0))).update_should_notify(&old)
        );
        assert!(
            theme(Some(MaterialBannerThemeData::new().elevation(2.0))).update_should_notify(&old)
        );
        assert!(theme(None).update_should_notify(&old));
    }

    #[test]
    fn theme_data_holds_the_banner_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.banner_theme, MaterialBannerThemeData::new());

            let raised = ThemeData::builder()
                .banner_theme(MaterialBannerThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.banner_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .banner_theme(raised.banner_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .banner_theme(MaterialBannerThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5).banner_theme.elevation,
                Some(4.0)
            );
        });
    }
}

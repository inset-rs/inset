//! Flutter counterpart: material_ui's `card_theme.dart`.

use waft_embedder::{Clip, Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, ShapeBorder};
use waft_widgets::{BuildContext, InheritedWidget, IntoWidget, KeyRef, SizedBox, WidgetRef};

use crate::theme::Theme;

/// Defines default property values for descendant [`Card`](crate::Card) widgets.
///
/// Descendant widgets obtain the current [`CardThemeData`] object using
/// [`CardTheme::of`]. Instances of [`CardThemeData`] can be
/// customized with [`CardThemeData::copy_with`].
///
/// Typically a [`CardThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::card_theme`.
///
/// All [`CardThemeData`] properties are `null` by default. When null, the [`Card`](crate::Card)
/// will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults.
///
/// Dart's `CardTheme` constructor takes either the properties or a [`data`](Self::data);
/// here the properties are the `with_*` setters and [`with_data`](Self::with_data) the other,
/// and each setter that breaks Dart's constructor assert (a `data` together with any property)
/// fails in a debug build. The getters are methods, as Dart's, hence the `with_` prefix.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug)]
pub struct CardTheme {
    pub key: Option<KeyRef>,
    data: Option<CardThemeData>,
    clip_behavior: Option<Clip>,
    color: Option<Color>,
    surface_tint_color: Option<Color>,
    shadow_color: Option<Color>,
    elevation: Option<f64>,
    margin: Option<EdgeInsetsGeometry>,
    shape: Option<Box<dyn ShapeBorder>>,
    child: WidgetRef,
}

impl CardTheme {
    /// Creates a theme that can be used for `ThemeData::card_theme`.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative.
    pub fn new() -> CardTheme {
        CardTheme {
            key: None,
            data: None,
            clip_behavior: None,
            color: None,
            surface_tint_color: None,
            shadow_color: None,
            elevation: None,
            margin: None,
            shape: None,
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart `CardTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> CardTheme {
        self.key = Some(key);
        self
    }

    /// Dart `CardTheme(child:)`; the default is an empty `SizedBox`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CardTheme {
        self.child = child.into_widget();
        self
    }

    /// Dart `CardTheme(data:)`.
    pub fn with_data(mut self, data: CardThemeData) -> CardTheme {
        debug_assert!(self.has_no_property());
        self.data = Some(data);
        self
    }

    /// Dart `CardTheme(clipBehavior:)`.
    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `CardTheme(color:)`.
    pub fn with_color(mut self, color: Color) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.color = Some(color);
        self
    }

    /// Dart `CardTheme(surfaceTintColor:)`.
    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `CardTheme(shadowColor:)`.
    pub fn with_shadow_color(mut self, shadow_color: Color) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `CardTheme(elevation:)`.
    pub fn with_elevation(mut self, elevation: f64) -> CardTheme {
        debug_assert!(self.data.is_none());
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `CardTheme(margin:)`.
    pub fn with_margin(mut self, margin: EdgeInsetsGeometry) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.margin = Some(margin);
        self
    }

    /// Dart `CardTheme(shape:)`.
    pub fn with_shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> CardTheme {
        debug_assert!(self.data.is_none());
        self.shape = Some(shape.into());
        self
    }

    /// Overrides the default value for `Card::clip_behavior`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::clip_behavior`] property in [`data`](Self::data) instead.
    pub fn clip_behavior(&self) -> Option<Clip> {
        match &self.data {
            Some(data) => data.clip_behavior,
            None => self.clip_behavior,
        }
    }

    /// Overrides the default value for `Card::color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::color`] property in [`data`](Self::data) instead.
    pub fn color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.color,
            None => self.color,
        }
    }

    /// Overrides the default value for `Card::surface_tint_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::surface_tint_color`] property in [`data`](Self::data)
    /// instead.
    pub fn surface_tint_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.surface_tint_color,
            None => self.surface_tint_color,
        }
    }

    /// Overrides the default value for `Card::shadow_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::shadow_color`] property in [`data`](Self::data) instead.
    pub fn shadow_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.shadow_color,
            None => self.shadow_color,
        }
    }

    /// Overrides the default value for `Card::elevation`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::elevation`] property in [`data`](Self::data) instead.
    pub fn elevation(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.elevation,
            None => self.elevation,
        }
    }

    /// Overrides the default value for `Card::margin`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::margin`] property in [`data`](Self::data) instead.
    pub fn margin(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.margin,
            None => self.margin,
        }
    }

    /// Overrides the default value for `Card::shape`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::shape`] property in [`data`](Self::data) instead.
    pub fn shape(&self) -> Option<&dyn ShapeBorder> {
        match &self.data {
            Some(data) => data.shape.as_deref(),
            None => self.shape.as_deref(),
        }
    }

    /// The properties used for all descendant [`Card`](crate::Card) widgets.
    pub fn data(&self) -> CardThemeData {
        self.data.clone().unwrap_or_else(|| CardThemeData {
            clip_behavior: self.clip_behavior,
            color: self.color,
            shadow_color: self.shadow_color,
            surface_tint_color: self.surface_tint_color,
            elevation: self.elevation,
            margin: self.margin,
            shape: self.shape.clone(),
        })
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::copy_with`] instead.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`theme.copy_with().with_elevation(2.0)`). As in Dart, the copy carries neither the key
    /// nor the child, and holds the properties, never a [`data`](Self::data).
    pub fn copy_with(&self) -> CardTheme {
        CardTheme {
            key: None,
            data: None,
            clip_behavior: self.clip_behavior(),
            color: self.color(),
            surface_tint_color: self.surface_tint_color(),
            shadow_color: self.shadow_color(),
            elevation: self.elevation(),
            margin: self.margin(),
            shape: self.shape().map(|shape| shape.clone_box()),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`CardTheme`] ancestor.
    ///
    /// If there is no ancestor, it returns `ThemeData::card_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = CardTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> CardThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<CardTheme>(app) {
            Some(card_theme) => card_theme.data(),
            None => Theme::of(app, context).card_theme.clone(),
        }
    }

    /// Linearly interpolate between two Card themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`CardThemeData::lerp`] instead.
    pub fn lerp(a: Option<&CardTheme>, b: Option<&CardTheme>, t: f64) -> CardTheme {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        CardTheme {
            key: None,
            data: None,
            clip_behavior: if t < 0.5 {
                a.and_then(|a| a.clip_behavior())
            } else {
                b.and_then(|b| b.clip_behavior())
            },
            color: Color::lerp(a.and_then(|a| a.color()), b.and_then(|b| b.color()), t),
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
            elevation: lerp_double(
                a.and_then(|a| a.elevation()),
                b.and_then(|b| b.elevation()),
                t,
            ),
            margin: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.margin()),
                b.and_then(|b| b.margin()),
                t,
            ),
            shape: <dyn ShapeBorder>::lerp(a.and_then(|a| a.shape()), b.and_then(|b| b.shape()), t),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart's constructor assert: a `data` takes the place of every property.
    fn has_no_property(&self) -> bool {
        self.clip_behavior.is_none()
            && self.color.is_none()
            && self.surface_tint_color.is_none()
            && self.shadow_color.is_none()
            && self.elevation.is_none()
            && self.margin.is_none()
            && self.shape.is_none()
    }
}

impl Default for CardTheme {
    fn default() -> CardTheme {
        CardTheme::new()
    }
}

impl InheritedWidget for CardTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &CardTheme) -> bool {
        self.data() != old_widget.data()
    }
}

/// Defines default property values for descendant [`Card`](crate::Card) widgets.
///
/// Descendant widgets obtain the current [`CardThemeData`] object using
/// `CardTheme::of(app, context)`. Instances of [`CardThemeData`] can be
/// customized with [`CardThemeData::copy_with`].
///
/// Typically a [`CardThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::card_theme`.
///
/// All [`CardThemeData`] properties are `null` by default. When null, the [`Card`](crate::Card)
/// will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults. See the individual [`Card`](crate::Card) properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardThemeData {
    /// Overrides the default value for `Card::clip_behavior`.
    pub clip_behavior: Option<Clip>,

    /// Overrides the default value for `Card::color`.
    pub color: Option<Color>,

    /// Overrides the default value for `Card::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value for `Card::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value for `Card::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value for `Card::margin`.
    pub margin: Option<EdgeInsetsGeometry>,

    /// Overrides the default value for `Card::shape`.
    pub shape: Option<Box<dyn ShapeBorder>>,
}

impl CardThemeData {
    /// Creates a theme that can be used for `ThemeData::card_theme`; Dart's optional named
    /// arguments are the fluent setters.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative.
    pub fn new() -> CardThemeData {
        CardThemeData::default()
    }

    /// Dart `CardThemeData(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CardThemeData {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `CardThemeData(color:)`.
    pub fn color(mut self, color: Color) -> CardThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `CardThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> CardThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `CardThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> CardThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `CardThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> CardThemeData {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `CardThemeData(margin:)`.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> CardThemeData {
        self.margin = Some(margin);
        self
    }

    /// Dart `CardThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> CardThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> CardThemeData {
        self.clone()
    }

    /// Linearly interpolate between two Card themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: Option<&CardThemeData>, b: Option<&CardThemeData>, t: f64) -> CardThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        CardThemeData {
            clip_behavior: if t < 0.5 {
                a.and_then(|a| a.clip_behavior)
            } else {
                b.and_then(|b| b.clip_behavior)
            },
            color: Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
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
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            margin: EdgeInsetsGeometry::lerp(a.and_then(|a| a.margin), b.and_then(|b| b.margin), t),
            shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/card_theme_test.dart`; the `debugFillProperties` cases wait for
    //! diagnostics, `hashCode` for `Hash`, and the goldens are not ported.

    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};

    use super::*;
    use crate::card::Card;
    use crate::card::tests::{card_material, card_padding, rounded};
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::scaffold::Scaffold;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};

    /// Dart's `_cardTheme()`.
    fn card_theme() -> CardThemeData {
        CardThemeData::new()
            .clip_behavior(Clip::AntiAlias)
            .color(Colors::GREEN.primary())
            .shadow_color(Colors::RED.primary())
            .surface_tint_color(Colors::PURPLE.primary())
            .elevation(6.0)
            .margin(EdgeInsetsGeometry::all(7.0))
            .shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::circular(5.0),
            ))
    }

    fn scaffold_with_card() -> impl FnOnce(&mut App) -> WidgetRef {
        |_app| Scaffold::new().body(Card::new()).into_widget()
    }

    #[test]
    fn card_theme_data_copy_with_equality_basics() {
        assert_eq!(CardThemeData::new(), CardThemeData::new().copy_with());

        let theme = card_theme();
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(theme.copy_with().elevation(9.0).elevation, Some(9.0));
        assert_eq!(theme.copy_with().elevation(9.0).color, theme.color);
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(
            theme.copy_with().shape(RoundedRectangleBorder::default()),
            theme
        );
    }

    #[test]
    fn card_theme_data_lerp_special_cases() {
        assert_eq!(CardThemeData::lerp(None, None, 0.0), CardThemeData::new());
        let theme = CardThemeData::new();
        assert_eq!(CardThemeData::lerp(Some(&theme), Some(&theme), 0.5), theme);
    }

    #[test]
    fn card_theme_data_lerps_each_field() {
        let a = CardThemeData::new()
            .clip_behavior(Clip::None)
            .color(Colors::BLACK)
            .elevation(2.0)
            .margin(EdgeInsetsGeometry::all(10.0));
        let b = CardThemeData::new()
            .clip_behavior(Clip::HardEdge)
            .color(Colors::WHITE)
            .shadow_color(Colors::RED.primary())
            .elevation(6.0)
            .margin(EdgeInsetsGeometry::all(30.0));

        let before = CardThemeData::lerp(Some(&a), Some(&b), 0.25);
        let after = CardThemeData::lerp(Some(&a), Some(&b), 0.75);

        assert_eq!(before.clip_behavior, Some(Clip::None));
        assert_eq!(after.clip_behavior, Some(Clip::HardEdge));
        assert_eq!(
            before.color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.25)
        );
        assert_eq!(
            before.shadow_color,
            Color::lerp(None, Some(Colors::RED.primary()), 0.25)
        );
        assert_eq!(before.elevation, Some(3.0));
        assert_eq!(before.margin, Some(EdgeInsetsGeometry::all(15.0)));
    }

    #[test]
    fn card_theme_data_defaults() {
        let card_theme_data = CardThemeData::new();

        assert_eq!(card_theme_data.clip_behavior, None);
        assert_eq!(card_theme_data.color, None);
        assert_eq!(card_theme_data.elevation, None);
        assert_eq!(card_theme_data.margin, None);
        assert_eq!(card_theme_data.shadow_color, None);
        assert!(card_theme_data.shape.is_none());
        assert_eq!(card_theme_data.surface_tint_color, None);

        let card_theme = CardTheme::new().with_data(CardThemeData::new());
        assert_eq!(card_theme.clip_behavior(), None);
        assert_eq!(card_theme.color(), None);
        assert_eq!(card_theme.elevation(), None);
        assert_eq!(card_theme.margin(), None);
        assert_eq!(card_theme.shadow_color(), None);
        assert!(card_theme.shape().is_none());
        assert_eq!(card_theme.surface_tint_color(), None);
    }

    #[test]
    fn card_theme_reads_its_properties_or_its_data() {
        let by_properties = CardTheme::new()
            .with_clip_behavior(Clip::AntiAlias)
            .with_color(Colors::GREEN.primary())
            .with_shadow_color(Colors::RED.primary())
            .with_surface_tint_color(Colors::PURPLE.primary())
            .with_elevation(6.0)
            .with_margin(EdgeInsetsGeometry::all(7.0))
            .with_shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::circular(5.0),
            ));
        let by_data = CardTheme::new().with_data(card_theme());

        assert_eq!(by_properties.data(), card_theme());
        assert_eq!(by_data.data(), card_theme());
        assert_eq!(by_properties.elevation(), by_data.elevation());
        assert_eq!(by_properties.shape(), by_data.shape());
        assert_eq!(by_data.copy_with().data(), card_theme());
    }

    #[test]
    fn card_theme_lerps_through_its_getters() {
        let a = CardTheme::new().with_elevation(2.0);
        let b = CardTheme::new().with_data(CardThemeData::new().elevation(6.0));

        assert_eq!(
            CardTheme::lerp(Some(&a), Some(&b), 0.5).elevation(),
            Some(4.0)
        );
        assert_eq!(
            CardTheme::lerp(None, None, 0.5).data(),
            CardThemeData::new()
        );
        assert_eq!(
            CardTheme::lerp(Some(&a), Some(&a), 0.5).elevation(),
            Some(2.0)
        );
    }

    #[test]
    fn material3_passing_no_card_theme_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, scaffold_with_card());
            let colors = m3(&tester.cell.borrow()).color_scheme;

            let padding = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.color, Some(colors.surface_container_low()));
            assert_eq!(material.shadow_color, Some(colors.shadow()));
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 1.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(4.0));
            assert_eq!(material.shape, rounded(12.0));
        });
    }

    #[test]
    fn card_uses_values_from_card_theme() {
        with_main_thread_stack(|| {
            let card_theme = card_theme();
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .card_theme(card_theme.clone())
                        .build(app)
                },
                scaffold_with_card(),
            );

            let padding = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(Some(material.clip_behavior), card_theme.clip_behavior);
            assert_eq!(material.color, card_theme.color);
            assert_eq!(material.shadow_color, card_theme.shadow_color);
            assert_eq!(material.surface_tint_color, card_theme.surface_tint_color);
            assert_eq!(Some(material.elevation), card_theme.elevation);
            assert_eq!(Some(padding), card_theme.margin);
            assert_eq!(material.shape, card_theme.shape);
        });
    }

    #[test]
    fn card_widget_properties_take_priority_over_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .card_color(Colors::PINK.primary())
                        .card_theme(card_theme())
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .body(
                            Card::new()
                                .clip_behavior(Clip::HardEdge)
                                .color(Colors::ORANGE.primary())
                                .shadow_color(Colors::PINK.primary())
                                .elevation(7.0)
                                .margin(EdgeInsetsGeometry::all(3.0))
                                .shape(RoundedRectangleBorder::new(
                                    BorderSide::NONE,
                                    BorderRadiusGeometry::circular(9.0),
                                )),
                        )
                        .into_widget()
                },
            );

            let padding = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(material.clip_behavior, Clip::HardEdge);
            assert_eq!(material.color, Some(Colors::ORANGE.primary()));
            assert_eq!(material.shadow_color, Some(Colors::PINK.primary()));
            assert_eq!(material.elevation, 7.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(3.0));
            assert_eq!(material.shape, rounded(9.0));
        });
    }

    #[test]
    fn card_theme_properties_take_priority_over_theme_data_properties() {
        with_main_thread_stack(|| {
            let card_theme = card_theme();
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .card_color(Colors::PINK.primary())
                        .card_theme(card_theme.clone())
                        .build(app)
                },
                scaffold_with_card(),
            );

            assert_eq!(card_material(&tester).color, card_theme.color);
        });
    }

    #[test]
    fn material3_theme_data_properties_are_used_when_no_card_theme_is_set() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, scaffold_with_card());
            let colors = m3(&tester.cell.borrow()).color_scheme;

            assert_eq!(
                card_material(&tester).color,
                Some(colors.surface_container_low())
            );
        });
    }

    #[test]
    fn card_properties_are_taken_over_the_theme_values() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .card_theme(
                            CardThemeData::new()
                                .clip_behavior(Clip::AntiAlias)
                                .color(Colors::RED.primary())
                                .shadow_color(Colors::ORANGE.primary())
                                .elevation(10.0)
                                .margin(EdgeInsetsGeometry::all(12.0))
                                .shape(RoundedRectangleBorder::new(
                                    BorderSide::NONE,
                                    BorderRadiusGeometry::circular(15.0),
                                )),
                        )
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .body(
                            Card::new()
                                .clip_behavior(Clip::HardEdge)
                                .color(Colors::YELLOW.primary())
                                .shadow_color(Colors::GREEN.primary())
                                .elevation(20.0)
                                .margin(EdgeInsetsGeometry::all(18.0))
                                .shape(RoundedRectangleBorder::new(
                                    BorderSide::NONE,
                                    BorderRadiusGeometry::circular(25.0),
                                ))
                                .child(waft_widgets::SizedBox::new().width(200.0).height(200.0)),
                        )
                        .into_widget()
                },
            );

            let card_margin = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(material.clip_behavior, Clip::HardEdge);
            assert_eq!(material.color, Some(Colors::YELLOW.primary()));
            assert_eq!(material.shadow_color, Some(Colors::GREEN.primary()));
            assert_eq!(material.elevation, 20.0);
            assert_eq!(material.shape, rounded(25.0));
            assert_eq!(card_margin, EdgeInsetsGeometry::all(18.0));
        });
    }

    #[test]
    fn local_card_theme_can_override_global_card_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .card_theme(
                            CardThemeData::new()
                                .clip_behavior(Clip::AntiAlias)
                                .color(Colors::RED.primary())
                                .shadow_color(Colors::ORANGE.primary())
                                .elevation(10.0)
                                .margin(EdgeInsetsGeometry::all(12.0))
                                .shape(RoundedRectangleBorder::new(
                                    BorderSide::NONE,
                                    BorderRadiusGeometry::circular(15.0),
                                )),
                        )
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .body(
                            CardTheme::new()
                                .with_data(
                                    CardThemeData::new()
                                        .clip_behavior(Clip::HardEdge)
                                        .color(Colors::YELLOW.primary())
                                        .shadow_color(Colors::GREEN.primary())
                                        .elevation(20.0)
                                        .margin(EdgeInsetsGeometry::all(18.0))
                                        .shape(RoundedRectangleBorder::new(
                                            BorderSide::NONE,
                                            BorderRadiusGeometry::circular(25.0),
                                        )),
                                )
                                .child(Card::new().child(
                                    waft_widgets::SizedBox::new().width(200.0).height(200.0),
                                )),
                        )
                        .into_widget()
                },
            );

            let card_margin = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(material.clip_behavior, Clip::HardEdge);
            assert_eq!(material.color, Some(Colors::YELLOW.primary()));
            assert_eq!(material.shadow_color, Some(Colors::GREEN.primary()));
            assert_eq!(material.elevation, 20.0);
            assert_eq!(material.shape, rounded(25.0));
            assert_eq!(card_margin, EdgeInsetsGeometry::all(18.0));
        });
    }

    #[test]
    fn material2_theme_data_properties_are_used_when_no_card_theme_is_set() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m2, scaffold_with_card());
            let card_color = m2(&tester.cell.borrow()).card_color;

            assert_eq!(card_material(&tester).color, Some(card_color));
        });
    }

    #[test]
    fn material2_passing_no_card_theme_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m2, scaffold_with_card());

            let padding = card_padding(&tester);
            let material = card_material(&tester);

            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.color, Some(Colors::WHITE));
            assert_eq!(material.shadow_color, Some(Colors::BLACK));
            assert_eq!(material.surface_tint_color, None);
            assert_eq!(material.elevation, 1.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(4.0));
            assert_eq!(material.shape, rounded(4.0));
        });
    }

    #[test]
    fn of_returns_the_theme_data_card_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = card_theme();
            let theme_data_card = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .card_theme(theme_data_card.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                CardTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_card_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_card = CardThemeData::new().elevation(7.0);
            let expected = widget_card.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .card_theme(CardThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(
                        data,
                        CardTheme::new().with_data(widget_card.clone()).child(child),
                    )
                    .into_widget()
                },
                CardTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_card_theme_without_properties_shadows_theme_data_with_an_empty_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .card_theme(CardThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(data, CardTheme::new().child(child)).into_widget()
                },
                CardTheme::of,
            );

            assert_eq!(seen, CardThemeData::new());
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |elevation: f64| CardTheme::new().with_elevation(elevation);
        let old = theme(1.0);

        assert!(!theme(1.0).update_should_notify(&old));
        assert!(theme(2.0).update_should_notify(&old));
        assert!(
            !CardTheme::new()
                .with_data(CardThemeData::new().elevation(1.0))
                .update_should_notify(&old)
        );
    }

    #[test]
    fn theme_data_holds_the_card_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.card_theme, CardThemeData::new());

            let raised = ThemeData::builder()
                .card_theme(CardThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.card_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .card_theme(raised.card_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .card_theme(CardThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5).card_theme.elevation,
                Some(4.0)
            );
        });
    }

    #[test]
    fn a_captured_card_theme_is_not_a_theme_to_wrap() {
        // CardTheme extends InheritedWidget, not InheritedTheme: a `Theme` that captures its
        // ancestors does not carry it, so the probe reads what is nearest to it.
        with_main_thread_stack(|| {
            let cell = test_cell();

            let seen = theme_seen_through_capture(
                &cell,
                |probe| {
                    CardTheme::new()
                        .with_data(CardThemeData::new().elevation(1.0))
                        .child(probe)
                        .into_widget()
                },
                |wrapped| {
                    CardTheme::new()
                        .with_data(CardThemeData::new().elevation(2.0))
                        .child(wrapped)
                        .into_widget()
                },
                CardTheme::of,
            );

            assert_eq!(seen.elevation, Some(2.0));
        });
    }
}

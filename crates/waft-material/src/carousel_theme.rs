//! Flutter counterpart: material_ui's `carousel_theme.dart`.

use waft_embedder::{Clip, Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsets, OutlinedBorderRef};
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant [`CarouselView`](crate::CarouselView)
/// widgets.
///
/// Descendant widgets obtain the current [`CarouselViewThemeData`] object using
/// [`CarouselViewTheme::of`]. Instances of [`CarouselViewThemeData`] can be customized with
/// [`CarouselViewThemeData::copy_with`].
///
/// Typically a [`CarouselViewThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::carousel_view_theme`.
///
/// All [`CarouselViewThemeData`] properties are `None` by default. When null, the
/// [`CarouselView`](crate::CarouselView) will use the values from the default values
/// provided by the carousel.
///
/// See also:
///
///  * [`CarouselViewTheme`], an [`InheritedWidget`] that propagates the theme down its
///    subtree.
///  * `ThemeData::carousel_view_theme`, which describes the overall theme information for
///    the application.
#[derive(Clone, Debug, Default)]
pub struct CarouselViewThemeData {
    /// The amount of space surrounding each carousel item.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to `EdgeInsets::all(4.0)`.
    pub padding: Option<EdgeInsets>,

    /// The background color of each carousel item.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to
    /// `ColorScheme::surface`.
    pub background_color: Option<Color>,

    /// The z-coordinate of each carousel item's `Material`.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to 0.0.
    pub elevation: Option<f64>,

    /// The shape of each carousel item's `Material`.
    ///
    /// Defines each item's `Material::shape`.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to a
    /// `RoundedRectangleBorder` with a circular border radius of 28.0.
    pub shape: Option<OutlinedBorderRef>,

    /// The clip behavior of each carousel item.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to [`Clip::AntiAlias`].
    pub item_clip_behavior: Option<Clip>,

    /// The highlight color of each carousel item in the different `WidgetState`s.
    ///
    /// If null, [`CarouselView`](crate::CarouselView) defaults to
    /// `ColorScheme::onSurface` with the Material 3 opacities for hover, focus and press.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
}

impl CarouselViewThemeData {
    /// Creates a theme that can be used for `ThemeData::carousel_view_theme`. Dart's
    /// optional named arguments are the fluent setters.
    pub fn new() -> CarouselViewThemeData {
        CarouselViewThemeData::default()
    }

    /// Dart `CarouselViewThemeData(padding:)`.
    pub fn padding(mut self, padding: EdgeInsets) -> CarouselViewThemeData {
        self.padding = Some(padding);
        self
    }

    /// Dart `CarouselViewThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> CarouselViewThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `CarouselViewThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> CarouselViewThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `CarouselViewThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> CarouselViewThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `CarouselViewThemeData(itemClipBehavior:)`.
    pub fn item_clip_behavior(mut self, item_clip_behavior: Clip) -> CarouselViewThemeData {
        self.item_clip_behavior = Some(item_clip_behavior);
        self
    }

    /// Dart `CarouselViewThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CarouselViewThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new values:
    /// Dart's `copyWith`, whose named arguments are the fluent setters on the copy.
    pub fn copy_with(&self) -> CarouselViewThemeData {
        self.clone()
    }

    /// Linearly interpolate between two carousel themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&CarouselViewThemeData>,
        b: Option<&CarouselViewThemeData>,
        t: f64,
    ) -> CarouselViewThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        CarouselViewThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shape: OutlinedBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            padding: EdgeInsets::lerp(a.and_then(|a| a.padding), b.and_then(|b| b.padding), t),
            item_clip_behavior: if t < 0.5 {
                a.and_then(|a| a.item_clip_behavior)
            } else {
                b.and_then(|b| b.item_clip_behavior)
            },
        }
    }
}

impl PartialEq for CarouselViewThemeData {
    fn eq(&self, other: &CarouselViewThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.background_color == self.background_color
            && other.elevation == self.elevation
            && other.shape == self.shape
            && other.overlay_color == self.overlay_color
            && other.padding == self.padding
            && other.item_clip_behavior == self.item_clip_behavior
    }
}

/// An inherited widget that defines the configuration for [`CarouselView`](crate::CarouselView)s
/// in this widget's subtree.
///
/// Values specified here are used for [`CarouselView`](crate::CarouselView) properties that
/// are not given an explicit non-null value.
#[derive(Debug)]
pub struct CarouselViewTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant [`CarouselView`](crate::CarouselView) widgets.
    pub data: CarouselViewThemeData,
    pub child: WidgetRef,
}

impl CarouselViewTheme {
    /// Creates a theme that controls the configurations for [`CarouselView`](crate::CarouselView)
    /// in its widget subtree.
    pub fn new<K>(data: CarouselViewThemeData, child: impl IntoWidget<K>) -> CarouselViewTheme {
        CarouselViewTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `CarouselViewTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> CarouselViewTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest
    /// [`CarouselViewTheme`] ancestor. If there is no ancestor, it returns
    /// `ThemeData::carousel_view_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = CarouselViewTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> CarouselViewThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<CarouselViewTheme>(app) {
            Some(inherited_theme) => inherited_theme.data.clone(),
            None => Theme::of(app, context).carousel_view_theme.clone(),
        }
    }
}

impl InheritedTheme for CarouselViewTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        CarouselViewTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for CarouselViewTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &CarouselViewTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::EdgeInsetsGeometry;
    use waft_painting::{BorderRadiusGeometry, RoundedRectangleBorder};
    use waft_widgets::testing::{WidgetTester, expect, find, finds_one_widget, test_widgets};
    use waft_widgets::{Center, Padding, SizedBox, WidgetStates};

    use super::*;
    use crate::carousel::CarouselView;
    use crate::colors::Colors;
    use crate::ink_well::InkWell;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn carousel_view_theme_data() -> CarouselViewThemeData {
        CarouselViewThemeData::new()
            .background_color(Color::new(0xFF0000FF))
            .elevation(5.0)
            .padding(EdgeInsets::all(10.0))
            .shape(RoundedRectangleBorder::default())
            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                Colors::RED.primary(),
            )))
            .item_clip_behavior(Clip::HardEdge)
    }

    /// Dart's `find.descendant(of: find.byType(CarouselView), matching:
    /// find.byWidgetPredicate((widget) => widget is Padding && widget.child is Material))`.
    fn item_padding() -> waft_widgets::testing::Finder {
        find::descendant(
            find::by_type::<CarouselView>(),
            find::by_widget_predicate("Padding with a Material child", |_app, widget| {
                waft_widgets::downcast_widget::<Padding>(widget).is_some_and(|padding| {
                    padding.child.as_ref().is_some_and(|child| {
                        waft_widgets::downcast_widget::<Material>(&**child).is_some()
                    })
                })
            }),
        )
    }

    fn pump_carousel(
        tester: &mut WidgetTester,
        theme: ThemeData,
        carousel: impl FnOnce() -> WidgetRef,
        local_theme: Option<CarouselViewThemeData>,
    ) {
        let mut child = carousel();
        if let Some(data) = local_theme {
            child = CarouselViewTheme::new(data, child).into_widget();
        }
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .body(Center::new().child(child))
                .into_widget(),
        ));
    }

    fn read_item(
        tester: &WidgetTester,
    ) -> (
        EdgeInsetsGeometry,
        Option<Color>,
        f64,
        Option<waft_painting::ShapeBorderRef>,
        Clip,
    ) {
        tester.widget(item_padding(), |padding: &Padding| {
            let child = padding.child.as_ref().expect("a child");
            let material =
                waft_widgets::downcast_widget::<Material>(&**child).expect("a Material child");
            (
                padding.padding,
                material.color,
                material.elevation,
                material.shape.clone(),
                material.clip_behavior,
            )
        })
    }

    #[test]
    fn carousel_view_theme_data_copy_with_equals_and_hash_code_basics() {
        assert_eq!(
            CarouselViewThemeData::new(),
            CarouselViewThemeData::new().copy_with()
        );
    }

    #[test]
    fn carousel_view_theme_data_null_fields_by_default() {
        let carousel_view_theme = CarouselViewThemeData::new();
        assert_eq!(carousel_view_theme.background_color, None);
        assert_eq!(carousel_view_theme.elevation, None);
        assert!(carousel_view_theme.overlay_color.is_none());
        assert_eq!(carousel_view_theme.padding, None);
        assert!(carousel_view_theme.shape.is_none());
        assert_eq!(carousel_view_theme.item_clip_behavior, None);
    }

    #[test]
    fn uses_value_from_carousel_view_theme_data() {
        test_widgets(|tester| {
            let carousel_view_theme = carousel_view_theme_data();
            let theme = ThemeData::builder()
                .carousel_view_theme(carousel_view_theme.clone())
                .build(&tester.cell.borrow());
            pump_carousel(
                tester,
                theme,
                || {
                    CarouselView::new(
                        100.0,
                        vec![SizedBox::new().width(100.0).height(100.0).into_widget()],
                    )
                    .into_widget()
                },
                None,
            );
            expect(tester, &find::by_type::<CarouselView>(), finds_one_widget());
            expect(tester, &item_padding(), finds_one_widget());

            let (padding, color, elevation, shape, clip_behavior) = read_item(tester);
            let ink_well_overlay_color = tester.widget(
                find::descendant(find::by_type::<CarouselView>(), find::by_type::<InkWell>()),
                |ink_well: &InkWell| ink_well.ink_response.overlay_color.clone(),
            );

            assert_eq!(Some(padding), carousel_view_theme.padding.map(Into::into));
            assert_eq!(color, carousel_view_theme.background_color);
            assert_eq!(Some(elevation), carousel_view_theme.elevation);
            assert_eq!(shape, carousel_view_theme.shape.clone().map(Into::into));
            assert_eq!(ink_well_overlay_color, carousel_view_theme.overlay_color);
            assert_eq!(Some(clip_behavior), carousel_view_theme.item_clip_behavior);
        });
    }

    #[test]
    fn widgets_properties_override_theme() {
        test_widgets(|tester| {
            let carousel_view_theme = carousel_view_theme_data();
            let background_color = Color::new(0xFFFF0000);
            let elevation = 10.0;
            let padding = EdgeInsets::all(15.0);
            let shape = RoundedRectangleBorder::new(
                waft_painting::BorderSide::NONE,
                BorderRadiusGeometry::circular(10.0),
            );
            let overlay_color =
                <dyn WidgetStateProperty<Option<Color>>>::all(Some(Colors::GREEN.primary()));
            let theme = ThemeData::builder()
                .carousel_view_theme(carousel_view_theme)
                .build(&tester.cell.borrow());
            pump_carousel(
                tester,
                theme,
                || {
                    CarouselView::new(
                        100.0,
                        vec![SizedBox::new().width(100.0).height(100.0).into_widget()],
                    )
                    .background_color(background_color)
                    .elevation(elevation)
                    .padding(padding)
                    .shape(shape)
                    .overlay_color(overlay_color.clone())
                    .item_clip_behavior(Clip::HardEdge)
                    .into_widget()
                },
                None,
            );
            expect(tester, &find::by_type::<CarouselView>(), finds_one_widget());
            expect(tester, &item_padding(), finds_one_widget());

            let (item_padding, color, item_elevation, item_shape, clip_behavior) =
                read_item(tester);
            let ink_well_overlay_color = tester.widget(
                find::descendant(find::by_type::<CarouselView>(), find::by_type::<InkWell>()),
                |ink_well: &InkWell| ink_well.ink_response.overlay_color.clone(),
            );

            assert_eq!(item_padding, padding.into());
            assert_eq!(color, Some(background_color));
            assert_eq!(item_elevation, elevation);
            assert_eq!(item_shape, Some(shape.into()));
            assert_eq!(ink_well_overlay_color, Some(overlay_color));
            assert_eq!(clip_behavior, Clip::HardEdge);
        });
    }

    #[test]
    fn carousel_view_theme_can_override_theme_carousel_view_theme() {
        test_widgets(|tester| {
            let local_padding = EdgeInsets::all(15.0);
            let local_shape = RoundedRectangleBorder::new(
                waft_painting::BorderSide::NONE,
                BorderRadiusGeometry::circular(10.0),
            );
            let theme = ThemeData::builder()
                .carousel_view_theme(
                    CarouselViewThemeData::new()
                        .background_color(Color::new(0xfffffff1))
                        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                            Color::new(0xff000000),
                        )))
                        .elevation(5.0)
                        .padding(EdgeInsets::all(10.0))
                        .shape(RoundedRectangleBorder::default())
                        .item_clip_behavior(Clip::HardEdge),
                )
                .build(&tester.cell.borrow());
            pump_carousel(
                tester,
                theme,
                || {
                    CarouselView::new(
                        100.0,
                        vec![SizedBox::new().width(100.0).height(100.0).into_widget()],
                    )
                    .into_widget()
                },
                Some(
                    CarouselViewThemeData::new()
                        .background_color(Color::new(0xffff0000))
                        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                            Color::new(0xffffffff),
                        )))
                        .elevation(10.0)
                        .padding(local_padding)
                        .shape(local_shape)
                        .item_clip_behavior(Clip::AntiAlias),
                ),
            );
            expect(tester, &item_padding(), finds_one_widget());

            let (padding, color, elevation, shape, clip_behavior) = read_item(tester);
            let overlay_color = tester
                .widget(
                    find::descendant(find::by_type::<CarouselView>(), find::by_type::<InkWell>()),
                    |ink_well: &InkWell| ink_well.ink_response.overlay_color.clone(),
                )
                .and_then(|overlay_color| overlay_color.resolve(&WidgetStates::new()));

            assert_eq!(padding, local_padding.into());
            assert_eq!(color, Some(Color::new(0xffff0000)));
            assert_eq!(elevation, 10.0);
            assert_eq!(shape, Some(local_shape.into()));
            assert_eq!(overlay_color, Some(Color::new(0xffffffff)));
            assert_eq!(clip_behavior, Clip::AntiAlias);
        });
    }
}

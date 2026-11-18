//! Flutter counterpart: material_ui's `navigation_rail_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{ShapeBorderRef, TextStyle};
use waft_widgets::{
    BuildContext, IconThemeData, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
};

use crate::navigation_rail::NavigationRailLabelType;
use crate::theme::Theme;

/// Defines default property values for descendant `NavigationRail`
/// widgets.
///
/// Descendant widgets obtain the current [`NavigationRailThemeData`] object
/// using [`NavigationRailTheme::of`]. Instances of [`NavigationRailThemeData`]
/// can be customized with [`NavigationRailThemeData::copy_with`].
///
/// Typically a [`NavigationRailThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::navigation_rail_theme`.
///
/// All [`NavigationRailThemeData`] properties are `null` by default.
/// When null, the `NavigationRail` will use the values from `ThemeData`
/// if they exist, otherwise it will provide its own defaults based on the
/// overall `Theme`'s text_theme and color_scheme. See the individual
/// `NavigationRail` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NavigationRailThemeData {
    /// Color to be used for the `NavigationRail`'s background.
    pub background_color: Option<Color>,

    /// The z-coordinate to be used for the `NavigationRail`'s elevation.
    pub elevation: Option<f64>,

    /// The style to merge with the default text style for
    /// `NavigationRailDestination` labels, when the destination is not selected.
    pub unselected_label_text_style: Option<TextStyle>,

    /// The style to merge with the default text style for
    /// `NavigationRailDestination` labels, when the destination is selected.
    pub selected_label_text_style: Option<TextStyle>,

    /// The theme to merge with the default icon theme for
    /// `NavigationRailDestination` icons, when the destination is not selected.
    pub unselected_icon_theme: Option<IconThemeData>,

    /// The theme to merge with the default icon theme for
    /// `NavigationRailDestination` icons, when the destination is selected.
    pub selected_icon_theme: Option<IconThemeData>,

    /// The alignment for the `NavigationRailDestination`s as they are positioned
    /// within the `NavigationRail`.
    pub group_alignment: Option<f64>,

    /// The type that defines the layout and behavior of the labels in the
    /// `NavigationRail`.
    pub label_type: Option<NavigationRailLabelType>,

    /// Whether or not the selected `NavigationRailDestination` should include a
    /// `NavigationIndicator`.
    pub use_indicator: Option<bool>,

    /// Overrides the default value of `NavigationRail`'s selection indicator color,
    /// when [`use_indicator`](Self::use_indicator) is true.
    pub indicator_color: Option<Color>,

    /// Overrides the default shape of the `NavigationRail`'s selection indicator.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// Overrides the default value of `NavigationRail`'s minimum width when it
    /// is not extended.
    pub min_width: Option<f64>,

    /// Overrides the default value of `NavigationRail`'s minimum width when it
    /// is extended.
    pub min_extended_width: Option<f64>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `NavigationRailThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait NavigationRailThemeDataGetters for NavigationRailThemeData, fields: navigation_rail_theme_data {
        background_color: Option<Color>,
        elevation: Option<f64>,
        unselected_label_text_style: Option<TextStyle>,
        selected_label_text_style: Option<TextStyle>,
        unselected_icon_theme: Option<IconThemeData>,
        selected_icon_theme: Option<IconThemeData>,
        group_alignment: Option<f64>,
        label_type: Option<NavigationRailLabelType>,
        use_indicator: Option<bool>,
        indicator_color: Option<Color>,
        indicator_shape: Option<ShapeBorderRef>,
        min_width: Option<f64>,
        min_extended_width: Option<f64>,
    }
}

impl NavigationRailThemeData {
    /// Creates a theme that can be used for `ThemeData::navigation_rail_theme`; Dart's
    /// optional named arguments are the fluent setters.
    pub fn new() -> NavigationRailThemeData {
        NavigationRailThemeData::default()
    }

    /// Dart `NavigationRailThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationRailThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationRailThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationRailThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationRailThemeData(unselectedLabelTextStyle:)`.
    pub fn unselected_label_text_style(
        mut self,
        unselected_label_text_style: TextStyle,
    ) -> NavigationRailThemeData {
        self.unselected_label_text_style = Some(unselected_label_text_style);
        self
    }

    /// Dart `NavigationRailThemeData(selectedLabelTextStyle:)`.
    pub fn selected_label_text_style(
        mut self,
        selected_label_text_style: TextStyle,
    ) -> NavigationRailThemeData {
        self.selected_label_text_style = Some(selected_label_text_style);
        self
    }

    /// Dart `NavigationRailThemeData(unselectedIconTheme:)`.
    pub fn unselected_icon_theme(
        mut self,
        unselected_icon_theme: IconThemeData,
    ) -> NavigationRailThemeData {
        self.unselected_icon_theme = Some(unselected_icon_theme);
        self
    }

    /// Dart `NavigationRailThemeData(selectedIconTheme:)`.
    pub fn selected_icon_theme(
        mut self,
        selected_icon_theme: IconThemeData,
    ) -> NavigationRailThemeData {
        self.selected_icon_theme = Some(selected_icon_theme);
        self
    }

    /// Dart `NavigationRailThemeData(groupAlignment:)`.
    pub fn group_alignment(mut self, group_alignment: f64) -> NavigationRailThemeData {
        self.group_alignment = Some(group_alignment);
        self
    }

    /// Dart `NavigationRailThemeData(labelType:)`.
    pub fn label_type(mut self, label_type: NavigationRailLabelType) -> NavigationRailThemeData {
        self.label_type = Some(label_type);
        self
    }

    /// Dart `NavigationRailThemeData(useIndicator:)`.
    pub fn use_indicator(mut self, use_indicator: bool) -> NavigationRailThemeData {
        self.use_indicator = Some(use_indicator);
        self
    }

    /// Dart `NavigationRailThemeData(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationRailThemeData {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationRailThemeData(indicatorShape:)`.
    pub fn indicator_shape(
        mut self,
        indicator_shape: impl Into<ShapeBorderRef>,
    ) -> NavigationRailThemeData {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationRailThemeData(minWidth:)`.
    pub fn min_width(mut self, min_width: f64) -> NavigationRailThemeData {
        self.min_width = Some(min_width);
        self
    }

    /// Dart `NavigationRailThemeData(minExtendedWidth:)`.
    pub fn min_extended_width(mut self, min_extended_width: f64) -> NavigationRailThemeData {
        self.min_extended_width = Some(min_extended_width);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().min_width(56.0)`).
    pub fn copy_with(&self) -> NavigationRailThemeData {
        self.clone()
    }

    /// Linearly interpolate between two navigation rail themes.
    ///
    /// If both arguments are null then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&NavigationRailThemeData>,
        b: Option<&NavigationRailThemeData>,
        t: f64,
    ) -> Option<NavigationRailThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(NavigationRailThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            unselected_label_text_style: TextStyle::lerp(
                a.and_then(|a| a.unselected_label_text_style.as_ref()),
                b.and_then(|b| b.unselected_label_text_style.as_ref()),
                t,
            ),
            selected_label_text_style: TextStyle::lerp(
                a.and_then(|a| a.selected_label_text_style.as_ref()),
                b.and_then(|b| b.selected_label_text_style.as_ref()),
                t,
            ),
            unselected_icon_theme: lerp_icon_theme(
                a.and_then(|a| a.unselected_icon_theme.as_ref()),
                b.and_then(|b| b.unselected_icon_theme.as_ref()),
                t,
            ),
            selected_icon_theme: lerp_icon_theme(
                a.and_then(|a| a.selected_icon_theme.as_ref()),
                b.and_then(|b| b.selected_icon_theme.as_ref()),
                t,
            ),
            group_alignment: lerp_double(
                a.and_then(|a| a.group_alignment),
                b.and_then(|b| b.group_alignment),
                t,
            ),
            label_type: if t < 0.5 {
                a.and_then(|a| a.label_type)
            } else {
                b.and_then(|b| b.label_type)
            },
            use_indicator: if t < 0.5 {
                a.and_then(|a| a.use_indicator)
            } else {
                b.and_then(|b| b.use_indicator)
            },
            indicator_color: Color::lerp(
                a.and_then(|a| a.indicator_color),
                b.and_then(|b| b.indicator_color),
                t,
            ),
            indicator_shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.indicator_shape.as_ref()),
                b.and_then(|b| b.indicator_shape.as_ref()),
                t,
            ),
            min_width: lerp_double(a.and_then(|a| a.min_width), b.and_then(|b| b.min_width), t),
            min_extended_width: lerp_double(
                a.and_then(|a| a.min_extended_width),
                b.and_then(|b| b.min_extended_width),
                t,
            ),
        })
    }
}

/// Dart's `a == null && b == null ? null : IconThemeData.lerp(a, b, t)`.
fn lerp_icon_theme(
    a: Option<&IconThemeData>,
    b: Option<&IconThemeData>,
    t: f64,
) -> Option<IconThemeData> {
    if a.is_none() && b.is_none() {
        None
    } else {
        Some(IconThemeData::lerp(a, b, t))
    }
}

/// An inherited widget that defines visual properties for `NavigationRail`s and
/// `NavigationRailDestination`s in this widget's subtree.
///
/// Values specified here are used for `NavigationRail` properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct NavigationRailTheme {
    pub key: Option<KeyRef>,
    /// Specifies the background color, elevation, label text style, icon theme,
    /// group alignment, and label type and border values for descendant
    /// `NavigationRail` widgets.
    pub data: NavigationRailThemeData,
    pub child: WidgetRef,
}

impl NavigationRailTheme {
    /// Creates a navigation rail theme that controls the
    /// [`NavigationRailThemeData`] properties for a `NavigationRail`.
    pub fn new<K>(data: NavigationRailThemeData, child: impl IntoWidget<K>) -> NavigationRailTheme {
        NavigationRailTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `NavigationRailTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationRailTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`NavigationRailThemeData`] from the closest ancestor
    /// [`NavigationRailTheme`].
    ///
    /// If there is no enclosing [`NavigationRailTheme`] widget, then
    /// `ThemeData::navigation_rail_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = NavigationRailTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> NavigationRailThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<NavigationRailTheme>(app) {
            Some(navigation_rail_theme) => navigation_rail_theme.data.clone(),
            None => Theme::of(app, context).navigation_rail_theme.clone(),
        }
    }
}

impl InheritedTheme for NavigationRailTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        NavigationRailTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for NavigationRailTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &NavigationRailTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/navigation_rail_theme_test.dart`.

    use waft_embedder::Size;
    use waft_painting::ShapeBorder;
    use waft_painting::{
        Alignment, AlignmentGeometry, BorderSide, RoundedRectangleBorder, ShapeDecoration,
        StadiumBorder,
    };
    use waft_widgets::{AnyElement, Flexible, Icon, IconData, IconTheme, RichText, Text};

    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_nothing, finds_widgets, test_widgets,
    };

    use super::*;
    use crate::icons::Icons;
    use crate::ink_decoration::Ink;
    use crate::ink_well::InkResponse;
    use crate::material::Material;
    use crate::navigation_bar::NavigationIndicator;
    use crate::navigation_rail::{NavigationRail, NavigationRailDestination};
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `_destinations()`.
    fn destinations() -> Vec<NavigationRailDestination> {
        vec![
            NavigationRailDestination::new(
                Icon::new(Some(Icons::favorite_border())),
                Text::new("Abc"),
            )
            .selected_icon(Icon::new(Some(Icons::favorite()))),
            NavigationRailDestination::new(Icon::new(Some(Icons::star_border())), Text::new("Def"))
                .selected_icon(Icon::new(Some(Icons::star()))),
        ]
    }

    fn pump(tester: &mut WidgetTester, theme: impl FnOnce(&App) -> ThemeData, body: WidgetRef) {
        let root = {
            let app = tester.cell.borrow();
            material_app(theme(&app), Scaffold::new().body(body).into_widget())
        };
        tester.pump_widget(root);
    }

    fn rail(tester: &WidgetTester) -> AnyElement {
        find::by_type::<NavigationRail>().evaluate(tester)[0]
    }

    /// Dart's `_railMaterial`: the first material is for the rail, and the rest are for the
    /// destinations.
    fn rail_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let material = tester.descendants_of_type::<Material>(rail(tester))[0];
        tester.widget(material, read)
    }

    /// Dart's `_indicatorDecoration`.
    fn indicator_decoration<R>(
        tester: &WidgetTester,
        read: impl FnOnce(&ShapeDecoration) -> R,
    ) -> R {
        let indicator = find::by_type::<NavigationIndicator>().evaluate(tester)[0];
        let ink = tester.descendants_of_type::<Ink>(indicator)[0];
        tester.widget(ink, |ink: &Ink| {
            read(
                ink.decoration
                    .as_deref()
                    .and_then(|decoration| decoration.as_any().downcast_ref())
                    .expect("a ShapeDecoration"),
            )
        })
    }

    /// Dart's `_iconTheme`: the first IconTheme is the one added by the navigation rail.
    fn icon_theme(tester: &WidgetTester, icon: IconData) -> IconThemeData {
        let element = tester
            .element(find::ancestor(find::by_icon(&icon), find::by_type::<IconTheme>()).first());
        tester.widget(element, |icon_theme: &IconTheme| icon_theme.data.clone())
    }

    fn selected_icon_theme(tester: &WidgetTester) -> IconThemeData {
        icon_theme(tester, Icons::favorite())
    }

    fn unselected_icon_theme(tester: &WidgetTester) -> IconThemeData {
        icon_theme(tester, Icons::star_border())
    }

    fn label_style(tester: &WidgetTester, text: &str) -> TextStyle {
        let text = find::text(text).evaluate(tester)[0];
        let rich_text = tester.descendants_of_type::<RichText>(text)[0];
        tester.widget(rich_text, |rich_text: &RichText| {
            rich_text.text.style().cloned().expect("a styled label")
        })
    }

    fn selected_label_style(tester: &WidgetTester) -> TextStyle {
        label_style(tester, "Abc")
    }

    fn unselected_label_style(tester: &WidgetTester) -> TextStyle {
        label_style(tester, "Def")
    }

    /// Dart's `_destinationSize`.
    fn destination_size(tester: &WidgetTester) -> Size {
        let material = tester.ancestor_of_type::<Material>(
            tester.element(find::by_icon(&Icons::favorite()).first()),
        );
        let render = tester.render_box_of(material);
        render.size(&tester.cell.borrow())
    }

    /// Dart's `_destinationsAlign(tester).alignment`: the first Flexible widget is the one
    /// within the main Column for the rail content.
    fn destinations_alignment(tester: &WidgetTester) -> AlignmentGeometry {
        let flexible = find::by_type::<Flexible>().evaluate(tester)[0];
        let align = tester.descendants_of_type::<waft_widgets::Align>(flexible)[0];
        tester.widget(align, |align: &waft_widgets::Align| align.alignment)
    }

    /// Dart's `_visibilityAboveLabel(text)`: the `Visibility` above the label, if any.
    fn visibility_above_label(tester: &WidgetTester, text: &str) -> Option<bool> {
        let text = find::text(text).evaluate(tester)[0];
        let ancestors = tester.ancestors(text);
        let app = tester.cell.borrow();
        ancestors.iter().find_map(|element| {
            waft_widgets::downcast_widget::<waft_widgets::Visibility>(&**element.widget(&app))
                .map(|visibility| visibility.visible)
        })
    }

    /// Dart's `_labelType`.
    fn label_type(tester: &WidgetTester) -> NavigationRailLabelType {
        match (
            visibility_above_label(tester, "Abc"),
            visibility_above_label(tester, "Def"),
        ) {
            (Some(abc_visible), Some(_)) => {
                if abc_visible {
                    NavigationRailLabelType::Selected
                } else {
                    NavigationRailLabelType::None
                }
            }
            _ => NavigationRailLabelType::All,
        }
    }

    #[test]
    fn copy_with_equality_basics() {
        assert_eq!(
            NavigationRailThemeData::new(),
            NavigationRailThemeData::new().copy_with()
        );
    }

    #[test]
    fn material3_default_values_are_used_when_no_navigation_rail_or_navigation_rail_theme_data_properties_are_specified()
     {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            // Material 3 defaults
            pump(
                tester,
                ThemeData::new,
                NavigationRail::new(destinations(), Some(0)).into_widget(),
            );

            assert_eq!(
                rail_material(tester, |material| material.color),
                Some(theme.color_scheme.surface())
            );
            assert_eq!(rail_material(tester, |material| material.elevation), 0.0);
            assert_eq!(destination_size(tester).width(), 80.0);
            assert_eq!(selected_icon_theme(tester).size, Some(24.0));
            assert_eq!(
                selected_icon_theme(tester).color,
                Some(theme.color_scheme.on_secondary_container().into())
            );
            assert_eq!(selected_icon_theme(tester).opacity, None);
            assert_eq!(unselected_icon_theme(tester).size, Some(24.0));
            assert_eq!(
                unselected_icon_theme(tester).color,
                Some(theme.color_scheme.on_surface_variant().into())
            );
            assert_eq!(unselected_icon_theme(tester).opacity, None);
            assert_eq!(selected_label_style(tester).font_size, Some(14.0));
            assert_eq!(unselected_label_style(tester).font_size, Some(14.0));
            assert_eq!(
                destinations_alignment(tester),
                AlignmentGeometry::TOP_CENTER
            );
            assert_eq!(label_type(tester), NavigationRailLabelType::None);
            expect(
                tester,
                &find::by_type::<NavigationIndicator>(),
                finds_widgets(),
            );
            assert_eq!(
                indicator_decoration(tester, |decoration| decoration.color.clone()),
                Some(theme.color_scheme.secondary_container().into())
            );
            let stadium = StadiumBorder::new(BorderSide::NONE);
            assert!(indicator_decoration(tester, |decoration| {
                &*decoration.shape == &stadium as &dyn ShapeBorder
            }));
            let ink_response = find::by_type::<InkResponse>().evaluate(tester)[0];
            assert!(tester.widget(ink_response, |ink_response: &InkResponse| {
                ink_response.custom_border.as_deref() == Some(&stadium as &dyn ShapeBorder)
            }));
        });
    }

    #[test]
    #[expect(
        deprecated,
        reason = "Dart's case calls the deprecated `copyWith(useMaterial3:)`"
    )]
    fn material2_default_values_are_used_when_no_navigation_rail_or_navigation_rail_theme_data_properties_are_specified()
     {
        test_widgets(|tester| {
            // This test can be removed when `useMaterial3` is deprecated.
            let theme = ThemeData::new(&tester.cell.borrow());
            pump(
                tester,
                |app| {
                    ThemeData::light(app, None)
                        .copy_with()
                        .use_material3(false)
                        .build()
                },
                NavigationRail::new(destinations(), Some(0)).into_widget(),
            );

            assert_eq!(
                rail_material(tester, |material| material.color),
                Some(theme.color_scheme.surface())
            );
            assert_eq!(rail_material(tester, |material| material.elevation), 0.0);
            assert_eq!(destination_size(tester).width(), 72.0);
            assert_eq!(selected_icon_theme(tester).size, Some(24.0));
            assert_eq!(
                selected_icon_theme(tester).color,
                Some(theme.color_scheme.primary().into())
            );
            assert_eq!(selected_icon_theme(tester).opacity, Some(1.0));
            assert_eq!(unselected_icon_theme(tester).size, Some(24.0));
            assert_eq!(
                unselected_icon_theme(tester).color,
                Some(theme.color_scheme.on_surface().into())
            );
            assert_eq!(unselected_icon_theme(tester).opacity, Some(0.64));
            assert_eq!(selected_label_style(tester).font_size, Some(14.0));
            assert_eq!(unselected_label_style(tester).font_size, Some(14.0));
            assert_eq!(
                destinations_alignment(tester),
                AlignmentGeometry::TOP_CENTER
            );
            assert_eq!(label_type(tester), NavigationRailLabelType::None);
            expect(
                tester,
                &find::by_type::<NavigationIndicator>(),
                finds_nothing(),
            );
        });
    }

    #[test]
    fn navigation_rail_theme_data_values_are_used_when_no_navigation_rail_properties_are_specified()
    {
        test_widgets(|tester| {
            let background_color = Color::new(0x00000001);
            let elevation = 7.0;
            let selected_icon_size = 25.0;
            let unselected_icon_size = 23.0;
            let selected_icon_color = Color::new(0x00000002);
            let unselected_icon_color = Color::new(0x00000003);
            let selected_icon_opacity = 0.99;
            let unselected_icon_opacity = 0.98;
            let selected_label_font_size = 13.0;
            let unselected_label_font_size = 11.0;
            let group_alignment = 0.0;
            let label_type_value = NavigationRailLabelType::All;
            let use_indicator = true;
            let indicator_color = Color::new(0x00000004);
            let indicator_shape = RoundedRectangleBorder::default();

            pump(
                tester,
                ThemeData::new,
                NavigationRailTheme::new(
                    NavigationRailThemeData::new()
                        .background_color(background_color)
                        .elevation(elevation)
                        .selected_icon_theme(
                            IconThemeData::new()
                                .size(selected_icon_size)
                                .color(selected_icon_color)
                                .opacity(selected_icon_opacity),
                        )
                        .unselected_icon_theme(
                            IconThemeData::new()
                                .size(unselected_icon_size)
                                .color(unselected_icon_color)
                                .opacity(unselected_icon_opacity),
                        )
                        .selected_label_text_style(
                            TextStyle::new().font_size(selected_label_font_size),
                        )
                        .unselected_label_text_style(
                            TextStyle::new().font_size(unselected_label_font_size),
                        )
                        .group_alignment(group_alignment)
                        .label_type(label_type_value)
                        .use_indicator(use_indicator)
                        .indicator_color(indicator_color)
                        .indicator_shape(indicator_shape),
                    NavigationRail::new(destinations(), Some(0)),
                )
                .into_widget(),
            );

            assert_eq!(
                rail_material(tester, |material| material.color),
                Some(background_color)
            );
            assert_eq!(
                rail_material(tester, |material| material.elevation),
                elevation
            );
            assert_eq!(selected_icon_theme(tester).size, Some(selected_icon_size));
            assert_eq!(
                selected_icon_theme(tester).color,
                Some(selected_icon_color.into())
            );
            assert_eq!(
                selected_icon_theme(tester).opacity,
                Some(selected_icon_opacity)
            );
            assert_eq!(
                unselected_icon_theme(tester).size,
                Some(unselected_icon_size)
            );
            assert_eq!(
                unselected_icon_theme(tester).color,
                Some(unselected_icon_color.into())
            );
            assert_eq!(
                unselected_icon_theme(tester).opacity,
                Some(unselected_icon_opacity)
            );
            assert_eq!(
                selected_label_style(tester).font_size,
                Some(selected_label_font_size)
            );
            assert_eq!(
                unselected_label_style(tester).font_size,
                Some(unselected_label_font_size)
            );
            assert_eq!(
                destinations_alignment(tester),
                AlignmentGeometry::Alignment(Alignment::CENTER)
            );
            assert_eq!(label_type(tester), label_type_value);
            expect(
                tester,
                &find::by_type::<NavigationIndicator>(),
                finds_widgets(),
            );
            assert_eq!(
                indicator_decoration(tester, |decoration| decoration.color.clone()),
                Some(indicator_color.into())
            );
            assert!(indicator_decoration(tester, |decoration| {
                &*decoration.shape == &indicator_shape as &dyn ShapeBorder
            }));
        });
    }

    #[test]
    fn navigation_rail_values_take_priority_over_navigation_rail_theme_data_values_when_both_properties_are_specified()
     {
        test_widgets(|tester| {
            let background_color = Color::new(0x00000001);
            let elevation = 7.0;
            let selected_icon_size = 25.0;
            let unselected_icon_size = 23.0;
            let selected_icon_color = Color::new(0x00000002);
            let unselected_icon_color = Color::new(0x00000003);
            let selected_icon_opacity = 0.99;
            let unselected_icon_opacity = 0.98;
            let selected_label_font_size = 13.0;
            let unselected_label_font_size = 11.0;
            let group_alignment = 0.0;
            let label_type_value = NavigationRailLabelType::All;
            let use_indicator = true;
            let indicator_color = Color::new(0x00000004);

            pump(
                tester,
                ThemeData::new,
                NavigationRailTheme::new(
                    NavigationRailThemeData::new()
                        .background_color(Color::new(0x00000099))
                        .elevation(5.0)
                        .selected_icon_theme(
                            IconThemeData::new()
                                .size(31.0)
                                .color(Color::new(0x00000098))
                                .opacity(0.81),
                        )
                        .unselected_icon_theme(
                            IconThemeData::new()
                                .size(37.0)
                                .color(Color::new(0x00000097))
                                .opacity(0.82),
                        )
                        .selected_label_text_style(TextStyle::new().font_size(9.0))
                        .unselected_label_text_style(TextStyle::new().font_size(7.0))
                        .group_alignment(1.0)
                        .label_type(NavigationRailLabelType::Selected)
                        .use_indicator(false)
                        .indicator_color(Color::new(0x00000096)),
                    NavigationRail::new(destinations(), Some(0))
                        .background_color(background_color)
                        .elevation(elevation)
                        .selected_icon_theme(
                            IconThemeData::new()
                                .size(selected_icon_size)
                                .color(selected_icon_color)
                                .opacity(selected_icon_opacity),
                        )
                        .unselected_icon_theme(
                            IconThemeData::new()
                                .size(unselected_icon_size)
                                .color(unselected_icon_color)
                                .opacity(unselected_icon_opacity),
                        )
                        .selected_label_text_style(
                            TextStyle::new().font_size(selected_label_font_size),
                        )
                        .unselected_label_text_style(
                            TextStyle::new().font_size(unselected_label_font_size),
                        )
                        .group_alignment(group_alignment)
                        .label_type(label_type_value)
                        .use_indicator(use_indicator)
                        .indicator_color(indicator_color),
                )
                .into_widget(),
            );

            assert_eq!(
                rail_material(tester, |material| material.color),
                Some(background_color)
            );
            assert_eq!(
                rail_material(tester, |material| material.elevation),
                elevation
            );
            assert_eq!(selected_icon_theme(tester).size, Some(selected_icon_size));
            assert_eq!(
                selected_icon_theme(tester).color,
                Some(selected_icon_color.into())
            );
            assert_eq!(
                selected_icon_theme(tester).opacity,
                Some(selected_icon_opacity)
            );
            assert_eq!(
                unselected_icon_theme(tester).size,
                Some(unselected_icon_size)
            );
            assert_eq!(
                unselected_icon_theme(tester).color,
                Some(unselected_icon_color.into())
            );
            assert_eq!(
                unselected_icon_theme(tester).opacity,
                Some(unselected_icon_opacity)
            );
            assert_eq!(
                selected_label_style(tester).font_size,
                Some(selected_label_font_size)
            );
            assert_eq!(
                unselected_label_style(tester).font_size,
                Some(unselected_label_font_size)
            );
            assert_eq!(
                destinations_alignment(tester),
                AlignmentGeometry::Alignment(Alignment::CENTER)
            );
            assert_eq!(label_type(tester), label_type_value);
            expect(
                tester,
                &find::by_type::<NavigationIndicator>(),
                finds_widgets(),
            );
            assert_eq!(
                indicator_decoration(tester, |decoration| decoration.color.clone()),
                Some(indicator_color.into())
            );
        });
    }

    /// Regression test for https://github.com/flutter/flutter/issues/118618.
    #[test]
    fn navigation_rail_theme_data_lerps_correctly_with_null_icon_themes() {
        let lerp = NavigationRailThemeData::lerp(
            Some(&NavigationRailThemeData::new()),
            Some(&NavigationRailThemeData::new()),
            0.5,
        )
        .expect("both given");

        assert!(lerp.selected_icon_theme.is_none());
        assert!(lerp.unselected_icon_theme.is_none());
    }

    #[test]
    fn navigation_rail_theme_data_lerps_each_field() {
        let a = NavigationRailThemeData::new()
            .elevation(2.0)
            .min_width(60.0)
            .group_alignment(-1.0)
            .label_type(NavigationRailLabelType::None)
            .use_indicator(false)
            .selected_icon_theme(IconThemeData::new().size(20.0));
        let b = NavigationRailThemeData::new()
            .elevation(6.0)
            .min_width(100.0)
            .group_alignment(1.0)
            .label_type(NavigationRailLabelType::All)
            .use_indicator(true);

        let halfway = NavigationRailThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.min_width, Some(80.0));
        assert_eq!(halfway.group_alignment, Some(0.0));
        assert_eq!(halfway.label_type, Some(NavigationRailLabelType::All));
        assert_eq!(halfway.use_indicator, Some(true));
        assert_eq!(
            halfway.selected_icon_theme,
            Some(IconThemeData::lerp(
                a.selected_icon_theme.as_ref(),
                None,
                0.5
            ))
        );
        assert!(halfway.unselected_icon_theme.is_none());

        let early = NavigationRailThemeData::lerp(Some(&a), Some(&b), 0.25).expect("both given");
        assert_eq!(early.label_type, Some(NavigationRailLabelType::None));
        assert_eq!(early.use_indicator, Some(false));
    }

    #[test]
    fn theme_data_holds_the_navigation_rail_theme() {
        test_widgets(|tester| {
            let app = tester.cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.navigation_rail_theme, NavigationRailThemeData::new());

            let wide = ThemeData::builder()
                .navigation_rail_theme(NavigationRailThemeData::new().min_width(100.0))
                .build(&app);
            assert_eq!(wide.navigation_rail_theme.min_width, Some(100.0));
            assert_ne!(theme, wide);
            assert_eq!(
                theme
                    .copy_with()
                    .navigation_rail_theme(wide.navigation_rail_theme.clone())
                    .build(),
                wide
            );

            let narrow = ThemeData::builder()
                .navigation_rail_theme(NavigationRailThemeData::new().min_width(60.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&narrow, &wide, 0.5)
                    .navigation_rail_theme
                    .min_width,
                Some(80.0)
            );
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: NavigationRailThemeData| {
            NavigationRailTheme::new(data, waft_widgets::SizedBox::shrink())
        };
        let old = theme(NavigationRailThemeData::new().min_width(1.0));

        assert!(!theme(NavigationRailThemeData::new().min_width(1.0)).update_should_notify(&old));
        assert!(theme(NavigationRailThemeData::new().min_width(2.0)).update_should_notify(&old));
    }
}

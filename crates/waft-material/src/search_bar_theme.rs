//! Flutter counterpart: material_ui's `search_bar_theme.dart`.

use std::fmt::{self, Debug};

use waft_embedder::{Color, TextCapitalization, lerp_double};
use waft_foundation::App;
use waft_painting::{BorderSide, EdgeInsetsGeometry, OutlinedBorderRef, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetStateBorderSide,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `SearchBar` widgets.
///
/// Descendant widgets obtain the current [`SearchBarThemeData`] object using
/// [`SearchBarTheme::of`]. Instances of [`SearchBarThemeData`] can be customized with
/// [`SearchBarThemeData::copy_with`].
///
/// Typically a [`SearchBarThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::search_bar_theme`.
///
/// All [`SearchBarThemeData`] properties are `None` by default. When null, the `SearchBar`
/// will use the values from `ThemeData` if they exist, otherwise it will provide its own
/// defaults based on the overall `Theme`'s colorScheme. See the individual `SearchBar`
/// properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the application.
///  * `SearchViewThemeData`, which describes the theme for the search view of a
///    `SearchAnchor`.
#[derive(Clone, Default)]
pub struct SearchBarThemeData {
    /// Overrides the default value of the `SearchBar::elevation`.
    pub elevation: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// Overrides the default value of the `SearchBar::background_color`.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of the `SearchBar::shadow_color`.
    pub shadow_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of the `SearchBar::surface_tint_color`.
    pub surface_tint_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of the `SearchBar::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Overrides the default value of the `SearchBar::side`.
    pub side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,

    /// Overrides the default value of the `SearchBar::shape`.
    pub shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,

    /// Overrides the default value for `SearchBar::padding`.
    pub padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,

    /// Overrides the default value for `SearchBar::text_style`.
    pub text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// Overrides the default value for `SearchBar::hint_style`.
    pub hint_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// Overrides the value of size constraints for `SearchBar`.
    pub constraints: Option<BoxConstraints>,

    /// Overrides the value of `SearchBar::text_capitalization`.
    pub text_capitalization: Option<TextCapitalization>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `SearchBarThemeData`: the theme data answers with its fields, and
    /// `_SearchBarDefaultsM3(context)` computes the ones it overrides when they are read.
    pub trait SearchBarThemeDataGetters for SearchBarThemeData, fields: search_bar_theme_data {
        elevation: Option<WidgetStatePropertyRef<Option<f64>>>,
        background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        shadow_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        surface_tint_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,
        shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
        padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,
        text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
        hint_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
        constraints: Option<BoxConstraints>,
        text_capitalization: Option<TextCapitalization>,
    }
}

impl SearchBarThemeData {
    /// Creates a theme that can be used for `ThemeData::search_bar_theme`.
    pub fn new() -> SearchBarThemeData {
        SearchBarThemeData::default()
    }

    /// Dart `SearchBarThemeData(elevation:)`.
    pub fn elevation(
        mut self,
        elevation: WidgetStatePropertyRef<Option<f64>>,
    ) -> SearchBarThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SearchBarThemeData(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBarThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SearchBarThemeData(shadowColor:)`.
    pub fn shadow_color(
        mut self,
        shadow_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBarThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `SearchBarThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(
        mut self,
        surface_tint_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBarThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `SearchBarThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBarThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `SearchBarThemeData(side:)`.
    pub fn side(mut self, side: WidgetStatePropertyRef<Option<BorderSide>>) -> SearchBarThemeData {
        self.side = Some(side);
        self
    }

    /// Dart `SearchBarThemeData(shape:)`.
    pub fn shape(
        mut self,
        shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>>,
    ) -> SearchBarThemeData {
        self.shape = Some(shape);
        self
    }

    /// Dart `SearchBarThemeData(padding:)`.
    pub fn padding(
        mut self,
        padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>,
    ) -> SearchBarThemeData {
        self.padding = Some(padding);
        self
    }

    /// Dart `SearchBarThemeData(textStyle:)`.
    pub fn text_style(
        mut self,
        text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchBarThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `SearchBarThemeData(hintStyle:)`.
    pub fn hint_style(
        mut self,
        hint_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchBarThemeData {
        self.hint_style = Some(hint_style);
        self
    }

    /// Dart `SearchBarThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> SearchBarThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `SearchBarThemeData(textCapitalization:)`.
    pub fn text_capitalization(
        mut self,
        text_capitalization: TextCapitalization,
    ) -> SearchBarThemeData {
        self.text_capitalization = Some(text_capitalization);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new values:
    /// `data.copy_with().elevation(..)`.
    pub fn copy_with(&self) -> SearchBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`SearchBarThemeData`].
    pub fn lerp(
        a: Option<&SearchBarThemeData>,
        b: Option<&SearchBarThemeData>,
        t: f64,
    ) -> Option<SearchBarThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(SearchBarThemeData {
            elevation: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.elevation.clone()),
                b.and_then(|b| b.elevation.clone()),
                t,
                lerp_double,
            ),
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
            overlay_color: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
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
                |a, b, t| OutlinedBorderRef::lerp(a.as_ref(), b.as_ref(), t),
            ),
            padding: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.padding.clone()),
                b.and_then(|b| b.padding.clone()),
                t,
                EdgeInsetsGeometry::lerp,
            ),
            text_style: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.text_style.clone()),
                b.and_then(|b| b.text_style.clone()),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            ),
            hint_style: <dyn WidgetStateProperty<Option<_>>>::lerp_optional(
                a.and_then(|a| a.hint_style.clone()),
                b.and_then(|b| b.hint_style.clone()),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            ),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
            text_capitalization: if t < 0.5 {
                a.and_then(|a| a.text_capitalization)
            } else {
                b.and_then(|b| b.text_capitalization)
            },
        })
    }
}

impl PartialEq for SearchBarThemeData {
    fn eq(&self, other: &SearchBarThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.elevation == self.elevation
            && other.background_color == self.background_color
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.overlay_color == self.overlay_color
            && other.side == self.side
            && other.shape == self.shape
            && other.padding == self.padding
            && other.text_style == self.text_style
            && other.hint_style == self.hint_style
            && other.constraints == self.constraints
            && other.text_capitalization == self.text_capitalization
    }
}

impl Debug for SearchBarThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("SearchBarThemeData");
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
            elevation,
            background_color,
            shadow_color,
            surface_tint_color,
            overlay_color,
            side,
            shape,
            padding,
            text_style,
            hint_style,
            constraints,
            text_capitalization,
        );
        properties.finish()
    }
}

/// Applies a search bar theme to descendant `SearchBar` widgets.
///
/// Descendant widgets obtain the current theme's [`SearchBarTheme`] object using
/// [`SearchBarTheme::of`]. When a widget uses [`SearchBarTheme::of`], it is automatically
/// rebuilt if the theme later changes.
///
/// A search bar theme can be specified as part of the overall Material theme using
/// `ThemeData::search_bar_theme`.
///
/// See also:
///
///  * [`SearchBarThemeData`], which describes the actual configuration of a search bar
///    theme.
#[derive(Debug)]
pub struct SearchBarTheme {
    pub key: Option<KeyRef>,
    /// Specifies the default color, shape, and text style values for descendant `SearchBar`
    /// widgets.
    pub data: SearchBarThemeData,
    pub child: WidgetRef,
}

impl SearchBarTheme {
    /// Creates a search bar theme that controls the configurations for descendant
    /// `SearchBar` widgets.
    pub fn new<K>(data: SearchBarThemeData, child: impl IntoWidget<K>) -> SearchBarTheme {
        SearchBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SearchBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SearchBarTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`SearchBarThemeData`] from the closest ancestor [`SearchBarTheme`].
    ///
    /// If there is no enclosing [`SearchBarTheme`] widget, then
    /// `ThemeData::search_bar_theme` is used.
    pub fn of(app: &mut App, context: BuildContext) -> SearchBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<SearchBarTheme>(app) {
            Some(search_bar_theme) => search_bar_theme.data.clone(),
            None => Theme::of(app, context).search_bar_theme.clone(),
        }
    }
}

impl InheritedWidget for SearchBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SearchBarTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Size;
    use waft_painting::{BorderRadiusGeometry, EdgeInsets, RoundedRectangleBorder};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        Builder, Center, EditableText, Icon, SizedBox, Text, WidgetState, WidgetStatePropertyAll,
        WidgetStates,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::icons::Icons;
    use crate::ink_well::InkWell;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::search_anchor::SearchBar;
    use crate::test_support::material_app;
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;
    use std::rc::Rc;

    // search_bar_theme_test.dart 'SearchBarThemeData copyWith, ==, hashCode basics'.
    #[test]
    fn search_bar_theme_data_copy_with_equals_basics() {
        assert_eq!(
            SearchBarThemeData::new(),
            SearchBarThemeData::new().copy_with()
        );
    }

    // search_bar_theme_test.dart 'SearchBarThemeData lerp special cases'.
    #[test]
    fn search_bar_theme_data_lerp_special_cases() {
        assert_eq!(SearchBarThemeData::lerp(None, None, 0.0), None);
        let data = SearchBarThemeData::new();
        assert_eq!(
            SearchBarThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // search_bar_theme_test.dart 'SearchBarThemeData defaults'.
    #[test]
    fn search_bar_theme_data_defaults_are_all_null() {
        let theme_data = SearchBarThemeData::new();
        assert!(theme_data.elevation.is_none());
        assert!(theme_data.background_color.is_none());
        assert!(theme_data.shadow_color.is_none());
        assert!(theme_data.surface_tint_color.is_none());
        assert!(theme_data.overlay_color.is_none());
        assert!(theme_data.side.is_none());
        assert!(theme_data.shape.is_none());
        assert!(theme_data.padding.is_none());
        assert!(theme_data.text_style.is_none());
        assert!(theme_data.hint_style.is_none());
        assert_eq!(theme_data.constraints, None);
        assert_eq!(theme_data.text_capitalization, None);

        let theme = SearchBarTheme::new(SearchBarThemeData::new(), SizedBox::new());
        assert!(theme.data.elevation.is_none());
        assert!(theme.data.background_color.is_none());
        assert!(theme.data.shadow_color.is_none());
        assert!(theme.data.surface_tint_color.is_none());
        assert!(theme.data.overlay_color.is_none());
        assert!(theme.data.side.is_none());
        assert!(theme.data.shape.is_none());
        assert!(theme.data.padding.is_none());
        assert!(theme.data.text_style.is_none());
        assert!(theme.data.hint_style.is_none());
        assert_eq!(theme.data.constraints, None);
        assert_eq!(theme.data.text_capitalization, None);
    }

    const ELEVATION_VALUE: f64 = 5.0;
    const BACKGROUND_COLOR_VALUE: Color = Color::new(0xff000001);
    const SHADOW_COLOR_VALUE: Color = Color::new(0xff000001);
    const SURFACE_TINT_COLOR_VALUE: Color = Color::new(0xff000001);
    const OVERLAY_COLOR_VALUE: Color = Color::new(0xff000001);

    fn side_value() -> BorderSide {
        BorderSide {
            color: Color::new(0xff000004),
            width: 2.0,
            ..BorderSide::default()
        }
    }

    fn shape_value() -> OutlinedBorderRef {
        OutlinedBorderRef::new(RoundedRectangleBorder::new(
            side_value(),
            BorderRadiusGeometry::circular(2.0),
        ))
    }

    fn all<T: Clone + std::fmt::Debug + PartialEq + 'static>(
        value: T,
    ) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    fn text_style_value() -> TextStyle {
        TextStyle::new()
            .color(Color::new(0xff000005))
            .font_size(20.0)
    }

    fn hint_style_value() -> TextStyle {
        TextStyle::new()
            .color(Color::new(0xff000006))
            .font_size(18.0)
    }

    fn constraints_value() -> BoxConstraints {
        BoxConstraints {
            min_width: 250.0,
            max_width: 300.0,
            min_height: 80.0,
            ..BoxConstraints::new()
        }
    }

    fn search_bar_theme() -> SearchBarThemeData {
        SearchBarThemeData::new()
            .elevation(all(ELEVATION_VALUE))
            .background_color(all(BACKGROUND_COLOR_VALUE))
            .shadow_color(all(SHADOW_COLOR_VALUE))
            .surface_tint_color(all(SURFACE_TINT_COLOR_VALUE))
            .overlay_color(all(OVERLAY_COLOR_VALUE))
            .side(all(side_value()))
            .shape(all(shape_value()))
            .padding(all(EdgeInsetsGeometry::from(EdgeInsets::symmetric(
                0.0, 16.0,
            ))))
            .text_style(all(text_style_value()))
            .hint_style(all(hint_style_value()))
            .constraints(constraints_value())
            .text_capitalization(TextCapitalization::Words)
    }

    fn themed_bar(use_search_bar_properties: bool) -> WidgetRef {
        let bar = SearchBar::new()
            .hint_text("hint")
            .leading(Icon::new(Some(Icons::search())))
            .trailing(vec![Icon::new(Some(Icons::menu())).into_widget()]);
        let bar = if use_search_bar_properties {
            let theme = search_bar_theme();
            let mut bar = bar;
            bar.elevation = theme.elevation;
            bar.background_color = theme.background_color;
            bar.shadow_color = theme.shadow_color;
            bar.surface_tint_color = theme.surface_tint_color;
            bar.overlay_color = theme.overlay_color;
            bar.side = theme.side;
            bar.shape = theme.shape;
            bar.padding = theme.padding;
            bar.text_style = theme.text_style;
            bar.hint_style = theme.hint_style;
            bar.constraints = theme.constraints;
            bar.text_capitalization = theme.text_capitalization;
            bar
        } else {
            bar
        };
        bar.into_widget()
    }

    /// Dart's `buildFrame`.
    fn build_frame(
        tester: &WidgetTester,
        use_search_bar_properties: bool,
        search_bar_theme_data: Option<SearchBarThemeData>,
        overall_theme: Option<SearchBarThemeData>,
    ) -> WidgetRef {
        let child = Builder::new(move |_app, _context| themed_bar(use_search_bar_properties));
        let theme = {
            let app = tester.cell.borrow();
            let theme = ThemeData::from(ColorScheme::light()).build(&app);
            match overall_theme {
                Some(overall_theme) => theme.copy_with().search_bar_theme(overall_theme).build(),
                None => theme,
            }
        };
        material_app(
            theme,
            Scaffold::new()
                .body(Center::new().child(
                    // If the SearchBarThemeData widget is present, it's used instead of the
                    // Theme's ThemeData.searchBarTheme.
                    match search_bar_theme_data {
                        None => child.into_widget(),
                        Some(data) => SearchBarTheme::new(data, child).into_widget(),
                    },
                ))
                .into_widget(),
        )
    }

    fn check_search_bar(tester: &mut WidgetTester) {
        let find_material =
            find::descendant(find::by_type::<SearchBar>(), find::by_type::<Material>());
        let find_ink_well =
            find::descendant(find::by_type::<SearchBar>(), find::by_type::<InkWell>());
        let (elevation, color, shadow_color, surface_tint_color, shape) =
            tester.widget(find_material, |material: &Material| {
                (
                    material.elevation,
                    material.color,
                    material.shadow_color,
                    material.surface_tint_color,
                    material.shape.clone(),
                )
            });
        assert_eq!(elevation, ELEVATION_VALUE);
        assert_eq!(color, Some(BACKGROUND_COLOR_VALUE));
        assert_eq!(shadow_color, Some(SHADOW_COLOR_VALUE));
        assert_eq!(surface_tint_color, Some(SURFACE_TINT_COLOR_VALUE));
        assert_eq!(shape, Some(shape_value().into()));
        let (overlay_color, custom_border) = tester.widget(find_ink_well, |ink_well: &InkWell| {
            (
                ink_well.ink_response.overlay_color.clone(),
                ink_well.ink_response.custom_border.clone(),
            )
        });
        let overlay_color = overlay_color.expect("an overlay color");
        for state in [
            WidgetState::Hovered,
            WidgetState::Focused,
            WidgetState::Pressed,
        ] {
            let states: WidgetStates = [state].into_iter().collect();
            assert_eq!(overlay_color.resolve(&states), Some(OVERLAY_COLOR_VALUE));
        }
        assert_eq!(custom_border, Some(shape_value().into()));

        assert_eq!(
            tester.get_size(find::by_type::<SearchBar>()),
            Size::new(300.0, 80.0)
        );

        let hint_style = tester
            .widget(find::text("hint"), |text: &Text| text.style.clone())
            .expect("a hint style");
        assert_eq!(
            hint_style.color.as_ref().map(|color| color.color()),
            hint_style_value().color.as_ref().map(|color| color.color())
        );
        assert_eq!(hint_style.font_size, hint_style_value().font_size);

        tester.enter_text(find::by_type::<TextField>(), "input");
        let (input_style, text_capitalization) = tester
            .widget(find::text("input"), |text: &EditableText| {
                (text.style.clone(), text.text_capitalization)
            });
        assert_eq!(
            input_style.color.as_ref().map(|color| color.color()),
            text_style_value().color.as_ref().map(|color| color.color())
        );
        assert_eq!(input_style.font_size, text_style_value().font_size);
        assert_eq!(text_capitalization, TextCapitalization::Words);

        let bar_rect = tester.get_rect(find::by_type::<SearchBar>());
        let leading_rect = tester.get_rect(find::by_icon(&Icons::search()));
        let text_field_rect = tester.get_rect(find::by_type::<TextField>());
        let trailing_rect = tester.get_rect(find::by_icon(&Icons::menu()));

        assert_eq!(bar_rect.left, leading_rect.left - 16.0);
        assert_eq!(leading_rect.right, text_field_rect.left - 16.0);
        assert_eq!(text_field_rect.right, trailing_rect.left - 16.0);
        assert_eq!(trailing_rect.right, bar_rect.right - 16.0);
    }

    /// A theme argument of Dart's `buildFrame`: absent, empty or the full theme (built inside
    /// the case, because a case runs on its own thread).
    #[derive(Clone, Copy)]
    enum ThemeArgument {
        Null,
        Empty,
        Full,
    }

    impl ThemeArgument {
        fn data(self) -> Option<SearchBarThemeData> {
            match self {
                ThemeArgument::Null => None,
                ThemeArgument::Empty => Some(SearchBarThemeData::new()),
                ThemeArgument::Full => Some(search_bar_theme()),
            }
        }
    }

    fn pump_and_check(
        use_search_bar_properties: bool,
        search_bar_theme_data: ThemeArgument,
        overall_theme: ThemeArgument,
    ) {
        test_widgets(move |tester| {
            let root = build_frame(
                tester,
                use_search_bar_properties,
                search_bar_theme_data.data(),
                overall_theme.data(),
            );
            tester.pump_widget(root);
            tester.pump_and_settle(); // allow the animations to finish
            check_search_bar(tester);
        });
    }

    // search_bar_theme_test.dart 'SearchBar properties overrides defaults'.
    #[test]
    fn search_bar_properties_override_defaults() {
        pump_and_check(true, ThemeArgument::Null, ThemeArgument::Null);
    }

    // search_bar_theme_test.dart 'SearchBar theme data overrides defaults'.
    #[test]
    fn search_bar_theme_data_overrides_defaults() {
        pump_and_check(false, ThemeArgument::Full, ThemeArgument::Null);
    }

    // search_bar_theme_test.dart 'Overall Theme SearchBar theme overrides defaults'.
    #[test]
    fn overall_theme_search_bar_theme_overrides_defaults() {
        pump_and_check(false, ThemeArgument::Null, ThemeArgument::Full);
    }

    // search_bar_theme_test.dart 'SearchBar properties overrides defaults, empty theme and
    // overall theme'.
    #[test]
    fn search_bar_properties_override_defaults_with_empty_theme_and_overall_theme() {
        pump_and_check(true, ThemeArgument::Empty, ThemeArgument::Empty);
    }

    // search_bar_theme_test.dart 'SearchBar theme overrides defaults and overall theme'.
    #[test]
    fn search_bar_theme_overrides_defaults_and_overall_theme() {
        pump_and_check(false, ThemeArgument::Full, ThemeArgument::Empty);
    }

    // search_bar_theme_test.dart 'Overall Theme SearchBar theme overrides defaults and null
    // theme'.
    #[test]
    fn overall_theme_search_bar_theme_overrides_defaults_and_null_theme() {
        pump_and_check(false, ThemeArgument::Null, ThemeArgument::Full);
    }
}

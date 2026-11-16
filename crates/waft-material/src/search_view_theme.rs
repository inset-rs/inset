//! Flutter counterpart: material_ui's `search_view_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyBorderSide, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateBorderSide, WidgetStateProperty, WidgetStates,
};

use crate::theme::Theme;

/// Defines the configuration of the search views created by the `SearchAnchor` widget.
///
/// Descendant widgets obtain the current [`SearchViewThemeData`] object using
/// [`SearchViewTheme::of`]. Instances of [`SearchViewThemeData`] can be customized with
/// [`SearchViewThemeData::copy_with`].
///
/// Typically a [`SearchViewThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::search_view_theme`.
///
/// All [`SearchViewThemeData`] properties are `None` by default. When null, the
/// `SearchAnchor` will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults based on the overall `Theme`'s colorScheme. See the individual
/// `SearchAnchor` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the application.
///  * `SearchBarThemeData`, which describes the theme for the search bar itself in a
///    `SearchBar` widget.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchViewThemeData {
    /// Overrides the default value of the `SearchAnchor::view_background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of the `SearchAnchor::view_elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value surfaceTintColor of the `SearchAnchor` view.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value of the `SearchAnchor::view_side`.
    pub side: Option<AnyBorderSide>,

    /// Overrides the default value of the `SearchAnchor::view_shape`.
    pub shape: Option<OutlinedBorderRef>,

    /// Overrides the value of the `SearchAnchor::header_height`.
    pub header_height: Option<f64>,

    /// Overrides the default value of the `SearchAnchor::header_text_style`.
    pub header_text_style: Option<TextStyle>,

    /// Overrides the default value of the `SearchAnchor::header_hint_style`.
    pub header_hint_style: Option<TextStyle>,

    /// Overrides the value of size constraints for the search view.
    pub constraints: Option<BoxConstraints>,

    /// Overrides the value of the padding for the search view.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Overrides the value of the padding for the search bar of the search view.
    pub bar_padding: Option<EdgeInsetsGeometry>,

    /// If true, the search view will be as tall as its content, or the available height if
    /// that is smaller.
    pub shrink_wrap: Option<bool>,

    /// Overrides the default value of the `SearchAnchor::divider_color`.
    pub divider_color: Option<Color>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `SearchViewThemeData`: the theme data answers with its fields, and
    /// `_SearchViewDefaultsM3(context, isFullScreen:)` computes the ones it overrides when
    /// they are read.
    pub trait SearchViewThemeDataGetters for SearchViewThemeData, fields: search_view_theme_data {
        background_color: Option<Color>,
        elevation: Option<f64>,
        surface_tint_color: Option<Color>,
        side: Option<AnyBorderSide>,
        shape: Option<OutlinedBorderRef>,
        header_height: Option<f64>,
        header_text_style: Option<TextStyle>,
        header_hint_style: Option<TextStyle>,
        constraints: Option<BoxConstraints>,
        padding: Option<EdgeInsetsGeometry>,
        bar_padding: Option<EdgeInsetsGeometry>,
        shrink_wrap: Option<bool>,
        divider_color: Option<Color>,
    }
}

impl SearchViewThemeData {
    /// Creates a theme that can be used for `ThemeData::search_view_theme`.
    pub fn new() -> SearchViewThemeData {
        SearchViewThemeData::default()
    }

    /// Dart `SearchViewThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> SearchViewThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SearchViewThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> SearchViewThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SearchViewThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> SearchViewThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `SearchViewThemeData(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> SearchViewThemeData {
        self.side = Some(side.into());
        self
    }

    /// Dart `SearchViewThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> SearchViewThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `SearchViewThemeData(headerHeight:)`.
    pub fn header_height(mut self, header_height: f64) -> SearchViewThemeData {
        self.header_height = Some(header_height);
        self
    }

    /// Dart `SearchViewThemeData(headerTextStyle:)`.
    pub fn header_text_style(mut self, header_text_style: TextStyle) -> SearchViewThemeData {
        self.header_text_style = Some(header_text_style);
        self
    }

    /// Dart `SearchViewThemeData(headerHintStyle:)`.
    pub fn header_hint_style(mut self, header_hint_style: TextStyle) -> SearchViewThemeData {
        self.header_hint_style = Some(header_hint_style);
        self
    }

    /// Dart `SearchViewThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> SearchViewThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `SearchViewThemeData(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> SearchViewThemeData {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `SearchViewThemeData(barPadding:)`.
    pub fn bar_padding(
        mut self,
        bar_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SearchViewThemeData {
        self.bar_padding = Some(bar_padding.into());
        self
    }

    /// Dart `SearchViewThemeData(shrinkWrap:)`.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> SearchViewThemeData {
        self.shrink_wrap = Some(shrink_wrap);
        self
    }

    /// Dart `SearchViewThemeData(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> SearchViewThemeData {
        self.divider_color = Some(divider_color);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new values:
    /// `data.copy_with().elevation(..)`.
    pub fn copy_with(&self) -> SearchViewThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`SearchViewThemeData`].
    pub fn lerp(
        a: Option<&SearchViewThemeData>,
        b: Option<&SearchViewThemeData>,
        t: f64,
    ) -> Option<SearchViewThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(SearchViewThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            side: Self::lerp_sides(
                a.and_then(|a| a.side.as_ref()),
                b.and_then(|b| b.side.as_ref()),
                t,
            )
            .map(AnyBorderSide::new),
            shape: OutlinedBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            header_height: lerp_double(
                a.and_then(|a| a.header_height),
                b.and_then(|b| b.header_height),
                t,
            ),
            header_text_style: TextStyle::lerp(
                a.and_then(|a| a.header_text_style.as_ref()),
                b.and_then(|b| b.header_text_style.as_ref()),
                t,
            ),
            // Dart reads the header text styles here, not the hint styles.
            header_hint_style: TextStyle::lerp(
                a.and_then(|a| a.header_text_style.as_ref()),
                b.and_then(|b| b.header_text_style.as_ref()),
                t,
            ),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            bar_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.bar_padding),
                b.and_then(|b| b.bar_padding),
                t,
            ),
            shrink_wrap: if t < 0.5 {
                a.and_then(|a| a.shrink_wrap)
            } else {
                b.and_then(|b| b.shrink_wrap)
            },
            divider_color: Color::lerp(
                a.and_then(|a| a.divider_color),
                b.and_then(|b| b.divider_color),
                t,
            ),
        })
    }

    /// Special case because `BorderSide::lerp` doesn't support null arguments.
    fn lerp_sides(
        a: Option<&AnyBorderSide>,
        b: Option<&AnyBorderSide>,
        t: f64,
    ) -> Option<BorderSide> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.and_then(Self::resolve_unconditionally);
        let b = b.and_then(Self::resolve_unconditionally);
        let a = a.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: b.expect("b! in Dart").color.with_alpha(0),
            ..BorderSide::default()
        });
        let b = b.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: a.color.with_alpha(0),
            ..BorderSide::default()
        });
        Some(BorderSide::lerp(a, b, t))
    }

    /// Dart's `if (a is WidgetStateBorderSide) { a = a.resolve(const <WidgetState>{}); }`.
    fn resolve_unconditionally(side: &AnyBorderSide) -> Option<BorderSide> {
        match side.extension::<WidgetStateBorderSide>() {
            Some(side) => side.resolve(&WidgetStates::new()),
            None => Some(**side),
        }
    }
}

/// An inherited widget that defines the visual properties for `SearchAnchor`s' search views
/// in this widget's subtree.
///
/// Values specified here are used for `SearchAnchor`'s view properties that are not given an
/// explicit non-null value.
///
/// See also:
///
///  * [`SearchViewThemeData`], which describes the actual configuration of a search view
///    theme.
#[derive(Debug)]
pub struct SearchViewTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `SearchAnchor` widgets.
    pub data: SearchViewThemeData,
    pub child: WidgetRef,
}

impl SearchViewTheme {
    /// Creates a const theme that controls the configurations for the search views in its
    /// widget subtree.
    pub fn new<K>(data: SearchViewThemeData, child: impl IntoWidget<K>) -> SearchViewTheme {
        SearchViewTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SearchViewTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SearchViewTheme {
        self.key = Some(key);
        self
    }

    /// Returns the closest [`SearchViewThemeData`] instance given the build context. If there
    /// is no enclosing [`SearchViewTheme`], it returns `ThemeData::search_view_theme`.
    pub fn of(app: &mut App, context: BuildContext) -> SearchViewThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<SearchViewTheme>(app) {
            Some(search_view_theme) => search_view_theme.data.clone(),
            None => Theme::of(app, context).search_view_theme.clone(),
        }
    }
}

impl InheritedTheme for SearchViewTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        SearchViewTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for SearchViewTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SearchViewTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{TextAffinity, TextEditingValue, TextSelection};
    use waft_foundation::CompleterFuture;
    use waft_painting::{BorderRadiusGeometry, RoundedRectangleBorder};
    use waft_services::TextInputClient;
    use waft_widgets::testing::{Finder, WidgetTester, find, test_widgets};
    use waft_widgets::{
        Builder, Center, ConstrainedBox, EditableText, EditableTextState, Icon, SizedBox, Text,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::search_anchor::{SearchAnchor, SearchBar, ViewContent};
    use crate::test_support::material_app;
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;

    // search_view_theme_test.dart 'SearchViewThemeData copyWith, ==, hashCode basics'.
    #[test]
    fn search_view_theme_data_copy_with_equals_basics() {
        assert_eq!(
            SearchViewThemeData::new(),
            SearchViewThemeData::new().copy_with()
        );
    }

    // search_view_theme_test.dart 'SearchViewThemeData lerp special cases'.
    #[test]
    fn search_view_theme_data_lerp_special_cases() {
        assert_eq!(SearchViewThemeData::lerp(None, None, 0.0), None);
        let data = SearchViewThemeData::new();
        assert_eq!(
            SearchViewThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    // search_view_theme_test.dart 'SearchViewThemeData defaults'.
    #[test]
    fn search_view_theme_data_defaults_are_all_null() {
        let theme_data = SearchViewThemeData::new();
        assert_eq!(theme_data.background_color, None);
        assert_eq!(theme_data.elevation, None);
        assert_eq!(theme_data.surface_tint_color, None);
        assert_eq!(theme_data.constraints, None);
        assert_eq!(theme_data.side, None);
        assert_eq!(theme_data.shape, None);
        assert_eq!(theme_data.header_height, None);
        assert_eq!(theme_data.header_text_style, None);
        assert_eq!(theme_data.header_hint_style, None);
        assert_eq!(theme_data.padding, None);
        assert_eq!(theme_data.bar_padding, None);
        assert_eq!(theme_data.shrink_wrap, None);
        assert_eq!(theme_data.divider_color, None);

        let theme = SearchViewTheme::new(SearchViewThemeData::new(), SizedBox::new());
        assert_eq!(theme.data, SearchViewThemeData::new());
    }

    const BACKGROUND_COLOR: Color = Color::new(0xff000001);
    const ELEVATION: f64 = 5.0;
    const SURFACE_TINT_COLOR: Color = Color::new(0xff000002);
    const HEADER_HEIGHT: f64 = 45.0;

    fn side() -> BorderSide {
        BorderSide {
            color: Color::new(0xff000003),
            width: 2.0,
            ..BorderSide::default()
        }
    }

    fn shape() -> OutlinedBorderRef {
        OutlinedBorderRef::new(RoundedRectangleBorder::new(
            side(),
            BorderRadiusGeometry::circular(20.0),
        ))
    }

    fn header_text_style() -> TextStyle {
        TextStyle::new()
            .color(Color::new(0xff000004))
            .font_size(20.0)
    }

    fn header_hint_style() -> TextStyle {
        TextStyle::new()
            .color(Color::new(0xff000005))
            .font_size(18.0)
    }

    fn constraints() -> BoxConstraints {
        BoxConstraints {
            min_width: 250.0,
            max_width: 300.0,
            min_height: 450.0,
            ..BoxConstraints::new()
        }
    }

    fn search_view_theme() -> SearchViewThemeData {
        SearchViewThemeData::new()
            .background_color(BACKGROUND_COLOR)
            .elevation(ELEVATION)
            .surface_tint_color(SURFACE_TINT_COLOR)
            .side(side())
            .shape(shape())
            .header_height(HEADER_HEIGHT)
            .header_text_style(header_text_style())
            .header_hint_style(header_hint_style())
            .constraints(constraints())
    }

    /// A theme argument of Dart's `buildFrame`: absent, empty or the full theme.
    #[derive(Clone, Copy)]
    enum ThemeArgument {
        Null,
        Empty,
        Full,
    }

    impl ThemeArgument {
        fn data(self) -> Option<SearchViewThemeData> {
            match self {
                ThemeArgument::Null => None,
                ThemeArgument::Empty => Some(SearchViewThemeData::new()),
                ThemeArgument::Full => Some(search_view_theme()),
            }
        }
    }

    /// Dart's `buildFrame`.
    fn build_frame(
        tester: &WidgetTester,
        use_search_view_properties: bool,
        search_view_theme_data: Option<SearchViewThemeData>,
        overall_theme: Option<SearchViewThemeData>,
    ) -> WidgetRef {
        let child = Builder::new(move |_app, _context| {
            let mut anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                |_app, _context, _controller| CompleterFuture::ready(Vec::new()),
            )
            .view_hint_text("hint text")
            .is_full_screen(false);
            if use_search_view_properties {
                anchor = anchor
                    .view_elevation(ELEVATION)
                    .view_background_color(BACKGROUND_COLOR)
                    .view_surface_tint_color(SURFACE_TINT_COLOR)
                    .view_side(side())
                    .view_shape(shape())
                    .header_height(HEADER_HEIGHT)
                    .header_text_style(header_text_style())
                    .header_hint_style(header_hint_style())
                    .view_constraints(constraints());
            }
            anchor.into_widget()
        });
        let theme = {
            let app = tester.cell.borrow();
            let theme = ThemeData::from(ColorScheme::light()).build(&app);
            match overall_theme {
                Some(overall_theme) => theme.copy_with().search_view_theme(overall_theme).build(),
                None => theme,
            }
        };
        material_app(
            theme,
            Scaffold::new()
                .body(Center::new().child(
                    // If the SearchViewThemeData widget is present, it's used instead of the
                    // Theme's ThemeData.searchViewTheme.
                    match search_view_theme_data {
                        None => child.into_widget(),
                        Some(data) => SearchViewTheme::new(data, child).into_widget(),
                    },
                ))
                .into_widget(),
        )
    }

    fn find_view_content() -> Finder {
        find::by_type::<ViewContent>()
    }

    fn enter_text(tester: &mut WidgetTester, text: &str) {
        let element = tester.element(find::by_type::<TextField>().first());
        let state = element.state_handle::<EditableTextState>(&tester.cell.borrow());
        let state = match state {
            Some(state) => state,
            None => tester
                .element(find::by_type::<EditableText>().first())
                .state_handle::<EditableTextState>(&tester.cell.borrow())
                .expect("an EditableText state"),
        };
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
        let length = text.encode_utf16().count() as i32;
        let value = TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(length, TextAffinity::Downstream));
        TextInputClient::update_editing_value(state, &mut tester.cell.borrow_mut(), value);
        tester.cell.checkpoint();
    }

    fn check_search_view(tester: &mut WidgetTester) {
        let (elevation, color, surface_tint_color, material_shape) = tester.widget(
            find::descendant(find_view_content(), find::by_type::<Material>()).first(),
            |material: &Material| {
                (
                    material.elevation,
                    material.color,
                    material.surface_tint_color,
                    material.shape.clone(),
                )
            },
        );
        assert_eq!(elevation, ELEVATION);
        assert_eq!(color, Some(BACKGROUND_COLOR));
        assert_eq!(surface_tint_color, Some(SURFACE_TINT_COLOR));
        assert_eq!(material_shape, Some(shape().into()));

        let size = tester.get_size(
            find::descendant(find_view_content(), find::by_type::<ConstrainedBox>()).first(),
        );
        assert_eq!(size.width(), 250.0);
        assert_eq!(size.height(), 450.0);

        let hint_style = tester
            .widget(find::text("hint text"), |text: &Text| text.style.clone())
            .expect("a hint style");
        assert_eq!(
            hint_style.color.as_ref().map(|color| color.color()),
            header_hint_style()
                .color
                .as_ref()
                .map(|color| color.color())
        );
        assert_eq!(hint_style.font_size, header_hint_style().font_size);

        let bar_height = tester
            .get_size(find::descendant(
                find_view_content(),
                find::by_type::<SearchBar>(),
            ))
            .height();
        assert_eq!(bar_height, HEADER_HEIGHT);
        enter_text(tester, "input");
        let input_style = tester.widget(find::text("input"), |text: &EditableText| {
            text.style.clone()
        });
        assert_eq!(
            input_style.color.as_ref().map(|color| color.color()),
            header_text_style()
                .color
                .as_ref()
                .map(|color| color.color())
        );
        assert_eq!(input_style.font_size, header_text_style().font_size);
    }

    fn pump_and_check(
        use_search_view_properties: bool,
        search_view_theme_data: ThemeArgument,
        overall_theme: ThemeArgument,
    ) {
        test_widgets(move |tester| {
            let root = build_frame(
                tester,
                use_search_view_properties,
                search_view_theme_data.data(),
                overall_theme.data(),
            );
            tester.pump_widget(root);
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle(); // allow the animations to finish
            check_search_view(tester);
        });
    }

    // search_view_theme_test.dart 'SearchView properties overrides defaults'.
    #[test]
    fn search_view_properties_override_defaults() {
        pump_and_check(true, ThemeArgument::Null, ThemeArgument::Null);
    }

    // search_view_theme_test.dart 'SearchView theme data overrides defaults'.
    #[test]
    fn search_view_theme_data_overrides_defaults() {
        pump_and_check(false, ThemeArgument::Full, ThemeArgument::Null);
    }

    // search_view_theme_test.dart 'Overall Theme SearchView theme overrides defaults'.
    #[test]
    fn overall_theme_search_view_theme_overrides_defaults() {
        pump_and_check(false, ThemeArgument::Null, ThemeArgument::Full);
    }

    // search_view_theme_test.dart 'SearchView properties overrides defaults, empty theme and
    // overall theme'.
    #[test]
    fn search_view_properties_override_defaults_with_empty_theme_and_overall_theme() {
        pump_and_check(true, ThemeArgument::Empty, ThemeArgument::Empty);
    }

    // search_view_theme_test.dart 'SearchView theme overrides defaults and overall theme'.
    #[test]
    fn search_view_theme_overrides_defaults_and_overall_theme() {
        pump_and_check(false, ThemeArgument::Full, ThemeArgument::Empty);
    }

    // search_view_theme_test.dart 'Overall Theme SearchView theme overrides defaults and null
    // theme'.
    #[test]
    fn overall_theme_search_view_theme_overrides_defaults_and_null_theme() {
        pump_and_check(false, ThemeArgument::Null, ThemeArgument::Full);
    }
}

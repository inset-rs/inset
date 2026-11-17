//! Flutter counterpart: material_ui's `data_table_theme.dart`.

use std::fmt;
use std::rc::Rc;

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{DecorationRef, TextStyle};
use waft_rendering::MainAxisAlignment;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Signature of the builder of a `DataTable`'s sort arrow: whether the arrow is `visible`,
/// and whether the column sorts `ascending`.
pub type DataTableSortIconBuilder = Rc<dyn Fn(&mut App, BuildContext, bool, bool) -> WidgetRef>;

/// Defines default property values for descendant `DataTable` widgets.
///
/// Descendant widgets obtain the current [`DataTableThemeData`] object using
/// [`DataTableTheme::of`]. Instances of [`DataTableThemeData`] can be customized with
/// [`DataTableThemeData::copy_with`].
///
/// Typically a [`DataTableThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::data_table_theme`.
///
/// All [`DataTableThemeData`] properties are `None` by default. When null, the `DataTable`
/// provides its own defaults.
///
/// See also:
///
///  * [`DataTable`](crate::DataTable), which uses this to style itself.
///  * `ThemeData`, which describes the overall theme information for the application.
#[derive(Clone, Default)]
pub struct DataTableThemeData {
    /// The background and borders of the table used by a `DataTable` widget.
    ///
    /// The `DataTable` stacks this decoration with the `Material` it is rendered in.
    pub decoration: Option<DecorationRef>,

    /// The background color of the table's rows.
    ///
    /// If `None`, `DataTable` uses `ThemeData::data_table_theme`'s default instead.
    pub data_row_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The height of each row (excluding the row that contains column headings).
    ///
    /// Dart's `dataRowMinHeight`; `None` lets `DataTable` default it.
    pub data_row_min_height: Option<f64>,

    /// The maximum height of each row (excluding the row that contains column headings).
    pub data_row_max_height: Option<f64>,

    /// The text style for data rows.
    pub data_text_style: Option<TextStyle>,

    /// The background color of the heading row.
    pub heading_row_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The height of the heading row.
    pub heading_row_height: Option<f64>,

    /// The text style for the heading row.
    pub heading_text_style: Option<TextStyle>,

    /// The horizontal margin between the edges of the table and the content in the first and
    /// last cells of each row.
    pub horizontal_margin: Option<f64>,

    /// The horizontal margin between the contents of each data column.
    pub column_spacing: Option<f64>,

    /// The thickness of the divider between rows.
    pub divider_thickness: Option<f64>,

    /// Horizontal margin around the checkbox, if it is displayed.
    pub checkbox_horizontal_margin: Option<f64>,

    /// If specified, overrides the default value of `DataColumn::mouse_cursor`.
    pub heading_cell_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// If specified, overrides the default value of `DataRow::mouse_cursor`.
    pub data_row_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// If specified, overrides the default value of `DataTable::heading_row_alignment`.
    pub heading_row_alignment: Option<MainAxisAlignment>,

    /// If specified, overrides the default value of `DataTable::sort_icon_builder`.
    pub sort_icon_builder: Option<DataTableSortIconBuilder>,
}

impl DataTableThemeData {
    /// Creates a theme that can be used for `ThemeData::data_table_theme` and
    /// [`DataTableTheme`]; Dart's optional named arguments are the fluent setters.
    pub fn new() -> DataTableThemeData {
        DataTableThemeData::default()
    }

    /// Dart `DataTableThemeData(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> DataTableThemeData {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `DataTableThemeData(dataRowColor:)`.
    pub fn data_row_color(
        mut self,
        data_row_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DataTableThemeData {
        self.data_row_color = Some(data_row_color);
        self
    }

    /// Dart's deprecated `DataTableThemeData(dataRowHeight:)`: it sets
    /// [`data_row_min_height`](Self::data_row_min_height) and
    /// [`data_row_max_height`](Self::data_row_max_height) to one value.
    ///
    /// Dart asserts that the argument is not given with either of them in one call; a fluent
    /// chain has no call to compare, so a later setter replaces what an earlier one wrote.
    #[deprecated = "Migrate to use data_row_min_height and data_row_max_height instead. This feature was deprecated after v3.7.0-5.0.pre."]
    pub fn data_row_height(mut self, data_row_height: f64) -> DataTableThemeData {
        self.data_row_min_height = Some(data_row_height);
        self.data_row_max_height = Some(data_row_height);
        self
    }

    /// Dart `DataTableThemeData(dataRowMinHeight:)`.
    pub fn data_row_min_height(mut self, data_row_min_height: f64) -> DataTableThemeData {
        self.data_row_min_height = Some(data_row_min_height);
        self
    }

    /// Dart `DataTableThemeData(dataRowMaxHeight:)`.
    pub fn data_row_max_height(mut self, data_row_max_height: f64) -> DataTableThemeData {
        self.data_row_max_height = Some(data_row_max_height);
        self
    }

    /// Dart `DataTableThemeData(dataTextStyle:)`.
    pub fn data_text_style(mut self, data_text_style: TextStyle) -> DataTableThemeData {
        self.data_text_style = Some(data_text_style);
        self
    }

    /// Dart `DataTableThemeData(headingRowColor:)`.
    pub fn heading_row_color(
        mut self,
        heading_row_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DataTableThemeData {
        self.heading_row_color = Some(heading_row_color);
        self
    }

    /// Dart `DataTableThemeData(headingRowHeight:)`.
    pub fn heading_row_height(mut self, heading_row_height: f64) -> DataTableThemeData {
        self.heading_row_height = Some(heading_row_height);
        self
    }

    /// Dart `DataTableThemeData(headingTextStyle:)`.
    pub fn heading_text_style(mut self, heading_text_style: TextStyle) -> DataTableThemeData {
        self.heading_text_style = Some(heading_text_style);
        self
    }

    /// Dart `DataTableThemeData(horizontalMargin:)`.
    pub fn horizontal_margin(mut self, horizontal_margin: f64) -> DataTableThemeData {
        self.horizontal_margin = Some(horizontal_margin);
        self
    }

    /// Dart `DataTableThemeData(columnSpacing:)`.
    pub fn column_spacing(mut self, column_spacing: f64) -> DataTableThemeData {
        self.column_spacing = Some(column_spacing);
        self
    }

    /// Dart `DataTableThemeData(dividerThickness:)`.
    pub fn divider_thickness(mut self, divider_thickness: f64) -> DataTableThemeData {
        self.divider_thickness = Some(divider_thickness);
        self
    }

    /// Dart `DataTableThemeData(checkboxHorizontalMargin:)`.
    pub fn checkbox_horizontal_margin(
        mut self,
        checkbox_horizontal_margin: f64,
    ) -> DataTableThemeData {
        self.checkbox_horizontal_margin = Some(checkbox_horizontal_margin);
        self
    }

    /// Dart `DataTableThemeData(headingCellCursor:)`.
    pub fn heading_cell_cursor(
        mut self,
        heading_cell_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> DataTableThemeData {
        self.heading_cell_cursor = Some(heading_cell_cursor);
        self
    }

    /// Dart `DataTableThemeData(dataRowCursor:)`.
    pub fn data_row_cursor(
        mut self,
        data_row_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> DataTableThemeData {
        self.data_row_cursor = Some(data_row_cursor);
        self
    }

    /// Dart `DataTableThemeData(headingRowAlignment:)`.
    pub fn heading_row_alignment(
        mut self,
        heading_row_alignment: MainAxisAlignment,
    ) -> DataTableThemeData {
        self.heading_row_alignment = Some(heading_row_alignment);
        self
    }

    /// Dart `DataTableThemeData(sortIconBuilder:)`.
    pub fn sort_icon_builder(
        mut self,
        sort_icon_builder: impl Fn(&mut App, BuildContext, bool, bool) -> WidgetRef + 'static,
    ) -> DataTableThemeData {
        self.sort_icon_builder = Some(Rc::new(sort_icon_builder));
        self
    }

    /// Dart's deprecated `dataRowHeight` getter: the row height, if the minimum and the
    /// maximum are the same.
    #[deprecated = "Migrate to use data_row_min_height and data_row_max_height instead. This feature was deprecated after v3.7.0-5.0.pre."]
    pub fn data_row_height_value(&self) -> Option<f64> {
        if self.data_row_min_height == self.data_row_max_height {
            self.data_row_min_height
        } else {
            None
        }
    }

    /// Creates a copy of this object but with the given fields replaced with the new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().divider_thickness(2.0)`); the deprecated `dataRowHeight` argument
    /// is [`data_row_height`](Self::data_row_height), which replaces both heights.
    pub fn copy_with(&self) -> DataTableThemeData {
        self.clone()
    }

    /// Linearly interpolates between two [`DataTableThemeData`].
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: &DataTableThemeData, b: &DataTableThemeData, t: f64) -> DataTableThemeData {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        DataTableThemeData {
            decoration: DecorationRef::lerp(a.decoration.as_ref(), b.decoration.as_ref(), t),
            data_row_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.data_row_color.clone(),
                b.data_row_color.clone(),
                t,
                Color::lerp,
            ),
            data_row_min_height: lerp_double(a.data_row_min_height, b.data_row_min_height, t),
            data_row_max_height: lerp_double(a.data_row_max_height, b.data_row_max_height, t),
            data_text_style: TextStyle::lerp(
                a.data_text_style.as_ref(),
                b.data_text_style.as_ref(),
                t,
            ),
            heading_row_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.heading_row_color.clone(),
                b.heading_row_color.clone(),
                t,
                Color::lerp,
            ),
            heading_row_height: lerp_double(a.heading_row_height, b.heading_row_height, t),
            heading_text_style: TextStyle::lerp(
                a.heading_text_style.as_ref(),
                b.heading_text_style.as_ref(),
                t,
            ),
            horizontal_margin: lerp_double(a.horizontal_margin, b.horizontal_margin, t),
            column_spacing: lerp_double(a.column_spacing, b.column_spacing, t),
            divider_thickness: lerp_double(a.divider_thickness, b.divider_thickness, t),
            checkbox_horizontal_margin: lerp_double(
                a.checkbox_horizontal_margin,
                b.checkbox_horizontal_margin,
                t,
            ),
            heading_cell_cursor: pick(a, b, t, |theme| theme.heading_cell_cursor.clone()),
            data_row_cursor: pick(a, b, t, |theme| theme.data_row_cursor.clone()),
            heading_row_alignment: pick(a, b, t, |theme| theme.heading_row_alignment),
            sort_icon_builder: pick(a, b, t, |theme| theme.sort_icon_builder.clone()),
        }
    }
}

/// Dart's `t < 0.5 ? a.field : b.field`.
fn pick<T>(
    a: &DataTableThemeData,
    b: &DataTableThemeData,
    t: f64,
    field: impl Fn(&DataTableThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 { field(a) } else { field(b) }
}

impl fmt::Debug for DataTableThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataTableThemeData")
            .field("decoration", &self.decoration)
            .field("data_row_color", &self.data_row_color)
            .field("data_row_min_height", &self.data_row_min_height)
            .field("data_row_max_height", &self.data_row_max_height)
            .field("data_text_style", &self.data_text_style)
            .field("heading_row_color", &self.heading_row_color)
            .field("heading_row_height", &self.heading_row_height)
            .field("heading_text_style", &self.heading_text_style)
            .field("horizontal_margin", &self.horizontal_margin)
            .field("column_spacing", &self.column_spacing)
            .field("divider_thickness", &self.divider_thickness)
            .field(
                "checkbox_horizontal_margin",
                &self.checkbox_horizontal_margin,
            )
            .field("heading_cell_cursor", &self.heading_cell_cursor)
            .field("data_row_cursor", &self.data_row_cursor)
            .field("heading_row_alignment", &self.heading_row_alignment)
            .field("has_sort_icon_builder", &self.sort_icon_builder.is_some())
            .finish()
    }
}

impl PartialEq for DataTableThemeData {
    fn eq(&self, other: &DataTableThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.decoration == self.decoration
            && other.data_row_color == self.data_row_color
            && other.data_row_min_height == self.data_row_min_height
            && other.data_row_max_height == self.data_row_max_height
            && other.data_text_style == self.data_text_style
            && other.heading_row_color == self.heading_row_color
            && other.heading_row_height == self.heading_row_height
            && other.heading_text_style == self.heading_text_style
            && other.horizontal_margin == self.horizontal_margin
            && other.column_spacing == self.column_spacing
            && other.divider_thickness == self.divider_thickness
            && other.checkbox_horizontal_margin == self.checkbox_horizontal_margin
            && other.heading_cell_cursor == self.heading_cell_cursor
            && other.data_row_cursor == self.data_row_cursor
            && other.heading_row_alignment == self.heading_row_alignment
            && match (&other.sort_icon_builder, &self.sort_icon_builder) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// Applies a data table theme to descendant [`DataTable`](crate::DataTable) widgets.
///
/// Descendant widgets obtain the current theme's [`DataTableTheme`] object using
/// [`DataTableTheme::of`]. When a widget uses [`DataTableTheme::of`], it is automatically
/// rebuilt if the theme later changes.
///
/// A data table theme can be specified as part of the overall Material theme using
/// `ThemeData::data_table_theme`.
///
/// See also:
///
///  * [`DataTableThemeData`], which describes the actual configuration of a data table theme.
#[derive(Debug)]
pub struct DataTableTheme {
    pub key: Option<KeyRef>,
    /// Specifies the visual properties used by descendant `DataTable` widgets.
    pub data: DataTableThemeData,
    pub child: WidgetRef,
}

impl DataTableTheme {
    /// Constructs a data table theme that configures all descendant `DataTable` widgets.
    pub fn new<K>(data: DataTableThemeData, child: impl IntoWidget<K>) -> DataTableTheme {
        DataTableTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `DataTableTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DataTableTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`DataTableTheme`]
    /// ancestor. If there is no ancestor, it returns `ThemeData::data_table_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = DataTableTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> DataTableThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<DataTableTheme>(app) {
            Some(data_table_theme) => data_table_theme.data.clone(),
            None => Theme::of(app, context).data_table_theme.clone(),
        }
    }
}

impl InheritedWidget for DataTableTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DataTableTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/data_table_theme_test.dart`.

    use waft_embedder::Color;
    use waft_painting::BoxDecoration;
    use waft_services::SystemMouseCursors;
    use waft_widgets::testing::{
        Finder, TestPointer, WidgetTester, assert_f32_equals, find, test_widgets,
    };
    use waft_widgets::{Container, Table, Text, WidgetStatePropertyAll, WidgetStates};

    use super::*;
    use crate::bottom_navigation_bar::tests::paragraph_style;
    use crate::data_table::{DataCell, DataColumn, DataRow, DataTable};
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn all_color(color: Color) -> WidgetStatePropertyRef<Option<Color>> {
        Rc::new(WidgetStatePropertyAll::new(Some(color)))
    }

    fn all_cursor(cursor: MouseCursorRef) -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        Rc::new(WidgetStatePropertyAll::new(Some(cursor)))
    }

    fn box_decoration(color: Color) -> DecorationRef {
        DecorationRef::new(BoxDecoration::new().color(color))
    }

    /// Dart's `_tableRowBoxDecoration`: the decoration of the table's row `index`.
    fn table_row_box_decoration(tester: &WidgetTester, index: usize) -> BoxDecoration {
        tester.widget(find::by_type::<Table>(), |table: &Table| {
            table.children[index]
                .decoration
                .as_ref()
                .and_then(|decoration| decoration.as_any().downcast_ref::<BoxDecoration>())
                .expect("a BoxDecoration")
                .clone()
        })
    }

    /// Dart's `_findFirstContainerFor`: the finder matches with the `Container` of the cell
    /// content, as well as the `Container` wrapping the whole table. The first one is used to
    /// test row heights.
    fn find_first_container_for(text: &str) -> Finder {
        find::widget_with_text::<Container>(text).first()
    }

    fn table_container_decoration(tester: &WidgetTester) -> Option<DecorationRef> {
        tester.widget(
            find::ancestor(find::by_type::<Table>(), find::by_type::<Container>()).first(),
            |container: &Container| container.decoration.clone(),
        )
    }

    fn font_size_of(tester: &WidgetTester, text: &str) -> Option<f64> {
        paragraph_style(tester, text).font_size
    }

    fn two_column_table(show_checkbox_column: bool) -> DataTable {
        DataTable::new(
            vec![
                DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                DataColumn::new(Text::new("B")),
            ],
            vec![
                DataRow::new(vec![
                    DataCell::new(Text::new("Data")),
                    DataCell::new(Text::new("Data 2")),
                ])
                .on_select_changed(|_app, _selected| {}),
            ],
        )
        .sort_column_index(0)
        .show_checkbox_column(show_checkbox_column)
    }

    fn app_with_theme<K>(
        tester: &WidgetTester,
        data_table_theme: DataTableThemeData,
        body: impl IntoWidget<K>,
    ) -> WidgetRef {
        let theme = ThemeData::builder()
            .data_table_theme(data_table_theme)
            .build(&tester.cell.borrow());
        material_app(theme, Scaffold::new().body(body).into_widget())
    }

    #[test]
    fn data_table_theme_data_copy_with_equals_basics() {
        assert_eq!(
            DataTableThemeData::new(),
            DataTableThemeData::new().copy_with()
        );
    }

    #[test]
    #[expect(deprecated)]
    fn data_table_theme_data_copy_with_data_row_height() {
        let theme_data = DataTableThemeData::new()
            .data_row_min_height(10.0)
            .data_row_max_height(10.0);
        assert_eq!(theme_data, theme_data.copy_with());
        assert_eq!(
            theme_data
                .copy_with()
                .data_row_min_height(20.0)
                .data_row_max_height(20.0),
            DataTableThemeData::new().data_row_height(20.0)
        );
    }

    #[test]
    fn data_table_theme_data_lerp_special_cases() {
        let data = DataTableThemeData::new();
        assert_eq!(DataTableThemeData::lerp(&data, &data, 0.5), data);
    }

    #[test]
    fn data_table_theme_data_defaults() {
        let theme_data = DataTableThemeData::new();
        assert_eq!(theme_data.decoration, None);
        assert!(theme_data.data_row_color.is_none());
        #[expect(deprecated)]
        let data_row_height = theme_data.data_row_height_value();
        assert_eq!(data_row_height, None);
        assert_eq!(theme_data.data_row_min_height, None);
        assert_eq!(theme_data.data_row_max_height, None);
        assert_eq!(theme_data.data_text_style, None);
        assert!(theme_data.heading_row_color.is_none());
        assert_eq!(theme_data.heading_row_height, None);
        assert_eq!(theme_data.heading_text_style, None);
        assert_eq!(theme_data.horizontal_margin, None);
        assert_eq!(theme_data.column_spacing, None);
        assert_eq!(theme_data.divider_thickness, None);
        assert_eq!(theme_data.checkbox_horizontal_margin, None);
        assert!(theme_data.heading_cell_cursor.is_none());
        assert!(theme_data.data_row_cursor.is_none());
        assert_eq!(theme_data.heading_row_alignment, None);
        assert!(theme_data.sort_icon_builder.is_none());
    }

    #[test]
    fn data_table_is_themeable() {
        test_widgets(|tester| {
            let decoration = box_decoration(Color::new(0xfffffff0));
            let data_row_color = all_color(Color::new(0xfffffff1));
            let min_max_data_row_height = 41.0;
            let data_text_style = TextStyle::new().font_size(12.5);
            let heading_row_color = all_color(Color::new(0xfffffff2));
            let heading_row_height = 52.0;
            let heading_text_style = TextStyle::new().font_size(14.5);
            let horizontal_margin = 3.0;
            let column_spacing = 4.0;
            let divider_thickness = 5.0;
            let heading_cell_cursor = all_cursor(SystemMouseCursors::GRAB.into());
            let data_row_cursor = all_cursor(SystemMouseCursors::FORBIDDEN.into());

            let theme = DataTableThemeData::new()
                .decoration(decoration.clone())
                .data_row_color(data_row_color.clone())
                .data_row_min_height(min_max_data_row_height)
                .data_row_max_height(min_max_data_row_height)
                .data_text_style(data_text_style.clone())
                .heading_row_color(heading_row_color.clone())
                .heading_row_height(heading_row_height)
                .heading_text_style(heading_text_style.clone())
                .horizontal_margin(horizontal_margin)
                .column_spacing(column_spacing)
                .divider_thickness(divider_thickness)
                .heading_cell_cursor(heading_cell_cursor)
                .data_row_cursor(data_row_cursor);
            let root = app_with_theme(tester, theme, two_column_table(false));
            tester.pump_widget(root);

            assert_eq!(table_container_decoration(tester), Some(decoration));

            assert_eq!(font_size_of(tester, "Data"), data_text_style.font_size);
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .color
                    .map(|color| color.color()),
                data_row_color.resolve(&WidgetStates::new())
            );
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .border
                    .expect("a border")
                    .top()
                    .width,
                divider_thickness
            );
            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                min_max_data_row_height
            );

            assert_eq!(font_size_of(tester, "A"), heading_text_style.font_size);
            assert_eq!(
                table_row_box_decoration(tester, 0)
                    .color
                    .map(|color| color.color()),
                heading_row_color.resolve(&WidgetStates::new())
            );

            assert_eq!(
                tester.get_size(find_first_container_for("A")).height(),
                heading_row_height
            );
            assert_eq!(tester.get_top_left(find::text("A")).dx(), horizontal_margin);
            assert_eq!(
                tester.get_top_left(find::text("Data 2")).dx()
                    - tester.get_top_right(find::text("Data")).dx(),
                column_spacing
            );

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::text("A"));
            tester.add_pointer(&mut gesture, at);
            tester.pump();

            let grab: MouseCursorRef = SystemMouseCursors::GRAB.into();
            assert!(*tester.active_cursor(1) == *grab);

            let at = tester.get_center(find::text("Data"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();

            let forbidden: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();
            assert!(*tester.active_cursor(1) == *forbidden);
        });
    }

    #[test]
    #[expect(deprecated)]
    fn data_table_is_themeable_separate_test_for_deprecated_data_row_height() {
        test_widgets(|tester| {
            let data_row_height = 51.0;
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![DataRow::new(vec![
                    DataCell::new(Text::new("Data")),
                    DataCell::new(Text::new("Data 2")),
                ])],
            )
            .sort_column_index(0);
            let root = app_with_theme(
                tester,
                DataTableThemeData::new().data_row_height(data_row_height),
                table,
            );
            tester.pump_widget(root);

            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                data_row_height
            );
        });
    }

    #[test]
    fn data_table_properties_are_taken_over_the_theme_values() {
        test_widgets(|tester| {
            let theme_decoration = box_decoration(Color::new(0xfffffff1));
            let theme_data_row_color = all_color(Color::new(0xfffffff0));
            let min_max_theme_data_row_height = 50.0;
            let theme_data_text_style = TextStyle::new().font_size(11.5);
            let theme_heading_row_color = all_color(Color::new(0xfffffff1));
            let theme_heading_row_height = 51.0;
            let theme_heading_text_style = TextStyle::new().font_size(13.5);
            let theme_heading_cell_cursor = all_cursor(SystemMouseCursors::COPY.into());
            let theme_data_row_cursor = all_cursor(SystemMouseCursors::COPY.into());

            let decoration = box_decoration(Color::new(0xfffffff0));
            let data_row_color = all_color(Color::new(0xfffffff1));
            let min_max_data_row_height = 51.0;
            let data_text_style = TextStyle::new().font_size(12.5);
            let heading_row_color = all_color(Color::new(0xfffffff2));
            let heading_row_height = 52.0;
            let heading_text_style = TextStyle::new().font_size(14.5);
            let horizontal_margin = 3.0;
            let column_spacing = 4.0;
            let divider_thickness = 5.0;
            let heading_cell_cursor = all_cursor(SystemMouseCursors::FORBIDDEN.into());
            let data_row_cursor = all_cursor(SystemMouseCursors::FORBIDDEN.into());

            let theme = DataTableThemeData::new()
                .decoration(theme_decoration)
                .data_row_color(theme_data_row_color)
                .data_row_min_height(min_max_theme_data_row_height)
                .data_row_max_height(min_max_theme_data_row_height)
                .data_text_style(theme_data_text_style)
                .heading_row_color(theme_heading_row_color)
                .heading_row_height(theme_heading_row_height)
                .heading_text_style(theme_heading_text_style)
                .horizontal_margin(2.0)
                .column_spacing(3.0)
                .divider_thickness(4.0)
                .heading_cell_cursor(theme_heading_cell_cursor)
                .data_row_cursor(theme_data_row_cursor);
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("A"))
                        .mouse_cursor(heading_cell_cursor)
                        .on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data")),
                        DataCell::new(Text::new("Data 2")),
                    ])
                    .mouse_cursor(data_row_cursor)
                    .on_select_changed(|_app, _selected| {}),
                ],
            )
            .show_checkbox_column(false)
            .decoration(decoration.clone())
            .data_row_color(data_row_color.clone())
            .data_row_min_height(min_max_data_row_height)
            .data_row_max_height(min_max_data_row_height)
            .data_text_style(data_text_style.clone())
            .heading_row_color(heading_row_color.clone())
            .heading_row_height(heading_row_height)
            .heading_text_style(heading_text_style.clone())
            .horizontal_margin(horizontal_margin)
            .column_spacing(column_spacing)
            .divider_thickness(divider_thickness)
            .sort_column_index(0);
            let root = app_with_theme(tester, theme, table);
            tester.pump_widget(root);

            assert_eq!(table_container_decoration(tester), Some(decoration));

            assert_eq!(font_size_of(tester, "Data"), data_text_style.font_size);
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .color
                    .map(|color| color.color()),
                data_row_color.resolve(&WidgetStates::new())
            );
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .border
                    .expect("a border")
                    .top()
                    .width,
                divider_thickness
            );
            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                min_max_data_row_height
            );

            assert_eq!(font_size_of(tester, "A"), heading_text_style.font_size);
            assert_eq!(
                table_row_box_decoration(tester, 0)
                    .color
                    .map(|color| color.color()),
                heading_row_color.resolve(&WidgetStates::new())
            );

            assert_eq!(
                tester.get_size(find_first_container_for("A")).height(),
                heading_row_height
            );
            assert_eq!(tester.get_top_left(find::text("A")).dx(), horizontal_margin);
            assert_eq!(
                tester.get_top_left(find::text("Data 2")).dx()
                    - tester.get_top_right(find::text("Data")).dx(),
                column_spacing
            );

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::text("A"));
            tester.add_pointer(&mut gesture, at);
            tester.pump();
            let forbidden: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();
            assert!(*tester.active_cursor(1) == *forbidden);

            let at = tester.get_center(find::text("Data"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();
            assert!(*tester.active_cursor(1) == *forbidden);
        });
    }

    #[test]
    #[expect(deprecated)]
    fn data_table_properties_are_taken_over_the_theme_values_separate_test_for_deprecated_data_row_height()
     {
        test_widgets(|tester| {
            let data_row_height = 51.0;
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![DataRow::new(vec![
                    DataCell::new(Text::new("Data")),
                    DataCell::new(Text::new("Data 2")),
                ])],
            )
            .data_row_height(data_row_height)
            .sort_column_index(0);
            let root = app_with_theme(
                tester,
                DataTableThemeData::new().data_row_height(50.0),
                table,
            );
            tester.pump_widget(root);

            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                data_row_height
            );
        });
    }

    #[test]
    fn local_data_table_theme_can_override_global_data_table_theme() {
        test_widgets(|tester| {
            let global = DataTableThemeData::new()
                .decoration(box_decoration(Color::new(0xfffffff1)))
                .data_row_color(all_color(Color::new(0xfffffff0)))
                .data_row_min_height(50.0)
                .data_row_max_height(50.0)
                .data_text_style(TextStyle::new().font_size(11.5))
                .heading_row_color(all_color(Color::new(0xfffffff1)))
                .heading_row_height(51.0)
                .heading_text_style(TextStyle::new().font_size(13.5))
                .horizontal_margin(2.0)
                .column_spacing(3.0)
                .divider_thickness(4.0)
                .heading_cell_cursor(all_cursor(SystemMouseCursors::ALL_SCROLL.into()))
                .data_row_cursor(all_cursor(SystemMouseCursors::ALL_SCROLL.into()));

            let local_decoration = box_decoration(Color::new(0xfffffff0));
            let local_data_row_color = all_color(Color::new(0xfffffff1));
            let local_heading_row_color = all_color(Color::new(0xfffffff2));
            let local = DataTableThemeData::new()
                .decoration(local_decoration.clone())
                .data_row_color(local_data_row_color.clone())
                .data_row_min_height(51.0)
                .data_row_max_height(51.0)
                .data_text_style(TextStyle::new().font_size(12.5))
                .heading_row_color(local_heading_row_color.clone())
                .heading_row_height(52.0)
                .heading_text_style(TextStyle::new().font_size(14.5))
                .horizontal_margin(3.0)
                .column_spacing(4.0)
                .divider_thickness(5.0)
                .heading_cell_cursor(all_cursor(SystemMouseCursors::MOVE.into()))
                .data_row_cursor(all_cursor(SystemMouseCursors::MOVE.into()));
            let root = app_with_theme(
                tester,
                global,
                DataTableTheme::new(local, two_column_table(false)),
            );
            tester.pump_widget(root);

            assert_eq!(table_container_decoration(tester), Some(local_decoration));

            assert_eq!(font_size_of(tester, "Data"), Some(12.5));
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .color
                    .map(|color| color.color()),
                local_data_row_color.resolve(&WidgetStates::new())
            );
            assert_eq!(
                table_row_box_decoration(tester, 1)
                    .border
                    .expect("a border")
                    .top()
                    .width,
                5.0
            );
            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                51.0
            );

            assert_eq!(font_size_of(tester, "A"), Some(14.5));
            assert_eq!(
                table_row_box_decoration(tester, 0)
                    .color
                    .map(|color| color.color()),
                local_heading_row_color.resolve(&WidgetStates::new())
            );

            assert_eq!(
                tester.get_size(find_first_container_for("A")).height(),
                52.0
            );
            assert_eq!(tester.get_top_left(find::text("A")).dx(), 3.0);
            assert_eq!(
                tester.get_top_left(find::text("Data 2")).dx()
                    - tester.get_top_right(find::text("Data")).dx(),
                4.0
            );

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::text("A"));
            tester.add_pointer(&mut gesture, at);
            tester.pump();
            let move_cursor: MouseCursorRef = SystemMouseCursors::MOVE.into();
            assert!(*tester.active_cursor(1) == *move_cursor);

            let at = tester.get_center(find::text("Data"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();
            assert!(*tester.active_cursor(1) == *move_cursor);
        });
    }

    #[test]
    #[expect(deprecated)]
    fn local_data_table_theme_can_override_global_data_table_theme_separate_test_for_deprecated_data_row_height()
     {
        test_widgets(|tester| {
            let local_theme_data_row_height = 51.0;
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![DataRow::new(vec![
                    DataCell::new(Text::new("Data")),
                    DataCell::new(Text::new("Data 2")),
                ])],
            )
            .sort_column_index(0);
            let root = app_with_theme(
                tester,
                DataTableThemeData::new().data_row_height(50.0),
                DataTableTheme::new(
                    DataTableThemeData::new().data_row_height(local_theme_data_row_height),
                    table,
                ),
            );
            tester.pump_widget(root);

            assert_eq!(
                tester.get_size(find_first_container_for("Data")).height(),
                local_theme_data_row_height
            );
        });
    }

    #[test]
    fn data_column_label_can_be_centered_with_data_table_theme_heading_row_alignment() {
        test_widgets(|tester| {
            let horizontal_margin = 24.0;
            let build_table = |tester: &WidgetTester,
                               heading_row_alignment: Option<MainAxisAlignment>,
                               sort_enabled: bool|
             -> WidgetRef {
                let mut theme = DataTableThemeData::new();
                theme.heading_row_alignment = heading_row_alignment;
                let mut column = DataColumn::new(Text::new("Header"));
                if sort_enabled {
                    column = column.on_sort(|_app, _index, _ascending| {});
                }
                let table = DataTable::new(
                    vec![column],
                    vec![DataRow::new(vec![DataCell::new(Text::new("Data"))])],
                );
                let theme = ThemeData::builder()
                    .data_table_theme(theme)
                    .build(&tester.cell.borrow());
                material_app(
                    theme,
                    crate::material::Material::new().child(table).into_widget(),
                )
            };

            // Test mainAxisAlignment without sort arrow.
            let root = build_table(tester, None, false);
            tester.pump_widget(root);
            assert_eq!(
                tester.get_top_left(find::text("Header")).dx(),
                horizontal_margin
            );

            // Test mainAxisAlignment.center without sort arrow.
            let root = build_table(tester, Some(MainAxisAlignment::Center), false);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_f32_equals(tester.get_center(find::text("Header")).dx(), 400.0);

            // Test mainAxisAlignment with sort arrow.
            let root = build_table(tester, None, true);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_eq!(
                tester.get_top_left(find::text("Header")).dx(),
                horizontal_margin
            );

            // Test mainAxisAlignment.center with sort arrow.
            let root = build_table(tester, Some(MainAxisAlignment::Center), true);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_f32_equals(tester.get_center(find::text("Header")).dx(), 400.0);
        });
    }

    #[test]
    fn data_table_theme_data_lerp_interpolates_numbers_and_picks_discrete_fields_at_half() {
        let a = DataTableThemeData::new()
            .horizontal_margin(2.0)
            .heading_row_alignment(MainAxisAlignment::Start);
        let b = DataTableThemeData::new()
            .horizontal_margin(4.0)
            .heading_row_alignment(MainAxisAlignment::End);
        let lerp = DataTableThemeData::lerp(&a, &b, 0.25);
        assert_f32_equals(lerp.horizontal_margin.expect("a margin"), 2.5);
        assert_eq!(lerp.heading_row_alignment, Some(MainAxisAlignment::Start));
        let lerp = DataTableThemeData::lerp(&a, &b, 0.75);
        assert_eq!(lerp.heading_row_alignment, Some(MainAxisAlignment::End));
    }
}

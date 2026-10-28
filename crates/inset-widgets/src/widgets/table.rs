//! Flutter counterpart: `widgets/table.dart`.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use inset_embedder::{TextBaseline, TextDirection};
use inset_foundation::{App, Handle, Key, LocalKey};
use inset_painting::Decoration;
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, ErasedRenderObject, RenderBox, RenderHandle, RenderTable,
};
pub use inset_rendering::{
    FixedColumnWidth, FlexColumnWidth, FractionColumnWidth, IntrinsicColumnWidth, MaxColumnWidth,
    MinColumnWidth, TableBorder, TableCellParentData, TableCellVerticalAlignment, TableColumnWidth,
    TableColumnWidthRef,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, KeyRef, ParentDataWidget, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget, Slot, StatelessWidget,
    Widget, WidgetKind, WidgetRef, downcast_widget,
};
use crate::widgets::basic::Directionality;
use crate::widgets::image::create_local_image_configuration;

/// A horizontal group of cells in a [`Table`].
///
/// Every row in a table must have the same number of children.
///
/// The alignment of individual cells in a row can be controlled using a [`TableCell`].
#[derive(Default)]
pub struct TableRow {
    /// An identifier for the row.
    pub key: Option<Rc<dyn LocalKey>>,

    /// A decoration to paint behind this row.
    ///
    /// Row decorations fill the horizontal and vertical extent of each row in the table, unlike
    /// decorations for individual cells, which might not fill either.
    pub decoration: Option<Box<dyn Decoration>>,

    /// The widgets that comprise the cells in this row.
    ///
    /// Children may be wrapped in [`TableCell`] widgets to provide per-cell configuration to
    /// the [`Table`], but children are not required to be wrapped in [`TableCell`] widgets.
    pub children: Vec<WidgetRef>,
}

impl TableRow {
    /// Creates a row in a [`Table`]; Dart's named arguments are the setters.
    pub fn new() -> TableRow {
        TableRow::default()
    }

    /// Dart `TableRow(key:)`.
    pub fn key(mut self, key: Rc<dyn LocalKey>) -> TableRow {
        self.key = Some(key);
        self
    }

    /// Dart `TableRow(decoration:)`.
    pub fn decoration(mut self, decoration: impl Decoration + 'static) -> TableRow {
        self.decoration = Some(Box::new(decoration));
        self
    }

    /// Dart `TableRow(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> TableRow {
        self.children = children.into_iter().collect();
        self
    }
}

impl fmt::Debug for TableRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TableRow(")?;
        if let Some(key) = &self.key {
            write!(f, "{key:?}, ")?;
        }
        if let Some(decoration) = &self.decoration {
            write!(f, "{decoration:?}, ")?;
        }
        if self.children.is_empty() {
            f.write_str("no children")?;
        } else {
            write!(f, "{:?}", self.children)?;
        }
        f.write_str(")")
    }
}

struct TableElementRow {
    key: Option<Rc<dyn LocalKey>>,
    children: Vec<AnyElement>,
}

/// A widget that uses the table layout algorithm for its children.
///
/// If you only have one row, the `Row` widget is more appropriate. If you only have one column,
/// the `SliverList` or `Column` widgets will be more appropriate.
///
/// Rows size vertically based on their contents. To control the individual column widths, use
/// the [`column_widths`](Self::column_widths) property to specify a [`TableColumnWidth`] for
/// each column. If [`column_widths`](Self::column_widths) is `None`, or there is no entry for a
/// given column in [`column_widths`](Self::column_widths), the table uses the
/// [`default_column_width`](Self::default_column_width) instead.
///
/// By default, [`default_column_width`](Self::default_column_width) is a [`FlexColumnWidth`].
/// This [`TableColumnWidth`] divides up the remaining space in the horizontal axis to determine
/// the column width. If wrapping a [`Table`] in a horizontal `ScrollView`, choose a different
/// [`TableColumnWidth`], such as [`FixedColumnWidth`].
///
/// For more details about the table layout algorithm, see [`RenderTable`]. To control the
/// alignment of children, see [`TableCell`].
pub struct Table {
    pub key: Option<KeyRef>,

    /// The rows of the table.
    ///
    /// Every row in a table must have the same number of children.
    pub children: Vec<TableRow>,

    /// How the horizontal extents of the columns of this table should be determined.
    ///
    /// If the map has no entry for a given column, the table uses the
    /// [`default_column_width`](Self::default_column_width) instead. By default, that uses flex
    /// sizing to distribute free space equally among the columns.
    ///
    /// The [`FixedColumnWidth`] class can be used to specify a specific width in pixels. That
    /// is the cheapest way to size a table's columns.
    ///
    /// The layout performance of the table depends critically on which column sizing
    /// algorithms are used here. In particular, [`IntrinsicColumnWidth`] is quite expensive
    /// because it needs to measure each cell in the column to determine the intrinsic size of
    /// the column.
    ///
    /// The keys of this map (column indexes) are zero-based.
    ///
    /// If this is set to `None`, then an empty map is assumed.
    pub column_widths: Option<HashMap<usize, TableColumnWidthRef>>,

    /// How to determine with widths of columns that don't have an explicit sizing algorithm.
    ///
    /// Specifically, the [`default_column_width`](Self::default_column_width) is used for
    /// column `i` if `column_widths[i]` is absent. Defaults to [`FlexColumnWidth`], which will
    /// divide the remaining horizontal space up evenly between columns of the same type
    /// [`TableColumnWidth`].
    ///
    /// A [`Table`] in a horizontal `ScrollView` must use a [`FixedColumnWidth`], or an
    /// [`IntrinsicColumnWidth`] as the horizontal space is infinite.
    pub default_column_width: TableColumnWidthRef,

    /// The direction in which the columns are ordered.
    ///
    /// Defaults to the ambient [`Directionality`].
    pub text_direction: Option<TextDirection>,

    /// The style to use when painting the boundary and interior divisions of the table.
    pub border: Option<TableBorder>,

    /// How cells that do not explicitly specify a vertical alignment are aligned vertically.
    ///
    /// Cells may specify a vertical alignment by wrapping their contents in a [`TableCell`]
    /// widget.
    pub default_vertical_alignment: TableCellVerticalAlignment,

    /// The text baseline to use when aligning rows using
    /// [`TableCellVerticalAlignment::Baseline`].
    ///
    /// This must be set if using baseline alignment. There is no default because there is no
    /// way for the framework to know the correct baseline _a priori_.
    pub text_baseline: Option<TextBaseline>,
}

impl Default for Table {
    fn default() -> Table {
        Table {
            key: None,
            children: Vec::new(),
            column_widths: None,
            default_column_width: FlexColumnWidth::default_width(),
            text_direction: None,
            border: None,
            default_vertical_alignment: TableCellVerticalAlignment::Top,
            text_baseline: None, // NO DEFAULT: we don't know what the text's baseline should be
        }
    }
}

impl Table {
    /// Creates a table; Dart's named arguments are the setters.
    pub fn new() -> Table {
        Table::default()
    }

    /// Dart `Table(key:)`.
    pub fn key(mut self, key: KeyRef) -> Table {
        self.key = Some(key);
        self
    }

    /// Dart `Table(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = TableRow>) -> Table {
        self.children = children.into_iter().collect();
        self
    }

    /// Dart `Table(columnWidths:)`.
    pub fn column_widths(mut self, column_widths: HashMap<usize, TableColumnWidthRef>) -> Table {
        self.column_widths = Some(column_widths);
        self
    }

    /// Dart `Table(defaultColumnWidth:)`.
    pub fn default_column_width(mut self, default_column_width: TableColumnWidthRef) -> Table {
        self.default_column_width = default_column_width;
        self
    }

    /// Dart `Table(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Table {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Table(border:)`.
    pub fn border(mut self, border: TableBorder) -> Table {
        self.border = Some(border);
        self
    }

    /// Dart `Table(defaultVerticalAlignment:)`.
    pub fn default_vertical_alignment(
        mut self,
        default_vertical_alignment: TableCellVerticalAlignment,
    ) -> Table {
        self.default_vertical_alignment = default_vertical_alignment;
        self
    }

    /// Dart `Table(textBaseline:)`.
    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> Table {
        self.text_baseline = Some(text_baseline);
        self
    }

    /// The tree node: a render object widget with its own element, outside the `IntoWidget`
    /// kinds.
    pub fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }

    /// Dart's `_rowDecorations` field, as its constructor fills it: each row's decoration when
    /// any row has one, else `None`.
    pub fn effective_row_decorations(&self) -> Option<Vec<Option<Box<dyn Decoration>>>> {
        if !self.children.iter().any(|row| row.decoration.is_some()) {
            return None;
        }
        Some(
            self.children
                .iter()
                .map(|row| {
                    row.decoration
                        .as_ref()
                        .map(|decoration| decoration.clone_box())
                })
                .collect(),
        )
    }

    /// The number of columns: the length of the first row.
    fn column_count(&self) -> usize {
        self.children.first().map_or(0, |row| row.children.len())
    }

    /// Dart's constructor asserts: a baseline alignment has a text baseline, and the rows have
    /// the same number of cells and none is empty. Dart's checks for duplicate keys wait with
    /// the framework's.
    fn debug_assert_is_valid(&self) -> bool {
        debug_assert!(
            self.default_vertical_alignment != TableCellVerticalAlignment::Baseline
                || self.text_baseline.is_some(),
            "textBaseline is required if you specify the defaultVerticalAlignment with \
             TableCellVerticalAlignment.baseline"
        );
        if cfg!(debug_assertions) && !self.children.is_empty() {
            let cell_count = self.column_count();
            assert!(
                self.children
                    .iter()
                    .all(|row| row.children.len() == cell_count),
                "Table contains irregular row lengths.\n\
                 Every TableRow in a Table must have the same number of children, so that \
                 every cell is filled. Otherwise, the table will contain holes."
            );
            assert!(
                self.children.iter().all(|row| !row.children.is_empty()),
                "One or more TableRow have no children.\n\
                 Every TableRow in a Table must have at least one child, so there is no empty \
                 row. "
            );
        }
        true
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl Widget for Table {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        TableElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<Table>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for Table {
    type RenderObject = RenderTable;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        debug_assert!(self.debug_assert_is_valid());
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        let render_object = RenderTable::new(app, text_direction);
        render_object.set_columns(app, self.column_count());
        render_object.set_rows(app, self.children.len());
        render_object.set_column_widths(app, self.column_widths.clone());
        render_object.set_default_column_width(app, Rc::clone(&self.default_column_width));
        render_object.set_border(app, self.border);
        render_object.set_row_decorations(app, self.effective_row_decorations());
        let configuration = create_local_image_configuration(app, context, None);
        render_object.set_configuration(app, configuration);
        render_object.set_default_vertical_alignment(app, self.default_vertical_alignment);
        render_object.set_text_baseline(app, self.text_baseline);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTable>,
    ) {
        debug_assert!(self.debug_assert_is_valid());
        debug_assert_eq!(render_object.columns(app), self.column_count());
        debug_assert_eq!(render_object.rows(app), self.children.len());
        render_object.set_column_widths(app, self.column_widths.clone());
        render_object.set_default_column_width(app, Rc::clone(&self.default_column_width));
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        render_object.set_text_direction(app, text_direction);
        render_object.set_border(app, self.border);
        render_object.set_row_decorations(app, self.effective_row_decorations());
        let configuration = create_local_image_configuration(app, context, None);
        render_object.set_configuration(app, configuration);
        render_object.set_default_vertical_alignment(app, self.default_vertical_alignment);
        render_object.set_text_baseline(app, self.text_baseline);
    }
}

/// Dart's `_TableSlot`: the cell a child of a [`Table`] fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TableSlot {
    column: usize,
    row: usize,
}

/// Dart's `_TableElement`.
struct TableElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    children: Vec<TableElementRow>,
    doing_mount_or_update: bool,
    forgotten_children: HashSet<AnyElement>,
    /// One slot object per cell: a [`Slot::Custom`] compares by identity, and Dart's
    /// `_TableSlot` compares by column and row, so each cell keeps the same object.
    slots: HashMap<TableSlot, Rc<dyn Any>>,
}

impl TableElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<TableElement> {
        debug_assert!(downcast_widget::<Table>(&*widget).is_some());
        app.create(TableElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            children: Vec::new(),
            doing_mount_or_update: false,
            forgotten_children: HashSet::new(),
            slots: HashMap::new(),
        })
    }

    /// Dart's `_TableSlot(column, row)` as an element slot.
    fn slot(self: Handle<Self>, app: &mut App, column: usize, row: usize) -> Slot {
        let slot = app
            .get_mut(self)
            .slots
            .entry(TableSlot { column, row })
            .or_insert_with(|| Rc::new(TableSlot { column, row }));
        Slot::Custom(Rc::clone(slot))
    }

    /// Dart's `_TableSlot slot` parameter type.
    fn table_slot(slot: Option<&Slot>) -> TableSlot {
        match slot {
            Some(Slot::Custom(slot)) => *slot
                .downcast_ref::<TableSlot>()
                .expect("a table gives its children table slots"),
            _ => unreachable!("a table gives its children table slots"),
        }
    }

    fn update_render_object_children(self: Handle<Self>, app: &mut App) {
        let this = app.get(self);
        let columns = this.children.first().map_or(0, |row| row.children.len());
        let cells: Vec<AnyElement> = this
            .children
            .iter()
            .flat_map(|row| row.children.iter().copied())
            .collect();
        let cells = cells
            .into_iter()
            .map(|child| {
                Some(AnyRenderBox::from_object(
                    child
                        .render_object(app)
                        .expect("a table cell has a render object"),
                ))
            })
            .collect();
        let render_object = self.typed_render_object(app);
        render_object.set_flat_children(app, columns, cells);
    }

    fn is_forgotten(self: Handle<Self>, app: &App, child: AnyElement) -> bool {
        app.get(self).forgotten_children.contains(&child)
    }
}

impl RenderObjectElementWidget for TableElement {
    type Widget = Table;

    fn widget_of(widget: &WidgetRef) -> &Table {
        downcast_widget::<Table>(&**widget).expect("a TableElement holds its Table")
    }
}

impl RenderObjectElement for TableElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl Element for TableElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(!app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = true;
        RenderObjectElement::mount(self, app, parent, new_slot);
        let widget = self.as_element().widget(app).clone();
        let table = Self::widget_of(&widget);
        let mut children = Vec::with_capacity(table.children.len());
        for (row_index, row) in table.children.iter().enumerate() {
            let mut row_children = Vec::with_capacity(row.children.len());
            for (column_index, child) in row.children.iter().enumerate() {
                let slot = self.slot(app, column_index, row_index);
                row_children.push(
                    self.as_element()
                        .inflate_widget(app, child.clone(), Some(slot)),
                );
            }
            children.push(TableElementRow {
                key: row.key.clone(),
                children: row_children,
            });
        }
        app.get_mut(self).children = children;
        self.update_render_object_children(app);
        debug_assert!(app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = false;
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let render_object = self.typed_render_object(app);
        render_object.as_object().setup_parent_data(app, child);
        // Once [mount]/[update] are done, the children are getting set all at once
        // in [update_render_object_children].
        if !app.get(self).doing_mount_or_update {
            let slot = Self::table_slot(slot.as_ref());
            render_object.set_child(
                app,
                slot.column,
                slot.row,
                Some(AnyRenderBox::from_object(child)),
            );
        }
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        debug_assert!(app.get(self).doing_mount_or_update);
        // Child gets moved at the end of [update] in [update_render_object_children].
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        _child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let slot = Self::table_slot(slot.as_ref());
        self.typed_render_object(app)
            .set_child(app, slot.column, slot.row, None);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        debug_assert!(!app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = true;
        let mut old_keyed_rows: Vec<(Rc<dyn LocalKey>, Vec<AnyElement>)> = Vec::new();
        let mut old_unkeyed: Vec<Vec<AnyElement>> = Vec::new();
        for row in &app.get(self).children {
            match &row.key {
                Some(key) => old_keyed_rows.push((Rc::clone(key), row.children.clone())),
                None => old_unkeyed.push(row.children.clone()),
            }
        }
        let mut old_unkeyed_rows = old_unkeyed.iter();
        let mut new_children = Vec::new();
        let mut taken: HashSet<usize> = HashSet::new();
        let is_forgotten = |app: &App, child: AnyElement| self.is_forgotten(app, child);
        let table = Self::widget_of(&new_widget);
        for (row_index, row) in table.children.iter().enumerate() {
            let keyed = row.key.as_ref().and_then(|key| {
                old_keyed_rows
                    .iter()
                    .position(|(old_key, _)| (**old_key).eq_key(&**key as &dyn Key))
            });
            let old_children = if let Some(index) = keyed {
                taken.insert(index);
                old_keyed_rows[index].1.clone()
            } else if row.key.is_none()
                && let Some(old_row) = old_unkeyed_rows.next()
            {
                old_row.clone()
            } else {
                Vec::new()
            };
            let slots: Vec<Option<Slot>> = (0..row.children.len())
                .map(|column_index| Some(self.slot(app, column_index, row_index)))
                .collect();
            let children = self.as_element().update_children(
                app,
                &old_children,
                &row.children,
                Some(&is_forgotten),
                Some(&slots),
            );
            new_children.push(TableElementRow {
                key: row.key.clone(),
                children,
            });
        }
        for old_row in old_unkeyed_rows {
            self.as_element()
                .update_children(app, old_row, &[], Some(&is_forgotten), None);
        }
        for (index, (_, old_children)) in old_keyed_rows.iter().enumerate() {
            if !taken.contains(&index) {
                self.as_element().update_children(
                    app,
                    old_children,
                    &[],
                    Some(&is_forgotten),
                    None,
                );
            }
        }

        app.get_mut(self).children = new_children;
        self.update_render_object_children(app);
        app.get_mut(self).forgotten_children.clear();
        RenderObjectElement::update(self, app, new_widget);
        debug_assert!(app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = false;
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        let this = app.get(self);
        for child in this.children.iter().flat_map(|row| &row.children) {
            if !this.forgotten_children.contains(child) {
                visitor(*child);
            }
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        app.get_mut(self).forgotten_children.insert(child);
    }
}

/// A widget that controls how a child of a [`Table`] is aligned.
///
/// A [`TableCell`] widget must be a descendant of a [`Table`], and the path from the
/// [`TableCell`] widget to its enclosing [`Table`] must contain only [`TableRow`]s,
/// [`StatelessWidget`]s, or `StatefulWidget`s (not other kinds of widgets, like
/// [`RenderObjectWidget`]s).
///
/// To create an empty [`TableCell`], provide a `SizedBox::shrink` as the
/// [`child`](Self::child).
#[derive(Debug)]
pub struct TableCell {
    pub key: Option<KeyRef>,

    /// How this cell is aligned vertically.
    pub vertical_alignment: Option<TableCellVerticalAlignment>,

    /// The child of this cell.
    pub child: WidgetRef,
}

impl TableCell {
    /// Creates a widget that controls how a child of a [`Table`] is aligned; Dart's optional
    /// named arguments are the setters.
    pub fn new<K>(child: impl crate::framework::IntoWidget<K>) -> TableCell {
        TableCell {
            key: None,
            vertical_alignment: None,
            child: child.into_widget(),
        }
    }

    /// Dart `TableCell(key:)`.
    pub fn key(mut self, key: KeyRef) -> TableCell {
        self.key = Some(key);
        self
    }

    /// Dart `TableCell(verticalAlignment:)`.
    pub fn vertical_alignment(
        mut self,
        vertical_alignment: TableCellVerticalAlignment,
    ) -> TableCell {
        self.vertical_alignment = Some(vertical_alignment);
        self
    }
}

impl StatelessWidget for TableCell {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        use crate::framework::IntoWidget;
        TableCellParentDataWidget {
            vertical_alignment: self.vertical_alignment,
            child: self.child.clone(),
        }
        .into_widget()
    }
}

/// Dart's `_TableCell`: writes a cell's vertical alignment into its parent data.
#[derive(Debug)]
struct TableCellParentDataWidget {
    vertical_alignment: Option<TableCellVerticalAlignment>,
    child: WidgetRef,
}

impl ParentDataWidget for TableCellParentDataWidget {
    type ParentData = TableCellParentData;

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        let parent_data = render_object.parent_data_of_mut::<TableCellParentData>(app);
        if parent_data.vertical_alignment != self.vertical_alignment {
            parent_data.vertical_alignment = self.vertical_alignment;
            if let Some(parent) = render_object.parent(app) {
                parent.mark_needs_layout(app);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::{Offset, Size};
    use inset_foundation::{AppCell, ValueKey};
    use inset_painting::PaintingBinding;
    use inset_rendering::RenderParagraph;

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Center, SizedBox};
    use crate::widgets::text::Text;

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn directional(text_direction: TextDirection, child: WidgetRef) -> WidgetRef {
        Directionality::new(text_direction, child).into_widget()
    }

    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        install_fonts(app);
        let harness = Harness::mount(app, child);
        harness.pump(app);
        harness
    }

    fn update(app: &mut App, harness: &Harness, child: WidgetRef) {
        harness.set_child(app, child);
        harness.pump(app);
    }

    /// The first `RenderTable` below `node`, depth first.
    fn find_table(app: &App, node: AnyRenderObject) -> Option<RenderHandle<RenderTable>> {
        if let Some(table) = node.downcast::<RenderTable>(app) {
            return Some(table);
        }
        let mut children = Vec::new();
        node.visit_children(app, &mut |child| children.push(child));
        children
            .into_iter()
            .find_map(|child| find_table(app, child))
    }

    fn table_under(app: &App, harness: &Harness) -> RenderHandle<RenderTable> {
        find_table(app, harness.render_root(app).as_object()).expect("a RenderTable")
    }

    fn text_row(texts: &[&str]) -> TableRow {
        TableRow::new().children(texts.iter().map(|text| Text::new(*text).into_widget()))
    }

    /// The cell at column `x` of row `y`.
    fn cell(app: &App, table: RenderHandle<RenderTable>, x: usize, y: usize) -> AnyRenderBox {
        table.row(app, y)[x]
    }

    /// `table_test.dart`: "Table widget - empty".
    #[test]
    fn an_empty_table_mounts() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount(
            &mut app,
            directional(TextDirection::Ltr, Table::new().into_widget()),
        );
        assert_eq!(table_under(&app, &harness).rows(&app), 0);
    }

    /// `table_test.dart`: "Table widget - control test": flexible columns share the width
    /// equally whatever the text, in either direction.
    #[test]
    fn flexible_columns_are_equal_in_both_directions() {
        for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
            let app_cell = AppCell::new();
            let mut app = app_cell.borrow_mut();
            let table = Table::new().children([
                text_row(&["AAAAAA", "B", "C"]),
                text_row(&["D", "EEE", "F"]),
                text_row(&["G", "H", "III"]),
            ]);
            let harness = mount(&mut app, directional(text_direction, table.into_widget()));
            let table = table_under(&app, &harness);
            let a = cell(&app, table, 0, 0).size(&app);
            for (x, y) in [(0, 1), (0, 2), (1, 0)] {
                assert_eq!(cell(&app, table, x, y).size(&app), a, "{text_direction:?}");
            }
        }
    }

    fn fixed(width: f64) -> TableColumnWidthRef {
        Rc::new(FixedColumnWidth::new(width))
    }

    /// `table_test.dart`: "Table widget - column offset (LTR)" and "(RTL)".
    #[test]
    fn fixed_columns_are_placed_from_the_start_edge() {
        for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
            let app_cell = AppCell::new();
            let mut app = app_cell.borrow_mut();
            let widths = HashMap::from([(0, fixed(50.0)), (1, fixed(60.0)), (2, fixed(75.0))]);
            let table = Table::new()
                .column_widths(widths)
                .default_column_width(fixed(333.0))
                .children([
                    text_row(&["A1", "B1", "C1"]),
                    text_row(&["A2", "B2", "C2"]),
                    text_row(&["A3", "B3", "C3"]),
                ]);
            let harness = mount(
                &mut app,
                directional(
                    text_direction,
                    Center::new().child(table.into_widget()).into_widget(),
                ),
            );
            let table = table_under(&app, &harness);
            assert_eq!(table.size(&app).width(), 185.0);
            let left = |app: &App, x: usize, y: usize| {
                cell(app, table, x, y).box_parent_data(app).offset.dx()
            };
            for y in 0..3 {
                for (x, width) in [(0, 50.0), (1, 60.0), (2, 75.0)] {
                    assert_eq!(cell(&app, table, x, y).size(&app).width(), width);
                }
                let expected = match text_direction {
                    TextDirection::Ltr => [0.0, 50.0, 110.0],
                    TextDirection::Rtl => [135.0, 75.0, 0.0],
                };
                assert_eq!(
                    [left(&app, 0, y), left(&app, 1, y), left(&app, 2, y)],
                    expected,
                    "{text_direction:?}"
                );
            }
        }
    }

    /// `table_test.dart`: "Table widget - changing table dimensions": the first cell keeps its
    /// render object, and a cell that moved to another place gets a new one.
    #[test]
    fn a_reshaped_table_keeps_the_cells_that_stay() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let three_by_three = Table::new().children([
            text_row(&["A", "B", "C"]),
            text_row(&["D", "E", "F"]),
            text_row(&["G", "H", "I"]),
        ]);
        let harness = mount(
            &mut app,
            directional(TextDirection::Ltr, three_by_three.into_widget()),
        );
        let table = table_under(&app, &harness);
        let a1 = cell(&app, table, 0, 0);
        let g1 = cell(&app, table, 0, 2);

        let four_by_two = Table::new().children([
            text_row(&["a", "b", "c", "d"]),
            text_row(&["e", "f", "g", "h"]),
        ]);
        update(
            &mut app,
            &harness,
            directional(TextDirection::Ltr, four_by_two.into_widget()),
        );
        assert_eq!((table.columns(&app), table.rows(&app)), (4, 2));
        assert_eq!(cell(&app, table, 0, 0), a1);
        assert_ne!(cell(&app, table, 2, 1), g1);
        assert!(
            !app.contains(g1.as_object().id()),
            "the old cell left the tree"
        );
    }

    /// `table_test.dart`: "Table widget - intrinsic sizing test": intrinsic columns are as wide
    /// as their widest text, and the rows as tall as their text.
    #[test]
    fn intrinsic_columns_fit_their_text() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let table = Table::new()
            .default_column_width(Rc::new(IntrinsicColumnWidth::new(None)))
            .children([
                text_row(&["AAA", "B", "C"]),
                text_row(&["D", "E", "FFFFFF"]),
                text_row(&["G", "H", "III"]),
            ]);
        let harness = mount(
            &mut app,
            directional(TextDirection::Ltr, table.into_widget()),
        );
        let table = table_under(&app, &harness);
        let a = cell(&app, table, 0, 0).size(&app);
        assert_eq!(cell(&app, table, 0, 1).size(&app), a);
        assert_eq!(cell(&app, table, 0, 2).size(&app), a);
        let b = cell(&app, table, 1, 0).size(&app);
        assert!(a.width() > b.width());
        assert_eq!(a.height(), b.height());
        let widest = cell(&app, table, 2, 1)
            .as_object()
            .downcast::<RenderParagraph>(&app)
            .expect("a paragraph")
            .text_size(&app)
            .width();
        assert_eq!(cell(&app, table, 2, 0).size(&app).width(), widest);
    }

    /// `table_test.dart`: "Table widget - moving test" and "Table widget - keyed rows": a keyed
    /// row keeps its cells when it moves, and the cells of a removed keyed row leave.
    #[test]
    fn a_keyed_row_keeps_its_cells_when_it_moves() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let keyed = || {
            TableRow::new()
                .key(Rc::new(ValueKey::new(1)))
                .children([SizedBox::new().height(10.0).into_widget()])
        };
        let plain = || TableRow::new().children([SizedBox::new().height(20.0).into_widget()]);
        let harness = mount(
            &mut app,
            directional(
                TextDirection::Ltr,
                Table::new().children([keyed(), plain()]).into_widget(),
            ),
        );
        let table = table_under(&app, &harness);
        let keyed_cell = cell(&app, table, 0, 0);

        update(
            &mut app,
            &harness,
            directional(
                TextDirection::Ltr,
                Table::new().children([plain(), keyed()]).into_widget(),
            ),
        );
        assert_eq!(cell(&app, table, 0, 1), keyed_cell);
        assert_eq!(cell(&app, table, 0, 1).size(&app).height(), 10.0);

        update(
            &mut app,
            &harness,
            directional(
                TextDirection::Ltr,
                Table::new().children([plain()]).into_widget(),
            ),
        );
        assert_eq!(table.rows(&app), 1);
        assert!(
            !app.contains(keyed_cell.as_object().id()),
            "the keyed row's cell left the tree"
        );
    }

    /// A `TableCell` places its child in the row by its own alignment, over the table's
    /// default.
    #[test]
    fn a_table_cell_aligns_its_child_in_the_row() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let sized = |height: f64| SizedBox::new().height(height).into_widget();
        let table = Table::new().children([TableRow::new().children([
            sized(100.0),
            TableCell::new(sized(20.0))
                .vertical_alignment(TableCellVerticalAlignment::Middle)
                .into_widget(),
            TableCell::new(sized(20.0))
                .vertical_alignment(TableCellVerticalAlignment::Bottom)
                .into_widget(),
            TableCell::new(sized(20.0)).into_widget(),
            TableCell::new(sized(20.0))
                .vertical_alignment(TableCellVerticalAlignment::Fill)
                .into_widget(),
        ])]);
        let harness = mount(
            &mut app,
            directional(TextDirection::Ltr, table.into_widget()),
        );
        let table = table_under(&app, &harness);
        let top = |x: usize| cell(&app, table, x, 0).box_parent_data(&app).offset.dy();
        assert_eq!([top(1), top(2), top(3), top(4)], [40.0, 80.0, 0.0, 0.0]);
        assert_eq!(cell(&app, table, 4, 0).size(&app).height(), 100.0);
        let parent_data = cell(&app, table, 1, 0)
            .as_object()
            .parent_data_of::<TableCellParentData>(&app);
        assert_eq!(
            parent_data.vertical_alignment,
            Some(TableCellVerticalAlignment::Middle)
        );
    }

    /// `table_test.dart`: "Set defaultVerticalAlignment to intrinsic height and check their
    /// heights".
    #[test]
    fn intrinsic_height_rows_stretch_their_cells_to_the_tallest() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let sized = |height: f64, text: &str| {
            SizedBox::new()
                .height(height)
                .child(Text::new(text))
                .into_widget()
        };
        let table = Table::new()
            .default_vertical_alignment(TableCellVerticalAlignment::IntrinsicHeight)
            .children([
                TableRow::new().children([sized(100.0, "A"), sized(200.0, "B")]),
                TableRow::new().children([sized(200.0, "C"), sized(300.0, "D")]),
            ]);
        let harness = mount(
            &mut app,
            directional(
                TextDirection::Ltr,
                crate::widgets::basic::UnconstrainedBox::new()
                    .child(SizedBox::new().width(200.0).child(table.into_widget()))
                    .into_widget(),
            ),
        );
        let table = table_under(&app, &harness);
        let height = |x: usize, y: usize| cell(&app, table, x, y).size(&app).height();
        assert_eq!(height(0, 0), height(1, 0));
        assert_eq!(height(0, 1), height(1, 1));
        assert!(height(1, 1) > height(0, 0));
        assert_eq!(table.size(&app), Size::new(200.0, 500.0));
    }

    /// A row decoration is given to the render table only when some row has one.
    #[test]
    fn row_decorations_reach_the_render_table() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let plain = Table::new().children([text_row(&["a"]), text_row(&["b"])]);
        assert!(plain.effective_row_decorations().is_none());
        let decorated = Table::new().children([
            text_row(&["a"]),
            text_row(&["b"]).decoration(inset_painting::BoxDecoration::new()),
        ]);
        let harness = mount(
            &mut app,
            directional(TextDirection::Ltr, decorated.into_widget()),
        );
        let table = table_under(&app, &harness);
        let decorations = table.row_decorations(&app);
        assert_eq!(decorations.len(), 2);
        assert!(decorations[0].is_none() && decorations[1].is_some());
        assert_eq!(table.get_row_box(&app, 0).top, 0.0);
        assert_eq!(
            cell(&app, table, 0, 1).box_parent_data(&app).offset,
            Offset::new(0.0, table.get_row_box(&app, 1).top)
        );
    }

    /// `table_test.dart`: "Table widget requires all TableRows to have same number of children".
    #[test]
    #[should_panic(expected = "Table contains irregular row lengths.")]
    fn rows_of_different_lengths_are_refused() {
        let app_cell = AppCell::new();
        let mut app = app_cell.borrow_mut();
        let table = Table::new().children([text_row(&["a", "b"]), text_row(&["c"])]);
        mount(
            &mut app,
            directional(TextDirection::Ltr, table.into_widget()),
        );
    }
}

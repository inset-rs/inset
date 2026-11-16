//! Data tables: a `DataTable` sorted by a column header and selectable by row, and a
//! `PaginatedDataTable` over a `DataTableSource` with a footer that pages and a rows-per-page
//! picker.
//!
//! A caption under each table says what the last interaction did: which column sorts, in which
//! direction, and how many rows are selected.

use std::collections::HashSet;

use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Ref};
use waft_material::{
    AnyDataTableSource, DataCell, DataColumn, DataRow, DataTable, DataTableSource,
    PaginatedDataTable,
};
use waft_widgets::{
    BuildContext, IntoWidget, ListView, SingleChildScrollView, State, StateData, StatefulWidget,
    Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

/// The rows both tables show: a dessert and its calories.
const DESSERTS: [(&str, i32); 6] = [
    ("Frozen yogurt", 159),
    ("Ice cream sandwich", 237),
    ("Eclair", 262),
    ("Cupcake", 305),
    ("Gingerbread", 356),
    ("Honeycomb", 408),
];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::DataTables.title(), DataTablesDemo.into_widget()).into_widget()
}

/// The sort and the selection live here, in the smallest widget that renders them.
#[derive(Debug)]
pub struct DataTablesDemo;

/// Dart would spell this `_DataTablesDemoState`.
pub struct DataTablesDemoState {
    state: StateData<DataTablesDemo>,
    sort_column: usize,
    ascending: bool,
    selected: HashSet<usize>,
    source: Option<Ref<DessertSource>>,
}

impl StatefulWidget for DataTablesDemo {
    type State = DataTablesDemoState;

    fn create_state(&self) -> DataTablesDemoState {
        DataTablesDemoState {
            state: StateData::new(),
            sort_column: 0,
            ascending: true,
            selected: HashSet::new(),
            source: None,
        }
    }
}

impl State for DataTablesDemoState {
    type Widget = DataTablesDemo;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let source = DessertSource::new(app);
        app.get_mut(self).source = Some(source);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (sort_column, ascending, selected_count) = {
            let state = app.get(self);
            (state.sort_column, state.ascending, state.selected.len())
        };
        let source = app
            .get(self)
            .source
            .as_ref()
            .expect("init_state created the source")
            .erase::<AnyDataTableSource>();
        let sort_caption = format!(
            "Sorted by {} {}; {selected_count} selected",
            ["Dessert", "Calories"][sort_column],
            if ascending { "ascending" } else { "descending" },
        );

        ListView::new()
            .children(vec![
                section_header(app, context, "DataTable"),
                caption(app, context, sort_caption),
                SingleChildScrollView::new()
                    .scroll_direction(waft_painting::Axis::Horizontal)
                    .child(sortable_table(self, app))
                    .into_widget(),
                section_header(app, context, "PaginatedDataTable"),
                PaginatedDataTable::new(columns_without_sort(), source)
                    .header(Text::new("Desserts"))
                    .rows_per_page(3)
                    .into_widget(),
            ])
            .into_widget()
    }
}

impl DataTablesDemoState {
    fn sorted_indices(self: Handle<Self>, app: &App) -> Vec<usize> {
        let (sort_column, ascending) = {
            let state = app.get(self);
            (state.sort_column, state.ascending)
        };
        let mut indices: Vec<usize> = (0..DESSERTS.len()).collect();
        indices.sort_by(|&a, &b| match sort_column {
            0 => DESSERTS[a].0.cmp(DESSERTS[b].0),
            _ => DESSERTS[a].1.cmp(&DESSERTS[b].1),
        });
        if !ascending {
            indices.reverse();
        }
        indices
    }
}

/// A `DataTable` whose two column headers sort it and whose rows select.
fn sortable_table(state: Handle<DataTablesDemoState>, app: &mut App) -> WidgetRef {
    let (sort_column, ascending) = {
        let demo = app.get(state);
        (demo.sort_column, demo.ascending)
    };
    let sort_by = move |column: usize| {
        move |app: &mut App, _index: i32, ascending: bool| {
            state.set_state(app, |demo| {
                demo.sort_column = column;
                demo.ascending = ascending;
            });
        }
    };
    let columns = vec![
        DataColumn::new(Text::new("Dessert")).on_sort(sort_by(0)),
        DataColumn::new(Text::new("Calories"))
            .numeric(true)
            .on_sort(sort_by(1)),
    ];
    let selected = app.get(state).selected.clone();
    let rows = state
        .sorted_indices(app)
        .into_iter()
        .map(|index| {
            let (name, calories) = DESSERTS[index];
            DataRow::by_index(
                Some(index as i32),
                vec![
                    DataCell::new(Text::new(name)),
                    DataCell::new(Text::new(calories.to_string())),
                ],
            )
            .selected(selected.contains(&index))
            .on_select_changed(move |app, value| {
                state.set_state(app, |demo| {
                    if value.unwrap_or(false) {
                        demo.selected.insert(index);
                    } else {
                        demo.selected.remove(&index);
                    }
                });
            })
        })
        .collect();
    DataTable::new(columns, rows)
        .sort_column_index(sort_column)
        .sort_ascending(ascending)
        .into_widget()
}

/// The paginated table's columns: this table does not sort.
fn columns_without_sort() -> Vec<DataColumn> {
    vec![
        DataColumn::new(Text::new("Dessert")),
        DataColumn::new(Text::new("Calories")).numeric(true),
    ]
}

/// A source of [`DESSERTS`] repeated twenty times, whose rows select.
pub struct DessertSource {
    change_notifier: ChangeNotifierData,
    selected: HashSet<usize>,
}

impl ChangeNotifier for DessertSource {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl DessertSource {
    fn new(app: &mut App) -> Ref<DessertSource> {
        app.create(DessertSource {
            change_notifier: ChangeNotifierData::new(),
            selected: HashSet::new(),
        })
    }

    fn select(self: Handle<Self>, app: &mut App, index: usize, selected: Option<bool>) {
        if selected.unwrap_or(false) {
            app.get_mut(self).selected.insert(index);
        } else {
            app.get_mut(self).selected.remove(&index);
        }
        self.notify_listeners(app);
    }
}

impl DataTableSource for DessertSource {
    fn get_row(self: Handle<Self>, app: &mut App, index: usize) -> Option<DataRow> {
        let (name, calories) = DESSERTS[index % DESSERTS.len()];
        let selected = app.get(self).selected.contains(&index);
        Some(
            DataRow::by_index(
                Some(index as i32),
                vec![
                    DataCell::new(Text::new(format!("{name} {}", index / DESSERTS.len() + 1))),
                    DataCell::new(Text::new(calories.to_string())),
                ],
            )
            .selected(selected)
            .on_select_changed(move |app, value| self.select(app, index, value)),
        )
    }

    fn row_count(self: Handle<Self>, _app: &App) -> usize {
        20 * DESSERTS.len()
    }

    fn is_row_count_approximate(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn selected_row_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).selected.len()
    }
}

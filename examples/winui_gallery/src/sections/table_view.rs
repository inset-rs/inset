//! TableView: a table of network connections whose columns the user resizes and sorts, the first frozen, one row selected, a filter the page applies to the source, and an empty table's template.

use crate::{column, example, label, section};
use waft_embedder::{Color, FontWeight};
use waft_foundation::{App, Handle, Ref};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry};
use waft_widgets::*;
use waft_winui::*;

/// Builds the feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("TableView", resources, TableViewDemo.into_widget())
}

/// A network connection, the kind of row an inspector's live panel shows.
#[derive(Clone, Debug)]
struct Connection {
    process: &'static str,
    pid: u32,
    state: ConnectionState,
    remote: &'static str,
    received: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ConnectionState {
    Established,
    Listening,
    Closing,
}

impl ConnectionState {
    fn name(self) -> &'static str {
        match self {
            ConnectionState::Established => "Established",
            ConnectionState::Listening => "Listening",
            ConnectionState::Closing => "Closing",
        }
    }
}

fn connections() -> Vec<Connection> {
    use ConnectionState::*;
    let rows = [
        ("browser", 4120, Established, "151.101.1.69:443", 2_480_311),
        ("mail", 2210, Established, "17.57.146.20:993", 88_102),
        ("sync", 3301, Closing, "140.82.112.25:443", 1_204),
        ("waos", 1001, Listening, "*:7400", 0),
        ("music", 5120, Established, "35.186.224.25:443", 14_092_448),
        ("updater", 812, Established, "13.107.4.50:443", 402_118),
        ("ssh", 6402, Established, "192.168.1.20:22", 33_650),
        ("dns", 77, Listening, "*:53", 0),
        ("chat", 4411, Closing, "162.159.135.234:443", 9_870),
        ("backup", 3350, Established, "52.216.20.14:443", 120_448_002),
    ];
    rows.into_iter()
        .map(|(process, pid, state, remote, received)| Connection {
            process,
            pid,
            state,
            remote,
            received,
        })
        .collect()
}

/// The page's tables and what they hold.
#[derive(Debug)]
struct TableViewDemo;

struct TableViewDemoState {
    state: StateData<TableViewDemo>,
    source: Option<Ref<TableViewSource<Connection>>>,
    columns: Vec<Ref<TableViewColumn<Connection>>>,
    selection: Option<Ref<TableViewSelection>>,
    sort: Option<Ref<TableViewSort>>,
    established_only: bool,
    empty_source: Option<Ref<TableViewSource<Connection>>>,
    empty_columns: Vec<Ref<TableViewColumn<Connection>>>,
}

impl StatefulWidget for TableViewDemo {
    type State = TableViewDemoState;

    fn create_state(&self) -> Self::State {
        TableViewDemoState {
            state: StateData::new(),
            source: None,
            columns: Vec::new(),
            selection: None,
            sort: None,
            established_only: false,
            empty_source: None,
            empty_columns: Vec::new(),
        }
    }
}

/// The connection table's columns: the process frozen, the pid and the bytes sorting by number and opening on the largest, the state a badge.
fn connection_columns(app: &mut App) -> Vec<Ref<TableViewColumn<Connection>>> {
    let process = TableViewColumn::text(app, "Process", |c: &Connection| c.process.to_string());
    process.set_frozen_edge(app, TableViewFrozenEdge::Leading);
    process.set_header_tool_tip(app, Some("The process that owns the connection".to_owned()));
    let pid = TableViewColumn::text(app, "PID", |c: &Connection| c.pid.to_string());
    pid.set_width(app, GridLength::AUTO);
    pid.set_sort_key(app, |c: &Connection| c.pid);
    pid.set_sort_cycle(app, TableViewSortCycle::DescendingAscendingNone);
    let state = TableViewColumn::template(app, "State", |app, context, c: &Connection| {
        state_badge(app, context, c.state)
    });
    state.set_custom_sort_comparer(app, |a: &Connection, b: &Connection| a.state.cmp(&b.state));
    let remote =
        TableViewColumn::text(app, "Remote address", |c: &Connection| c.remote.to_string());
    remote.set_width(app, GridLength::STAR);
    remote.set_min_width(app, 160.0);
    let received = TableViewColumn::text(app, "Received", |c: &Connection| {
        format!("{} B", c.received)
    });
    received.set_sort_key(app, |c: &Connection| c.received);
    received.set_sort_cycle(app, TableViewSortCycle::DescendingAscendingNone);
    vec![process, pid, state, remote, received]
}

/// A template column's cell: the state as a small accent badge.
fn state_badge(app: &mut App, context: BuildContext, state: ConnectionState) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    let (background, foreground) = match state {
        ConnectionState::Established => (
            r.accent_fill_color_default_brush,
            r.text_on_accent_fill_color_primary,
        ),
        ConnectionState::Listening => (r.control_fill_color_secondary, r.text_fill_color_primary),
        ConnectionState::Closing => (r.control_fill_color_tertiary, r.text_fill_color_secondary),
    };
    Padding::new(EdgeInsetsGeometry::from_ltrb(8.0, 0.0, 8.0, 0.0))
        .child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .child(
                    ControlBorder::new(Brush::Solid(background), Brush::Solid(Color::new(0)))
                        .border_thickness(0.0)
                        .corner_radius(10.0)
                        .padding([8.0, 2.0, 8.0, 2.0])
                        .child(Text::new(state.name()).style(control_text_style(
                            &r,
                            12.0,
                            FontWeight::NORMAL,
                            foreground,
                        ))),
                ),
        )
        .into_widget()
}

impl TableViewDemoState {
    fn source(self: Handle<Self>, app: &App) -> Ref<TableViewSource<Connection>> {
        app.get(self).source.clone().expect("made in `init_state`")
    }

    /// The page's filter: a predicate on the source, the table showing what it keeps.
    fn set_established_only(self: Handle<Self>, app: &mut App, on: bool) {
        let source = self.source(app);
        if on {
            source.filter(app, |c| c.state == ConnectionState::Established);
        } else {
            source.clear_filter(app);
        }
        self.set_state(app, |state| state.established_only = on);
    }

    /// A connection arriving at the top of the items, as a live panel's do: the table places it and the selection stays on its row's item.
    fn add_connection(self: Handle<Self>, app: &mut App) {
        let source = self.source(app);
        let arrived = source.item_count(app) as u32;
        source.insert(
            app,
            0,
            Connection {
                process: "worker",
                pid: 9000 + arrived,
                state: ConnectionState::Established,
                remote: "10.0.0.8:8443",
                received: u64::from(arrived) * 1_000,
            },
        );
    }

    fn caption(self: Handle<Self>, app: &App) -> String {
        let selection = app
            .get(self)
            .selection
            .clone()
            .expect("made in `init_state`");
        match selection.selected_item(app) {
            Some(item) => {
                let connection = self.source(app).item(app, item);
                format!("Selected: {} ({})", connection.process, connection.remote)
            }
            None => "Nothing selected".to_owned(),
        }
    }
}

impl State for TableViewDemoState {
    type Widget = TableViewDemo;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let source = TableViewSource::from(app, connections());
        let columns = connection_columns(app);
        let selection = TableViewSelection::new(app);
        let sort = TableViewSort::new(app);
        let empty_source = TableViewSource::from(app, Vec::new());
        let empty_columns = connection_columns(app);
        let state = app.get_mut(self);
        state.source = Some(source);
        state.columns = columns;
        state.selection = Some(selection);
        state.sort = Some(sort);
        state.empty_source = Some(empty_source);
        state.empty_columns = empty_columns;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let sources = [state.source.take(), state.empty_source.take()];
        let columns: Vec<_> = state
            .columns
            .drain(..)
            .chain(state.empty_columns.drain(..))
            .collect();
        let selection = state.selection.take();
        for source in sources.into_iter().flatten() {
            source.dispose(app);
        }
        for column in columns {
            column.dispose(app);
        }
        if let Some(selection) = selection {
            selection.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let secondary = resources.text_fill_color_secondary;
        let state = app.get(self);
        let source = state.source.clone().expect("made in `init_state`");
        let selection = state.selection.clone().expect("made in `init_state`");
        let sort = state.sort.clone().expect("made in `init_state`");
        let columns = state.columns.clone();
        let established_only = state.established_only;
        let empty_source = state.empty_source.clone().expect("made in `init_state`");
        let empty_columns = state.empty_columns.clone();
        let by_received = columns[4].clone();
        let table = TableView::new(source, columns)
            .selection(selection)
            .sort(sort.clone())
            .alternating_row_background(Brush::Solid(
                resources.tabular_surface_row_background_alternating_brush,
            ))
            .selection_changed(move |app, _| self.set_state(app, |_| {}));
        let sort_button = Button::text(
            "Largest first",
            waft_foundation::Listener::new(move |app| {
                sort.sort_by_column(app, &by_received, SortDirection::Descending);
            }),
        );
        let add_button = Button::text(
            "New connection",
            waft_foundation::Listener::new(move |app| self.add_connection(app)),
        );
        let filter = ToggleSwitch::new(established_only, move |app, on| {
            self.set_established_only(app, on)
        })
        .header(Text::new("Established only"));
        let empty = TableView::new(empty_source, empty_columns)
            .density(TableViewDensity::Compact)
            .grid_lines_visibility(TableViewGridLinesVisibility::Horizontal)
            .empty_template(|app, context| {
                let r = ThemeResources::of(app, context);
                label(
                    "No connections yet",
                    TextBlockStyle::Body,
                    r.text_fill_color_secondary,
                )
            });
        column(
            vec![
                example(
                    "Connections",
                    "Drag a header's edge to resize its column, click a header to sort; PID and Received open on the largest. The process column stays while the rest scroll. A click or the arrow keys select a row, and the selection stays on its connection as new ones arrive.",
                    &resources,
                    column(
                        vec![
                            SizedBox::new().height(300.0).child(table).into_widget(),
                            example_controls(vec![
                                filter.into_widget(),
                                sort_button.into_widget(),
                                add_button.into_widget(),
                            ]),
                            label(self.caption(app), TextBlockStyle::Caption, secondary),
                        ],
                        8.0,
                    ),
                ),
                example(
                    "Empty, compact, horizontal lines",
                    "A table with no rows shows its EmptyTemplate under the header.",
                    &resources,
                    SizedBox::new().height(160.0).child(empty).into_widget(),
                ),
            ],
            20.0,
        )
    }
}

/// The filter and the sort button side by side.
fn example_controls(children: Vec<WidgetRef>) -> WidgetRef {
    crate::example_row(children, 16.0)
}

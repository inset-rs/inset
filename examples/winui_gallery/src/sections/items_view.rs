//! ItemsView: a list whose rows light under the pointer, take the focus and are selected one, several or a range at a time, a long list built only where it is seen, and invoking a row.

use crate::{column, example, label, section};
use waft_embedder::FontWeight;
use waft_foundation::{App, Handle, Listener};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::*;
use waft_winui::*;

/// Builds the feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ItemsView", resources, ItemsViewDemo.into_widget())
}

/// The examples, whose selections the page owns.
#[derive(Debug)]
struct ItemsViewDemo;

/// One selection per example, and what was last invoked.
struct ItemsViewDemoState {
    state: StateData<ItemsViewDemo>,
    single: Option<Handle<ItemsViewSelection>>,
    multiple: Option<Handle<ItemsViewSelection>>,
    extended: Option<Handle<ItemsViewSelection>>,
    long: Option<Handle<ItemsViewSelection>>,
    /// The row last invoked in the single-selection example.
    invoked: Option<usize>,
}

impl StatefulWidget for ItemsViewDemo {
    type State = ItemsViewDemoState;

    fn create_state(&self) -> Self::State {
        ItemsViewDemoState {
            state: StateData::new(),
            single: None,
            multiple: None,
            extended: None,
            long: None,
            invoked: None,
        }
    }
}

/// The rows of the short examples.
const FEEDS: [(&str, &str); 8] = [
    ("Morning news", "12 unread"),
    ("Science daily", "3 unread"),
    ("Design notes", "All read"),
    ("Rust blog", "1 unread"),
    ("Photography", "8 unread"),
    ("Release notes", "All read"),
    ("Cooking", "5 unread"),
    ("Travel", "2 unread"),
];

/// How many rows the long example has.
const LONG_COUNT: usize = 10_000;

/// The height of each of the long example's rows, which the view is told so that End and Home
/// jump without building the rows between.
const LONG_ROW: f64 = 56.0;

impl ItemsViewDemoState {
    fn selection(
        self: Handle<Self>,
        app: &App,
        pick: fn(&ItemsViewDemoState) -> Option<Handle<ItemsViewSelection>>,
    ) -> Handle<ItemsViewSelection> {
        pick(app.get(self)).expect("made in `init_state`")
    }

    /// Rebuilds the page so its captions show the selections.
    fn changed(self: Handle<Self>) -> Listener {
        Listener::new(move |app| self.set_state(app, |_| {}))
    }

    /// A short list of feeds in `mode`.
    fn feeds(
        self: Handle<Self>,
        mode: ItemsViewSelectionMode,
        selection: Handle<ItemsViewSelection>,
    ) -> WidgetRef {
        let view = ItemsView::new(FEEDS.len(), |_, _, index| feed_row(index))
            .selection_mode(mode)
            .selection(selection)
            .selection_changed(move |app| self.changed().call(app));
        SizedBox::new().height(200.0).child(view).into_widget()
    }

    fn caption(self: Handle<Self>, app: &App, selection: Handle<ItemsViewSelection>) -> String {
        let selected: Vec<&str> = selection
            .selected_indices(app)
            .into_iter()
            .map(|index| FEEDS[index].0)
            .collect();
        if selected.is_empty() {
            "Nothing selected".to_owned()
        } else {
            format!("Selected: {}", selected.join(", "))
        }
    }
}

/// A feed's row: its name and a second line, in an `ItemContainer`.
fn feed_row(index: usize) -> ItemContainer {
    let (name, detail) = FEEDS[index];
    ItemContainer::new(row_content(name.to_owned(), detail.to_owned()))
}

/// Two lines at the container's usual padding.
fn row_content(title: String, detail: String) -> WidgetRef {
    Builder::new(move |app, context| {
        let r = ThemeResources::of(app, context);
        Padding::new(EdgeInsetsGeometry::from_ltrb(12.0, 8.0, 36.0, 8.0))
            .child(
                Column::new()
                    .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Start)
                    .children([
                        Text::new(title.clone())
                            .style(control_text_style(
                                &r,
                                14.0,
                                FontWeight::W600,
                                r.text_fill_color_primary,
                            ))
                            .into_widget(),
                        Text::new(detail.clone())
                            .style(control_text_style(
                                &r,
                                12.0,
                                FontWeight::NORMAL,
                                r.text_fill_color_secondary,
                            ))
                            .into_widget(),
                    ]),
            )
            .into_widget()
    })
    .into_widget()
}

impl State for ItemsViewDemoState {
    type Widget = ItemsViewDemo;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let made: Vec<_> = (0..4).map(|_| ItemsViewSelection::new(app)).collect();
        let state = app.get_mut(self);
        state.single = Some(made[0]);
        state.multiple = Some(made[1]);
        state.extended = Some(made[2]);
        state.long = Some(made[3]);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let all = [
            state.single.take(),
            state.multiple.take(),
            state.extended.take(),
            state.long.take(),
        ];
        for selection in all.into_iter().flatten() {
            selection.dispose(app);
            app.destroy(selection);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let secondary = resources.text_fill_color_secondary;
        let single = self.selection(app, |s| s.single);
        let multiple = self.selection(app, |s| s.multiple);
        let extended = self.selection(app, |s| s.extended);
        let long = self.selection(app, |s| s.long);
        let invoked = app
            .get(self)
            .invoked
            .map_or("Nothing invoked".to_owned(), |index| {
                format!("Opened {}", FEEDS[index].0)
            });
        let single_view = SizedBox::new()
            .height(200.0)
            .child(
                ItemsView::new(FEEDS.len(), |_, _, index| feed_row(index))
                    .selection(single)
                    .is_item_invoked_enabled(true)
                    .item_invoked(move |app, index| {
                        self.set_state(app, |state| state.invoked = Some(index))
                    })
                    .selection_changed(move |app| self.changed().call(app)),
            )
            .into_widget();
        let long_view = SizedBox::new()
            .height(240.0)
            .child(
                ItemsView::new(LONG_COUNT, |_, _, index| {
                    ItemContainer::new(SizedBox::new().height(LONG_ROW).child(row_content(
                        format!("Item {}", index + 1),
                        format!("Row {index} of {LONG_COUNT}"),
                    )))
                })
                .selection_mode(ItemsViewSelectionMode::Extended)
                .selection(long)
                .item_extent(LONG_ROW)
                .selection_changed(move |app| self.changed().call(app)),
            )
            .into_widget();
        let long_caption = format!(
            "{} of {LONG_COUNT} selected",
            long.selected_indices(app).len()
        );
        column(
            vec![
                example(
                    "Single selection and invoking",
                    "A click or the arrow keys select one feed; a double click or Enter opens it.",
                    &resources,
                    column(
                        vec![
                            single_view,
                            label(
                                self.caption(app, single),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                            label(invoked, TextBlockStyle::Caption, secondary),
                        ],
                        8.0,
                    ),
                ),
                example(
                    "Multiple selection",
                    "Each click toggles a feed's check box; Shift extends from the last one clicked, and Control+A selects all.",
                    &resources,
                    column(
                        vec![
                            self.feeds(ItemsViewSelectionMode::Multiple, multiple),
                            label(
                                self.caption(app, multiple),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        8.0,
                    ),
                ),
                example(
                    "Extended selection",
                    "A click selects one feed; Control adds or removes one, Shift selects a range, Shift with the arrow keys extends it.",
                    &resources,
                    column(
                        vec![
                            self.feeds(ItemsViewSelectionMode::Extended, extended),
                            label(
                                self.caption(app, extended),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        8.0,
                    ),
                ),
                example(
                    "Ten thousand rows",
                    "Only the rows in sight are built. Page Down, End and Home move through the whole list; Control+A selects every row.",
                    &resources,
                    column(
                        vec![
                            long_view,
                            label(long_caption, TextBlockStyle::Caption, secondary),
                        ],
                        8.0,
                    ),
                ),
            ],
            20.0,
        )
    }
}

//! `ReorderableListView`: a list the user reorders by dragging.
//!
//! The same three tiles reordered by a drag handle the page places and by a long press on the
//! tile; the builder form with the platform's default drag handles (a handle on desktop, a long
//! press on mobile); a list with a header and a footer that stay where they are; and a
//! horizontal list. Every list reports its order in a caption.

use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_material::{Card, Icons, ListTile, ReorderableListView};
use waft_painting::Axis;
use waft_widgets::{
    BuildContext, Center, Column, Icon, IntoWidget, KeyRef, ListView, NeverScrollableScrollPhysics,
    ReorderableDelayedDragStartListener, ReorderableDragStartListener, SizedBox, State, StateData,
    StatefulWidget, StatelessWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ReorderableListsPage.into_widget()
}

/// The page: a labelled demo for each way to reorder.
#[derive(Debug)]
struct ReorderableListsPage;

impl StatelessWidget for ReorderableListsPage {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut children = Vec::new();
        for demo in Demo::ALL {
            children.push(section_header(app, context, demo.title()));
            children.push(caption(app, context, demo.description()));
            children.push(ReorderDemo { demo }.into_widget());
        }
        page(
            Entry::ReorderableListView.title(),
            ListView::new().children(children),
        )
        .into_widget()
    }
}

/// The ways a list in this page is reordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Demo {
    /// A handle the page places in each tile, with the default handles off.
    Handle,
    /// A long press anywhere on the tile, with the default handles off.
    LongPress,
    /// The builder form with the platform's default handles.
    Builder,
    /// A header and a footer around the reorderable tiles.
    HeaderAndFooter,
    /// A list that scrolls along the horizontal axis.
    Horizontal,
}

impl Demo {
    const ALL: [Demo; 5] = [
        Demo::Handle,
        Demo::LongPress,
        Demo::Builder,
        Demo::HeaderAndFooter,
        Demo::Horizontal,
    ];

    fn title(self) -> &'static str {
        match self {
            Demo::Handle => "Drag handle",
            Demo::LongPress => "Long press",
            Demo::Builder => "Builder",
            Demo::HeaderAndFooter => "Header and footer",
            Demo::Horizontal => "Horizontal",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Demo::Handle => {
                "Drag a tile by the handle at its end; the list has no default handles."
            }
            Demo::LongPress => "Press and hold a tile, then drag it.",
            Demo::Builder => {
                "Items built on demand. A handle at the end of each tile on desktop, a long press \
                 on mobile."
            }
            Demo::HeaderAndFooter => "The header and the footer do not move.",
            Demo::Horizontal => "Items along the horizontal axis.",
        }
    }

    /// The items in their first order; each name is its item's key.
    fn names(self) -> &'static [&'static str] {
        match self {
            Demo::Handle => &["Red", "Green", "Blue"],
            Demo::LongPress => &["Cat", "Dog", "Fox"],
            Demo::Builder => &["Mercury", "Venus", "Earth", "Mars"],
            Demo::HeaderAndFooter => &["Alpha", "Beta", "Gamma"],
            Demo::Horizontal => &["North", "East", "South", "West"],
        }
    }
}

/// One demo list, with the order the user has given it.
#[derive(Debug)]
struct ReorderDemo {
    demo: Demo,
}

/// Dart would spell this `_ReorderDemoState`.
struct ReorderDemoState {
    state: StateData<ReorderDemo>,
    order: Vec<&'static str>,
}

impl StatefulWidget for ReorderDemo {
    type State = ReorderDemoState;

    fn create_state(&self) -> ReorderDemoState {
        ReorderDemoState {
            state: StateData::new(),
            order: self.demo.names().to_vec(),
        }
    }
}

impl State for ReorderDemoState {
    type Widget = ReorderDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let demo = self.widget(app).demo;
        let order = app.get(self).order.clone();
        let reading = format!("Order: {}", order.join(", "));
        let list = demo_list(demo, order, move |app, old_index, new_index| {
            self.set_state(app, |state| {
                let name = state.order.remove(old_index as usize);
                state.order.insert(new_index as usize, name);
            });
        });
        Column::new()
            .children([list, caption(app, context, reading)])
            .into_widget()
    }
}

/// A tile's key: the name it shows.
fn name_key(name: &str) -> KeyRef {
    waft_foundation::ValueKey::new(name.to_string()).into()
}

/// The tile every list but the horizontal one shows, `name` its key's value.
fn tile(name: &'static str) -> ListTile {
    ListTile::new().key(name_key(name)).title(Text::new(name))
}

/// The list of `demo` in `order`, each reorder reported to `on_reorder`.
///
/// Nested in the page's own scroll view, a list takes the height of its items and leaves the
/// scrolling to the page.
fn demo_list(
    demo: Demo,
    order: Vec<&'static str>,
    on_reorder: impl Fn(&mut App, i32, i32) + 'static,
) -> WidgetRef {
    let nested = |list: ReorderableListView| {
        list.shrink_wrap(true)
            .primary(false)
            .physics(Rc::new(NeverScrollableScrollPhysics::new()))
    };
    match demo {
        Demo::Handle => nested(
            ReorderableListView::new(order.iter().enumerate().map(|(index, name)| {
                tile(name)
                    .trailing(ReorderableDragStartListener::new(
                        Icon::new(Some(Icons::drag_handle())),
                        index as i32,
                    ))
                    .into_widget()
            }))
            .build_default_drag_handles(false)
            .on_reorder_item(on_reorder),
        )
        .into_widget(),
        Demo::LongPress => nested(
            ReorderableListView::new(order.iter().enumerate().map(|(index, name)| {
                ReorderableDelayedDragStartListener::new(tile(name), index as i32)
                    .key(name_key(name))
                    .into_widget()
            }))
            .build_default_drag_handles(false)
            .on_reorder_item(on_reorder),
        )
        .into_widget(),
        Demo::Builder => nested(
            ReorderableListView::builder(
                {
                    let order = order.clone();
                    move |_app, _context, index| tile(order[index as usize]).into_widget()
                },
                order.len() as i32,
            )
            .on_reorder_item(on_reorder),
        )
        .into_widget(),
        Demo::HeaderAndFooter => nested(
            ReorderableListView::new(order.iter().map(|name| tile(name).into_widget()))
                .header(ListTile::new().title(Text::new("Header")))
                .footer(ListTile::new().title(Text::new("Footer")))
                .on_reorder_item(on_reorder),
        )
        .into_widget(),
        Demo::Horizontal => SizedBox::new()
            .height(112.0)
            .child(
                ReorderableListView::builder(
                    {
                        let order = order.clone();
                        move |_app, _context, index| {
                            let name = order[index as usize];
                            SizedBox::new()
                                .key(name_key(name))
                                .width(104.0)
                                .child(Card::new().child(Center::new().child(Text::new(name))))
                                .into_widget()
                        }
                    },
                    order.len() as i32,
                )
                .scroll_direction(Axis::Horizontal)
                .primary(false)
                .on_reorder_item(on_reorder),
            )
            .into_widget(),
    }
}

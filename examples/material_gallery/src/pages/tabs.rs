//! Tabs: a primary bar over swipeable pages, a secondary bar, and a scrollable bar whose tabs
//! do not fit the width.
//!
//! Each bar gets its `TabController` from the `DefaultTabController` above it.

use waft_foundation::App;
use waft_material::{DefaultTabController, Icons, Tab, TabBar, TabBarView};
use waft_widgets::{
    BuildContext, Center, Column, Icon, IntoWidget, ListView, SizedBox, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

const PAGE_HEIGHT: f64 = 120.0;
const SCROLLABLE_TABS: [&str; 8] = [
    "Overview",
    "Specs",
    "Reviews",
    "Questions",
    "Shipping",
    "Returns",
    "Warranty",
    "Related",
];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Primary"),
                primary(),
                caption(
                    app,
                    context,
                    "Icons over text; swipe the pages or tap a tab.",
                ),
                section_header(app, context, "Secondary"),
                secondary(),
                section_header(app, context, "Scrollable"),
                scrollable(),
            ])
            .into_widget()
    });
    page(Entry::Tabs.title(), content).into_widget()
}

fn pages(labels: &[&str]) -> WidgetRef {
    SizedBox::new()
        .height(PAGE_HEIGHT)
        .child(TabBarView::new(labels.iter().map(|label| {
            Center::new()
                .child(Text::new(format!("{label} page")))
                .into_widget()
        })))
        .into_widget()
}

fn tabbed(bar: TabBar, labels: &[&str]) -> WidgetRef {
    DefaultTabController::new(
        labels.len(),
        Column::new().children([bar.into_widget(), pages(labels)]),
    )
    .into_widget()
}

fn primary() -> WidgetRef {
    let tabs = [
        ("Flights", Icons::flight()),
        ("Transit", Icons::directions_transit()),
        ("Bikes", Icons::directions_bike()),
    ];
    let labels: Vec<&str> = tabs.iter().map(|(label, _)| *label).collect();
    tabbed(
        TabBar::new(tabs.iter().map(|(label, icon)| {
            Tab::new()
                .icon(Icon::new(Some(icon.clone())))
                .text(*label)
                .into_widget()
        })),
        &labels,
    )
}

fn secondary() -> WidgetRef {
    let labels = ["Videos", "Photos", "Audio"];
    tabbed(
        TabBar::secondary(
            labels
                .iter()
                .map(|label| Tab::new().text(*label).into_widget()),
        ),
        &labels,
    )
}

fn scrollable() -> WidgetRef {
    tabbed(
        TabBar::new(
            SCROLLABLE_TABS
                .iter()
                .map(|label| Tab::new().text(*label).into_widget()),
        )
        .is_scrollable(true),
        &SCROLLABLE_TABS,
    )
}

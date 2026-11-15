//! `SliverAppBar`: the large app bar that collapses as the list rises under it, and, one page
//! further, a floating app bar that snaps back in on the first scroll towards the top.

use waft_foundation::{App, Listener};
use waft_material::{Icons, ListTile, Scaffold, SliverAppBar};
use waft_widgets::{BuildContext, CustomScrollView, Icon, IntoWidget, SliverList, Text, WidgetRef};

use crate::app::open_sub_page;
use crate::catalog::Entry;

pub const SUB_TITLE: &str = "Floating App Bar";

/// How many filler tiles each page scrolls through: enough to scroll the bar fully away.
const ITEM_COUNT: usize = 40;

/// A large app bar over a long list; the first tile opens the floating app bar.
pub fn body(_app: &mut App, context: BuildContext) -> WidgetRef {
    let mut children = vec![
        ListTile::new()
            .leading(Icon::new(Some(Icons::vertical_split())))
            .title(Text::new(SUB_TITLE))
            .subtitle(Text::new("Floats back in, and snaps"))
            .trailing(Icon::new(Some(Icons::chevron_right())))
            .on_tap(Listener::new(move |app: &mut App| {
                open_sub_page(app, context, Entry::AppBars, 0);
            }))
            .into_widget(),
    ];
    children.extend(items());
    Scaffold::new()
        .body(CustomScrollView::new().slivers(vec![
            SliverAppBar::large()
                .title(Text::new(Entry::AppBars.title()))
                .into_widget(),
            SliverList::list(children).build().into_widget(),
        ]))
        .into_widget()
}

/// A floating, snapping app bar: it leaves with the first scroll down and comes back whole on
/// the first scroll up, wherever the list is.
pub fn sub_body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    Scaffold::new()
        .body(CustomScrollView::new().slivers(vec![
            SliverAppBar::new()
                .title(Text::new(SUB_TITLE))
                .floating(true)
                .snap(true)
                .into_widget(),
            SliverList::list(items()).build().into_widget(),
        ]))
        .into_widget()
}

fn items() -> Vec<WidgetRef> {
    (1..=ITEM_COUNT)
        .map(|index| {
            ListTile::new()
                .title(Text::new(format!("Item {index}")))
                .into_widget()
        })
        .collect()
}

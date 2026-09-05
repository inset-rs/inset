//! Scrolling under a navigation bar.
//!
//! This is the one screen whose bar is a sliver. Scroll the list and watch it: the large title
//! shrinks into the middle slot as the rows rise past it, and the background and hairline fade
//! in over the first ten points. Neither is a scroll callback this page wired up — the bar is a
//! `SliverPersistentHeader`, and the fade reaches it through `ScrollNotificationObserver`.

use reveal_cupertino::{
    CupertinoColors, CupertinoListSection, CupertinoListTile, CupertinoPageScaffold,
    CupertinoSliverNavigationBar,
};
use reveal_foundation::App;
use reveal_widgets::{
    BuildContext, CustomScrollView, IntoWidget, SliverList, SliverSafeArea, Text, WidgetRef,
};

/// Ten rows each. Enough sections that the list is genuinely longer than the viewport.
const SECTIONS: usize = 6;

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let mut slivers = vec![
        // The bar is a sliver here, not the scaffold's `navigation_bar`: that is the whole
        // point of the screen. `stretch` lets the large title grow when the list is dragged
        // past its top.
        CupertinoSliverNavigationBar::new()
            .large_title(Text::new("Scrolling"))
            .stretch(true)
            .into_widget(),
    ];
    slivers.push(
        // `top: false` — the bar above already consumed the top inset. The bottom one is still
        // ours, so the last row clears the home indicator.
        SliverSafeArea::new(
            SliverList::list((0..SECTIONS).map(rows_section).collect::<Vec<_>>()).build(),
        )
        .top(false)
        .into_widget(),
    );

    CupertinoPageScaffold::new(CustomScrollView::new().slivers(slivers))
        .background_color(CupertinoColors::SYSTEM_GROUPED_BACKGROUND)
        .into_widget()
}

/// One section per ten rows, so the group boundaries give the scroll something to measure
/// itself against.
fn rows_section(offset: usize) -> WidgetRef {
    let first = offset * 10;
    CupertinoListSection::inset_grouped()
        .header(Text::new(format!(
            "Rows {}\u{2013}{}",
            first + 1,
            first + 10
        )))
        .children(
            (first..first + 10)
                .map(|index| {
                    CupertinoListTile::notched(Text::new(format!("Row {}", index + 1)))
                        .into_widget()
                })
                .collect::<Vec<_>>(),
        )
        .into_widget()
}

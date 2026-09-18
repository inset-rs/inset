//! The catalog index.
//!
//! It is a `CupertinoListSection` of `CupertinoListTile`s — the same widgets the "Lists &
//! Forms" entry demonstrates. The index is not a mock-up of iOS Settings; it is the
//! framework's own list section, rendering itself.

use std::rc::Rc;

use inset_cupertino::{
    CupertinoDynamicColor, CupertinoListTile, CupertinoListTileChevron,
    CupertinoSliverNavigationBar,
};
use inset_foundation::{App, Task};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::MainAxisSize;
use inset_widgets::{BuildContext, IntoWidget, Padding, Row, StatelessWidget, Text, WidgetRef};

use crate::app::{INDEX_TITLE, open_entry};
use crate::catalog::Entry;
use crate::support::{badge, secondary, section, sliver_screen};

/// The leading badge's side. `CupertinoListTile::notched` constrains `leading` to 30 points,
/// so the badge is built to that size rather than being squeezed into it.
const BADGE_SIZE: f64 = 30.0;

/// The gallery's index screen.
#[derive(Debug)]
pub struct Home;

impl StatelessWidget for Home {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let rows = Entry::ALL
            .iter()
            .map(|entry| row(*entry, app, context))
            .collect();

        sliver_screen(
            // The index's bar is a SLIVER, so its large title collapses into the middle slot
            // as the rows rise past it. Most entries wear the standard fixed bar, so a push
            // shows that title fly into a centred middle — two bars, one gesture apart.
            CupertinoSliverNavigationBar::new().large_title(Text::new(INDEX_TITLE)),
            vec![section("Widgets", rows)],
        )
    }
}

fn row(entry: Entry, app: &mut App, context: BuildContext) -> WidgetRef {
    let tint = CupertinoDynamicColor::resolve(&entry.tint(), app, context);
    CupertinoListTile::notched(Text::new(entry.title()))
        .leading(badge(entry.icon(), tint, BADGE_SIZE))
        // The greyed value and the chevron, as in iOS Settings. `trailing` is unconstrained by
        // design, so the row packs them itself and asks for no more width than it needs.
        .trailing(
            Row::new().main_axis_size(MainAxisSize::Min).children([
                secondary(app, context, entry.summary()),
                Padding::new(EdgeInsetsGeometry::from_steb(6.0, 0.0, 0.0, 0.0))
                    .child(CupertinoListTileChevron::new())
                    .into_widget(),
            ]),
        )
        .on_tap(Rc::new(move |app: &mut App| {
            open_entry(app, context, entry);
            Task::ready(())
        }))
        .into_widget()
}

//! Navigation bars: the two constructors, the automatic leading and middle, the back label's
//! twelve-character rule, and the hero flight the bars take between routes.
//!
//! The demonstration is the chain of pushes itself. Each screen's title is chosen so the NEXT
//! screen's back button shows something different, which is the only way that rule is visible
//! at all.

use reveal_cupertino::{CupertinoListTile, CupertinoListTileChevron, CupertinoNavigationBar};
use reveal_foundation::{App, Listener};
use reveal_widgets::{BuildContext, IntoWidget, Text, WidgetRef};

use crate::app::open_sub_page;
use crate::catalog::Entry;
use crate::support::{screen, screen_with_bar, scrolling_body, secondary, section_with_footer};

/// The chain, in push order. Each is the previous screen's back label.
pub const SUB_TITLES: [&str; 3] = [
    // Eleven characters — under the twelve the back label keeps verbatim.
    "Second Item",
    // Well over twelve, so the screen after this one says "Back".
    "A Very Long Third Title",
    "Back",
];

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    screen(scrolling_body(vec![section_with_footer(
        "Push a screen",
        "This title is ten characters, so the next screen's back button repeats it verbatim.",
        vec![push_row(0, "Standard bar, short title", app, context)],
    )]))
}

pub fn sub_body(index: usize, app: &mut App, context: BuildContext) -> WidgetRef {
    match index {
        // The trailing slot, so a push shows it flying too: `trailing` is one of the components
        // a bar hero carries across.
        0 => screen_with_bar(
            CupertinoNavigationBar::new().trailing(Text::new("Edit")),
            scrolling_body(vec![section_with_footer(
                "Push a screen",
                "The bar grew a trailing, and it flew here with the rest of the bar.",
                vec![push_row(1, "Large bar, long title", app, context)],
            )]),
        ),
        // The large constructor, arriving from a collapsed one: the title unfolds onto its own
        // row and folds back on the way out.
        1 => screen_with_bar(
            CupertinoNavigationBar::large().large_title(Text::new(SUB_TITLES[1])),
            scrolling_body(vec![section_with_footer(
                "Push a screen",
                "Twenty-three characters, so the next back button will just say \"Back\".",
                vec![push_row(2, "No route transition", app, context)],
            )]),
        ),
        // The one bar in the gallery that opts out. Arriving here the bar SLIDES with its route
        // instead of flying over it — the difference between this push and the previous two is
        // what the flag does.
        2 => screen_with_bar(
            CupertinoNavigationBar::new().transition_between_routes(false),
            scrolling_body(vec![section_with_footer(
                "transition_between_routes: false",
                "This bar has no hero: it slid in with the page instead of flying over it.",
                vec![],
            )]),
        ),
        _ => panic!("navigation has no sub-page {index}"),
    }
}

fn push_row(index: usize, caption: &str, app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoListTile::notched(Text::new(SUB_TITLES[index]))
        .subtitle(secondary(app, context, caption))
        .trailing(CupertinoListTileChevron::new())
        .on_tap(Listener::new(move |app: &mut App| {
            open_sub_page(app, context, Entry::Navigation, index);
        }))
        .into_widget()
}

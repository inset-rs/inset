//! `CupertinoPageScaffold`: what an obstructing navigation bar does to the body underneath it.
//!
//! The two screens here differ in one field — the bar's background alpha — and the band lands
//! in the same place on both. That is the point worth taking away: a translucent bar hands the
//! obstructed strip to the body as padding and a `SafeArea` consumes it, an opaque bar has the
//! scaffold move the body by the same amount, and the two mechanisms arrive at the same static
//! layout.
//!
//! Which means you cannot see the difference in a still picture. You see it when content MOVES:
//! under a translucent bar it passes behind the blur, under an opaque one it disappears. The
//! Scrolling entry is where that happens, and the footer below points at it rather than
//! rebuilding it here.

use reveal_cupertino::{
    CupertinoColors, CupertinoDynamicColor, CupertinoListTile, CupertinoListTileChevron,
    CupertinoNavigationBar,
};
use reveal_foundation::{App, Listener};
use reveal_painting::{AlignmentGeometry, BoxDecoration, EdgeInsetsGeometry};
use reveal_rendering::CrossAxisAlignment;
use reveal_widgets::{BuildContext, Column, Container, IntoWidget, SafeArea, Text, WidgetRef};

use crate::app::open_sub_page;
use crate::catalog::Entry;
use crate::support::{body_style, screen, screen_with_bar, secondary, section_with_footer};

pub const SUB_TITLE: &str = "Opaque Bar";

/// The theme's default bar background is `0xF0`-alpha, so `should_fully_obstruct` is false: the
/// scaffold does NOT move the body down. It hands the obstructed strip to the body as
/// `MediaQuery` padding and lets content paint behind the blur.
///
/// `SafeArea` is what consumes that padding, and leaving it out is how the first row of a page
/// ends up underneath the bar. Flutter's own scaffold behaves the same way and its examples
/// wrap the body the same way.
pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    screen(SafeArea::new(column(vec![
        band(app, context, "Body starts here"),
        section_with_footer(
            "Translucent bar",
            "The body begins at the top of the window; the SafeArea is what moved the band.",
            vec![push_row(app, context)],
        ),
    ])))
}

/// An opaque bar fully obstructs, so the scaffold shifts the body down by the bar's height and
/// there is no top padding left for a `SafeArea` to consume — which is why this body has none.
pub fn sub_body(app: &mut App, context: BuildContext) -> WidgetRef {
    screen_with_bar(
        CupertinoNavigationBar::new().background_color(CupertinoColors::SYSTEM_BACKGROUND),
        column(vec![
            band(app, context, "Body starts here"),
            section_with_footer(
                "Opaque bar",
                "No SafeArea here — the scaffold moved the body itself, to the same place.",
                vec![],
            ),
        ]),
    )
}

/// A full-width marker for the body's first pixel, so the two screens can be compared by eye —
/// and so it is visible that they agree.
fn band(app: &mut App, context: BuildContext, label: &str) -> WidgetRef {
    let fill = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_YELLOW, app, context);
    Container::new()
        .alignment(AlignmentGeometry::CENTER)
        .padding(EdgeInsetsGeometry::symmetric(8.0, 0.0))
        .decoration(BoxDecoration::new().color(fill))
        .child(Text::new(label).style(body_style(app, context)))
        .into_widget()
}

/// A fixed, non-scrolling body — the case `SafeArea` is for.
fn column(children: Vec<WidgetRef>) -> WidgetRef {
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .children(children)
        .into_widget()
}

fn push_row(app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoListTile::notched(Text::new(SUB_TITLE))
        .subtitle(secondary(
            app,
            context,
            "Same band, opaque bar, no SafeArea",
        ))
        .trailing(CupertinoListTileChevron::new())
        .on_tap(Listener::new(move |app: &mut App| {
            open_sub_page(app, context, Entry::PageScaffold, 0);
        }))
        .into_widget()
}

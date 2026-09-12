//! Light and dark, and what a theme's brightness reaches.
//!
//! A `CupertinoDynamicColor` is not one colour with a light and a dark spelling picked by the
//! app — it resolves itself against the ambient theme. The two cards below hold the SAME
//! constants; only the theme wrapped around each differs.

use std::rc::Rc;

use inset_cupertino::CupertinoDynamicColor;
use inset_cupertino::{
    CupertinoColors, CupertinoListTile, CupertinoListTileChevron, CupertinoNavigationBar,
    CupertinoTheme, CupertinoThemeData,
};
use inset_embedder::Brightness;
use inset_foundation::{App, Task};
use inset_painting::{AnyColor, BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry};
use inset_rendering::{CrossAxisAlignment, MainAxisAlignment};
use inset_widgets::{
    BuildContext, Builder, Column, Container, IntoWidget, Padding, Row, SizedBox, Text, WidgetRef,
};

use crate::app::open_sub_page;
use crate::catalog::Entry;
use crate::support::{
    body_style, screen, screen_with_bar, scrolling_body, secondary, section_with_footer, showcase,
};

pub const SUB_TITLE: &str = "Dark Screen";

/// The constants each card resolves. Same list, both sides.
const SWATCHES: [(&str, AnyColor); 5] = [
    ("LABEL", CupertinoColors::LABEL),
    ("SECONDARY_LABEL", CupertinoColors::SECONDARY_LABEL),
    ("SEPARATOR", CupertinoColors::SEPARATOR),
    ("SYSTEM_BLUE", CupertinoColors::SYSTEM_BLUE),
    ("SYSTEM_FILL", CupertinoColors::SYSTEM_FILL),
];

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    screen(scrolling_body(vec![
        section_with_footer(
            "Resolved against the theme",
            "Same five constants either side. Only the theme around them differs.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .children([card(Brightness::Light), card(Brightness::Dark)]),
            )],
        ),
        section_with_footer(
            "Brightness leaves the app",
            "Brightness also becomes the status-bar style sent to the platform each frame.",
            vec![push_row(app, context)],
        ),
    ]))
}

/// A whole screen under a dark theme — the bar, the scaffold background and every resolved
/// colour at once.
pub fn sub_body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    CupertinoTheme::new(
        CupertinoThemeData::new().with_brightness(Brightness::Dark),
        // The rows are built INSIDE the theme, not handed to it. A swatch is a concrete colour
        // by the time it reaches `chip`, and `resolve` walks inherited widgets UPWARDS — so
        // resolving with this function's own context would ask an ancestor of the theme being
        // constructed, and every chip would come back in the platform brightness while the bar
        // and the text around them went dark.
        Builder::new(|app: &mut App, context: BuildContext| {
            screen_with_bar(
                CupertinoNavigationBar::large().large_title(Text::new(SUB_TITLE)),
                scrolling_body(vec![section_with_footer(
                    "Dark",
                    "Nothing on this screen names a dark colour. The theme above it does.",
                    SWATCHES
                        .iter()
                        .map(|(name, color)| swatch_row(name, color, app, context))
                        .collect(),
                )]),
            )
        }),
    )
    .into_widget()
}

fn card(brightness: Brightness) -> WidgetRef {
    CupertinoTheme::new(
        CupertinoThemeData::new().with_brightness(brightness),
        Builder::new(move |app: &mut App, context: BuildContext| {
            let background =
                CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_BACKGROUND, app, context);
            Container::new()
                .padding(EdgeInsetsGeometry::all(10.0))
                .width(150.0)
                .decoration(
                    BoxDecoration::new()
                        .color(background)
                        .border_radius(BorderRadiusGeometry::circular(10.0)),
                )
                .child(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .children(
                            SWATCHES
                                .iter()
                                .map(|(name, color)| swatch(name, color, app, context))
                                .collect::<Vec<_>>(),
                        ),
                )
                .into_widget()
        }),
    )
    .into_widget()
}

fn swatch(name: &str, color: &AnyColor, app: &mut App, context: BuildContext) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(3.0, 0.0))
        .child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .children([
                    chip(color, app, context),
                    Padding::new(EdgeInsetsGeometry::from_ltrb(6.0, 0.0, 0.0, 0.0))
                        .child(Text::new(name).style(body_style(app, context).font_size(11.0)))
                        .into_widget(),
                ]),
        )
        .into_widget()
}

fn swatch_row(name: &str, color: &AnyColor, app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoListTile::notched(Text::new(name))
        .leading(chip(color, app, context))
        .leading_size(16.0)
        .into_widget()
}

fn chip(color: &AnyColor, app: &mut App, context: BuildContext) -> WidgetRef {
    let resolved = CupertinoDynamicColor::resolve(color, app, context);
    SizedBox::square(Some(14.0))
        .child(
            Container::new().decoration(
                BoxDecoration::new()
                    .color(resolved)
                    .border_radius(BorderRadiusGeometry::circular(3.0)),
            ),
        )
        .into_widget()
}

fn push_row(app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoListTile::notched(Text::new(SUB_TITLE))
        .subtitle(secondary(app, context, "A whole screen, dark"))
        .trailing(CupertinoListTileChevron::new())
        .on_tap(Rc::new(move |app: &mut App| {
            open_sub_page(app, context, Entry::Theming, 0);
            Task::ready(())
        }))
        .into_widget()
}

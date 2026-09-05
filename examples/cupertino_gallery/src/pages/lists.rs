//! The list and form family — the widgets the gallery's own index is built from.
//!
//! Two section styles, two tile styles, and the four slots a tile has. The divider inset is the
//! detail worth watching: a section with leading widgets starts its dividers past them, and
//! `has_leading(false)` is what tells it there are none.

use reveal_cupertino::{
    CupertinoColors, CupertinoDynamicColor, CupertinoFormRow, CupertinoFormSection, CupertinoIcons,
    CupertinoListSection, CupertinoListTile, CupertinoListTileChevron,
};
use reveal_foundation::App;
use reveal_widgets::{BuildContext, IntoWidget, Text, WidgetRef};

use crate::support::{badge, screen, scrolling_body, secondary, section_with_footer};

const BADGE_SIZE: f64 = 28.0;

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    let bell = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_RED, app, context);

    screen(scrolling_body(vec![
        section_with_footer(
            "Tile slots",
            "additional_info sits before trailing; subtitle goes under the title.",
            vec![
                CupertinoListTile::notched(Text::new("Title only")).into_widget(),
                CupertinoListTile::notched(Text::new("Title"))
                    .subtitle(secondary(app, context, "and a subtitle"))
                    .into_widget(),
                CupertinoListTile::notched(Text::new("Network"))
                    .additional_info(secondary(app, context, "Wi-Fi"))
                    .trailing(CupertinoListTileChevron::new())
                    .into_widget(),
                CupertinoListTile::notched(Text::new("Notifications"))
                    .leading(badge(CupertinoIcons::bell_fill(), bell, BADGE_SIZE))
                    .additional_info(secondary(app, context, "3"))
                    .trailing(CupertinoListTileChevron::new())
                    .into_widget(),
            ],
        ),
        // The base constructor: edge to edge, square corners, and a hairline above and below
        // the group. The inset-grouped sections everywhere else on this page are the other one.
        base_section(app, context),
        no_leading_section(),
        // A form section is a list section with form rows in it: the rows carry a prefix, a
        // helper line and an error line rather than a title and a subtitle.
        form_section(),
    ]))
}

fn base_section(app: &mut App, context: BuildContext) -> WidgetRef {
    let grey = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_GREY, app, context);
    let indigo = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_INDIGO, app, context);

    CupertinoListSection::new()
        .header(Text::new("Base section"))
        .footer(Text::new(
            "Edge to edge, square, bordered above and below — and ::new tiles, not ::notched.",
        ))
        .children([
            CupertinoListTile::new(Text::new("General"))
                .leading(badge(CupertinoIcons::gear_solid(), grey, BADGE_SIZE))
                .trailing(CupertinoListTileChevron::new())
                .into_widget(),
            CupertinoListTile::new(Text::new("Display"))
                .leading(badge(CupertinoIcons::moon_fill(), indigo, BADGE_SIZE))
                .trailing(CupertinoListTileChevron::new())
                .into_widget(),
        ])
        .into_widget()
}

fn no_leading_section() -> WidgetRef {
    CupertinoListSection::inset_grouped()
        .header(Text::new("Dividers without leading"))
        .footer(Text::new(
            "has_leading(false). The first section's dividers clear a 30-point leading; these \
             start further back.",
        ))
        .has_leading(false)
        .children([
            CupertinoListTile::notched(Text::new("Alpha")).into_widget(),
            CupertinoListTile::notched(Text::new("Beta")).into_widget(),
            CupertinoListTile::notched(Text::new("Gamma")).into_widget(),
        ])
        .into_widget()
}

fn form_section() -> WidgetRef {
    CupertinoFormSection::inset_grouped([
        CupertinoFormRow::new(Text::new("reveal"))
            .prefix(Text::new("Framework"))
            .into_widget(),
        CupertinoFormRow::new(Text::new("valo"))
            .prefix(Text::new("Backend"))
            .helper(Text::new("The paint backend in use"))
            .into_widget(),
        // Both lines at once, which is the thing worth seeing: the error does not replace the
        // helper, it is appended under it in the destructive colour.
        CupertinoFormRow::new(Text::new("en_US"))
            .prefix(Text::new("Locale"))
            .helper(Text::new("Reported by the platform"))
            .error(Text::new("No platform source for the locale yet"))
            .into_widget(),
    ])
    .header(Text::new("Form section"))
    .footer(Text::new(
        "prefix labels the row. helper and error are separate lines: the last row sets both.",
    ))
    .into_widget()
}

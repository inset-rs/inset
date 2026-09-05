//! The Cupertino icon set.
//!
//! `CupertinoIcons` carries all 1,322 of Flutter's constants; this page shows a readable sample
//! of them beside their names. Two things worth noticing: the glyphs come from a font
//! registered by `install_cupertino_icon_font`, and the two arrows at the bottom mirror under a
//! right-to-left `Directionality` because their `IconData` sets `match_text_direction`.

use reveal_cupertino::{CupertinoColors, CupertinoDynamicColor, CupertinoIcons, CupertinoListTile};
use reveal_embedder::TextDirection;
use reveal_foundation::App;
use reveal_widgets::{BuildContext, Directionality, Icon, IconData, IntoWidget, Text, WidgetRef};

use crate::support::{screen, scrolling_body, secondary, section_with_footer};

/// A sample, named as the constant is spelled, so a reader can go straight from a glyph to the
/// identifier.
fn sample() -> Vec<(&'static str, IconData)> {
    vec![
        ("house_fill", CupertinoIcons::house_fill()),
        ("search", CupertinoIcons::search()),
        ("gear_solid", CupertinoIcons::gear_solid()),
        ("bell_fill", CupertinoIcons::bell_fill()),
        ("heart_fill", CupertinoIcons::heart_fill()),
        ("star_fill", CupertinoIcons::star_fill()),
        ("bookmark_solid", CupertinoIcons::bookmark_solid()),
        ("trash_fill", CupertinoIcons::trash_fill()),
        ("pencil", CupertinoIcons::pencil()),
        ("share", CupertinoIcons::share()),
        ("camera_fill", CupertinoIcons::camera_fill()),
        ("mic_fill", CupertinoIcons::mic_fill()),
        ("play_fill", CupertinoIcons::play_fill()),
        ("pause_fill", CupertinoIcons::pause_fill()),
        ("clock_fill", CupertinoIcons::clock_fill()),
        ("calendar", CupertinoIcons::calendar()),
        ("map_fill", CupertinoIcons::map_fill()),
        ("paperplane_fill", CupertinoIcons::paperplane_fill()),
        ("cloud_fill", CupertinoIcons::cloud_fill()),
        ("bolt_fill", CupertinoIcons::bolt_fill()),
        ("moon_fill", CupertinoIcons::moon_fill()),
        ("sun_max_fill", CupertinoIcons::sun_max_fill()),
        ("sparkles", CupertinoIcons::sparkles()),
        ("wand_stars", CupertinoIcons::wand_stars()),
    ]
}

/// The two whose glyphs point along the reading direction.
fn directional() -> Vec<(&'static str, IconData)> {
    vec![
        ("back", CupertinoIcons::back()),
        ("forward", CupertinoIcons::forward()),
    ]
}

const GLYPH_SIZE: f64 = 24.0;

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    let listed = sample()
        .into_iter()
        .map(|(name, icon)| row(icon, name, app, context))
        .collect();
    let mirrored = directional()
        .into_iter()
        .map(|(name, icon)| {
            CupertinoListTile::notched(Text::new(name))
                .leading(glyph(icon.clone(), app, context))
                // The SAME `IconData`, drawn against a right-to-left direction.
                // `match_text_direction` is the field that makes these two flip and the
                // twenty-four above stand still.
                .trailing(Directionality::new(
                    TextDirection::Rtl,
                    glyph(icon, app, context),
                ))
                .into_widget()
        })
        .collect();

    screen(scrolling_body(vec![
        section_with_footer(
            "A sample",
            "CupertinoIcons declares 1,322 icons, generated from Flutter's icons.dart.",
            listed,
        ),
        section_with_footer(
            "Direction-aware icons",
            "Left is the ambient left-to-right direction; right is the same icon under \
             right-to-left.",
            mirrored,
        ),
    ]))
}

fn row(icon: IconData, name: &str, app: &mut App, context: BuildContext) -> WidgetRef {
    let code_point = icon.code_point;
    CupertinoListTile::notched(Text::new(name))
        .leading(glyph(icon, app, context))
        .additional_info(secondary(app, context, format!("{code_point:#06x}")))
        .into_widget()
}

fn glyph(icon: IconData, app: &mut App, context: BuildContext) -> WidgetRef {
    let color = CupertinoDynamicColor::resolve(&CupertinoColors::LABEL, app, context);
    Icon::new(Some(icon))
        .size(GLYPH_SIZE)
        .color(color)
        .into_widget()
}

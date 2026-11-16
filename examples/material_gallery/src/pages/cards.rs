//! Containers and markers: the three `Card` variants, `Badge`s with a count, a label and as a
//! dot, `CircleAvatar`s with text and with an icon, and `ExpansionTile`s collapsed, expanded
//! and nested.

use waft_foundation::App;
use waft_material::{Badge, Card, CircleAvatar, ExpansionTile, Icons, ListTile};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{BuildContext, Icon, IntoWidget, ListView, Padding, SizedBox, Text, WidgetRef};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub const EXPANDED_TILE_CHILD: &str = "Shown when the tile is expanded";

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Cards"),
                showcase(cards()),
                caption(app, context, "Elevated, filled and outlined."),
                section_header(app, context, "Badges"),
                showcase(badges()),
                section_header(app, context, "Avatars"),
                showcase(avatars()),
                section_header(app, context, "Expansion tiles"),
                expansion_tiles(),
            ])
            .into_widget()
    });
    page(Entry::Cards.title(), content).into_widget()
}

fn card_content(label: &str) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::all(16.0))
        .child(SizedBox::new().width(96.0).child(Text::new(label)))
        .into_widget()
}

fn cards() -> Vec<WidgetRef> {
    vec![
        Card::new().child(card_content("Elevated")).into_widget(),
        Card::filled().child(card_content("Filled")).into_widget(),
        Card::outlined()
            .child(card_content("Outlined"))
            .into_widget(),
    ]
}

fn badges() -> Vec<WidgetRef> {
    let bell = || Icon::new(Some(Icons::notifications()));
    vec![
        Badge::count(3).child(bell()).into_widget(),
        Badge::count(1000).child(bell()).into_widget(),
        Badge::new()
            .label(Text::new("New"))
            .child(bell())
            .into_widget(),
        Badge::new().child(bell()).into_widget(),
        Badge::new()
            .is_label_visible(false)
            .child(bell())
            .into_widget(),
    ]
}

fn avatars() -> Vec<WidgetRef> {
    vec![
        CircleAvatar::new().child(Text::new("AB")).into_widget(),
        CircleAvatar::new()
            .child(Icon::new(Some(Icons::person())))
            .into_widget(),
        CircleAvatar::new()
            .radius(32.0)
            .child(Text::new("Big"))
            .into_widget(),
    ]
}

fn expansion_tiles() -> WidgetRef {
    let child = || {
        ListTile::new()
            .title(Text::new(EXPANDED_TILE_CHILD))
            .into_widget()
    };
    waft_widgets::Column::new()
        .children([
            ExpansionTile::new(Text::new("Collapsed tile"))
                .subtitle(Text::new("Tap to expand"))
                .children([child()])
                .into_widget(),
            ExpansionTile::new(Text::new("Initially expanded"))
                .leading(Icon::new(Some(Icons::info())))
                .initially_expanded(true)
                .children([child(), child()])
                .into_widget(),
        ])
        .into_widget()
}

//! Grid tiles: a plain tile, and tiles with a header and a footer bar over their child.

use waft_foundation::App;
use waft_material::{Colors, GridTile, GridTileBar, Icons};
use waft_widgets::{
    BuildContext, Center, ColoredBox, Icon, IntoWidget, ListView, SizedBox, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Grid tiles"),
                showcase(tiles()),
                caption(
                    app,
                    context,
                    "A header bar and a footer bar sit over a tile that fills its cell.",
                ),
            ])
            .into_widget()
    });
    page(Entry::GridTiles.title(), content).into_widget()
}

fn tile(size: f64, body: &'static str) -> SizedBox {
    SizedBox::new()
        .width(size)
        .height(size)
        .child(ColoredBox::new(Colors::BLUE.shade200()).child(Center::new().child(Text::new(body))))
}

fn tiles() -> Vec<WidgetRef> {
    vec![
        GridTile::new(tile(120.0, "Plain")).into_widget(),
        GridTile::new(tile(120.0, "Header"))
            .header(
                GridTileBar::new()
                    .title(Text::new("Title"))
                    .trailing(Icon::new(Some(Icons::more_vert()))),
            )
            .into_widget(),
        GridTile::new(tile(120.0, "Footer"))
            .footer(
                GridTileBar::new()
                    .title(Text::new("Title"))
                    .subtitle(Text::new("Subtitle"))
                    .leading(Icon::new(Some(Icons::person()))),
            )
            .into_widget(),
    ]
}

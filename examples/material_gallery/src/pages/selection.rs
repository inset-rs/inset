//! Selection: a `SelectionArea` over paragraphs, a short list and a card, so a drag selects
//! across all of them and the context menu copies what was selected.

use waft_foundation::App;
use waft_material::{Card, ListTile, SelectionArea};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, Builder, Column, IntoWidget, ListView, Padding, SizedBox, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

/// The first paragraph; the gallery's test reads it to know the page opened.
pub const INTRO: &str = "Drag across this text to select it. A selection can start here, run \
                         down through the list, and end inside the card at the bottom of the page.";

const SECOND: &str = "The context menu copies what is selected, or the platform's copy shortcut \
                      does. A click collapses the selection, a double click selects a word, and a \
                      triple click selects its paragraph.";

const LIST_TITLES: [&str; 4] = [
    "The list scrolls when a selection drag reaches its edge",
    "Every tile can be selected with the paragraphs",
    "Dragging past the edge scrolls it",
    "And the selection follows",
];

/// The card's text.
pub const CARD_TEXT: &str = "A card is not a text widget, but the text inside it is selectable \
                             as any other is.";

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Selection"),
                caption(
                    app,
                    context,
                    "Drag from a paragraph into the list or the card; the menu copies.",
                ),
                Padding::new(EdgeInsetsGeometry::symmetric(16.0, 16.0))
                    .child(selectable_content())
                    .into_widget(),
            ])
            .into_widget()
    });
    page(Entry::Selection.title(), content).into_widget()
}

/// The subtree a drag selects across: two paragraphs, a list and a card.
fn selectable_content() -> WidgetRef {
    SelectionArea::new(
        Column::new()
            .children(vec![
                Text::new(INTRO).into_widget(),
                Padding::new(EdgeInsetsGeometry::only(0.0, 12.0, 0.0, 12.0))
                    .child(Text::new(SECOND))
                    .into_widget(),
                SizedBox::new()
                    .height(180.0)
                    .child(
                        ListView::new()
                            .children(
                                LIST_TITLES
                                    .iter()
                                    .map(|title| {
                                        ListTile::new().title(Text::new(*title)).into_widget()
                                    })
                                    .collect::<Vec<WidgetRef>>(),
                            )
                            .into_widget(),
                    )
                    .into_widget(),
                Padding::new(EdgeInsetsGeometry::only(0.0, 12.0, 0.0, 0.0))
                    .child(
                        Card::new()
                            .child(
                                Padding::new(EdgeInsetsGeometry::all(16.0))
                                    .child(Text::new(CARD_TEXT))
                                    .into_widget(),
                            )
                            .into_widget(),
                    )
                    .into_widget(),
            ])
            .into_widget(),
    )
    .into_widget()
}

//! Button bars: the end-aligned row of buttons, in its padded and constrained layouts, and one
//! wrapped onto a second line.

use waft_foundation::{App, Listener};
use waft_material::{ButtonBar, ButtonBarLayoutBehavior, TextButton};
use waft_widgets::{BuildContext, IntoWidget, ListView, SizedBox, Text, WidgetRef};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Padded"),
                showcase(vec![
                    ButtonBar::new()
                        .children(vec![button("Cancel"), button("OK")])
                        .into_widget(),
                ]),
                section_header(app, context, "Constrained"),
                showcase(vec![
                    ButtonBar::new()
                        .layout_behavior(ButtonBarLayoutBehavior::Constrained)
                        .children(vec![button("Back"), button("Next")])
                        .into_widget(),
                ]),
                section_header(app, context, "Wrapped"),
                SizedBox::new()
                    .width(220.0)
                    .child(
                        ButtonBar::new()
                            .children(vec![button("One"), button("Two"), button("Three")])
                            .into_widget(),
                    )
                    .into_widget(),
                caption(
                    app,
                    context,
                    "A bar narrows into a column when its buttons do not fit.",
                ),
            ])
            .into_widget()
    });
    page(Entry::ButtonBars.title(), content).into_widget()
}

fn button(label: &'static str) -> WidgetRef {
    TextButton::new(
        Some(Listener::new(|_app: &mut App| {})),
        Some(Text::new(label)),
    )
    .into_widget()
}

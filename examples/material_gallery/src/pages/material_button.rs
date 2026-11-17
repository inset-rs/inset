//! The classic `MaterialButton`: the plain button with a fill, an outlined one, and a disabled
//! one that keeps its disabled colours.

use waft_foundation::{App, Listener};
use waft_material::{ButtonTextTheme, Colors, Material, MaterialButton};
use waft_painting::{BorderSide, EdgeInsets};
use waft_widgets::{BuildContext, IntoWidget, ListView, Text, WidgetRef};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Material buttons"),
                showcase(vec![
                    MaterialButton::new(Some(Listener::new(|_app: &mut App| {})))
                        .child(Text::new("Primary"))
                        .into_widget(),
                    MaterialButton::new(Some(Listener::new(|_app: &mut App| {})))
                        .text_theme(ButtonTextTheme::Normal)
                        .color(Colors::BLUE.shade50())
                        .child(Text::new("Normal"))
                        .into_widget(),
                    MaterialButton::new(Some(Listener::new(|_app: &mut App| {})))
                        .padding(EdgeInsets::symmetric(0.0, 12.0))
                        .shape(waft_painting::StadiumBorder::new(BorderSide::NONE))
                        .child(Text::new("Round"))
                        .into_widget(),
                    MaterialButton::new(None)
                        .child(Text::new("Disabled"))
                        .into_widget(),
                ]),
                caption(
                    app,
                    context,
                    "Left to right: primary, normal with a fill, stadium-shaped, disabled.",
                ),
                section_header(app, context, "In a material"),
                Material::new()
                    .child(
                        MaterialButton::new(Some(Listener::new(|_app: &mut App| {})))
                            .child(Text::new("On a raised surface")),
                    )
                    .into_widget(),
            ])
            .into_widget()
    });
    page(Entry::MaterialButton.title(), content).into_widget()
}

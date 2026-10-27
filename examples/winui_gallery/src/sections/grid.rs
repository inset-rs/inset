//! `Grid`: sizing, spacing, spans and child alignment.

use crate::{example, label, section};
use inset_embedder::Size;
use inset_painting::Alignment;
use inset_widgets::*;
use inset_winui::*;

/// Builds the grid sizing, spanning and alignment example.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    let text = resources.text_fill_color_primary;
    let cell = |name: &str, brush: Brush| {
        ControlBorder::new(brush, Brush::Solid(resources.control_stroke_color_default))
            .corner_radius(resources.control_corner_radius[0])
            .padding([8.0, 4.0, 8.0, 4.0])
            .child(label(name, TextBlockStyle::Caption, text))
    };
    let accent = Brush::Solid(resources.accent.light3);
    let fill = Brush::Solid(resources.control_fill_color_default);
    let grid = Grid::new()
        .column_definitions([
            ColumnDefinition::new(GridLength::AUTO),
            ColumnDefinition::new(GridLength::STAR),
            ColumnDefinition::new(GridLength::pixel(80.0)),
        ])
        .row_definitions([
            RowDefinition::new(GridLength::AUTO),
            RowDefinition::new(GridLength::STAR),
            RowDefinition::new(GridLength::AUTO),
        ])
        .row_spacing(8.0)
        .column_spacing(8.0)
        .children([
            GridCell::new(cell("Auto", fill)).into_widget(),
            GridCell::new(cell("*", fill)).column(1).into_widget(),
            GridCell::new(cell("80 px", fill)).column(2).into_widget(),
            GridCell::new(cell("Row 1, ColumnSpan 2", accent))
                .row(1)
                .column(1)
                .column_span(2)
                .into_widget(),
            GridCell::new(cell("RowSpan 2", fill))
                .row(1)
                .row_span(2)
                .into_widget(),
            GridCell::new(
                Align::new()
                    .alignment(Alignment::CENTER_RIGHT.into())
                    .child(cell("Aligned right", accent)),
            )
            .row(2)
            .column(1)
            .into_widget(),
            GridCell::new(cell("Bottom", fill))
                .row(2)
                .column(2)
                .into_widget(),
        ]);
    section(
        "Grid",
        resources,
        example(
            "Sizing, spans and alignment",
            "Auto, star and pixel tracks share a grid with a column span, a row span and a right-aligned child.",
            resources,
            SizedBox::from_size(Some(Size::new(420.0, 140.0)))
                .child(grid)
                .into_widget(),
        ),
    )
}

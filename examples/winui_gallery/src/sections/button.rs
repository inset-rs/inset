//! `Button` styles, icon content, custom content and disabled states.

use crate::{GalleryState, column, example, example_row, row, section};
use inset_foundation::{Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the button style and content examples.
pub fn build(state: Handle<GalleryState>, resources: &ThemeResources) -> Vec<WidgetRef> {
    let click = Listener::new(move |app| state.set_state(app, |s| s.clicks += 1));
    let styles = example(
        "Styles",
        "Compare the standard, accent and subtle templates with disabled states.",
        resources,
        example_row(
            vec![
                Button::text("Standard", click.clone()).into_widget(),
                Button::text("Accent", click.clone())
                    .style(ButtonStyle::Accent)
                    .into_widget(),
                Button::text("Subtle", click.clone())
                    .style(ButtonStyle::Subtle)
                    .into_widget(),
                Button::text("Disabled", click.clone())
                    .is_enabled(false)
                    .into_widget(),
                Button::text("Disabled accent", click.clone())
                    .style(ButtonStyle::Accent)
                    .is_enabled(false)
                    .into_widget(),
            ],
            8.0,
        ),
    );
    let content = example(
        "Content",
        "Buttons accept any widget as content, including an icon or a composed row.",
        resources,
        example_row(
            vec![
                Button::new(FontIcon::symbol(FluentSymbol::Add), click.clone())
                    .style(ButtonStyle::Accent)
                    .into_widget(),
                Button::new(
                    row(
                        vec![
                            FontIcon::symbol(FluentSymbol::Settings).into_widget(),
                            Text::new("Custom content").into_widget(),
                        ],
                        8.0,
                    ),
                    click,
                )
                .into_widget(),
            ],
            8.0,
        ),
    );
    section("Button", resources, column(vec![styles, content], 16.0))
}

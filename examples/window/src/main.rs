//! Opens the implicit window and runs a small widget tree in it through `run_app`: a padded,
//! centered, decorated card holding a line of text. The card is a mouse region that shows
//! the pointing-hand cursor while hovered.
//!
//! ```text
//! cargo run -p window
//! ```

use reveal_embedder::{Color, TextDirection};
use reveal_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use reveal_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use reveal_rendering::DecorationPosition;
use reveal_services::SystemMouseCursors;
use reveal_shell::Shell;
use reveal_widgets::{
    Center, DecoratedBox, Directionality, IntoWidget, MouseRegion, Padding, SizedBox, Text,
    WidgetRef, run_app,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const CARD: Color = Color::from_argb(255, 51, 51, 64);
const LABEL: Color = Color::from_argb(255, 245, 245, 240);

fn main() {
    WinitEmbedder {
        implicit_view: Some(ImplicitViewConfig {
            title: "reveal — widgets".to_owned(),
            logical_size: [600.0, 400.0],
        }),
    }
    .run(|platform| Shell::new(platform, |app| run_app(app, home())));
}

/// What `WidgetsApp` would supply: the ambient reading direction the text needs.
fn home() -> WidgetRef {
    Directionality {
        key: None,
        text_direction: TextDirection::Ltr,
        child: background(),
    }
    .into_widget()
}

fn background() -> WidgetRef {
    DecoratedBox {
        key: None,
        decoration: Box::new(BoxDecoration::new().color(BACKGROUND)),
        position: DecorationPosition::Background,
        child: Some(
            Padding {
                key: None,
                padding: EdgeInsetsGeometry::all(24.0),
                child: Some(
                    Center {
                        key: None,
                        width_factor: None,
                        height_factor: None,
                        child: Some(card()),
                    }
                    .into_widget(),
                ),
            }
            .into_widget(),
        ),
    }
    .into_widget()
}

fn card() -> WidgetRef {
    MouseRegion {
        cursor: SystemMouseCursors::CLICK.into(),
        child: Some(
            SizedBox {
                key: None,
                width: Some(200.0),
                height: Some(120.0),
                child: Some(
                    DecoratedBox {
                        key: None,
                        decoration: Box::new(BoxDecoration::new().color(CARD)),
                        position: DecorationPosition::Background,
                        child: Some(
                            Center {
                                key: None,
                                width_factor: None,
                                height_factor: None,
                                child: Some(label()),
                            }
                            .into_widget(),
                        ),
                    }
                    .into_widget(),
                ),
            }
            .into_widget(),
        ),
        ..MouseRegion::default()
    }
    .into_widget()
}

fn label() -> WidgetRef {
    Text::new("Hello, reveal")
        .style(TextStyle::new().font_size(24.0).color(LABEL))
        .into_widget()
}

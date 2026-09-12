//! Opens the implicit window and runs a small widget tree in it through `run_app`: a padded,
//! centered, decorated card holding a `CupertinoButton` that fades while pressed and counts
//! its presses on stdout. The card is a mouse region that shows the pointing-hand cursor
//! while hovered.
//!
//! ```text
//! cargo run -p window
//! ```

use std::cell::Cell;
use std::rc::Rc;

use inset_cupertino::{CupertinoButton, CupertinoTheme, CupertinoThemeData};
use inset_embedder::{Brightness, Color, TextDirection};
use inset_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use inset_foundation::Listener;
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_services::SystemMouseCursors;
use inset_shell::Shell;
use inset_widgets::{
    Center, DecoratedBox, Directionality, IntoWidget, MouseRegion, Padding, SizedBox, Text,
    WidgetRef, run_app,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const CARD: Color = Color::from_argb(255, 51, 51, 64);
const LABEL: Color = Color::from_argb(255, 245, 245, 240);

fn main() {
    WinitEmbedder {
        implicit_view: Some(ImplicitViewConfig {
            title: "Inset — widgets".to_owned(),
            logical_size: [600.0, 400.0],
        }),
    }
    .run(|platform| Shell::new(platform, |app| run_app(app, home())));
}

/// What `WidgetsApp` would supply: the ambient reading direction the text needs.
fn home() -> WidgetRef {
    Directionality::new(TextDirection::Ltr, background()).into_widget()
}

fn background() -> WidgetRef {
    DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
        .child(Padding::new(EdgeInsetsGeometry::all(24.0)).child(Center::new().child(card())))
        .into_widget()
}

fn card() -> WidgetRef {
    MouseRegion::new()
        .cursor(SystemMouseCursors::CLICK.into())
        .child(
            SizedBox::new().width(240.0).height(140.0).child(
                DecoratedBox::new(BoxDecoration::new().color(CARD))
                    .child(Center::new().child(button())),
            ),
        )
        .into_widget()
}

/// A filled iOS button under a dark Cupertino theme, counting its presses.
fn button() -> WidgetRef {
    let presses = Rc::new(Cell::new(0));
    let on_pressed = Listener::new(move |_app| {
        presses.set(presses.get() + 1);
        println!("pressed {} time(s)", presses.get());
    });
    CupertinoTheme::new(
        CupertinoThemeData::new().with_brightness(Brightness::Dark),
        CupertinoButton::filled(label(), Some(on_pressed)),
    )
    .into_widget()
}

fn label() -> WidgetRef {
    Text::new("Press me")
        .style(TextStyle::new().font_size(20.0).color(LABEL))
        .into_widget()
}

//! A Cupertino widget gallery.
//!
//! The index is a list of entries; each opens a screen that demonstrates one feature. See
//! `lib.rs` for the layout, and `catalog.rs` for the entries.
//!
//! This file is the embedder setup and nothing else.
//!
//! ```text
//! cargo run -p cupertino_gallery
//! ```

use cupertino_gallery::gallery;
use reveal_cupertino::install_cupertino_icon_font;
use reveal_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use reveal_shell::Shell;
use reveal_widgets::{IntoWidget, run_app};

fn main() {
    WinitEmbedder {
        implicit_view: Some(ImplicitViewConfig {
            title: "reveal — cupertino gallery".to_owned(),
            logical_size: [420.0, 720.0],
        }),
    }
    .run(|platform| {
        Shell::new(platform, |app| {
            run_app(app, gallery().into_widget());
            // AFTER `run_app`, which is what installs the default font collection this
            // registers into. Without the icon face every glyph in the gallery — the back
            // chevron, the row badges, the whole Icons entry — is a missing glyph and draws
            // nothing at all, silently. There is no asset manifest here, so registration is
            // explicit.
            install_cupertino_icon_font(app);
        })
    });
}

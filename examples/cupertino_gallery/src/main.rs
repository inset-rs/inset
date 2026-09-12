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

use cupertino_gallery::{Entry, run_gallery};
use inset_embedder_default::{DefaultEmbedder, ImplicitViewConfig};
use inset_shell::Shell;

fn main() {
    // `GALLERY_ENTRY=indicators` opens that screen over the index without a tap.
    let opening = std::env::var("GALLERY_ENTRY")
        .ok()
        .and_then(|name| Entry::from_name(&name));
    DefaultEmbedder::default()
        .implicit_view(Some(ImplicitViewConfig {
            title: "Inset — cupertino gallery".to_owned(),
            logical_size: [420.0, 720.0],
        }))
        .run(move |platform| Shell::new(platform, move |app| run_gallery(app, opening)));
}

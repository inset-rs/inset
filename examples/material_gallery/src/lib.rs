//! A Material widget gallery: an index of entries, each opening one page that demonstrates
//! one family of Material widgets.
//!
//! The index is itself a `Scaffold` with an `AppBar`, a `Drawer` and a `ListView` of
//! `ListTile`s, so the gallery's own structure is built out of the widgets it shows.
//!
//! The crate is a library as well as a binary so `tests/` can push the real gallery's pages
//! instead of rebuilding a look-alike tree of its own.
#![feature(arbitrary_self_types)]

pub mod app;
pub mod catalog;
pub mod pages;
pub mod support;

pub use app::{Gallery, entry_route, open_entry, open_sub_page, run_gallery, sub_route};
pub use catalog::Entry;

/// Every host's entry point; `src/main.rs` calls the generated `main`.
#[waft::main]
fn main(app: &mut waft::App) {
    // `GALLERY_ENTRY=buttons` opens that page over the index without a tap.
    let opening = std::env::var("GALLERY_ENTRY")
        .ok()
        .and_then(|name| Entry::from_name(&name));
    run_gallery(app, opening);
}

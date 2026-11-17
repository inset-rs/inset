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
    // `GALLERY_WINDOW=1300x850` opens the window at that logical size.
    let size = std::env::var("GALLERY_WINDOW")
        .ok()
        .and_then(|size| window_size_from(&size));
    run_gallery(app, opening, size);
}

/// `"1300x850"` as a logical window size.
fn window_size_from(text: &str) -> Option<[f64; 2]> {
    let (width, height) = text.split_once('x')?;
    Some([width.parse().ok()?, height.parse().ok()?])
}

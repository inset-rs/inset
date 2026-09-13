//! A Cupertino widget gallery: an index of entries, each opening one screen that demonstrates
//! one feature.
//!
//! The index is itself a [`CupertinoListSection`](inset_cupertino::CupertinoListSection) of
//! [`CupertinoListTile`](inset_cupertino::CupertinoListTile)s, so the gallery's own structure
//! is built out of the widgets it shows.
//!
//! The crate is a library as well as a binary so `tests/` can push the real gallery's pages
//! instead of rebuilding a look-alike tree of its own.
#![feature(arbitrary_self_types)]

pub mod app;
pub mod catalog;
pub mod pages;
pub mod support;

pub use app::{entry_route, gallery, open_entry, open_sub_page, run_gallery, sub_route};
pub use catalog::Entry;

/// Every host's entry point; `src/main.rs` calls the generated `main`.
#[inset::main(title = "Inset — cupertino gallery", size = [420.0, 720.0])]
fn main(app: &mut inset::App) {
    // `GALLERY_ENTRY=indicators` opens that screen over the index without a tap.
    let opening = std::env::var("GALLERY_ENTRY")
        .ok()
        .and_then(|name| Entry::from_name(&name));
    run_gallery(app, opening);
}

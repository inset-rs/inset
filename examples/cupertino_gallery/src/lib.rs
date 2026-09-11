//! A Cupertino widget gallery: an index of entries, each opening one screen that demonstrates
//! one feature.
//!
//! The index is itself a [`CupertinoListSection`](reveal_cupertino::CupertinoListSection) of
//! [`CupertinoListTile`](reveal_cupertino::CupertinoListTile)s, so the gallery's own structure
//! is built out of the widgets it shows.
//!
//! The crate is a library as well as a binary so `tests/` can push the real gallery's pages
//! instead of rebuilding a look-alike tree of its own.
#![feature(arbitrary_self_types)]

pub mod app;
pub mod catalog;
pub mod pages;
pub mod support;

#[cfg(target_arch = "wasm32")]
mod web;

pub use app::{entry_route, gallery, open_entry, open_sub_page, run_gallery, sub_route};
pub use catalog::Entry;

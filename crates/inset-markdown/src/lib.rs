//! Markdown widgets for Inset, with flutter_markdown's API ideas and look.
//!
//! [`parse`] turns Markdown into a typed document of [`Block`]s and [`Inline`]s with
//! pulldown-cmark; [`MarkdownBody`] and [`Markdown`] keep that document and render it, in the
//! look of the inherited [`DefaultMarkdownStyle`] or their own [`MarkdownStyleSheet`]. Each
//! element renders as its function in [`render`] does unless the widget has a hook for it.
#![feature(arbitrary_self_types)]

mod document;
pub mod render;
mod style_sheet;
#[cfg(test)]
mod test_support;
mod widget;

pub use document::*;
pub use pulldown_cmark::Options;
pub use render::Bullet;
pub use style_sheet::*;
pub use widget::*;

//! The font collection a paragraph shapes against. Flutter's engine owns one per process and
//! asks the OS for faces; here the collection is the host's [`FontCollection`], filled from
//! [`Platform::font_source`](crate::Platform::font_source) and whatever the application
//! registers, and handed to [`ParagraphBuilder::build`](crate::ParagraphBuilder::build).

pub use valo::{FontCollection, FontDemand, FontId, FontSource};

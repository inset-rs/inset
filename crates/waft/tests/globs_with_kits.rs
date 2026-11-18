//! `use waft::*` beside a kit's `*` names every item once.
//!
//! An app writes both globs: the facade for the framework, a kit for its controls. A name both
//! export would be ambiguous at every use, so each pair is re-exported here with
//! `ambiguous_glob_reexports` denied: a new clash in either crate fails this test's build, and
//! names the item.
#![deny(ambiguous_glob_reexports)]

/// The facade and the WinUI kit.
pub mod winui {
    pub use waft::*;
    pub use waft_winui::*;
}

/// The facade and the Material kit.
pub mod material {
    pub use waft::*;
    pub use waft_material::*;
}

/// The facade and the Cupertino kit.
pub mod cupertino {
    pub use waft::*;
    pub use waft_cupertino::*;
}

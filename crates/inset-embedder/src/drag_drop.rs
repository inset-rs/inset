//! Files dragged from outside the application over a view.
//!
//! No dart:ui counterpart: Flutter leaves external drops to plugins. The shape is the one
//! every desktop toolkit reports — Qt's drag events, wry's `DragDropEvent`, gpui's
//! `FileDropEvent` — put in `PointerData`'s terms: a view, a phase, a physical position, and
//! the paths as a batch.

use std::path::PathBuf;

use crate::{Offset, ViewId};

/// One report of an external drag over a view.
#[derive(Clone, Debug, PartialEq)]
pub struct DropData {
    /// The [`crate::View`] the drag is over.
    pub view_id: ViewId,
    /// What happened since the last report.
    pub change: DropChange,
    /// Where over the view, in physical pixels, when the host knows: a host may report no
    /// position at all, and none reports one on [`DropChange::Exited`].
    pub physical_position: Option<Offset>,
    /// The dragged files: always on [`DropChange::Dropped`], on [`DropChange::Entered`]
    /// where the system tells the host ahead of the drop, empty otherwise.
    pub paths: Vec<PathBuf>,
}

/// The four phases of an external drag, as every toolkit has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropChange {
    /// The drag came over the view.
    Entered,
    /// The drag moved over the view.
    Moved,
    /// The files were let go over the view.
    Dropped,
    /// The drag left the view without dropping.
    Exited,
}

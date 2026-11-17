//! How one pane is shown in the current layout.

use crate::compose::ThreePaneScaffoldRole;

/// How one pane is shown in the current layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneAdaptedValue {
    /// The pane takes its own partition.
    Expanded,
    /// The pane is hidden.
    Hidden,
    /// The pane is shown under `reflow_under`, at most half its height.
    Reflowed {
        /// The pane this one is shown under.
        reflow_under: ThreePaneScaffoldRole,
    },
}

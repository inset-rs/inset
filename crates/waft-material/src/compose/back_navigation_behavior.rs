//! How a back gesture moves through a pane scene's entries.

/// How a back gesture moves through a pane scene's entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BackNavigationBehavior {
    /// Remove exactly one entry.
    PopLatest,
    /// Remove entries until the shown panes change; the default.
    #[default]
    PopUntilScaffoldValueChange,
    /// Remove entries until the current destination's pane changes.
    PopUntilCurrentDestinationChange,
    /// Remove entries until the current destination's content or the shown panes change.
    PopUntilContentChange,
}

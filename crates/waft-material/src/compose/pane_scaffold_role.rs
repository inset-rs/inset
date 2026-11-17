//! The roles a pane scaffold's three panes can have.

/// The role of one pane in a three-pane scaffold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreePaneScaffoldRole {
    /// The most important pane, such as a list-detail view's detail.
    Primary,
    /// The pane that supplements the primary one, such as a list-detail view's list.
    Secondary,
    /// The pane for additional content, shown on a wide window.
    Tertiary,
}

/// [`ThreePaneScaffoldRole`]'s names in a list-detail scaffold.
pub struct ListDetailPaneScaffoldRole;

impl ListDetailPaneScaffoldRole {
    /// The list.
    pub const LIST: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Secondary;
    /// The selected item's detail.
    pub const DETAIL: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Primary;
    /// The third pane.
    pub const EXTRA: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Tertiary;
}

/// [`ThreePaneScaffoldRole`]'s names in a supporting-pane scaffold.
pub struct SupportingPaneScaffoldRole;

impl SupportingPaneScaffoldRole {
    /// The main content.
    pub const MAIN: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Primary;
    /// The supporting content, below the main content on a phone.
    pub const SUPPORTING: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Secondary;
    /// The third pane.
    pub const EXTRA: ThreePaneScaffoldRole = ThreePaneScaffoldRole::Tertiary;
}

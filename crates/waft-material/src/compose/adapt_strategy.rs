//! What a pane does when there is no partition left for it.

use crate::compose::ThreePaneScaffoldRole;

/// What a pane does when there is no partition left for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptStrategy {
    /// Hide the pane.
    Hide,
    /// Show the pane under another one, at most half its height.
    Reflow {
        /// The pane this one is shown under.
        reflow_under: ThreePaneScaffoldRole,
    },
}

/// One [`AdaptStrategy`] per pane role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreePaneScaffoldAdaptStrategies {
    /// The primary pane's strategy.
    pub primary: AdaptStrategy,
    /// The secondary pane's strategy.
    pub secondary: AdaptStrategy,
    /// The tertiary pane's strategy.
    pub tertiary: AdaptStrategy,
}

impl ThreePaneScaffoldAdaptStrategies {
    /// The strategy for `role`.
    pub fn get(&self, role: ThreePaneScaffoldRole) -> AdaptStrategy {
        match role {
            ThreePaneScaffoldRole::Primary => self.primary,
            ThreePaneScaffoldRole::Secondary => self.secondary,
            ThreePaneScaffoldRole::Tertiary => self.tertiary,
        }
    }

    /// All three hide, as Compose's `ListDetailPaneScaffoldDefaults` does.
    pub fn list_detail() -> ThreePaneScaffoldAdaptStrategies {
        ThreePaneScaffoldAdaptStrategies {
            primary: AdaptStrategy::Hide,
            secondary: AdaptStrategy::Hide,
            tertiary: AdaptStrategy::Hide,
        }
    }

    /// The supporting pane reflows under the main one, as Compose's
    /// `SupportingPaneScaffoldDefaults` does.
    pub fn supporting() -> ThreePaneScaffoldAdaptStrategies {
        ThreePaneScaffoldAdaptStrategies {
            primary: AdaptStrategy::Hide,
            secondary: AdaptStrategy::Reflow {
                reflow_under: ThreePaneScaffoldRole::Primary,
            },
            tertiary: AdaptStrategy::Hide,
        }
    }
}

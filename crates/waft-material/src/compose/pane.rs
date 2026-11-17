//! One pane's content and the size it asks for.

use waft_embedder::Size;
use waft_widgets::{IntoWidget, WidgetRef};

/// The size a pane asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanePreferredSize {
    /// A fixed size in logical pixels.
    Fixed(f64),
    /// A fraction of the scaffold, from 0 to 1.
    Proportion(f64),
}

impl PanePreferredSize {
    pub(crate) fn resolve(self, available: f64) -> f64 {
        match self {
            PanePreferredSize::Fixed(size) => size,
            PanePreferredSize::Proportion(proportion) => available * proportion,
        }
    }
}

/// One pane's content and the size it asks for.
#[derive(Clone, Debug)]
pub struct Pane {
    /// The pane's content.
    pub child: WidgetRef,
    /// The width the pane asks for; `None` uses the directive's default.
    pub preferred_width: Option<PanePreferredSize>,
    /// The height a reflowed pane asks for; `None` uses the directive's default.
    pub preferred_height: Option<PanePreferredSize>,
}

impl Pane {
    /// Creates a pane around `child`.
    pub fn new<K>(child: impl IntoWidget<K>) -> Pane {
        Pane {
            child: child.into_widget(),
            preferred_width: None,
            preferred_height: None,
        }
    }

    /// The width the pane asks for.
    pub fn preferred_width(mut self, preferred_width: PanePreferredSize) -> Pane {
        self.preferred_width = Some(preferred_width);
        self
    }

    /// The height a reflowed pane asks for.
    pub fn preferred_height(mut self, preferred_height: PanePreferredSize) -> Pane {
        self.preferred_height = Some(preferred_height);
        self
    }

    pub(crate) fn width_for(&self, scaffold: Size, default_width: f64) -> f64 {
        self.preferred_width
            .map(|size| size.resolve(scaffold.width()))
            .unwrap_or(default_width)
    }

    pub(crate) fn height_for(&self, scaffold: Size, default_height: f64) -> f64 {
        self.preferred_height
            .map(|size| size.resolve(scaffold.height()))
            .unwrap_or(default_height)
    }
}

impl From<WidgetRef> for Pane {
    fn from(child: WidgetRef) -> Pane {
        Pane::new(child)
    }
}

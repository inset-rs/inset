//! One pane's content, the size it asks for, its margins and its shape.

use waft_embedder::{Rect, Size, TextDirection};
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef};
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

/// One pane's content, the size it asks for, its margins and its shape.
#[derive(Clone, Debug)]
pub struct Pane {
    /// The pane's content.
    pub child: WidgetRef,
    /// The width the pane asks for; `None` uses the directive's default.
    pub preferred_width: Option<PanePreferredSize>,
    /// The height a reflowed pane asks for; `None` uses the directive's default.
    pub preferred_height: Option<PanePreferredSize>,
    /// Compose's `paneMargins(fixedMargins)`: how far the pane keeps from each edge of the
    /// scaffold. A margin only moves an edge the pane shares with the scaffold; the gap between
    /// two panes is the directive's spacer.
    pub margins: EdgeInsetsGeometry,
    /// Compose's `AnimatedPane(shape:)`: the shape the pane is clipped to; `None` is Compose's
    /// `RectangleShape`.
    pub shape: Option<ShapeBorderRef>,
}

impl Pane {
    /// Creates a pane around `child`.
    pub fn new<K>(child: impl IntoWidget<K>) -> Pane {
        Pane {
            child: child.into_widget(),
            preferred_width: None,
            preferred_height: None,
            margins: EdgeInsetsGeometry::ZERO,
            shape: None,
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

    /// How far the pane keeps from each edge of the scaffold.
    pub fn margins(mut self, margins: EdgeInsetsGeometry) -> Pane {
        self.margins = margins;
        self
    }

    /// The shape the pane is clipped to.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> Pane {
        self.shape = Some(shape.into());
        self
    }

    /// Compose's `PaneMarginsImpl`: `bounds` with each edge kept at least the margin from the
    /// same edge of a scaffold `scaffold` wide and high.
    pub(crate) fn within_margins(
        &self,
        bounds: Rect,
        scaffold: Size,
        text_direction: TextDirection,
    ) -> Rect {
        let margins = self.margins.resolve(Some(text_direction));
        Rect::from_ltrb(
            bounds.left.max(margins.left),
            bounds.top.max(margins.top),
            bounds.right.min(scaffold.width() - margins.right),
            bounds.bottom.min(scaffold.height() - margins.bottom),
        )
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

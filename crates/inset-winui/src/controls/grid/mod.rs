//! XAML `Grid` (`dxaml/xcp/core/core/elements/grid.cpp`): rows and columns of `Auto`, pixel and star lengths, spacing, spans, and the panel background and border.
//!
//! Children fill their cell, as XAML's default `Stretch` alignment does; a child a template aligns or sizes inside its cell is wrapped in `Align` or `SizedBox`.
//! An `Align` or `Center` wrapper takes `width_factor(1.0)` and `height_factor(1.0)`, so it measures at its child's size as an aligned XAML element does; without them it measures at all its cell offers, and a star row or column grows to the space the Grid is given.

mod definition;
mod layout;
mod render_grid;

pub use definition::*;
pub use layout::CellPlacement;
pub use render_grid::*;

use crate::{BackgroundSizing, Brush, ControlBorder};
use inset_foundation::App;
use inset_rendering::{AnyRenderObject, RenderBox, RenderHandle};
use inset_widgets::*;

/// XAML `Grid`.
#[derive(Debug, Default)]
pub struct Grid {
    /// Identifies this panel across parent rebuilds.
    pub key: Option<KeyRef>,
    /// The rows and their Auto, pixel or star lengths; an empty collection uses a single cell.
    pub row_definitions: Vec<RowDefinition>,
    /// The columns and their Auto, pixel or star lengths; an empty collection uses a single cell.
    pub column_definitions: Vec<ColumnDefinition>,
    /// The distance between adjacent rows.
    pub row_spacing: f64,
    /// The distance between adjacent columns.
    pub column_spacing: f64,
    /// The panel fill; `None` leaves it unpainted.
    pub background: Option<Brush>,
    /// The brush used to draw the panel border.
    pub border_brush: Option<Brush>,
    /// Left, top, right, bottom.
    pub border_thickness: [f64; 4],
    /// Top-left, top-right, bottom-right, bottom-left.
    pub corner_radius: [f64; 4],
    /// Whether the panel background extends beneath the border.
    pub background_sizing: BackgroundSizing,
    /// Left, top, right, bottom, inside the border.
    pub padding: [f64; 4],
    /// Children positioned with `GridCell` attached row, column and span values.
    pub children: Vec<WidgetRef>,
}

impl Grid {
    /// Creates an empty Grid with no row or column definitions.
    pub fn new() -> Grid {
        Grid::default()
    }

    /// Sets the panel identity.
    pub fn key(mut self, key: KeyRef) -> Grid {
        self.key = Some(key);
        self
    }

    /// XAML `Grid.RowDefinitions`.
    pub fn row_definitions(mut self, rows: impl IntoIterator<Item = RowDefinition>) -> Grid {
        self.row_definitions = rows.into_iter().collect();
        self
    }

    /// XAML `Grid.ColumnDefinitions`.
    pub fn column_definitions(
        mut self,
        columns: impl IntoIterator<Item = ColumnDefinition>,
    ) -> Grid {
        self.column_definitions = columns.into_iter().collect();
        self
    }

    /// XAML `RowSpacing`.
    pub fn row_spacing(mut self, spacing: f64) -> Grid {
        self.row_spacing = spacing;
        self
    }

    /// XAML `ColumnSpacing`.
    pub fn column_spacing(mut self, spacing: f64) -> Grid {
        self.column_spacing = spacing;
        self
    }

    /// XAML `Background`.
    pub fn background(mut self, background: Brush) -> Grid {
        self.background = Some(background);
        self
    }

    /// XAML `BorderBrush`.
    pub fn border_brush(mut self, border_brush: Brush) -> Grid {
        self.border_brush = Some(border_brush);
        self
    }

    /// XAML `BorderThickness`.
    pub fn border_thickness(mut self, thickness: f64) -> Grid {
        self.border_thickness = [thickness; 4];
        self
    }

    /// XAML `BorderThickness`, in left, top, right, bottom order.
    pub fn border_thickness_ltrb(mut self, thickness: [f64; 4]) -> Grid {
        self.border_thickness = thickness;
        self
    }

    /// XAML `CornerRadius`.
    pub fn corner_radius(mut self, radius: f64) -> Grid {
        self.corner_radius = [radius; 4];
        self
    }

    /// XAML `CornerRadius`, in top-left, top-right, bottom-right, bottom-left order.
    pub fn corner_radius_corners(mut self, radii: [f64; 4]) -> Grid {
        self.corner_radius = radii;
        self
    }

    /// XAML `BackgroundSizing`.
    pub fn background_sizing(mut self, sizing: BackgroundSizing) -> Grid {
        self.background_sizing = sizing;
        self
    }

    /// XAML `Padding`.
    pub fn padding(mut self, padding: [f64; 4]) -> Grid {
        self.padding = padding;
        self
    }

    /// Sets the panel children in paint order.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> Grid {
        self.children = children.into_iter().collect();
        self
    }
}

impl StatelessWidget for Grid {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let panel = GridPanel {
            key: self.key.clone(),
            row_definitions: self.row_definitions.clone(),
            column_definitions: self.column_definitions.clone(),
            row_spacing: self.row_spacing,
            column_spacing: self.column_spacing,
            children: self.children.clone(),
        };
        ControlBorder::new_optional(self.background, self.border_brush)
            .border_thickness_ltrb(self.border_thickness)
            .corner_radius_corners(self.corner_radius)
            .background_sizing(self.background_sizing)
            .padding(self.padding)
            .child(panel)
            .into_widget()
    }
}

/// The layout part of `Grid`: rows, columns, spacing and the children with their placements.
#[derive(Debug)]
struct GridPanel {
    key: Option<KeyRef>,
    row_definitions: Vec<RowDefinition>,
    column_definitions: Vec<ColumnDefinition>,
    row_spacing: f64,
    column_spacing: f64,
    children: Vec<WidgetRef>,
}

impl RenderObjectWidget for GridPanel {
    type RenderObject = RenderGrid;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let render_object = RenderGrid::new(app);
        self.update(app, render_object);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderGrid>,
    ) {
        self.update(app, render_object);
    }
}

impl GridPanel {
    fn update(&self, app: &mut App, render_object: RenderHandle<RenderGrid>) {
        render_object.set_definitions(
            app,
            self.row_definitions.clone(),
            self.column_definitions.clone(),
        );
        render_object.set_row_spacing(app, self.row_spacing);
        render_object.set_column_spacing(app, self.column_spacing);
    }
}

impl MultiChildRenderObjectWidget for GridPanel {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// The attached properties `Grid.Row`, `Grid.Column`, `Grid.RowSpan` and `Grid.ColumnSpan` on a child of a `Grid`.
#[derive(Debug)]
pub struct GridCell {
    /// Identifies this child placement across parent rebuilds.
    pub key: Option<KeyRef>,
    /// Attached row, column and span values used by the parent Grid.
    pub placement: CellPlacement,
    /// The widget arranged within the selected cell or span.
    pub child: WidgetRef,
}

impl GridCell {
    /// Places a child in row zero, column zero, spanning one row and one column.
    pub fn new<K>(child: impl IntoWidget<K>) -> GridCell {
        GridCell {
            key: None,
            placement: CellPlacement::default(),
            child: child.into_widget(),
        }
    }

    /// Sets the identity of this child placement.
    pub fn key(mut self, key: KeyRef) -> GridCell {
        self.key = Some(key);
        self
    }

    /// XAML `Grid.Row`.
    pub fn row(mut self, row: usize) -> GridCell {
        self.placement.row = row;
        self
    }

    /// XAML `Grid.Column`.
    pub fn column(mut self, column: usize) -> GridCell {
        self.placement.column = column;
        self
    }

    /// XAML `Grid.RowSpan`.
    pub fn row_span(mut self, span: usize) -> GridCell {
        self.placement.row_span = span;
        self
    }

    /// XAML `Grid.ColumnSpan`.
    pub fn column_span(mut self, span: usize) -> GridCell {
        self.placement.column_span = span;
        self
    }
}

impl ParentDataWidget for GridCell {
    type ParentData = GridParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        let parent_data = render_object.parent_data_of_mut::<GridParentData>(app);
        if parent_data.placement == self.placement {
            return;
        }
        parent_data.placement = self.placement;
        if let Some(parent) = render_object.parent(app) {
            parent.mark_needs_layout(app);
        }
    }
}

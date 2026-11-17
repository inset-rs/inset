//! How the gallery's index and page sit side by side: Material 3's large-screen layout on top
//! of the pane scaffold.
//!
//! When the window has room for panes side by side, each pane is its own rounded surface over
//! the tinted window, kept the window margin from its edges and the directive's spacer from the
//! next pane. A single pane fills the window as a phone's page does.

use waft_embedder::Color;
use waft_foundation::App;
use waft_material::{ColorScheme, Pane, PaneScaffoldDirective, WindowSizeClass};
use waft_painting::{
    BorderRadius, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_widgets::{BuildContext, IntoWidget};

/// The radius of a pane's corners: the large corner of Material 3's shape scale
/// (m3.material.io/styles/shape/corner-radius-scale), which Compose's own list-detail sample
/// gives its extra pane (`AnimatedPane(shape = RoundedCornerShape(16.dp))`).
pub const CORNER_RADIUS: f64 = 16.0;

/// Whether `directive` puts panes side by side.
pub fn side_by_side(directive: &PaneScaffoldDirective) -> bool {
    directive.max_horizontal_partitions > 1
}

/// The window behind the panes and the navigation: the surface container, a tone off the panes'
/// surface, so each pane reads as a sheet laid on it.
pub fn window_color(color_scheme: &ColorScheme) -> Color {
    color_scheme.surface_container()
}

/// `child` as a pane: rounded and inside the window margin when panes sit side by side, the
/// whole window otherwise.
pub fn pane<K>(
    app: &mut App,
    context: BuildContext,
    directive: &PaneScaffoldDirective,
    child: impl IntoWidget<K>,
) -> Pane {
    let pane = Pane::new(child);
    if !side_by_side(directive) {
        return pane;
    }
    let margin = WindowSizeClass::of(app, context).width.margin();
    pane.margins(EdgeInsetsGeometry::all(margin))
        .shape(rounded_shape())
}

/// The shape of a pane.
fn rounded_shape() -> ShapeBorderRef {
    RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::circular(CORNER_RADIUS).into(),
    )
    .into()
}

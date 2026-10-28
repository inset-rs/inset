//! The surfaces a scene or a present placed last, in paint order: laying the next ones out
//! beneath a layer's other sublayers, and hiding the ones no longer placed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use inset_embedder::SurfacePlacement;
use objc2_quartz_core::CALayer;

use super::CoreAnimationSurface;

/// The surfaces placed last, each with its placement, bottom-most first.
pub(super) struct PlacedSurfaces {
    placed: Vec<(Arc<CoreAnimationSurface>, SurfacePlacement)>,
}

impl PlacedSurfaces {
    pub(super) fn new() -> PlacedSurfaces {
        PlacedSurfaces { placed: Vec::new() }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Makes `surfaces` the placed ones, lays them out in `parent` once the window's scale is
    /// known, and hides the ones no longer placed. Inside the caller's transaction.
    pub(super) fn replace(
        &mut self,
        surfaces: Vec<(Arc<CoreAnimationSurface>, SurfacePlacement)>,
        parent: &CALayer,
        window_scale: Option<f64>,
    ) {
        let previous = std::mem::replace(&mut self.placed, surfaces);
        if let Some(window_scale) = window_scale {
            self.lay_out(parent, window_scale);
        }
        for (gone, _) in previous.iter().filter(|(old, _)| !self.contains(old)) {
            gone.hide();
        }
    }

    /// Lays the placed surfaces out among `parent`'s first sublayers, bottom-most first, in a
    /// window `window_scale` pixels to the point. Inside the caller's transaction.
    pub(super) fn lay_out(&self, parent: &CALayer, window_scale: f64) {
        for (index, (surface, placement)) in (0u32..).zip(&self.placed) {
            surface.place(parent, index, placement, window_scale);
        }
    }

    fn contains(&self, surface: &Arc<CoreAnimationSurface>) -> bool {
        self.placed
            .iter()
            .any(|(placed, _)| Arc::ptr_eq(placed, surface))
    }
}

/// Reports, once, a placed surface this host did not open.
pub(super) fn report_foreign_surface() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "inset: a scene placed a surface this host's compositor did not open; it is not shown"
        );
    }
}

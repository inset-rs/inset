//! The composition layers a scene or a present placed last, in paint order: laying the next ones
//! out beneath a `CALayer`'s other sublayers, and letting go of the ones no longer placed, and of
//! every one when the placer goes.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use inset_embedder::Placement;
use objc2_quartz_core::CALayer;

use super::{CoreAnimationLayer, in_transaction};

/// The layers placed last, each with its placement, bottom-most first.
pub(super) struct PlacedLayers {
    placed: Vec<(Arc<CoreAnimationLayer>, Placement)>,
}

impl PlacedLayers {
    pub(super) fn new() -> PlacedLayers {
        PlacedLayers { placed: Vec::new() }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Makes `layers` the placed ones, lays them out in `parent` once the window's scale is
    /// known, and lets go of the ones no longer placed. Inside the caller's transaction.
    pub(super) fn replace(
        &mut self,
        layers: Vec<(Arc<CoreAnimationLayer>, Placement)>,
        parent: &CALayer,
        window_scale: Option<f64>,
    ) {
        let previous = std::mem::replace(&mut self.placed, layers);
        if let Some(window_scale) = window_scale {
            self.lay_out(parent, window_scale);
        }
        for (gone, _) in previous.iter().filter(|(old, _)| !self.contains(old)) {
            gone.let_go();
        }
    }

    /// Lays the placed layers out among `parent`'s first sublayers, bottom-most first, in a
    /// window `window_scale` pixels to the point. Inside the caller's transaction.
    pub(super) fn lay_out(&self, parent: &CALayer, window_scale: f64) {
        for (index, (layer, placement)) in (0u32..).zip(&self.placed) {
            layer.show_in(parent, index, placement, window_scale);
        }
    }

    fn contains(&self, layer: &Arc<CoreAnimationLayer>) -> bool {
        self.placed
            .iter()
            .any(|(placed, _)| Arc::ptr_eq(placed, layer))
    }
}

impl Drop for PlacedLayers {
    /// The placer goes: every layer it placed is let go, for another placer to adopt.
    fn drop(&mut self) {
        if self.placed.is_empty() {
            return;
        }
        in_transaction(|| {
            for (layer, _) in &self.placed {
                layer.let_go();
            }
        });
    }
}

/// Reports, once, a placed layer this host did not make.
pub(super) fn report_foreign_layer() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("inset: a composition layer this host did not make was placed; it is not shown");
    }
}

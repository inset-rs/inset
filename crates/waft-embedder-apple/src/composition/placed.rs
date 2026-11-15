//! The composition layers a scene or a present placed last, in paint order: laying the next ones
//! out beneath a `CALayer`'s other sublayers, and letting go of the ones no longer placed, and of
//! every one when the placer goes.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use waft_embedder::{CompositionLayer, Placement, ViewId};

use super::{CoreAnimationLayer, Placer, adopted_by, in_transaction, own_layers};

/// The composition layers one view's last scene placed in a `CALayer` of its host's, beneath what
/// the view draws there, as the main thread holds them.
pub struct SceneLayers(RefCell<PlacedLayers>);

impl SceneLayers {
    pub fn new() -> SceneLayers {
        SceneLayers(RefCell::new(PlacedLayers::new()))
    }

    /// Places the layers of `view`'s scene in the layer `parent` answers, bottom-most first, laid
    /// out `scale` pixels to the point once that is known, and lets go of the ones the scene no
    /// longer places. `parent` is asked only when there is something to place or let go, and
    /// nothing changes where it answers none. Main thread.
    pub fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
        parent: impl FnOnce() -> Option<Retained<CALayer>>,
        scale: Option<f64>,
    ) {
        let layers = adopted_by(own_layers(placements), Placer::View(view));
        if layers.is_empty() && self.0.borrow().is_empty() {
            return;
        }
        let Some(parent) = parent() else {
            return;
        };
        in_transaction(|| self.0.borrow_mut().replace(layers, &parent, scale));
    }
}

impl Default for SceneLayers {
    fn default() -> SceneLayers {
        SceneLayers::new()
    }
}

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
        eprintln!("waft: a composition layer this host did not make was placed; it is not shown");
    }
}

//! A root no view shows, on macOS: the surfaces opened under it are in a layer of its own, which
//! the app places itself, as an app does that shows its surfaces in another process's window.
//!
//! No scene places these surfaces, so each places itself at each of its presents: it fills the
//! root's layer as the layer is then, at the layer's contents scale, which the app sets with the
//! layer's size to the scale of the window it shows in. A surface the layer outgrows between two
//! presents keeps its last size until its next. A surface opened later lies above those opened
//! before. Otherwise a surface here is one as any other: its presents, its children and when its
//! buffers go back to its producer.

use std::sync::Arc;

use inset_embedder::{Surface, SurfaceParent};
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;

use super::{CoreAnimationSurface, CrossThreadLayer, Placer, in_transaction};

/// A root no view shows, and the layer its surfaces are in, for the app to place.
pub(crate) fn detached_root() -> (Arc<dyn SurfaceParent>, Retained<CALayer>) {
    let layer = CALayer::new();
    let root = DetachedRoot {
        layer: CrossThreadLayer(layer.clone()),
    };
    (Arc::new(root), layer)
}

struct DetachedRoot {
    layer: CrossThreadLayer,
}

impl SurfaceParent for DetachedRoot {
    /// A surface in the root's layer, above those opened before, shown from its first present.
    fn open_child(&self, opaque: bool) -> Arc<dyn Surface> {
        let surface = CoreAnimationSurface::new(Placer::Itself, opaque);
        in_transaction(|| self.layer.0.addSublayer(&surface.clip_layer.0));
        Arc::new(surface)
    }
}

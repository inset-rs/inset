//! A root no view shows, on macOS: the surfaces opened under it are in a layer of its own, which
//! the app places itself, as an app does that shows its surfaces in another process's window.
//!
//! No scene places these surfaces, so each places itself at each of its presents, by the placement
//! a view's scene gives a surface that fills the view: the root's layer as it is then, in its
//! points, under a root transform of the layer's contents scale, which the app sets with the
//! layer's size to the scale of the window it shows in. A surface the layer outgrows between two
//! presents keeps its last size until its next. A surface opened later lies above those opened
//! before. Otherwise a surface here is one as any other: its presents, its children and when its
//! buffers go back to its producer.

use std::sync::Arc;

use inset_embedder::{Matrix4, Mutator, Rect, Surface, SurfaceParent, SurfacePlacement};
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
        let surface = Arc::new_cyclic(|itself| {
            CoreAnimationSurface::new(Placer::Itself(itself.clone()), opaque)
        });
        in_transaction(|| self.layer.0.addSublayer(&surface.clip_layer.0));
        surface
    }
}

/// `surface` filling `layer`, as a view's scene places a surface that fills the view: the layer's
/// bounds in its points, under the root transform to the pixels of the window it shows in, whose
/// scale is the layer's contents scale.
pub(super) fn placement_filling(layer: &CALayer, surface: Arc<dyn Surface>) -> SurfacePlacement {
    let size = layer.bounds().size;
    let scale = layer.contentsScale() as f32;
    SurfacePlacement {
        surface,
        rect: Rect::from_ltwh(0.0, 0.0, size.width, size.height),
        mutators: vec![Mutator::Transform(Matrix4::scale(scale, scale))],
    }
}

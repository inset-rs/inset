//! One window's composition layers as the main thread holds them: placing a scene's layers in the
//! layer of the window's view, beneath its content, and letting go of the ones it no longer
//! places.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;

use inset_embedder::{CompositionLayer, Placement, ViewId};
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use winit::window::Window;

use super::placed::PlacedLayers;
use super::{Placer, adopted_by, in_transaction, own_layers};
use crate::os::WindowComposition;

/// Where a system's window keeps the layers of its content: the layer of winit's view, whose
/// sublayers wgpu's `CAMetalLayer` and the composition layers beneath it are.
pub(crate) trait WindowLayer {
    /// The layer of `window`'s view; `None` off the main thread, or for a view without one.
    fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>>;
}

/// One window's composition layers, as the main thread holds them, on the system `S`.
pub(crate) struct CoreAnimationWindowComposition<S> {
    window: Arc<dyn Window>,
    placed: RefCell<PlacedLayers>,
    system: PhantomData<S>,
}

impl<S: WindowLayer> WindowComposition for CoreAnimationWindowComposition<S> {
    fn new(window: Arc<dyn Window>) -> Self {
        CoreAnimationWindowComposition {
            window,
            placed: RefCell::new(PlacedLayers::new()),
            system: PhantomData,
        }
    }

    /// Places the layers of `view`'s scene beneath the window's content, in paint order, and
    /// lets go of the ones the scene no longer places.
    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        let layers = adopted_by(own_layers(placements), Placer::View(view));
        if layers.is_empty() && self.placed.borrow().is_empty() {
            return;
        }
        let Some(parent) = S::window_layer(&*self.window) else {
            eprintln!("inset: a window's view has no layer; its composition layers are not shown");
            return;
        };
        let window_scale = self.window.scale_factor();
        in_transaction(|| {
            self.placed
                .borrow_mut()
                .replace(layers, &parent, Some(window_scale));
        });
    }
}

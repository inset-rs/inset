//! One window's surfaces as the main thread holds them: placing a scene's surfaces in the layer
//! of the window's view, beneath its content, and hiding the ones it no longer places.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;

use inset_embedder::{SurfacePlacement, ViewId};
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use winit::window::Window;

use super::placed::PlacedSurfaces;
use super::{Placer, in_transaction, surfaces_placed_by};
use crate::os::WindowSurfaces;

/// Where a system's window keeps the layers of its content: the layer of winit's view, whose
/// sublayers wgpu's `CAMetalLayer` and the surfaces beneath it are.
pub(crate) trait WindowLayer {
    /// The layer of `window`'s view; `None` off the main thread, or for a view without one.
    fn window_layer(window: &Window) -> Option<Retained<CALayer>>;
}

/// One window's surfaces, as the main thread holds them, on the system `S`.
pub(crate) struct CoreAnimationWindowSurfaces<S> {
    window: Arc<Window>,
    placed: RefCell<PlacedSurfaces>,
    system: PhantomData<S>,
}

impl<S: WindowLayer> WindowSurfaces for CoreAnimationWindowSurfaces<S> {
    fn new(window: Arc<Window>) -> Self {
        CoreAnimationWindowSurfaces {
            window,
            placed: RefCell::new(PlacedSurfaces::new()),
            system: PhantomData,
        }
    }

    /// Places `view`'s surfaces in the scene beneath the window's content, in paint order, and
    /// hides the ones the scene no longer places.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId) {
        let surfaces = surfaces_placed_by(placements, Placer::View(view));
        if surfaces.is_empty() && self.placed.borrow().is_empty() {
            return;
        }
        let Some(parent) = S::window_layer(&self.window) else {
            eprintln!("inset: a window's view has no layer; its surfaces are not shown");
            return;
        };
        let window_scale = self.window.scale_factor();
        in_transaction(|| {
            self.placed
                .borrow_mut()
                .replace(surfaces, &parent, Some(window_scale));
        });
    }
}

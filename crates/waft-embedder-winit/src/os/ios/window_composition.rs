//! One window's composition layers on iOS, where a view draws through its window's handle, as the
//! main thread holds them: placing a scene's layers in the layer of the window's view, beneath the
//! `CAMetalLayer` wgpu draws the view into, and letting go of the ones it no longer places.

use std::sync::Arc;

use waft_embedder::{CompositionLayer, Placement, ViewId};
use waft_embedder_apple::composition::SceneLayers;
use winit::window::Window;

use super::composition::window_layer;
use crate::os::window_handle_layer::WindowComposition;

/// One window's composition layers, as the main thread holds them.
pub(crate) struct CoreAnimationWindowComposition {
    window: Arc<dyn Window>,
    placed: SceneLayers,
}

impl WindowComposition for CoreAnimationWindowComposition {
    fn new(window: Arc<dyn Window>) -> Self {
        CoreAnimationWindowComposition {
            window,
            placed: SceneLayers::new(),
        }
    }

    /// Places the layers of `view`'s scene beneath the window's content, in paint order, and
    /// lets go of the ones the scene no longer places.
    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        let parent = || {
            let layer = window_layer(&*self.window);
            if layer.is_none() {
                eprintln!(
                    "waft: a window's view has no layer; its composition layers are not shown"
                );
            }
            layer
        };
        self.placed
            .place(placements, view, parent, Some(self.window.scale_factor()));
    }
}

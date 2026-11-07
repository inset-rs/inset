//! What a view shows into on every system but macOS: its window. wgpu makes the view's surface from
//! the window's handle, as from any raw window handle, and the system sizes it with the window; the
//! composition layers the view's scenes place go where the system's share of its compositor for
//! that window puts them ([`WindowComposition`]): beneath the window's content on iOS, nowhere on a
//! system without a compositor (`absent`).

use std::sync::Arc;

use waft_embedder::{CompositionLayer, Placement, ViewId};
use winit::window::Window;

use super::capabilities::ViewLayer;
use crate::gpu::Gpu;

/// One window's share of the compositor, where a view draws through its window's handle: it
/// places a scene's composition layers beneath the window's content.
pub(crate) trait WindowComposition {
    fn new(window: Arc<dyn Window>) -> Self;

    /// Places a scene's layers beneath the window's content, bottom-most first, and lets go of
    /// the ones it no longer places. Main thread.
    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    );
}

/// A window's view's layer that is the window itself, drawn into through the window's handle, with
/// the system's share `C` of its compositor for that window.
pub(crate) struct WindowHandleLayer<C> {
    window: Arc<dyn Window>,
    composition: C,
}

/// A window's hold on its view's layer where that layer is the window itself: nothing, since the
/// system places the window's content and sizes it with the window.
pub(crate) struct NoWindowHold;

impl<C: WindowComposition> WindowHandleLayer<C> {
    /// `window`'s view's layer, the window itself, with the window's hold on it.
    pub(crate) fn in_window(window: &Arc<dyn Window>) -> (WindowHandleLayer<C>, NoWindowHold) {
        let layer = WindowHandleLayer {
            window: Arc::clone(window),
            composition: C::new(Arc::clone(window)),
        };
        (layer, NoWindowHold)
    }
}

impl<C: WindowComposition> ViewLayer for WindowHandleLayer<C> {
    fn make_surface(
        &self,
        gpu: &Gpu,
        size: [u32; 2],
        alpha: valo::SurfaceAlpha,
    ) -> Result<valo::Surface, String> {
        valo::Surface::new_with_options(
            &gpu.instance,
            &gpu.adapter,
            &gpu.device,
            Arc::clone(&self.window),
            size,
            valo::SurfaceOptions::default().with_alpha(alpha),
        )
        .map_err(|error| error.to_string())
    }

    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        self.composition.place(placements, view);
    }
}

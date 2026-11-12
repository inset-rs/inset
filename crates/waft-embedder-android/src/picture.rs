//! How the view's picture reaches the screen: from one source per device, as Waft's contract asks.
//! Where Waft's device imports hardware buffers, which is Vulkan's with the import's extensions,
//! the picture is a composition layer's content, drawn into buffers of the view's own and placed
//! in a surface control made from the window, with the scene's composition layers beneath it in
//! the same transaction (`composition::view_layer`). Elsewhere, on OpenGL ES, there is no import
//! into the device, so no buffer the compositor shows can be drawn: the picture goes through a
//! swapchain on the window (`surface.rs`), and the platform has no compositor, so no scene places
//! a layer.

use ndk::native_window::NativeWindow;
use waft_embedder::Scene;

use crate::composition::{SurfaceControlCompositor, SurfaceControlViewLayer};
use crate::gpu::Gpu;
use crate::surface::Surface;

pub(crate) enum ViewPicture {
    /// A composition layer of the view's own, shown with its layers beneath it.
    Composited(SurfaceControlViewLayer),
    /// A swapchain on the window.
    Swapchain(Surface),
}

impl ViewPicture {
    /// The picture on `compositor` where there is one, through a swapchain where there is not,
    /// drawn with `context` on `gpu`'s device.
    pub(crate) fn new(
        gpu: &Gpu,
        compositor: Option<&SurfaceControlCompositor>,
        context: valo::Context,
    ) -> ViewPicture {
        match compositor {
            Some(compositor) => {
                ViewPicture::Composited(SurfaceControlViewLayer::new(compositor, gpu, context))
            }
            None => ViewPicture::Swapchain(Surface::new(context)),
        }
    }

    /// The system gave the activity `window`, `size` pixels large. `Err` where the picture cannot
    /// reach it.
    pub(crate) fn window_arrived(
        &mut self,
        gpu: &Gpu,
        window: &NativeWindow,
        size: [u32; 2],
    ) -> Result<(), String> {
        match self {
            ViewPicture::Composited(layer) => layer.window_arrived(window, size),
            ViewPicture::Swapchain(surface) => surface.window_arrived(gpu, window, size),
        }
    }

    /// The system takes the window away.
    pub(crate) fn window_gone(&mut self) {
        match self {
            ViewPicture::Composited(layer) => layer.window_gone(),
            ViewPicture::Swapchain(surface) => surface.window_gone(),
        }
    }

    /// The window is `size` pixels large from now on.
    pub(crate) fn resized(&mut self, size: [u32; 2]) {
        match self {
            ViewPicture::Composited(layer) => layer.resized(size),
            ViewPicture::Swapchain(surface) => surface.resized(size),
        }
    }

    /// Draws `scene` over `clear`, its composition layers beneath it where they show, laid out at
    /// `pixel_ratio`; with no video here, no reader has a frame to show. `Err` is the picture
    /// refused.
    pub(crate) fn draw(
        &mut self,
        scene: &Scene,
        clear: valo::Color,
        pixel_ratio: f64,
    ) -> Result<(), valo::Refused> {
        let slices = waft_embedder::slices(scene, |_reader| None);
        match self {
            ViewPicture::Composited(layer) => layer.draw(&slices, clear, pixel_ratio),
            ViewPicture::Swapchain(surface) => surface.draw(&slices.picture_with_room(), clear),
        }
    }
}

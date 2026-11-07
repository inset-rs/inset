//! Headless rendering output shared by test drivers.

use std::{cell::RefCell, sync::Arc};
use waft_embedder::valo::{Color, Context};
use waft_embedder::{Scene, View, ViewConstraints, ViewHost, ViewId, ViewMetrics};
use waft_test::TestViewHost;

/// A headless view that retains the last rendered RGBA frame.
pub struct CaptureView {
    /// Physical output size in pixels.
    pub size: [u32; 2],
    /// Renderer used by `View::present`.
    pub renderer: RefCell<Context>,
    /// Last frame as tightly packed RGBA bytes.
    pub pixels: RefCell<Vec<u8>>,
    /// What the view asks of the system, recorded.
    pub host: TestViewHost,
}
impl View for CaptureView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }
    fn metrics(&self) -> ViewMetrics {
        ViewMetrics {
            physical_size: self.size.map(|v| v as f64),
            physical_constraints: ViewConstraints::tight(self.size[0] as f64, self.size[1] as f64),
            device_pixel_ratio: 1.0,
            ..Default::default()
        }
    }
    fn present(&self, scene: Arc<Scene>) {
        *self.pixels.borrow_mut() = crate::gpu::render_to_rgba(
            &mut self.renderer.borrow_mut(),
            waft_embedder::slices(&scene, |_| None).picture_with_room(),
            self.size,
            Some(Color::WHITE),
        );
    }
    fn host(&self) -> &dyn ViewHost {
        &self.host
    }
}

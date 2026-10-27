//! Headless rendering output shared by test drivers.

use inset_embedder::valo::{Color, Context};
use inset_embedder::{Scene, View, ViewConstraints, ViewId, ViewMetrics};
use std::{cell::RefCell, sync::Arc};

/// A headless view that retains the last rendered RGBA frame.
pub struct CaptureView {
    /// Physical output size in pixels.
    pub size: [u32; 2],
    /// Renderer used by `View::present`.
    pub renderer: RefCell<Context>,
    /// Last frame as tightly packed RGBA bytes.
    pub pixels: RefCell<Vec<u8>>,
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
        *self.pixels.borrow_mut() = self.renderer.borrow_mut().render_to_rgba(
            &scene.picture,
            self.size,
            Some(Color::WHITE),
        );
    }
}

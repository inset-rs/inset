//! Headless rendering output shared by test drivers.

use std::cell::{Cell, Ref, RefCell};
use std::sync::Arc;
use waft_embedder::valo::Color;
use waft_embedder::{Scene, View, ViewConstraints, ViewHost, ViewId, ViewMetrics};
use waft_test::TestViewHost;

use crate::gpu;

/// The last scene a headless view presented, rasterized only when a test reads its pixels.
///
/// A widget test pumps frames to settle state, not to look at each one; Flutter's test binding
/// likewise composites without rasterizing.
pub struct LastFrame {
    scene: RefCell<Option<Arc<Scene>>>,
    pixels: RefCell<Vec<u8>>,
}

impl LastFrame {
    /// A frame with nothing presented yet.
    pub fn new() -> Self {
        Self {
            scene: RefCell::new(None),
            pixels: RefCell::new(Vec::new()),
        }
    }

    /// Keeps `scene` as the frame a pixel read draws, replacing the one not drawn yet.
    pub fn present(&self, scene: Arc<Scene>) {
        *self.scene.borrow_mut() = Some(scene);
    }

    /// The last presented scene drawn into `size` pixels over white, packed RGBA8, drawn on the
    /// first read after a present and kept for later reads.
    pub fn pixels(&self, size: [u32; 2]) -> Ref<'_, Vec<u8>> {
        if let Some(scene) = self.scene.borrow_mut().take() {
            let picture = waft_embedder::slices(&scene, |_| None).picture_with_room();
            let rendered = gpu::with_context(|context| {
                gpu::render_to_rgba(context, picture, size, Some(Color::WHITE))
            })
            .expect("GPU required");
            *self.pixels.borrow_mut() = rendered;
        }
        self.pixels.borrow()
    }
}

impl Default for LastFrame {
    fn default() -> Self {
        Self::new()
    }
}

/// A headless view that retains the last frame it presented.
pub struct CaptureView {
    /// Physical output size in pixels, in a cell because a window resize changes it.
    size: Cell<[u32; 2]>,
    /// The last frame presented, drawn when a test reads its pixels.
    frame: LastFrame,
    /// What the view asks of the system, recorded.
    pub host: TestViewHost,
}

impl CaptureView {
    /// A view of `size` pixels with no frame yet.
    pub fn new(size: [u32; 2]) -> Self {
        Self {
            size: Cell::new(size),
            frame: LastFrame::new(),
            host: TestViewHost::default(),
        }
    }

    /// The physical output size in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size.get()
    }

    /// Resizes the view, as the host does when the window is resized.
    pub fn set_size(&self, size: [u32; 2]) {
        self.size.set(size);
    }

    /// The last presented frame as RGBA rows, drawn on the first read.
    pub fn pixels(&self) -> Ref<'_, Vec<u8>> {
        self.frame.pixels(self.size())
    }
}

impl View for CaptureView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }
    fn metrics(&self) -> ViewMetrics {
        let size = self.size();
        ViewMetrics {
            physical_size: size.map(|v| v as f64),
            physical_constraints: ViewConstraints::tight(size[0] as f64, size[1] as f64),
            device_pixel_ratio: 1.0,
            ..Default::default()
        }
    }
    fn present(&self, scene: Arc<Scene>) {
        self.frame.present(scene);
    }
    fn host(&self) -> &dyn ViewHost {
        &self.host
    }
}

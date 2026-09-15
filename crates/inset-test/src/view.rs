//! A view that draws nowhere.

use std::cell::Cell;
use std::sync::Arc;

use inset_embedder::{Picture, View, ViewConstraints, ViewId, ViewMetrics};

/// A view of a fixed physical size and pixel ratio that counts the pictures presented to it
/// and shows none. `flutter_test`'s `TestFlutterView` in its two settable dimensions.
pub struct TestView {
    id: ViewId,
    size: Cell<[f64; 2]>,
    device_pixel_ratio: f64,
    presented: Cell<u32>,
}

impl TestView {
    /// View `ViewId(0)` at pixel ratio 1, `width` by `height` physical pixels.
    pub fn new(width: f64, height: f64) -> TestView {
        TestView::with_pixel_ratio(width, height, 1.0)
    }

    /// View `ViewId(0)` at `device_pixel_ratio`, `width` by `height` physical pixels.
    pub fn with_pixel_ratio(width: f64, height: f64, device_pixel_ratio: f64) -> TestView {
        TestView {
            id: ViewId(0),
            size: Cell::new([width, height]),
            device_pixel_ratio,
            presented: Cell::new(0),
        }
    }

    /// The same view under another id, for a platform with several.
    pub fn with_id(mut self, id: ViewId) -> TestView {
        self.id = id;
        self
    }

    /// Changes the physical size; the platform's client hears of it as it would from a host.
    pub fn resize(&self, width: f64, height: f64) {
        self.size.set([width, height]);
    }

    /// How many pictures have been presented.
    pub fn presented(&self) -> u32 {
        self.presented.get()
    }
}

impl View for TestView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        let [width, height] = self.size.get();
        ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio: self.device_pixel_ratio,
            ..ViewMetrics::default()
        }
    }

    fn present(&self, _picture: Arc<Picture>) {
        self.presented.set(self.presented.get() + 1);
    }
}

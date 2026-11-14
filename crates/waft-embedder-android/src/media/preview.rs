//! A camera's preview on a composition layer: each picture of the camera's stream presented to the
//! layer as a picture the system draws onto it (`composition`'s `present_system_frame`), in the
//! camera's own format, which the compositor shows with no copy, turned upright and fitted whole in
//! the layer's box, the picture held until the compositor is done with it. What the layer shows is
//! read from the camera's newest picture, drawn in RGBA (`camera_frames`).
//!
//! The invariant: a picture's slot in its stream is held from its present until its release, and
//! a preview that goes leaves the layer showing nothing of the camera.

use std::sync::{Arc, Weak};

use super::camera_images::CameraImage;
use crate::buffer_transform::BufferTransform;
use crate::composition::{SurfaceControlLayer, SystemFrame, SystemPicture};
use crate::hardware_buffers::AndroidHardwareBuffer;

/// A camera's preview shown on one layer.
pub(super) struct CameraPreview {
    layer: Arc<SurfaceControlLayer>,
    /// What the layer reads it shows from, which it holds weakly.
    picture: Arc<dyn SystemPicture>,
}

impl CameraPreview {
    pub(super) fn new(
        layer: Arc<SurfaceControlLayer>,
        picture: Arc<dyn SystemPicture>,
    ) -> CameraPreview {
        CameraPreview { layer, picture }
    }

    /// Shows `image` on the layer, turned by `transform`, holding it until the compositor is done
    /// with it.
    pub(super) fn show(&self, image: &Arc<CameraImage>, transform: BufferTransform) {
        let Ok(buffer) = image.hardware_buffer() else {
            return;
        };
        let held = Arc::clone(image);
        self.layer.present_system_frame(
            SystemFrame {
                buffer: AndroidHardwareBuffer::new(buffer),
                transform,
            },
            self.picture_shown(),
            Box::new(move || drop(held)),
        );
    }

    fn picture_shown(&self) -> Weak<dyn SystemPicture> {
        Arc::downgrade(&self.picture)
    }
}

impl Drop for CameraPreview {
    /// The layer shows nothing of the camera any more.
    fn drop(&mut self) {
        self.layer.withdraw_system_picture(&self.picture_shown());
    }
}

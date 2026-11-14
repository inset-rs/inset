//! A video's frame on Android as a frame reader hands it out: an RGBA8 `AHardwareBuffer` the host
//! drew a camera's picture into (`frame_conversion`), in the camera's own pixels, with the turn and
//! the mirror it shows at, whose memory is the frame's own, freed once
//! nothing holds it, which the app's GPU samples with no copy through a texture imported over it
//! (`gpu_interop`) and the compositor shows, and whose pixels are copied into the app's memory
//! while it is locked for the CPU (`hardware_buffers`).

use std::sync::Arc;

use waft_embedder::image_codecs::{ImageFuture, PixelBuffer};
use waft_embedder::{HardwareBuffer, VideoFrame};

use crate::hardware_buffers::AndroidHardwareBuffer;

/// One picture of a camera on Android: an RGBA8 `AHardwareBuffer`, in the camera's own pixels,
/// with how it shows: turned, then mirrored. Platform code reaches it from the [`VideoFrame`] a
/// reader answers by downcasting through `Any`:
///
/// ```
/// # #[cfg(target_os = "android")]
/// # fn platform_code(frame: std::sync::Arc<dyn waft_embedder::VideoFrame>) {
/// use waft_embedder_android::AndroidVideoFrame;
///
/// let frame: &dyn std::any::Any = &*frame;
/// if let Some(frame) = frame.downcast_ref::<AndroidVideoFrame>() {
///     let _raw = frame.buffer().buffer().as_ptr();
/// }
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AndroidVideoFrame {
    buffer: AndroidHardwareBuffer,
    /// The turn it is shown at, in degrees clockwise, before `flip`.
    rotation: u32,
    /// Whether it is shown mirrored, after `rotation`.
    flip: bool,
}

impl AndroidVideoFrame {
    /// The frame of `buffer`, an RGBA8 `AHardwareBuffer` its holder may sample, shown as it is:
    /// what platform code that holds the system's own buffer, as WaOS does for a program, hands
    /// the app back.
    pub fn new(buffer: AndroidHardwareBuffer) -> AndroidVideoFrame {
        AndroidVideoFrame {
            buffer,
            rotation: 0,
            flip: false,
        }
    }

    /// The frame shown turned `rotation` degrees clockwise, then mirrored where `flip` says, as
    /// a camera's frame is on a phone.
    pub fn shown_turned(self, rotation: u32, flip: bool) -> AndroidVideoFrame {
        AndroidVideoFrame {
            rotation: rotation % 360,
            flip,
            ..self
        }
    }

    /// The buffer the frame is. A reference on it holds the frame's memory, as the frame does.
    pub fn buffer(&self) -> &AndroidHardwareBuffer {
        &self.buffer
    }
}

impl VideoFrame for AndroidVideoFrame {
    fn width(&self) -> u32 {
        self.buffer.width()
    }

    fn height(&self) -> u32 {
        self.buffer.height()
    }

    /// Its rows as they lie, RGBA and opaque, copied while the buffer is locked for reading.
    fn read_pixels(&self) -> ImageFuture<PixelBuffer> {
        self.buffer.read_pixels()
    }

    fn hardware_buffer(&self) -> Arc<dyn HardwareBuffer> {
        Arc::new(self.buffer.clone())
    }

    fn rotation(&self) -> u32 {
        self.rotation
    }

    fn flip(&self) -> bool {
        self.flip
    }
}

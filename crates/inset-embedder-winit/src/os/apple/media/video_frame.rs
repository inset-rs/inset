//! A video's frame on macOS and iOS as a frame reader hands it out (`frames`): the `CVPixelBuffer`
//! AVFoundation handed out, backed by an `IOSurface`, in 32-bit BGRA, whose texture the app's GPU
//! samples with no copy (`frame_textures`).

use inset_embedder::VideoFrame;
use objc2_core_foundation::CFRetained;
use objc2_core_video::{CVPixelBuffer, CVPixelBufferGetHeight, CVPixelBufferGetWidth};

use super::frame_textures::texture_of;

/// One picture of a player or a camera on macOS and iOS: the `CVPixelBuffer` AVFoundation handed
/// out, backed by an `IOSurface`, in 32-bit BGRA. Platform code reaches it from the
/// [`VideoFrame`] a reader answers by downcasting through `Any`:
///
/// ```
/// # fn platform_code(frame: std::sync::Arc<dyn inset_embedder::VideoFrame>) {
/// use inset_embedder_winit::CoreVideoFrame;
///
/// let frame: &dyn std::any::Any = &*frame;
/// if let Some(frame) = frame.downcast_ref::<CoreVideoFrame>() {
///     let _buffer = frame.pixel_buffer();
/// }
/// # }
/// ```
pub struct CoreVideoFrame(CFRetained<CVPixelBuffer>);

impl CoreVideoFrame {
    /// The frame of `buffer`, as a reader's output handed it out.
    pub(super) fn new(buffer: CFRetained<CVPixelBuffer>) -> CoreVideoFrame {
        CoreVideoFrame(buffer)
    }

    /// The pixel buffer AVFoundation handed out. Retained, it holds the frame's slot in its pool,
    /// as the frame does.
    pub fn pixel_buffer(&self) -> &CVPixelBuffer {
        &self.0
    }
}

impl VideoFrame for CoreVideoFrame {
    fn width(&self) -> u32 {
        CVPixelBufferGetWidth(&self.0) as u32
    }

    fn height(&self) -> u32 {
        CVPixelBufferGetHeight(&self.0) as u32
    }

    /// A Metal texture on the frame's `IOSurface`, through CoreVideo's texture cache.
    ///
    /// # Panics
    ///
    /// On a device that is not Metal's, and where CoreVideo makes no texture of the frame, as
    /// wgpu's default error handler panics on a texture wgpu cannot make.
    fn texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        texture_of(&self.0, device)
    }
}

// SAFETY: a pixel buffer is made to be shared between threads: its retain count is atomic, and
// nothing here writes its pixels. The binding cannot say so.
unsafe impl Send for CoreVideoFrame {}
unsafe impl Sync for CoreVideoFrame {}

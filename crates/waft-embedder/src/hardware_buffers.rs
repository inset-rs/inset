//! GPU memory the system's compositor shows, as a host hands it to the app: on Apple platforms an
//! `IOSurface`, Android's `HardwareBuffer`, a dmabuf behind Wayland's `wl_buffer`, a shared D3D
//! texture. Not a dart:ui type: Flutter's engine keeps its buffers to itself. They follow
//! `wasdk:platform/hardware-buffers` one to one.
//!
//! A hardware buffer is memory the app holds: one it allocates
//! ([`HardwareBufferAllocator::allocate`]), as `IOSurfaceCreate` and `AHardwareBuffer_allocate`
//! make one, and an image the system's hardware codecs decode
//! ([`HardwareImageCodecs`](crate::image_codecs::HardwareImageCodecs)), which no system pool waits
//! for. A video's frame is apart, lent with its slot in a decoder's pool
//! ([`VideoFrame`](crate::VideoFrame)), and its memory is one of these too, which keeps the frame
//! while it is held. The app draws into a buffer and samples it through a texture it imports on
//! its own device ([`Platform::import_texture`](crate::Platform::import_texture)), shows it on a
//! composition layer ([`CompositionLayer::present`](crate::CompositionLayer::present)), and has
//! the hardware codecs encode it; its pixels come back to the app's memory only through
//! [`HardwareBuffer::read_pixels`]. A buffer knows only its own facts, its size and what it was
//! allocated for, and names no GPU API: the host, which knows both the buffer and the device,
//! imports it, as each system's GPU API imports such memory.
//!
//! Each host implements [`HardwareBuffer`] over its system's buffers. Platform code reaches the
//! system's own buffer by downcasting through `Any` to the type of the host that handed it out: on
//! Apple platforms `waft-embedder-winit`'s `SurfaceBuffer`, whose `surface` is the `IOSurface`.
//!
//! The invariant a host keeps: a texture imported of a buffer holds the buffer, and the frame it
//! is the memory of, until the GPU is done with it.

use std::any::Any;
use std::sync::Arc;

use crate::image_codecs::{ImageError, ImageFuture, PixelBuffer, PixelFormat};

/// What makes the app's hardware buffers: the role of Android's graphics allocator (gralloc's
/// `IAllocator`), Fuchsia's sysmem `Allocator` and a GBM device on Linux. `None` from
/// [`Platform::hardware_buffer_allocator`](crate::Platform::hardware_buffer_allocator) where the
/// system allocates none. It may be called from any thread, as a producer allocates a buffer where
/// it draws.
pub trait HardwareBufferAllocator: Send + Sync {
    /// A buffer of `width` by `height` pixels in `format`, for what `usage` names, its pixels as
    /// the system leaves new memory until the app draws them:
    /// [`ImageErrorKind::Unsupported`](crate::image_codecs::ImageErrorKind::Unsupported) for a
    /// format the system does not allocate a buffer in, as Apple platforms allocate only
    /// [`PixelFormat::Bgra8`] and Android's `AHardwareBuffer` only [`PixelFormat::Rgba8`];
    /// [`TooLarge`](crate::image_codecs::ImageErrorKind::TooLarge) for a side past the largest the
    /// system makes a texture of; [`OutOfMemory`](crate::image_codecs::ImageErrorKind::OutOfMemory)
    /// where the system has no memory left for it; and
    /// [`Other`](crate::image_codecs::ImageErrorKind::Other) for a side of no pixels.
    fn allocate(
        &self,
        width: u32,
        height: u32,
        format: PixelFormat,
        usage: HardwareBufferUsage,
    ) -> Result<Arc<dyn HardwareBuffer>, ImageError>;
}

/// What a buffer is allocated for, named after its users, as Android's `AHardwareBuffer` usage
/// is: so far what the GPU does with it, which a texture imported of it allows
/// ([`Platform::import_texture`](crate::Platform::import_texture)). The CPU, the system's
/// compositor and a video encoder join as they are needed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HardwareBufferUsage {
    /// The GPU samples it in its shaders: Android's `GPU_SAMPLED_IMAGE`.
    pub gpu_sampled: bool,
    /// The GPU draws into it, as a render pass's colour output, and copies into it: Android's
    /// `GPU_COLOR_OUTPUT`.
    pub gpu_color_output: bool,
}

impl HardwareBufferUsage {
    /// Sampled by the GPU only, as a decoded image's buffer and a video frame's are.
    pub const GPU_SAMPLED: HardwareBufferUsage = HardwareBufferUsage {
        gpu_sampled: true,
        gpu_color_output: false,
    };
}

/// One block of pixels in memory the GPU, the system's compositor and other processes all reach.
/// Every method may be called from any thread.
pub trait HardwareBuffer: Any + Send + Sync {
    /// The buffer's width, in its own pixels.
    fn width(&self) -> u32;

    /// The buffer's height, in its own pixels.
    fn height(&self) -> u32;

    /// The buffer's pixels copied into the app's memory, in the layout the buffer holds them: BGRA
    /// on Apple platforms. It reads what the GPU has finished writing, so an app that drew into
    /// the buffer waits for that work first, as it does before presenting it;
    /// [`ImageErrorKind::Unsupported`](crate::image_codecs::ImageErrorKind::Unsupported) for a
    /// buffer whose pixels are in no [`PixelFormat`].
    fn read_pixels(&self) -> ImageFuture<PixelBuffer>;
}

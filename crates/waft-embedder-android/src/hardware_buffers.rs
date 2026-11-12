//! Hardware buffers on Android: an `AHardwareBuffer` in RGBA8, the memory SurfaceFlinger shows as
//! a composition layer's content (`composition`), and Vulkan draws into and samples through a
//! texture imported over it (`gpu_interop`). A buffer the app allocates is made by
//! `AHardwareBuffer_allocate` with the usage the app names, and its pixels are read while it is
//! locked for the CPU. A buffer's usage is what its `AHardwareBuffer` says it was allocated for
//! (`AHardwareBuffer_describe`), whoever allocated it, mapped back to Waft's flags.
//!
//! The invariant: a buffer holds a reference on its `AHardwareBuffer` for as long as anything holds
//! the buffer, and `read_pixels` reads only a buffer whose usage has `cpu_read`: some allocators
//! lock any buffer and the emulator's refuses one without a CPU usage, so the usage, not the lock,
//! decides (`experiments/2026-11-11-android-surface-control`, question 7).

// Locking a buffer and reading its memory needs `unsafe`, and so does vouching for the buffer's
// reference across threads; this module writes it for them.
#![allow(unsafe_code)]

use std::sync::Arc;

use ndk::hardware_buffer::{
    HardwareBufferDesc, HardwareBufferRef, HardwareBufferUsage as AndroidUsage,
};
use ndk::hardware_buffer_format::HardwareBufferFormat;
use waft_embedder::image_codecs::{
    AlphaType, ImageError, ImageErrorKind, ImageFuture, PixelBuffer, PixelFormat, PixelLayout,
};
use waft_embedder::{HardwareBuffer, HardwareBufferAllocator, HardwareBufferUsage};

/// Bytes one pixel of RGBA8 occupies.
const BYTES_PER_PIXEL: u32 = 4;

/// A hardware buffer on Android: an `AHardwareBuffer` in RGBA8, with what it says of itself.
/// Platform code reaches the `AHardwareBuffer` from a [`HardwareBuffer`] by downcasting
/// through `Any`:
///
/// ```
/// # #[cfg(target_os = "android")]
/// # fn platform_code(buffer: std::sync::Arc<dyn waft_embedder::HardwareBuffer>) {
/// use waft_embedder_android::AndroidHardwareBuffer;
///
/// let buffer: &dyn std::any::Any = &*buffer;
/// if let Some(buffer) = buffer.downcast_ref::<AndroidHardwareBuffer>() {
///     let _raw = buffer.buffer().as_ptr();
/// }
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AndroidHardwareBuffer {
    buffer: HardwareBufferRef,
    usage: HardwareBufferUsage,
    width: u32,
    height: u32,
}

// SAFETY: an `AHardwareBuffer` is made to be shared between threads and processes: its reference
// count is atomic, and its memory is written only by the GPU work and the locks the app orders.
// `ndk` cannot say so of its pointer.
unsafe impl Send for AndroidHardwareBuffer {}
unsafe impl Sync for AndroidHardwareBuffer {}

impl AndroidHardwareBuffer {
    /// The buffer of `buffer`, an `AHardwareBuffer` in RGBA8, for what it was allocated for: what
    /// platform code that holds the system's own buffer, as WaOS does for a program, hands the
    /// app's compositor.
    pub fn new(buffer: HardwareBufferRef) -> AndroidHardwareBuffer {
        let description = buffer.describe();
        AndroidHardwareBuffer {
            buffer,
            usage: waft_usage(description.usage),
            width: description.width,
            height: description.height,
        }
    }

    /// The `AHardwareBuffer` the buffer is.
    pub fn buffer(&self) -> &HardwareBufferRef {
        &self.buffer
    }

    /// The pixels copied out while the buffer is locked for the CPU to read, rows as they lie.
    fn read_locked(&self) -> Result<PixelBuffer, ImageError> {
        if !self.usage.cpu_read {
            return Err(ImageError::new(
                ImageErrorKind::Unsupported,
                "a buffer whose usage lacks `cpu_read`, which Android's CPU may not read",
            ));
        }
        let description = self.buffer.describe();
        if description.format != HardwareBufferFormat::R8G8B8A8_UNORM {
            return Err(ImageError::new(
                ImageErrorKind::Unsupported,
                format!("a buffer in {:?}, not RGBA8", description.format),
            ));
        }
        let row_bytes = description.stride * BYTES_PER_PIXEL;
        let length = row_bytes as usize * description.height as usize;
        let address = self
            .buffer
            .lock(AndroidUsage::CPU_READ_RARELY, None, None)
            .map_err(|error| {
                ImageError::new(
                    ImageErrorKind::Other,
                    format!("the system did not lock the buffer's pixels: {error}"),
                )
            })?;
        // SAFETY: the lock maps the whole buffer for reading, `stride` pixels of 4 bytes a row,
        // `height` rows; the slice is copied before the unlock below.
        let rows = unsafe { std::slice::from_raw_parts(address.cast::<u8>(), length) }.to_vec();
        let _ = self.buffer.unlock();
        let layout = PixelLayout {
            size: [description.width, description.height],
            row_bytes,
            format: PixelFormat::Rgba8,
            alpha: AlphaType::Premultiplied,
        };
        PixelBuffer::new(layout, rows)
            .map_err(|error| ImageError::new(ImageErrorKind::Other, format!("{error:?}")))
    }
}

impl HardwareBuffer for AndroidHardwareBuffer {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    /// What the `AHardwareBuffer` was allocated for, as it describes itself.
    fn usage(&self) -> HardwareBufferUsage {
        self.usage
    }

    /// The buffer's rows as they lie, RGBA and premultiplied, copied while it is locked for
    /// reading: `Unsupported` for a buffer whose usage lacks `cpu_read`.
    fn read_pixels(&self) -> ImageFuture<PixelBuffer> {
        Box::pin(std::future::ready(self.read_locked()))
    }
}

/// The system's hardware buffer allocator on Android, gralloc behind `AHardwareBuffer_allocate`,
/// which allocates RGBA8 buffers no side of which is past the largest texture Waft's device makes.
pub(crate) struct GrallocAllocator {
    pub(crate) largest_side: u32,
}

impl HardwareBufferAllocator for GrallocAllocator {
    fn allocate(
        &self,
        width: u32,
        height: u32,
        format: PixelFormat,
        usage: HardwareBufferUsage,
    ) -> Result<Arc<dyn HardwareBuffer>, ImageError> {
        if let Some(refused) = refusal(width, height, format, self.largest_side) {
            return Err(refused);
        }
        let description = HardwareBufferDesc {
            width,
            height,
            layers: 1,
            format: HardwareBufferFormat::R8G8B8A8_UNORM,
            usage: android_usage(usage),
            stride: 0,
        };
        let buffer =
            ndk::hardware_buffer::HardwareBuffer::allocate(description).map_err(|error| {
                ImageError::new(
                    ImageErrorKind::OutOfMemory,
                    format!("the system allocated no buffer of that size: {error}"),
                )
            })?;
        Ok(Arc::new(AndroidHardwareBuffer::new(buffer)))
    }
}

/// Why a buffer of `width` by `height` in `format` is not allocated, sides past `largest_side`
/// refused; `None` for one that is.
fn refusal(width: u32, height: u32, format: PixelFormat, largest_side: u32) -> Option<ImageError> {
    if width == 0 || height == 0 {
        return Some(ImageError::new(
            ImageErrorKind::Other,
            format!("a buffer of {width} by {height} pixels, which holds none"),
        ));
    }
    if format != PixelFormat::Rgba8 {
        return Some(ImageError::new(
            ImageErrorKind::Unsupported,
            "Android allocates hardware buffers in RGBA",
        ));
    }
    if width.max(height) > largest_side {
        return Some(ImageError::new(
            ImageErrorKind::TooLarge,
            format!(
                "a side past {largest_side} pixels, the largest Waft's device makes a texture of"
            ),
        ));
    }
    None
}

/// Android's usage for a buffer the app allocates for `usage`.
pub(crate) fn android_usage(usage: HardwareBufferUsage) -> AndroidUsage {
    let flags = [
        (usage.gpu_sampled, AndroidUsage::GPU_SAMPLED_IMAGE),
        (usage.gpu_color_output, AndroidUsage::GPU_COLOR_OUTPUT),
        (usage.composer_overlay, AndroidUsage::COMPOSER_OVERLAY),
        (usage.cpu_read, AndroidUsage::CPU_READ_RARELY),
    ];
    flags
        .into_iter()
        .filter(|(asked, _)| *asked)
        .fold(AndroidUsage::empty(), |all, (_, flag)| all | flag)
}

/// Waft's flags for an `AHardwareBuffer` allocated for `usage`, each CPU read, rare or often, as
/// `cpu_read`; the uses Waft names no flag for are left out.
fn waft_usage(usage: AndroidUsage) -> HardwareBufferUsage {
    HardwareBufferUsage {
        gpu_sampled: usage.contains(AndroidUsage::GPU_SAMPLED_IMAGE),
        gpu_color_output: usage.contains(AndroidUsage::GPU_COLOR_OUTPUT),
        composer_overlay: usage.contains(AndroidUsage::COMPOSER_OVERLAY),
        cpu_read: usage.intersects(AndroidUsage::CPU_READ_RARELY | AndroidUsage::CPU_READ_OFTEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_usage_is_androids_own_flag() {
        let all = HardwareBufferUsage {
            gpu_sampled: true,
            gpu_color_output: true,
            composer_overlay: true,
            cpu_read: true,
        };
        assert_eq!(
            android_usage(all),
            AndroidUsage::GPU_SAMPLED_IMAGE
                | AndroidUsage::GPU_COLOR_OUTPUT
                | AndroidUsage::COMPOSER_OVERLAY
                | AndroidUsage::CPU_READ_RARELY
        );
        assert_eq!(
            android_usage(HardwareBufferUsage::default()),
            AndroidUsage::empty()
        );
    }

    #[test]
    fn androids_usage_maps_back_to_the_flags_it_was_allocated_for() {
        let all = HardwareBufferUsage::ALL;
        assert_eq!(waft_usage(android_usage(all)), all);
        assert_eq!(
            waft_usage(AndroidUsage::CPU_READ_OFTEN | AndroidUsage::CPU_WRITE_OFTEN),
            HardwareBufferUsage {
                cpu_read: true,
                ..HardwareBufferUsage::default()
            }
        );
        assert_eq!(
            waft_usage(AndroidUsage::empty()),
            HardwareBufferUsage::default()
        );
    }

    #[test]
    fn only_rgba_and_sides_the_device_takes_are_allocated() {
        let refused =
            |width, height, format| refusal(width, height, format, 4096).map(|error| error.kind);
        assert_eq!(
            refused(8, 8, PixelFormat::Bgra8),
            Some(ImageErrorKind::Unsupported)
        );
        assert_eq!(
            refused(4097, 8, PixelFormat::Rgba8),
            Some(ImageErrorKind::TooLarge)
        );
        assert_eq!(
            refused(0, 8, PixelFormat::Rgba8),
            Some(ImageErrorKind::Other)
        );
        assert_eq!(refused(4096, 8, PixelFormat::Rgba8), None);
    }
}

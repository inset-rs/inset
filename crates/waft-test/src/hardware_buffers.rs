//! Hardware buffers in memory, as a host with no GPU of its own hands them out: each knows its
//! size and usage, is imported as no texture (`TestPlatform::import_texture`), and reads back
//! opaque black, in BGRA as Apple's buffers are, where its usage has `cpu_read`; and the system's
//! allocator, which allocates them for exactly what the app asked for, and keeps each ask. A
//! buffer the codecs decode or a video frame's is for what Apple's systems give one
//! ([`SYSTEM_GIVEN`]).

use std::sync::{Arc, Mutex};

use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageFuture, PixelBuffer, PixelFormat,
};
use waft_embedder::{HardwareBuffer, HardwareBufferAllocator, HardwareBufferUsage};

use crate::image_codecs::pixels_of;

/// What a decoded image's buffer and a video frame's are for: sampled, as the system made them,
/// and shown by the compositor and read by the CPU, as on Apple's systems.
pub(crate) const SYSTEM_GIVEN: HardwareBufferUsage = HardwareBufferUsage {
    composer_overlay: true,
    cpu_read: true,
    ..HardwareBufferUsage::GPU_SAMPLED
};

/// A hardware buffer of a size, with no pixels on any GPU: it is imported as no texture, as a
/// buffer a device cannot take, which a drawing skips, and its pixels read back opaque black.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MockHardwareBuffer {
    pub width: u32,
    pub height: u32,
    pub usage: HardwareBufferUsage,
}

impl HardwareBuffer for MockHardwareBuffer {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn usage(&self) -> HardwareBufferUsage {
        self.usage
    }

    fn read_pixels(&self) -> ImageFuture<PixelBuffer> {
        let read = if !self.usage.cpu_read {
            Err(ImageError::new(
                ImageErrorKind::Unsupported,
                "a buffer whose usage lacks `cpu_read`",
            ))
        } else if self.width == 0 || self.height == 0 {
            Err(ImageError::new(
                ImageErrorKind::Other,
                "a buffer with no pixels",
            ))
        } else {
            Ok(pixels_of(
                [self.width, self.height],
                [0, 0, 0, 255],
                PixelFormat::Bgra8,
            ))
        };
        Box::pin(std::future::ready(read))
    }
}

/// The system's hardware buffer allocator, which allocates [`MockHardwareBuffer`]s in BGRA, as
/// Apple's systems do, and keeps each allocation asked for. Every clone is the same allocator.
#[derive(Clone, Default)]
pub struct MockHardwareBufferAllocator(Arc<Mutex<Vec<AskedAllocation>>>);

/// An allocation the app asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AskedAllocation {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub usage: HardwareBufferUsage,
}

impl MockHardwareBufferAllocator {
    pub fn new() -> MockHardwareBufferAllocator {
        MockHardwareBufferAllocator::default()
    }

    /// The allocations asked for, in order, those refused among them.
    pub fn asked(&self) -> Vec<AskedAllocation> {
        self.0.lock().expect("never held across a panic").clone()
    }
}

impl HardwareBufferAllocator for MockHardwareBufferAllocator {
    /// A buffer of the size asked; [`ImageErrorKind::Unsupported`] for RGBA and
    /// [`ImageErrorKind::Other`] for a side of no pixels.
    fn allocate(
        &self,
        width: u32,
        height: u32,
        format: PixelFormat,
        usage: HardwareBufferUsage,
    ) -> Result<Arc<dyn HardwareBuffer>, ImageError> {
        self.0
            .lock()
            .expect("never held across a panic")
            .push(AskedAllocation {
                width,
                height,
                format,
                usage,
            });
        if width == 0 || height == 0 {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                "a buffer with no pixels",
            ));
        }
        if format != PixelFormat::Bgra8 {
            return Err(ImageError::new(
                ImageErrorKind::Unsupported,
                "these buffers are BGRA",
            ));
        }
        Ok(Arc::new(MockHardwareBuffer {
            width,
            height,
            usage,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("answers at once"),
        }
    }

    #[test]
    fn a_buffer_answers_what_it_was_allocated_for_and_is_read_only_with_cpu_read() {
        let allocator = MockHardwareBufferAllocator::new();
        let drawn = allocator
            .allocate(4, 4, PixelFormat::Bgra8, HardwareBufferUsage::GPU_SAMPLED)
            .expect("a buffer");
        assert_eq!(drawn.usage(), HardwareBufferUsage::GPU_SAMPLED);
        let refused = now(drawn.read_pixels()).err().map(|error| error.kind);
        assert_eq!(refused, Some(ImageErrorKind::Unsupported));

        let read = HardwareBufferUsage {
            cpu_read: true,
            ..HardwareBufferUsage::default()
        };
        let readable = allocator
            .allocate(4, 4, PixelFormat::Bgra8, read)
            .expect("a buffer");
        assert!(now(readable.read_pixels()).is_ok());
    }
}

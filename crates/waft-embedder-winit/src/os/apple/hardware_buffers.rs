//! Hardware buffers on macOS and iOS: an `IOSurface`, in 32-bit BGRA, the memory Core Animation
//! shows as a layer's contents (`composition`), Metal draws into and samples through a texture
//! imported over it (`gpu_interop`), Image I/O encodes and decodes into (`image_codecs`), and a
//! video frame's pixel buffer lies in (`media`). A buffer the app allocates is made by
//! `IOSurfaceCreate`; a video frame's is its pixel buffer's surface, held with the pixel buffer,
//! which keeps the frame's slot in its pool. Its pixels are read while the surface is locked
//! (`media::locked_pixels`).
//!
//! Any `IOSurface` can be sampled, drawn into, shown by Core Animation and read by the CPU, so a
//! buffer of the app's own, however it was allocated, answers every usage; one the system hands
//! out, a decoded image's or a video frame's, answers what the system made it for, sampling, with
//! the compositor's and the CPU's uses every surface has (`SYSTEM_GIVEN`).
//!
//! The invariant: a buffer holds its surface, and a frame's buffer the frame, for as long as
//! anything holds the buffer, since a CoreVideo pool hands a frame's surface out again once nothing
//! holds the frame, whatever still reads the surface (`experiments/2026-11-01-corevideo-pool-reuse`).

// Allocating an `IOSurface` needs `unsafe`; this module writes it for it.
#![allow(unsafe_code)]

use std::sync::Arc;

use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString};
use objc2_core_video::{CVPixelBuffer, CVPixelBufferGetIOSurface, kCVPixelFormatType_32BGRA};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow, kIOSurfaceHeight,
    kIOSurfacePixelFormat, kIOSurfaceWidth,
};
use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageFuture, PixelBuffer, PixelFormat,
};
use waft_embedder::{HardwareBuffer, HardwareBufferAllocator, HardwareBufferUsage};

use super::media::locked_pixels::read_locked;

/// The largest side of a buffer, in pixels: the largest 2D texture Metal makes on every GPU since
/// Apple's A9 and on every Mac.
const LARGEST_SIDE: u32 = 16384;

/// Bytes one pixel of 32-bit BGRA occupies.
const BYTES_PER_PIXEL: usize = 4;

/// What a buffer the system hands out is for: sampled by the GPU, as the system made it, and shown
/// by Core Animation and read by the CPU, as any surface is.
const SYSTEM_GIVEN: HardwareBufferUsage = HardwareBufferUsage {
    composer_overlay: true,
    cpu_read: true,
    ..HardwareBufferUsage::GPU_SAMPLED
};

/// A hardware buffer on macOS and iOS: an `IOSurface` in 32-bit BGRA, what it can be used for,
/// and the video frame it is the memory of, where it is one's. Platform
/// code reaches the surface from a [`HardwareBuffer`] by downcasting through `Any`:
///
/// ```
/// # fn platform_code(buffer: std::sync::Arc<dyn waft_embedder::HardwareBuffer>) {
/// use waft_embedder_winit::SurfaceBuffer;
///
/// let buffer: &dyn std::any::Any = &*buffer;
/// if let Some(buffer) = buffer.downcast_ref::<SurfaceBuffer>() {
///     let _surface = buffer.surface();
/// }
/// # }
/// ```
#[derive(Clone)]
pub struct SurfaceBuffer {
    surface: CFRetained<IOSurfaceRef>,
    usage: HardwareBufferUsage,
    /// The frame whose memory the surface is, held so its pool does not hand the surface out
    /// again; none for a buffer of the app's own.
    _frame: Option<CFRetained<CVPixelBuffer>>,
}

// SAFETY: an `IOSurface` and a pixel buffer are made to be shared between threads and processes:
// their retain counts and the surface's use count are atomic, and nothing here writes their pixels
// but the app's own GPU work and Core Graphics' drawing of a decoded image before it is handed
// out. The bindings cannot say so.
unsafe impl Send for SurfaceBuffer {}
unsafe impl Sync for SurfaceBuffer {}

impl SurfaceBuffer {
    /// The buffer of `surface`, an `IOSurface` in 32-bit BGRA, for every use, as any surface is:
    /// what platform code that holds the system's own surface, as WaOS does for a program, hands
    /// the app's compositor and codecs.
    pub fn new(surface: CFRetained<IOSurfaceRef>) -> SurfaceBuffer {
        SurfaceBuffer::for_usage(surface, HardwareBufferUsage::ALL)
    }

    /// The buffer of `surface` the system hands out, as a decoded image's is, for what it made it
    /// for (`SYSTEM_GIVEN`).
    pub(in crate::os::apple) fn system_given(surface: CFRetained<IOSurfaceRef>) -> SurfaceBuffer {
        SurfaceBuffer::for_usage(surface, SYSTEM_GIVEN)
    }

    fn for_usage(surface: CFRetained<IOSurfaceRef>, usage: HardwareBufferUsage) -> SurfaceBuffer {
        SurfaceBuffer {
            surface,
            usage,
            _frame: None,
        }
    }

    /// The memory of `frame`, a pixel buffer on an `IOSurface`, for what the system made it for
    /// (`SYSTEM_GIVEN`), held with the frame; `None` for a pixel buffer on no surface.
    pub(crate) fn of_frame(frame: CFRetained<CVPixelBuffer>) -> Option<SurfaceBuffer> {
        let surface = CVPixelBufferGetIOSurface(Some(&frame))?;
        Some(SurfaceBuffer {
            _frame: Some(frame),
            ..SurfaceBuffer::system_given(surface)
        })
    }

    /// The `IOSurface` the buffer is.
    pub fn surface(&self) -> &IOSurfaceRef {
        &self.surface
    }
}

impl HardwareBuffer for SurfaceBuffer {
    fn width(&self) -> u32 {
        self.surface.width() as u32
    }

    fn height(&self) -> u32 {
        self.surface.height() as u32
    }

    /// Every use for a buffer of the app's own, and `SYSTEM_GIVEN` for one the system handed
    /// out.
    fn usage(&self) -> HardwareBufferUsage {
        self.usage
    }

    /// The surface's rows as they lie, BGRA and premultiplied, copied while it is locked for
    /// reading. Every buffer here has `cpu_read`, so the usage refuses none.
    fn read_pixels(&self) -> ImageFuture<PixelBuffer> {
        let read = read_locked(&self.surface, |pixels| pixels.copied()).and_then(|copied| copied);
        Box::pin(std::future::ready(read))
    }
}

/// The system's hardware buffer allocator on macOS and iOS, which allocates `IOSurface`s in
/// 32-bit BGRA, each for every use whatever the app asked for.
pub(crate) struct SurfaceBufferAllocator;

impl HardwareBufferAllocator for SurfaceBufferAllocator {
    fn allocate(
        &self,
        width: u32,
        height: u32,
        format: PixelFormat,
        _usage: HardwareBufferUsage,
    ) -> Result<Arc<dyn HardwareBuffer>, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                format!("a buffer of {width} by {height} pixels, which holds none"),
            ));
        }
        if format != PixelFormat::Bgra8 {
            return Err(ImageError::new(
                ImageErrorKind::Unsupported,
                "Apple's systems allocate hardware buffers in BGRA",
            ));
        }
        if width.max(height) > LARGEST_SIDE {
            return Err(ImageError::new(
                ImageErrorKind::TooLarge,
                format!("a side past {LARGEST_SIDE} pixels, the largest Metal makes a texture of"),
            ));
        }
        let surface = allocate(width as usize, height as usize).ok_or_else(|| {
            ImageError::new(
                ImageErrorKind::OutOfMemory,
                "the system made no IOSurface of that size",
            )
        })?;
        Ok(Arc::new(SurfaceBuffer::new(surface)))
    }
}

/// An `IOSurface` of `width` by `height` pixels in 32-bit BGRA; `None` where the system makes
/// none.
pub(in crate::os::apple) fn allocate(
    width: usize,
    height: usize,
) -> Option<CFRetained<IOSurfaceRef>> {
    // SAFETY: the keys are constants IOSurface exports for the life of the process.
    let keys: [&CFString; 5] = unsafe {
        [
            kIOSurfaceWidth,
            kIOSurfaceHeight,
            kIOSurfaceBytesPerElement,
            kIOSurfaceBytesPerRow,
            kIOSurfacePixelFormat,
        ]
    };
    // IOSurface picks its own row stride; asking for the aligned one keeps Metal's view of the
    // rows and everyone else's identical.
    let bytes_per_row = IOSurfaceRef::align_property(keys[3], width * BYTES_PER_PIXEL);
    let numbers = [
        CFNumber::new_isize(width as isize),
        CFNumber::new_isize(height as isize),
        CFNumber::new_isize(BYTES_PER_PIXEL as isize),
        CFNumber::new_isize(bytes_per_row as isize),
        CFNumber::new_i32(kCVPixelFormatType_32BGRA as i32),
    ];
    let values: [&CFNumber; 5] = numbers.each_ref().map(|number| &**number);
    let properties = CFDictionary::from_slices(&keys, &values);
    // SAFETY: every value is of the type its key names.
    unsafe { IOSurfaceRef::new(properties.as_opaque()) }
}

#[cfg(test)]
mod tests {
    use waft_embedder::VideoFrame;

    use super::*;

    const DRAWN: HardwareBufferUsage = HardwareBufferUsage {
        gpu_color_output: true,
        ..HardwareBufferUsage::GPU_SAMPLED
    };

    #[test]
    fn a_buffer_allocated_answers_every_use_whatever_was_asked() {
        let buffer = SurfaceBufferAllocator
            .allocate(8, 8, PixelFormat::Bgra8, HardwareBufferUsage::GPU_SAMPLED)
            .expect("an IOSurface of 8 by 8");
        assert_eq!(buffer.usage(), HardwareBufferUsage::ALL);
        assert!(pollster::block_on(buffer.read_pixels()).is_ok());
    }

    #[test]
    fn a_frames_buffer_answers_what_the_system_made_it_for() {
        let frame = super::super::media::test_frame();
        let buffer = frame.hardware_buffer();
        assert_eq!(buffer.usage(), SYSTEM_GIVEN);
        assert!(!buffer.usage().gpu_color_output);
    }

    #[test]
    fn only_bgra_and_sides_metal_takes_are_allocated() {
        let refused = |width, height, format| {
            SurfaceBufferAllocator
                .allocate(width, height, format, DRAWN)
                .err()
                .map(|error| error.kind)
        };
        assert_eq!(
            refused(8, 8, PixelFormat::Rgba8),
            Some(ImageErrorKind::Unsupported)
        );
        assert_eq!(
            refused(LARGEST_SIDE + 1, 8, PixelFormat::Bgra8),
            Some(ImageErrorKind::TooLarge)
        );
        assert_eq!(
            refused(0, 8, PixelFormat::Bgra8),
            Some(ImageErrorKind::Other)
        );
    }
}

//! A hardware buffer's pixels read where they lie: its `IOSurface` locked for reading, as IOSurface
//! asks before the CPU reads its memory, its rows handed to whoever reads them, and unlocked after:
//! copied into the app's memory (`hardware_buffers`' `read_pixels`, and a video frame's, through
//! its buffer), or encoded in place by the hardware image codecs (`apple/image_codecs`).
//!
//! The invariant: nothing that reads the rows outlives the lock, which their borrow keeps.

use objc2_core_video::kCVPixelFormatType_32BGRA;
use objc2_io_surface::{IOSurfaceLockOptions, IOSurfaceRef};
use waft_embedder::image_codecs::{
    AlphaType, ImageError, ImageErrorKind, PixelBuffer, PixelFormat, PixelLayout,
};

/// A buffer's pixels while its surface is locked: 32-bit BGRA, premultiplied, `height` rows of
/// `bytes_per_row`, each starting with `width` pixels.
pub(in crate::os::apple) struct LockedPixels<'a> {
    pub(in crate::os::apple) width: usize,
    pub(in crate::os::apple) height: usize,
    pub(in crate::os::apple) bytes_per_row: usize,
    pub(in crate::os::apple) rows: &'a [u8],
}

impl LockedPixels<'_> {
    /// The pixels copied into the app's memory, rows as they lie.
    pub(in crate::os::apple) fn copied(&self) -> Result<PixelBuffer, ImageError> {
        let size = |length: usize| u32::try_from(length).ok();
        let (Some(width), Some(height), Some(row_bytes)) = (
            size(self.width),
            size(self.height),
            size(self.bytes_per_row),
        ) else {
            return Err(ImageError::new(
                ImageErrorKind::TooLarge,
                "a buffer larger than a pixel buffer holds",
            ));
        };
        let layout = PixelLayout {
            size: [width, height],
            row_bytes,
            format: PixelFormat::Bgra8,
            alpha: AlphaType::Premultiplied,
        };
        PixelBuffer::new(layout, self.rows.to_vec())
            .map_err(|error| ImageError::new(ImageErrorKind::Other, format!("{error:?}")))
    }
}

/// What `read` makes of `surface`'s pixels, read while the surface is locked for reading:
/// [`ImageErrorKind::Unsupported`] for a surface in another format than the BGRA buffers are made
/// in.
pub(in crate::os::apple) fn read_locked<R>(
    surface: &IOSurfaceRef,
    read: impl FnOnce(LockedPixels<'_>) -> R,
) -> Result<R, ImageError> {
    let format = surface.pixel_format();
    if format != kCVPixelFormatType_32BGRA {
        return Err(ImageError::new(
            ImageErrorKind::Unsupported,
            format!("a buffer in the pixel format {format:#x}, not 32-bit BGRA"),
        ));
    }
    // SAFETY: a read-only lock with no seed, released below; the base address is read only
    // between the two, and the rows' borrow ends with `read`, before the unlock.
    unsafe {
        if surface.lock(IOSurfaceLockOptions::ReadOnly, std::ptr::null_mut()) != 0 {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                "the system did not lock the buffer's pixels",
            ));
        }
        let height = surface.height();
        let bytes_per_row = surface.bytes_per_row();
        let base = surface.base_address().as_ptr().cast::<u8>().cast_const();
        let rows = std::slice::from_raw_parts(base, height * bytes_per_row);
        let answer = read(LockedPixels {
            width: surface.width(),
            height,
            bytes_per_row,
            rows,
        });
        surface.unlock(IOSurfaceLockOptions::ReadOnly, std::ptr::null_mut());
        Ok(answer)
    }
}

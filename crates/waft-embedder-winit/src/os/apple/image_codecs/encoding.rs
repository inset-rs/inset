//! Pixels encoded: a `CGImage` over them, with no copy, written by a `CGImageDestination` of the
//! format's UTI into bytes, at the quality asked where given. The app's pixels are read from its
//! own memory, in their layout; a hardware buffer's in place, its 32-bit BGRA rows read while its
//! surface is locked (`media::locked_pixels`). Nothing is written but the picture: no metadata.
//!
//! The invariant: the image over the pixels, its data and its provider are let go before the
//! pixels are, or the buffer unlocked.

use objc2::runtime::AnyObject;
use objc2_core_foundation::{
    CFData, CFDictionary, CFIndex, CFMutableData, CFRetained, kCFAllocatorNull,
};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use objc2_image_io::{CGImageDestination, kCGImageDestinationLossyCompressionQuality};
use waft_embedder::image_codecs::{
    AlphaType, ImageError, ImageErrorKind, PixelBuffer, PixelFormat,
};

use super::formats::encoded_uti;
use crate::os::apple::hardware_buffers::SurfaceBuffer;
use crate::os::apple::media::as_ns_string;
use crate::os::apple::media::locked_pixels::read_locked;

/// `pixels` encoded in `format`, a MIME type, at `quality`.
pub(super) fn pixels_encoded(
    pixels: &PixelBuffer,
    format: &str,
    quality: Option<f32>,
) -> Result<Vec<u8>, ImageError> {
    let uti = encoded_uti(format)?;
    let layout = pixels.layout();
    let rows = Rows {
        bytes: pixels.bytes(),
        width: layout.size[0] as usize,
        height: layout.size[1] as usize,
        bytes_per_row: layout.row_bytes as usize,
        bitmap: bitmap_of(layout.format, layout.alpha),
    };
    encoded(&rows, &uti, quality)
}

/// `buffer`'s pixels encoded in `format`, a MIME type, at `quality`, read in place while its
/// surface is locked.
pub(super) fn buffer_encoded(
    buffer: &SurfaceBuffer,
    format: &str,
    quality: Option<f32>,
) -> Result<Vec<u8>, ImageError> {
    let uti = encoded_uti(format)?;
    read_locked(buffer.surface(), |pixels| {
        let rows = Rows {
            bytes: pixels.rows,
            width: pixels.width,
            height: pixels.height,
            bytes_per_row: pixels.bytes_per_row,
            bitmap: bitmap_of(PixelFormat::Bgra8, AlphaType::Premultiplied),
        };
        encoded(&rows, &uti, quality)
    })?
}

/// Pixels as Core Graphics reads them: `height` rows of `bytes_per_row` bytes, each starting with
/// `width` pixels of four bytes laid out as `bitmap` says.
struct Rows<'a> {
    bytes: &'a [u8],
    width: usize,
    height: usize,
    bytes_per_row: usize,
    bitmap: CGBitmapInfo,
}

/// Core Graphics' words for `format` and `alpha`: the alpha's place and kind, and the order of a
/// pixel's bytes as one 32-bit word.
fn bitmap_of(format: PixelFormat, alpha: AlphaType) -> CGBitmapInfo {
    let (alpha, order) = match (format, alpha) {
        (PixelFormat::Rgba8, AlphaType::Premultiplied) => (
            CGImageAlphaInfo::PremultipliedLast,
            CGImageByteOrderInfo::Order32Big,
        ),
        (PixelFormat::Rgba8, AlphaType::Straight) => {
            (CGImageAlphaInfo::Last, CGImageByteOrderInfo::Order32Big)
        }
        (PixelFormat::Bgra8, AlphaType::Premultiplied) => (
            CGImageAlphaInfo::PremultipliedFirst,
            CGImageByteOrderInfo::Order32Little,
        ),
        (PixelFormat::Bgra8, AlphaType::Straight) => {
            (CGImageAlphaInfo::First, CGImageByteOrderInfo::Order32Little)
        }
    };
    CGBitmapInfo(alpha.0 | order.0)
}

/// `rows` written by a destination of `uti` into bytes, at `quality` where given.
fn encoded(rows: &Rows<'_>, uti: &NSString, quality: Option<f32>) -> Result<Vec<u8>, ImageError> {
    if quality.is_some_and(|quality| !(0.0..=1.0).contains(&quality)) {
        return Err(ImageError::new(
            ImageErrorKind::Other,
            format!("a quality of {quality:?}, outside 0 to 1"),
        ));
    }
    let image = image_over(rows)?;
    let written = CFMutableData::new(None, 0)
        .ok_or_else(|| other("the system made no data to encode into"))?;
    let uti: *const NSString = uti;
    // SAFETY: an NSString is the CFString it is toll-free bridged to.
    let uti = unsafe { &*uti.cast::<objc2_core_foundation::CFString>() };
    // SAFETY: one image of a type Image I/O writes, and no options.
    let destination = unsafe { CGImageDestination::with_data(&written, uti, 1, None) }
        .ok_or_else(|| other("Image I/O made no destination of the format"))?;
    let options = quality.map(quality_options);
    let options = options.as_deref().map(|options| {
        let options: *const NSDictionary<NSString, AnyObject> = options;
        // SAFETY: an NSDictionary is the CFDictionary it is toll-free bridged to.
        unsafe { &*options.cast::<CFDictionary>() }
    });
    // SAFETY: the image and options of the types Image I/O's keys name; the destination is
    // finalized and let go, with the image, before the pixels are.
    let finished = unsafe {
        destination.add_image(&image, options);
        destination.finalize()
    };
    drop(destination);
    drop(image);
    if !finished {
        return Err(other("Image I/O could not write the image"));
    }
    let written: &CFData = &written;
    Ok(written.to_vec())
}

/// A lossy format's quality, from 0 to 1.
fn quality_options(quality: f32) -> objc2::rc::Retained<NSDictionary<NSString, AnyObject>> {
    let quality = NSNumber::new_f32(quality);
    // SAFETY: a constant Image I/O exports for the life of the process, a CF string.
    let key = as_ns_string(unsafe { kCGImageDestinationLossyCompressionQuality });
    NSDictionary::from_slices(&[key], &[quality.as_ref() as &AnyObject])
}

/// A `CGImage` over `rows`, which reads them where they are, in sRGB.
fn image_over(rows: &Rows<'_>) -> Result<CFRetained<CGImage>, ImageError> {
    let length = rows.height * rows.bytes_per_row;
    if rows.width == 0 || rows.bytes_per_row < rows.width * 4 || rows.bytes.len() < length {
        return Err(ImageError::new(
            ImageErrorKind::Unsupported,
            "pixels whose rows do not hold them",
        ));
    }
    // SAFETY: the bytes outlive the data, which only the provider and the image hold, and both
    // are let go before the bytes are (the module's invariant); a null deallocator leaves the
    // bytes to their owner.
    let data = unsafe {
        CFData::with_bytes_no_copy(
            None,
            rows.bytes.as_ptr(),
            length as CFIndex,
            kCFAllocatorNull,
        )
    }
    .ok_or_else(|| other("the system made no data over the pixels"))?;
    let provider = CGDataProvider::with_cf_data(Some(&data))
        .ok_or_else(|| other("the system made no provider of the pixels"))?;
    // SAFETY: a constant Core Graphics exports for the life of the process.
    let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    // SAFETY: eight bits a channel, four channels a pixel, rows as long as the provider's bytes
    // hold, and no decode array.
    unsafe {
        CGImage::new(
            rows.width,
            rows.height,
            8,
            32,
            rows.bytes_per_row,
            srgb.as_deref(),
            rows.bitmap,
            Some(&provider),
            std::ptr::null(),
            false,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
    .ok_or_else(|| other("the system made no image of the pixels"))
}

fn other(message: &str) -> ImageError {
    ImageError::new(ImageErrorKind::Other, message)
}

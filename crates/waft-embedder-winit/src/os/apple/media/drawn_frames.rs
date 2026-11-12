//! A still picture drawn into GPU memory: a `CGImage` drawn, in 32-bit BGRA, premultiplied, in
//! sRGB, into a pixel buffer of its size backed by an `IOSurface`, the kind a frame reader hands
//! out (`frames`), so the app draws a media asset's picture as any video frame (`video_frame`,
//! `asset_frames`); or into an `IOSurface` of its size, a hardware buffer of the app's own, which
//! no pool waits for, as the hardware codecs decode an image (`hardware_buffers`,
//! `apple/image_codecs`). The cost is one copy of the picture, at the size it was made at, on the
//! CPU.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_foundation::{CFDictionary, CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferCreate, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
    kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_io_surface::IOSurfaceLockOptions;

use super::frames::pixel_buffer_attributes;
use super::video_frame::CoreVideoFrame;
use crate::os::apple::hardware_buffers::{self, SurfaceBuffer};

/// `image`, drawn into a frame of its size; why not, in the system's terms, where the system made
/// no pixel buffer or context for it.
pub(in crate::os::apple) fn frame_of(image: &CGImage) -> Result<CoreVideoFrame, String> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    let buffer = pixel_buffer(width, height)?;
    // SAFETY: the buffer is locked while its base address is drawn into, as a bitmap of its own
    // size, rows and format.
    unsafe {
        if CVPixelBufferLockBaseAddress(&buffer, CVPixelBufferLockFlags(0)) != kCVReturnSuccess {
            return Err("the system did not lock the picture's pixel buffer".to_owned());
        }
        let base = CVPixelBufferGetBaseAddress(&buffer);
        let drawn = draw(
            image,
            base,
            CVPixelBufferGetBytesPerRow(&buffer),
            width,
            height,
        );
        CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags(0));
        drawn?;
    }
    Ok(CoreVideoFrame::new(buffer))
}

/// `image`, drawn into a hardware buffer of its size, which answers what the system makes a decoded
/// image's buffer for (`SurfaceBuffer::system_given`); why not, in the system's
/// terms, where the system made no surface or context for it.
pub(in crate::os::apple) fn buffer_of(image: &CGImage) -> Result<SurfaceBuffer, String> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    let surface = hardware_buffers::allocate(width, height)
        .ok_or("the system made no IOSurface for the picture")?;
    // SAFETY: the surface is locked while its base address is drawn into, as a bitmap of its own
    // size, rows and format, with no seed.
    unsafe {
        let options = IOSurfaceLockOptions(0);
        if surface.lock(options, std::ptr::null_mut()) != 0 {
            return Err("the system did not lock the picture's surface".to_owned());
        }
        let base = surface.base_address().as_ptr();
        let drawn = draw(image, base, surface.bytes_per_row(), width, height);
        surface.unlock(options, std::ptr::null_mut());
        drawn?;
    }
    Ok(SurfaceBuffer::system_given(surface))
}

/// A pixel buffer of `width` by `height`, as a frame reader's frames are.
fn pixel_buffer(width: usize, height: usize) -> Result<CFRetained<CVPixelBuffer>, String> {
    let attributes = pixel_buffer_attributes();
    let attributes: *const _ = &*attributes;
    // SAFETY: an NSDictionary is the CFDictionary it is toll-free bridged to.
    let attributes = unsafe { &*attributes.cast::<CFDictionary>() };
    let mut buffer = std::ptr::null_mut();
    // SAFETY: attributes of the types their keys name, and `buffer` a valid place for the answer.
    let status = unsafe {
        CVPixelBufferCreate(
            None,
            width,
            height,
            kCVPixelFormatType_32BGRA,
            Some(attributes),
            NonNull::from(&mut buffer),
        )
    };
    let buffer = NonNull::new(buffer).filter(|_| status == kCVReturnSuccess);
    let buffer = buffer.ok_or("the system made no pixel buffer for the picture")?;
    // SAFETY: a buffer made just now, handed over with its one retain.
    Ok(unsafe { CFRetained::from_raw(buffer) })
}

/// Draws `image` over the whole of the `width` by `height` BGRA pixels at `base`, rows
/// `bytes_per_row` apart.
///
/// # Safety
///
/// `base` is locked memory of `height` rows of `bytes_per_row` bytes.
unsafe fn draw(
    image: &CGImage,
    base: *mut c_void,
    bytes_per_row: usize,
    width: usize,
    height: usize,
) -> Result<(), String> {
    // SAFETY: a constant Core Graphics exports for the life of the process.
    let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    // BGRA in memory: 32-bit words in little-endian order, alpha first and premultiplied.
    let layout = CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0;
    // SAFETY: the locked base address holds `height` rows of `bytes_per_row` bytes, each with
    // `width` pixels of four bytes.
    let context = unsafe {
        CGBitmapContextCreate(
            base,
            width,
            height,
            8,
            bytes_per_row,
            srgb.as_deref(),
            layout,
        )
    };
    let context = context.ok_or("the system made no context to draw the picture")?;
    let whole = CGRect::new(CGPoint::ZERO, CGSize::new(width as f64, height as f64));
    CGContext::draw_image(Some(&context), whole, Some(image));
    Ok(())
}

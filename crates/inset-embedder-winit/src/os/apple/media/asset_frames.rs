//! An asset's pictures: AVFoundation's `AVAssetImageGenerator`, which lands within the tolerance
//! asked, scales down to fit the size asked and applies the track's preferred transform, as a
//! player presents the picture, and answers a `CGImage` in the background. The image is drawn into
//! a pixel buffer backed by an `IOSurface`, in 32-bit BGRA, the kind a frame reader hands out
//! (`frames`), so the app draws it as any video frame (`video_frame`).
//!
//! The generator, not an `AVAssetReader`, since the reader decodes from a time forwards and leaves
//! the landing on a keyframe, the scaling and the track's rotation to its caller; the cost is one
//! copy of the picture, at the size asked, on the CPU.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use block2::RcBlock;
use inset_embedder::{
    MediaAssetFuture, PlayerError, PlayerErrorKind, SeekTolerance, Size, VideoFrame,
};
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAsset, AVAssetImageGenerator, AVAssetImageGeneratorResult, NSValueAVFoundationExtensions,
};
use objc2_core_foundation::{CFDictionary, CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_core_media::CMTime;
use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferCreate, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
    kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_foundation::{NSArray, NSError, NSValue};

use super::abort_on_panic;
use super::frames::pixel_buffer_attributes;
use super::media_times::{cm_time_of, tolerance_time};
use super::player_errors::error_of;
use super::video_frame::CoreVideoFrame;
use crate::answer::answer;

/// `asset`'s picture at `time`, within `tolerance`, scaled down to fit `max_size` where given.
pub(super) fn frame_at(
    asset: &AVAsset,
    time: Duration,
    tolerance: SeekTolerance,
    max_size: Option<Size>,
) -> MediaAssetFuture<Arc<dyn VideoFrame>> {
    let generator = generator_of(asset, tolerance, max_size);
    let (given, answered) = answer(Err(PlayerError {
        kind: PlayerErrorKind::Other,
        message: "the system answered no picture".to_owned(),
    }));
    let given = Mutex::new(Some(given));
    // Held by its own handler, so its request is not cancelled with it before it answers.
    let held = generator.clone();
    let handler = RcBlock::new(
        move |_requested: CMTime,
              image: *mut CGImage,
              _actual: CMTime,
              result: AVAssetImageGeneratorResult,
              error: *mut NSError| {
            abort_on_panic(|| {
                let _generator = &held;
                // SAFETY: the image and the error AVFoundation hands the call, each or null.
                let picture = unsafe { picture_of(image.as_ref(), result, error.as_ref()) };
                let given = given.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(given) = given {
                    given.give(picture);
                }
            })
        },
    );
    // SAFETY: a plain value of a time.
    let times =
        NSArray::from_retained_slice(&[unsafe { NSValue::valueWithCMTime(cm_time_of(time)) }]);
    // SAFETY: one time, and a handler that only draws the image it is handed and hands the frame
    // over, behind a mutex, from whichever queue AVFoundation calls it on.
    unsafe {
        generator.generateCGImagesAsynchronouslyForTimes_completionHandler(
            &times,
            RcBlock::as_ptr(&handler),
        )
    };
    Box::pin(answered)
}

/// A generator of `asset`'s pictures, landing within `tolerance`, presenting each as a player
/// does, and scaling it down to fit `max_size` where given.
fn generator_of(
    asset: &AVAsset,
    tolerance: SeekTolerance,
    max_size: Option<Size>,
) -> Retained<AVAssetImageGenerator> {
    // SAFETY: a generator of the asset, set up before it is asked anything.
    unsafe {
        let generator = AVAssetImageGenerator::assetImageGeneratorWithAsset(asset);
        generator.setAppliesPreferredTrackTransform(true);
        generator.setRequestedTimeToleranceBefore(tolerance_time(tolerance.before));
        generator.setRequestedTimeToleranceAfter(tolerance_time(tolerance.after));
        if let Some(size) = max_size {
            generator.setMaximumSize(CGSize::new(size.width(), size.height()));
        }
        generator
    }
}

/// The generator's answer as a frame: its image in a pixel buffer, or why there is none.
fn picture_of(
    image: Option<&CGImage>,
    result: AVAssetImageGeneratorResult,
    error: Option<&NSError>,
) -> Result<Arc<dyn VideoFrame>, PlayerError> {
    match image {
        Some(image) if result == AVAssetImageGeneratorResult::Succeeded => {
            let buffer = pixel_buffer_of(image)?;
            Ok(Arc::new(CoreVideoFrame::new(buffer)))
        }
        _ => Err(error_of(error)),
    }
}

/// `image`, drawn into a pixel buffer of its size backed by an `IOSurface`, in 32-bit BGRA, in
/// sRGB.
fn pixel_buffer_of(image: &CGImage) -> Result<CFRetained<CVPixelBuffer>, PlayerError> {
    let (width, height) = (CGImage::width(Some(image)), CGImage::height(Some(image)));
    let buffer = pixel_buffer(width, height)?;
    // SAFETY: the buffer is locked while its base address is drawn into, as a bitmap of its own
    // size, rows and format.
    unsafe {
        if CVPixelBufferLockBaseAddress(&buffer, CVPixelBufferLockFlags(0)) != kCVReturnSuccess {
            return Err(failure(
                "the system did not lock the picture's pixel buffer",
            ));
        }
        let drawn = draw(image, &buffer, width, height);
        CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags(0));
        drawn?;
    }
    Ok(buffer)
}

/// A pixel buffer of `width` by `height`, as a frame reader's frames are.
fn pixel_buffer(width: usize, height: usize) -> Result<CFRetained<CVPixelBuffer>, PlayerError> {
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
    let buffer =
        buffer.ok_or_else(|| failure("the system made no pixel buffer for the picture"))?;
    // SAFETY: a buffer made just now, handed over with its one retain.
    Ok(unsafe { CFRetained::from_raw(buffer) })
}

/// Draws `image` over the whole of `buffer`'s pixels, `width` by `height`.
///
/// # Safety
///
/// `buffer`'s base address is locked.
unsafe fn draw(
    image: &CGImage,
    buffer: &CVPixelBuffer,
    width: usize,
    height: usize,
) -> Result<(), PlayerError> {
    let base: *mut c_void = CVPixelBufferGetBaseAddress(buffer);
    let bytes_per_row = CVPixelBufferGetBytesPerRow(buffer);
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
    let context =
        context.ok_or_else(|| failure("the system made no context to draw the picture"))?;
    let whole = CGRect::new(CGPoint::ZERO, CGSize::new(width as f64, height as f64));
    CGContext::draw_image(Some(&context), whole, Some(image));
    Ok(())
}

fn failure(message: &str) -> PlayerError {
    PlayerError {
        kind: PlayerErrorKind::Other,
        message: message.to_owned(),
    }
}

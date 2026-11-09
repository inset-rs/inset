//! An asset's pictures: AVFoundation's `AVAssetImageGenerator`, which lands within the tolerance
//! asked, scales down to fit the size asked and applies the track's preferred transform, as a
//! player presents the picture, and answers a `CGImage` in the background. The image is drawn into
//! a frame of the kind a frame reader hands out (`drawn_frames`), so the app draws it as any video
//! frame.
//!
//! The generator, not an `AVAssetReader`, since the reader decodes from a time forwards and leaves
//! the landing on a keyframe, the scaling and the track's rotation to its caller; the cost is one
//! copy of the picture, at the size asked, on the CPU.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2_av_foundation::{
    AVAsset, AVAssetImageGenerator, AVAssetImageGeneratorResult, NSValueAVFoundationExtensions,
};
use objc2_core_foundation::CGSize;
use objc2_core_graphics::CGImage;
use objc2_core_media::CMTime;
use objc2_foundation::{NSArray, NSError, NSValue};
use waft_embedder::{
    MediaAssetFuture, PlayerError, PlayerErrorKind, SeekTolerance, Size, VideoFrame,
};

use super::abort_on_panic;
use super::drawn_frames::frame_of;
use super::media_times::{cm_time_of, tolerance_time};
use super::player_errors::error_of;
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

/// The generator's answer as a frame: its image drawn into a frame (`drawn_frames`), or why there
/// is none.
fn picture_of(
    image: Option<&CGImage>,
    result: AVAssetImageGeneratorResult,
    error: Option<&NSError>,
) -> Result<Arc<dyn VideoFrame>, PlayerError> {
    match image {
        Some(image) if result == AVAssetImageGeneratorResult::Succeeded => {
            let frame = frame_of(image).map_err(|message| PlayerError {
                kind: PlayerErrorKind::Other,
                message,
            })?;
            Ok(Arc::new(frame))
        }
        _ => Err(error_of(error)),
    }
}

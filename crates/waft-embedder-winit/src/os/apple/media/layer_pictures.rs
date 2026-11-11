//! What a player's or a camera's picture shown on a composition layer shows now, read beside the
//! layer the system draws it into, which the composition layer answers as its current picture, a
//! lent frame (`apple/composition`): Core Animation hands nothing back of what an `AVPlayerLayer`
//! or an `AVCaptureVideoPreviewLayer` shows, so the source feeds a second output of its own beside
//! it. A player's is an `AVPlayerItemVideoOutput` on the same item, a camera's its session's video
//! data output (`frames`), each frame a BGRA pixel buffer backed by an `IOSurface`.
//!
//! The invariant: a frame answered here is the pixel buffer itself, which holds its slot in its
//! pool, so the pool does not hand its surface out again while it is kept (`hardware_buffers`).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use objc2_av_foundation::AVPlayerItemVideoOutput;
use waft_embedder::VideoFrame;

use super::SystemPicture;
use super::frames::{CameraFrames, PlayerFrames};
use super::video_frame::CoreVideoFrame;

/// A player's picture on a composition layer: the frame for now of a video output beside the
/// layer, on the same item.
pub(super) struct PlayerPicture {
    frames: PlayerFrames,
    /// The frame read last: the output hands each frame out once, and a paused player's stays
    /// what it shows.
    last: Mutex<Option<CoreVideoFrame>>,
}

impl PlayerPicture {
    pub(super) fn new() -> PlayerPicture {
        PlayerPicture {
            frames: PlayerFrames::new(),
            last: Mutex::new(None),
        }
    }

    /// The output the player's item hands its pictures to, beside its layer.
    pub(super) fn output(&self) -> &AVPlayerItemVideoOutput {
        self.frames.output()
    }
}

impl SystemPicture for PlayerPicture {
    /// The item's frame for now, or the one read last while the output has no newer.
    fn current_frame(&self) -> Option<Arc<dyn VideoFrame>> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(newer) = self.frames.new_frame_for(Instant::now()) {
            *last = Some(newer);
        }
        Some(Arc::new(last.clone()?))
    }
}

/// A camera's picture on a composition layer: the newest frame of its session's video data
/// output.
pub(super) struct CameraPicture(pub(super) Arc<CameraFrames>);

impl SystemPicture for CameraPicture {
    fn current_frame(&self) -> Option<Arc<dyn VideoFrame>> {
        Some(Arc::new(self.0.newest()?))
    }
}

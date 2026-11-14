//! A running camera's pictures as the sessions' outputs take them: each newest picture of its
//! stream shown on every preview's layer (`preview`), drawn into an RGBA frame for the frame
//! readers it feeds, where it feeds any (`frame_conversion`), and kept as what the camera shows
//! now, which a preview's layer is read from (`SystemPicture`).
//!
//! The invariants: a reader is answered each frame once, and a frame answered is of the newest
//! picture, or of one before it, never a stale one after a newer; a picture is drawn into RGBA only
//! while a reader is fed, or when a layer is read.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Instant;

use waft_embedder::VideoFrame;

use super::camera_images::CameraImage;
use super::frame_conversion::to_rgba;
use super::frames::FrameSource;
use super::orientation::SensorMount;
use super::preview::CameraPreview;
use super::video_frame::AndroidVideoFrame;
use crate::composition::{SurfaceControlLayer, SystemPicture};
use crate::hardware_buffers::AndroidHardwareBuffer;

/// What a running camera's outputs take of its pictures.
pub(super) struct CameraFrames {
    mount: SensorMount,
    /// The layers its preview shows on.
    previews: Mutex<Vec<Weak<CameraPreview>>>,
    newest: Mutex<Newest>,
    /// How many readers it feeds.
    readers: AtomicUsize,
}

/// The newest picture, numbered, and the newest frame drawn of a picture.
#[derive(Default)]
struct Newest {
    number: u64,
    picture: Option<Arc<CameraImage>>,
    frame: Option<(u64, AndroidVideoFrame)>,
}

impl CameraFrames {
    pub(super) fn new(mount: SensorMount) -> Arc<CameraFrames> {
        Arc::new(CameraFrames {
            mount,
            previews: Mutex::new(Vec::new()),
            newest: Mutex::new(Newest::default()),
            readers: AtomicUsize::new(0),
        })
    }

    /// The camera's stream delivered `picture`: it is shown on every preview, and drawn for the
    /// readers where any is fed. On the stream's thread.
    pub(super) fn delivered(&self, picture: CameraImage) {
        let picture = Arc::new(picture);
        let number = {
            let mut newest = self.newest();
            newest.number += 1;
            newest.picture = Some(Arc::clone(&picture));
            newest.number
        };
        let transform = self.mount.preview_transform();
        let previews: Vec<_> = self.previews().iter().filter_map(Weak::upgrade).collect();
        for preview in previews {
            preview.show(&picture, transform);
        }
        if self.readers.load(Ordering::Relaxed) > 0 {
            self.draw_frame(&picture, number);
        }
    }

    /// Shows the camera's preview on `layer` from its next picture on, until the preview answered
    /// is dropped.
    pub(super) fn show_on(self: &Arc<Self>, layer: Arc<SurfaceControlLayer>) -> Arc<CameraPreview> {
        let picture: Arc<dyn SystemPicture> = Arc::new(CameraPicture(Arc::downgrade(self)));
        let preview = Arc::new(CameraPreview::new(layer, picture));
        let mut previews = self.previews();
        previews.retain(|preview| preview.strong_count() > 0);
        previews.push(Arc::downgrade(&preview));
        preview
    }

    /// A source of the camera's frames for one reader, drawn from its next picture on, until the
    /// source answered is dropped.
    pub(super) fn feed(self: &Arc<Self>) -> Arc<CameraFrameSource> {
        self.readers.fetch_add(1, Ordering::Relaxed);
        Arc::new(CameraFrameSource {
            frames: Arc::clone(self),
            answered: AtomicU64::new(0),
        })
    }

    /// `picture`, numbered `number`, drawn into an RGBA frame, kept as the newest unless a newer
    /// one was drawn meanwhile.
    fn draw_frame(&self, picture: &CameraImage, number: u64) -> Option<AndroidVideoFrame> {
        let drawn = picture
            .hardware_buffer()
            .map_err(|error| format!("no buffer of a camera's picture: {error}"))
            .and_then(|buffer| to_rgba(AndroidHardwareBuffer::new(buffer)));
        let frame = match drawn {
            Ok(buffer) => AndroidVideoFrame::new(buffer)
                .shown_turned(self.mount.rotation_angle(), self.mount.faces_user),
            Err(error) => {
                report_undrawn(&error);
                return None;
            }
        };
        let mut newest = self.newest();
        if newest
            .frame
            .as_ref()
            .is_none_or(|(drawn, _)| *drawn < number)
        {
            newest.frame = Some((number, frame.clone()));
        }
        Some(frame)
    }

    /// The newest frame drawn, if newer than `answered`, which it then is.
    fn frame_after(&self, answered: &AtomicU64) -> Option<AndroidVideoFrame> {
        let newest = self.newest();
        let (number, frame) = newest.frame.as_ref()?;
        (answered.fetch_max(*number, Ordering::Relaxed) < *number).then(|| frame.clone())
    }

    /// The frame of the newest picture, drawn now where it was not.
    fn current_frame(&self) -> Option<AndroidVideoFrame> {
        let (number, picture) = {
            let newest = self.newest();
            if let Some((drawn, frame)) = &newest.frame
                && *drawn == newest.number
            {
                return Some(frame.clone());
            }
            (newest.number, Arc::clone(newest.picture.as_ref()?))
        };
        self.draw_frame(&picture, number)
    }

    fn newest(&self) -> MutexGuard<'_, Newest> {
        self.newest.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn previews(&self) -> MutexGuard<'_, Vec<Weak<CameraPreview>>> {
        self.previews.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The camera's frames as one reader reads them: the newest, each once.
pub(super) struct CameraFrameSource {
    frames: Arc<CameraFrames>,
    /// The number of the picture of the frame answered last.
    answered: AtomicU64,
}

impl FrameSource for CameraFrameSource {
    /// The newest frame, once: a camera's frames are live, so each is for now.
    fn frame_for(&self, _display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
        Some(Arc::new(self.frames.frame_after(&self.answered)?))
    }
}

impl Drop for CameraFrameSource {
    fn drop(&mut self) {
        self.frames.readers.fetch_sub(1, Ordering::Relaxed);
    }
}

/// What a preview's layer shows: the camera's newest picture, as an RGBA frame.
struct CameraPicture(Weak<CameraFrames>);

impl SystemPicture for CameraPicture {
    fn current_frame(&self) -> Option<Arc<dyn VideoFrame>> {
        Some(Arc::new(self.0.upgrade()?.current_frame()?))
    }
}

/// Says once that a camera's picture could not be drawn into a frame, and why.
fn report_undrawn(error: &str) {
    static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!("waft: a camera's picture is not drawn into a frame: {error}");
    }
}

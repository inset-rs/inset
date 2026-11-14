//! A camera running for the app: one session's share of a camera this process runs
//! (`running_camera`), with the outputs the session started with: its preview on a layer
//! (`preview`), its frames in a reader (`camera_frames`), and its photos, taken with the camera's
//! JPEG stream. Its state is the camera's, which every session on it shares, and so are its two
//! rotation angles, one number, told changed as the screen turns.
//!
//! Recordings are not built: the NDK has no recorder, as AVFoundation's movie output is, only an
//! encoder and a muxer to build one of, so a session asked for movies starts without them, says so
//! once, and its recordings fail with that reason.
//!
//! The invariant: a session dropped leaves the layer it showed on showing nothing of the camera,
//! its reader reading nothing, and its change futures resolved; the camera stops with the last
//! session on it.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use waft_embedder::{
    CameraFormat, CameraOutputs, CaptureSession, CaptureState, MediaChangeFuture, MediaError,
    PhotoFuture, RecordingFuture, VideoFrameReader,
};

use super::camera_frames::CameraFrameSource;
use super::changes::Changes;
use super::frames::{FrameSource, NdkFrameReader, report_foreign_reader, report_reader_in_use};
use super::preview::CameraPreview;
use super::running_camera::RunningCamera;
use crate::composition::SurfaceControlLayer;

/// Why a session records nothing.
const NO_MOVIES: &str = "Android's host records no movies yet: the NDK has no recorder of its own";

/// One session on a camera this process runs.
pub(super) struct NdkCaptureSession {
    camera: Arc<RunningCamera>,
    changes: Changes,
    _preview: Option<Arc<CameraPreview>>,
    reader: Option<FedReader>,
    takes_photos: bool,
}

/// A reader the camera's frames go to, and the source it reads them from.
struct FedReader {
    reader: Arc<NdkFrameReader>,
    source: Arc<dyn FrameSource>,
}

impl NdkCaptureSession {
    /// A session on `camera` feeding `outputs`: its preview on a layer this host made, its frames
    /// to a reader this host made and no other source feeds.
    pub(super) fn new(camera: Arc<RunningCamera>, outputs: CameraOutputs) -> NdkCaptureSession {
        let changes = Changes::default();
        camera.watch(changes.clone());
        let preview = outputs
            .preview
            .and_then(|layer| show_preview(&camera, &layer));
        let reader = outputs.frames.and_then(|reader| feed(&camera, reader));
        if outputs.movies.is_some() {
            report_no_movies();
        }
        NdkCaptureSession {
            camera,
            changes,
            _preview: preview,
            reader,
            takes_photos: outputs.photos.is_some(),
        }
    }
}

impl CaptureSession for NdkCaptureSession {
    fn format(&self) -> CameraFormat {
        self.camera.format().clone()
    }

    fn state(&self) -> CaptureState {
        self.camera.state()
    }

    /// The capture's angle: Android tells no turn of the device apart from the screen's.
    fn video_rotation_angle_for_horizon_level_preview(&self) -> u32 {
        self.camera.rotation_angle()
    }

    fn video_rotation_angle_for_horizon_level_capture(&self) -> u32 {
        self.camera.rotation_angle()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }

    fn take_photo(&self) -> PhotoFuture {
        if !self.takes_photos {
            return failed("the session has no photos: they were not among its outputs");
        }
        self.camera.take_photo()
    }

    fn start_recording(&self, _path: PathBuf) -> RecordingFuture {
        failed(NO_MOVIES)
    }

    fn stop_recording(&self) {}
}

impl Drop for NdkCaptureSession {
    /// Its reader reads nothing more and its changes end; its preview leaves its layer as it
    /// drops, and the camera goes with its last session.
    fn drop(&mut self) {
        if let Some(fed) = self.reader.take() {
            fed.reader.stop_reading(&fed.source);
        }
        self.camera.unwatch(&self.changes);
        self.changes.end();
    }
}

/// The camera's preview on `layer`; `None` for a layer this host did not make, which shows
/// nothing.
fn show_preview(
    camera: &RunningCamera,
    layer: &Arc<dyn waft_embedder::CompositionLayer>,
) -> Option<Arc<CameraPreview>> {
    let Some(layer) = SurfaceControlLayer::downcast_from(layer) else {
        crate::log::warn_to_log!(
            "waft: a camera's preview was given a layer this host did not make; it shows nothing"
        );
        return None;
    };
    Some(camera.frames().show_on(layer))
}

/// The camera's frames read by `reader`; `None`, the frames going to no reader, for a reader this
/// host did not make or one another source feeds, which the host says once.
fn feed(camera: &RunningCamera, reader: Arc<dyn VideoFrameReader>) -> Option<FedReader> {
    let Some(reader) = NdkFrameReader::downcast_from(reader) else {
        report_foreign_reader();
        return None;
    };
    let source: Arc<CameraFrameSource> = camera.frames().feed();
    let source: Arc<dyn FrameSource> = source;
    if !reader.read(Arc::clone(&source)) {
        report_reader_in_use();
        return None;
    }
    Some(FedReader { reader, source })
}

/// Says once that a session asked for recordings, which this host does not make.
fn report_no_movies() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!("waft: a camera session asked for movies; {NO_MOVIES}");
    }
}

fn failed<T: 'static>(
    reason: &str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, MediaError>>>> {
    Box::pin(std::future::ready(Err(MediaError::Failed(
        reason.to_owned(),
    ))))
}

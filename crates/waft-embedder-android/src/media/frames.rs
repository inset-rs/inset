//! Frame readers: a reader the app makes on its own reads the frames of the one source it is given
//! to, as Android's `ImageReader` takes one producer: a camera session's (`camera_frames`), the
//! only source this host has.
//!
//! The invariant: a reader reads one source at a time, and a source that stops feeding it takes
//! away only itself.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use waft_embedder::{VideoFrame, VideoFrameReader, VideoFrameReaders};

/// The readers the host's sources feed.
pub(crate) struct NdkFrameReaders;

impl VideoFrameReaders for NdkFrameReaders {
    fn make_reader(&self) -> Arc<dyn VideoFrameReader> {
        Arc::new(NdkFrameReader::default())
    }
}

/// What feeds a reader, read from any thread.
pub(super) trait FrameSource: Send + Sync {
    /// The frame to show at `display_time`, as [`VideoFrameReader::frame_for`] answers it.
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>>;
}

/// A reader made on its own, which reads the one source feeding it.
#[derive(Default)]
pub(super) struct NdkFrameReader {
    /// `None` until a source feeds it, and once the source that fed it stops.
    source: Mutex<Option<Arc<dyn FrameSource>>>,
}

impl NdkFrameReader {
    /// `reader` as this host's own; `None` for a reader no `NdkFrameReaders` made.
    pub(super) fn downcast_from(reader: Arc<dyn VideoFrameReader>) -> Option<Arc<NdkFrameReader>> {
        let any: Arc<dyn std::any::Any + Send + Sync> = reader;
        any.downcast().ok()
    }

    /// Reads `source` from now on; false, reading on as before, while another source feeds it.
    pub(super) fn read(&self, source: Arc<dyn FrameSource>) -> bool {
        let mut read = self.source();
        if read.is_some() {
            return false;
        }
        *read = Some(source);
        true
    }

    /// Reads `source` no more; a reader fed by another source reads on.
    pub(super) fn stop_reading(&self, source: &Arc<dyn FrameSource>) {
        let mut read = self.source();
        if read.as_ref().is_some_and(|read| Arc::ptr_eq(read, source)) {
            *read = None;
        }
    }

    fn source(&self) -> MutexGuard<'_, Option<Arc<dyn FrameSource>>> {
        self.source.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl VideoFrameReader for NdkFrameReader {
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
        let source = self.source().clone()?;
        source.frame_for(display_time)
    }
}

/// Says once that a reader this host did not make was given to a source, which feeds it nothing.
pub(super) fn report_foreign_reader() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!(
            "waft: a frame reader this host did not make was given to a source; it reads nothing"
        );
    }
}

/// Says once that a reader another source feeds was given to a second one, which sends its frames
/// nowhere.
pub(super) fn report_reader_in_use() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!(
            "waft: a frame reader another source feeds was given to a second; the second sends \
             its frames nowhere"
        );
    }
}

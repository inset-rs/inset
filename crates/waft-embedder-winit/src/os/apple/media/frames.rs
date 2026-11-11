//! Frame readers: a reader the app makes on its own reads the output AVFoundation has for the kind
//! of source it is given to, a player's frames from an `AVPlayerItemVideoOutput` added to its item,
//! a camera's from an `AVCaptureVideoDataOutput`'s sample buffers. Each frame is a `CVPixelBuffer`
//! backed by an `IOSurface` in 32-bit BGRA, which the system converts the video into on the GPU,
//! so the app's GPU reads it with no copy (`video_frame`).
//!
//! AVFoundation has no output that both kinds of source feed, as Android's `ImageReader` is fed by
//! a decoder and a camera alike, so the source makes its own output and hands it to the reader.
//! The invariant: a reader reads one source's output at a time, as an `ImageReader` takes one
//! producer, and a source that stops feeding it takes away only its own output.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_av_foundation::{
    AVCaptureConnection, AVCaptureOutput, AVCaptureVideoDataOutput,
    AVCaptureVideoDataOutputSampleBufferDelegate, AVPlayerItemVideoOutput,
};
use objc2_core_foundation::CFRetained;
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{
    CVPixelBuffer, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey,
    kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use waft_embedder::{VideoFrame, VideoFrameReader, VideoFrameReaders};

use super::video_frame::CoreVideoFrame;
use super::{abort_on_panic, as_ns_string};
use crate::os::apple::media_time::media_time_of;

/// The readers AVFoundation's players and cameras feed.
pub(super) struct AvFrameReaders;

impl VideoFrameReaders for AvFrameReaders {
    fn make_reader(&self) -> Arc<dyn VideoFrameReader> {
        Arc::new(AvFrameReader::default())
    }
}

/// A reader made on its own, which reads the output of the one source feeding it.
#[derive(Default)]
pub(super) struct AvFrameReader {
    /// `None` until a source feeds it, and once the source that fed it stops.
    output: Mutex<Option<Arc<dyn FrameOutput>>>,
}

impl AvFrameReader {
    /// `reader` as this host's own; `None` for a reader no `AvFrameReaders` made.
    pub(super) fn downcast_from(reader: Arc<dyn VideoFrameReader>) -> Option<Arc<AvFrameReader>> {
        let any: Arc<dyn std::any::Any + Send + Sync> = reader;
        any.downcast().ok()
    }

    /// Reads `output` from now on; false, reading on as before, while another source's output
    /// feeds it.
    pub(super) fn read(&self, output: Arc<dyn FrameOutput>) -> bool {
        let mut read = self.output();
        if read.is_some() {
            return false;
        }
        *read = Some(output);
        true
    }

    /// Reads `output` no more; a reader fed by another source reads on.
    pub(super) fn stop_reading(&self, output: &Arc<dyn FrameOutput>) {
        let mut read = self.output();
        if read.as_ref().is_some_and(|read| Arc::ptr_eq(read, output)) {
            *read = None;
        }
    }

    fn output(&self) -> MutexGuard<'_, Option<Arc<dyn FrameOutput>>> {
        self.output.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl VideoFrameReader for AvFrameReader {
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
        let output = self.output().clone()?;
        output.frame_for(display_time)
    }
}

/// Says once that a reader this host did not make was given to a source, which feeds it nothing.
pub(super) fn report_foreign_reader() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "waft: a frame reader this host did not make was given to a source; it reads nothing"
        );
    }
}

/// Says once that a reader another source feeds was given to a second one, which sends its frames
/// nowhere.
pub(super) fn report_reader_in_use() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "waft: a frame reader another source feeds was given to a second; the second sends \
             its frames nowhere"
        );
    }
}

/// An AVFoundation output of one source's frames, which a reader reads from any thread.
pub(super) trait FrameOutput: Send + Sync {
    /// The frame to show at `display_time`, as [`VideoFrameReader::frame_for`] answers it.
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>>;
}

/// A player's frames: the newest picture its item's output has for now.
pub(super) struct PlayerFrames {
    output: CrossThreadOutput,
}

impl PlayerFrames {
    pub(super) fn new() -> PlayerFrames {
        // SAFETY: an output of pixel buffers with these attributes, which name CoreVideo's keys.
        let output = unsafe {
            AVPlayerItemVideoOutput::initWithPixelBufferAttributes(
                AVPlayerItemVideoOutput::alloc(),
                Some(&pixel_buffer_attributes()),
            )
        };
        PlayerFrames {
            output: CrossThreadOutput(output),
        }
    }

    /// The output the player's item hands its pictures to.
    pub(super) fn output(&self) -> &AVPlayerItemVideoOutput {
        &self.output.0
    }
}

impl PlayerFrames {
    /// The item's frame for the moment `display_time` is in the video, as Apple's display link
    /// pattern asks for its target time; `None` when the output handed that frame out already,
    /// or has none.
    pub(super) fn new_frame_for(&self, display_time: Instant) -> Option<CoreVideoFrame> {
        let output = &self.output.0;
        // SAFETY: the output answers from any thread, as a display link's thread reads it.
        let buffer = unsafe {
            let time = output.itemTimeForHostTime(media_time_of(display_time));
            if !output.hasNewPixelBufferForItemTime(time) {
                return None;
            }
            output.copyPixelBufferForItemTime_itemTimeForDisplay(time, std::ptr::null_mut())?
        };
        Some(CoreVideoFrame::new(cf_pixel_buffer_of(buffer)))
    }
}

impl FrameOutput for PlayerFrames {
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
        Some(Arc::new(self.new_frame_for(display_time)?))
    }
}

/// An item's video output handed between threads.
struct CrossThreadOutput(Retained<AVPlayerItemVideoOutput>);

// SAFETY: `AVPlayerItemVideoOutput` is read from a display link's thread as a matter of course;
// AVFoundation guards its own state, and the binding cannot say so.
unsafe impl Send for CrossThreadOutput {}
unsafe impl Sync for CrossThreadOutput {}

/// A camera's frames: the newest picture its video data output delivered, kept until the next
/// replaces it, and handed to a reader once.
pub(super) struct CameraFrames {
    newest: Arc<Mutex<Newest>>,
    output: CrossThreadVideoDataOutput,
    /// The delegate the output calls, which it holds weakly.
    _delegate: CrossThreadDelegate,
    /// The queue the delegate is called on, apart from the session's, which a start blocks.
    _queue: DispatchRetained<DispatchQueue>,
}

/// The newest picture a video data output delivered, and whether a reader has had it.
#[derive(Default)]
struct Newest {
    frame: Option<CoreVideoFrame>,
    answered: bool,
}

impl Newest {
    /// `frame` came, replacing the one before, for a reader to have.
    fn delivered(&mut self, frame: CoreVideoFrame) {
        self.frame = Some(frame);
        self.answered = false;
    }

    /// The newest frame, once: `None` when the reader had it already.
    fn for_reader(&mut self) -> Option<CoreVideoFrame> {
        if self.answered {
            return None;
        }
        self.answered = true;
        self.frame.clone()
    }
}

impl CameraFrames {
    /// An output delivering pixel buffers on a queue of its own, kept here as they come.
    pub(super) fn new() -> CameraFrames {
        let newest = Arc::new(Mutex::new(Newest::default()));
        let delegate = SampleBufferDelegate::alloc().set_ivars(Arc::clone(&newest));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let delegate: Retained<SampleBufferDelegate> = unsafe { msg_send![super(delegate), init] };
        let queue = DispatchQueue::new("rs.waft.camera-frames", DispatchQueueAttr::SERIAL);
        // SAFETY: a video data output in BGRA, whose late frames go, calling the delegate on
        // `queue`.
        let output = unsafe {
            let output = AVCaptureVideoDataOutput::new();
            output.setVideoSettings(Some(&pixel_buffer_attributes()));
            output.setAlwaysDiscardsLateVideoFrames(true);
            output.setSampleBufferDelegate_queue(
                Some(ProtocolObject::from_ref(&*delegate)),
                Some(&queue),
            );
            output
        };
        CameraFrames {
            newest,
            output: CrossThreadVideoDataOutput(output),
            _delegate: CrossThreadDelegate {
                _delegate: delegate,
            },
            _queue: queue,
        }
    }

    /// The output the session hands its pictures to.
    pub(super) fn output(&self) -> &AVCaptureVideoDataOutput {
        &self.output.0
    }

    /// The newest frame the output delivered, whether or not a reader has had it: what the
    /// camera shows now.
    pub(super) fn newest(&self) -> Option<CoreVideoFrame> {
        self.newest_guard().frame.clone()
    }

    fn newest_guard(&self) -> MutexGuard<'_, Newest> {
        self.newest.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl FrameOutput for CameraFrames {
    /// The newest frame, once: a camera's frames are live, so each is for now.
    fn frame_for(&self, _display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
        Some(Arc::new(self.newest_guard().for_reader()?))
    }
}

define_class!(
    /// Keeps the newest picture a video data output delivers.
    //
    // SAFETY: NSObject has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "WaftCameraFramesDelegate"]
    #[ivars = Arc<Mutex<Newest>>]
    struct SampleBufferDelegate;

    unsafe impl NSObjectProtocol for SampleBufferDelegate {}

    unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for SampleBufferDelegate {
        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        fn delivered(
            &self,
            _output: &AVCaptureOutput,
            sample_buffer: &CMSampleBuffer,
            _connection: &AVCaptureConnection,
        ) {
            abort_on_panic(|| {
                // SAFETY: the sample buffer is live for the call, and a retained image buffer
                // outlives it.
                let Some(image) = (unsafe { sample_buffer.image_buffer() }) else {
                    return;
                };
                self.ivars()
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .delivered(CoreVideoFrame::new(image));
            });
        }
    }
);

/// The attributes every frame reader asks its pictures in: 32-bit BGRA, backed by an
/// `IOSurface` a Metal texture can be made of.
pub(super) fn pixel_buffer_attributes() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: CoreVideo's constants, which it exports for the life of the process, are CF
    // strings, which are NSStrings.
    let (format_key, surface_key, metal_key) = unsafe {
        (
            as_ns_string(kCVPixelBufferPixelFormatTypeKey),
            as_ns_string(kCVPixelBufferIOSurfacePropertiesKey),
            as_ns_string(kCVPixelBufferMetalCompatibilityKey),
        )
    };
    let format = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
    let surface_properties = NSDictionary::<NSString, AnyObject>::new();
    let metal = NSNumber::new_bool(true);
    NSDictionary::from_slices(
        &[format_key, surface_key, metal_key],
        &[
            format.as_ref() as &AnyObject,
            surface_properties.as_ref(),
            metal.as_ref(),
        ],
    )
}

/// A pixel buffer AVFoundation answered retained, as the CF type a frame holds.
fn cf_pixel_buffer_of(buffer: Retained<CVPixelBuffer>) -> CFRetained<CVPixelBuffer> {
    let pointer = NonNull::new(Retained::into_raw(buffer)).expect("a retained buffer");
    // SAFETY: the pointer carries the +1 retain `Retained` gave up.
    unsafe { CFRetained::from_raw(pointer) }
}

/// A camera's video data output handed between threads.
struct CrossThreadVideoDataOutput(Retained<AVCaptureVideoDataOutput>);

// SAFETY: a capture output is configured on the session's queue and only held elsewhere;
// AVFoundation guards its own state, and the binding cannot say so.
unsafe impl Send for CrossThreadVideoDataOutput {}
unsafe impl Sync for CrossThreadVideoDataOutput {}

/// The sample buffer delegate handed between threads, kept while its output is: the output holds
/// it weakly.
struct CrossThreadDelegate {
    _delegate: Retained<SampleBufferDelegate>,
}

// SAFETY: the delegate's one state is behind a mutex; the output calls it on its queue.
unsafe impl Send for CrossThreadDelegate {}
unsafe impl Sync for CrossThreadDelegate {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    /// An output with no frame yet, which counts how often it is read.
    #[derive(Default)]
    struct CountedOutput(AtomicUsize);

    impl FrameOutput for CountedOutput {
        fn frame_for(&self, _display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
            self.0.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    impl CountedOutput {
        fn reads(&self) -> usize {
            self.0.load(Ordering::Relaxed)
        }
    }

    /// A reader takes one source at a time: a second is refused while the first feeds it, and
    /// taken once the first stops.
    #[test]
    fn a_reader_takes_one_source_at_a_time() {
        let reader = AvFrameReader::default();
        let (first, second) = (
            Arc::new(CountedOutput::default()),
            Arc::new(CountedOutput::default()),
        );
        assert!(reader.read(Arc::clone(&first) as Arc<dyn FrameOutput>));
        assert!(!reader.read(Arc::clone(&second) as Arc<dyn FrameOutput>));
        reader.frame_for(Instant::now());
        assert_eq!((first.reads(), second.reads()), (1, 0));

        reader.stop_reading(&(Arc::clone(&first) as Arc<dyn FrameOutput>));
        assert!(reader.read(Arc::clone(&second) as Arc<dyn FrameOutput>));
        reader.frame_for(Instant::now());
        assert_eq!((first.reads(), second.reads()), (1, 1));

        reader.stop_reading(&(Arc::clone(&second) as Arc<dyn FrameOutput>));
        reader.frame_for(Instant::now());
        assert_eq!(second.reads(), 1, "fed by nothing, it reads nothing");
    }

    /// A camera's newest frame goes to its reader once, and stays what the camera shows until the
    /// next comes.
    #[test]
    fn a_cameras_newest_frame_is_read_once_and_shown_until_the_next() {
        let mut newest = Newest::default();
        assert!(newest.for_reader().is_none(), "none came yet");
        let first = crate::os::apple::media::test_frame();
        newest.delivered(first.clone());
        assert!(is_frame(newest.for_reader(), &first));
        assert!(newest.for_reader().is_none(), "the reader had it");
        assert!(is_frame(newest.frame.clone(), &first), "still shown");
        let second = crate::os::apple::media::test_frame();
        newest.delivered(second.clone());
        assert!(is_frame(newest.frame.clone(), &second));
        assert!(is_frame(newest.for_reader(), &second));
    }

    /// Whether `frame` is `expected`'s pixel buffer.
    fn is_frame(frame: Option<CoreVideoFrame>, expected: &CoreVideoFrame) -> bool {
        frame.is_some_and(|frame| std::ptr::eq(frame.pixel_buffer(), expected.pixel_buffer()))
    }

    /// A source feeds a reader this host made, and not one made elsewhere.
    #[test]
    fn only_a_reader_this_host_made_is_its_own() {
        assert!(AvFrameReader::downcast_from(AvFrameReaders.make_reader()).is_some());
        let foreign: Arc<dyn VideoFrameReader> = Arc::new(ForeignReader);
        assert!(AvFrameReader::downcast_from(foreign).is_none());
    }

    struct ForeignReader;

    impl VideoFrameReader for ForeignReader {
        fn frame_for(&self, _display_time: Instant) -> Option<Arc<dyn VideoFrame>> {
            None
        }
    }
}

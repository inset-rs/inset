//! A camera session's photos and recordings: `AVCapturePhotoOutput`, which captures, processes
//! and encodes a photo at the size the app asked for, and `AVCaptureMovieFileOutput`, which
//! encodes and writes a movie, each answering through a delegate. An output holds its delegate
//! only weakly, so each answer's future keeps its delegate until it is answered.

use std::path::PathBuf;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, ClassType, DefinedClass, define_class, msg_send, sel};
use objc2_av_foundation::{
    AVCaptureConnection, AVCaptureFileOutput, AVCaptureFileOutputRecordingDelegate,
    AVCaptureMovieFileOutput, AVCaptureOutput, AVCapturePhoto, AVCapturePhotoCaptureDelegate,
    AVCapturePhotoOutput, AVCapturePhotoSettings, AVErrorRecordingSuccessfullyFinishedKey,
    AVVideoCodecKey, AVVideoCodecTypeJPEG,
};
use objc2_core_media::CMVideoDimensions;
use objc2_foundation::{NSArray, NSDictionary, NSError, NSNumber, NSString, NSURL};
use waft_embedder::{CaptureState, MediaError, Photo, PhotoFuture, RecordingFuture};

use super::abort_on_panic;
use super::capture_session::SessionShared;
use crate::media::{MediaAnswer, media_answer};

/// A session's photos, in JPEG, which every system reads. Every clone is the same output.
#[derive(Clone)]
pub(super) struct PhotoTaker(Arc<PhotoTakerShared>);

struct PhotoTakerShared {
    session: Arc<SessionShared>,
    output: CrossThreadPhotoOutput,
    /// The size each photo is taken at; `None` for the system's own.
    dimensions: Option<CMVideoDimensions>,
}

impl PhotoTaker {
    /// Photos of `session` at `dimensions`, or the system's own size.
    pub(super) fn new(
        session: Arc<SessionShared>,
        dimensions: Option<CMVideoDimensions>,
    ) -> PhotoTaker {
        // SAFETY: a new photo output, which the session takes on its queue.
        let output = unsafe { AVCapturePhotoOutput::new() };
        PhotoTaker(Arc::new(PhotoTakerShared {
            session,
            output: CrossThreadPhotoOutput(output),
            dimensions,
        }))
    }

    pub(super) fn output(&self) -> &AVCaptureOutput {
        &self.0.output.0
    }

    /// Readies the output for photos at its size, once the session has its format. On the
    /// session's queue.
    pub(super) fn prepare(&self) {
        let Some(dimensions) = self.0.dimensions.filter(|_| takes_photo_sizes()) else {
            return;
        };
        // SAFETY: one of the format's photo sizes, set on the session's queue before it runs, as
        // Apple's documentation of `maxPhotoDimensions` recommends.
        unsafe { self.0.output.0.setMaxPhotoDimensions(dimensions) };
    }

    /// Takes a photo while the session runs, answering it once the system has encoded it.
    pub(super) fn take(&self) -> PhotoFuture {
        if self.0.session.state() != CaptureState::Running {
            return Box::pin(std::future::ready(Err(MediaError::NotRunning)));
        }
        let (answer, answered) = media_answer();
        let delegate = PhotoDelegate::new(answer);
        let capture = CrossThreadPhotoCapture {
            taker: self.clone(),
            delegate: delegate.clone(),
        };
        self.0.session.queue().exec_async(move || capture.capture());
        Box::pin(async move {
            let _answering = delegate;
            answered.await
        })
    }
}

/// One capture, handed to the session's queue.
struct CrossThreadPhotoCapture {
    taker: PhotoTaker,
    delegate: Retained<PhotoDelegate>,
}

// SAFETY: the delegate's one state is behind a mutex, and the output is used on the session's
// queue; the binding cannot say so.
unsafe impl Send for CrossThreadPhotoCapture {}

impl CrossThreadPhotoCapture {
    fn capture(self) {
        // SAFETY: AVFoundation's constants, which it exports for the life of the process.
        let (Some(codec_key), Some(jpeg)) =
            (unsafe { AVVideoCodecKey }, unsafe { AVVideoCodecTypeJPEG })
        else {
            self.delegate.answer(Err(MediaError::Failed(
                "this system has no JPEG photos".to_owned(),
            )));
            return;
        };
        let format =
            NSDictionary::<NSString, AnyObject>::from_slices(&[codec_key], &[jpeg.as_ref()]);
        // SAFETY: settings for one JPEG photo at the output's size, captured on the session's queue
        // with a delegate the answer's future keeps.
        unsafe {
            let settings = AVCapturePhotoSettings::photoSettingsWithFormat(Some(&format));
            if let Some(dimensions) = self.taker.0.dimensions.filter(|_| takes_photo_sizes()) {
                settings.setMaxPhotoDimensions(dimensions);
            }
            self.taker.0.output.0.capturePhotoWithSettings_delegate(
                &settings,
                ProtocolObject::from_ref(&*self.delegate),
            );
        }
    }
}

define_class!(
    /// Answers one photo.
    //
    // SAFETY: NSObject has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "WaftPhotoCaptureDelegate"]
    #[ivars = std::sync::Mutex<Option<MediaAnswer<Photo>>>]
    struct PhotoDelegate;

    unsafe impl NSObjectProtocol for PhotoDelegate {}

    unsafe impl AVCapturePhotoCaptureDelegate for PhotoDelegate {
        #[unsafe(method(captureOutput:didFinishProcessingPhoto:error:))]
        fn finished(&self, _output: &AVCapturePhotoOutput, photo: &AVCapturePhoto, error: Option<&NSError>) {
            abort_on_panic(|| self.answer(photo_of(photo, error)));
        }
    }
);

impl PhotoDelegate {
    fn new(answer: MediaAnswer<Photo>) -> Retained<PhotoDelegate> {
        let this = PhotoDelegate::alloc().set_ivars(std::sync::Mutex::new(Some(answer)));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    fn answer(&self, photo: Result<Photo, MediaError>) {
        let answer = self
            .ivars()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(answer) = answer {
            answer.give(photo);
        }
    }
}

/// Whether the system takes photos at a size asked for: macOS 13 and iOS 16 on, which list a
/// format's photo sizes.
fn takes_photo_sizes() -> bool {
    AVCapturePhotoSettings::class().responds_to(sel!(setMaxPhotoDimensions:))
}

/// The photo the system finished, or why it did not.
fn photo_of(photo: &AVCapturePhoto, error: Option<&NSError>) -> Result<Photo, MediaError> {
    if let Some(error) = error {
        return Err(MediaError::Failed(error.localizedDescription().to_string()));
    }
    // SAFETY: the photo is live for the delegate's call.
    let data = unsafe { photo.fileDataRepresentation() }
        .ok_or_else(|| MediaError::Failed("the photo has no data".to_owned()))?;
    Ok(Photo {
        bytes: data.to_vec(),
        mime_type: "image/jpeg".to_owned(),
    })
}

/// A session's recordings, one at a time. Every clone is the same output.
#[derive(Clone)]
pub(super) struct MovieRecorder(Arc<MovieRecorderShared>);

struct MovieRecorderShared {
    session: Arc<SessionShared>,
    output: CrossThreadMovieOutput,
    /// The delegate of the recording under way, kept so that the recording answers even where
    /// the app let go of its future.
    recording: std::sync::Mutex<Option<KeptDelegate>>,
}

impl MovieRecorder {
    pub(super) fn new(session: Arc<SessionShared>) -> MovieRecorder {
        // SAFETY: a new movie file output, which the session takes on its queue.
        let output = unsafe { AVCaptureMovieFileOutput::new() };
        MovieRecorder(Arc::new(MovieRecorderShared {
            session,
            output: CrossThreadMovieOutput(output),
            recording: std::sync::Mutex::new(None),
        }))
    }

    pub(super) fn output(&self) -> &AVCaptureOutput {
        &self.0.output.0
    }

    /// Records to a new file at `path` while the session runs, until [`stop`](Self::stop);
    /// answers once the file is written whole, or why it was not.
    pub(super) fn start(&self, path: PathBuf) -> RecordingFuture {
        if self.0.session.state() != CaptureState::Running {
            return Box::pin(std::future::ready(Err(MediaError::NotRunning)));
        }
        let (answer, answered) = media_answer();
        let delegate = RecordingDelegate::new(answer);
        *self
            .0
            .recording
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(KeptDelegate {
            _delegate: delegate.clone(),
        });
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let start = CrossThreadRecordingStart {
            recorder: self.clone(),
            url,
            delegate: delegate.clone(),
        };
        self.0.session.queue().exec_async(move || start.start());
        Box::pin(async move {
            let _answering = delegate;
            answered.await
        })
    }

    /// Ends the recording, whose future then answers.
    pub(super) fn stop(&self) {
        let recorder = self.clone();
        self.0.session.queue().exec_async(move || {
            // SAFETY: stopping the recording on the session's queue.
            unsafe { recorder.0.output.0.stopRecording() };
        });
    }
}

/// A recording's start, handed to the session's queue.
struct CrossThreadRecordingStart {
    recorder: MovieRecorder,
    url: Retained<NSURL>,
    delegate: Retained<RecordingDelegate>,
}

// SAFETY: the delegate's one state is behind a mutex, the URL is immutable, and the output is used
// on the session's queue; the binding cannot say so.
unsafe impl Send for CrossThreadRecordingStart {}

impl CrossThreadRecordingStart {
    fn start(self) {
        // SAFETY: recording to a new file on the session's queue, with a delegate the recorder
        // and the answer's future keep.
        unsafe {
            self.recorder
                .0
                .output
                .0
                .startRecordingToOutputFileURL_recordingDelegate(
                    &self.url,
                    ProtocolObject::from_ref(&*self.delegate),
                );
        }
    }
}

define_class!(
    /// Answers one recording once its file is written.
    //
    // SAFETY: NSObject has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "WaftMovieRecordingDelegate"]
    #[ivars = std::sync::Mutex<Option<MediaAnswer<()>>>]
    struct RecordingDelegate;

    unsafe impl NSObjectProtocol for RecordingDelegate {}

    unsafe impl AVCaptureFileOutputRecordingDelegate for RecordingDelegate {
        #[unsafe(method(captureOutput:didFinishRecordingToOutputFileAtURL:fromConnections:error:))]
        fn finished(
            &self,
            _output: &AVCaptureFileOutput,
            _url: &NSURL,
            _connections: &NSArray<AVCaptureConnection>,
            error: Option<&NSError>,
        ) {
            abort_on_panic(|| self.answer(recording_of(error)));
        }
    }
);

impl RecordingDelegate {
    fn new(answer: MediaAnswer<()>) -> Retained<RecordingDelegate> {
        let this = RecordingDelegate::alloc().set_ivars(std::sync::Mutex::new(Some(answer)));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    fn answer(&self, recorded: Result<(), MediaError>) {
        let answer = self
            .ivars()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(answer) = answer {
            answer.give(recorded);
        }
    }
}

/// Whether a recording that ended with `error` wrote its file whole: the system reports an error
/// for a recording it ended itself, such as at a size limit, whose file is still whole.
fn recording_of(error: Option<&NSError>) -> Result<(), MediaError> {
    let Some(error) = error else {
        return Ok(());
    };
    // SAFETY: AVFoundation's key, and the error's user info, live for the call.
    let finished = unsafe { AVErrorRecordingSuccessfullyFinishedKey }
        .and_then(|key| error.userInfo().objectForKey(key))
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .is_some_and(|value| value.as_bool());
    if finished {
        Ok(())
    } else {
        Err(MediaError::Failed(error.localizedDescription().to_string()))
    }
}

struct CrossThreadPhotoOutput(Retained<AVCapturePhotoOutput>);

// SAFETY: the output is configured and asked for photos on the session's queue alone.
unsafe impl Send for CrossThreadPhotoOutput {}
unsafe impl Sync for CrossThreadPhotoOutput {}

struct CrossThreadMovieOutput(Retained<AVCaptureMovieFileOutput>);

// SAFETY: the output is configured, started and stopped on the session's queue alone.
unsafe impl Send for CrossThreadMovieOutput {}
unsafe impl Sync for CrossThreadMovieOutput {}

/// A recording's delegate, kept by its recorder until the next recording.
struct KeptDelegate {
    _delegate: Retained<RecordingDelegate>,
}

// SAFETY: the delegate's one state is behind a mutex; the binding cannot say so.
unsafe impl Send for KeptDelegate {}
unsafe impl Sync for KeptDelegate {}

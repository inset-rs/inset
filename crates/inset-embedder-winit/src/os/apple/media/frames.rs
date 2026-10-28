//! Frame readers: a player's newest picture from an `AVPlayerItemVideoOutput`, and a camera's from
//! an `AVCaptureVideoDataOutput`'s sample buffers, each a `CVPixelBuffer` backed by an `IOSurface`
//! in 32-bit BGRA, which the system converts the video into on the GPU, so the app's GPU reads it
//! with no copy.

use std::ptr::NonNull;
use std::sync::{Mutex, PoisonError};

use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use inset_embedder::{FrameReader, PixelBuffer};
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
use objc2_quartz_core::CACurrentMediaTime;

use super::abort_on_panic;

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

impl FrameReader for PlayerFrames {
    fn newest(&self) -> Option<PixelBuffer> {
        let output = &self.output.0;
        // SAFETY: the output answers from any thread, as a display link's thread reads it.
        unsafe {
            let time = output.itemTimeForHostTime(CACurrentMediaTime());
            if !output.hasNewPixelBufferForItemTime(time) {
                return None;
            }
            output
                .copyPixelBufferForItemTime_itemTimeForDisplay(time, std::ptr::null_mut())
                .map(pixel_buffer_of)
        }
    }
}

/// An item's video output handed between threads.
struct CrossThreadOutput(Retained<AVPlayerItemVideoOutput>);

// SAFETY: `AVPlayerItemVideoOutput` is read from a display link's thread as a matter of course;
// AVFoundation guards its own state, and the binding cannot say so.
unsafe impl Send for CrossThreadOutput {}
unsafe impl Sync for CrossThreadOutput {}

/// A camera's frames: the newest picture its video data output delivered, kept until read.
pub(super) struct CameraFrames {
    newest: std::sync::Arc<Mutex<Option<CrossThreadPixelBuffer>>>,
    output: CrossThreadVideoDataOutput,
    /// The delegate the output calls, which it holds weakly.
    _delegate: CrossThreadDelegate,
    /// The queue the delegate is called on, apart from the session's, which a start blocks.
    _queue: DispatchRetained<DispatchQueue>,
}

impl CameraFrames {
    /// An output delivering pixel buffers on a queue of its own, kept here as they come.
    pub(super) fn new() -> CameraFrames {
        let newest = std::sync::Arc::new(Mutex::new(None));
        let delegate = SampleBufferDelegate::alloc().set_ivars(std::sync::Arc::clone(&newest));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let delegate: Retained<SampleBufferDelegate> = unsafe { msg_send![super(delegate), init] };
        let queue = DispatchQueue::new("rs.inset.camera-frames", DispatchQueueAttr::SERIAL);
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
}

impl FrameReader for CameraFrames {
    fn newest(&self) -> Option<PixelBuffer> {
        let newest = self
            .newest
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()?;
        Some(newest.0)
    }
}

define_class!(
    /// Keeps the newest picture a video data output delivers.
    //
    // SAFETY: NSObject has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "InsetCameraFramesDelegate"]
    #[ivars = std::sync::Arc<Mutex<Option<CrossThreadPixelBuffer>>>]
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
                *self.ivars().lock().unwrap_or_else(PoisonError::into_inner) =
                    Some(CrossThreadPixelBuffer(image));
            });
        }
    }
);

/// What a frame reader of nothing answers: a player that failed has no pictures.
pub(super) struct NoFrames;

impl FrameReader for NoFrames {
    fn newest(&self) -> Option<PixelBuffer> {
        None
    }
}

/// The attributes every frame reader asks its pictures in: 32-bit BGRA, backed by an
/// `IOSurface` a Metal texture can be made of.
fn pixel_buffer_attributes() -> Retained<NSDictionary<NSString, AnyObject>> {
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

/// A CF string as the NSString it is toll-free bridged to.
fn as_ns_string(string: &objc2_core_foundation::CFString) -> &NSString {
    let pointer: *const objc2_core_foundation::CFString = string;
    // SAFETY: a CFString is an NSString.
    unsafe { &*pointer.cast::<NSString>() }
}

/// A pixel buffer AVFoundation answered retained, as the CF type the app is handed.
fn pixel_buffer_of(buffer: Retained<CVPixelBuffer>) -> PixelBuffer {
    let pointer = NonNull::new(Retained::into_raw(buffer)).expect("a retained buffer");
    // SAFETY: the pointer carries the +1 retain `Retained` gave up.
    unsafe { CFRetained::from_raw(pointer) }
}

/// A camera's picture, handed from the output's queue to the reader's thread.
pub(super) struct CrossThreadPixelBuffer(PixelBuffer);

// SAFETY: a pixel buffer is made to be shared between threads: its retain count is atomic, and
// nothing here writes its pixels. The binding cannot say so.
unsafe impl Send for CrossThreadPixelBuffer {}

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

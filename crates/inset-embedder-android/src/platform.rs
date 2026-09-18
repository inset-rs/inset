//! The host capability the framework holds: [`Platform`] and what it hands out.
//!
//! A request from the framework only records what was asked for and wakes the activity's
//! looper; the loop serves it on its next pass, since the window, the keyboard and the
//! surface all belong to that thread.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use android_activity::AndroidAppWaker;
use inset_embedder::{
    Brightness, Dispatcher, FontSource, ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame,
    ImageFrameFuture, ImageRepetition, Locale, Platform, SystemChrome, SystemFontSource,
    TargetPlatform, ViewRef,
};

use crate::chrome::AndroidChrome;
use crate::view::{AndroidView, VIEW};

/// The long-lived host capability held by the application.
pub struct AndroidPlatform {
    pub(crate) view: Rc<AndroidView>,
    pub(crate) frame_requested: Cell<bool>,
    /// When the framework asked to be woken, shared with whatever thread asked.
    pub(crate) wakes: Arc<Wakes>,
    origin: Instant,
    pub(crate) brightness: Cell<Brightness>,
    pub(crate) locales: RefCell<Vec<Locale>>,
    pub(crate) image_loader: RefCell<Option<valo_codec::ImageLoader>>,
    pub(crate) images: RefCell<Option<valo::ImageContext>>,
    chrome: AndroidChrome,
}

impl AndroidPlatform {
    pub(crate) fn new(
        view: Rc<AndroidView>,
        app: &android_activity::AndroidApp,
    ) -> AndroidPlatform {
        let waker = app.create_waker();
        AndroidPlatform {
            view,
            frame_requested: Cell::new(false),
            wakes: Arc::new(Wakes {
                timer_wakeup: Mutex::new(None),
                wake_now_requested: AtomicBool::new(false),
                waker,
            }),
            origin: Instant::now(),
            brightness: Cell::new(Brightness::Light),
            locales: RefCell::new(Vec::new()),
            image_loader: RefCell::new(None),
            images: RefCell::new(None),
            chrome: AndroidChrome::new(app.clone()),
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(self.origin)
    }

    /// The renderer's image store, on the device the window draws with: an image made
    /// through it is one `RawImage` shows.
    pub fn image_context(&self) -> Option<valo::ImageContext> {
        self.images.borrow().clone()
    }
}

/// The two reasons the loop may have to wake the framework, and the looper to wake so it
/// does.
///
/// These are two separate things. The timer wakeup waits for its own time to arrive. The
/// wake-now request is cleared by the next wake. Holding both in one field would let a
/// ready task overwrite a timer that has not run yet.
pub(crate) struct Wakes {
    /// When to wake the framework for its earliest waiting timer, as it last set it.
    timer_wakeup: Mutex<Option<Instant>>,
    /// A ready task asking for the framework to be woken.
    wake_now_requested: AtomicBool,
    waker: AndroidAppWaker,
}

impl Wakes {
    /// Takes the framework's word for when to wake it next for a timer. The looper is
    /// woken only when that is sooner than what the loop is already waiting for; a later
    /// time is picked up when the wait it is in now ends.
    fn set_timer_wakeup(&self, wanted: Option<Instant>) {
        let mut wakeup = self.timer_wakeup.lock().expect("wake lock");
        let sooner = match (wanted, *wakeup) {
            (Some(wanted), Some(held)) => wanted < held,
            (wanted, _) => wanted.is_some(),
        };
        *wakeup = wanted;
        drop(wakeup);
        if sooner {
            self.waker.wake();
        }
    }

    /// Asks for the framework to be woken as soon as the loop can.
    fn request_wake_now(&self) {
        self.wake_now_requested.store(true, Ordering::Release);
        self.waker.wake();
    }

    /// When the loop must next be awake for a timer, if there is such a time.
    pub(crate) fn timer_wakeup(&self) -> Option<Instant> {
        *self.timer_wakeup.lock().expect("wake lock")
    }

    /// Whether a ready task is waiting to be woken, without clearing the request.
    pub(crate) fn wake_now_requested(&self) -> bool {
        self.wake_now_requested.load(Ordering::Acquire)
    }

    /// Whether the framework should be woken now: a ready task asked for it, the timer
    /// wakeup has arrived, or both. A timer wakeup that has arrived is cleared here, and
    /// the framework sets the next one before the wake ends.
    pub(crate) fn should_wake(&self, now: Instant) -> bool {
        let asked = self.wake_now_requested.swap(false, Ordering::AcqRel);
        let mut wakeup = self.timer_wakeup.lock().expect("wake lock");
        let timer_came = wakeup.is_some_and(|wakeup| wakeup <= now);
        if timer_came {
            *wakeup = None;
        }
        asked || timer_came
    }
}

impl Platform for AndroidPlatform {
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::Android
    }

    fn request_frame(&self) {
        if !self.frame_requested.replace(true) {
            self.wakes.waker.wake();
        }
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(AndroidDispatcher {
            wakes: Arc::clone(&self.wakes),
        })
    }

    fn views(&self) -> Vec<ViewRef> {
        vec![Rc::clone(&self.view) as ViewRef]
    }

    fn view(&self, id: inset_embedder::ViewId) -> Option<ViewRef> {
        (id == VIEW).then(|| Rc::clone(&self.view) as ViewRef)
    }

    /// The activity has one window, and it is the implicit view.
    fn implicit_view(&self) -> Option<ViewRef> {
        Some(Rc::clone(&self.view) as ViewRef)
    }

    fn system_chrome(&self) -> Option<&dyn SystemChrome> {
        Some(&self.chrome)
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    fn locales(&self) -> Vec<Locale> {
        self.locales.borrow().clone()
    }

    /// The platform's fonts, as its own configuration lists them.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::platform()))
    }

    fn open_image_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        let loader = self.image_loader.borrow().clone();
        Box::pin(async move {
            if bytes.is_empty() {
                return Err(ImageDecodeError::Empty);
            }
            let loader = loader.ok_or(ImageDecodeError::NoDecoder)?;
            let codec = loader
                .open(
                    bytes,
                    valo_codec::DecodeOptions {
                        // Pictures are routinely drawn smaller than they decode, and mip
                        // levels are the difference between a smooth downscale and a
                        // shimmering one.
                        mipmaps: true,
                        ..Default::default()
                    },
                )
                .await
                .map_err(as_decode_error)?;
            Ok(Box::new(AndroidImageCodec { codec }) as Box<dyn ImageCodec>)
        })
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<inset_embedder::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
    }
}

/// The host's threads as any thread reaches them: a wake is a time the loop keeps and a
/// wake of the activity's looper; work goes to a thread of its own.
struct AndroidDispatcher {
    wakes: Arc<Wakes>,
}

impl Dispatcher for AndroidDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        self.wakes.set_timer_wakeup(timer_wakeup);
    }

    fn wake_now(&self) {
        self.wakes.request_wake_now();
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        std::thread::Builder::new()
            .name("inset-worker".into())
            .spawn(work)
            .expect("a thread for work off the main thread");
    }
}

/// Adapts valo's drawable frames to the framework's codec contract.
struct AndroidImageCodec {
    codec: valo_codec::Codec,
}

impl ImageCodec for AndroidImageCodec {
    fn frame_count(&self) -> u32 {
        self.codec.info().frame_count
    }

    fn repetition(&self) -> ImageRepetition {
        match self.codec.info().repetition {
            valo_codec::Repetition::Once => ImageRepetition::Once,
            valo_codec::Repetition::Times(times) => ImageRepetition::Times(times),
            valo_codec::Repetition::Forever => ImageRepetition::Forever,
        }
    }

    fn next_frame(&mut self) -> ImageFrameFuture<'_> {
        let frame = self.codec.next_frame();
        Box::pin(async move {
            let frame = frame.await.map_err(as_decode_error)?;
            Ok(ImageFrame {
                image: frame.image,
                duration: frame.duration,
            })
        })
    }
}

fn as_decode_error(error: valo_codec::DecodeError) -> ImageDecodeError {
    match error {
        valo_codec::DecodeError::NoDecoder => ImageDecodeError::NoDecoder,
        valo_codec::DecodeError::Unsupported(_) => ImageDecodeError::UnknownFormat,
        valo_codec::DecodeError::InvalidData(reason) => ImageDecodeError::Damaged(reason),
        other => ImageDecodeError::Failed(other.to_string()),
    }
}

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
use waft_embedder::{
    Brightness, Clipboard, Compositor, Dispatcher, FileDialogs, FontSource, HardwareBuffer,
    HardwareBufferAllocator, ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame,
    ImageFrameFuture, ImageRepetition, Locale, Notifications, Offset, Opener, Platform, Screen,
    Screens, SystemChrome, SystemFontSource, TargetPlatform, ViewRef,
};

use crate::chrome::AndroidChrome;
use crate::clipboard::AndroidClipboard;
use crate::composition::SurfaceControlCompositor;
use crate::file_dialogs::DocumentPickers;
use crate::gpu::Gpu;
use crate::hardware_buffers::GrallocAllocator;
use crate::notifications::AndroidNotifications;
use crate::opener::IntentOpener;
use crate::view::{AndroidView, VIEW};
use crate::vsync_waiter::{ChoreographerWaiter, Waits};

/// The long-lived host capability held by the application.
pub struct AndroidPlatform {
    pub(crate) view: Rc<AndroidView>,
    pub(crate) frame_requested: Cell<bool>,
    /// When the framework asked to be woken, shared with whatever thread asked.
    pub(crate) wakes: Arc<Wakes>,
    origin: Instant,
    pub(crate) brightness: Cell<Brightness>,
    pub(crate) locales: RefCell<Vec<Locale>>,
    /// The screen the activity is on, as the app was last told it; `None` until the system first
    /// describes it.
    pub(crate) screen: RefCell<Option<Screen>>,
    /// The id of that screen, for the waits on its refreshes asked from any thread.
    pub(crate) screen_id: Arc<Mutex<Option<waft_embedder::ScreenId>>>,
    /// The waits on the display's refreshes not yet served (`vsync_waiter.rs`).
    pub(crate) vsync_waits: Arc<Waits>,
    /// SurfaceFlinger's surface controls, where Waft's device imports hardware buffers
    /// (`composition/`).
    compositor: Option<Arc<SurfaceControlCompositor>>,
    /// The system's allocator of the buffers the compositor shows (`hardware_buffers.rs`).
    hardware_buffer_allocator: Arc<GrallocAllocator>,
    pub(crate) image_loader: RefCell<Option<valo_codec::ImageLoader>>,
    pub(crate) images: RefCell<Option<valo::ImageContext>>,
    chrome: AndroidChrome,
    pub(crate) clipboard: AndroidClipboard,
    /// The document pickers, where the activity is Waft's.
    file_dialogs: Option<DocumentPickers>,
    opener: IntentOpener,
    /// The system's notifications, where the activity is Waft's.
    notifications: Option<AndroidNotifications>,
}

impl AndroidPlatform {
    /// The platform of `app`, whose one view is `view`, drawn with `gpu`.
    pub(crate) fn new(
        view: Rc<AndroidView>,
        app: &android_activity::AndroidApp,
        gpu: &Gpu,
        compositor: Option<Arc<SurfaceControlCompositor>>,
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
            screen: RefCell::new(None),
            screen_id: Arc::new(Mutex::new(None)),
            vsync_waits: Waits::new(app.create_waker()),
            compositor,
            hardware_buffer_allocator: Arc::new(GrallocAllocator {
                largest_side: gpu.device.limits().max_texture_dimension_2d,
            }),
            image_loader: RefCell::new(None),
            images: RefCell::new(None),
            chrome: AndroidChrome::new(app.clone()),
            clipboard: AndroidClipboard::new(app.clone()),
            file_dialogs: DocumentPickers::new(app.clone()),
            opener: IntentOpener::new(app.clone()),
            notifications: AndroidNotifications::new(app.clone()),
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

    /// The activity's window is the one view: a frame of any other is no frame of the app's.
    fn request_frame(&self, view: waft_embedder::ViewId) {
        if view == VIEW && !self.frame_requested.replace(true) {
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

    /// The activity's window, the one view the system gives the app.
    fn views(&self) -> Vec<ViewRef> {
        vec![Rc::clone(&self.view) as ViewRef]
    }

    fn view(&self, id: waft_embedder::ViewId) -> Option<ViewRef> {
        (id == VIEW).then(|| Rc::clone(&self.view) as ViewRef)
    }

    /// The system's clipboard, as far as an app without Java code reaches it (`clipboard.rs`).
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&self.clipboard)
    }

    /// The document pickers, under Waft's activity (`file_dialogs.rs`).
    fn file_dialogs(&self) -> Option<&dyn FileDialogs> {
        self.file_dialogs
            .as_ref()
            .map(|pickers| pickers as &dyn FileDialogs)
    }

    /// Intents to view (`opener.rs`).
    fn opener(&self) -> Option<&dyn Opener> {
        Some(&self.opener)
    }

    /// The notification manager, through Waft's notification receiver (`notifications/`).
    fn notifications(&self) -> Option<&dyn Notifications> {
        self.notifications
            .as_ref()
            .map(|notifications| notifications as &dyn Notifications)
    }

    fn system_chrome(&self) -> Option<&dyn SystemChrome> {
        Some(&self.chrome)
    }

    /// The display's refreshes, which the loop serves from the choreographer the framework's frames
    /// are drawn at (`vsync_waiter.rs`).
    fn vsync_waiter(&self) -> Option<Arc<dyn waft_embedder::VsyncWaiter>> {
        Some(Arc::new(ChoreographerWaiter {
            waits: Arc::clone(&self.vsync_waits),
            screen: Arc::clone(&self.screen_id),
        }))
    }

    /// The one screen the activity is on (`display.rs`).
    fn screens(&self) -> Option<&dyn Screens> {
        Some(self)
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

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<waft_embedder::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
    }

    /// The buffer's texture on the device every view draws with, imported by valo; none where
    /// that device imports no hardware buffers.
    fn import_hardware_buffer(&self, buffer: &dyn HardwareBuffer) -> Option<waft_embedder::Image> {
        let images = self.image_context()?;
        let texture = crate::gpu_interop::import_texture(buffer, images.device())?;
        images.import_texture(texture, false).ok()
    }

    /// A `VkImage` over the buffer's memory, on a device opened to import hardware buffers
    /// (`gpu_interop.rs`): Waft's own, where Vulkan can, and none an app opened itself.
    fn import_texture(
        &self,
        buffer: &dyn HardwareBuffer,
        device: &wgpu::Device,
    ) -> Option<wgpu::Texture> {
        crate::gpu_interop::import_texture(buffer, device)
    }

    /// Surface controls beneath the view's picture (`composition/`), where Waft's device imports
    /// hardware buffers; none on OpenGL ES, whose view draws through a swapchain.
    fn compositor(&self) -> Option<Arc<dyn Compositor>> {
        self.compositor
            .as_ref()
            .map(|compositor| Arc::clone(compositor) as Arc<dyn Compositor>)
    }

    /// The allocator of the system's `AHardwareBuffer`s (`hardware_buffers.rs`).
    fn hardware_buffer_allocator(&self) -> Option<Arc<dyn HardwareBufferAllocator>> {
        Some(Arc::clone(&self.hardware_buffer_allocator) as Arc<dyn HardwareBufferAllocator>)
    }
}

/// A phone's one screen, which a finger points at only while it touches it.
impl Screens for AndroidPlatform {
    fn screens(&self) -> Vec<Screen> {
        self.screen.borrow().iter().cloned().collect()
    }

    fn pointer_location(&self) -> Option<Offset> {
        None
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
            .name("waft-worker".into())
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

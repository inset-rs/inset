//! The host capability the framework holds: [`Platform`] and what it hands out.
//!
//! A request from the framework only records what was asked for and wakes the app's loop; the
//! loop serves it on its next pass, since the windows, the keyboard and the surfaces all belong
//! to that thread.
//!
//! The platform is the app's, not an activity's: its views are the registry's (`views.rs`), each
//! an activity's window, and what it hands out goes through the application wherever any context
//! answers. What one view asks of the system, the bars' style, its task's card and its focus, is
//! that view's host's (`view.rs`, `task_window.rs`); a file dialog starts from its parent's
//! activity. Where Waft's interface names no view, opening a link starts from the application, in
//! a task of its own, and what only an activity can show, the notification permission's question
//! and a file dialog with no parent, goes to the activity in front (`Views::activity_in_front`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use waft_embedder::{
    Brightness, Clipboard, Compositor, Dispatcher, FileDialogs, FontSource, HardwareBuffer,
    HardwareBufferAllocator, ImageCodec, ImageCodecFuture, ImageDecodeError, ImageFrame,
    ImageFrameFuture, ImageRepetition, Locale, Notifications, Offset, Opener, Platform, RequestId,
    Screen, Screens, SystemChrome, SystemFontSource, TargetPlatform, ViewError, ViewRef,
    WindowConfig,
};

use crate::activity::Activity;
use crate::application::Application;
use crate::chrome::{AndroidChrome, AppDescription};
use crate::clipboard::AndroidClipboard;
use crate::composition::SurfaceControlCompositor;
use crate::file_dialogs::DocumentPickers;
use crate::gpu::Gpu;
use crate::hardware_buffers::GrallocAllocator;
use crate::image_codecs::AndroidImageCodecs;
use crate::loop_waker::LoopWaker;
use crate::media::{NdkCameras, NdkFrameReaders};
use crate::notifications::AndroidNotifications;
use crate::opener::IntentOpener;
use crate::view_requests::ViewRequests;
use crate::views::Views;
use crate::vsync_waiter::{ChoreographerWaiter, Waits};

/// The long-lived host capability held by the application.
pub struct AndroidPlatform {
    /// The app's views, and the frames they asked for (`views.rs`).
    views: Rc<Views>,
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
    /// The decoders of this build, decoding into `images`; none where they could not start.
    image_loader: Option<valo_codec::ImageLoader>,
    /// The renderer's image store, on the device every view draws with.
    images: valo::ImageContext,
    /// Android's own image decoder and encoder, for the app's own use (`image_codecs/`).
    image_codecs: AndroidImageCodecs,
    chrome: AndroidChrome,
    /// What every view's task shows where its window says nothing (`chrome.rs`).
    pub(crate) app_description: AppDescription,
    /// The views asked for whose activities have not arrived (`view_requests.rs`).
    pub(crate) view_requests: ViewRequests,
    pub(crate) clipboard: AndroidClipboard,
    /// The document pickers, where the activity is Waft's.
    file_dialogs: Option<DocumentPickers>,
    opener: IntentOpener,
    /// The system's notifications, where the activity is Waft's.
    notifications: Option<AndroidNotifications>,
    /// The NDK's cameras, where the system has a camera manager (`media/`).
    cameras: Option<NdkCameras>,
    /// The readers a camera's frames go to (`media/`).
    video_frame_readers: NdkFrameReaders,
}

impl AndroidPlatform {
    /// The platform of the app `application` is, whose views are `views`, drawn with `gpu`;
    /// `waker` wakes the loop that serves it, and `first`, the app's first activity, tells whether
    /// the app's activities are Waft's.
    pub(crate) fn new(
        views: Rc<Views>,
        application: &Application,
        first: &Activity,
        waker: LoopWaker,
        gpu: &Gpu,
        compositor: Option<Arc<SurfaceControlCompositor>>,
    ) -> AndroidPlatform {
        let images = valo::ImageContext::new(gpu.device.clone(), gpu.queue.clone());
        let app_description = AppDescription::default();
        AndroidPlatform {
            chrome: AndroidChrome::new(Rc::clone(&views), app_description.clone()),
            app_description,
            view_requests: ViewRequests::new(application.clone()),
            file_dialogs: DocumentPickers::new(application.clone(), Rc::clone(&views), first),
            notifications: AndroidNotifications::new(application.clone(), Rc::clone(&views), first),
            cameras: NdkCameras::new(application.clone(), Rc::clone(&views)),
            video_frame_readers: NdkFrameReaders,
            views,
            wakes: Arc::new(Wakes {
                timer_wakeup: Mutex::new(None),
                wake_now_requested: AtomicBool::new(false),
                waker: waker.clone(),
            }),
            origin: Instant::now(),
            brightness: Cell::new(Brightness::Light),
            locales: RefCell::new(Vec::new()),
            screen: RefCell::new(None),
            screen_id: Arc::new(Mutex::new(None)),
            vsync_waits: Waits::new(waker),
            compositor,
            hardware_buffer_allocator: Arc::new(GrallocAllocator {
                largest_side: gpu.device.limits().max_texture_dimension_2d,
            }),
            image_loader: crate::images::loader(images.clone()),
            images,
            image_codecs: AndroidImageCodecs::new(application.clone()),
            clipboard: AndroidClipboard::new(application.clone()),
            opener: IntentOpener::new(application.clone()),
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.elapsed_at(Instant::now())
    }

    /// The platform's clock at `instant`, since its origin.
    pub(crate) fn elapsed_at(&self, instant: Instant) -> Duration {
        instant.saturating_duration_since(self.origin)
    }

    /// The renderer's image store, on the device every view draws with: an image made
    /// through it is one `RawImage` shows.
    pub fn image_context(&self) -> Option<valo::ImageContext> {
        Some(self.images.clone())
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
    waker: LoopWaker,
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

    /// The request is the registry's to hold until the view has a window (`views.rs`); the loop
    /// is woken for one that is news. A frame of a view that is not the app's is no frame.
    fn request_frame(&self, view: waft_embedder::ViewId) {
        if self.views.request_frame(view) {
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

    /// The activities' windows, the views the system gives the app.
    fn views(&self) -> Vec<ViewRef> {
        let views = self.views.all();
        views.into_iter().map(|view| view as ViewRef).collect()
    }

    fn view(&self, id: waft_embedder::ViewId) -> Option<ViewRef> {
        self.views.get(id).map(|view| view as ViewRef)
    }

    /// A task of its own, Waft's document activity's (`view_requests.rs`), titled and iconed as
    /// `config` says; the rest of `config` is the system's to decide on a phone. It fails at once
    /// while no view of the app has a window, as Android starts no activity for an app in the
    /// background.
    fn request_view(
        &self,
        config: WindowConfig,
        initial_data: Option<String>,
    ) -> Result<RequestId, ViewError> {
        self.view_requests
            .request(config, initial_data, self.views.any_has_window())
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

    /// The NDK's cameras (`media/`).
    fn cameras(&self) -> Option<&dyn waft_embedder::Cameras> {
        self.cameras
            .as_ref()
            .map(|cameras| cameras as &dyn waft_embedder::Cameras)
    }

    /// The readers a camera's frames, drawn in RGBA, go to (`media/`).
    fn video_frame_readers(&self) -> Option<&dyn waft_embedder::VideoFrameReaders> {
        Some(&self.video_frame_readers)
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
        let loader = self.image_loader.clone();
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

    /// `AImageDecoder` and `AndroidBitmap_compress`, with no hardware codecs (`image_codecs/`).
    fn image_codecs(&self) -> Option<&dyn waft_embedder::image_codecs::ImageCodecs> {
        Some(&self.image_codecs)
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<waft_embedder::Image> {
        self.images.upload_pixels(pixels, false).ok()
    }

    /// The frame's buffer, imported as any hardware buffer is.
    fn import_video_frame(
        &self,
        frame: &dyn waft_embedder::VideoFrame,
    ) -> Option<waft_embedder::Image> {
        self.import_hardware_buffer(&*frame.hardware_buffer())
    }

    /// The buffer's texture on the device every view draws with, imported by valo; none where
    /// that device imports no hardware buffers.
    fn import_hardware_buffer(&self, buffer: &dyn HardwareBuffer) -> Option<waft_embedder::Image> {
        let texture = crate::gpu_interop::import_texture(buffer, self.images.device())?;
        self.images.import_texture(texture, false).ok()
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
/// wake of the app's loop; work goes to a thread of its own.
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

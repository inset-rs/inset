//! The host capability the framework holds: [`Platform`] and what it hands out.
//!
//! A request from the framework only records what was asked for and wakes the app's looper;
//! the app's loop serves it on its next pass (`app_loop`), since the views, their surfaces and
//! the framework all belong to the app's thread.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use android_activity::AndroidApp;
use inset_embedder::{
    Brightness, Clipboard, Dispatcher, FileDialogs, FontSource, ImageCodec, ImageCodecFuture,
    ImageDecodeError, ImageFrame, ImageFrameFuture, ImageRepetition, Locale, Notifications, Offset,
    Opener, Platform, RequestId, Screen, Screens, SystemChrome, SystemFontSource, TargetPlatform,
    ViewError, ViewId, ViewRef, ViewRequest,
};
use ndk::looper::ForeignLooper;

use crate::activity::Activities;
use crate::chrome::AndroidChrome;
use crate::clipboard::AndroidClipboard;
use crate::file_dialogs::DocumentPickers;
use crate::gpu::Gpu;
use crate::host_window::AndroidWindow;
use crate::notifications::{AndroidNotifications, NotificationChannelGroups};
use crate::opener::IntentOpener;
use crate::shortcuts::PinnedShortcuts;
use crate::task::{TaskDescriptions, ViewRequests};
use crate::view::AndroidView;

/// The long-lived host capability held by the application.
///
/// What only Android has is reached by downcasting the platform
/// (`platform.downcast_ref::<AndroidPlatform>()`): the app's [pinned
/// shortcuts](AndroidPlatform::pinned_shortcuts) and the [sections of its
/// notifications](AndroidPlatform::notification_channel_groups).
pub struct AndroidPlatform {
    /// The activities attached to the app, each a view.
    pub(crate) activities: Activities,
    /// Each view's window, its activity's task, by the view's id.
    windows: RefCell<BTreeMap<ViewId, Rc<AndroidWindow>>>,
    /// The views the app asked for that have not arrived.
    requests: ViewRequests,
    /// How Recents describes each view's task.
    descriptions: TaskDescriptions,
    /// The views that asked for a frame since their last one.
    frames_requested: RefCell<BTreeSet<ViewId>>,
    /// When the framework asked to be woken, shared with whatever thread asked.
    pub(crate) wakes: Arc<Wakes>,
    origin: Instant,
    pub(crate) brightness: Cell<Brightness>,
    pub(crate) locales: RefCell<Vec<Locale>>,
    /// The screen the activities are on, as the app was last told it; `None` until the system
    /// first describes it.
    pub(crate) screen: RefCell<Option<Screen>>,
    pub(crate) image_loader: RefCell<Option<valo_codec::ImageLoader>>,
    pub(crate) images: RefCell<Option<valo::ImageContext>>,
    chrome: AndroidChrome,
    pub(crate) clipboard: AndroidClipboard,
    /// The document pickers, where the activities are Inset's.
    file_dialogs: Option<DocumentPickers>,
    opener: IntentOpener,
    /// The system's notifications, where the activities are Inset's.
    notifications: Option<AndroidNotifications>,
    notification_channel_groups: NotificationChannelGroups,
    pinned_shortcuts: PinnedShortcuts,
}

impl AndroidPlatform {
    /// The platform of the app's thread, whose looper is `looper`, with no view yet.
    pub(crate) fn new(looper: ForeignLooper) -> AndroidPlatform {
        let activities = Activities::default();
        let descriptions = TaskDescriptions::new(activities.clone());
        let notifications = AndroidNotifications::new(activities.clone());
        AndroidPlatform {
            windows: RefCell::default(),
            requests: ViewRequests::new(),
            frames_requested: RefCell::default(),
            wakes: Arc::new(Wakes {
                timer_wakeup: Mutex::new(None),
                wake_now_requested: AtomicBool::new(false),
                looper,
            }),
            origin: Instant::now(),
            brightness: Cell::new(Brightness::Light),
            locales: RefCell::new(Vec::new()),
            screen: RefCell::new(None),
            image_loader: RefCell::new(None),
            images: RefCell::new(None),
            chrome: AndroidChrome::new(activities.clone(), descriptions.clone()),
            clipboard: AndroidClipboard::new(),
            file_dialogs: DocumentPickers::new(activities.clone()),
            opener: IntentOpener::new(activities.clone()),
            notification_channel_groups: NotificationChannelGroups::new(notifications.as_ref()),
            notifications,
            pinned_shortcuts: PinnedShortcuts::new(),
            activities,
            descriptions,
        }
    }

    /// The activity the system created, `activity`, attached as the view `view` of the app's
    /// from now on, in its task, carrying what it was started with: the request it answers, or
    /// the data of a task the platform gave.
    pub(crate) fn attach(&self, view: ViewId, activity: AndroidApp, gpu: &Gpu) -> Rc<AndroidView> {
        let launch = crate::task::launch_of(&activity);
        let answered = self.requests.answered_by(&launch);
        let request = answered.as_ref().map(|(request, _)| *request);
        let android_view = Rc::new(AndroidView::new(view, request, launch.initial_data, gpu));
        self.activities.attach(view, activity.clone());
        self.descriptions.add(view);
        let window = Rc::new(AndroidWindow::new(
            Rc::clone(&android_view),
            activity,
            self.descriptions.clone(),
        ));
        android_view.set_window(&window);
        if let Some((_, title)) = answered.filter(|(_, title)| !title.is_empty()) {
            self.descriptions.set_title(view, &title);
        }
        self.windows.borrow_mut().insert(view, window);
        android_view
    }

    /// The activity `view` is was destroyed: the view is gone from the app, with its task and
    /// whatever it asked for.
    pub(crate) fn detach(&self, view: ViewId) {
        self.activities.detach(view);
        self.descriptions.forget(view);
        self.frames_requested.borrow_mut().remove(&view);
        if let Some(window) = self.windows.borrow_mut().remove(&view) {
            window.android_view().activity_destroyed();
        }
    }

    pub(crate) fn android_view(&self, view: ViewId) -> Option<Rc<AndroidView>> {
        self.windows
            .borrow()
            .get(&view)
            .map(|window| Rc::clone(window.android_view()))
    }

    pub(crate) fn any_view_has_a_native_window(&self) -> bool {
        self.windows
            .borrow()
            .values()
            .any(|window| window.android_view().has_native_window())
    }

    /// Whether a view with a native window asked for a frame. One without waits for its window.
    pub(crate) fn frames_wanted(&self) -> bool {
        !self.frames_due().is_empty()
    }

    /// The views with a native window that asked for a frame, their requests answered by the
    /// frame about to be drawn; one without keeps its request.
    pub(crate) fn take_frame_requests(&self) -> Vec<ViewId> {
        let due = self.frames_due();
        let mut requested = self.frames_requested.borrow_mut();
        for view in &due {
            requested.remove(view);
        }
        due
    }

    /// The views with a native window that asked for a frame.
    fn frames_due(&self) -> Vec<ViewId> {
        let windows = self.windows.borrow();
        self.frames_requested
            .borrow()
            .iter()
            .copied()
            .filter(|view| {
                windows
                    .get(view)
                    .is_some_and(|window| window.android_view().has_native_window())
            })
            .collect()
    }

    pub(crate) fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(self.origin)
    }

    /// The sections of the app's notifications, each its own channel group with its own switch in
    /// the app's notification settings.
    pub fn notification_channel_groups(&self) -> &NotificationChannelGroups {
        &self.notification_channel_groups
    }

    /// The app's shortcuts on the home screen, each opening one view.
    pub fn pinned_shortcuts(&self) -> &PinnedShortcuts {
        &self.pinned_shortcuts
    }

    /// The renderer's image store, on the device the window draws with: an image made
    /// through it is one `RawImage` shows.
    pub fn image_context(&self) -> Option<valo::ImageContext> {
        self.images.borrow().clone()
    }
}

/// The two reasons the loop may have to wake the framework, and the app's looper to wake so it
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
    looper: ForeignLooper,
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
            self.looper.wake();
        }
    }

    /// Asks for the framework to be woken as soon as the loop can.
    fn request_wake_now(&self) {
        self.wake_now_requested.store(true, Ordering::Release);
        self.looper.wake();
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

    /// A view without a window keeps the request until its window arrives.
    fn request_frame(&self, view: ViewId) {
        if self.windows.borrow().contains_key(&view)
            && self.frames_requested.borrow_mut().insert(view)
        {
            self.wakes.looper.wake();
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

    /// The activities' windows, one a view, which the system gives the app.
    fn views(&self) -> Vec<ViewRef> {
        self.windows
            .borrow()
            .values()
            .map(|window| Rc::clone(window.android_view()) as ViewRef)
            .collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.android_view(id).map(|view| view as ViewRef)
    }

    /// A document task of its own, whose activity arrives as the view (`task.rs`).
    fn request_view(&self, request: ViewRequest) -> Result<RequestId, ViewError> {
        self.requests.request(request)
    }

    /// The system's clipboard, as far as an app without Java code reaches it (`clipboard.rs`).
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&self.clipboard)
    }

    /// The document pickers, under Inset's activity (`file_dialogs.rs`).
    fn file_dialogs(&self) -> Option<&dyn FileDialogs> {
        self.file_dialogs
            .as_ref()
            .map(|pickers| pickers as &dyn FileDialogs)
    }

    /// Intents to view (`opener.rs`).
    fn opener(&self) -> Option<&dyn Opener> {
        Some(&self.opener)
    }

    /// The notification manager, through Inset's notification receiver (`notifications/`).
    fn notifications(&self) -> Option<&dyn Notifications> {
        self.notifications
            .as_ref()
            .map(|notifications| notifications as &dyn Notifications)
    }

    fn system_chrome(&self) -> Option<&dyn SystemChrome> {
        Some(&self.chrome)
    }

    /// The one screen the activities are on (`display.rs`).
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

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<inset_embedder::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
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
/// wake of the app's looper; work goes to a thread of its own.
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

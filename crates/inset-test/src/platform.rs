//! A platform a test configures and reads back.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use inset_embedder::{
    Brightness, Clipboard, Dispatcher, FontSource, Haptics, ImageCodecFuture, ImageDecodeError,
    Instant, MouseCursor, Platform, PopupMenus, Restoration, SystemChrome, SystemContextMenu,
    TargetPlatform, TextServices, ViewId, ViewRef, WindowingOwner,
};

type FontSourceMaker = Rc<dyn Fn() -> Box<dyn FontSource>>;
type ImageCodecOpener = Rc<dyn Fn(Arc<[u8]>) -> ImageCodecFuture>;

/// A platform a test builds: which host it claims to be, the views it offers, and each
/// capability as the test's own implementation or absent. It counts the frames the framework
/// asks for and keeps the deadlines it asks to be woken at, so a test can assert on them; the
/// test itself pumps the app, since there is no loop to run it.
pub struct TestPlatform {
    target: Cell<TargetPlatform>,
    brightness: Cell<Brightness>,
    default_route: String,
    views: Vec<ViewRef>,
    frames_requested: Cell<u32>,
    dispatcher: Arc<TestDispatcher>,
    font_source: Option<FontSourceMaker>,
    image_codec: Option<ImageCodecOpener>,
    windowing: Option<Rc<dyn WindowingOwner>>,
    clipboard: Option<Rc<dyn Clipboard>>,
    system_chrome: Option<Rc<dyn SystemChrome>>,
    haptics: Option<Rc<dyn Haptics>>,
    system_context_menu: Option<Rc<dyn SystemContextMenu>>,
    popup_menus: Option<Rc<dyn PopupMenus>>,
    text_services: Option<Rc<dyn TextServices>>,
    mouse_cursor: Option<Rc<dyn MouseCursor>>,
    restoration: Option<Rc<dyn Restoration>>,
}

impl TestPlatform {
    /// A platform claiming to be Android, as Flutter's test binding does, in light mode, with
    /// no view and no capability.
    pub fn new() -> TestPlatform {
        TestPlatform {
            target: Cell::new(TargetPlatform::Android),
            brightness: Cell::new(Brightness::Light),
            default_route: "/".into(),
            views: Vec::new(),
            frames_requested: Cell::new(0),
            dispatcher: Arc::new(TestDispatcher::default()),
            font_source: None,
            image_codec: None,
            windowing: None,
            clipboard: None,
            system_chrome: None,
            haptics: None,
            system_context_menu: None,
            popup_menus: None,
            text_services: None,
            mouse_cursor: None,
            restoration: None,
        }
    }

    /// The same platform claiming to be `target`.
    pub fn on(self, target: TargetPlatform) -> TestPlatform {
        self.target.set(target);
        self
    }

    /// The same platform with `view` among its views; the first one added is the implicit
    /// view.
    pub fn with_view(mut self, view: ViewRef) -> TestPlatform {
        self.views.push(view);
        self
    }

    pub fn with_brightness(self, brightness: Brightness) -> TestPlatform {
        self.brightness.set(brightness);
        self
    }

    /// The route the host would open the app at; `/` unless set.
    pub fn with_default_route(mut self, route: impl Into<String>) -> TestPlatform {
        self.default_route = route.into();
        self
    }

    /// Fonts for text to shape against; none unless set, so text lays out empty.
    pub fn with_font_source(
        mut self,
        make: impl Fn() -> Box<dyn FontSource> + 'static,
    ) -> TestPlatform {
        self.font_source = Some(Rc::new(make));
        self
    }

    /// The decoder behind `open_image_codec`; none unless set, so every image fails to open.
    pub fn with_image_codec(
        mut self,
        open: impl Fn(Arc<[u8]>) -> ImageCodecFuture + 'static,
    ) -> TestPlatform {
        self.image_codec = Some(Rc::new(open));
        self
    }

    pub fn with_windowing(mut self, owner: Rc<dyn WindowingOwner>) -> TestPlatform {
        self.windowing = Some(owner);
        self
    }

    pub fn with_clipboard(mut self, clipboard: Rc<dyn Clipboard>) -> TestPlatform {
        self.clipboard = Some(clipboard);
        self
    }

    pub fn with_system_chrome(mut self, system_chrome: Rc<dyn SystemChrome>) -> TestPlatform {
        self.system_chrome = Some(system_chrome);
        self
    }

    pub fn with_haptics(mut self, haptics: Rc<dyn Haptics>) -> TestPlatform {
        self.haptics = Some(haptics);
        self
    }

    pub fn with_system_context_menu(mut self, menu: Rc<dyn SystemContextMenu>) -> TestPlatform {
        self.system_context_menu = Some(menu);
        self
    }

    pub fn with_popup_menus(mut self, menus: Rc<dyn PopupMenus>) -> TestPlatform {
        self.popup_menus = Some(menus);
        self
    }

    pub fn with_text_services(mut self, services: Rc<dyn TextServices>) -> TestPlatform {
        self.text_services = Some(services);
        self
    }

    pub fn with_mouse_cursor(mut self, cursor: Rc<dyn MouseCursor>) -> TestPlatform {
        self.mouse_cursor = Some(cursor);
        self
    }

    pub fn with_restoration(mut self, restoration: Rc<dyn Restoration>) -> TestPlatform {
        self.restoration = Some(restoration);
        self
    }

    /// Changes which host the platform claims to be, for a test that switches mid-way.
    pub fn set_target(&self, target: TargetPlatform) {
        self.target.set(target);
    }

    /// Changes the brightness; the client hears of it as it would from a host.
    pub fn set_brightness(&self, brightness: Brightness) {
        self.brightness.set(brightness);
    }

    /// How many frames the framework has asked for.
    pub fn frames_requested(&self) -> u32 {
        self.frames_requested.get()
    }

    /// The timer wakeups the framework has set, in order. A `None` is the framework
    /// saying no timer is waiting.
    pub fn wakes(&self) -> Vec<Option<Instant>> {
        self.dispatcher.timer_wakeups()
    }

    /// How often a ready task has asked the host to wake the app.
    pub fn wake_now_requests(&self) -> usize {
        self.dispatcher.wake_now_requests()
    }
}

impl Default for TestPlatform {
    fn default() -> TestPlatform {
        TestPlatform::new()
    }
}

impl Platform for TestPlatform {
    fn target_platform(&self) -> TargetPlatform {
        self.target.get()
    }

    fn request_frame(&self) {
        self.frames_requested.set(self.frames_requested.get() + 1);
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::clone(&self.dispatcher) as Arc<dyn Dispatcher>
    }

    fn views(&self) -> Vec<ViewRef> {
        self.views.clone()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views.iter().find(|view| view.id() == id).cloned()
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        self.views.first().cloned()
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    fn default_route_name(&self) -> String {
        self.default_route.clone()
    }

    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        self.font_source.as_ref().map(|make| make())
    }

    fn open_image_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        match &self.image_codec {
            Some(open) => open(bytes),
            None => Box::pin(std::future::ready(Err(ImageDecodeError::NoDecoder))),
        }
    }

    fn windowing_owner(&self) -> Option<Rc<dyn WindowingOwner>> {
        self.windowing.clone()
    }

    fn clipboard(&self) -> Option<&dyn Clipboard> {
        self.clipboard.as_deref()
    }

    fn system_chrome(&self) -> Option<&dyn SystemChrome> {
        self.system_chrome.as_deref()
    }

    fn haptics(&self) -> Option<&dyn Haptics> {
        self.haptics.as_deref()
    }

    fn system_context_menu(&self) -> Option<&dyn SystemContextMenu> {
        self.system_context_menu.as_deref()
    }

    fn popup_menus(&self) -> Option<&dyn PopupMenus> {
        self.popup_menus.as_deref()
    }

    fn text_services(&self) -> Option<&dyn TextServices> {
        self.text_services.as_deref()
    }

    fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
        self.mouse_cursor.as_deref()
    }

    fn restoration(&self) -> Option<&dyn Restoration> {
        self.restoration.as_deref()
    }
}

/// A host that records what it was asked for and never wakes the app. The test pumps the app
/// instead, where a host would call `EmbedderClient::wake`, and work runs at once on the
/// calling thread, so a test needs only a checkpoint to see its result.
#[derive(Default)]
pub struct TestDispatcher {
    timer_wakeups: Mutex<Vec<Option<Instant>>>,
    wake_now_requests: AtomicUsize,
}

impl TestDispatcher {
    /// The timer wakeups set, in order. A `None` is the application saying no timer is
    /// waiting.
    pub fn timer_wakeups(&self) -> Vec<Option<Instant>> {
        self.timer_wakeups
            .lock()
            .expect("the wakeups are never poisoned: nothing runs under their lock")
            .clone()
    }

    /// How often a ready task has asked the host to wake the app.
    pub fn wake_now_requests(&self) -> usize {
        self.wake_now_requests.load(Ordering::Acquire)
    }
}

impl Dispatcher for TestDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        self.timer_wakeups
            .lock()
            .expect("the wakeups are never poisoned: nothing runs under their lock")
            .push(timer_wakeup);
    }

    fn wake_now(&self) {
        self.wake_now_requests.fetch_add(1, Ordering::AcqRel);
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }
}

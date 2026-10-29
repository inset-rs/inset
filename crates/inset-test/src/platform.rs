//! A platform a test configures and reads back.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use inset_embedder::{
    AppExit, AppLifecycleState, Brightness, Cameras, Clipboard, Compositor, Dispatcher, DockMenu,
    FileDialogs, FontSource, Haptics, Image, ImageCodecFuture, ImageDecodeError, Instant, MenuBar,
    MouseCursor, Notifications, Opener, Platform, Players, PopupMenus, RequestId, Restoration,
    SystemChrome, SystemContextMenu, TargetPlatform, TextServices, TrayIcons, View, ViewError,
    ViewId, ViewRef, ViewRequest,
};

use inset_embedder::valo;

use crate::{TestView, TestWindows};

type FontSourceMaker = Rc<dyn Fn() -> Box<dyn FontSource>>;
type ImageCodecOpener = Rc<dyn Fn(Arc<[u8]>) -> ImageCodecFuture>;
type PixelImporter = Rc<dyn Fn(valo::PixelBuffer) -> Option<Image>>;

/// A platform a test builds: which host it claims to be, the views it offers, and each
/// capability as the test's own implementation or absent. It keeps the frames the framework asks
/// for and the deadlines it asks to be woken at, so a test can assert on them; the test itself
/// pumps the app, since there is no loop to run it.
pub struct TestPlatform {
    target: Cell<TargetPlatform>,
    brightness: Cell<Brightness>,
    default_route: String,
    /// The views the platform gave the app, as a phone gives its screen.
    views: RefCell<Vec<ViewRef>>,
    /// The [`TestView`]s among them that [`with_views`](Self::with_views) made, which
    /// [`test_view`](Self::test_view) answers.
    test_views: Vec<Rc<TestView>>,
    /// The view of each frame asked for since the test last took them, in the order asked.
    frame_requests: RefCell<Vec<ViewId>>,
    dispatcher: Arc<TestDispatcher>,
    font_source: Option<FontSourceMaker>,
    image_codec: Option<ImageCodecOpener>,
    pixel_import: Option<PixelImporter>,
    windows: Option<Rc<TestWindows>>,
    clipboard: Option<Rc<dyn Clipboard>>,
    system_chrome: Option<Rc<dyn SystemChrome>>,
    haptics: Option<Rc<dyn Haptics>>,
    system_context_menu: Option<Rc<dyn SystemContextMenu>>,
    menu_bar: Option<Rc<dyn MenuBar>>,
    popup_menus: Option<Rc<dyn PopupMenus>>,
    tray_icons: Option<Rc<dyn TrayIcons>>,
    file_dialogs: Option<Rc<dyn FileDialogs>>,
    opener: Option<Rc<dyn Opener>>,
    notifications: Option<Rc<dyn Notifications>>,
    compositor: Option<Arc<dyn Compositor>>,
    players: Option<Rc<dyn Players>>,
    cameras: Option<Rc<dyn Cameras>>,
    dock_menu: Option<Rc<dyn DockMenu>>,
    text_services: Option<Rc<dyn TextServices>>,
    mouse_cursor: Option<Rc<dyn MouseCursor>>,
    restoration: Option<Rc<dyn Restoration>>,
    app_exit: Option<Rc<dyn AppExit>>,
    initial_lifecycle_state: Option<AppLifecycleState>,
}

impl TestPlatform {
    /// A platform claiming to be Android, as Flutter's test binding does, in light mode, with
    /// no view and no capability.
    pub fn new() -> TestPlatform {
        TestPlatform {
            target: Cell::new(TargetPlatform::Android),
            brightness: Cell::new(Brightness::Light),
            default_route: "/".into(),
            views: RefCell::default(),
            test_views: Vec::new(),
            frame_requests: RefCell::default(),
            dispatcher: Arc::new(TestDispatcher::default()),
            font_source: None,
            image_codec: None,
            pixel_import: None,
            windows: None,
            clipboard: None,
            system_chrome: None,
            haptics: None,
            system_context_menu: None,
            menu_bar: None,
            popup_menus: None,
            tray_icons: None,
            file_dialogs: None,
            opener: None,
            notifications: None,
            compositor: None,
            players: None,
            cameras: None,
            dock_menu: None,
            text_services: None,
            mouse_cursor: None,
            restoration: None,
            app_exit: None,
            initial_lifecycle_state: None,
        }
    }

    /// The same platform claiming to be `target`.
    pub fn on(self, target: TargetPlatform) -> TestPlatform {
        self.target.set(target);
        self
    }

    /// The same platform giving the app `view`, as a phone gives its screen.
    pub fn with_view(self, view: ViewRef) -> TestPlatform {
        self.views.borrow_mut().push(view);
        self
    }

    /// The same platform giving the app an 800 x 600 [`TestView`] numbered by each of `ids`, in
    /// that order, as a desktop gives one window to each.
    pub fn with_views(mut self, ids: &[ViewId]) -> TestPlatform {
        for &id in ids {
            let view = Rc::new(TestView::new(800.0, 600.0).with_id(id));
            self.views.borrow_mut().push(Rc::clone(&view) as ViewRef);
            self.test_views.push(view);
        }
        self
    }

    /// The view numbered `id` that [`with_views`](Self::with_views) gave, to read what was
    /// presented to it.
    ///
    /// # Panics
    ///
    /// If `with_views` gave none numbered `id`.
    pub fn test_view(&self, id: ViewId) -> Rc<TestView> {
        let view = self.test_views.iter().find(|view| view.id() == id);
        Rc::clone(view.expect("a view with_views gave"))
    }

    /// Gives the app `view` while it runs, as an embedder adds one; the test tells the
    /// framework of it itself, as a host would through `EmbedderClient::view_added`.
    pub fn give_view(&self, view: ViewRef) {
        self.views.borrow_mut().push(view);
    }

    /// Takes the view numbered `id` back while the app runs.
    pub fn take_view(&self, id: ViewId) {
        self.views.borrow_mut().retain(|view| view.id() != id);
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

    /// The lifecycle state the host would start the app in; none unless set.
    pub fn with_initial_lifecycle_state(mut self, state: AppLifecycleState) -> TestPlatform {
        self.initial_lifecycle_state = Some(state);
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

    /// The renderer behind `import_pixels`; none unless set, so no image is made from pixels.
    pub fn with_pixel_import(
        mut self,
        import: impl Fn(valo::PixelBuffer) -> Option<Image> + 'static,
    ) -> TestPlatform {
        self.pixel_import = Some(Rc::new(import));
        self
    }

    /// The same platform answering requests for views with `windows`, as a desktop makes a
    /// window for each.
    pub fn with_windows(mut self, windows: Rc<TestWindows>) -> TestPlatform {
        self.windows = Some(windows);
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

    pub fn with_menu_bar(mut self, menu_bar: Rc<dyn MenuBar>) -> TestPlatform {
        self.menu_bar = Some(menu_bar);
        self
    }

    pub fn with_popup_menus(mut self, menus: Rc<dyn PopupMenus>) -> TestPlatform {
        self.popup_menus = Some(menus);
        self
    }

    pub fn with_tray_icons(mut self, tray_icons: Rc<dyn TrayIcons>) -> TestPlatform {
        self.tray_icons = Some(tray_icons);
        self
    }

    pub fn with_file_dialogs(mut self, file_dialogs: Rc<dyn FileDialogs>) -> TestPlatform {
        self.file_dialogs = Some(file_dialogs);
        self
    }

    pub fn with_opener(mut self, opener: Rc<dyn Opener>) -> TestPlatform {
        self.opener = Some(opener);
        self
    }

    pub fn with_notifications(mut self, notifications: Rc<dyn Notifications>) -> TestPlatform {
        self.notifications = Some(notifications);
        self
    }

    pub fn with_compositor(mut self, compositor: Arc<dyn Compositor>) -> TestPlatform {
        self.compositor = Some(compositor);
        self
    }

    pub fn with_players(mut self, players: Rc<dyn Players>) -> TestPlatform {
        self.players = Some(players);
        self
    }

    pub fn with_cameras(mut self, cameras: Rc<dyn Cameras>) -> TestPlatform {
        self.cameras = Some(cameras);
        self
    }

    pub fn with_dock_menu(mut self, dock_menu: Rc<dyn DockMenu>) -> TestPlatform {
        self.dock_menu = Some(dock_menu);
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

    pub fn with_app_exit(mut self, app_exit: Rc<dyn AppExit>) -> TestPlatform {
        self.app_exit = Some(app_exit);
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

    /// The view of each frame the framework asked for since the test last took them, in the
    /// order asked, a view asked again included.
    pub fn frame_requests(&self) -> Vec<ViewId> {
        self.frame_requests.borrow().clone()
    }

    /// Whether `view` asked for a frame since the test last took the requests.
    pub fn has_frame_request(&self, view: ViewId) -> bool {
        self.frame_requests.borrow().contains(&view)
    }

    /// Takes the request `view` made for a frame, for the test to give it its frame; answers
    /// whether it had made one since the last take.
    pub fn take_frame_request(&self, view: ViewId) -> bool {
        let mut requests = self.frame_requests.borrow_mut();
        let asked = requests.contains(&view);
        requests.retain(|&asking| asking != view);
        asked
    }

    /// The views that asked for a frame since the last take, each once, in the order the platform
    /// gained them, for the test to give them a refresh, as a host does. A request of a view the
    /// platform does not have is dropped, as a host drops it.
    pub fn take_frame_requests(&self) -> Vec<ViewId> {
        let requests = std::mem::take(&mut *self.frame_requests.borrow_mut());
        self.views()
            .iter()
            .map(|view| view.id())
            .filter(|id| requests.contains(id))
            .collect()
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

    fn request_frame(&self, view: ViewId) {
        self.frame_requests.borrow_mut().push(view);
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::clone(&self.dispatcher) as Arc<dyn Dispatcher>
    }

    fn views(&self) -> Vec<ViewRef> {
        let requested = self.windows.iter().flat_map(|windows| windows.open_views());
        let given = self.views.borrow().clone();
        given.into_iter().chain(requested).collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views().into_iter().find(|view| view.id() == id)
    }

    fn request_view(&self, request: ViewRequest) -> Result<RequestId, ViewError> {
        match &self.windows {
            Some(windows) => Ok(windows.request(request)),
            None => Err(ViewError::Unsupported),
        }
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    fn default_route_name(&self) -> String {
        self.default_route.clone()
    }

    fn initial_lifecycle_state(&self) -> Option<AppLifecycleState> {
        self.initial_lifecycle_state
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

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<Image> {
        self.pixel_import.as_ref().and_then(|import| import(pixels))
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

    fn menu_bar(&self) -> Option<&dyn MenuBar> {
        self.menu_bar.as_deref()
    }

    fn popup_menus(&self) -> Option<&dyn PopupMenus> {
        self.popup_menus.as_deref()
    }

    fn tray_icons(&self) -> Option<&dyn TrayIcons> {
        self.tray_icons.as_deref()
    }

    fn file_dialogs(&self) -> Option<&dyn FileDialogs> {
        self.file_dialogs.as_deref()
    }

    fn opener(&self) -> Option<&dyn Opener> {
        self.opener.as_deref()
    }

    fn notifications(&self) -> Option<&dyn Notifications> {
        self.notifications.as_deref()
    }

    fn compositor(&self) -> Option<Arc<dyn Compositor>> {
        self.compositor.clone()
    }

    fn players(&self) -> Option<&dyn Players> {
        self.players.as_deref()
    }

    fn cameras(&self) -> Option<&dyn Cameras> {
        self.cameras.as_deref()
    }

    fn dock_menu(&self) -> Option<&dyn DockMenu> {
        self.dock_menu.as_deref()
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

    fn app_exit(&self) -> Option<&dyn AppExit> {
        self.app_exit.as_deref()
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

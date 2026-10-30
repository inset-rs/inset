//! What an operating system adds beyond winit, as one small trait per concern.
//!
//! Every method has the answer of a system that adds nothing, so a system implements only
//! what it has, and the embedder calls through [`ThisOs`], the one system this build targets. The
//! exceptions are the target platform and the associated types: the display link and the
//! window's native surfaces, which `absent` answers for a system that has neither, and the
//! clipboard, which is arboard's plain text on a system without code of its own for one. What a
//! system lets the app do is answered in one place, on [`OperatingSystem`].
//! Five systems: macOS adds a window frame that animates, its menus (the menu bar, a menu at a
//! point, the Dock icon's and the status items), its pasteboard, a display link, native surfaces,
//! its own answers about its screens, whether a window is occluded and its notices that a window
//! was minimized, the fingers coming down on a trackpad, and the requests it makes of the application and what it says changed, among
//! others (`macos`); iOS reports its window's size and safe area differently, and has its
//! pasteboard and native surfaces (`ios`); Windows tells the clipboard's changes by its sequence
//! number, and has a menu at a point, icons in the notification area and toasts (`windows`); Linux
//! has the desktop's tray and its notification server (`linux`); every other system winit covers by
//! itself (`winit_only`). What macOS and iOS share, their notification center and the compositor
//! of native surfaces among it, is in `apple`.
//!
//! A system's module is compiled for its own target only, so a change to a trait is checked
//! against that system's answers only when building for it: build every target a change
//! touches.

#[cfg(not(target_os = "macos"))]
mod absent;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
mod winit_only;

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use inset_embedder::{
    Cameras, Clipboard, Compositor, DockMenu, FileDialogs, KeyData, MenuBar, MenuEntry, MenuId,
    Notifications, Offset, Opener, Players, Rect, Screen, ScreenId, SurfacePlacement,
    TargetPlatform, TrayIcons, ViewId, ViewPadding, VsyncWaiter, WindowConfig,
};
use valo_codec::Decoder;
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::monitor::MonitorHandle;
use winit::window::{Window, WindowAttributes};

use crate::clipboard::ClipboardChanges;
use crate::file_dialogs::ParentWindows;
use crate::lifecycle::LifecycleNotice;
use crate::main_loop::MainLoopProxy;
use crate::menus::MenuEvents;
use crate::notifications::NotificationActivations;
use crate::screens::{monitor_number, monitor_screens};
use crate::system_changes::SystemChange;
use crate::system_requests::SystemRequest;
use crate::vsync_waiter::LoopVsyncWaiter;

/// The system this build targets.
#[cfg(target_os = "macos")]
pub(crate) type ThisOs = macos::AppKit;
#[cfg(target_os = "ios")]
pub(crate) type ThisOs = ios::Ios;
#[cfg(target_os = "windows")]
pub(crate) type ThisOs = windows::Windows;
#[cfg(target_os = "linux")]
pub(crate) type ThisOs = linux::Linux;
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
pub(crate) type ThisOs = winit_only::WinitOnly;

/// This system's vsyncs for a display.
pub(crate) type ThisOsDisplayLink = <ThisOs as FramePresentation>::DisplayLink;

/// This system's share of its compositor for one window.
pub(crate) type ThisOsWindowSurfaces = <ThisOs as NativeComposition>::WindowSurfaces;

/// This system's notices that a window may have been minimized or occluded, for one window.
pub(crate) type ThisOsVisibilityNotices = <ThisOs as WindowVisibility>::Notices;

/// This system's report of the fingers coming down on a trackpad, for one window.
pub(crate) type ThisOsTrackpadTouchObserver = <ThisOs as TrackpadTouches>::Observer;

/// This system's clipboard.
pub(crate) type ThisOsClipboard = <ThisOs as SystemClipboard>::Clipboard;

/// Everything the embedder asks of a system, as one bound: which system it is, what it lets the
/// app do, and the concerns it answers.
pub(crate) trait OperatingSystem:
    EventLoopSetup
    + WindowGeometry
    + WindowMotion
    + WindowChrome
    + WindowVisibility
    + Screens
    + PointerQueries
    + TrackpadTouches
    + FramePresentation
    + NativeComposition
    + NativeMenus
    + SystemFileDialogs
    + SystemOpener
    + SystemNotifications
    + SystemMedia
    + SystemClipboard
    + ImageDecoding
    + WorkQueue
    + SystemRequests
    + SystemChanges
    + ApplicationControl
    + ApplicationLifecycle
{
    /// The platform the framework is told it runs on, where Flutter's `defaultTargetPlatform`
    /// bottoms out in `dart:io`'s `Platform.isAndroid` chain.
    const TARGET_PLATFORM: TargetPlatform;

    /// Whether the window is the screen, which the system sizes: a phone's. A desktop's
    /// windows are sized by the app.
    const WINDOW_IS_THE_SCREEN: bool = false;

    /// Whether the app may open windows of its own beside the screen view's: a desktop's may
    /// at once, an iPhone's may not, and an iPad's would ask the system, which this embedder does
    /// not.
    const APP_MAY_OPEN_WINDOWS: bool = true;

    /// Whether the app may quit itself. iOS gives it no way to (Apple's QA1561).
    const APP_MAY_QUIT: bool = true;
}

/// The build fails here, rather than at some call site, when this system leaves a concern out.
const _: fn() = answers_everything::<ThisOs>;

fn answers_everything<T: OperatingSystem>() {}

/// What winit sets up of the application as it builds its event loop, which the system's module
/// sets up otherwise.
pub(crate) trait EventLoopSetup {
    /// Configures winit's event loop before it is built: macOS leaves winit's own menu bar out,
    /// for the system's standard one, and leaves the app's coming forward at launch to the system.
    fn configure_event_loop(_builder: &mut EventLoopBuilder) {}
}

/// How big a window's drawing surface is, and what of it the system covers.
pub(crate) trait WindowGeometry {
    /// The pixels valo presents into: the window's surface.
    fn surface_size(window: &dyn Window) -> [u32; 2] {
        let size = window.surface_size();
        [size.width, size.height]
    }

    /// The safe area as Flutter reports it: the whole view is the size, what the system
    /// draws over it is padding. None on a desktop.
    fn view_padding(_window: &dyn Window) -> ViewPadding {
        ViewPadding::ZERO
    }
}

/// Where a window is and how it comes there. Positions are logical points from the top-left
/// corner of the main screen, as winit's are.
pub(crate) trait WindowMotion {
    /// Puts the window at `frame`, position and size in one step. `false` where the system
    /// cannot, and the caller sets them through winit instead.
    fn set_frame(_window: &dyn Window, _frame: Rect) -> bool {
        false
    }

    /// Moves and sizes the window over `duration`. `false` where the system cannot animate a
    /// frame, and the caller sets it at once instead.
    fn animate_frame(_window: &dyn Window, _frame: Rect, _duration: Duration) -> bool {
        false
    }

    /// Shows the window, sliding in to where it is placed from `from` away while it fades in,
    /// over `duration`, without changing its size. `false` where the system cannot animate a
    /// window, and the caller shows it at once instead.
    fn show_sliding_in(_window: &dyn Window, _from: Offset, _duration: Duration) -> bool {
        false
    }
}

/// What a window's configuration asks for beyond what winit's attributes carry.
pub(crate) trait WindowChrome {
    /// The attributes the system adds to winit's for `config`.
    fn extend_attributes(attributes: WindowAttributes, _config: &WindowConfig) -> WindowAttributes {
        attributes
    }

    /// Applies, once the window exists, what `config` asks for and no attribute carries.
    fn configure(_window: &dyn Window, _config: &WindowConfig) {}
}

/// Whether a window is occluded, and the system's notices that a window may have been minimized
/// or restored, occluded or shown: what winit neither answers nor reports. Whether a window is
/// minimized winit answers itself.
pub(crate) trait WindowVisibility {
    /// The notices for one window; they end when this is dropped.
    type Notices;

    /// Whether `window` is occluded now: covered all over, or otherwise not shown. `None` where the
    /// system cannot be asked, and the embedder answers with winit's last `Occluded` event instead.
    fn is_occluded(_window: &dyn Window) -> Option<bool> {
        None
    }

    /// Calls `on_notice`, on the main thread and from wherever the system posts, so it must only
    /// queue, whenever the system says `window` may have been minimized or restored, occluded or
    /// shown, until the answer is dropped. `None` where winit's events are all the system says.
    fn observe_visibility(
        _window: &dyn Window,
        _on_notice: Box<dyn Fn()>,
    ) -> Option<Self::Notices> {
        None
    }
}

/// The screens the views are on, and where the pointer is on them (`screens.rs`).
pub(crate) trait Screens {
    /// The screens, the main one first, in the coordinates of a window's frame. winit's monitors
    /// by default, which winit lists only through a window or inside its own calls: `window` is
    /// one of the app's, and with none no screen is known.
    fn screens(window: Option<&dyn Window>) -> Vec<Screen> {
        window.map(monitor_screens).unwrap_or_default()
    }

    /// The id `screens` gives the screen `monitor` is. By default the number winit's monitor
    /// keeps for the app's life (`screens.rs`).
    fn screen_id(monitor: &MonitorHandle) -> ScreenId {
        monitor_number(monitor)
    }

    /// Where the pointer is now, in the coordinates of a window's frame, over a window of the app
    /// or not. `None` where the system cannot be asked.
    fn pointer_location() -> Option<Offset> {
        None
    }
}

/// What the system can be asked about the pointer outside the events winit reports.
pub(crate) trait PointerQueries {
    /// The mouse buttons down now, as Flutter numbers them. `None` where the system cannot
    /// be asked.
    fn pressed_buttons() -> Option<i64> {
        None
    }
}

/// The fingers coming down on a trackpad, which winit does not report.
pub(crate) trait TrackpadTouches {
    /// What reports the touches over one window, until it is dropped.
    type Observer;

    /// Calls `on_touch` with the moment fingers come down on a trackpad over `window`, until the
    /// answer is dropped: on the main thread, from wherever the system reports it, so it must only
    /// queue. `None` where the system reports no trackpad's touches.
    fn observe_trackpad_touches(
        _window: &dyn Window,
        _on_touch: Box<dyn Fn(Instant)>,
    ) -> Option<Self::Observer> {
        None
    }
}

/// How a frame reaches the display, and when.
pub(crate) trait FramePresentation {
    /// The system's vsyncs for a display, which the framework's frames are drawn at.
    type DisplayLink: DisplayLink;

    /// Where any thread waits for a screen's next vsync: by default the main loop, which serves
    /// a wait at the vsync its frames on that screen are drawn at (`vsync_waiter.rs`). A system
    /// whose display links tick in every mode of its run loop serves the waits there, so they are
    /// served while the system runs a loop of its own and the main loop hears nothing. Main
    /// thread.
    fn vsync_waiter(proxy: &MainLoopProxy) -> Arc<dyn VsyncWaiter> {
        Arc::new(LoopVsyncWaiter(proxy.clone()))
    }

    /// Whether a resize draws a frame before it returns, for a system that commits window
    /// geometry on its own before the next redraw.
    const FRAME_ON_RESIZE: bool = false;

    /// Whether the surface presents through the system's own transaction, so a frame and the
    /// geometry it was laid out for are committed together. Nothing to join by default.
    fn present_in_transaction(_surface: &mut valo::Surface, _wanted: bool) {}
}

/// A display's vsyncs: a call at each vsync of one display, whether or not a window on it is
/// shown. It stays on its display; the embedder starts another when the frames move to another
/// display.
pub(crate) trait DisplayLink: Sized {
    /// Starts calling `on_vsync` at each vsync of `display`. `None` where the system has no display
    /// link for the display, and the embedder takes its vsyncs from a timer at the display's rate
    /// instead.
    fn start(display: &MonitorHandle, on_vsync: Box<dyn Fn()>) -> Option<Self>;

    /// Whether the calls are wanted: they stop while they are not, and start again once they
    /// are.
    fn set_wanted(&self, wanted: bool);
}

/// The system's compositor for native surfaces: Core Animation's on macOS and iOS, and why it is
/// shaped as it is, in `apple/composition`.
pub(crate) trait NativeComposition {
    /// One window's share of the compositor.
    type WindowSurfaces: WindowSurfaces;

    /// The compositor producers open surfaces on. `None` where the embedder hands the system
    /// no surfaces.
    fn compositor() -> Option<Arc<dyn Compositor>> {
        None
    }
}

/// One window's share of the compositor: it places a scene's native surfaces beneath the
/// window's content.
pub(crate) trait WindowSurfaces {
    fn new(window: Arc<dyn Window>) -> Self;

    /// Places a scene's surfaces beneath the window's content, bottom-most first. Main thread.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId);
}

/// Menus the system draws itself (`menus/`): each place is the system's, or none where it has
/// none, and the app draws its own there.
pub(crate) trait NativeMenus {
    /// Whether the system opens a menu at a point in a window: macOS's and Windows' do. Linux's
    /// desktops have no menu of the system's: GTK and Qt draw theirs in a pop-up surface, which
    /// winit offers as a popup window (`WindowType::Popup`), where a framework can draw them.
    const POPUP_MENUS: bool = false;

    /// Shows `menu` at `at`, in logical pixels from the top-left corner of `window`'s content,
    /// once the embedder's current call has returned, its loop outside every call into the
    /// embedder, and hands `closed` the id of the item chosen once it closes. A provided item acts
    /// as the system's own does, its edit commands sent through `events`, and answers none.
    fn open_popup_menu(
        _window: &dyn Window,
        _at: Offset,
        _menu: Vec<MenuEntry>,
        _events: MenuEvents,
        closed: Box<dyn FnOnce(Option<MenuId>) + Send>,
    ) {
        closed(None);
    }

    /// The app's menu bar, which shows the system's standard one until the app sets its own:
    /// macOS's. `None` where the system has none.
    fn menu_bar(_events: MenuEvents) -> Option<Rc<dyn MenuBar>> {
        None
    }

    /// The Dock icon's menu: macOS's. `None` where the system has no Dock.
    fn dock_menu(_events: MenuEvents) -> Option<Rc<dyn DockMenu>> {
        None
    }

    /// The app's icons in the system's tray: macOS's status items, Windows' notification area,
    /// Linux's StatusNotifierItems. `None` where the system has no tray.
    fn tray_icons(_events: MenuEvents) -> Option<Rc<dyn TrayIcons>> {
        None
    }

    /// Whether the system's menus send the focused view Edit's cut, copy and paste, which a key
    /// the framework leaves reaches: macOS's menu bar, while it holds those items.
    fn provides_edit_commands() -> bool {
        false
    }

    /// Offers the key-down being handled, which the framework left, to the system's menus, as
    /// AppKit offers a key equivalent to its main menu after the window's views; answers whether
    /// a menu took it. Main thread, inside the call that reports the key.
    fn perform_key_equivalent() -> bool {
        false
    }

    /// Keeps the system's own event of `key`, being reported, a short while, for an app that
    /// claims the key and hands it back to the menu bar later (`MenuBar::perform_key_equivalent`):
    /// macOS's. Main thread, inside the call that reports the key.
    fn keep_key_event(_key: &KeyData) {}
}

/// The system's way to open links and files in other apps.
pub(crate) trait SystemOpener {
    /// The system's opener, or `None` where it has none.
    fn opener(_windows: ParentWindows) -> Option<Rc<dyn Opener>> {
        None
    }
}

/// The system's notifications (`os/*/notifications`), their clicks sent to the main loop
/// (`notifications.rs`).
pub(crate) trait SystemNotifications {
    /// Starts hearing the clicks on the app's notifications before the loop runs, where the system
    /// reports the click that launched the app only to a listener set by then: Apple's, whose
    /// center wants its delegate before the launch finishes, and Windows', whose toast activator
    /// the app registers once it starts.
    fn hear_notification_activations(_activations: NotificationActivations) {}

    /// The system's notifications, each click sent to `activations`, or `None` where the app has
    /// none.
    fn notifications(_activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        None
    }
}

/// The system's file dialogs (`file_dialogs.rs`).
pub(crate) trait SystemFileDialogs {
    /// The system's file dialogs, each shown on its parent view's window, or `None` where the
    /// system has none.
    fn file_dialogs(_windows: ParentWindows) -> Option<Rc<dyn FileDialogs>> {
        None
    }
}

/// The system's media stack: its players and its cameras (`apple/media` on macOS and iOS).
pub(crate) trait SystemMedia {
    /// The system's players, or `None` where the embedder has none.
    fn players() -> Option<Rc<dyn Players>> {
        None
    }

    /// The system's cameras, or `None` where the embedder has none.
    fn cameras() -> Option<Rc<dyn Cameras>> {
        None
    }
}

/// The system's clipboard (`clipboard/`).
pub(crate) trait SystemClipboard {
    /// The clipboard as the framework reaches it on this system.
    type Clipboard: Clipboard + ClipboardChanges;

    /// The clipboard, which counts the app's own writes as seen, and calls `look` to have the main
    /// loop look at it, where the system says it changed or the clipboard clears an expired copy
    /// itself, on the main thread, from the system's own dispatch, so `look` must only queue.
    fn clipboard(look: Box<dyn Fn()>) -> Self::Clipboard;
}

/// The system's own image decoders.
pub(crate) trait ImageDecoding {
    /// The decoders the system has, which come before the portable ones: they read formats no
    /// portable decoder carries, and hand back frames the GPU can sample without an upload.
    /// None where the system has no decoder, or this build left it out.
    fn system_decoders() -> Vec<Box<dyn Decoder>> {
        Vec::new()
    }
}

/// Where work off the main thread runs.
pub(crate) trait WorkQueue {
    /// Runs `work` off the main thread: on a thread of its own where the system has no work
    /// queue to give it to. A panic in `work` ends that work alone, and the application runs
    /// on; the panic hook prints it as it does any.
    fn run_off_main(work: Box<dyn FnOnce() + Send>) {
        std::thread::Builder::new()
            .name("inset-worker".into())
            .spawn(work)
            .expect("a thread for work off the main thread");
    }
}

/// What the system asks of the application as a whole, beyond the events winit reports
/// (`system_requests.rs`).
pub(crate) trait SystemRequests {
    /// Starts sending the system's requests to `send`, on the main thread, from wherever the
    /// system makes them, so `send` must only queue. Nothing is sent where the system makes
    /// none of these requests of an application.
    fn send_system_requests(_send: Box<dyn Fn(SystemRequest)>) {}
}

/// What the system says changed about the machine beyond what winit reports: its screens and the
/// user's languages (`system_changes.rs`).
pub(crate) trait SystemChanges {
    /// Starts sending what the system says changed to `send`, on the main thread, from wherever
    /// the system posts it, so `send` must only queue. Nothing is sent where the embedder hears
    /// none of it.
    fn send_system_changes(_send: Box<dyn Fn(SystemChange)>) {}
}

/// What the embedder tells the system of the application as a whole: how it quits, and how it
/// steps aside.
pub(crate) trait ApplicationControl {
    /// Quits the way the system quits an application; the event loop ends where the system has
    /// no quit of its own.
    fn quit(event_loop: &dyn ActiveEventLoop) {
        event_loop.exit();
    }

    /// Hides the application's windows and hands the focus to the application that had it
    /// before, as the system's Hide command does. Nothing where the system has no such command.
    fn hide_application() {}
}

/// What the system says of the application as a whole beyond the events winit reports: whether
/// it is active, and whether any of it is visible (`lifecycle.rs`).
pub(crate) trait ApplicationLifecycle {
    /// Starts sending the system's notices to `send`, on the main thread, from wherever the
    /// system posts them, so `send` must only queue. Nothing is sent where the embedder hears none,
    /// and the framework is told no lifecycle state there.
    fn send_lifecycle_notices(_send: Box<dyn Fn(LifecycleNotice)>) {}
}

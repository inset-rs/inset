//! What an operating system adds beyond winit, as one small trait per concern.
//!
//! Every method has the answer of a system that adds nothing, so a system implements only
//! what it has, and the embedder calls through [`ThisOs`], the one system this build targets. The
//! exceptions are the target platform and the two associated types, the display link and the
//! window's native surfaces, which `absent` answers for a system that has neither. What a system
//! lets the app do is answered in one place, on [`OperatingSystem`].
//! Three systems: macOS adds a window frame that animates, the popup menu, a display link,
//! native surfaces, its own answers about its screens, whether a window is occluded and its
//! notices that a window was minimized, and the requests it makes of the application and what
//! it says changed, among others (`macos`); iOS reports its window's size
//! and safe area differently (`ios`); every other system winit covers by itself (`winit_only`).
//! What macOS and iOS share is in `apple`.
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
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod winit_only;

use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    Compositor, Offset, PopupMenuEntry, Rect, Screen, ScreenId, SurfacePlacement, TargetPlatform,
    ViewId, ViewPadding, VsyncWaiter, WindowConfig,
};
use valo_codec::Decoder;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::monitor::MonitorHandle;
use winit::window::{Window, WindowAttributes};

use crate::lifecycle::LifecycleNotice;
use crate::main_loop::MainLoopEvent;
use crate::screens::{monitor_number, monitor_screens};
use crate::system_changes::SystemChange;
use crate::system_requests::SystemRequest;
use crate::vsync_waiter::LoopVsyncWaiter;

/// The system this build targets.
#[cfg(target_os = "macos")]
pub(crate) type ThisOs = macos::AppKit;
#[cfg(target_os = "ios")]
pub(crate) type ThisOs = ios::Ios;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) type ThisOs = winit_only::WinitOnly;

/// This system's vsyncs for a display.
pub(crate) type ThisOsDisplayLink = <ThisOs as FramePresentation>::DisplayLink;

/// This system's share of its compositor for one window.
pub(crate) type ThisOsWindowSurfaces = <ThisOs as NativeComposition>::WindowSurfaces;

/// This system's notices that a window may have been minimized or occluded, for one window.
pub(crate) type ThisOsVisibilityNotices = <ThisOs as WindowVisibility>::Notices;

/// Everything the embedder asks of a system, as one bound: which system it is, what it lets the
/// app do, and the concerns it answers.
pub(crate) trait OperatingSystem:
    WindowGeometry
    + WindowMotion
    + WindowChrome
    + WindowVisibility
    + Screens
    + PointerQueries
    + FramePresentation
    + NativeComposition
    + NativeMenus
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

/// How big a window's drawing surface is, and what of it the system covers.
pub(crate) trait WindowGeometry {
    /// The pixels valo presents into: the window's content.
    fn surface_size(window: &Window) -> [u32; 2] {
        let size = window.inner_size();
        [size.width, size.height]
    }

    /// The safe area as Flutter reports it: the whole view is the size, what the system
    /// draws over it is padding. None on a desktop.
    fn view_padding(_window: &Window, _size: [f64; 2]) -> ViewPadding {
        ViewPadding::ZERO
    }
}

/// Where a window is and how it comes there. Positions are logical points from the top-left
/// corner of the main screen, as winit's are.
pub(crate) trait WindowMotion {
    /// Puts the window at `frame`, position and size in one step. `false` where the system
    /// cannot, and the caller sets them through winit instead.
    fn set_frame(_window: &Window, _frame: Rect) -> bool {
        false
    }

    /// Moves and sizes the window over `duration`. `false` where the system cannot animate a
    /// frame, and the caller sets it at once instead.
    fn animate_frame(_window: &Window, _frame: Rect, _duration: Duration) -> bool {
        false
    }

    /// Shows the window, sliding in to where it is placed from `from` away while it fades in,
    /// over `duration`, without changing its size. `false` where the system cannot animate a
    /// window, and the caller shows it at once instead.
    fn show_sliding_in(_window: &Window, _from: Offset, _duration: Duration) -> bool {
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
    fn configure(_window: &Window, _config: &WindowConfig) {}
}

/// Whether a window is occluded, and the system's notices that a window may have been minimized
/// or restored, occluded or shown: what winit neither answers nor reports. Whether a window is
/// minimized winit answers itself.
pub(crate) trait WindowVisibility {
    /// The notices for one window; they end when this is dropped.
    type Notices;

    /// Whether `window` is occluded now: covered all over, or otherwise not shown. `None` where the
    /// system cannot be asked, and the embedder answers with winit's last `Occluded` event instead.
    fn is_occluded(_window: &Window) -> Option<bool> {
        None
    }

    /// Calls `on_notice`, on the main thread and from wherever the system posts, so it must only
    /// queue, whenever the system says `window` may have been minimized or restored, occluded or
    /// shown, until the answer is dropped. `None` where winit's events are all the system says.
    fn observe_visibility(_window: &Window, _on_notice: Box<dyn Fn()>) -> Option<Self::Notices> {
        None
    }
}

/// The screens the views are on, and where the pointer is on them (`screens.rs`).
pub(crate) trait Screens {
    /// The screens, the main one first, in the coordinates of a window's frame. winit's monitors
    /// by default, which winit lists only through a window or inside its own calls: `window` is
    /// one of the app's, and with none no screen is known.
    fn screens(window: Option<&Window>) -> Vec<Screen> {
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

    /// Where the pointer is now, in `window`'s physical pixels from its top-left corner.
    /// `None` where the system cannot be asked.
    fn pointer_position_in(_window: &Window) -> Option<[f64; 2]> {
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
    fn vsync_waiter(proxy: &EventLoopProxy<MainLoopEvent>) -> Arc<dyn VsyncWaiter> {
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

/// The system's compositor for native surfaces: macOS's, and why it is shaped as it is, in
/// `macos/composition`.
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
    fn new(window: Arc<Window>) -> Self;

    /// Places a scene's surfaces beneath the window's content, bottom-most first. Main thread.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId);
}

/// Menus the system draws itself.
pub(crate) trait NativeMenus {
    /// Shows `entries` as the system's menu at the pointer once the embedder's current call has
    /// returned, its loop outside every call into the embedder, and hands `closed` the index of
    /// the entry chosen once it closes. Hands it `None` at once where the system has no such
    /// menu.
    fn open_popup_menu(
        _entries: Vec<PopupMenuEntry>,
        closed: Box<dyn FnOnce(Option<usize>) + Send>,
    ) {
        closed(None);
    }
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
    /// Whether the system asks the application to open files, rather than starting it with
    /// them on its command line.
    const ASKS_TO_OPEN_FILES: bool = false;

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
    fn quit(event_loop: &ActiveEventLoop) {
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

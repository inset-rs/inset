//! What every system answers, one small trait per concern, and [`OperatingSystem`], the bound
//! they make together: each method has the answer of a system that adds nothing, so a system
//! implements only what it has. The embedder calls the one system this build targets through
//! `os::System`, and names its associated types through `os`'s aliases.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use inset_embedder::{
    Cameras, CaptionSettings, Clipboard, CompositionLayer, Compositor, DataTransferItem, DockMenu,
    FileDialogs, KeyData, MediaAssets, MenuBar, MenuEntry, MenuId, Notifications, Offset, Opener,
    Placement, Players, Rect, Screen, ScreenId, TargetPlatform, TrayIcons, VideoFrameReaders,
    ViewId, ViewPadding, VsyncWaiter, WindowConfig,
};
use valo_codec::Decoder;
use winit::event_loop::{ActiveEventLoop, EventLoopBuilder};
use winit::monitor::MonitorHandle;
use winit::window::{Window, WindowAttributes};

use crate::clipboard::ClipboardChanges;
use crate::file_dialogs::ParentWindows;
use crate::gpu::Gpu;
use crate::lifecycle::LifecycleNotice;
use crate::main_loop::MainLoopProxy;
use crate::menus::MenuEvents;
use crate::notifications::NotificationActivations;
use crate::screens::{monitor_number, monitor_screens};
use crate::system_changes::SystemChange;
use crate::system_requests::SystemRequest;
use crate::vsync_waiter::LoopVsyncWaiter;

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
    + SystemDrags
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

    /// When the picture drawn at the latest vsync reaches the screen, as the system's display
    /// link says; `None` before the first vsync.
    fn target_time(&self) -> Option<Instant>;
}

/// The system's compositor for composition layers, Core Animation's on macOS and iOS, and why it
/// is shaped as it is, in `apple/composition`; and what a view shows into on this system, one kind
/// for every view.
pub(crate) trait NativeComposition {
    /// What every view shows into on this system.
    type ViewLayer: ViewLayer;

    /// A window's hold on the layer its view shows into, kept while the window is open: on macOS
    /// the composition layer the window placed in itself and keeps filling it; nothing where the
    /// view draws into the window itself.
    type WindowHold;

    /// The compositor producers make layers on. `None` where the embedder hands the system no
    /// layers.
    fn compositor() -> Option<Arc<dyn Compositor>> {
        None
    }

    /// What a window's code does for its view as `window` opens, as an app does for a view it
    /// adds: makes the layer the view shows into and places it in the window, filling it. Answers
    /// the layer, which the view is added on, and the window's hold on it. Main thread.
    fn view_layer_in(window: &Arc<dyn Window>) -> (Self::ViewLayer, Self::WindowHold);

    /// Whether a view the app adds can draw into a composition layer the app places, which
    /// `view_layer_on` then answers for every layer this host's compositor made.
    const DRAWS_VIEWS_INTO_LAYERS: bool = false;

    /// What a view the app adds on `layer` shows into, and whether `layer` was made opaque; `None`
    /// where the system cannot draw a view into a composition layer, and for a layer another host
    /// made. Main thread.
    fn view_layer_on(_layer: &Arc<dyn CompositionLayer>) -> Option<(Self::ViewLayer, bool)> {
        None
    }
}

/// What a view shows into: the source of the surface it draws on, and the place of the composition
/// layers its scenes place, beneath its drawing. On macOS a composition layer of the view's own,
/// whose content is a `CAMetalLayer` the view draws into (`apple/composition/view_layer.rs`);
/// elsewhere the view's window, which the view draws into through the window's handle
/// (`window_handle_layer.rs`).
///
/// Whoever holds a view places its layer, as Flutter's embedder holds a view it adds with
/// `AddView`: a window's code makes its view's layer with the window and keeps a hold of its own on
/// it while the window is open, through which it places the layer in the window and keeps it
/// filling the window (`NativeComposition::view_layer_in`); the view lets go of its layer when the
/// window closes. The view sets on its layer what its own metrics say: its scale here, and its size
/// through the surface.
pub(crate) trait ViewLayer {
    /// Whether a frame that paints nothing besides its composition layers reaches the display
    /// without being drawn: its layers, which the compositor places, over the background the layer
    /// shows itself (`show_background`). Where it does not, every frame is drawn, its background
    /// at least.
    const SHOWS_FRAMES_UNDRAWN: bool = false;

    /// The view's device pixel ratio is `scale` from now on: its drawing and the composition
    /// layers its scenes place are laid out at it. Main thread.
    fn set_scale(&self, _scale: f64) {}

    /// A surface on `gpu`'s device that draws into the layer, `size` pixels large, with the
    /// compositor honouring its alpha or not. Main thread.
    fn make_surface(
        &self,
        gpu: &Gpu,
        size: [u32; 2],
        alpha: valo::SurfaceAlpha,
    ) -> Result<valo::Surface, String>;

    /// Places a scene's composition layers beneath the view's drawing, bottom-most first, and
    /// lets go of the ones it no longer places. Main thread.
    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    );

    /// Shows `color` behind the view's drawing and its composition layers, for the frames that
    /// are not drawn and the empty ones. Main thread.
    fn show_background(&self, _color: valo::Color) {}
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

/// The system's media stack: its players, its reading of media without playing it, its cameras
/// and the frame readers they feed (`apple/media` on macOS and iOS), and the user's caption
/// settings (`apple/caption_appearance`).
pub(crate) trait SystemMedia {
    /// The system's players, or `None` where the embedder has none.
    fn players() -> Option<Rc<dyn Players>> {
        None
    }

    /// The system's reading of media without playing it, or `None` where the embedder has none.
    fn media_assets() -> Option<Rc<dyn MediaAssets>> {
        None
    }

    /// The system's cameras, or `None` where the embedder has none.
    fn cameras() -> Option<Rc<dyn Cameras>> {
        None
    }

    /// The readers the system's players and cameras feed, or `None` where it hands out no
    /// frames.
    fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
        None
    }

    /// The user's caption settings, or `None` where the system keeps none.
    fn caption_settings() -> Option<Rc<dyn CaptionSettings>> {
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

/// What a drag over a window carries, where the embedder reads it as the system's clipboard reads
/// a paste rather than through winit's data transfer (`drag_drop`).
pub(crate) trait SystemDrags {
    /// What is being dragged over the app now, listed as the clipboard lists a paste; `None` where
    /// the embedder reads a drag through winit.
    fn dragged_items() -> Option<Vec<Box<dyn DataTransferItem>>> {
        None
    }

    /// Lets every drag the system's reader takes reach `window`, beyond the forms winit registers
    /// it for.
    fn accept_drags(_window: &dyn Window) {}
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

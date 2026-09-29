//! What an operating system adds beyond winit, as one small trait per concern.
//!
//! Every method has the answer of a system that adds nothing, so a system implements only
//! what it has, and the host calls through [`ThisOs`], the one system this build targets.
//! Three systems: macOS adds a window frame that animates, the popup menu, a display link and
//! native surfaces, among others (`macos`); iOS reports its window's size and safe area
//! differently (`ios`); every other system winit covers by itself (`winit_only`).
//!
//! A system's module is compiled for its own target only, so a change to a trait is checked
//! against that system's answers only when building for it: build every target a change
//! touches.

#[cfg(not(target_os = "macos"))]
mod absent;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod winit_only;

use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    Compositor, PopupMenuEntry, Rect, SurfacePlacement, ViewId, ViewPadding, WindowConfig,
};
use winit::window::{Window, WindowAttributes};

/// The system this build targets.
#[cfg(target_os = "macos")]
pub(crate) type ThisOs = macos::MacOs;
#[cfg(target_os = "ios")]
pub(crate) type ThisOs = ios::Ios;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) type ThisOs = winit_only::WinitOnly;

/// This system's refresh signal for a window.
pub(crate) type ThisOsDisplayLink = <ThisOs as FramePresentation>::DisplayLink;

/// This system's share of its compositor for one window.
pub(crate) type ThisOsWindowSurfaces = <ThisOs as NativeComposition>::WindowSurfaces;

/// Everything the host asks of a system, as one bound.
pub(crate) trait OperatingSystem:
    WindowGeometry
    + WindowChrome
    + FramePresentation
    + PointerState
    + NativeComposition
    + NativeMenus
    + WorkQueue
{
}

/// The build fails here, rather than at some call site, when this system leaves a concern out.
const _: fn() = answers_everything::<ThisOs>;

fn answers_everything<T: OperatingSystem>() {}

/// Where a window is, how big its drawing surface is, and what of it the system covers.
pub(crate) trait WindowGeometry {
    /// Whether the window is the screen, which the system sizes: a phone's. A desktop's
    /// windows are sized by the app.
    const WINDOW_IS_THE_SCREEN: bool = false;

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

/// How a frame reaches the display, and when.
pub(crate) trait FramePresentation {
    /// The system's refresh signal for a window.
    type DisplayLink: DisplayLink;

    /// Whether a resize draws a frame before it returns, for a system that commits window
    /// geometry on its own before the next redraw.
    const FRAME_ON_RESIZE: bool = false;

    /// Whether the surface presents through the system's own transaction, so a frame and the
    /// geometry it was laid out for are committed together. Nothing to join by default.
    fn present_in_transaction(_surface: &mut valo::Surface, _wanted: bool) {}
}

/// A window's refresh signal: a call at each refresh of the display the window is on.
pub(crate) trait DisplayLink: Sized {
    /// Starts calling `on_tick` at each refresh. `None` where the system has no signal for
    /// the window, and the host paces by a timer at the display's rate instead.
    fn start(window: &Window, on_tick: Box<dyn Fn()>) -> Option<Self>;

    /// Stops the calls, or starts them again.
    fn set_paused(&self, paused: bool);
}

/// What the system can be asked about the pointer outside the events winit reports.
pub(crate) trait PointerState {
    /// The mouse buttons down now, as Flutter numbers them. `None` where the system cannot
    /// be asked.
    fn pressed_buttons() -> Option<i64> {
        None
    }

    /// Where the pointer is now, in the window's physical pixels from its top-left corner.
    /// `None` where the system cannot be asked.
    fn pointer_position(_window: &Window) -> Option<[f64; 2]> {
        None
    }
}

/// The system's compositor for native surfaces (`docs/composition.md`).
pub(crate) trait NativeComposition {
    /// One window's share of the compositor.
    type WindowSurfaces: WindowSurfaces;

    /// The compositor producers open surfaces on. `None` where the host hands the system
    /// no surfaces.
    fn compositor() -> Option<Arc<dyn Compositor>> {
        None
    }
}

/// One window's share of the compositor: it places a scene's native surfaces beneath the
/// window's content. How the surfaces' producers get their vsyncs is the system's own.
pub(crate) trait WindowSurfaces {
    fn new(window: Arc<Window>) -> Self;

    /// Places a scene's surfaces beneath the window's content, bottom-most first. Main thread.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId);
}

/// Menus the system draws itself.
pub(crate) trait NativeMenus {
    /// Shows `entries` as the system's menu at the pointer and waits for it to close,
    /// answering the index of the entry chosen. `None` where the system has no such menu.
    fn popup_menu(_entries: &[PopupMenuEntry]) -> Option<usize> {
        None
    }
}

/// Where work off the main thread runs.
pub(crate) trait WorkQueue {
    /// Runs `work` off the main thread: on a thread of its own where the system has no work
    /// queue to give it to.
    fn run_off_main(work: Box<dyn FnOnce() + Send>) {
        std::thread::Builder::new()
            .name("inset-worker".into())
            .spawn(work)
            .expect("a thread for work off the main thread");
    }
}

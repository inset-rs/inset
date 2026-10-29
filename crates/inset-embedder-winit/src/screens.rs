//! The screens as the app reads them (`Platform::screens`), and the screen a window is on, which
//! its view's metrics carry (`view.rs`): the system's own answers where it has them (`os/`), and
//! winit's monitors otherwise.
//!
//! winit lists its monitors only through a window or inside its own calls, so a system without
//! answers of its own lists them through one of the app's windows, and knows none while the app
//! has no window. winit gives a monitor no number, so such a system numbers its monitors in the
//! order they are first seen, as Flutter's Linux embedder numbers its monitors. The invariant a
//! later editor must keep: a monitor keeps its number for the app's life, so that a view's screen
//! is always one `screens()` lists by the same id.

use std::cell::RefCell;

use inset_embedder::{Offset, Rect, Screen, ScreenId, Screens};
use winit::monitor::MonitorHandle;
use winit::window::Window;

use crate::WinitPlatform;
use crate::os::{Screens as _, ThisOs};

/// The system's answers where it has its own, and otherwise winit's monitors, listed through the
/// app's first window.
impl Screens for WinitPlatform {
    fn screens(&self) -> Vec<Screen> {
        let first = self.state().views().first_window();
        ThisOs::screens(first.as_ref().map(|open| &*open.window))
    }

    fn pointer_location(&self) -> Option<Offset> {
        ThisOs::pointer_location()
    }
}

/// The screen `window` is on, by the id `screens` gives it; `None` where the system names no
/// monitor for the window.
pub(crate) fn screen_of(window: &dyn Window) -> Option<ScreenId> {
    window
        .current_monitor()
        .map(|monitor| ThisOs::screen_id(&monitor))
}

/// The monitors winit lists through `window`, the primary one first, as screens.
pub(crate) fn monitor_screens(window: &dyn Window) -> Vec<Screen> {
    let mut monitors: Vec<MonitorHandle> = window.available_monitors().collect();
    if let Some(primary) = window.primary_monitor()
        && let Some(index) = monitors.iter().position(|monitor| *monitor == primary)
    {
        monitors[..=index].rotate_right(1);
    }
    monitors.iter().map(monitor_screen).collect()
}

/// `monitor` as winit describes it: its place and size in logical points at its own scale, which
/// is how a window on it measures its frame (`host_window.rs`), its size and rate those of the mode
/// it shows, and the name winit gives it, which is the system's name for the output: `\\.\DISPLAY1`
/// on Windows, `HDMI-1` on X11. winit knows nothing of the system's bars, so the visible frame is
/// the whole frame. A monitor winit gives no position for is at the origin, and one showing no
/// mode has no size.
fn monitor_screen(monitor: &MonitorHandle) -> Screen {
    let scale = monitor.scale_factor();
    let position = monitor
        .position()
        .unwrap_or_default()
        .to_logical::<f64>(scale);
    let mode = monitor.current_video_mode();
    let size = mode
        .as_ref()
        .map(|mode| mode.size())
        .unwrap_or_default()
        .to_logical::<f64>(scale);
    let frame = Rect::from_ltwh(position.x, position.y, size.width, size.height);
    Screen {
        id: ThisOs::screen_id(monitor),
        name: monitor.name().map(String::from).unwrap_or_default(),
        frame,
        visible_frame: frame,
        scale,
        refresh_rate: mode
            .and_then(|mode| mode.refresh_rate_millihertz())
            .map(|millihertz| f64::from(millihertz.get()) / 1000.0),
    }
}

thread_local! {
    /// Every monitor winit has shown, in the order first seen: a monitor's number is its place
    /// here, counted from one. Main thread, where winit answers about its monitors.
    static SEEN_MONITORS: RefCell<Vec<MonitorHandle>> = const { RefCell::new(Vec::new()) };
}

/// The number `monitor` keeps for the app's life: the next one, the first time it is seen.
/// winit tells two handles on one monitor apart by the system's own identity of it: its
/// `HMONITOR` on Windows, its output on X11 and Wayland.
pub(crate) fn monitor_number(monitor: &MonitorHandle) -> ScreenId {
    SEEN_MONITORS.with_borrow_mut(|seen| {
        let index = match seen.iter().position(|known| known == monitor) {
            Some(index) => index,
            None => {
                seen.push(monitor.clone());
                seen.len() - 1
            }
        };
        ScreenId(index as u64 + 1)
    })
}

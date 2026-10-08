//! One display's vsyncs, one call for each vsync asked for: those of the display the first window
//! is on, from the system's display link for it where it has one ([`ThisOsDisplayLink`]), which
//! comes whether or not a window on the display is shown, and from a timer at the display's rate
//! elsewhere.
//!
//! A vsync is asked for one at a time, as Flutter's `VsyncWaiter` is. The source runs on for one
//! vsync after the one asked for, so an animation that asks again from each frame finds it
//! running, and that vsync, which nobody asked for, pauses it, so an idle app makes no wakes. The
//! rule a later editor could break: the source runs whenever a vsync is asked for and has not
//! come, so none waits for ever; it may run one vsync longer than asked, never shorter.

use std::rc::Rc;
use std::time::{Duration, Instant};

use winit::monitor::MonitorHandle;
use winit::window::Window;

use crate::os::{DisplayLink, ThisOsDisplayLink};

/// The time between vsyncs assumed where the display reports no rate.
const DEFAULT_INTERVAL: Duration = Duration::from_micros(16_667);

pub(crate) struct VsyncSource {
    /// What a display link calls at each vsync: sends the loop its `Vsync`.
    on_vsync: Rc<dyn Fn()>,
    /// The display whose vsyncs these are; `None` before one was first asked for.
    display: Option<MonitorHandle>,
    /// The system's display link for `display`; `None` where it has none.
    display_link: Option<ThisOsDisplayLink>,
    /// The vsyncs where there is no display link.
    timer: VsyncTimer,
    /// Whether a vsync was asked for that has not come yet.
    requested: bool,
}

impl VsyncSource {
    /// A source whose display link calls `on_vsync` at each vsync; a timer's vsyncs are the
    /// loop's to notice, by [`timer_due`](Self::timer_due).
    pub(crate) fn new(on_vsync: impl Fn() + 'static) -> VsyncSource {
        VsyncSource {
            on_vsync: Rc::new(on_vsync),
            display: None,
            display_link: None,
            timer: VsyncTimer::new(DEFAULT_INTERVAL),
            requested: false,
        }
    }

    /// Asks for one call at the next vsync of `display`; asking again before it comes does
    /// nothing. It pauses after a vsync nobody asked for.
    pub(crate) fn request_vsync(&mut self, display: &MonitorHandle) {
        self.follow_display(display);
        self.request_next_vsync();
    }

    /// Asks for one call at the next vsync of the display the last request named; nothing
    /// before a request has named one.
    pub(crate) fn request_next_vsync(&mut self) {
        if self.display.is_none() {
            return;
        }
        self.requested = true;
        self.resume();
    }

    /// The vsyncs come from `display` from now on: one asked for and not come yet comes at its
    /// next vsync.
    pub(crate) fn follow_display(&mut self, display: &MonitorHandle) {
        if self.display.as_ref() == Some(display) {
            return;
        }
        self.start_on(display.clone());
        if self.requested {
            self.resume();
        }
    }

    /// A vsync came: the source runs on to the next if this one was asked for, and pauses if
    /// not.
    pub(crate) fn vsync_came(&mut self) {
        if std::mem::take(&mut self.requested) {
            self.run_on();
        } else {
            self.pause();
        }
    }

    /// When the timer's next vsync is due, for the loop to wait until; `None` on a display link.
    pub(crate) fn timer_due(&self) -> Option<Instant> {
        self.timer.due
    }

    /// Takes the vsyncs of `display` from now on, paused: from its display link where the
    /// system has one, from a timer at its rate otherwise.
    fn start_on(&mut self, display: MonitorHandle) {
        let on_vsync = Rc::clone(&self.on_vsync);
        self.display_link = ThisOsDisplayLink::start(&display, Box::new(move || on_vsync()));
        self.timer = VsyncTimer::new(interval_of(&display));
        self.display = Some(display);
        self.pause();
    }

    fn resume(&mut self) {
        match &self.display_link {
            Some(display_link) => display_link.set_wanted(true),
            None => self.timer.resume(Instant::now()),
        }
    }

    /// A display link runs on by itself; a timer is set for the next vsync.
    fn run_on(&mut self) {
        if self.display_link.is_none() {
            self.timer.advance(Instant::now());
        }
    }

    fn pause(&mut self) {
        match &self.display_link {
            Some(display_link) => display_link.set_wanted(false),
            None => self.timer.pause(),
        }
    }
}

/// Vsyncs from a timer at a display's rate, where the system has no display link for it: they
/// come at the display's rate, but not when the display refreshes.
struct VsyncTimer {
    interval: Duration,
    /// When the next vsync is due; `None` while paused.
    due: Option<Instant>,
}

impl VsyncTimer {
    /// A paused timer, whose vsyncs come `interval` apart.
    fn new(interval: Duration) -> VsyncTimer {
        VsyncTimer {
            interval,
            due: None,
        }
    }

    /// Runs from `now` if paused, with the first vsync one interval on; a running timer keeps
    /// its time.
    fn resume(&mut self, now: Instant) {
        if self.due.is_none() {
            self.due = Some(now + self.interval);
        }
    }

    /// The vsync due came: the next is one interval after it, or one interval from `now` where
    /// the loop came too late for that.
    fn advance(&mut self, now: Instant) {
        let next = self.due.unwrap_or(now) + self.interval;
        self.due = Some(if next < now {
            now + self.interval
        } else {
            next
        });
    }

    fn pause(&mut self) {
        self.due = None;
    }
}

/// The display `window` is on, whose vsyncs its frames take: on macOS, for a hidden or minimized
/// window, the one it was last shown on. The primary display for a window the system names none
/// for, and any display where it names no primary one; `None` where the system knows no display
/// at all.
pub(crate) fn display_of(window: &Window) -> Option<MonitorHandle> {
    window
        .current_monitor()
        .or_else(|| window.primary_monitor())
        .or_else(|| window.available_monitors().next())
}

/// The time between `display`'s vsyncs, from its rate as winit reports it; the default where it
/// reports none.
fn interval_of(display: &MonitorHandle) -> Duration {
    display
        .refresh_rate_millihertz()
        .filter(|&millihertz| millihertz > 0)
        .map(|millihertz| Duration::from_secs_f64(1000.0 / f64::from(millihertz)))
        .unwrap_or(DEFAULT_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_millis(10);

    #[test]
    fn a_resumed_timer_gives_its_first_vsync_one_interval_on() {
        let now = Instant::now();
        let mut timer = VsyncTimer::new(INTERVAL);
        assert_eq!(timer.due, None, "paused at first");
        timer.resume(now);
        assert_eq!(timer.due, Some(now + INTERVAL));
        timer.resume(now + INTERVAL / 2);
        assert_eq!(
            timer.due,
            Some(now + INTERVAL),
            "a running timer keeps its time"
        );
    }

    #[test]
    fn the_next_vsync_is_one_interval_after_the_last() {
        let now = Instant::now();
        let mut timer = VsyncTimer::new(INTERVAL);
        timer.resume(now);
        timer.advance(now + INTERVAL + Duration::from_millis(3));
        assert_eq!(
            timer.due,
            Some(now + 2 * INTERVAL),
            "a late loop does not push the vsyncs later"
        );
    }

    #[test]
    fn a_timer_the_loop_fell_behind_starts_again_from_now() {
        let now = Instant::now();
        let mut timer = VsyncTimer::new(INTERVAL);
        timer.resume(now);
        let late = now + 5 * INTERVAL;
        timer.advance(late);
        assert_eq!(timer.due, Some(late + INTERVAL));
    }

    #[test]
    fn a_paused_timer_is_due_never() {
        let mut timer = VsyncTimer::new(INTERVAL);
        timer.resume(Instant::now());
        timer.pause();
        assert_eq!(timer.due, None);
    }

    #[test]
    fn nothing_is_asked_of_a_source_before_a_display_is_named() {
        let mut source = VsyncSource::new(|| {});
        source.request_next_vsync();
        assert_eq!(source.timer_due(), None);
        source.vsync_came();
        assert_eq!(source.timer_due(), None);
    }
}

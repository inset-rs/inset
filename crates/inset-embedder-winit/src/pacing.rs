//! One frame per refresh of the display, and none while nothing is wanted.
//!
//! The framework asks for a frame; the host draws it at the display's next refresh and
//! no sooner, so an animation that asks again from each frame runs at the display's rate
//! rather than the loop's. The refresh comes from the system's signal where it has one
//! ([`os::Vsync`]), and from a timer at the display's nominal rate elsewhere. A frame
//! drawn for another reason inside an interval, a resize's, counts as that refresh's. The
//! ticks stop after a few refreshes with nothing to draw, so an idle app makes no wakes.

use std::time::{Duration, Instant};

use winit::window::Window;

use crate::os;

/// The refresh interval assumed until a window says what its display's is.
const DEFAULT_REFRESH: Duration = Duration::from_micros(16_667);
/// Refreshes in a row with no frame wanted before the ticks stop.
const IDLE_TICKS: u8 = 2;

pub(crate) struct Pacing {
    /// The system's signal, on the display of the window it was started with, while
    /// frames are wanted.
    vsync: Option<os::Vsync>,
    /// The display's refresh interval, for the timer where the system has no signal.
    refresh: Duration,
    /// When the timer's next tick is due; `None` while the ticks are stopped.
    tick_due: Option<Instant>,
    /// Whether a frame was drawn since the last tick.
    drawn_since_tick: bool,
    /// Ticks in a row with no frame wanted.
    idle_ticks: u8,
}

impl Pacing {
    pub(crate) fn new() -> Pacing {
        Pacing {
            vsync: None,
            refresh: DEFAULT_REFRESH,
            tick_due: None,
            drawn_since_tick: false,
            idle_ticks: 0,
        }
    }

    /// Keeps the ticks coming while a frame is wanted: the system's signal for the
    /// display `window` is on, started on first need with `on_tick` to call at each
    /// refresh, or the timer where the system has no signal.
    pub(crate) fn keep_ticking(&mut self, window: &Window, on_tick: impl Fn() + 'static) {
        self.idle_ticks = 0;
        if self.vsync.is_none() {
            self.vsync = os::Vsync::start(window, Box::new(on_tick));
            self.refresh = refresh_interval(window);
        }
        match &self.vsync {
            Some(vsync) => vsync.set_paused(false),
            None => {
                if self.tick_due.is_none() {
                    self.tick_due = Some(Instant::now() + self.refresh);
                }
            }
        }
    }

    /// When the timer's next tick is due, for the loop to wait until; `None` on a signal
    /// or while the ticks are stopped.
    pub(crate) fn tick_due(&self) -> Option<Instant> {
        self.tick_due
    }

    /// Whether a timer tick is due by `at`.
    pub(crate) fn tick_is_due(&self, at: Instant) -> bool {
        self.tick_due.is_some_and(|due| due <= at)
    }

    /// A refresh, with whether a frame is wanted: answers whether to draw one now, which
    /// is when one is wanted and none was drawn since the last tick. The frame a tick
    /// draws is that refresh's, not the next one's. Stops the ticks after a few refreshes
    /// with nothing wanted.
    pub(crate) fn tick(&mut self, frame_wanted: bool) -> bool {
        let drawn = std::mem::take(&mut self.drawn_since_tick);
        if frame_wanted {
            self.idle_ticks = 0;
        } else {
            self.idle_ticks += 1;
        }
        let stop = self.idle_ticks >= IDLE_TICKS;
        if stop {
            self.idle_ticks = 0;
        }
        match &self.vsync {
            Some(vsync) => vsync.set_paused(stop),
            None => {
                self.tick_due = (!stop).then(|| {
                    let now = Instant::now();
                    let next = self.tick_due.unwrap_or(now) + self.refresh;
                    if next < now { now + self.refresh } else { next }
                });
            }
        }
        frame_wanted && !drawn
    }

    /// A frame was drawn between ticks, for a resize: the next tick has nothing to add.
    pub(crate) fn drawn(&mut self) {
        self.drawn_since_tick = true;
    }

    /// The window the signal was on is gone; the next need starts one on another.
    pub(crate) fn window_gone(&mut self) {
        self.vsync = None;
        self.tick_due = None;
    }
}

/// The refresh interval of the display `window` is on, as winit reports it.
fn refresh_interval(window: &Window) -> Duration {
    window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .filter(|&millihertz| millihertz > 0)
        .map(|millihertz| Duration::from_secs_f64(1000.0 / f64::from(millihertz)))
        .unwrap_or(DEFAULT_REFRESH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wanted_frame_is_drawn_at_every_tick() {
        let mut pacing = Pacing::new();
        assert!(pacing.tick(true));
        assert!(
            pacing.tick(true),
            "the frame a tick draws is that refresh's"
        );
        assert!(pacing.tick(true));
    }

    #[test]
    fn a_frame_drawn_between_ticks_stands_in_for_the_next() {
        let mut pacing = Pacing::new();
        assert!(pacing.tick(true));
        pacing.drawn();
        assert!(!pacing.tick(true));
        assert!(pacing.tick(true));
    }

    #[test]
    fn the_ticks_stop_after_idle_refreshes() {
        let mut pacing = Pacing::new();
        assert!(pacing.tick(true));
        assert!(!pacing.tick(false));
        assert!(pacing.tick_due().is_some());
        assert!(!pacing.tick(false));
        assert!(pacing.tick_due().is_none(), "stopped after IDLE_TICKS");
    }
}

//! A window's share of its screen's display link: the surfaces its last scene placed, whether
//! they want the screen's ticks, and the subscription to that screen's link, which follows the
//! window to another screen and stops while it is out of sight.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::time::Instant;

use dispatch2::{DispatchQueue, MainThreadBound};
use inset_embedder::VsyncObserver;
use objc2::MainThreadMarker;
use winit::monitor::MonitorHandle;
use winit::window::Window;

use super::super::display_links::ScreenDisplayLink;
use super::super::visibility::is_visible;
use super::{CoreAnimationSurface, HELD_BRIEFLY};
use crate::os::DisplayLink;

/// Vsyncs in a row with nobody waiting before a window's surfaces stop wanting their screen's
/// ticks.
const IDLE_VSYNCS: u8 = 2;

/// What one window's surfaces share with their producers, from any thread: the surfaces the
/// last scene placed, whether they want their screen's ticks, and the subscription to that
/// screen's display link.
pub(crate) struct PlacedSurfaces {
    surfaces_and_demand: Mutex<SurfacesAndDemand>,
    /// The window's subscription to the display link of the screen it is on, made on the main
    /// thread when a surface is first placed and moved when the window moves; it holds `None`
    /// where the system has no display link for a screen. Any thread may hold it, only the main
    /// thread may touch the subscription inside, and it is dropped there too.
    screen_link: OnceLock<MainThreadBound<RefCell<Option<ScreenDisplayLink>>>>,
}

/// Under one lock, so that a producer asking for a vsync and the window deciding to pause its
/// ticks cannot miss each other. A surface's own locks are taken inside this one, never around
/// it: a vsync locks this, then each surface's observers, so a surface releases its own lock
/// before it asks its window for a vsync.
struct SurfacesAndDemand {
    surfaces: Vec<Arc<CoreAnimationSurface>>,
    demand: VsyncDemand,
}

/// What falls due at one vsync: the surfaces whose released buffers to hand back, the
/// producers to call, and whether the window wants ticks afterwards.
struct DueAtVsync {
    surfaces: Vec<Arc<CoreAnimationSurface>>,
    observers: Vec<Weak<dyn VsyncObserver>>,
    ticks_still_wanted: bool,
}

impl PlacedSurfaces {
    pub(super) fn new() -> PlacedSurfaces {
        PlacedSurfaces {
            surfaces_and_demand: Mutex::new(SurfacesAndDemand {
                surfaces: Vec::new(),
                demand: VsyncDemand::default(),
            }),
            screen_link: OnceLock::new(),
        }
    }

    /// A producer waits for a vsync: the window wants its screen's ticks from now on, and if
    /// they were paused the main queue restarts them. Any thread.
    pub(super) fn request_vsync(self: &Arc<Self>) {
        let was_unwanted = self.lock().demand.want_ticks();
        if was_unwanted {
            let placed = Arc::clone(self);
            DispatchQueue::main().exec_async(move || placed.resume_display_link());
        }
    }

    /// Asks for a vsync for the producers that wait on a placed surface, if any do.
    fn request_vsync_if_waiting(self: &Arc<Self>) {
        let any_waiting = self
            .lock()
            .surfaces
            .iter()
            .any(|surface| surface.is_waiting_for_vsync());
        if any_waiting {
            self.request_vsync();
        }
    }

    /// The window may have moved to another screen, or come into sight or gone out of it: the
    /// surfaces' ticks come from the screen it is on, and run while it is visible and a producer
    /// waits, and stop while it is out of sight. Nothing to follow before a scene places a
    /// surface in the window: the placement follows it. Main thread.
    pub(super) fn follow_window(
        self: &Arc<Self>,
        window: &Arc<Window>,
        main_thread: MainThreadMarker,
    ) {
        if self.is_empty() {
            return;
        }
        self.follow_screen(window, main_thread);
        if is_visible(window) {
            self.request_vsync_if_waiting();
        } else {
            self.stop_until_visible();
        }
    }

    /// Subscribes the window's surfaces to the display link of the screen `window` is on, or
    /// moves their subscription there from another screen's, keeping whether they want its
    /// ticks: each tick serves the surfaces placed in the window while it is visible. A window on
    /// no screen keeps the link it has. Main thread.
    fn follow_screen(self: &Arc<Self>, window: &Arc<Window>, main_thread: MainThreadMarker) {
        let Some(display) = window.current_monitor() else {
            return;
        };
        let screen_link = self
            .screen_link
            .get_or_init(|| MainThreadBound::new(RefCell::new(None), main_thread))
            .get(main_thread);
        let mut screen_link = screen_link.borrow_mut();
        match screen_link.as_mut() {
            Some(subscription) => subscription.follow(&display),
            None => *screen_link = self.subscribe_to(&display, window),
        }
    }

    /// A subscription to the display link of `display`'s screen, wanting its ticks while a
    /// producer waits.
    fn subscribe_to(
        self: &Arc<Self>,
        display: &MonitorHandle,
        window: &Arc<Window>,
    ) -> Option<ScreenDisplayLink> {
        let Some(subscription) = ScreenDisplayLink::start(display, self.tick_for(window)) else {
            report_no_display_link();
            return None;
        };
        subscription.set_wanted(self.lock().demand.wanted);
        Some(subscription)
    }

    /// What the window's screen calls at each refresh.
    fn tick_for(self: &Arc<Self>, window: &Arc<Window>) -> Box<dyn Fn()> {
        let placed = Arc::downgrade(self);
        let window = Arc::downgrade(window);
        Box::new(move || {
            if let Some(placed) = placed.upgrade() {
                let window_visible = window.upgrade().is_some_and(|window| is_visible(&window));
                placed.on_vsync(Instant::now(), window_visible);
            }
        })
    }

    /// A refresh of the window's screen, on the main thread: serves the placed surfaces while
    /// the window is visible, and stops wanting ticks once nobody waits, or at once while the
    /// window is out of sight, which a notice may not have said yet.
    fn on_vsync(&self, at: Instant, window_visible: bool) {
        if !window_visible {
            self.stop_until_visible();
            return;
        }
        if !self.serve_vsync(at) {
            self.set_ticks_wanted(false);
        }
    }

    /// The window is out of sight: its producers keep waiting, and its ticks stop until it is
    /// visible again or a producer asks again.
    fn stop_until_visible(&self) {
        self.lock().demand.stop();
        self.set_ticks_wanted(false);
    }

    /// On the main queue, which a producer asked to restart the window's ticks.
    fn resume_display_link(&self) {
        self.set_ticks_wanted(true);
    }

    /// Main thread, once a surface has been placed.
    fn set_ticks_wanted(&self, wanted: bool) {
        let Some(screen_link) = self.screen_link.get() else {
            return;
        };
        let main_thread = MainThreadMarker::new().expect("the display link is the main thread's");
        if let Some(subscription) = screen_link.get(main_thread).borrow().as_ref() {
            subscription.set_wanted(wanted);
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.lock().surfaces.is_empty()
    }

    /// Makes `surfaces` the ones placed now, and answers the ones that were.
    pub(super) fn replace_placed(
        &self,
        surfaces: Vec<Arc<CoreAnimationSurface>>,
    ) -> Vec<Arc<CoreAnimationSurface>> {
        std::mem::replace(&mut self.lock().surfaces, surfaces)
    }

    /// A vsync of the window's screen: hands back the buffers the compositor has finished
    /// with, then calls every producer that waited. Answers whether the window keeps wanting
    /// ticks.
    fn serve_vsync(&self, at: Instant) -> bool {
        let due = self.take_due_at_vsync();
        for surface in &due.surfaces {
            surface.release_unused_buffers();
        }
        for observer in due.observers.iter().filter_map(Weak::upgrade) {
            observer.vsync(at);
        }
        due.ticks_still_wanted
    }

    fn take_due_at_vsync(&self) -> DueAtVsync {
        let mut state = self.lock();
        let observers: Vec<_> = state
            .surfaces
            .iter()
            .flat_map(|surface| surface.take_vsync_observers())
            .collect();
        let ticks_still_wanted = state.demand.still_wanted_after_vsync(!observers.is_empty());
        DueAtVsync {
            surfaces: state.surfaces.clone(),
            observers,
            ticks_still_wanted,
        }
    }

    fn lock(&self) -> MutexGuard<'_, SurfacesAndDemand> {
        self.surfaces_and_demand.lock().expect(HELD_BRIEFLY)
    }
}

/// Reports, once, that the system has no display link for a screen.
fn report_no_display_link() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("inset: no display link for a screen; native surfaces get no vsync");
    }
}

/// Whether a window's surfaces want their screen's ticks, and how many in a row served nobody.
#[derive(Default)]
struct VsyncDemand {
    wanted: bool,
    idle_vsyncs: u8,
}

impl VsyncDemand {
    /// A producer waits for a vsync: the ticks are wanted from now on. Answers whether they
    /// were not, so that the caller has them start again.
    fn want_ticks(&mut self) -> bool {
        self.idle_vsyncs = 0;
        !std::mem::replace(&mut self.wanted, true)
    }

    /// A vsync passed, having served a producer or not: answers whether the ticks are still
    /// wanted, which they stop being after `IDLE_VSYNCS` in a row served nobody.
    fn still_wanted_after_vsync(&mut self, served_anyone: bool) -> bool {
        if served_anyone {
            self.idle_vsyncs = 0;
            return self.wanted;
        }
        self.idle_vsyncs += 1;
        if self.idle_vsyncs >= IDLE_VSYNCS {
            self.wanted = false;
            self.idle_vsyncs = 0;
        }
        self.wanted
    }

    /// The ticks are not wanted until a producer asks again.
    fn stop(&mut self) {
        self.wanted = false;
        self.idle_vsyncs = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use inset_embedder::Surface;
    use inset_embedder::ViewId;

    use super::*;

    #[test]
    fn the_ticks_are_wanted_from_a_request_until_idle_vsyncs() {
        let mut demand = VsyncDemand::default();
        assert!(demand.want_ticks(), "unwanted ticks must be started");
        assert!(!demand.want_ticks(), "wanted ones need not");
        assert!(demand.still_wanted_after_vsync(true));
        assert!(demand.still_wanted_after_vsync(false), "one idle vsync");
        assert!(
            !demand.still_wanted_after_vsync(false),
            "unwanted after two"
        );
        assert!(
            demand.want_ticks(),
            "a request after that has them start again"
        );
    }

    struct CountingObserver(AtomicUsize);

    impl VsyncObserver for CountingObserver {
        fn vsync(&self, _at: Instant) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_placed_surface_calls_each_waiting_producer_once_per_vsync() {
        let placed = Arc::new(PlacedSurfaces::new());
        let surface = Arc::new(CoreAnimationSurface::new(ViewId(0), true));
        let observer = Arc::new(CountingObserver(AtomicUsize::new(0)));
        let weak = Arc::downgrade(&observer) as Weak<dyn VsyncObserver>;

        surface.next_vsync(weak.clone());
        assert!(
            !placed.lock().demand.wanted,
            "not placed: no window's link to run"
        );
        surface.attach_to_window(Arc::downgrade(&placed));
        placed.replace_placed(vec![Arc::clone(&surface)]);
        surface.next_vsync(weak);
        assert!(
            placed.lock().demand.wanted,
            "placed: the window's link runs"
        );

        assert!(placed.serve_vsync(Instant::now()));
        assert_eq!(
            observer.0.load(Ordering::SeqCst),
            1,
            "asked twice, called once"
        );
        assert!(placed.serve_vsync(Instant::now()));
        assert!(
            !placed.serve_vsync(Instant::now()),
            "nobody asked again: the link pauses"
        );
        assert_eq!(observer.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_producer_in_a_window_out_of_sight_waits_for_the_first_refresh_after_it_is_seen() {
        let placed = Arc::new(PlacedSurfaces::new());
        let surface = Arc::new(CoreAnimationSurface::new(ViewId(0), true));
        surface.attach_to_window(Arc::downgrade(&placed));
        placed.replace_placed(vec![Arc::clone(&surface)]);
        let observer = Arc::new(CountingObserver(AtomicUsize::new(0)));
        surface.next_vsync(Arc::downgrade(&observer) as Weak<dyn VsyncObserver>);
        assert!(placed.lock().demand.wanted);

        placed.on_vsync(Instant::now(), false);
        assert_eq!(
            observer.0.load(Ordering::SeqCst),
            0,
            "out of sight: it waits"
        );
        assert!(
            !placed.lock().demand.wanted,
            "and the window's ticks stop meanwhile"
        );

        placed.request_vsync_if_waiting();
        assert!(placed.lock().demand.wanted, "seen again: the ticks run");
        placed.on_vsync(Instant::now(), true);
        assert_eq!(observer.0.load(Ordering::SeqCst), 1);
    }
}

//! The platform's vsync waiter on macOS: a wait is served at the next tick of its screen's display
//! link, on the main thread in the run loop's common modes, never through the main loop. Flutter's
//! `FlutterVSyncWaiter` hands its ticks to the main run loop's common modes the same way
//! (`FlutterRunLoop`), so a producer is served at the tick itself, whatever loop AppKit runs and
//! whether or not winit calls the main loop meanwhile. The waiter subscribes to each screen's one display link, which the
//! frames of the views on that screen share (`display_links.rs`), and wants its ticks while a wait
//! on that screen is pending; a wait asked from any thread has the main dispatch queue, which runs
//! in the common modes too, start them.
//!
//! The invariant: a wait is recorded before the main queue is asked to start the ticks, and a tick
//! takes the screen's waits under the same lock, so a wait asked while the ticks pause starts them
//! again and none is lost. The ticks run on for one tick after the last wait served, and pause at
//! a tick nobody waited for.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Instant;

use dispatch2::{DispatchQueue, MainThreadBound};
use inset_embedder::{ScreenId, VsyncObserver, VsyncWaiter};
use objc2::MainThreadMarker;

use super::display_links::{DisplayId, ScreenDisplayLink};

const HELD_BRIEFLY: &str = "the waits are never held across a panic";

/// Waits served by the screens' display links.
pub(crate) struct DisplayLinkVsyncWaiter(Arc<Waiting>);

/// What the threads that wait share with the main thread, which serves them.
struct Waiting {
    waits: Mutex<Waits>,
    /// The waiter's subscription to each screen's display link, made at the first wait on it:
    /// the main thread's alone, and dropped there.
    links: MainThreadBound<RefCell<HashMap<ScreenId, ScreenDisplayLink>>>,
}

/// The waits on each screen not served yet.
#[derive(Default)]
struct Waits(HashMap<ScreenId, Vec<Weak<dyn VsyncObserver>>>);

impl DisplayLinkVsyncWaiter {
    pub(crate) fn new(main_thread: MainThreadMarker) -> DisplayLinkVsyncWaiter {
        DisplayLinkVsyncWaiter(Arc::new(Waiting {
            waits: Mutex::default(),
            links: MainThreadBound::new(RefCell::default(), main_thread),
        }))
    }
}

impl VsyncWaiter for DisplayLinkVsyncWaiter {
    fn next_vsync(&self, screen: ScreenId, observer: Weak<dyn VsyncObserver>) {
        let starts_ticks = self.0.waits().add(screen, observer);
        if starts_ticks {
            let waiting = Arc::clone(&self.0);
            DispatchQueue::main().exec_async(move || waiting.start_ticks(screen));
        }
    }
}

impl Waiting {
    /// The ticks of `screen`'s display link are wanted, on a subscription made at the first wait
    /// on it. The waits on a screen with no display link, one that is not connected, are dropped:
    /// no tick would serve them. Main thread.
    fn start_ticks(self: &Arc<Self>, screen: ScreenId) {
        let main_thread = MainThreadMarker::new().expect("the main queue runs on the main thread");
        let mut links = self.links.get(main_thread).borrow_mut();
        if let Some(link) = links.get(&screen) {
            link.set_ticks_wanted(true);
            return;
        }
        let link = display_of(screen).and_then(|display| {
            ScreenDisplayLink::of_display(display, self.tick_of(screen), main_thread)
        });
        match link {
            Some(link) => {
                link.set_ticks_wanted(true);
                links.insert(screen, link);
            }
            None => drop(self.waits().take(screen)),
        }
    }

    /// What `screen`'s display link calls at each tick.
    fn tick_of(self: &Arc<Self>, screen: ScreenId) -> Box<dyn Fn()> {
        let waiting = Arc::downgrade(self);
        Box::new(move || {
            if let Some(waiting) = waiting.upgrade() {
                waiting.tick(screen);
            }
        })
    }

    /// A tick of `screen`'s display link: every wait on it is served, or, with none, the ticks
    /// pause until the next wait. Main thread.
    fn tick(&self, screen: ScreenId) {
        let due = self.waits().take(screen);
        if due.is_empty() {
            self.pause_ticks(screen);
            return;
        }
        let at = Instant::now();
        for observer in due.iter().filter_map(Weak::upgrade) {
            observer.vsync(at);
        }
    }

    fn pause_ticks(&self, screen: ScreenId) {
        let main_thread = MainThreadMarker::new().expect("a display link ticks on the main thread");
        if let Some(link) = self.links.get(main_thread).borrow().get(&screen) {
            link.set_ticks_wanted(false);
        }
    }

    fn waits(&self) -> MutexGuard<'_, Waits> {
        self.waits.lock().expect(HELD_BRIEFLY)
    }
}

impl Waits {
    /// Records `observer`'s wait on `screen`, once however often it asks before the tick.
    /// Answers whether it is the first wait on the screen since its last tick, which starts the
    /// ticks.
    fn add(&mut self, screen: ScreenId, observer: Weak<dyn VsyncObserver>) -> bool {
        let waiting = self.0.entry(screen).or_default();
        let first = waiting.is_empty();
        if !waiting.iter().any(|known| Weak::ptr_eq(known, &observer)) {
            waiting.push(observer);
        }
        first
    }

    /// The waits on `screen`, which a tick serves, taken out.
    fn take(&mut self, screen: ScreenId) -> Vec<Weak<dyn VsyncObserver>> {
        self.0.remove(&screen).unwrap_or_default()
    }
}

/// The display `screen` is: on macOS a screen's id is its display's number (`screens.rs`).
fn display_of(screen: ScreenId) -> Option<DisplayId> {
    DisplayId::try_from(screen.0).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Observer;

    impl VsyncObserver for Observer {
        fn vsync(&self, _at: Instant) {}
    }

    #[test]
    fn the_first_wait_on_a_screen_since_its_tick_starts_the_ticks_and_each_wait_is_taken_once() {
        let observer = Arc::new(Observer);
        let weak = Arc::downgrade(&observer) as Weak<dyn VsyncObserver>;
        let mut waits = Waits::default();
        assert!(
            waits.add(ScreenId(1), weak.clone()),
            "the first starts the ticks"
        );
        assert!(
            !waits.add(ScreenId(1), weak.clone()),
            "the ticks run already"
        );
        assert!(waits.add(ScreenId(2), weak.clone()), "another screen's own");
        assert_eq!(waits.take(ScreenId(1)).len(), 1, "asked twice, served once");
        assert!(waits.take(ScreenId(1)).is_empty(), "taken");
        assert!(
            waits.add(ScreenId(1), weak),
            "after the tick, the next wait starts them"
        );
    }
}

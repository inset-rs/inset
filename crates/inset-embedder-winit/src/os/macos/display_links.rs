//! The screens' refreshes as signals: one `CADisplayLink` per screen, `NSScreen`'s (macOS 14),
//! shared by everyone who paces by that screen, the frames of the views on it and the waits on it
//! through the platform's vsync waiter (`vsync_waiter.rs`). A screen's link ticks whether or not
//! any window is shown (`2026-09-29-view-display-link-unseen` in the `experiments` folder beside
//! this repository), and is delivered on the main run loop in the common modes, so it keeps
//! ticking through a window's own frame animations and while a menu tracks the pointer.
//!
//! A screen's link is made at the first subscription to that screen, runs while any
//! subscription to it wants its ticks, pauses once none does, and goes with the last one. The
//! invariant a later editor must keep: a screen has at most one link, and it is paused exactly
//! while no subscription to it wants ticks.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel,
};
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::CADisplayLink;
use winit::monitor::MonitorHandle;
use winit::platform::macos::MonitorHandleExtMacOS;

use super::abort_on_panic;
use super::screens::display_id_of;
use crate::os::DisplayLink;

/// A display as Core Graphics numbers it, its `CGDirectDisplayID`: a screen's `NSScreenNumber`
/// in AppKit, and a monitor's native id in winit.
pub(crate) type DisplayId = u32;

/// A subscription to the display link of one screen, as the frames of the views on it and the
/// vsync waiter hold it.
pub(crate) type ScreenDisplayLink = ScreenSubscription<CoreAnimationLink>;

thread_local! {
    /// The screens' links, on the main thread, where AppKit makes and runs them.
    static SCREEN_CLOCKS: Rc<ScreenClocks<CoreAnimationLink>> = Rc::new(ScreenClocks::default());
}

impl DisplayLink for ScreenDisplayLink {
    /// Subscribes `on_tick` to the link of `display`'s screen, wanting its ticks. `None` off the
    /// main thread, before macOS 14, where a screen has no display link, and for a display
    /// AppKit shows no screen for.
    fn start(display: &MonitorHandle, on_tick: Box<dyn Fn()>) -> Option<ScreenDisplayLink> {
        let main_thread = MainThreadMarker::new()?;
        let display = display.native_id();
        let subscription = SCREEN_CLOCKS.with(|clocks| {
            clocks.subscribe(display, on_tick, |tick| {
                CoreAnimationLink::of_screen(display, tick, main_thread)
            })
        })?;
        subscription.set_ticks_wanted(true);
        Some(subscription)
    }

    fn set_wanted(&self, wanted: bool) {
        self.set_ticks_wanted(wanted);
    }
}

impl ScreenDisplayLink {
    /// Subscribes `on_tick` to the link of the screen `display` numbers, not wanting its ticks
    /// yet. `None` before macOS 14, where a screen has no display link, and for a display AppKit
    /// shows no screen for.
    pub(crate) fn of_display(
        display: DisplayId,
        on_tick: Box<dyn Fn()>,
        main_thread: MainThreadMarker,
    ) -> Option<ScreenDisplayLink> {
        SCREEN_CLOCKS.with(|clocks| {
            clocks.subscribe(display, on_tick, |tick| {
                CoreAnimationLink::of_screen(display, tick, main_thread)
            })
        })
    }
}

/// Whether this system gives a screen a display link: macOS 14 and later.
pub(crate) fn screens_have_display_links() -> bool {
    NSScreen::class().responds_to(sel!(displayLinkWithTarget:selector:))
}

/// What a screen's clock needs of the link it runs on.
pub(crate) trait PausableLink: 'static {
    fn set_paused(&self, paused: bool);
}

/// The screens' clocks, one per screen, each shared by the subscriptions to it. Generic over the
/// link, so that its bookkeeping is tested without a window server.
pub(crate) struct ScreenClocks<L: PausableLink> {
    clocks: RefCell<HashMap<DisplayId, Weak<ScreenClock<L>>>>,
}

impl<L: PausableLink> Default for ScreenClocks<L> {
    fn default() -> Self {
        ScreenClocks {
            clocks: RefCell::default(),
        }
    }
}

impl<L: PausableLink> ScreenClocks<L> {
    /// Subscribes `on_tick` to the clock of `display`, not wanting its ticks yet. `make_link`
    /// makes, paused, the link of a screen nobody is subscribed to yet, for the tick it is
    /// handed to call; `None` where it makes none.
    pub(crate) fn subscribe(
        self: &Rc<Self>,
        display: DisplayId,
        on_tick: Box<dyn Fn()>,
        make_link: impl FnOnce(Box<dyn Fn()>) -> Option<L>,
    ) -> Option<ScreenSubscription<L>> {
        let clock = self.clock_of(display, make_link)?;
        let subscriber = Rc::new(Subscriber {
            on_tick,
            wanted: Cell::new(false),
        });
        clock.add(Rc::clone(&subscriber));
        Some(ScreenSubscription { clock, subscriber })
    }

    /// The clock the subscriptions to `display` share, or a new one on the link `make_link`
    /// makes.
    fn clock_of(
        self: &Rc<Self>,
        display: DisplayId,
        make_link: impl FnOnce(Box<dyn Fn()>) -> Option<L>,
    ) -> Option<Rc<ScreenClock<L>>> {
        let shared = self.clocks.borrow().get(&display).and_then(Weak::upgrade);
        if shared.is_some() {
            return shared;
        }
        let clocks = Rc::downgrade(self);
        let link = make_link(Box::new(move || {
            if let Some(clocks) = clocks.upgrade() {
                clocks.tick(display);
            }
        }))?;
        let clock = Rc::new(ScreenClock {
            link,
            subscribers: RefCell::default(),
        });
        self.clocks
            .borrow_mut()
            .insert(display, Rc::downgrade(&clock));
        Some(clock)
    }

    /// A refresh of `display`.
    fn tick(&self, display: DisplayId) {
        let clock = self.clocks.borrow().get(&display).and_then(Weak::upgrade);
        if let Some(clock) = clock {
            clock.tick();
        }
    }
}

/// One screen's link and the subscriptions to it.
struct ScreenClock<L: PausableLink> {
    link: L,
    subscribers: RefCell<Vec<Rc<Subscriber>>>,
}

impl<L: PausableLink> ScreenClock<L> {
    /// Calls every subscription that wants ticks. A call may change who wants them, or end a
    /// subscription, so the list is not held across the calls.
    fn tick(&self) {
        let wanting: Vec<Rc<Subscriber>> = self
            .subscribers
            .borrow()
            .iter()
            .filter(|subscriber| subscriber.wanted.get())
            .cloned()
            .collect();
        for subscriber in wanting {
            (subscriber.on_tick)();
        }
    }

    fn add(&self, subscriber: Rc<Subscriber>) {
        self.subscribers.borrow_mut().push(subscriber);
        self.run_while_wanted();
    }

    fn remove(&self, subscriber: &Rc<Subscriber>) {
        self.subscribers
            .borrow_mut()
            .retain(|known| !Rc::ptr_eq(known, subscriber));
        self.run_while_wanted();
    }

    /// Runs the link while any subscription wants its ticks, and pauses it otherwise.
    fn run_while_wanted(&self) {
        let wanted = self
            .subscribers
            .borrow()
            .iter()
            .any(|subscriber| subscriber.wanted.get());
        self.link.set_paused(!wanted);
    }
}

struct Subscriber {
    on_tick: Box<dyn Fn()>,
    wanted: Cell<bool>,
}

/// A subscription to one screen's clock: its calls come at each refresh while it wants ticks.
/// Main thread; it ends when dropped.
pub(crate) struct ScreenSubscription<L: PausableLink> {
    clock: Rc<ScreenClock<L>>,
    subscriber: Rc<Subscriber>,
}

impl<L: PausableLink> ScreenSubscription<L> {
    /// Wants the ticks from now on, or no longer.
    pub(crate) fn set_ticks_wanted(&self, wanted: bool) {
        self.subscriber.wanted.set(wanted);
        self.clock.run_while_wanted();
    }
}

impl<L: PausableLink> Drop for ScreenSubscription<L> {
    fn drop(&mut self) {
        self.clock.remove(&self.subscriber);
    }
}

/// A screen's `CADisplayLink` on the main run loop, calling its target at each refresh;
/// invalidated when dropped.
pub(crate) struct CoreAnimationLink {
    link: Retained<CADisplayLink>,
    _target: Retained<Ticker>,
}

impl CoreAnimationLink {
    /// The link of the screen `display` numbers, paused, calling `on_tick` at each refresh.
    /// `None` before macOS 14, where a screen has no display link, and for a display AppKit
    /// shows no screen for.
    fn of_screen(
        display: DisplayId,
        on_tick: Box<dyn Fn()>,
        main_thread: MainThreadMarker,
    ) -> Option<CoreAnimationLink> {
        if !screens_have_display_links() {
            return None;
        }
        let screen = screen_numbered(display, main_thread)?;
        let target = Ticker::new(main_thread, on_tick);
        // SAFETY: `tick:` is defined on `Ticker` below with the one-argument signature a display
        // link calls its target with, and the target outlives the link.
        let link = unsafe { screen.displayLinkWithTarget_selector(&target, sel!(tick:)) };
        link.setPaused(true);
        // SAFETY: the main run loop, added to on the main thread; the mode is a constant
        // Foundation exports for the life of the process.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Some(CoreAnimationLink {
            link,
            _target: target,
        })
    }
}

impl PausableLink for CoreAnimationLink {
    fn set_paused(&self, paused: bool) {
        self.link.setPaused(paused);
    }
}

impl Drop for CoreAnimationLink {
    fn drop(&mut self) {
        self.link.invalidate();
    }
}

/// The screen AppKit shows the display `display` numbers as.
fn screen_numbered(
    display: DisplayId,
    main_thread: MainThreadMarker,
) -> Option<Retained<NSScreen>> {
    NSScreen::screens(main_thread)
        .iter()
        .find(|screen| display_id_of(screen) == Some(display))
}

struct Tick {
    on_tick: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Ticker` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetDisplayLinkTicker"]
    #[ivars = Tick]
    struct Ticker;

    impl Ticker {
        #[unsafe(method(tick:))]
        fn tick(&self, _link: &CADisplayLink) {
            // A tick may end the last subscription to its screen, which lets go of this target
            // and its `on_tick`; both live until the tick returns.
            let _alive = self.retain();
            abort_on_panic(|| (self.ivars().on_tick)());
        }
    }
);

impl Ticker {
    fn new(mtm: MainThreadMarker, on_tick: Box<dyn Fn()>) -> Retained<Ticker> {
        let this = mtm.alloc::<Ticker>().set_ivars(Tick { on_tick });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A link as a test sees it: whether it is paused, whether it is gone, and its tick.
    struct LinkState {
        display: DisplayId,
        paused: Cell<bool>,
        gone: Cell<bool>,
        tick: Box<dyn Fn()>,
    }

    struct FakeLink(Rc<LinkState>);

    impl PausableLink for FakeLink {
        fn set_paused(&self, paused: bool) {
            self.0.paused.set(paused);
        }
    }

    impl Drop for FakeLink {
        fn drop(&mut self) {
            self.0.gone.set(true);
        }
    }

    /// Every link a test's clocks made, in order.
    #[derive(Default)]
    struct MadeLinks(RefCell<Vec<Rc<LinkState>>>);

    impl MadeLinks {
        /// Makes a paused link for `display`, as a screen's link is made.
        fn for_screen(
            &self,
            display: DisplayId,
        ) -> impl FnOnce(Box<dyn Fn()>) -> Option<FakeLink> + '_ {
            move |tick| {
                let state = Rc::new(LinkState {
                    display,
                    paused: Cell::new(true),
                    gone: Cell::new(false),
                    tick,
                });
                self.0.borrow_mut().push(Rc::clone(&state));
                Some(FakeLink(state))
            }
        }

        fn count(&self) -> usize {
            self.0.borrow().len()
        }

        /// The last link made for `display`.
        fn of(&self, display: DisplayId) -> Rc<LinkState> {
            let links = self.0.borrow();
            let link = links.iter().rev().find(|link| link.display == display);
            Rc::clone(link.expect("a link was made for the screen"))
        }
    }

    fn counting(count: &Rc<Cell<u32>>) -> Box<dyn Fn()> {
        let count = Rc::clone(count);
        Box::new(move || count.set(count.get() + 1))
    }

    fn ignoring() -> Box<dyn Fn()> {
        Box::new(|| {})
    }

    #[test]
    fn the_subscriptions_to_a_screen_share_one_link() {
        let clocks = Rc::new(ScreenClocks::default());
        let links = MadeLinks::default();
        let _frames = clocks.subscribe(1, ignoring(), links.for_screen(1));
        let _waits = clocks.subscribe(1, ignoring(), links.for_screen(1));
        let _elsewhere = clocks.subscribe(2, ignoring(), links.for_screen(2));
        assert_eq!(links.count(), 2, "one link per screen");
    }

    #[test]
    fn a_screen_link_runs_while_a_subscription_wants_ticks_and_pauses_when_none_does() {
        let clocks = Rc::new(ScreenClocks::default());
        let links = MadeLinks::default();
        let frames = clocks
            .subscribe(1, ignoring(), links.for_screen(1))
            .unwrap();
        let waits = clocks
            .subscribe(1, ignoring(), links.for_screen(1))
            .unwrap();
        let link = links.of(1);
        assert!(link.paused.get(), "nobody wants ticks yet");

        frames.set_ticks_wanted(true);
        waits.set_ticks_wanted(true);
        frames.set_ticks_wanted(false);
        assert!(!link.paused.get(), "the waits still want them");
        waits.set_ticks_wanted(false);
        assert!(link.paused.get());
    }

    #[test]
    fn a_tick_calls_only_the_subscriptions_that_want_it() {
        let clocks = Rc::new(ScreenClocks::default());
        let links = MadeLinks::default();
        let (wanting, idle) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let first = clocks
            .subscribe(1, counting(&wanting), links.for_screen(1))
            .unwrap();
        let _second = clocks.subscribe(1, counting(&idle), links.for_screen(1));
        first.set_ticks_wanted(true);

        (links.of(1).tick)();
        assert_eq!((wanting.get(), idle.get()), (1, 0));
    }

    #[test]
    fn a_screen_link_goes_with_its_last_subscription() {
        let clocks = Rc::new(ScreenClocks::default());
        let links = MadeLinks::default();
        let first = clocks.subscribe(1, ignoring(), links.for_screen(1));
        let second = clocks.subscribe(1, ignoring(), links.for_screen(1));
        drop(first);
        assert!(!links.of(1).gone.get(), "one subscription is left");
        drop(second);
        assert!(links.of(1).gone.get());

        let _again = clocks.subscribe(1, ignoring(), links.for_screen(1));
        assert_eq!(links.count(), 2, "a new link for a new subscription");
    }
}

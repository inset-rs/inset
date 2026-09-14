//! The display's refresh as a signal: a `CADisplayLink` on a window's view, delivered on
//! the main run loop in the common modes so it keeps ticking through the window's own
//! frame animations.

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::CADisplayLink;
use winit::window::Window;

use super::ns_view;

/// A display link ticking for a window; invalidated when dropped.
pub(crate) struct Vsync {
    link: Retained<CADisplayLink>,
    _target: Retained<Ticker>,
}

impl Vsync {
    /// Starts calling `on_tick` at each refresh of the display `window` is on. `None` where
    /// the system has no display link for a view, before macOS 14.
    pub(crate) fn start(window: &Window, on_tick: Box<dyn Fn()>) -> Option<Vsync> {
        let mtm = MainThreadMarker::new()?;
        let view = ns_view(window)?;
        if !view.respondsToSelector(sel!(displayLinkWithTarget:selector:)) {
            return None;
        }
        let target = Ticker::new(mtm, on_tick);
        // SAFETY: `tick:` is defined on `Ticker` below with the one-argument signature a
        // display link calls its target with, and the target outlives the link.
        let link = unsafe { view.displayLinkWithTarget_selector(&target, sel!(tick:)) };
        // SAFETY: the main run loop, added to on the main thread; the mode is a constant
        // Foundation exports for the life of the process.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Some(Vsync {
            link,
            _target: target,
        })
    }

    /// Stops the ticks, or starts them again.
    pub(crate) fn set_paused(&self, paused: bool) {
        self.link.setPaused(paused);
    }
}

impl Drop for Vsync {
    fn drop(&mut self) {
        self.link.invalidate();
    }
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
            (self.ivars().on_tick)();
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

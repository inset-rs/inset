//! The display link the host's frames and the producers' waits are served at, as Flutter's
//! `VSyncClient` keeps one: a `CADisplayLink` on the main run loop in its common modes, so it ticks
//! while a scroll tracks a finger too, asking for the screen's highest rate, which ProMotion screens
//! give an app whose `Info.plist` sets `CADisableMinimumFrameDurationOnPhone`, as cargo waft's does.
//! Each tick hands the host when it came and when the picture drawn for it reaches the screen
//! (`targetTimestamp`), on the host's clock. The host pauses it while nothing waits on it.

// Core Animation's display link and its target need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use objc2_ui_kit::UIScreen;
use waft_embedder_apple::media_time::instant_of;

use crate::host::{HostEvent, deliver};

/// The host's display link, paused until a frame is asked for.
pub(crate) struct DisplayLink {
    link: Retained<CADisplayLink>,
}

impl DisplayLink {
    pub(crate) fn new(main_thread: MainThreadMarker) -> DisplayLink {
        let target = TickTarget::new(main_thread);
        // SAFETY: `tick:` is defined on `TickTarget` below with the one-argument signature a display
        // link calls; the link retains its target for its own life.
        let link = unsafe { CADisplayLink::displayLinkWithTarget_selector(&target, sel!(tick:)) };
        #[allow(deprecated)]
        let fastest = UIScreen::mainScreen(main_thread).maximumFramesPerSecond() as f32;
        link.setPreferredFrameRateRange(CAFrameRateRange {
            minimum: fastest,
            maximum: fastest,
            preferred: fastest,
        });
        link.setPaused(true);
        // SAFETY: the main run loop and a mode constant Foundation exports for the life of the
        // process.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        DisplayLink { link }
    }

    pub(crate) fn set_paused(&self, paused: bool) {
        if self.link.isPaused() != paused {
            self.link.setPaused(paused);
        }
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `TickTarget` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftDisplayLinkTarget"]
    struct TickTarget;

    unsafe impl NSObjectProtocol for TickTarget {}

    impl TickTarget {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &CADisplayLink) {
            deliver(HostEvent::Vsync {
                at: instant_of(link.timestamp()),
                target: instant_of(link.targetTimestamp()),
            });
        }
    }
);

impl TickTarget {
    fn new(main_thread: MainThreadMarker) -> Retained<TickTarget> {
        let this = main_thread.alloc::<TickTarget>().set_ivars(());
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

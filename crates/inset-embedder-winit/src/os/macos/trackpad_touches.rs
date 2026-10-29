//! The fingers coming down on a trackpad, which winit does not report (`TrackpadTouches`).
//!
//! AppKit sends `touchesBeganWithEvent:` to a view that allows indirect touches, and winit's view,
//! which does not answer it, passes it on to its next responder. The embedder puts a responder of
//! its own there, as Flutter's macOS view controller follows its view in the responder chain and
//! answers the touch (`FlutterViewController.mm`, `viewDidLoad` and `touchesBeganWithEvent:`).
//!
//! AppKit does not retain a next responder, so the observer keeps the responder, and the invariant
//! a later editor must keep: the observer takes it out of the view's chain as it is dropped, so
//! AppKit never reaches it freed.

use std::time::Instant;

use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSResponder, NSTouchTypeMask, NSView};
use winit::window::Window;

use super::{AppKit, abort_on_panic, ns_view};
use crate::os::TrackpadTouches;

impl TrackpadTouches for AppKit {
    type Observer = TouchObserver;

    fn observe_trackpad_touches(
        window: &dyn Window,
        on_touch: Box<dyn Fn(Instant)>,
    ) -> Option<TouchObserver> {
        TouchObserver::observe(window, on_touch)
    }
}

/// A responder behind one window's view, which reports the touches the view passes on.
pub(crate) struct TouchObserver {
    view: Weak<NSView>,
    responder: Retained<TouchResponder>,
    /// The view's next responder before, which it gets back.
    previous: Option<Weak<NSResponder>>,
}

impl TouchObserver {
    /// Puts the responder behind `window`'s view, which allows the trackpad's touches, resting
    /// ones included, as Flutter's view does. `None` off the main thread, and for a window with no
    /// view.
    fn observe(window: &dyn Window, on_touch: Box<dyn Fn(Instant)>) -> Option<TouchObserver> {
        let main_thread = MainThreadMarker::new()?;
        let view = ns_view(window)?;
        let responder = TouchResponder::new(main_thread, on_touch);
        // SAFETY: the view's next responder, its window, is alive while the view is in it.
        let previous = unsafe { view.nextResponder() };
        // SAFETY: the responder is kept by the observer, which takes it out of the chain as it is
        // dropped; `previous` stays the view's window's, which AppKit keeps.
        unsafe {
            responder.setNextResponder(previous.as_deref());
            view.setNextResponder(Some(&responder));
        }
        view.setAllowedTouchTypes(NSTouchTypeMask::Indirect);
        view.setWantsRestingTouches(true);
        Some(TouchObserver {
            view: Weak::from_retained(&view),
            responder,
            previous: previous.as_ref().map(Weak::from_retained),
        })
    }
}

impl Drop for TouchObserver {
    /// Gives the view its former next responder back, unless AppKit has set another since.
    fn drop(&mut self) {
        let Some(view) = self.view.load() else {
            return;
        };
        // SAFETY: the view's next responder is this observer's responder, still kept, or one
        // AppKit set since and keeps.
        let next = unsafe { view.nextResponder() };
        let ours: &NSResponder = &self.responder;
        if next.as_deref().is_some_and(|next| std::ptr::eq(next, ours)) {
            let previous = self.previous.as_ref().and_then(Weak::load);
            // SAFETY: the former next responder is alive, being loaded, and kept by AppKit.
            unsafe { view.setNextResponder(previous.as_deref()) };
        }
    }
}

define_class!(
    // SAFETY: NSResponder has no subclassing requirements, and `TouchResponder` has no `Drop`.
    #[unsafe(super(NSResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetTrackpadTouchResponder"]
    #[ivars = Box<dyn Fn(Instant)>]
    pub(crate) struct TouchResponder;

    impl TouchResponder {
        /// Reports fingers coming down on the trackpad, at the moment AppKit delivers them.
        #[unsafe(method(touchesBeganWithEvent:))]
        fn touches_began(&self, event: &NSEvent) {
            abort_on_panic(|| {
                if event.allTouches().anyObject().is_some() {
                    (self.ivars())(Instant::now());
                }
            });
        }
    }
);

impl TouchResponder {
    fn new(main_thread: MainThreadMarker, on_touch: Box<dyn Fn(Instant)>) -> Retained<Self> {
        let this = TouchResponder::alloc(main_thread).set_ivars(on_touch);
        // SAFETY: plain NSResponder initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

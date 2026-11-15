//! The `UIView` a scene's window shows a view of the app's in: its layer holds the view's picture
//! (`picture.rs`), its touches are the view's pointers (`touches.rs`), and it is the window's first
//! responder, so the hardware keyboard's presses reach its view controller, its next responder.

// A UIKit subclass needs `unsafe` to define and initialise; this module writes it for it.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::CGRect;
use objc2_foundation::NSSet;
use objc2_ui_kit::{UIEvent, UIResponder, UITouch, UIView};
use waft_embedder::ViewId;

use crate::host::{HostEvent, deliver};

pub(crate) struct Showing {
    view: ViewId,
}

define_class!(
    // SAFETY: UIView has no subclassing requirements, and `WaftView` has no `Drop`.
    #[unsafe(super(UIView, UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftView"]
    #[ivars = Showing]
    pub(crate) struct WaftView;

    unsafe impl NSObjectProtocol for WaftView {}

    impl WaftView {
        #[unsafe(method(canBecomeFirstResponder))]
        fn can_become_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.hand_on(touches);
        }

        #[unsafe(method(touchesMoved:withEvent:))]
        fn touches_moved(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.hand_on(touches);
        }

        #[unsafe(method(touchesEnded:withEvent:))]
        fn touches_ended(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.hand_on(touches);
        }

        #[unsafe(method(touchesCancelled:withEvent:))]
        fn touches_cancelled(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.hand_on(touches);
        }
    }
);

impl WaftView {
    /// The `UIView` of the view numbered `view`, which takes every finger at once.
    pub(crate) fn new(main_thread: MainThreadMarker, view: ViewId) -> Retained<WaftView> {
        let this = main_thread.alloc().set_ivars(Showing { view });
        // SAFETY: `initWithFrame:` of an allocated view; its window sizes it.
        let this: Retained<WaftView> =
            unsafe { msg_send![super(this), initWithFrame: CGRect::default()] };
        this.setMultipleTouchEnabled(true);
        this
    }

    /// Hands the host what `touches` did, each in its own phase.
    fn hand_on(&self, touches: &NSSet<UITouch>) {
        let reports = crate::touches::reports_of(touches, self, self.ivars().view);
        if !reports.is_empty() {
            deliver(HostEvent::Pointer(reports));
        }
    }
}

//! The root view controller of a scene's window, as Flutter's `FlutterViewController` is: it tells
//! the host when its view was laid out again, a new size, safe area or scale among it
//! (`viewDidLayoutSubviews`, `viewSafeAreaInsetsDidChange`), when the user's appearance changed
//! (`traitCollectionDidChange:`), and hands the host the hardware keyboard's presses, giving UIKit
//! the ones the framework leaves (`pressesBegan:withEvent:`, `pressesEnded:withEvent:`), as
//! Flutter's `FlutterKeyboardManager` hands its unhandled presses back to `super`.
//!
//! The presses reach it from its view, the window's first responder, whose next responder it is.

// A UIKit subclass, and the super calls its overrides make, need `unsafe`; this module writes it.
#![allow(unsafe_code)]

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::NSSet;
use objc2_ui_kit::{
    UIPress, UIPressesEvent, UIResponder, UITraitCollection, UIView, UIViewController,
};
use waft_embedder::ViewId;

use crate::host::{HostEvent, deliver};
use crate::keys::KeyResponder;
use crate::presses::{PressPhase, key_data_of};

pub(crate) struct Controlling {
    view: ViewId,
    keys: RefCell<KeyResponder>,
}

define_class!(
    // SAFETY: UIViewController has no subclassing requirements, and `WaftViewController` has no
    // `Drop`.
    #[unsafe(super(UIViewController, UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftViewController"]
    #[ivars = Controlling]
    pub(crate) struct WaftViewController;

    unsafe impl NSObjectProtocol for WaftViewController {}

    impl WaftViewController {
        #[unsafe(method(viewDidLayoutSubviews))]
        fn view_did_layout_subviews(&self) {
            // SAFETY: the super call of the method overridden here.
            let _: () = unsafe { msg_send![super(self), viewDidLayoutSubviews] };
            deliver(HostEvent::LaidOut(self.ivars().view));
        }

        #[unsafe(method(viewSafeAreaInsetsDidChange))]
        fn view_safe_area_insets_did_change(&self) {
            // SAFETY: the super call of the method overridden here.
            let _: () = unsafe { msg_send![super(self), viewSafeAreaInsetsDidChange] };
            deliver(HostEvent::LaidOut(self.ivars().view));
        }

        #[unsafe(method(traitCollectionDidChange:))]
        fn trait_collection_did_change(&self, previous: Option<&UITraitCollection>) {
            // SAFETY: the super call of the method overridden here.
            let _: () = unsafe { msg_send![super(self), traitCollectionDidChange: previous] };
            deliver(HostEvent::SettingsChanged);
        }

        #[unsafe(method(pressesBegan:withEvent:))]
        fn presses_began(&self, presses: &NSSet<UIPress>, event: Option<&UIPressesEvent>) {
            let left = self.hand_on(presses, PressPhase::Began);
            if !left.is_empty() {
                let left = NSSet::from_retained_slice(&left);
                // SAFETY: the super call of the method overridden here, with the presses unhandled.
                let _: () = unsafe { msg_send![super(self), pressesBegan: &*left, withEvent: event] };
            }
        }

        #[unsafe(method(pressesEnded:withEvent:))]
        fn presses_ended(&self, presses: &NSSet<UIPress>, event: Option<&UIPressesEvent>) {
            let left = self.hand_on(presses, PressPhase::Ended);
            if !left.is_empty() {
                let left = NSSet::from_retained_slice(&left);
                // SAFETY: the super call of the method overridden here, with the presses unhandled.
                let _: () = unsafe { msg_send![super(self), pressesEnded: &*left, withEvent: event] };
            }
        }

        #[unsafe(method(pressesCancelled:withEvent:))]
        fn presses_cancelled(&self, presses: &NSSet<UIPress>, event: Option<&UIPressesEvent>) {
            let left = self.hand_on(presses, PressPhase::Ended);
            if !left.is_empty() {
                let left = NSSet::from_retained_slice(&left);
                // SAFETY: the super call of the method overridden here, with the presses unhandled.
                let _: () =
                    unsafe { msg_send![super(self), pressesCancelled: &*left, withEvent: event] };
            }
        }
    }
);

impl WaftViewController {
    /// The controller of the view numbered `view`, whose `UIView` is `ui_view`.
    pub(crate) fn new(
        main_thread: MainThreadMarker,
        view: ViewId,
        ui_view: &UIView,
    ) -> Retained<WaftViewController> {
        let this = main_thread.alloc().set_ivars(Controlling {
            view,
            keys: RefCell::new(KeyResponder::default()),
        });
        // SAFETY: `init` of an allocated view controller, which loads no nib.
        let this: Retained<WaftViewController> = unsafe { msg_send![super(this), init] };
        this.setView(Some(ui_view));
        this
    }

    /// Hands the framework each of `presses` that is a key's, in `phase`, and answers those it left:
    /// a press of no key, and a key the framework did not handle.
    fn hand_on(&self, presses: &NSSet<UIPress>, phase: PressPhase) -> Vec<Retained<UIPress>> {
        presses
            .iter()
            .filter(|press| {
                let data = key_data_of(
                    press,
                    phase,
                    &mut self.ivars().keys.borrow_mut(),
                    self.mtm(),
                );
                match data {
                    Some(Some(data)) => !crate::host::key_data(data),
                    // A key the responder answered itself, held already or never down: handled.
                    Some(None) => false,
                    None => true,
                }
            })
            .collect()
    }
}

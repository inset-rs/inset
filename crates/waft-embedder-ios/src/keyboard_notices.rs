//! UIKit's keyboard notices, which Flutter's `FlutterViewController` hears
//! (`keyboardWillShowNotification:`, `keyboardWillChangeFrame:`, `keyboardWillBeHidden:`), handed to
//! the host with what their user info says of where the keyboard ends (`keyboard_inset.rs`).

// UIKit's notices and the values in their user info need `unsafe`; this module writes it.
#![allow(unsafe_code)]

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, Sel};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSNotificationName, NSNumber, NSObjectProtocol, NSValue,
};
use objc2_ui_kit::{
    NSValueUIGeometryExtensions, UIKeyboardFrameEndUserInfoKey, UIKeyboardIsLocalUserInfoKey,
    UIKeyboardWillChangeFrameNotification, UIKeyboardWillHideNotification,
    UIKeyboardWillShowNotification,
};

use crate::host::{HostEvent, deliver};
use crate::keyboard_inset::{KeyboardChange, KeyboardNotice, ScreenRect};

thread_local! {
    /// The observer, kept for the app's life. Main thread.
    static OBSERVER: RefCell<Option<Retained<KeyboardObserver>>> = const { RefCell::new(None) };
}

/// Hears the keyboard's notices for the app's life.
pub(crate) fn observe(main_thread: MainThreadMarker) {
    let observer = KeyboardObserver::new(main_thread);
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: constants UIKit exports for the life of the process.
    let notices: [(&NSNotificationName, Sel); 3] = unsafe {
        [
            (UIKeyboardWillShowNotification, sel!(willShow:)),
            (
                UIKeyboardWillChangeFrameNotification,
                sel!(willChangeFrame:),
            ),
            (UIKeyboardWillHideNotification, sel!(willHide:)),
        ]
    };
    for (name, selector) in notices {
        // SAFETY: each selector is defined on `KeyboardObserver` below with the one-argument
        // signature a notice is sent with; the observer is kept for the app's life.
        unsafe {
            center.addObserver_selector_name_object(&observer, selector, Some(name), None);
        }
    }
    OBSERVER.set(Some(observer));
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `KeyboardObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftKeyboardObserver"]
    struct KeyboardObserver;

    unsafe impl NSObjectProtocol for KeyboardObserver {}

    impl KeyboardObserver {
        #[unsafe(method(willShow:))]
        fn will_show(&self, notice: &NSNotification) {
            hand_on(notice, KeyboardNotice::Show);
        }

        #[unsafe(method(willChangeFrame:))]
        fn will_change_frame(&self, notice: &NSNotification) {
            hand_on(notice, KeyboardNotice::ChangeFrame);
        }

        #[unsafe(method(willHide:))]
        fn will_hide(&self, notice: &NSNotification) {
            hand_on(notice, KeyboardNotice::Hide);
        }
    }
);

impl KeyboardObserver {
    fn new(main_thread: MainThreadMarker) -> Retained<KeyboardObserver> {
        let this = main_thread.alloc::<KeyboardObserver>().set_ivars(());
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

fn hand_on(notice: &NSNotification, kind: KeyboardNotice) {
    deliver(HostEvent::Keyboard(change_of(notice, kind)));
}

/// What `notice` says: where the keyboard ends, nowhere where it says not, and whether it is this
/// app's, as it is where it does not say.
fn change_of(notice: &NSNotification, kind: KeyboardNotice) -> KeyboardChange {
    let info = notice.userInfo();
    let value_of = |key| info.as_ref()?.objectForKey(key);
    // SAFETY: constants UIKit exports for the life of the process.
    let (frame_key, local_key) =
        unsafe { (UIKeyboardFrameEndUserInfoKey, UIKeyboardIsLocalUserInfoKey) };
    let end_frame = value_of(frame_key)
        .and_then(|value| value.downcast::<NSValue>().ok())
        // SAFETY: the end frame's value holds a `CGRect`, as UIKit documents it.
        .map(|value| unsafe { value.CGRectValue() })
        .map_or(ScreenRect::default(), |rect| ScreenRect {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.width,
            height: rect.size.height,
        });
    let is_local = value_of(local_key)
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .is_none_or(|local| local.boolValue());
    KeyboardChange {
        notice: kind,
        end_frame,
        is_local,
    }
}

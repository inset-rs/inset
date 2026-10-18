//! What AppKit and Foundation say changed about the machine, which Flutter's macOS embedder hears
//! to tell the engine (`FlutterEngine.mm`): the screens' parameters, for which it sends the
//! displays again (`updateDisplayConfig`), and the user's locale, for which it sends the
//! languages again (`sendUserLocales`).
//!
//! Both reach the observer on the main thread, where Flutter's embedder observes them too, from
//! the system's own dispatch, which can be inside a winit callback, so they only queue the change
//! for the event loop.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSApplicationDidChangeScreenParametersNotification;
use objc2_foundation::{
    NSCurrentLocaleDidChangeNotification, NSNotification, NSNotificationCenter,
};

use super::{AppKit, abort_on_panic};
use crate::os::SystemChanges;
use crate::system_changes::SystemChange;

impl SystemChanges for AppKit {
    /// Observes the two notices for the life of the process, as Flutter's engine does from its
    /// start.
    fn send_system_changes(send: Box<dyn Fn(SystemChange)>) {
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let observer = ChangeObserver::new(main_thread, send);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: each selector is defined on `ChangeObserver` below with the one-argument
        // signature a notice is sent with; the names are constants AppKit and Foundation export
        // for the life of the process; and the observer is kept for the life of the process, so
        // it is never removed.
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(screenParametersDidChange:),
                Some(NSApplicationDidChangeScreenParametersNotification),
                None,
            );
            center.addObserver_selector_name_object(
                &observer,
                sel!(currentLocaleDidChange:),
                Some(NSCurrentLocaleDidChangeNotification),
                None,
            );
        }
        OBSERVER.set(Some(observer));
    }
}

thread_local! {
    /// The observer of the notices, kept for the life of the process. Main thread.
    static OBSERVER: RefCell<Option<Retained<ChangeObserver>>> = const { RefCell::new(None) };
}

/// Where the observer sends what it heard.
struct SendChange(Box<dyn Fn(SystemChange)>);

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `ChangeObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetSystemChangeObserver"]
    #[ivars = SendChange]
    struct ChangeObserver;

    impl ChangeObserver {
        #[unsafe(method(screenParametersDidChange:))]
        fn screen_parameters_did_change(&self, _notice: &NSNotification) {
            abort_on_panic(|| (self.ivars().0)(SystemChange::Screens));
        }

        #[unsafe(method(currentLocaleDidChange:))]
        fn current_locale_did_change(&self, _notice: &NSNotification) {
            abort_on_panic(|| (self.ivars().0)(SystemChange::Locales));
        }
    }
);

impl ChangeObserver {
    fn new(mtm: MainThreadMarker, send: Box<dyn Fn(SystemChange)>) -> Retained<ChangeObserver> {
        let this = mtm.alloc::<ChangeObserver>().set_ivars(SendChange(send));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

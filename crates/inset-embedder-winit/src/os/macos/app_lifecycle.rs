//! What AppKit says of the application as a whole, which Flutter's macOS embedder hears to report
//! its lifecycle (`FlutterAppLifecycleRegistrar`): that it is about to become active or to stop
//! being so, and that whether any of it is visible changed.
//!
//! AppKit posts these from its own dispatch, which can be inside a winit callback, so they only
//! queue the notice for the event loop, with what belongs to the moment AppKit posts it read
//! there: whether a window is visible, whether the application is.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationDidChangeOcclusionStateNotification, NSApplicationOcclusionState,
    NSApplicationWillBecomeActiveNotification, NSApplicationWillResignActiveNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter};

use super::{AppKit, abort_on_panic};
use crate::lifecycle::LifecycleNotice;
use crate::os::ApplicationLifecycle;

impl ApplicationLifecycle for AppKit {
    /// Observes the three notices for the life of the process, as Flutter's registrar does from
    /// its delegate's start. This must run before the loop does, so the notices of the launch
    /// are heard.
    fn send_lifecycle_notices(send: Box<dyn Fn(LifecycleNotice)>) {
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let observer = LifecycleObserver::new(main_thread, send);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: each selector is defined on `LifecycleObserver` below with the one-argument
        // signature a notice is sent with; the names are constants AppKit exports for the life of
        // the process; and the observer is kept for the life of the process, so it is never
        // removed.
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(applicationWillBecomeActive:),
                Some(NSApplicationWillBecomeActiveNotification),
                None,
            );
            center.addObserver_selector_name_object(
                &observer,
                sel!(applicationWillResignActive:),
                Some(NSApplicationWillResignActiveNotification),
                None,
            );
            center.addObserver_selector_name_object(
                &observer,
                sel!(applicationDidChangeOcclusionState:),
                Some(NSApplicationDidChangeOcclusionStateNotification),
                None,
            );
        }
        OBSERVER.set(Some(observer));
    }
}

thread_local! {
    /// The observer of the notices, kept for the life of the process. Main thread.
    static OBSERVER: RefCell<Option<Retained<LifecycleObserver>>> = const { RefCell::new(None) };
}

/// Whether any window of the application is ordered in; a minimized one is not.
fn any_window_visible(main_thread: MainThreadMarker) -> bool {
    NSApplication::sharedApplication(main_thread)
        .windows()
        .iter()
        .any(|window| window.isVisible())
}

/// Whether any part of the application is visible on screen.
fn application_visible(main_thread: MainThreadMarker) -> bool {
    NSApplication::sharedApplication(main_thread)
        .occlusionState()
        .contains(NSApplicationOcclusionState::Visible)
}

/// Where the observer sends what it heard.
struct SendNotice(Box<dyn Fn(LifecycleNotice)>);

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `LifecycleObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetAppLifecycleObserver"]
    #[ivars = SendNotice]
    struct LifecycleObserver;

    impl LifecycleObserver {
        #[unsafe(method(applicationWillBecomeActive:))]
        fn application_will_become_active(&self, _notice: &NSNotification) {
            abort_on_panic(|| {
                let any_window_visible = any_window_visible(self.mtm());
                (self.ivars().0)(LifecycleNotice::WillBecomeActive { any_window_visible });
            });
        }

        #[unsafe(method(applicationWillResignActive:))]
        fn application_will_resign_active(&self, _notice: &NSNotification) {
            abort_on_panic(|| (self.ivars().0)(LifecycleNotice::WillResignActive));
        }

        #[unsafe(method(applicationDidChangeOcclusionState:))]
        fn application_did_change_occlusion_state(&self, _notice: &NSNotification) {
            abort_on_panic(|| {
                let visible = application_visible(self.mtm());
                (self.ivars().0)(LifecycleNotice::OcclusionChanged { visible });
            });
        }
    }
);

impl LifecycleObserver {
    fn new(
        mtm: MainThreadMarker,
        send: Box<dyn Fn(LifecycleNotice)>,
    ) -> Retained<LifecycleObserver> {
        let this = mtm.alloc::<LifecycleObserver>().set_ivars(SendNotice(send));
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

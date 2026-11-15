//! Waft's application delegate, as Flutter's `FlutterAppDelegate` is the application's: it starts
//! the app as the launch finishes, hears the notification center's clicks from before then, so the
//! click that launched the app is heard too, and hears the sessions the user closed for good.
//! Everything of a window is the scenes' (`scene_delegate.rs`), as Flutter's UIScene migration has
//! it: the delegate makes no window.

// A class UIKit makes by name needs `unsafe` to define; this module writes it for it.
#![allow(unsafe_code)]

use std::sync::Arc;

use dispatch2::DispatchQueue;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{MainThreadOnly, define_class};
use objc2_foundation::{NSDictionary, NSSet, NSString};
use objc2_ui_kit::{UIApplication, UIApplicationDelegate, UIResponder, UISceneSession};

use crate::host::{HostEvent, deliver};

define_class!(
    // SAFETY: UIResponder has no subclassing requirements, and `WaftAppDelegate` has no `Drop`.
    #[unsafe(super(UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftAppDelegate"]
    pub(crate) struct WaftAppDelegate;

    unsafe impl NSObjectProtocol for WaftAppDelegate {}

    unsafe impl UIApplicationDelegate for WaftAppDelegate {
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        fn did_finish_launching(
            &self,
            _application: &UIApplication,
            _options: Option<&NSDictionary<NSString, AnyObject>>,
        ) -> bool {
            hear_notification_activations();
            crate::embedder::launched(self.mtm());
            true
        }

        #[unsafe(method(application:didDiscardSceneSessions:))]
        fn did_discard_scene_sessions(
            &self,
            _application: &UIApplication,
            sessions: &NSSet<UISceneSession>,
        ) {
            let sessions = sessions
                .iter()
                .map(|session| session.persistentIdentifier().to_string())
                .collect();
            deliver(HostEvent::SessionsDiscarded(sessions));
        }
    }
);

/// Sends the notification center's clicks to the host, from whatever queue the center reports
/// them on.
fn hear_notification_activations() {
    waft_embedder_apple::notifications::hear_activations(Arc::new(|activation| {
        DispatchQueue::main()
            .exec_async(move || deliver(HostEvent::NotificationActivated(activation)));
    }));
}

//! What iOS asks the app to open: a document or a link whose scheme or kind is the app's. winit
//! gives the application no delegate, so a launch's URL is read from the launch's notice
//! (`UIApplicationDidFinishLaunchingNotification`, its `UIApplicationLaunchOptionsURLKey`), and
//! a delegate of the embedder's own, set then, hears each later one
//! (`application:openURL:options:`). A file URL opened in place is reached only while its
//! security-scoped access lasts, which starts as it is answered and is kept for the app's life,
//! as a picked file's is (`file_dialogs`).
//!
//! The delegate is the application's only one and answers nothing else, and UIKit holds it
//! weakly, so it is kept here for the app's life.

// UIKit's notices, its application delegate and a URL's access need `unsafe`; this module writes
// it for them.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_foundation::{
    NSDictionary, NSNotification, NSNotificationCenter, NSObjectProtocol, NSURL,
};
#[expect(
    deprecated,
    reason = "winit runs no scenes, so the launch's URL is the application's"
)]
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationDidFinishLaunchingNotification,
    UIApplicationLaunchOptionsURLKey,
};

use super::Ios;
use crate::os::SystemRequests;
use crate::system_requests::SystemRequest;

thread_local! {
    /// The observer of the launch and the delegate, kept for the app's life. Main thread.
    static KEPT: RefCell<Option<(Retained<LaunchObserver>, Retained<OpenDelegate>)>> =
        const { RefCell::new(None) };
    /// Each file URL opened in place whose access is kept for the app's life. Main thread.
    static ACCESSED: RefCell<Vec<Retained<NSURL>>> = const { RefCell::new(Vec::new()) };
}

impl SystemRequests for Ios {
    /// Hears the launch's URL from its notice, before the loop runs, and sets the delegate for
    /// later ones then.
    fn send_system_requests(send: Box<dyn Fn(SystemRequest)>) {
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let send: std::rc::Rc<dyn Fn(SystemRequest)> = std::rc::Rc::from(send);
        let observer = LaunchObserver::new(main_thread, std::rc::Rc::clone(&send));
        let delegate = OpenDelegate::new(main_thread, send);
        // SAFETY: `launched:` is defined on `LaunchObserver` below with the one-argument signature
        // a notice is sent with; the observer is kept for the app's life.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &observer,
                sel!(launched:),
                Some(UIApplicationDidFinishLaunchingNotification),
                None,
            );
        }
        KEPT.set(Some((observer, delegate)));
    }
}

/// `url` as what it asks to open: a file, its access started and kept, or a link.
fn request_for(url: &NSURL) -> Option<SystemRequest> {
    if url.isFileURL() {
        let path: PathBuf = url.to_file_path()?;
        // SAFETY: a URL iOS handed the app, whose access it grants until stopped.
        if unsafe { url.startAccessingSecurityScopedResource() } {
            ACCESSED.with_borrow_mut(|accessed| accessed.push(url.retain()));
        }
        return Some(SystemRequest::Open {
            files: vec![path],
            links: Vec::new(),
        });
    }
    Some(SystemRequest::Open {
        files: Vec::new(),
        links: vec![url.absoluteString()?.to_string()],
    })
}

struct Sending {
    send: std::rc::Rc<dyn Fn(SystemRequest)>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `LaunchObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetLaunchObserver"]
    #[ivars = Sending]
    struct LaunchObserver;

    unsafe impl NSObjectProtocol for LaunchObserver {}

    impl LaunchObserver {
        /// The launch finished: its URL, if it has one, is sent, and the delegate set for later
        /// ones.
        #[unsafe(method(launched:))]
        fn launched(&self, notice: &NSNotification) {
            let url = notice.userInfo().and_then(|info| {
                // SAFETY: a constant UIKit exports for the life of the process.
                #[expect(deprecated, reason = "winit runs no scenes, so the launch's URL is the application's")]
                let key = unsafe { UIApplicationLaunchOptionsURLKey };
                info.objectForKey(key)?.downcast::<NSURL>().ok()
            });
            if let Some(request) = url.as_deref().and_then(request_for) {
                (self.ivars().send)(request);
            }
            KEPT.with_borrow(|kept| {
                if let Some((_, delegate)) = kept {
                    // SAFETY: the delegate is kept here for the app's life.
                    unsafe {
                        UIApplication::sharedApplication(self.mtm())
                            .setDelegate(Some(ProtocolObject::from_ref(&**delegate)));
                    }
                }
            });
        }
    }
);

impl LaunchObserver {
    fn new(
        main_thread: MainThreadMarker,
        send: std::rc::Rc<dyn Fn(SystemRequest)>,
    ) -> Retained<LaunchObserver> {
        let this = main_thread
            .alloc::<LaunchObserver>()
            .set_ivars(Sending { send });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `OpenDelegate` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetOpenDelegate"]
    #[ivars = Sending]
    struct OpenDelegate;

    unsafe impl NSObjectProtocol for OpenDelegate {}

    unsafe impl UIApplicationDelegate for OpenDelegate {
        #[unsafe(method(application:openURL:options:))]
        fn open_url(
            &self,
            _application: &UIApplication,
            url: &NSURL,
            _options: &NSDictionary<objc2_foundation::NSString, AnyObject>,
        ) -> bool {
            match request_for(url) {
                Some(request) => {
                    (self.ivars().send)(request);
                    true
                }
                None => false,
            }
        }
    }
);

impl OpenDelegate {
    fn new(
        main_thread: MainThreadMarker,
        send: std::rc::Rc<dyn Fn(SystemRequest)>,
    ) -> Retained<OpenDelegate> {
        let this = main_thread
            .alloc::<OpenDelegate>()
            .set_ivars(Sending { send });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

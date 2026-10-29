//! The application's delegate, the embedder's own, which answers what AppKit asks of the
//! application: files and links to open (`application:openURLs:`), a click on the Dock icon or
//! another reopen (`applicationShouldHandleReopen:hasVisibleWindows:`), quitting
//! (`applicationShouldTerminate:`) and the Dock icon's menu (`applicationDockMenu:`, which
//! `menus/dock_menu` keeps); and hiding the application, which AppKit does itself.
//!
//! winit leaves `NSApp`'s delegate to the application, and only observes the launch and the
//! termination through notices. The embedder sets its delegate before the loop runs, since AppKit
//! asks for the files a launch opens before it reports the launch finished, and keeps it for the
//! process's life, since AppKit holds its delegate weakly.
//!
//! AppKit calls the delegate from its own dispatch, which can be inside a winit callback, so it
//! only queues the request for the event loop and must never reach the embedder's state. What
//! belongs to the moment AppKit asks, such as where the pointer is at a reopen, is read there, and
//! queued with the request. Each method's body runs through `abort_on_panic`, so a panic in one
//! aborts the process rather than unwinding into AppKit.
//!
//! A quit is two calls of `terminate:`, since `applicationShouldTerminate:` must answer at once and
//! the framework may answer much later: the first cancels and asks the framework, and once it
//! agrees the embedder calls `terminate:` again, which goes through. Every `terminate:` before
//! then asks the framework again, so a second Cmd-Q is the application's to decide. A quit the
//! application asks for itself goes through the same second call.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationTerminateReply, NSMenu};
use objc2_foundation::{NSArray, NSURL};
use winit::event_loop::ActiveEventLoop;

use super::{AppKit, abort_on_panic, menus, screens};
use crate::os::{ApplicationControl, SystemRequests};
use crate::system_requests::{Reopen, SystemRequest};

impl SystemRequests for AppKit {
    /// Sets the embedder's delegate as `NSApp`'s, its requests sent to `send`. This must run
    /// before the loop does: AppKit asks for the files a launch opens before it reports the launch
    /// finished.
    fn send_system_requests(send: Box<dyn Fn(SystemRequest)>) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let delegate = AppDelegate::new(mtm, send);
        NSApplication::sharedApplication(mtm)
            .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        APP_DELEGATE.set(Some(delegate));
    }
}

impl ApplicationControl for AppKit {
    /// Quits through `terminate:`, as Flutter's macOS embedder does once the framework agrees:
    /// AppKit asks the delegate again, which answers at once, and winit hears
    /// `NSApplicationWillTerminateNotification`. Run from the main queue, since winit's handler
    /// would be entered again from inside the callback that asked.
    fn quit(_event_loop: &dyn ActiveEventLoop) {
        APP_DELEGATE.with_borrow(|delegate| {
            if let Some(delegate) = delegate {
                delegate.ivars().termination.allow();
            }
        });
        DispatchQueue::main().exec_async(|| {
            if let Some(mtm) = MainThreadMarker::new() {
                NSApplication::sharedApplication(mtm).terminate(None);
            }
        });
    }

    /// `hide:`, as the application menu's Hide does: AppKit orders every window out and
    /// activates the application that was active before. winit hears the key window's focus
    /// go, and queues that, from inside the call as from any other.
    fn hide_application() {
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).hide(None);
        }
    }
}

thread_local! {
    /// The delegate set as `NSApp`'s, kept for the life of the process. Main thread.
    static APP_DELEGATE: RefCell<Option<Retained<AppDelegate>>> = const { RefCell::new(None) };
}

/// What the delegate keeps: where it sends its requests, and whether a quit may go through.
struct Delegate {
    send: Box<dyn Fn(SystemRequest)>,
    termination: Termination,
}

/// Whether AppKit may terminate the application, so that the `terminate:` the embedder calls goes
/// through: Flutter's `FlutterEngineTerminationHandler`'s `shouldTerminate`, but allowed only once
/// the application has decided to quit. Flutter sets it already when it asks, which lets a second
/// Cmd-Q through without asking.
#[derive(Default)]
struct Termination {
    allowed: Cell<bool>,
}

impl Termination {
    fn allow(&self) {
        self.allowed.set(true);
    }

    /// `applicationShouldTerminate:`, as `FlutterAppDelegate` answers it: now once termination
    /// is allowed; otherwise `ask` asks the framework, and this `terminate:` is
    /// cancelled.
    fn should_terminate(&self, ask: impl FnOnce()) -> NSApplicationTerminateReply {
        if self.allowed.get() {
            return NSApplicationTerminateReply::TerminateNow;
        }
        ask();
        NSApplicationTerminateReply::TerminateCancel
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `AppDelegate` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetApplicationDelegate"]
    #[ivars = Delegate]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(application:openURLs:))]
        fn application_open_urls(&self, _application: &NSApplication, urls: &NSArray<NSURL>) {
            abort_on_panic(|| {
                if let Some(request) = open_request(urls) {
                    (self.ivars().send)(request);
                }
            });
        }

        /// Reports every reopen, with the pointer read now, while it is still where the click
        /// was, and answers yes so AppKit still brings a minimized window back as it does for any
        /// application.
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(
            &self,
            _application: &NSApplication,
            has_visible_windows: bool,
        ) -> bool {
            abort_on_panic(|| {
                (self.ivars().send)(SystemRequest::Reopen(Reopen {
                    had_visible_windows: has_visible_windows,
                    pointer: screens::pointer_location(),
                }));
                true
            })
        }

        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _application: &NSApplication) -> NSApplicationTerminateReply {
            abort_on_panic(|| {
                let delegate = self.ivars();
                delegate
                    .termination
                    .should_terminate(|| (delegate.send)(SystemRequest::Quit))
            })
        }

        #[unsafe(method_id(applicationDockMenu:))]
        fn dock_menu(&self, _application: &NSApplication) -> Option<Retained<NSMenu>> {
            abort_on_panic(menus::dock_menu)
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker, send: Box<dyn Fn(SystemRequest)>) -> Retained<AppDelegate> {
        let this = AppDelegate::alloc(mtm).set_ivars(Delegate {
            send,
            termination: Termination::default(),
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

/// The files and links `urls` names, a file URL as a path and any other as a link; `None` when it
/// names neither.
fn open_request(urls: &NSArray<NSURL>) -> Option<SystemRequest> {
    let (files, links): (Vec<_>, Vec<_>) = urls.iter().partition(|url| url.isFileURL());
    let files: Vec<PathBuf> = files.iter().filter_map(|url| url.to_file_path()).collect();
    let links: Vec<String> = links
        .iter()
        .filter_map(|url| url.absoluteString())
        .map(|link| link.to_string())
        .collect();
    (!files.is_empty() || !links.is_empty()).then_some(SystemRequest::Open { files, links })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quit_asks_the_application_and_waits_for_its_answer() {
        let termination = Termination::default();
        let asked = Cell::new(0);
        let reply = termination.should_terminate(|| asked.set(asked.get() + 1));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateCancel);
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn a_quit_while_the_first_is_decided_asks_again() {
        let termination = Termination::default();
        termination.should_terminate(|| {});
        let asked = Cell::new(false);
        let reply = termination.should_terminate(|| asked.set(true));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateCancel);
        assert!(asked.get());
    }

    #[test]
    fn a_quit_the_host_carries_out_goes_through_when_appkit_asks() {
        let termination = Termination::default();
        termination.allow();
        let reply = termination.should_terminate(|| panic!("asked again"));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateNow);
    }
}

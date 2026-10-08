//! The requests AppKit makes of the application through its delegate, which winit's delegate
//! does not answer: files to open (`application:openURLs:`), a click on the Dock icon or
//! another reopen (`applicationShouldHandleReopen:hasVisibleWindows:`), and quitting
//! (`applicationShouldTerminate:`); and hiding the application, which AppKit does itself.
//!
//! winit sets `NSApp`'s delegate and checks its class whenever it reaches for it, so the
//! delegate cannot be replaced. These methods are added to the class of winit's delegate at
//! run time instead, as winit itself replaces `NSApplication`'s `sendEvent:`. A method winit
//! comes to answer itself stays winit's, with a warning printed, and that request is no
//! longer reported.
//!
//! AppKit calls these from its own dispatch, which can be inside a winit callback, so they
//! only queue the request for the event loop and must never reach the embedder's state. What
//! belongs to the moment AppKit asks, such as where the pointer is at a reopen, is read there, and
//! queued with the request. Each is `extern "C"`, so a panic in one aborts the process rather than
//! unwinding into AppKit.
//!
//! A quit is two calls of `terminate:`, since `applicationShouldTerminate:` must answer at once and
//! the framework may answer much later: the first cancels and asks the framework, and once it
//! agrees the embedder calls `terminate:` again, which goes through. A quit the embedder starts,
//! for the application or its last window, goes through the same second call.

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::path::PathBuf;

use dispatch2::DispatchQueue;
use objc2::encode::{EncodeArguments, EncodeReturn};
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, MethodImplementation, Sel};
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use objc2_foundation::{NSArray, NSURL};
use winit::event_loop::ActiveEventLoop;

use super::{AppKit, screens};
use crate::os::{ApplicationControl, SystemRequests};
use crate::system_requests::{Reopen, SystemRequest};

impl SystemRequests for AppKit {
    /// The Finder, Open With and the Dock icon ask through `application:openURLs:`.
    const ASKS_TO_OPEN_FILES: bool = true;

    /// Adds the three methods to winit's delegate, which exists once the event loop is built.
    /// This must run before the loop does: AppKit asks for the files a launch opens before it
    /// reports the launch finished.
    fn send_system_requests(send: Box<dyn Fn(SystemRequest)>) {
        let Some(class) = delegate_class() else {
            return;
        };
        DELEGATE.set(Some(Delegate {
            send,
            termination: Termination::default(),
        }));
        add_delegate_methods(class);
    }
}

impl ApplicationControl for AppKit {
    /// Flutter's `requestApplicationTermination:`, which notes the quit before it asks, so a
    /// `terminate:` meanwhile goes through at once.
    fn quit_being_decided() {
        with_termination(Termination::under_way);
    }

    /// Quits through `terminate:`, as Flutter's macOS embedder does once the framework agrees:
    /// AppKit asks the delegate again, which answers at once, and winit hears
    /// `applicationWillTerminate:`. Where there was no delegate to add the methods to, AppKit
    /// quits without asking, as Flutter's engine does when the app's delegate is not a
    /// `FlutterAppDelegate`. Run from the main queue, since winit's handler would be entered
    /// again from inside the callback that asked.
    fn quit(_event_loop: &ActiveEventLoop) {
        with_termination(Termination::under_way);
        DispatchQueue::main().exec_async(|| {
            if let Some(mtm) = MainThreadMarker::new() {
                NSApplication::sharedApplication(mtm).terminate(None);
            }
        });
    }

    /// The next `terminate:` asks the framework again.
    fn keep_running() {
        with_termination(Termination::cancelled);
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
    /// Where the delegate methods send their requests: set once, on the main thread, where
    /// AppKit calls them.
    static DELEGATE: RefCell<Option<Delegate>> = const { RefCell::new(None) };
}

struct Delegate {
    send: Box<dyn Fn(SystemRequest)>,
    termination: Termination,
}

/// The one way to the delegate's state; `None` before it is set, where there was no delegate to
/// add the methods to. The state stays borrowed across `send`, which only queues for the event
/// loop (`SystemRequests::send_system_requests`), so nothing borrows it again meanwhile.
fn with_delegate<R>(f: impl FnOnce(&Delegate) -> R) -> Option<R> {
    DELEGATE.with_borrow(|delegate| delegate.as_ref().map(f))
}

fn with_termination(change: impl FnOnce(&Termination)) {
    with_delegate(|delegate| change(&delegate.termination));
}

fn send(request: SystemRequest) {
    with_delegate(|delegate| (delegate.send)(request));
}

/// Whether a quit is under way, being decided or agreed to, so that `terminate:` goes through at
/// once: Flutter's `FlutterEngineTerminationHandler`, whose `shouldTerminate` is set when a quit
/// is asked for and cleared when the application keeps running.
#[derive(Default)]
struct Termination {
    should_terminate: Cell<bool>,
}

impl Termination {
    fn under_way(&self) {
        self.should_terminate.set(true);
    }

    fn cancelled(&self) {
        self.should_terminate.set(false);
    }

    /// `applicationShouldTerminate:`, as `FlutterAppDelegate` answers it: at once while a quit
    /// is under way; otherwise the quit is under way from now, `ask` asks the framework, and this
    /// `terminate:` is cancelled. A second quit while the first is being decided goes through at
    /// once, as Flutter's.
    fn should_terminate(&self, ask: impl FnOnce()) -> NSApplicationTerminateReply {
        if self.should_terminate.get() {
            return NSApplicationTerminateReply::TerminateNow;
        }
        self.under_way();
        ask();
        NSApplicationTerminateReply::TerminateCancel
    }
}

extern "C" fn open_urls(
    _delegate: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    urls: &NSArray<NSURL>,
) {
    let files: Vec<PathBuf> = urls
        .iter()
        .filter(|url| url.isFileURL())
        .filter_map(|url| url.to_file_path())
        .collect();
    if !files.is_empty() {
        send(SystemRequest::OpenFiles(files));
    }
}

/// Reports every reopen, with the pointer read now, while it is still where the click was, and
/// answers yes so AppKit still brings a minimized window back as it does for any application.
extern "C" fn should_handle_reopen(
    _delegate: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
    has_visible_windows: Bool,
) -> Bool {
    send(SystemRequest::Reopen(Reopen {
        had_visible_windows: has_visible_windows.as_bool(),
        pointer: screens::pointer_location(),
    }));
    Bool::YES
}

extern "C" fn should_terminate(
    _delegate: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
) -> NSApplicationTerminateReply {
    with_delegate(|delegate| {
        delegate
            .termination
            .should_terminate(|| (delegate.send)(SystemRequest::Quit))
    })
    .unwrap_or(NSApplicationTerminateReply::TerminateNow)
}

/// The class of `NSApp`'s delegate, which is winit's once the event loop is built.
fn delegate_class() -> Option<&'static AnyClass> {
    let mtm = MainThreadMarker::new()?;
    let delegate = NSApplication::sharedApplication(mtm).delegate()?;
    let delegate: &AnyObject = delegate.as_ref();
    Some(delegate.class())
}

fn add_delegate_methods(class: &AnyClass) {
    add_method(
        class,
        sel!(application:openURLs:),
        open_urls as extern "C" fn(_, _, _, _),
    );
    add_method(
        class,
        sel!(applicationShouldHandleReopen:hasVisibleWindows:),
        should_handle_reopen as extern "C" fn(_, _, _, _) -> _,
    );
    add_method(
        class,
        sel!(applicationShouldTerminate:),
        should_terminate as extern "C" fn(_, _, _) -> _,
    );
}

/// Adds `method` to `class` as `sel`, with the type encoding its Rust signature gives. A class
/// that already answers `sel` keeps its own.
fn add_method<F: MethodImplementation>(class: &AnyClass, sel: Sel, method: F) {
    if class.instance_method(sel).is_some() {
        eprintln!("inset: winit's application delegate answers {sel} itself; not reported");
        return;
    }
    assert_eq!(
        size_of::<F>(),
        size_of::<Imp>(),
        "a method is a function pointer"
    );
    // SAFETY: every `MethodImplementation` is an `extern` function pointer, which is what an
    // `Imp` is, called with the arguments its own encoding below declares.
    let imp = unsafe { std::mem::transmute_copy::<F, Imp>(&method) };
    let types = type_encoding::<F>();
    // SAFETY: a class registered with the runtime, on the main thread, before AppKit sends
    // it any of these messages; the encoding matches the function's signature.
    let added = unsafe {
        objc2::ffi::class_addMethod(
            std::ptr::from_ref(class).cast_mut(),
            sel,
            imp,
            types.as_ptr(),
        )
    };
    if !added.as_bool() {
        eprintln!("inset: could not add {sel} to the application delegate; not reported");
    }
}

/// The runtime's type encoding of a method: the return type, the receiver and selector, then
/// each argument.
fn type_encoding<F: MethodImplementation>() -> CString {
    let mut types = F::Return::ENCODING_RETURN.to_string();
    types.push_str("@:");
    for argument in F::Arguments::ENCODINGS {
        types.push_str(&argument.to_string());
    }
    CString::new(types).expect("a type encoding has no NUL")
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
    fn a_quit_while_the_first_is_decided_goes_through() {
        let termination = Termination::default();
        termination.should_terminate(|| {});
        let reply = termination.should_terminate(|| panic!("asked twice"));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateNow);
    }

    #[test]
    fn a_quit_the_host_carries_out_goes_through_when_appkit_asks() {
        let termination = Termination::default();
        termination.under_way();
        let reply = termination.should_terminate(|| panic!("asked again"));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateNow);
    }

    #[test]
    fn once_the_application_keeps_running_the_next_quit_asks_again() {
        let termination = Termination::default();
        termination.should_terminate(|| {});
        termination.cancelled();
        let asked = Cell::new(false);
        let reply = termination.should_terminate(|| asked.set(true));
        assert_eq!(reply, NSApplicationTerminateReply::TerminateCancel);
        assert!(asked.get());
    }

    #[test]
    fn a_method_is_encoded_from_its_signature() {
        type ShouldTerminate = extern "C" fn(
            &'static AnyObject,
            Sel,
            &'static NSApplication,
        ) -> NSApplicationTerminateReply;
        assert_eq!(type_encoding::<ShouldTerminate>().to_str().unwrap(), "Q@:@");
        type OpenUrls =
            extern "C" fn(&'static AnyObject, Sel, &'static NSApplication, &'static NSArray<NSURL>);
        assert_eq!(type_encoding::<OpenUrls>().to_str().unwrap(), "v@:@@");
    }
}

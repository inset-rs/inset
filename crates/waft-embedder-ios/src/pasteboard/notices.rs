//! UIKit's notice that the pasteboard changed, which it posts while the app is active, and which has
//! the host look at the pasteboard; the host looks again as a scene becomes active (`host.rs`).

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};
use objc2_ui_kit::UIPasteboardChangedNotification;

/// The notices heard, until this is dropped.
pub(super) struct Notices {
    observer: Retained<NoticeObserver>,
}

impl Notices {
    /// Calls `look` at each notice, on the main thread, from UIKit's posting.
    pub(super) fn observe(main_thread: MainThreadMarker, look: Box<dyn Fn()>) -> Notices {
        let observer = NoticeObserver::new(main_thread, look);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: `pasteboardMayHaveChanged:` is defined on `NoticeObserver` below with the
        // one-argument signature a notice is sent with, and `Drop` removes the observer before it
        // goes; the name is a constant UIKit exports for the life of the process.
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(pasteboardMayHaveChanged:),
                Some(UIPasteboardChangedNotification),
                None,
            );
        }
        Notices { observer }
    }
}

impl Drop for Notices {
    fn drop(&mut self) {
        // SAFETY: the observer this added for every notice it hears.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.observer) };
    }
}

struct Notice {
    look: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `NoticeObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftPasteboardNoticeObserver"]
    #[ivars = Notice]
    struct NoticeObserver;

    unsafe impl NSObjectProtocol for NoticeObserver {}

    impl NoticeObserver {
        #[unsafe(method(pasteboardMayHaveChanged:))]
        fn pasteboard_may_have_changed(&self, _notice: &NSNotification) {
            (self.ivars().look)();
        }
    }
);

impl NoticeObserver {
    fn new(main_thread: MainThreadMarker, look: Box<dyn Fn()>) -> Retained<NoticeObserver> {
        let this = main_thread
            .alloc::<NoticeObserver>()
            .set_ivars(Notice { look });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

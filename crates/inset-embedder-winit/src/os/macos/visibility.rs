//! Whether a window is visible, and AppKit's notices that its visibility may have changed: what
//! decides whether the producers of the window's native surfaces get vsyncs, and what the app
//! hears of its window being minimized or restored, occluded or shown (`WindowVisibility`).
//!
//! A window is visible while it is ordered in and its occlusion state has the visible bit: not
//! while it is hidden or minimized, nor while its application is hidden, nor while other
//! windows cover all of it. That is the test Chromium hides a page's renderer by on macOS
//! (`-[WebContentsViewCocoa updateWebContentsVisibility]` and `WebContentsOcclusionCheckerMac`,
//! in `content/app_shim_remote_cocoa/`), and WebKit a page (`PageClientImpl::isViewVisible`).
//!
//! AppKit posts a notice for every change of a window's occlusion state, which is enough to
//! hear every change of the test. The notices Chromium takes a window's visibility from are
//! heard too (`components/remote_cocoa/app_shim/native_widget_ns_window_bridge.mm`
//! `OnVisibilityChanged`, `views_nswindow_delegate.mm`), as WebKit's `WebViewImpl` hears them:
//! the application hidden and unhidden, and the window minimized and restored. The embedder's own
//! showing, hiding and ordering front of a window, and its moves, which Chromium hears by
//! overriding `-orderWindow:relativeTo:`, the embedder reports itself
//! (`WindowSurfaces::follow_window`).
//!
//! A window is occluded while its occlusion state has no visible bit, whatever the reason: covered
//! all over, hidden, minimized, or in a hidden application. winit reports a change of it as
//! `Occluded`, but not a window's minimizing, which the app hears through the same notices.

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplicationDidHideNotification, NSApplicationDidUnhideNotification, NSWindow,
    NSWindowDidChangeOcclusionStateNotification, NSWindowDidDeminiaturizeNotification,
    NSWindowDidMiniaturizeNotification, NSWindowOcclusionState,
};
use objc2_foundation::{NSNotification, NSNotificationCenter};
use winit::window::Window;

use super::{AppKit, abort_on_panic, ns_window};
use crate::os::WindowVisibility;

impl WindowVisibility for AppKit {
    type Notices = VisibilityNotices;

    fn is_occluded(window: &Window) -> Option<bool> {
        ns_window(window).map(|window| !has_visible_bit(&window))
    }

    fn observe_visibility(window: &Window, on_notice: Box<dyn Fn()>) -> Option<VisibilityNotices> {
        VisibilityNotices::observe(window, on_notice)
    }
}

/// Whether `window` is visible: ordered in, and not covered all over. Hidden, minimized and in a
/// hidden application, a window is out of sight.
pub(crate) fn is_visible(window: &Window) -> bool {
    ns_window(window).is_some_and(|window| window.isVisible() && has_visible_bit(&window))
}

/// Whether some of `window` shows, as its occlusion state says.
fn has_visible_bit(window: &NSWindow) -> bool {
    window
        .occlusionState()
        .contains(NSWindowOcclusionState::Visible)
}

/// Calls its `on_notice`, on the main thread, whenever AppKit says a window's visibility may
/// have changed; the calls end when this is dropped.
pub(crate) struct VisibilityNotices {
    observer: Retained<NoticeObserver>,
}

impl VisibilityNotices {
    /// `None` off the main thread, and for a window with no `NSWindow` behind it.
    pub(crate) fn observe(window: &Window, on_notice: Box<dyn Fn()>) -> Option<VisibilityNotices> {
        let main_thread = MainThreadMarker::new()?;
        let ns_window = ns_window(window)?;
        let observer = NoticeObserver::new(main_thread, on_notice);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the names are constants AppKit exports for the life of the process.
        let window_notices = unsafe {
            [
                NSWindowDidChangeOcclusionStateNotification,
                NSWindowDidMiniaturizeNotification,
                NSWindowDidDeminiaturizeNotification,
            ]
        };
        // SAFETY: as above.
        let application_notices = unsafe {
            [
                NSApplicationDidHideNotification,
                NSApplicationDidUnhideNotification,
            ]
        };
        for name in window_notices {
            // SAFETY: `windowMayHaveChangedVisibility:` is defined on `NoticeObserver` below with
            // the one-argument signature a notice is sent with, and `Drop` removes the observer
            // before it goes.
            unsafe {
                center.addObserver_selector_name_object(
                    &observer,
                    sel!(windowMayHaveChangedVisibility:),
                    Some(name),
                    Some(&ns_window),
                );
            }
        }
        for name in application_notices {
            // SAFETY: as above; the application's notices come from any object.
            unsafe {
                center.addObserver_selector_name_object(
                    &observer,
                    sel!(windowMayHaveChangedVisibility:),
                    Some(name),
                    None,
                );
            }
        }
        Some(VisibilityNotices { observer })
    }
}

impl Drop for VisibilityNotices {
    fn drop(&mut self) {
        // SAFETY: the observer this added for every notice it hears.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.observer) };
    }
}

struct Notice {
    on_notice: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `NoticeObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetWindowVisibilityObserver"]
    #[ivars = Notice]
    struct NoticeObserver;

    impl NoticeObserver {
        #[unsafe(method(windowMayHaveChangedVisibility:))]
        fn window_may_have_changed_visibility(&self, _notice: &NSNotification) {
            abort_on_panic(|| (self.ivars().on_notice)());
        }
    }
);

impl NoticeObserver {
    fn new(mtm: MainThreadMarker, on_notice: Box<dyn Fn()>) -> Retained<NoticeObserver> {
        let this = mtm
            .alloc::<NoticeObserver>()
            .set_ivars(Notice { on_notice });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

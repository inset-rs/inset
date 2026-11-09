//! A window without a title bar of its own (`MacosTitleBar::Hidden`), as Safari's and Finder's
//! are, and what System Settings says a double-click on a title bar does.
//!
//! winit makes the window with its view under a transparent, untitled title bar (`window_chrome`'s
//! attributes), and this puts the window's buttons in the app's band. AppKit lays them out again
//! whenever it lays out the title bar, as the window resizes or leaves full screen, so they are
//! placed again after each, as Tauri's `trafficLightPosition` does. The view covers the band and
//! takes its presses itself, so the title bar AppKit keeps under it moves nothing on its own: the
//! app moves the window (`HostWindow::drag`).

use std::ffi::c_void;

use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSWindow, NSWindowButton, NSWindowDidExitFullScreenNotification, NSWindowDidResizeNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSPoint, NSUserDefaults, ns_string};
use waft_embedder::TitleBarAction;
use winit::window::Window;

use super::{abort_on_panic, ns_window};

/// `put_buttons_in_band` puts the close, minimize and zoom buttons of `window`, made without a
/// title bar of its own, in a band `height` points tall at its top, and keeps them there.
pub(super) fn put_buttons_in_band(window: &dyn Window, height: f64) {
    let (Some(main_thread), Some(ns_window)) = (MainThreadMarker::new(), ns_window(window)) else {
        return;
    };
    place_buttons(&ns_window, height);
    ButtonPlacer::follow(main_thread, &ns_window, height);
}

/// `double_click_action` is System Settings' "Double-click a window's title bar to"
/// (`AppleActionOnDoubleClick`): zoom (`Maximize`, also when it was never changed), minimize, or
/// nothing; a value this doesn't know is `Unknown`.
pub(super) fn double_click_action() -> TitleBarAction {
    let action = NSUserDefaults::standardUserDefaults()
        .stringForKey(ns_string!("AppleActionOnDoubleClick"))
        .map(|action| action.to_string());
    match action.as_deref() {
        None | Some("Maximize") => TitleBarAction::Maximize,
        Some("Minimize") => TitleBarAction::Minimize,
        Some("None") => TitleBarAction::Nothing,
        Some(_) => TitleBarAction::Unknown,
    }
}

/// `place_buttons` centres the window's buttons in the band, as far from the left edge as from
/// the band's top, keeping AppKit's spacing between them. AppKit's own layout comes first, or it
/// would put them back once it ran.
fn place_buttons(window: &NSWindow, height: f64) {
    let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .map(|kind| window.standardWindowButton(kind));
    let [Some(close), Some(minimize), Some(zoom)] = buttons else {
        return;
    };
    // SAFETY: the button's superview, the title bar, read on the main thread while the window is
    // alive.
    let Some(title_bar) = (unsafe { close.superview() }) else {
        return;
    };
    // SAFETY: as above, the title bar's own superview.
    if let Some(container) = unsafe { title_bar.superview() } {
        container.layoutSubtreeIfNeeded();
    }
    let size = close.frame().size;
    let spacing = minimize.frame().origin.x - close.frame().origin.x;
    let inset = (height - size.height) / 2.0;
    let y = if title_bar.isFlipped() {
        inset
    } else {
        title_bar.frame().size.height - inset - size.height
    };
    for (index, button) in [close, minimize, zoom].iter().enumerate() {
        button.setFrameOrigin(NSPoint::new(inset + spacing * index as f64, y));
    }
}

/// `Band` is the window a placer keeps the buttons of, and the band's height.
struct Band {
    window: Weak<NSWindow>,
    height: f64,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `ButtonPlacer` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WaftTitleBarButtonPlacer"]
    #[ivars = Band]
    struct ButtonPlacer;

    impl ButtonPlacer {
        #[unsafe(method(titleBarLaidOut:))]
        fn title_bar_laid_out(&self, _notice: &NSNotification) {
            abort_on_panic(|| {
                if let Some(window) = self.ivars().window.load() {
                    place_buttons(&window, self.ivars().height);
                }
            });
        }
    }
);

/// `PLACER` is the key a window holds its placer under.
static PLACER: u8 = 0;

impl ButtonPlacer {
    /// `follow` places the buttons of `window` again each time AppKit lays out its title bar, for
    /// as long as the window lives: the window holds the placer, and the notification center lets
    /// go of an observer that is freed.
    fn follow(main_thread: MainThreadMarker, window: &NSWindow, height: f64) {
        let band = Band {
            window: Weak::new(window),
            height,
        };
        let this = main_thread.alloc::<ButtonPlacer>().set_ivars(band);
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let placer: Retained<ButtonPlacer> = unsafe { msg_send![super(this), init] };
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the names are constants AppKit exports for the life of the process.
        let notices = unsafe {
            [
                NSWindowDidResizeNotification,
                NSWindowDidExitFullScreenNotification,
            ]
        };
        for name in notices {
            // SAFETY: `titleBarLaidOut:` is defined on `ButtonPlacer` above with the one-argument
            // signature a notice is sent with.
            unsafe {
                center.addObserver_selector_name_object(
                    &placer,
                    sel!(titleBarLaidOut:),
                    Some(name),
                    Some(window),
                );
            }
        }
        // SAFETY: the window and the placer are live objects; the window retains the placer under
        // a key no other code uses, and releases it when it is freed.
        unsafe {
            objc2::ffi::objc_setAssociatedObject(
                (window as *const NSWindow).cast_mut().cast::<AnyObject>(),
                (&raw const PLACER).cast::<c_void>(),
                Retained::as_ptr(&placer).cast_mut().cast::<AnyObject>(),
                objc2::ffi::OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
    }
}

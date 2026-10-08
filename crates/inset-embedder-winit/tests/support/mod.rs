//! What the window tests share: a watchdog for a test that would otherwise wait for ever, the
//! windows whose making has finished, a window closed once it is made, and what they ask of
//! AppKit that the embedder offers no call for.

#![allow(dead_code, reason = "each test binary uses a part of it")]

use std::task::{Context, Poll, Waker};
use std::time::Duration;

use inset_embedder::{PlatformRef, WindowConfig, WindowFuture, WindowRef};

/// Ends the process as a failure of `test` after `after`, from a thread of its own, with
/// what `reason` says then.
pub fn fail_after(
    test: &'static str,
    after: Duration,
    reason: impl FnOnce() -> String + Send + 'static,
) {
    std::thread::spawn(move || {
        std::thread::sleep(after);
        fail(test, &reason());
    });
}

/// Ends the process as a failure of `test`.
pub fn fail(test: &str, reason: &str) -> ! {
    eprintln!("test {test} ... FAILED: {reason}");
    std::process::exit(1);
}

/// Ends the process as a pass of `test`.
pub fn pass(test: &str) -> ! {
    println!("test {test} ... ok");
    std::process::exit(0);
}

/// Moves the windows whose making has finished from `opening` to `windows`, in the order they
/// were asked for.
pub fn take_opened_windows(opening: &mut Vec<WindowFuture>, windows: &mut Vec<WindowRef>) {
    let mut context = Context::from_waker(Waker::noop());
    let mut still_opening = Vec::new();
    for mut window in opening.drain(..) {
        match window.as_mut().poll(&mut context) {
            Poll::Ready(window) => windows.push(window.expect("a window")),
            Poll::Pending => still_opening.push(window),
        }
    }
    *opening = still_opening;
}

/// One window, asked for at the start and closed through its `WindowRef` once it is made.
pub struct WindowClosedOnceMade {
    opening: Vec<WindowFuture>,
    windows: Vec<WindowRef>,
}

impl WindowClosedOnceMade {
    pub fn open(platform: &PlatformRef, title: &str) -> WindowClosedOnceMade {
        let owner = platform
            .windowing_owner()
            .expect("a desktop host makes windows");
        WindowClosedOnceMade {
            opening: vec![owner.create(WindowConfig {
                title: title.into(),
                size: [300.0, 200.0],
                ..WindowConfig::default()
            })],
            windows: Vec::new(),
        }
    }

    /// Closes the window if it was made since the last call; called at each wake.
    pub fn close_once_made(&mut self) {
        take_opened_windows(&mut self.opening, &mut self.windows);
        if let Some(window) = self.windows.pop() {
            window.close();
        }
    }
}

/// What the tests ask of AppKit that the embedder offers no call for.
pub mod appkit {
    use objc2::rc::{Retained, Weak};
    use objc2::{ClassType, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSWindow};
    use objc2_foundation::{NSObjectProtocol, NSPoint, NSString};
    use objc2_quartz_core::CAMetalLayer;

    fn application() -> Retained<NSApplication> {
        NSApplication::sharedApplication(MainThreadMarker::new().expect("the main thread"))
    }

    fn window_titled(title: &str) -> Option<Retained<NSWindow>> {
        let title = NSString::from_str(title);
        application()
            .windows()
            .iter()
            .find(|window| window.title() == title)
    }

    /// Whether AppKit lists a window titled `title` among the application's windows.
    pub fn lists_window_titled(title: &str) -> bool {
        window_titled(title).is_some()
    }

    /// Whether a Metal layer covers the content of the window titled `title`: the layer wgpu
    /// draws a view's surface into, which it adds to the content view's layer as it makes the
    /// surface.
    pub fn has_a_metal_layer(title: &str) -> bool {
        let Some(content) = window_titled(title)
            .and_then(|window| window.contentView())
            .and_then(|view| view.layer())
        else {
            return false;
        };
        let frame = content.frame();
        let centre = NSPoint::new(
            frame.origin.x + frame.size.width / 2.0,
            frame.origin.y + frame.size.height / 2.0,
        );
        content
            .hitTest(centre)
            .is_some_and(|layer| layer.isKindOfClass(CAMetalLayer::class()))
    }

    /// Clicks the close button of the window titled `title`, as a person would.
    pub fn press_close_button(title: &str) {
        window_titled(title).expect("the window").performClose(None);
    }

    pub fn minimize(title: &str) {
        window_titled(title).expect("the window").miniaturize(None);
    }

    pub fn restore(title: &str) {
        window_titled(title)
            .expect("the window")
            .deminiaturize(None);
    }

    pub fn hide_application() {
        application().hide(None);
    }

    pub fn unhide_application() {
        application().unhide(None);
    }

    /// Puts the window titled `title` in front of every application's windows without
    /// activating this one, so that no window of another application covers it.
    pub fn order_in_front_of_everything(title: &str) {
        window_titled(title)
            .expect("the window")
            .orderFrontRegardless();
    }

    /// A window as AppKit has it, held weakly: whether it is still alive tells whether anything
    /// still holds it.
    pub struct WeakWindow(Weak<NSWindow>);

    impl WeakWindow {
        pub fn titled(title: &str) -> WeakWindow {
            WeakWindow(Weak::from_retained(
                &window_titled(title).expect("the window"),
            ))
        }

        pub fn is_alive(&self) -> bool {
            self.0.load().is_some()
        }
    }
}

//! The platform's vsync waiter, asked from another thread, calls an observer once per wait, on
//! the main thread, at a refresh of the screen it names, however often the wait was asked before
//! that refresh came; a wait on a screen that is not connected is never served.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for about a second, and needs a login session's window server.

// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_vsync_waiter_calls_once_per_wait";

/// Long enough for the window to open and the waits to be served, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the waits were not served within {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::WaitingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::sync::{Arc, Weak};
    use std::thread::ThreadId;
    use std::time::Duration;

    use inset_embedder::{
        EmbedderClient, Frame, Instant, KeyData, PlatformRef, PointerReport, ScreenId, ViewId,
        VsyncObserver, VsyncWaiter,
    };

    use crate::TEST;
    use crate::support;

    /// How many waits the observer is served.
    const WAITS: usize = 30;
    /// The longest a wait takes to be served: a few refreshes, at the slowest rate a display has.
    const PATIENCE: Duration = Duration::from_millis(500);
    /// How long the waiter is given to make a call it must not make.
    const QUIET: Duration = Duration::from_millis(200);
    /// No display has this number.
    const NO_SCREEN: ScreenId = ScreenId(u64::MAX);

    /// Asks for a window, and once its view is on a screen, waits on that screen's refreshes from
    /// a thread of its own.
    pub(crate) struct WaitingClient {
        platform: PlatformRef,
        main_thread: ThreadId,
    }

    impl WaitingClient {
        pub(crate) fn new(platform: PlatformRef) -> WaitingClient {
            support::request_window(&platform, support::titled("waiting on its screen"));
            WaitingClient {
                platform,
                main_thread: std::thread::current().id(),
            }
        }
    }

    /// Tells the waiting thread the thread each call came on.
    struct Calls(Sender<ThreadId>);

    impl VsyncObserver for Calls {
        fn vsync(&self, _at: Instant) {
            let _ = self.0.send(std::thread::current().id());
        }
    }

    /// An observer and the calls it hears.
    fn observer() -> (Arc<Calls>, Receiver<ThreadId>) {
        let (sender, calls) = channel();
        (Arc::new(Calls(sender)), calls)
    }

    /// Waits `WAITS` times on `screen`, asking twice each time, and checks each wait is served
    /// once, on `main_thread`, and that a wait on no screen never is.
    fn wait_on(waiter: Arc<dyn VsyncWaiter>, screen: ScreenId, main_thread: ThreadId) {
        let (unserved, unserved_calls) = observer();
        waiter.next_vsync(
            NO_SCREEN,
            Arc::downgrade(&unserved) as Weak<dyn VsyncObserver>,
        );
        let (served, calls) = observer();
        let weak = Arc::downgrade(&served) as Weak<dyn VsyncObserver>;
        for wait in 0..WAITS {
            waiter.next_vsync(screen, weak.clone());
            waiter.next_vsync(screen, weak.clone());
            match calls.recv_timeout(PATIENCE) {
                Ok(thread) if thread == main_thread => {}
                Ok(_) => support::fail(TEST, "a wait was served off the main thread"),
                Err(_) => support::fail(TEST, &format!("wait {wait} was not served")),
            }
        }
        std::thread::sleep(QUIET);
        if calls.try_recv().is_ok() {
            support::fail(
                TEST,
                "a wait asked twice before its refresh was served twice",
            );
        }
        if unserved_calls.try_recv().is_ok() {
            support::fail(TEST, "a wait on no screen was served");
        }
        support::pass(TEST);
    }

    impl EmbedderClient for WaitingClient {
        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}

        fn wake(&mut self, _elapsed: Duration) {}

        fn view_added(&mut self, id: ViewId) {
            let view = self.platform.view(id).expect("the platform lists the view");
            let Some(screen) = view.metrics().screen else {
                support::fail(TEST, "the view is on no screen");
            };
            let Some(waiter) = self.platform.vsync_waiter() else {
                support::fail(TEST, "the platform has no vsync waiter");
            };
            let main_thread = self.main_thread;
            std::thread::spawn(move || wait_on(waiter, screen, main_thread));
        }

        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_reports(&mut self, _reports: Vec<PointerReport>) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

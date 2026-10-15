//! A press on a window's close button only reports it to the app, which decides: the window stays
//! open, the last one included, until the app closes it, and the app runs on with no window
//! after that.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for about a second, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_window_stays_open_until_the_app_closes_it";

/// Long enough for the window to open, be asked to close and close, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the steps did not end in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::DecidingClient::new);
    support::fail(TEST, "the app quit with its last window");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, RequestId, ViewId,
        WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    const TITLE: &str = "the last window";

    /// How long the app keeps the window open after the request, and runs on after closing it.
    const WAIT: Duration = Duration::from_millis(400);

    /// What the app has done so far.
    enum Step {
        /// Waiting for its window, to press its close button.
        Opening,
        /// The close button was pressed.
        Pressed,
        /// The request came in at this moment; the window is kept open until `WAIT` after it.
        Asked(Instant),
        /// The app closed the window at this moment, and runs on until `WAIT` after it.
        Closed(Instant),
    }

    /// Opens one window, presses its close button, keeps it open a while after the request, then
    /// closes it, and runs on with no window.
    pub(crate) struct DecidingClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        windows: Vec<WindowRef>,
        step: Step,
    }

    impl DecidingClient {
        pub(crate) fn new(platform: PlatformRef) -> DecidingClient {
            let requested = vec![support::request_window(&platform, support::titled(TITLE))];
            DecidingClient {
                platform,
                requested,
                windows: Vec::new(),
                step: Step::Opening,
            }
        }

        fn wake_after(&self, wait: Duration) {
            self.platform
                .dispatcher()
                .wake_at(Some(Instant::now() + wait));
        }

        fn press_close_once_open(&mut self) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            if self.windows.is_empty() {
                return;
            }
            appkit::press_close_button(TITLE);
            self.step = Step::Pressed;
        }

        /// Closes the window once it stayed open long enough after the request.
        fn close_once_kept_open(&mut self, asked_at: Instant) {
            if asked_at.elapsed() < WAIT {
                return;
            }
            if !appkit::lists_window_titled(TITLE) {
                support::fail(TEST, "the window closed before the app closed it");
            }
            self.windows.remove(0).close();
            self.step = Step::Closed(Instant::now());
            self.wake_after(WAIT);
        }

        fn pass_once_it_ran_on(&self, closed_at: Instant) {
            if closed_at.elapsed() < WAIT {
                return;
            }
            if !self.platform.views().is_empty() {
                support::fail(TEST, "a view is left");
            }
            support::pass(TEST);
        }
    }

    impl EmbedderClient for DecidingClient {
        fn view_close_requested(&mut self, _view: ViewId) {
            if matches!(self.step, Step::Pressed) {
                self.step = Step::Asked(Instant::now());
                self.wake_after(WAIT);
            }
        }

        fn wake(&mut self, _elapsed: Duration) {
            match self.step {
                Step::Opening => self.press_close_once_open(),
                Step::Pressed => {}
                Step::Asked(asked_at) => self.close_once_kept_open(asked_at),
                Step::Closed(closed_at) => self.pass_once_it_ran_on(closed_at),
            }
        }

        fn frame(&mut self, _frame: Frame) {}
        fn view_added(&mut self, _id: ViewId) {
            if matches!(self.step, Step::Opening) {
                self.press_close_once_open();
            }
        }
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn view_removed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

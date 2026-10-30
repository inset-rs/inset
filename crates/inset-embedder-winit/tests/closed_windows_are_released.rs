//! A window that closes is released, not only forgotten, even while the app still holds its
//! `WindowRef`: one whose close button was pressed, which the app closes when asked to, as the
//! framework does by default, and one the app closes on its own. Both leave the screen and AppKit's list of the application's windows, and their
//! `NSWindow`s are freed.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens three windows for about a second, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "closed_windows_are_released";

/// Long enough for the windows to open and close, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("no answer in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::ClosingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, RequestId, ViewId,
        WindowConfig, WindowRef,
    };

    use crate::TEST;
    use crate::support::{self, appkit};

    const CLOSED_BY_ITS_BUTTON: &str = "closed by its close button";
    const CLOSED_BY_THE_APP: &str = "closed by the app";
    const KEPT_OPEN: &str = "kept open";

    /// How long the embedder is given to let the closed windows go.
    const SETTLE: Duration = Duration::from_millis(500);

    /// Opens three windows, closes two of them once all are made, one through its close button
    /// and one through its `WindowRef`, holding on to every `WindowRef`, and looks for the two
    /// closed ones in AppKit a moment later. The third keeps the event loop running.
    pub(crate) struct ClosingClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        /// Every window made, held to the end, as an app that keeps its handles holds them.
        windows: Vec<WindowRef>,
        /// The closed windows as AppKit had them, held weakly, once they are closed.
        closed: Option<Vec<(&'static str, appkit::WeakWindow)>>,
        closed_at: Instant,
    }

    impl ClosingClient {
        pub(crate) fn new(platform: PlatformRef) -> ClosingClient {
            let requested = [CLOSED_BY_ITS_BUTTON, CLOSED_BY_THE_APP, KEPT_OPEN]
                .into_iter()
                .map(|title| {
                    support::request_window(
                        &platform,
                        WindowConfig {
                            title: title.into(),
                            size: [300.0, 200.0],
                            ..WindowConfig::default()
                        },
                    )
                })
                .collect();
            ClosingClient {
                platform,
                requested,
                windows: Vec::new(),
                closed: None,
                closed_at: Instant::now(),
            }
        }

        /// Closes the first two windows once all three are made, and starts asking for frames of
        /// the third, which time the look that follows.
        fn close_two_once_all_are_made(&mut self) {
            if self.closed.is_some() || self.windows.len() < 3 {
                return;
            }
            let closed = [CLOSED_BY_ITS_BUTTON, CLOSED_BY_THE_APP]
                .map(|title| (title, appkit::WeakWindow::titled(title)));
            appkit::press_close_button(CLOSED_BY_ITS_BUTTON);
            self.windows[1].close();
            self.closed = Some(closed.into());
            self.closed_at = Instant::now();
            self.request_frame_of_the_window_kept_open();
        }

        fn request_frame_of_the_window_kept_open(&self) {
            self.platform.request_frame(self.windows[2].view().id());
        }

        /// What is left of the closed windows: each still in AppKit's list of the application's
        /// windows, or not yet freed.
        fn what_is_left(&self) -> Vec<String> {
            let mut left = Vec::new();
            for (title, window) in self.closed.iter().flatten() {
                if appkit::lists_window_titled(title) {
                    left.push(format!("{title}: still among the application's windows"));
                }
                if window.is_alive() {
                    left.push(format!("{title}: its NSWindow is not freed"));
                }
            }
            left
        }
    }

    impl EmbedderClient for ClosingClient {
        fn view_close_requested(&mut self, view: ViewId) {
            if let Some(window) = self
                .windows
                .iter()
                .find(|window| window.view().id() == view)
            {
                window.close();
            }
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {
            if self.closed.is_none() {
                return;
            }
            if self.closed_at.elapsed() < SETTLE {
                self.request_frame_of_the_window_kept_open();
                return;
            }
            let left = self.what_is_left();
            if !left.is_empty() {
                support::fail(TEST, &format!("{left:?}"));
            }
            support::pass(TEST);
        }

        fn wake(&mut self, _elapsed: Duration) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.close_two_once_all_are_made();
        }

        fn view_added(&mut self, _id: ViewId) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.windows);
            self.close_two_once_all_are_made();
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

//! Closing the last window asks the app whether it quits, as Cmd-Q does, and an app that cancels
//! runs on with no window: its view goes, and it is still woken a while later.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "an_app_that_cancels_outlives_its_last_window";

/// Long enough for the window to open and close and the app to run on, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("the app was not woken with no window in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::CancellingClient::new);
    support::fail(TEST, "the app quit although it cancelled");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        AppExitFuture, AppExitResponse, EmbedderClient, Frame, KeyData, PlatformRef,
        PointerDataPacket, ViewId,
    };

    use crate::TEST;
    use crate::support::{self, WindowClosedOnceMade};

    /// How long the app runs on with no window before it passes.
    const RUNS_ON_FOR: Duration = Duration::from_millis(500);

    /// Opens one window, closes it once it is made, cancels the quit it is asked about, and asks
    /// to be woken a while after its view goes.
    pub(crate) struct CancellingClient {
        platform: PlatformRef,
        window: WindowClosedOnceMade,
        asked_to_quit: bool,
        closed_at: Option<Instant>,
    }

    impl CancellingClient {
        pub(crate) fn new(platform: PlatformRef) -> CancellingClient {
            CancellingClient {
                window: WindowClosedOnceMade::open(&platform, "the last window"),
                platform,
                asked_to_quit: false,
                closed_at: None,
            }
        }

        /// Passes once the app has run on with no window for long enough after it cancelled.
        fn pass_once_it_ran_on(&self) {
            let Some(closed_at) = self.closed_at else {
                return;
            };
            if closed_at.elapsed() < RUNS_ON_FOR {
                return;
            }
            if !self.asked_to_quit {
                support::fail(TEST, "the app was not asked whether it quits");
            }
            if !self.platform.views().is_empty() {
                support::fail(TEST, "a view is left");
            }
            support::pass(TEST);
        }
    }

    impl EmbedderClient for CancellingClient {
        fn request_app_exit(&mut self) -> AppExitFuture {
            self.asked_to_quit = true;
            Box::pin(std::future::ready(AppExitResponse::Cancel))
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.window.close_once_made();
            self.pass_once_it_ran_on();
        }

        fn view_removed(&mut self, _id: ViewId) {
            let now = Instant::now();
            self.closed_at = Some(now);
            self.platform.dispatcher().wake_at(Some(now + RUNS_ON_FOR));
        }

        fn frame(&mut self, _frame: Frame) {}
        fn view_added(&mut self, _id: ViewId) {}
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

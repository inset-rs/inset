//! Closing the last window asks the app whether it quits, as Cmd-Q does, and an app that answers
//! exit quits: on macOS through AppKit's `terminate:`, which ends the process with status 0.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window for a moment, and needs a login session's window server. The watchdog ends the
//! process with status 1 if the quit never comes, and so does a loop that ends without it.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "an_app_that_agrees_quits_with_its_last_window";

/// Long enough for the window to open and close and the quit to come, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("no quit in {GIVE_UP_AFTER:?} after the last window closed")
    });
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::AgreeingClient::new);
    support::fail(TEST, "the loop ended without quitting the process");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{
        AppExitFuture, AppExitResponse, EmbedderClient, Frame, KeyData, PlatformRef,
        PointerDataPacket, ViewId,
    };

    use crate::TEST;
    use crate::support::WindowClosedOnceMade;

    /// Opens one window, closes it once it is made, and agrees to quit when asked.
    pub(crate) struct AgreeingClient {
        window: WindowClosedOnceMade,
    }

    impl AgreeingClient {
        pub(crate) fn new(platform: PlatformRef) -> AgreeingClient {
            AgreeingClient {
                window: WindowClosedOnceMade::open(&platform, "the last window"),
            }
        }
    }

    impl EmbedderClient for AgreeingClient {
        fn request_app_exit(&mut self) -> AppExitFuture {
            println!("test {TEST} ... asked, and agreed; the quit ends the process");
            Box::pin(std::future::ready(AppExitResponse::Exit))
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.window.close_once_made();
        }

        fn frame(&mut self, _frame: Frame) {}
        fn view_added(&mut self, _id: ViewId) {}
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

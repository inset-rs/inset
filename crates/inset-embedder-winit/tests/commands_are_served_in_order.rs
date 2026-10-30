//! What the application asks through the platform is served in the order it asked, each on a turn
//! of the loop of its own: a window's close asked for before another window's opening is served
//! first, so the first view goes before the second comes. Served by kind, openings before closes,
//! the second view would come first.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for a moment, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "commands_are_served_in_order";

/// Long enough for the windows to open and close, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("not everything asked for was served in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::RecordingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{
        EmbedderClient, Frame, KeyData, PlatformRef, PointerReport, RequestId, ViewId,
        WindowConfig, WindowRef,
    };

    use crate::{TEST, support};

    /// What the client hears of the embedder, in the order heard.
    #[derive(Debug, PartialEq)]
    enum Heard {
        ViewAdded(ViewId),
        ViewRemoved(ViewId),
    }

    /// Served in the order asked: the first window opens, then closes, then the second opens.
    /// The windows' views are numbered as they open, after the screen view's number, which a
    /// desktop leaves unused.
    const IN_THE_ORDER_ASKED: [Heard; 3] = [
        Heard::ViewAdded(ViewId(1)),
        Heard::ViewRemoved(ViewId(1)),
        Heard::ViewAdded(ViewId(2)),
    ];

    /// Asks for a window at its start; once it is made, asks for its close and then another
    /// window in one go, and records what it hears.
    pub(crate) struct RecordingClient {
        platform: PlatformRef,
        requested: Vec<RequestId>,
        first: Vec<WindowRef>,
        asked_close_then_open: bool,
        heard: Vec<Heard>,
    }

    impl RecordingClient {
        pub(crate) fn new(platform: PlatformRef) -> RecordingClient {
            let first = open_window(&platform, "closed before the second opens");
            RecordingClient {
                platform,
                requested: vec![first],
                first: Vec::new(),
                asked_close_then_open: false,
                heard: Vec::new(),
            }
        }

        /// Once the first window is made, closes it and asks for the second in the same call.
        fn ask_close_then_open(&mut self) {
            support::take_opened_windows(&self.platform, &mut self.requested, &mut self.first);
            let Some(first) = self.first.pop() else {
                return;
            };
            if self.asked_close_then_open {
                return;
            }
            self.asked_close_then_open = true;
            first.close();
            self.requested
                .push(open_window(&self.platform, "opened after the first closes"));
        }

        /// Records `heard`, and judges the order once everything asked for was served.
        fn hear(&mut self, heard: Heard) {
            self.heard.push(heard);
            if self.heard.len() < IN_THE_ORDER_ASKED.len() {
                return;
            }
            if self.heard != IN_THE_ORDER_ASKED {
                support::fail(TEST, &format!("heard {:?}", self.heard));
            }
            support::pass(TEST);
        }
    }

    fn open_window(platform: &PlatformRef, title: &str) -> RequestId {
        support::request_window(
            platform,
            WindowConfig {
                title: title.into(),
                size: [200.0, 100.0],
                ..WindowConfig::default()
            },
        )
    }

    impl EmbedderClient for RecordingClient {
        fn view_added(&mut self, id: ViewId) {
            self.hear(Heard::ViewAdded(id));
            self.ask_close_then_open();
        }

        fn view_removed(&mut self, id: ViewId) {
            self.hear(Heard::ViewRemoved(id));
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.ask_close_then_open();
        }

        fn frame(&mut self, _views: &[ViewId], _frame: Frame) {}
        fn view_metrics_changed(&mut self, _id: ViewId) {}
        fn pointer_reports(&mut self, _reports: Vec<PointerReport>) {}
        fn key_data(&mut self, _data: KeyData) -> bool {
            false
        }
        fn platform_brightness_changed(&mut self) {}
        fn locales_changed(&mut self) {}
    }
}

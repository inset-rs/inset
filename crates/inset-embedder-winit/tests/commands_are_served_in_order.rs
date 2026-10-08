//! What the application asks through the platform is served in the order it asked, each on a turn
//! of the loop of its own: a window asked for before an exit opens before the framework is asked
//! whether the application may quit, and one asked for after the exit opens after. Served by kind,
//! windows before exits, as they once were, both windows came first.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens two windows for a moment, and needs a login session's window server.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "commands_are_served_in_order";

/// Long enough for the windows to open and the exit to be asked, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("not everything asked for was served in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::RecordingClient::new);
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::Duration;

    use inset_embedder::{
        AppExitFuture, AppExitResponse, AppExitType, EmbedderClient, Frame, KeyData, PlatformRef,
        PointerDataPacket, ViewId, WindowConfig, WindowFuture,
    };

    use crate::{TEST, support};

    /// What the client hears of the embedder, in the order heard.
    #[derive(Debug, PartialEq)]
    enum Heard {
        ViewAdded(ViewId),
        AskedWhetherToQuit,
    }

    /// Served in the order asked: the first window, the exit, the second window. The windows'
    /// views are numbered as they open, after the implicit view's number, which goes unused here.
    const IN_THE_ORDER_ASKED: [Heard; 3] = [
        Heard::ViewAdded(ViewId(1)),
        Heard::AskedWhetherToQuit,
        Heard::ViewAdded(ViewId(2)),
    ];

    /// Asks for a window, an exit and another window at its start, and records what it hears.
    pub(crate) struct RecordingClient {
        _opening: Vec<WindowFuture>,
        /// The application's exit request, held so its answer is heard.
        _exit: AppExitFuture,
        heard: Vec<Heard>,
    }

    impl RecordingClient {
        pub(crate) fn new(platform: PlatformRef) -> RecordingClient {
            let owner = platform
                .windowing_owner()
                .expect("a desktop host makes windows");
            let window = |title: &str| {
                owner.create(WindowConfig {
                    title: title.into(),
                    size: [200.0, 100.0],
                    ..WindowConfig::default()
                })
            };
            let first = window("asked before the exit");
            let exit = platform
                .app_exit()
                .expect("a macOS host quits itself")
                .exit_application(AppExitType::Cancelable, 0);
            let second = window("asked after the exit");
            RecordingClient {
                _opening: vec![first, second],
                _exit: exit,
                heard: Vec::new(),
            }
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

    impl EmbedderClient for RecordingClient {
        fn view_added(&mut self, id: ViewId) {
            self.hear(Heard::ViewAdded(id));
        }

        /// No, so the application keeps running until the test ends it.
        fn request_app_exit(&mut self) -> AppExitFuture {
            self.hear(Heard::AskedWhetherToQuit);
            Box::pin(std::future::ready(AppExitResponse::Cancel))
        }

        fn frame(&mut self, _frame: Frame) {}
        fn wake(&mut self, _elapsed: Duration) {}
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

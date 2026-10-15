//! The system's request to quit is answered long after it was asked, from a wake of its own with
//! nothing else happening after it, and the answer still quits: the reply reaches the event loop
//! as a request of its own, served on a turn of its own.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens no window. The request is AppKit's, made as Cmd-Q makes it, and the quit is AppKit's
//! `terminate:`, which ends the process with status 0; the watchdog ends it with status 1 if the
//! quit never comes.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_late_answer_to_an_exit_quits";

/// Long enough for the request, the answer and the quit, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("no quit in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder.run(client::LateAnsweringClient::new);
    support::fail(TEST, "the loop returned without quitting the process");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::time::{Duration, Instant};

    use inset_embedder::{
        AppExitReply, AppExitResponse, EmbedderClient, Frame, KeyData, PlatformRef,
        PointerDataPacket, ViewId,
    };

    use crate::{TEST, support};

    /// How long the framework takes to answer: long enough for every turn the request brought
    /// to have passed, so that only the answer can bring the loop round again.
    const ANSWER_AFTER: Duration = Duration::from_millis(300);

    /// Presses Cmd-Q at its first wake, and answers the request to quit only from a wake a while
    /// later, as the framework answers once its observers have.
    pub(crate) struct LateAnsweringClient {
        platform: PlatformRef,
        pressed_quit: bool,
        /// The request to quit, once asked, and when to answer it.
        request: Option<(AppExitReply, Instant)>,
    }

    impl LateAnsweringClient {
        pub(crate) fn new(platform: PlatformRef) -> LateAnsweringClient {
            platform.dispatcher().wake_at(Some(Instant::now()));
            LateAnsweringClient {
                platform,
                pressed_quit: false,
                request: None,
            }
        }

        fn press_quit_once(&mut self) {
            if !self.pressed_quit {
                self.pressed_quit = true;
                support::appkit::press_quit();
            }
        }

        fn answer_exit_when_due(&mut self) {
            let Some((_, answer_at)) = &self.request else {
                return;
            };
            if Instant::now() < *answer_at {
                return;
            }
            let (reply, _) = self.request.take().expect("the request");
            println!("test {TEST} ... answered; the quit ends the process");
            reply.answer(AppExitResponse::Exit);
        }
    }

    impl EmbedderClient for LateAnsweringClient {
        fn request_app_exit(&mut self, reply: AppExitReply) {
            let answer_at = Instant::now() + ANSWER_AFTER;
            self.platform.dispatcher().wake_at(Some(answer_at));
            self.request = Some((reply, answer_at));
        }

        fn wake(&mut self, _elapsed: Duration) {
            self.press_quit_once();
            self.answer_exit_when_due();
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

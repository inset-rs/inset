//! An exit the application asks for quits once the framework answers yes, even when the answer
//! comes on a later wake and nothing else happens after it: the answer reaches the event loop
//! through the waker the pending quit was polled with, which asks the loop for a turn.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens no window. On macOS the quit is AppKit's `terminate:`, which ends the process with status
//! 0; the watchdog ends it with status 1 if the quit never comes.

#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
const TEST: &str = "a_late_answer_to_an_exit_quits";

/// Long enough for the answer and the quit, many times over.
#[cfg(target_os = "macos")]
const GIVE_UP_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(target_os = "macos")]
fn main() {
    use inset_embedder_winit::WinitEmbedder;

    support::fail_after(TEST, GIVE_UP_AFTER, || {
        format!("no quit in {GIVE_UP_AFTER:?}")
    });
    WinitEmbedder::default()
        .implicit_view(None)
        .run(client::LateAnsweringClient::new);
    support::fail(TEST, "the loop returned without quitting the process");
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod client {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use inset_embedder::{
        AppExitFuture, AppExitResponse, AppExitType, EmbedderClient, Frame, KeyData, PlatformRef,
        PointerDataPacket, ViewId,
    };

    use crate::TEST;

    /// How long the framework takes to answer: long enough for every turn the start asked for
    /// to have passed, so that only the answer's own waker can bring the loop round again.
    const ANSWER_AFTER: Duration = Duration::from_millis(300);

    /// Asks to quit at its start, and answers the embedder's question of whether it may quit only
    /// from a wake a while later, as the framework answers once its observers have.
    pub(crate) struct LateAnsweringClient {
        /// The application's own exit request, held so its answer is heard.
        _exit: AppExitFuture,
        answer_at: Instant,
        answer: Rc<RefCell<Answer>>,
    }

    /// The framework's answer to the embedder, once it is given, and the waker to call then.
    #[derive(Default)]
    struct Answer {
        response: Option<AppExitResponse>,
        waker: Option<Waker>,
    }

    impl LateAnsweringClient {
        pub(crate) fn new(platform: PlatformRef) -> LateAnsweringClient {
            let exit = platform
                .app_exit()
                .expect("a macOS host quits itself")
                .exit_application(AppExitType::Cancelable, 0);
            let answer_at = Instant::now() + ANSWER_AFTER;
            platform.dispatcher().wake_at(Some(answer_at));
            LateAnsweringClient {
                _exit: exit,
                answer_at,
                answer: Rc::default(),
            }
        }

        /// Answers yes once the embedder has asked and the time has come; answers whether it did.
        fn answer_yes_when_due(&self) -> bool {
            let waker = {
                let mut answer = self.answer.borrow_mut();
                if Instant::now() < self.answer_at || answer.response.is_some() {
                    return false;
                }
                answer.response = Some(AppExitResponse::Exit);
                answer.waker.take()
            };
            let waker = waker.expect("the host asked before the answer was due");
            waker.wake();
            true
        }
    }

    impl EmbedderClient for LateAnsweringClient {
        fn request_app_exit(&mut self) -> AppExitFuture {
            let answer = Rc::clone(&self.answer);
            Box::pin(std::future::poll_fn(move |context: &mut Context<'_>| {
                let mut answer = answer.borrow_mut();
                match answer.response {
                    Some(response) => Poll::Ready(response),
                    None => {
                        answer.waker = Some(context.waker().clone());
                        Poll::Pending
                    }
                }
            }))
        }

        fn wake(&mut self, _elapsed: Duration) {
            if self.answer_yes_when_due() {
                println!("test {TEST} ... answered; the quit ends the process");
            }
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

//! Quitting, both ways Flutter has: the embedder asking the framework whether the application quits
//! (`System.requestAppExit`), with the reply its answer comes back through, and the application
//! quitting itself (`System.exitApplication` with a required exit, which is `AppExit::quit`; the
//! framework asks itself before a cancelable one).
//!
//! The framework may answer long after it is asked, such as once the user has chosen whether to
//! save, and answers inside a call into the client. So both the answer and the application's quit
//! are app requests, which the loop serves on turns of their own (`main_loop/quitting.rs`): the
//! loop is handling an event when it quits, never about to wait, where winit's loop on Windows
//! does not see a quit until some other message comes.

use std::rc::Rc;

use inset_embedder::{AppExit, AppExitReply};

use crate::WinitPlatform;
use crate::app_requests::{AppRequest, AppRequests};

impl AppExit for WinitPlatform {
    /// The exit code is ignored, as Flutter's macOS and Linux embedders ignore it.
    fn quit(&self, _exit_code: i32) {
        self.send(AppRequest::Quit);
    }
}

/// The reply to one request to exit: its answer goes to the loop through `requests`.
pub(crate) fn exit_reply(requests: Rc<AppRequests>) -> AppExitReply {
    AppExitReply::new(move |response| requests.send(AppRequest::AnswerExit(response)))
}

#[cfg(test)]
mod tests {
    use inset_embedder::AppExitResponse;

    use super::*;

    fn requests() -> Rc<AppRequests> {
        Rc::new(AppRequests::new(|_event| {}))
    }

    fn answer_sent(requests: &AppRequests) -> Option<AppExitResponse> {
        match requests.take_next() {
            Some(AppRequest::AnswerExit(response)) => Some(response),
            _ => None,
        }
    }

    #[test]
    fn an_answer_goes_to_the_loop() {
        let requests = requests();
        exit_reply(Rc::clone(&requests)).answer(AppExitResponse::Cancel);
        assert_eq!(answer_sent(&requests), Some(AppExitResponse::Cancel));
        assert_eq!(answer_sent(&requests), None, "answered once");
    }

    #[test]
    fn a_reply_let_go_of_unanswered_answers_exit() {
        let requests = requests();
        drop(exit_reply(Rc::clone(&requests)));
        assert_eq!(answer_sent(&requests), Some(AppExitResponse::Exit));
    }
}

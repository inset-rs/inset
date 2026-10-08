//! Quitting, both ways Flutter has: the embedder asking the framework whether the application quits
//! (`System.requestAppExit`), and the application asking to quit (`System.exitApplication`, which
//! is `AppExit::exit_application`). The embedder asks when the system asks the application to quit,
//! when the last window closes, and when the application asks with a cancelable exit; a required
//! one quits without asking.
//!
//! The framework may answer long after it is asked, such as once the user has chosen whether to
//! save, so each question stays open in [`QuitQuestions`] until its answer is in, and the loop
//! then quits or keeps running (`main_loop/`).

use std::task::{Context, Poll, Waker};

use inset_embedder::{AppExit, AppExitFuture, AppExitResponse, AppExitType};

use crate::WinitPlatform;
use crate::app_requests::AppRequest;
use crate::reply::{Reply, reply};

impl AppExit for WinitPlatform {
    /// Served by the event loop on a turn of its own, since asking the framework calls the client.
    /// The exit code is ignored, as Flutter's macOS and Linux embedders ignore it.
    fn exit_application(&self, exit_type: AppExitType, _exit_code: i32) -> AppExitFuture {
        let (reply, response) = reply();
        self.send(AppRequest::Exit(ExitRequest { exit_type, reply }));
        Box::pin(response)
    }
}

/// An exit the application asked for, on its way to the loop; the reply answers it.
pub(crate) struct ExitRequest {
    pub exit_type: AppExitType,
    pub reply: Reply<AppExitResponse>,
}

/// The framework's answers to whether the application quits that are not in yet.
#[derive(Default)]
pub(crate) struct QuitQuestions(Vec<QuitQuestion>);

struct QuitQuestion {
    answer: AppExitFuture,
    /// `exit_application`'s future, when the application asked: it hears the answer too.
    app_reply: Option<Reply<AppExitResponse>>,
}

/// What the answers that came in decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuitDecision {
    Quit,
    KeepRunning,
}

/// The answers that came in: what they decide, and what the application hears of the exits it
/// asked for.
pub(crate) struct QuitAnswers {
    /// `None` while nothing is decided.
    pub decision: Option<QuitDecision>,
    /// Each exit the application asked for that is answered, with its answer.
    pub to_the_app: Vec<(Reply<AppExitResponse>, AppExitResponse)>,
}

impl QuitQuestions {
    /// Keeps `answer`, the framework's answer on its way, until it is in; `app_reply` hears it
    /// then.
    pub(crate) fn ask(&mut self, answer: AppExitFuture, app_reply: Option<Reply<AppExitResponse>>) {
        self.0.push(QuitQuestion { answer, app_reply });
    }

    /// Takes the answers that are in. The application quits as soon as any answer is exit, and
    /// keeps running once every question is answered and none was.
    ///
    /// The answers are polled without a waker: the framework answers only inside a call into the
    /// client, and the loop calls this at the end of every turn.
    pub(crate) fn take_answers(&mut self) -> QuitAnswers {
        let mut context = Context::from_waker(Waker::noop());
        let mut responses = Vec::new();
        let mut to_the_app = Vec::new();
        self.0.retain_mut(
            |question| match question.answer.as_mut().poll(&mut context) {
                Poll::Ready(response) => {
                    if let Some(app_reply) = question.app_reply.take() {
                        to_the_app.push((app_reply, response));
                    }
                    responses.push(response);
                    false
                }
                Poll::Pending => true,
            },
        );
        let decision = if responses.contains(&AppExitResponse::Exit) {
            Some(QuitDecision::Quit)
        } else if !responses.is_empty() && self.0.is_empty() {
            Some(QuitDecision::KeepRunning)
        } else {
            None
        };
        QuitAnswers {
            decision,
            to_the_app,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use super::*;
    use crate::reply::ReplyFuture;

    fn answered(response: AppExitResponse) -> AppExitFuture {
        Box::pin(std::future::ready(response))
    }

    /// An answer the test gives later, through the reply.
    fn answered_later() -> (Reply<AppExitResponse>, AppExitFuture) {
        let (framework, answer) = reply();
        (framework, Box::pin(answer))
    }

    fn heard(app: &mut ReplyFuture<AppExitResponse>) -> Poll<AppExitResponse> {
        Pin::new(app).poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn nothing_is_decided_until_an_answer_is_in() {
        let mut questions = QuitQuestions::default();
        assert_eq!(questions.take_answers().decision, None);
        let (framework, answer) = answered_later();
        questions.ask(answer, None);
        assert_eq!(questions.take_answers().decision, None);

        framework.complete(AppExitResponse::Exit);
        assert_eq!(questions.take_answers().decision, Some(QuitDecision::Quit));
        assert_eq!(questions.take_answers().decision, None, "decided once");
    }

    #[test]
    fn any_exit_quits_whatever_the_other_answers() {
        let mut questions = QuitQuestions::default();
        questions.ask(answered(AppExitResponse::Exit), None);
        questions.ask(answered(AppExitResponse::Cancel), None);
        assert_eq!(questions.take_answers().decision, Some(QuitDecision::Quit));
    }

    #[test]
    fn the_app_keeps_running_once_every_question_is_answered_no() {
        let mut questions = QuitQuestions::default();
        let (framework, answer) = answered_later();
        questions.ask(answered(AppExitResponse::Cancel), None);
        questions.ask(answer, None);
        assert_eq!(
            questions.take_answers().decision,
            None,
            "a question is still open"
        );

        framework.complete(AppExitResponse::Cancel);
        assert_eq!(
            questions.take_answers().decision,
            Some(QuitDecision::KeepRunning)
        );
    }

    #[test]
    fn the_answer_to_an_exit_the_application_asked_for_is_handed_back_for_it() {
        let mut questions = QuitQuestions::default();
        let (app_reply, mut app) = reply();
        questions.ask(answered(AppExitResponse::Cancel), Some(app_reply));

        let answers = questions.take_answers();
        assert_eq!(answers.decision, Some(QuitDecision::KeepRunning));
        let [(reply, response)] = <[_; 1]>::try_from(answers.to_the_app)
            .ok()
            .expect("one answer");
        assert_eq!(response, AppExitResponse::Cancel);
        assert!(
            heard(&mut app).is_pending(),
            "not told until the answer is delivered"
        );
        reply.complete(response);
        assert_eq!(heard(&mut app), Poll::Ready(AppExitResponse::Cancel));
    }
}

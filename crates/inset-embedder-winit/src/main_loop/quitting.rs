//! The main loop's quitting (`app_exit.rs`): the three ways a quit starts, and acting on the
//! framework's answers.

use inset_embedder::{AppExitResponse, AppExitType, EmbedderClient};
use winit::event_loop::ActiveEventLoop;

use crate::app_exit::{ExitRequest, QuitDecision};
use crate::os::{ApplicationControl, ThisOs};
use crate::reply::Reply;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// The system asks the application to quit: Cmd-Q, Quit from the Dock, logging out.
    pub(super) fn on_quit_requested(&mut self) {
        self.ask_whether_to_quit(None);
    }

    /// The application asks to quit: a required exit quits now, a cancelable one asks the
    /// framework first, as Flutter's embedders serve `System.exitApplication`.
    pub(super) fn serve_exit(&mut self, event_loop: &ActiveEventLoop, request: ExitRequest) {
        match request.exit_type {
            AppExitType::Required => {
                request.reply.complete(AppExitResponse::Exit);
                ThisOs::quit(event_loop);
            }
            AppExitType::Cancelable => self.ask_whether_to_quit(Some(request.reply)),
        }
    }

    /// The last window closed, where the embedder is set to quit then: the app is asked, as
    /// AppKit asks it once Flutter's macOS template answers that the last window's closing quits,
    /// and it runs on with no window if it cancels.
    pub(super) fn on_last_window_closed(&mut self) {
        self.ask_whether_to_quit(None);
    }

    /// Asks the framework whether the application quits; `app_reply` hears the answer when the
    /// application asked.
    pub(super) fn ask_whether_to_quit(&mut self, app_reply: Option<Reply<AppExitResponse>>) {
        ThisOs::quit_being_decided();
        let answer = self.client.request_app_exit();
        self.quit_questions.ask(answer, app_reply);
    }

    /// Quits or keeps running once the framework's answers decide it. Called at the end of every
    /// turn, since the framework answers only inside a call into the client.
    pub(super) fn act_on_quit_answers(&mut self, event_loop: &ActiveEventLoop) {
        let answers = self.quit_questions.take_answers();
        if !answers.to_the_app.is_empty() {
            for (reply, response) in answers.to_the_app {
                reply.complete(response);
            }
            // As with a window the app asked for, a wake brings the app's checkpoint.
            self.wake_client();
        }
        match answers.decision {
            Some(QuitDecision::Quit) => ThisOs::quit(event_loop),
            Some(QuitDecision::KeepRunning) => ThisOs::keep_running(),
            None => {}
        }
    }
}

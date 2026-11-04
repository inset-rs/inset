//! The main loop's quitting (`app_exit.rs`): asking the framework when the system asks the
//! application to quit, and quitting on its answer or on the application's own quit.

use std::rc::Rc;

use inset_embedder::{AppExitResponse, EmbedderClient};
use winit::event_loop::ActiveEventLoop;

use crate::app_exit::exit_reply;
use crate::app_requests::ExitRequest;
use crate::os::System;
use crate::os::capabilities::ApplicationControl;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// Serves the app's exit: its own quit, or its answer to the system's request to quit.
    pub(super) fn on_exit_request(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        request: ExitRequest,
    ) {
        match request {
            ExitRequest::Quit => self.quit(event_loop),
            ExitRequest::Answer(response) => self.on_exit_answered(event_loop, response),
        }
    }

    /// The system asks the application to quit: Cmd-Q, Quit from the Dock, logging out. Every
    /// request reaches the framework, a second one while the first is being decided included, so
    /// the application decides each, such as quitting only at a second Cmd-Q.
    pub(super) fn on_quit_requested(&mut self) {
        let reply = exit_reply(Rc::clone(self.state.requests()));
        self.client.request_app_exit(reply);
    }

    /// The framework answered a request to exit: the application quits on exit, and runs on
    /// otherwise, with nothing to undo.
    pub(super) fn on_exit_answered(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        response: AppExitResponse,
    ) {
        if response == AppExitResponse::Exit {
            self.quit(event_loop);
        }
    }

    /// Quits the way the system quits an application: on the framework's answer, or for the
    /// application's own required exit.
    pub(super) fn quit(&mut self, event_loop: &dyn ActiveEventLoop) {
        System::quit(event_loop);
    }
}

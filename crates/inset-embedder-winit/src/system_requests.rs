//! What the system asks of the application as a whole: files and links to open, which reach the
//! client as an open request (`EmbedderClient::open_requested`); showing itself again, handed to
//! the handler the app sets through [`WinitPlatform::macos`](crate::WinitPlatform::macos); and
//! quitting, which goes to the framework instead, through the client (`app_exit.rs`).
//!
//! macOS asks to open through its application delegate, at launch and while the app runs
//! (`application:openURLs:`), and iOS through the application's delegate
//! (`application:openURL:options:`); Windows and Linux start the app with its files and links on
//! its command line, which the launch reads and sends as a request of its own, served on the
//! loop's first turn.
//!
//! The main loop calls a handler outside every framework call, so a handler may post to the app.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use inset_embedder::{Offset, OpenRequest, PickedFile};

/// A request the system makes of the application as a whole.
#[derive(Debug)]
pub(crate) enum SystemRequest {
    /// Open these files and links: documents opened from the system's file browser or dropped on
    /// the application's icon, links whose scheme is the app's, or a launch's arguments.
    Open {
        files: Vec<PathBuf>,
        links: Vec<String>,
    },
    /// Show yourself again: the application's icon was clicked, or the application opened
    /// again while it runs.
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "only macOS asks to reopen")
    )]
    Reopen(Reopen),
    /// Quit if the application agrees: the user chose Quit, or is logging out. The framework is
    /// asked, and an exit is carried out with
    /// [`ApplicationControl::quit`](crate::os::ApplicationControl::quit); a cancel changes
    /// nothing.
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "only macOS asks to quit")
    )]
    Quit,
}

impl SystemRequest {
    /// The files and links a launch's arguments ask to open, as Windows and Linux start an app with
    /// them on its command line; `None` when they ask for nothing.
    pub(crate) fn launch() -> Option<SystemRequest> {
        let current_dir = std::env::current_dir().unwrap_or_default();
        let request = OpenRequest::from_arguments(std::env::args_os().skip(1), &current_dir);
        if request.is_empty() {
            return None;
        }
        let files = request
            .files
            .into_iter()
            .filter_map(|file| match file {
                PickedFile::Path(path) => Some(path),
                PickedFile::Handle(_) | PickedFile::Stream(_) => None,
            })
            .collect();
        Some(SystemRequest::Open {
            files,
            links: request.links,
        })
    }
}

/// A handler for the system asking the app to show itself again.
pub type ReopenHandler = Rc<dyn Fn(Reopen)>;

/// The system asked the app to show itself again: its Dock icon was clicked, or it was opened
/// again from the Finder, Spotlight or `open` while it ran.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reopen {
    /// Whether any window of the app was visible when the system asked. Without one, AppKit
    /// brings back a minimized window itself.
    pub had_visible_windows: bool,
    /// Where the pointer was while the system asked, in the coordinates of window positions:
    /// logical points from the top-left corner of the main screen. After a click on the Dock
    /// icon it is on the icon; after a reopen from the keyboard or `open`, wherever it was.
    /// `None` where the system cannot say.
    pub pointer: Option<Offset>,
}

#[derive(Default)]
pub(crate) struct SystemRequestHandlers {
    reopen: RefCell<Option<ReopenHandler>>,
}

impl SystemRequestHandlers {
    pub(crate) fn set_reopen(&self, handler: Option<ReopenHandler>) {
        *self.reopen.borrow_mut() = handler;
    }

    /// The system asked the app to show itself again; nothing happens without a handler.
    pub(crate) fn reopen(&self, reopen: Reopen) {
        let handler = self.reopen.borrow().clone();
        if let Some(handler) = handler {
            handler(reopen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reopen_reaches_the_handler_set_when_it_comes() {
        let handlers = SystemRequestHandlers::default();
        let click_on_the_icon = Reopen {
            had_visible_windows: true,
            pointer: Some(Offset::new(864.0, 1090.0)),
        };
        handlers.reopen(click_on_the_icon);
        let received = Rc::new(RefCell::new(Vec::new()));
        handlers.set_reopen(Some(Rc::new({
            let received = Rc::clone(&received);
            move |reopen| received.borrow_mut().push(reopen)
        })));
        handlers.reopen(click_on_the_icon);
        assert_eq!(*received.borrow(), vec![click_on_the_icon]);
    }
}

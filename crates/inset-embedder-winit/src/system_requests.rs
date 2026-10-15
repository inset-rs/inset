//! What the system asks of the application as a whole: files to open, handed to the handler the
//! app sets through `Platform::file_opening`; showing itself again, handed to the handler it sets
//! through [`WinitPlatform::macos`](crate::WinitPlatform::macos); and quitting, which goes to the
//! framework instead, through the client (`app_exit.rs`).
//!
//! The main loop calls a handler outside every framework call, so a handler may post to the
//! app. Files asked for before there is a handler wait for one: a launch opens its files
//! before the app has started.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use inset_embedder::{FileOpening, Offset, OpenFilesHandler};

use crate::WinitPlatform;
use crate::app_requests::AppRequest;

/// A request the system makes of the application as a whole.
#[derive(Debug)]
#[cfg_attr(
    not(target_os = "macos"),
    expect(dead_code, reason = "only macOS makes these requests")
)]
pub(crate) enum SystemRequest {
    /// Open these files: documents opened from the system's file browser or dropped on the
    /// application's icon.
    OpenFiles(Vec<PathBuf>),
    /// Show yourself again: the application's icon was clicked, or the application opened
    /// again while it runs.
    Reopen(Reopen),
    /// Quit if the application agrees: the user chose Quit, or is logging out. The framework is
    /// asked, and an exit is carried out with
    /// [`ApplicationControl::quit`](crate::os::ApplicationControl::quit); a cancel changes
    /// nothing.
    Quit,
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
    /// Files the system asked to open that no handler has taken yet.
    files_to_open: RefCell<Vec<PathBuf>>,
    open_files: RefCell<Option<OpenFilesHandler>>,
    reopen: RefCell<Option<ReopenHandler>>,
}

impl SystemRequestHandlers {
    pub(crate) fn set_open_files(&self, handler: Option<OpenFilesHandler>) {
        *self.open_files.borrow_mut() = handler;
    }

    /// Whether files the system asked to open wait for a handler.
    pub(crate) fn files_waiting(&self) -> bool {
        !self.files_to_open.borrow().is_empty()
    }

    pub(crate) fn set_reopen(&self, handler: Option<ReopenHandler>) {
        *self.reopen.borrow_mut() = handler;
    }

    /// The system asked to open `files`: they go to the handler, or wait for one.
    pub(crate) fn open_files(&self, files: Vec<PathBuf>) {
        self.files_to_open.borrow_mut().extend(files);
        self.hand_over_files();
    }

    /// Hands the waiting files to the handler, all together, if there is one.
    pub(crate) fn hand_over_files(&self) {
        let Some(handler) = self.open_files.borrow().clone() else {
            return;
        };
        let files = self.files_to_open.take();
        if !files.is_empty() {
            handler(files);
        }
    }

    /// The system asked the app to show itself again; nothing happens without a handler.
    pub(crate) fn reopen(&self, reopen: Reopen) {
        let handler = self.reopen.borrow().clone();
        if let Some(handler) = handler {
            handler(reopen);
        }
    }
}

/// Where the system asks the app to open files: macOS. Elsewhere it starts the app with the files
/// on its command line, and the platform offers no file opening.
impl FileOpening for WinitPlatform {
    /// Files already waiting reach `handler` from the main loop, on its next turn, as files the
    /// system asks for later do.
    fn set_open_files_handler(&self, handler: Option<OpenFilesHandler>) {
        let handlers = self.state().handlers();
        handlers.set_open_files(handler);
        if handlers.files_waiting() {
            self.send(AppRequest::HandOverFiles);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording() -> (Rc<RefCell<Vec<Vec<PathBuf>>>>, OpenFilesHandler) {
        let received = Rc::new(RefCell::new(Vec::new()));
        let handler: OpenFilesHandler = Rc::new({
            let received = Rc::clone(&received);
            move |files| received.borrow_mut().push(files)
        });
        (received, handler)
    }

    #[test]
    fn files_asked_for_before_a_handler_wait_for_it() {
        let handlers = SystemRequestHandlers::default();
        handlers.open_files(vec![PathBuf::from("/a.wapk")]);
        handlers.open_files(vec![PathBuf::from("/b.wasm")]);
        let (received, handler) = recording();
        handlers.set_open_files(Some(handler));
        assert!(handlers.files_waiting());
        assert!(received.borrow().is_empty());
        handlers.hand_over_files();
        assert_eq!(
            *received.borrow(),
            vec![vec![PathBuf::from("/a.wapk"), PathBuf::from("/b.wasm")]]
        );
    }

    #[test]
    fn files_asked_for_with_a_handler_reach_it_at_once_and_once() {
        let handlers = SystemRequestHandlers::default();
        let (received, handler) = recording();
        handlers.set_open_files(Some(handler));
        assert!(!handlers.files_waiting());
        handlers.open_files(vec![PathBuf::from("/a.wapk")]);
        handlers.hand_over_files();
        assert_eq!(*received.borrow(), vec![vec![PathBuf::from("/a.wapk")]]);
    }

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

//! What the system asks of the application as a whole: files and links to open, which reach the
//! client as an open request (`EmbedderClient::open_requested`); showing itself again, handed to
//! the handler the app sets through [`WinitPlatform::macos`](crate::WinitPlatform::macos); a click
//! on a notification the app posted as another app, handed to the handler it gave with it
//! ([`Windows::notifications_of`](crate::Windows::notifications_of)), and held until it gives one;
//! and
//! quitting, which goes to the framework instead, through the client (`app_exit.rs`).
//!
//! macOS asks to open through its application delegate, at launch and while the app runs
//! (`application:openURLs:`), and iOS through the application's delegate
//! (`application:openURL:options:`). Windows and Linux start the app with what it opens on its
//! command line, which is the app's own to read: the system gives only strings there, and which of
//! them are files the app decided when it registered its file types.
//!
//! The main loop calls a handler outside every framework call, so a handler may post to the app.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use waft_embedder::{NotificationActivation, Offset};

/// A request the system makes of the application as a whole.
#[derive(Debug)]
pub(crate) enum SystemRequest {
    /// Open these files and links: documents opened from the system's file browser or dropped on
    /// the application's icon, or links whose scheme is the app's.
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "ios")),
        allow(dead_code, reason = "only macOS and iOS ask to open")
    )]
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
    /// The user clicked one of the app's notifications, or one of its actions, or replied, which
    /// the system's notifications report (`notifications.rs`): one of the app's own where
    /// `identity` is `None`, else one it posted as the app `identity`.
    NotificationActivated {
        identity: Option<String>,
        activation: NotificationActivation,
    },
    /// Quit if the application agrees: the user chose Quit, or is logging out. The framework is
    /// asked, and an exit is carried out with
    /// [`ApplicationControl::quit`](crate::os::capabilities::ApplicationControl::quit); a cancel changes
    /// nothing.
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "only macOS asks to quit")
    )]
    Quit,
}

/// A handler for the system asking the app to show itself again.
pub type ReopenHandler = Rc<dyn Fn(Reopen)>;

/// A handler for the clicks on the notifications an app posts as another app.
pub type NotificationActivationHandler = Rc<dyn Fn(NotificationActivation)>;

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
    /// By the identity of the app the notifications were posted as.
    notification_activations: RefCell<HashMap<String, NotificationActivationHandler>>,
    /// The clicks on notifications posted as an app the app has given no handler for yet: the
    /// click that started it, before its setup reached `notifications_of`.
    held_activations: RefCell<HashMap<String, Vec<NotificationActivation>>>,
}

impl SystemRequestHandlers {
    pub(crate) fn set_reopen(&self, handler: Option<ReopenHandler>) {
        *self.reopen.borrow_mut() = handler;
    }

    /// The clicks on notifications posted as the app `identity` go to `handler` from now on;
    /// answers those held until then, for the caller to send again.
    pub(crate) fn set_notification_activation(
        &self,
        identity: &str,
        handler: NotificationActivationHandler,
    ) -> Vec<NotificationActivation> {
        self.notification_activations
            .borrow_mut()
            .insert(identity.to_owned(), handler);
        self.held_activations
            .borrow_mut()
            .remove(identity)
            .unwrap_or_default()
    }

    /// The user clicked a notification posted as the app `identity`: it goes to that app's
    /// handler, or is held until the app gives one.
    pub(crate) fn notification_activated(
        &self,
        identity: String,
        activation: NotificationActivation,
    ) {
        let handler = self
            .notification_activations
            .borrow()
            .get(&identity)
            .cloned();
        match handler {
            Some(handler) => handler(activation),
            None => self
                .held_activations
                .borrow_mut()
                .entry(identity)
                .or_default()
                .push(activation),
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

    #[test]
    fn a_click_for_another_app_waits_for_its_handler() {
        let handlers = SystemRequestHandlers::default();
        let click = |id: &str| NotificationActivation {
            id: id.to_owned(),
            action: None,
            reply: None,
        };
        handlers.notification_activated("dev.waos.app.notes".to_owned(), click("started-me"));
        let received = Rc::new(RefCell::new(Vec::new()));
        let held = handlers.set_notification_activation(
            "dev.waos.app.notes",
            Rc::new({
                let received = Rc::clone(&received);
                move |activation| received.borrow_mut().push(activation)
            }),
        );
        assert_eq!(
            held,
            vec![click("started-me")],
            "the held click is handed back"
        );
        handlers.notification_activated("dev.waos.app.notes".to_owned(), click("later"));
        handlers.notification_activated("dev.waos.app.mail".to_owned(), click("not-notes"));
        assert_eq!(*received.borrow(), vec![click("later")]);
    }
}

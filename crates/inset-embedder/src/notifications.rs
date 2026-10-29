//! Notifications. Not a dart:ui file: Flutter reaches them through plugins such as
//! `flutter_local_notifications`.
//!
//! A notification is a value posted under an id, and posting the same id again replaces it, as
//! every system's own model is (`UNNotificationRequest`'s identifier, Android's `notify(id, …)`,
//! the web's `tag`, Linux's `replaces_id`, a Windows toast's `Tag`); not an object the app keeps,
//! which a click after a restart could not reach. A click, at launch or while the app runs, reaches
//! the app by the notification's id and the action chosen, through
//! `EmbedderClient::notification_activated`.
//!
//! A typed reply ([`NotificationReplies`]) and delivery at a later time
//! ([`NotificationScheduling`]) are capabilities, `None` where the system lacks them, as the
//! user's permission is ([`NotificationPermission`]) where the system keeps none for the app. A
//! host answers each from the system's own, and never stands in for one the system lacks.
//!
//! Only [`NotificationPermission::request`] asks the user, with the system's own question. A post
//! or a schedule never does, as no system's own post does (`UNUserNotificationCenter`'s `add`
//! fails with `notificationsNotAllowed`, and Android's and the web's show nothing): without the
//! permission it answers [`NotificationError::Denied`].

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use crate::IconImage;

/// The system's notifications.
pub trait Notifications {
    /// Shows `notification`, in place of the one posted under its id while that is still there.
    /// It never asks the user: where the system does not let the app notify, because the user was
    /// never asked or said no, it answers [`NotificationError::Denied`], as the systems' own posts
    /// do, and the app asks first through [`Notifications::permission`].
    fn post(&self, notification: &Notification) -> PostFuture;

    /// Takes away the notification posted or scheduled under `id`.
    fn remove(&self, id: &str);

    /// The user's permission, where the system keeps one for the app; `None` where any app may
    /// notify.
    fn permission(&self) -> Option<&dyn NotificationPermission>;

    /// A typed reply, where the system takes one.
    fn replies(&self) -> Option<&dyn NotificationReplies>;

    /// Delivery at a later time, held by the system whether or not the app runs.
    fn scheduling(&self) -> Option<&dyn NotificationScheduling>;
}

/// A notification: what the system shows under its id.
#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    /// The app's own name for it, which a later post replaces and a click names.
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub image: Option<IconImage>,
    /// Buttons, and reply fields from [`NotificationReplies::reply_action`], in order.
    pub actions: Vec<NotificationAction>,
    /// Shown without the system's sound.
    pub silent: bool,
}

impl Notification {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Notification {
        Notification {
            id: id.into(),
            title: title.into(),
            body: None,
            image: None,
            actions: Vec::new(),
            silent: false,
        }
    }
}

/// A button on a notification, or a field to reply in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationAction {
    /// The app's own name for it, which a click names.
    pub id: String,
    pub label: String,
    /// The reply field's placeholder, for an action the host made a reply.
    reply: Option<String>,
}

impl NotificationAction {
    pub fn button(id: impl Into<String>, label: impl Into<String>) -> NotificationAction {
        NotificationAction {
            id: id.into(),
            label: label.into(),
            reply: None,
        }
    }

    /// An action that asks for a typed reply, made only by [`NotificationReplies::reply_action`],
    /// so an app has one only where the system takes replies.
    pub(crate) fn with_reply(
        id: impl Into<String>,
        label: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> NotificationAction {
        NotificationAction {
            id: id.into(),
            label: label.into(),
            reply: Some(placeholder.into()),
        }
    }

    /// The placeholder of the reply field the action asks for; `None` for a button.
    pub fn reply_placeholder(&self) -> Option<&str> {
        self.reply.as_deref()
    }
}

/// The user clicked a notification, or one of its actions, or replied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationActivation {
    /// The notification's id.
    pub id: String,
    /// The action's id; `None` for the notification itself.
    pub action: Option<String>,
    /// What the user typed, for a reply action.
    pub reply: Option<String>,
}

/// What a post answers, once the system has the notification.
pub type PostFuture = Pin<Box<dyn Future<Output = Result<(), NotificationError>>>>;

/// Why a notification was not posted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationError {
    /// The user does not let the app notify.
    Denied,
    /// The system failed to post it.
    Failed(String),
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationError::Denied => f.write_str("the user does not let the app notify"),
            NotificationError::Failed(reason) => write!(f, "the notification failed: {reason}"),
        }
    }
}

impl std::error::Error for NotificationError {}

/// The user's permission to notify, as the system keeps it for the app and asks for it.
pub trait NotificationPermission {
    /// The permission as it stands, without asking.
    fn state(&self) -> PermissionFuture;

    /// Asks the user where they have not been asked, with the system's own question; answers the
    /// permission after. Where they answered before, or the system has no question, it asks
    /// nothing and answers the permission as it stands.
    fn request(&self) -> PermissionFuture;
}

/// Where the user's permission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionState {
    Granted,
    /// The user, or the system's settings, do not let the app notify.
    Denied,
    /// The user has not been asked, and the system would ask.
    NotAsked,
}

pub type PermissionFuture = Pin<Box<dyn Future<Output = PermissionState>>>;

/// Typed replies, where the system takes them.
pub trait NotificationReplies {
    /// An action that asks for a reply, shown as a field with `placeholder` in it.
    fn reply_action(&self, id: &str, label: &str, placeholder: &str) -> NotificationAction {
        NotificationAction::with_reply(id, label, placeholder)
    }
}

/// Delivery at a later time, which the system holds whether or not the app runs.
pub trait NotificationScheduling {
    /// Shows `notification` at `at`, or at once where `at` has passed; a later post or schedule
    /// under its id replaces it, and [`Notifications::remove`] cancels it. It never asks the user,
    /// as a post does not.
    fn schedule(&self, notification: &Notification, at: SystemTime) -> PostFuture;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_action_carries_its_placeholder_and_a_button_none() {
        struct Replies;
        impl NotificationReplies for Replies {}
        let reply = Replies.reply_action("answer", "Reply", "Your answer");
        assert_eq!(reply.reply_placeholder(), Some("Your answer"));
        assert_eq!(
            NotificationAction::button("later", "Later").reply_placeholder(),
            None
        );
        let mut notification = Notification::new("message-7", "New message");
        notification.actions.push(reply);
        assert_eq!(notification.actions[0].id, "answer");
    }
}

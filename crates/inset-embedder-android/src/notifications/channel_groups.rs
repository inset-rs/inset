//! The sections of the app's notifications: Android's notification channel groups (API 26). Each
//! is a section of its own in the app's notification settings, with its own switch, and holds its
//! own two channels, the one that sounds and the one that does not, as Gmail has a section per
//! account. A section's notifications are the app's (`Notifications`), with posting, the
//! permission, replies and scheduling, posted on the section's channels.
//!
//! Making a section again renames it and keeps what the user set for it and its channels; taking it
//! away removes its channels and their notifications. Android keeps a channel's settings when the
//! app makes it again, so a section taken away and made again comes back as the user left it.

use std::rc::Rc;

use inset_embedder::{
    Notification, NotificationError, NotificationPermission, NotificationReplies,
    NotificationScheduling, Notifications, PostFuture,
};
use jni::objects::JValue;
use jni::{jni_sig, jni_str};

use super::permission::Permission;
use super::{AndroidNotifications, call_receiver};
use crate::log::warn_to_log;

/// The sections of the app's notifications, reached as
/// `AndroidPlatform::notification_channel_groups`.
pub struct NotificationChannelGroups {
    /// The app's permission to notify, which every section shares; `None` in a package without
    /// Inset's notification receiver, where the app has no notifications.
    permission: Option<Rc<Permission>>,
}

impl NotificationChannelGroups {
    /// The sections beside `app`, the app's own notifications, where it has them.
    pub(crate) fn new(app: Option<&AndroidNotifications>) -> NotificationChannelGroups {
        NotificationChannelGroups {
            permission: app.map(|app| Rc::clone(&app.permission)),
        }
    }

    /// Makes the section, or renames it, keeping the user's settings for it.
    pub fn set(&self, id: &str, name: &str) {
        let set = call_receiver(|env, context, receiver| {
            let id = env.new_string(id)?;
            let name = env.new_string(name)?;
            env.call_static_method(
                receiver,
                jni_str!("setSection"),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)V"),
                &[
                    JValue::Object(context),
                    JValue::Object(&id),
                    JValue::Object(&name),
                ],
            )
            .map(drop)
        });
        if let Err(error) = set {
            warn_to_log!("inset: the notifications' section {id} was not made: {error}");
        }
    }

    /// Takes the section away, with its channels.
    pub fn remove(&self, id: &str) {
        let removed = call_receiver(|env, context, receiver| {
            let id = env.new_string(id)?;
            env.call_static_method(
                receiver,
                jni_str!("removeSection"),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
                &[JValue::Object(context), JValue::Object(&id)],
            )
            .map(drop)
        });
        if let Err(error) = removed {
            warn_to_log!("inset: the notifications' section {id} was not taken away: {error}");
        }
    }

    /// The app's notifications posted in section `id`: the same `Notifications` as everywhere,
    /// with posting, permission, replies and scheduling. A section posted in before it was set is
    /// made, named after its id.
    pub fn notifications(&self, id: &str) -> Rc<dyn Notifications> {
        match &self.permission {
            Some(permission) => Rc::new(AndroidNotifications {
                permission: Rc::clone(permission),
                section: Some(id.to_owned()),
            }),
            None => Rc::new(WithoutReceiver),
        }
    }
}

/// A section's notifications in a package without Inset's notification receiver: every post
/// fails, as nothing there would show it.
struct WithoutReceiver;

impl Notifications for WithoutReceiver {
    fn post(&self, _notification: &Notification) -> PostFuture {
        Box::pin(std::future::ready(Err(NotificationError::Failed(
            "the package has no Inset notification receiver; inset-cli declares one".to_owned(),
        ))))
    }

    fn remove(&self, _id: &str) {}

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        None
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        None
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        None
    }
}

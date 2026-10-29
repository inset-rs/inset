//! The center's delegate: it hands the app each click on its notifications, on their actions and
//! in their reply fields, and has the center show a notification while the app is in front, which
//! the center otherwise leaves unseen.
//!
//! A method `define_class!` makes may be unwound through, and the center's frames were never
//! built to be: each body aborts the process on a panic, once the panic hook has printed it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;

use block2::DynBlock;
use inset_embedder::NotificationActivation;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNNotification, UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
    UNNotificationPresentationOptions, UNNotificationResponse, UNTextInputNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};

use crate::notifications::NotificationActivations;

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `CenterDelegate` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "InsetNotificationCenterDelegate"]
    #[ivars = NotificationActivations]
    pub(super) struct CenterDelegate;

    unsafe impl NSObjectProtocol for CenterDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for CenterDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            abort_on_panic(|| completion.call((presentation(notification),)));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            abort_on_panic(|| {
                if let Some(activation) = activation(response) {
                    self.ivars().send(activation);
                }
                completion.call(());
            });
        }
    }
);

impl CenterDelegate {
    pub(super) fn new(activations: NotificationActivations) -> Retained<CenterDelegate> {
        let this = Self::alloc().set_ivars(activations);
        // SAFETY: NSObject's `init`, on an instance whose ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// How a notification shows while the app is in front: as a banner and in the list, as it does
/// behind other apps, with its sound unless it has none.
fn presentation(notification: &UNNotification) -> UNNotificationPresentationOptions {
    let shown = UNNotificationPresentationOptions::Banner | UNNotificationPresentationOptions::List;
    if notification.request().content().sound().is_some() {
        shown | UNNotificationPresentationOptions::Sound
    } else {
        shown
    }
}

/// What the user's response tells the app.
fn activation(response: &UNNotificationResponse) -> Option<NotificationActivation> {
    let reply = response
        .downcast_ref::<UNTextInputNotificationResponse>()
        .map(|replied| replied.userText().to_string());
    activation_for(
        response.notification().request().identifier().to_string(),
        &response.actionIdentifier(),
        reply,
    )
}

/// The click on the notification `id` with the action `action` and what was typed: the center's
/// own action for a click on the notification itself is no action, and a dismissal, which the
/// center reports only for a category that asks, is not heard.
fn activation_for(
    id: String,
    action: &NSString,
    reply: Option<String>,
) -> Option<NotificationActivation> {
    // SAFETY: constants UserNotifications exports for the life of the process.
    let (clicked, dismissed) = unsafe {
        (
            UNNotificationDefaultActionIdentifier,
            UNNotificationDismissActionIdentifier,
        )
    };
    if action.isEqualToString(dismissed) {
        return None;
    }
    Some(NotificationActivation {
        id,
        action: (!action.isEqualToString(clicked)).then(|| action.to_string()),
        reply,
    })
}

fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| process::abort())
}

#[cfg(test)]
mod tests {
    use objc2::runtime::ProtocolObject;
    use objc2::sel;

    use super::*;

    #[test]
    fn a_click_names_the_notification_and_the_action_and_a_dismissal_is_not_heard() {
        // SAFETY: constants UserNotifications exports for the life of the process.
        let (clicked, dismissed) = unsafe {
            (
                UNNotificationDefaultActionIdentifier,
                UNNotificationDismissActionIdentifier,
            )
        };
        assert_eq!(
            activation_for("message-7".to_owned(), clicked, None),
            Some(NotificationActivation {
                id: "message-7".to_owned(),
                action: None,
                reply: None,
            })
        );
        assert_eq!(
            activation_for(
                "message-7".to_owned(),
                &NSString::from_str("answer"),
                Some("on my way".to_owned())
            ),
            Some(NotificationActivation {
                id: "message-7".to_owned(),
                action: Some("answer".to_owned()),
                reply: Some("on my way".to_owned()),
            })
        );
        assert_eq!(
            activation_for("message-7".to_owned(), dismissed, None),
            None
        );
    }

    #[test]
    fn the_delegate_answers_the_center_for_clicks_and_for_the_app_in_front() {
        let delegate = CenterDelegate::new(NotificationActivations::to(|_| {}));
        let protocol = ProtocolObject::<dyn UNUserNotificationCenterDelegate>::from_ref(&*delegate);
        assert!(protocol.respondsToSelector(sel!(
            userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:
        )));
        assert!(protocol.respondsToSelector(sel!(
            userNotificationCenter:willPresentNotification:withCompletionHandler:
        )));
    }
}

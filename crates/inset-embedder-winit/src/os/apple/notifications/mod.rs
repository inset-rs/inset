//! Notifications: `UNUserNotificationCenter`'s, which macOS and iOS share. A notification is a
//! request under the app's id, which a later request under the same id replaces; its buttons and
//! reply field are a category's, which the center must hold before the request is added
//! (`categories`); its image is an attachment and its time a calendar trigger (`request`). A post
//! never asks the user, as the center's `add` does not: where the app is not let notify it answers
//! `Denied` before it touches the center, which refuses such an app's request and keeps no
//! category for it; only the permission's `request` asks (`permission`). A click comes to the
//! center's delegate, set before the launch finishes so the click that launched the app is heard
//! too (`delegate`).
//!
//! UserNotifications raises an exception, which aborts the process, when a process outside an
//! application bundle touches the center, as a program `cargo run` starts is: nothing here may
//! reach the center but through [`bundled_center`].
//!
//! The center answers on queues of its own, where each step's continuation runs, and the app's
//! future is answered through `answer`.

// The center's delegate is a class of the embedder's own, and the center's answers come as
// pointers into its completion handlers; this module writes the `unsafe` they need.
#![allow(unsafe_code)]

mod categories;
mod delegate;
mod permission;
mod request;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::SystemTime;

use block2::RcBlock;
use inset_embedder::{
    Notification, NotificationError, NotificationPermission, NotificationReplies,
    NotificationScheduling, Notifications, PermissionFuture, PermissionState, PostFuture,
};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSArray, NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNErrorCode, UNErrorDomain, UNNotificationRequest, UNNotificationTrigger,
    UNUserNotificationCenter,
};

use self::categories::Categories;
use self::delegate::CenterDelegate;
use crate::answer::{Answer, answer};
use crate::notifications::NotificationActivations;

thread_local! {
    /// The center's delegate, kept here for the app's life, as the center holds it weakly. Main
    /// thread.
    static DELEGATE: RefCell<Option<Retained<CenterDelegate>>> = const { RefCell::new(None) };
}

/// Hears the clicks on the app's notifications, sending each to `activations`; before the launch
/// finishes, as the center reports the click that launched the app only to a delegate set by
/// then. Nothing where the app runs outside a bundle. Main thread.
pub(crate) fn hear_activations(activations: NotificationActivations) {
    let Some(center) = bundled_center() else {
        return;
    };
    let delegate = CenterDelegate::new(activations);
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    DELEGATE.set(Some(delegate));
}

/// The app's notifications, or `None` where the app runs outside a bundle.
pub(crate) fn notifications() -> Option<Rc<dyn Notifications>> {
    let center = bundled_center()?;
    Some(Rc::new(Center {
        center,
        categories: Categories::default(),
    }))
}

/// The app's notification center, where the app runs from an application bundle: the center
/// knows an app by its bundle, and raises an exception for a process that has none.
fn bundled_center() -> Option<Retained<UNUserNotificationCenter>> {
    let bundle = NSBundle::mainBundle();
    let is_application =
        bundle.bundleIdentifier().is_some() && bundle.bundlePath().to_string().ends_with(".app");
    is_application.then(UNUserNotificationCenter::currentNotificationCenter)
}

/// The center, as the app's notifications reach it.
struct Center {
    center: Retained<UNUserNotificationCenter>,
    categories: Categories,
}

impl Center {
    /// Adds the request that shows `notification` at `trigger`'s time, or at once, where the user
    /// lets the app notify, once the center holds its actions' category; answers `Denied`
    /// without asking where they do not.
    fn add(
        &self,
        notification: &Notification,
        trigger: Option<Retained<UNNotificationTrigger>>,
    ) -> PostFuture {
        let request = match request::request(notification, trigger) {
            Ok(request) => request,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let (answer, answered) = answer(Err(NotificationError::Failed(
            "the notification center gave no answer".to_owned(),
        )));
        let center = self.center.clone();
        let categories = self.categories.clone();
        let actions = notification.actions.clone();
        permission::current(&self.center, move |permission| {
            if permission != PermissionState::Granted {
                return answer.give(Err(NotificationError::Denied));
            }
            let adding = center.clone();
            categories.hold(&center, &actions, move || {
                add_request(&adding, &request, answer)
            });
        });
        Box::pin(answered)
    }
}

impl Notifications for Center {
    fn post(&self, notification: &Notification) -> PostFuture {
        self.add(notification, None)
    }

    fn remove(&self, id: &str) {
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        self.center
            .removeDeliveredNotificationsWithIdentifiers(&ids);
        self.center
            .removePendingNotificationRequestsWithIdentifiers(&ids);
    }

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        Some(self)
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        Some(self)
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        Some(self)
    }
}

impl NotificationPermission for Center {
    fn state(&self) -> PermissionFuture {
        let (answer, answered) = answer(PermissionState::Denied);
        permission::current(&self.center, move |state| answer.give(state));
        Box::pin(answered)
    }

    fn request(&self) -> PermissionFuture {
        let (answer, answered) = answer(PermissionState::Denied);
        permission::asked(&self.center, move |state| answer.give(state));
        Box::pin(answered)
    }
}

/// `UNTextInputNotificationAction`.
impl NotificationReplies for Center {}

/// A calendar trigger, which fires when the clock reads the time, as a `SystemTime` is one.
impl NotificationScheduling for Center {
    fn schedule(&self, notification: &Notification, at: SystemTime) -> PostFuture {
        self.add(
            notification,
            request::calendar_trigger(at, SystemTime::now()),
        )
    }
}

/// Adds `request` to `center`, and gives `answer` what the center said.
fn add_request(
    center: &UNUserNotificationCenter,
    request: &UNNotificationRequest,
    answer: Answer<Result<(), NotificationError>>,
) {
    let answer = once(move |added| answer.give(added));
    let handler = RcBlock::new(move |error: *mut NSError| {
        // SAFETY: the center passes a valid error, or null for none.
        answer(outcome(unsafe { error.as_ref() }))
    });
    center.addNotificationRequest_withCompletionHandler(request, Some(&handler));
}

/// What the center's answer to a request means for the app: the user not letting it notify is
/// `Denied`, as the center may say so even after the app was let.
fn outcome(error: Option<&NSError>) -> Result<(), NotificationError> {
    let Some(error) = error else {
        return Ok(());
    };
    // SAFETY: a constant UserNotifications exports for the life of the process.
    let domain = unsafe { UNErrorDomain };
    let not_allowed = domain.is_some_and(|domain| error.domain().isEqualToString(domain))
        && error.code() == UNErrorCode::NotificationsNotAllowed.0;
    if not_allowed {
        Err(NotificationError::Denied)
    } else {
        Err(NotificationError::Failed(
            error.localizedDescription().to_string(),
        ))
    }
}

/// `then` as a completion handler, which the system calls once: a second call does nothing.
fn once<A>(then: impl FnOnce(A)) -> impl Fn(A) {
    let then = Cell::new(Some(then));
    move |argument| {
        if let Some(then) = then.take() {
            then(argument);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::ptr::NonNull;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;

    use inset_embedder::NotificationAction;
    use objc2_foundation::{NSErrorDomain, NSSet};
    use objc2_user_notifications::UNNotificationCategory;

    use super::*;

    #[test]
    fn an_app_outside_a_bundle_has_no_notifications() {
        assert!(notifications().is_none());
    }

    #[test]
    fn a_refusal_from_the_center_is_denied_and_any_other_error_a_failure() {
        assert_eq!(outcome(None), Ok(()));
        // SAFETY: a constant UserNotifications exports for the life of the process.
        let domain: &NSErrorDomain = unsafe { UNErrorDomain }.expect("UserNotifications' domain");
        let refused = NSError::new(UNErrorCode::NotificationsNotAllowed.0, domain);
        assert_eq!(outcome(Some(&refused)), Err(NotificationError::Denied));
        let other = NSError::new(UNErrorCode::AttachmentCorrupt.0, domain);
        assert!(matches!(
            outcome(Some(&other)),
            Err(NotificationError::Failed(_))
        ));
    }

    #[test]
    fn a_completion_handler_called_twice_continues_once() {
        let calls = Rc::new(Cell::new(0));
        let counted = Rc::clone(&calls);
        let handler = once(move |()| counted.set(counted.get() + 1));
        handler(());
        handler(());
        assert_eq!(calls.get(), 1);
    }

    /// Run from a bundle: the test binary as the executable of an `.app` (`CFBundleIdentifier`,
    /// `CFBundleExecutable`, `LSUIElement`), signed, and launched hidden with
    /// `open -g -j -W --stdout … <app> --args <this test's path> --exact --ignored`. It asks the
    /// user nothing. The center keeps categories and requests only for an app the user was asked
    /// about, so those are checked only once the bundle's app is let notify.
    #[test]
    #[ignore = "needs an application bundle: see the test's doc comment"]
    fn a_bundled_app_reaches_the_center_without_asking() {
        let notifications = notifications().expect("the center, from a bundle");
        let permission = notifications.permission().expect("the permission");
        let state = block_on(permission.state());
        eprintln!("the permission stands at {state:?}");
        let center = bundled_center().expect("the center");
        hear_activations(NotificationActivations::to(|_| {}));
        assert!(center.delegate().is_some(), "the delegate is kept");
        let actions = vec![NotificationAction::button("later", "Later")];
        let (held, holding) = mpsc::channel();
        Categories::default().hold(&center, &actions, move || held.send(()).unwrap());
        holding
            .recv_timeout(Duration::from_secs(10))
            .expect("the hold goes on");
        notifications.remove("never-posted");
        if state != PermissionState::Granted {
            return;
        }
        let id = request::category_id(&actions).expect("a category");
        assert!(category_ids(&center).contains(&id));
        let mut reminder = Notification::new("inset-test-reminder", "Stand up");
        reminder.actions = actions;
        let scheduling = notifications.scheduling().expect("scheduling");
        let at = SystemTime::now() + Duration::from_secs(3600);
        assert_eq!(block_on(scheduling.schedule(&reminder, at)), Ok(()));
        assert!(pending_ids(&center).contains(&reminder.id));
        notifications.remove(&reminder.id);
        assert!(!pending_ids(&center).contains(&reminder.id));
    }

    fn category_ids(center: &UNUserNotificationCenter) -> Vec<String> {
        let (read, reading) = mpsc::channel();
        let handler = RcBlock::new(move |held: NonNull<NSSet<UNNotificationCategory>>| {
            // SAFETY: the center passes its categories, valid for the call.
            let held = unsafe { held.as_ref() };
            let ids = held
                .iter()
                .map(|category| category.identifier().to_string());
            read.send(ids.collect()).unwrap();
        });
        center.getNotificationCategoriesWithCompletionHandler(&handler);
        reading
            .recv_timeout(Duration::from_secs(10))
            .expect("the categories")
    }

    fn pending_ids(center: &UNUserNotificationCenter) -> Vec<String> {
        let (read, reading) = mpsc::channel();
        let handler = RcBlock::new(move |pending: NonNull<NSArray<UNNotificationRequest>>| {
            // SAFETY: the center passes its requests, valid for the call.
            let pending = unsafe { pending.as_ref() };
            let ids = pending
                .iter()
                .map(|request| request.identifier().to_string());
            read.send(ids.collect()).unwrap();
        });
        center.getPendingNotificationRequestsWithCompletionHandler(&handler);
        reading
            .recv_timeout(Duration::from_secs(10))
            .expect("the requests")
    }

    fn block_on<T>(future: Pin<Box<dyn Future<Output = T>>>) -> T {
        struct Unpark(std::thread::Thread);
        impl Wake for Unpark {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let mut future = future;
        let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        for _ in 0..100 {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
            std::thread::park_timeout(Duration::from_millis(100));
        }
        panic!("no answer in ten seconds");
    }
}

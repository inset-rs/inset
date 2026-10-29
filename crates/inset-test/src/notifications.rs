//! The system's notifications in memory, as a host shows nothing: it keeps the notifications shown
//! and scheduled, by id, and answers the permission the test gave.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

use inset_embedder::{
    Notification, NotificationError, NotificationPermission, NotificationReplies,
    NotificationScheduling, Notifications, PermissionFuture, PermissionState, PostFuture,
};

/// Notifications that keep what the app posted, scheduled and removed. By default any app may
/// notify, as on a system with no permission question, and replies and scheduling are there. Every
/// clone is the same notifications.
#[derive(Clone)]
pub struct MockNotifications(Rc<NotificationsState>);

struct NotificationsState {
    shown: RefCell<Vec<Notification>>,
    scheduled: RefCell<Vec<(Notification, SystemTime)>>,
    /// The permission, where the test gave the system a question.
    permission: Option<MockPermission>,
    replies: bool,
    scheduling: bool,
}

impl Default for MockNotifications {
    fn default() -> MockNotifications {
        MockNotifications(Rc::new(NotificationsState {
            shown: RefCell::default(),
            scheduled: RefCell::default(),
            permission: None,
            replies: true,
            scheduling: true,
        }))
    }
}

/// The user's permission, and what they answer when asked.
struct MockPermission {
    state: RefCell<PermissionState>,
    answer: PermissionState,
    asked: RefCell<u32>,
}

impl MockNotifications {
    pub fn new() -> MockNotifications {
        MockNotifications::default()
    }

    /// A system with a question of its own, where the permission stands at `state`, and the user
    /// answers `answer` when the app asks (`NotificationPermission::request`); a post never asks,
    /// and is denied until the permission is granted.
    pub fn asking(self, state: PermissionState, answer: PermissionState) -> MockNotifications {
        self.rebuilt(|notifications| {
            notifications.permission = Some(MockPermission {
                state: RefCell::new(state),
                answer,
                asked: RefCell::new(0),
            });
        })
    }

    /// A system without typed replies.
    pub fn without_replies(self) -> MockNotifications {
        self.rebuilt(|notifications| notifications.replies = false)
    }

    /// A system without delivery at a later time.
    pub fn without_scheduling(self) -> MockNotifications {
        self.rebuilt(|notifications| notifications.scheduling = false)
    }

    /// The notifications showing, oldest first; a replaced one is where it first showed.
    pub fn shown(&self) -> Vec<Notification> {
        self.0.shown.borrow().clone()
    }

    /// The notifications waiting for their time, with it.
    pub fn scheduled(&self) -> Vec<(Notification, SystemTime)> {
        self.0.scheduled.borrow().clone()
    }

    /// How many times the user was asked.
    pub fn times_asked(&self) -> u32 {
        self.0
            .permission
            .as_ref()
            .map_or(0, |permission| *permission.asked.borrow())
    }

    fn rebuilt(self, change: impl FnOnce(&mut NotificationsState)) -> MockNotifications {
        let mut state = Rc::try_unwrap(self.0)
            .unwrap_or_else(|_| panic!("configure MockNotifications before cloning it"));
        change(&mut state);
        MockNotifications(Rc::new(state))
    }

    /// Whether the app may notify, without asking: a post never asks.
    fn allowed(&self) -> Result<(), NotificationError> {
        let state = self
            .0
            .permission
            .as_ref()
            .map(|permission| *permission.state.borrow());
        match state {
            None | Some(PermissionState::Granted) => Ok(()),
            Some(_) => Err(NotificationError::Denied),
        }
    }
}

impl MockPermission {
    fn ask(&self) -> PermissionState {
        let mut state = self.state.borrow_mut();
        if *state == PermissionState::NotAsked {
            *self.asked.borrow_mut() += 1;
            *state = self.answer;
        }
        *state
    }
}

/// Puts `notification` in `list`, in place of the one under its id.
fn replace(list: &mut Vec<Notification>, notification: &Notification) {
    match list.iter_mut().find(|shown| shown.id == notification.id) {
        Some(shown) => *shown = notification.clone(),
        None => list.push(notification.clone()),
    }
}

impl Notifications for MockNotifications {
    fn post(&self, notification: &Notification) -> PostFuture {
        let posted = self.allowed().map(|()| {
            self.0
                .scheduled
                .borrow_mut()
                .retain(|(scheduled, _)| scheduled.id != notification.id);
            replace(&mut self.0.shown.borrow_mut(), notification);
        });
        Box::pin(std::future::ready(posted))
    }

    fn remove(&self, id: &str) {
        self.0.shown.borrow_mut().retain(|shown| shown.id != id);
        self.0
            .scheduled
            .borrow_mut()
            .retain(|(scheduled, _)| scheduled.id != id);
    }

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        self.0.permission.is_some().then_some(self)
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        self.0.replies.then_some(self)
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        self.0.scheduling.then_some(self)
    }
}

impl NotificationPermission for MockNotifications {
    fn state(&self) -> PermissionFuture {
        let state = self
            .0
            .permission
            .as_ref()
            .map_or(PermissionState::Granted, |permission| {
                *permission.state.borrow()
            });
        Box::pin(std::future::ready(state))
    }

    fn request(&self) -> PermissionFuture {
        let state = self
            .0
            .permission
            .as_ref()
            .map_or(PermissionState::Granted, MockPermission::ask);
        Box::pin(std::future::ready(state))
    }
}

impl NotificationReplies for MockNotifications {}

impl NotificationScheduling for MockNotifications {
    fn schedule(&self, notification: &Notification, at: SystemTime) -> PostFuture {
        let scheduled = self.allowed().map(|()| {
            let mut scheduled = self.0.scheduled.borrow_mut();
            scheduled.retain(|(waiting, _)| waiting.id != notification.id);
            scheduled.push((notification.clone(), at));
        });
        Box::pin(std::future::ready(scheduled))
    }
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use super::*;

    fn now<T>(future: std::pin::Pin<Box<dyn std::future::Future<Output = T>>>) -> T {
        let mut future = future;
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the mock answers at once"),
        }
    }

    #[test]
    fn a_post_under_an_id_replaces_and_remove_takes_it_away() {
        let notifications = MockNotifications::new();
        let mut first = Notification::new("download", "Downloading");
        assert_eq!(now(notifications.post(&first)), Ok(()));
        first.title = "Downloaded".to_owned();
        assert_eq!(now(notifications.post(&first)), Ok(()));
        assert_eq!(notifications.shown(), [first]);
        notifications.remove("download");
        assert!(notifications.shown().is_empty());
    }

    #[test]
    fn a_post_never_asks_and_is_denied_until_the_user_allows() {
        let notifications =
            MockNotifications::new().asking(PermissionState::NotAsked, PermissionState::Granted);
        let reminder = Notification::new("reminder", "Stand up");
        assert_eq!(
            now(notifications.post(&reminder)),
            Err(NotificationError::Denied)
        );
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(60);
        let scheduling = notifications.scheduling().expect("scheduling");
        assert_eq!(
            now(scheduling.schedule(&reminder, at)),
            Err(NotificationError::Denied)
        );
        assert_eq!(notifications.times_asked(), 0, "no post asked");
        let permission = notifications.permission().expect("the system asks");
        assert_eq!(now(permission.request()), PermissionState::Granted);
        assert_eq!(now(notifications.post(&reminder)), Ok(()));
        assert_eq!(notifications.times_asked(), 1);
    }

    #[test]
    fn a_refusal_is_asked_once() {
        let notifications =
            MockNotifications::new().asking(PermissionState::NotAsked, PermissionState::Denied);
        let permission = notifications.permission().expect("the system asks");
        assert_eq!(now(permission.request()), PermissionState::Denied);
        assert_eq!(now(permission.request()), PermissionState::Denied);
        assert_eq!(notifications.times_asked(), 1);
        assert_eq!(
            now(notifications.post(&Notification::new("reminder", "Stand up"))),
            Err(NotificationError::Denied)
        );
    }

    #[test]
    fn a_schedule_waits_until_a_post_or_a_remove() {
        let notifications = MockNotifications::new();
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(60);
        let reminder = Notification::new("reminder", "Stand up");
        let scheduling = notifications.scheduling().expect("scheduling");
        assert_eq!(now(scheduling.schedule(&reminder, at)), Ok(()));
        assert_eq!(notifications.scheduled(), [(reminder.clone(), at)]);
        notifications.remove("reminder");
        assert!(notifications.scheduled().is_empty());
        assert!(
            MockNotifications::new()
                .without_scheduling()
                .scheduling()
                .is_none()
        );
    }
}

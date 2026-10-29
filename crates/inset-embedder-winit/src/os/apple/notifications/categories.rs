//! The categories the app's notifications show their actions through. The center keeps one set of
//! categories, which each `setNotificationCategories:` replaces whole and which outlives the app,
//! as a scheduled notification's category must: so the app keeps every category its notifications
//! needed, and each set is the center's own joined with them, so that two posts setting at once
//! lose neither's. A request added before the center holds its category shows without its
//! actions; the center serves its calls in order, so one `getNotificationCategories` after the set
//! answers once the set is in place.

use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use block2::RcBlock;
use inset_embedder::NotificationAction;
use objc2::Message;
use objc2::rc::Retained;
use objc2_foundation::{NSSet, NSString};
use objc2_user_notifications::{UNNotificationCategory, UNUserNotificationCenter};

use super::once;
use super::request::{category, category_id};

/// The categories the app's notifications needed, by id: the actions each shows.
#[derive(Clone, Default)]
pub(super) struct Categories(Arc<Mutex<HashMap<String, Vec<NotificationAction>>>>);

impl Categories {
    /// Calls `then`, on the center's queue, once the center holds the category of `actions`; at
    /// once where there are none.
    pub(super) fn hold(
        &self,
        center: &UNUserNotificationCenter,
        actions: &[NotificationAction],
        then: impl FnOnce() + 'static,
    ) {
        let Some(id) = category_id(actions) else {
            return then();
        };
        self.locked().insert(id.clone(), actions.to_vec());
        let kept = self.clone();
        let setting = center.retain();
        let handler = RcBlock::new(once(move |held: NonNull<NSSet<UNNotificationCategory>>| {
            // SAFETY: the center passes its categories, valid for the call.
            let held = unsafe { held.as_ref() };
            if holds(held, &id) {
                return then();
            }
            setting.setNotificationCategories(&kept.joined_with(held));
            after_the_set(&setting, then);
        }));
        center.getNotificationCategoriesWithCompletionHandler(&handler);
    }

    /// The center's categories `held`, with the app's in place of any under the same id.
    fn joined_with(
        &self,
        held: &NSSet<UNNotificationCategory>,
    ) -> Retained<NSSet<UNNotificationCategory>> {
        let kept = self.locked();
        let mut joined: Vec<Retained<UNNotificationCategory>> = held
            .iter()
            .filter(|category| !kept.contains_key(&category.identifier().to_string()))
            .collect();
        joined.extend(kept.iter().map(|(id, actions)| category(id, actions)));
        NSSet::from_retained_slice(&joined)
    }

    fn locked(&self) -> MutexGuard<'_, HashMap<String, Vec<NotificationAction>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the center's categories `held` include the one named `id`.
fn holds(held: &NSSet<UNNotificationCategory>, id: &str) -> bool {
    let id = NSString::from_str(id);
    held.iter()
        .any(|category| category.identifier().isEqualToString(&id))
}

/// Calls `then` once the center has put in place the categories set before.
fn after_the_set(center: &UNUserNotificationCenter, then: impl FnOnce() + 'static) {
    let then = once(move |()| then());
    let handler = RcBlock::new(move |_held: NonNull<NSSet<UNNotificationCategory>>| then(()));
    center.getNotificationCategoriesWithCompletionHandler(&handler);
}

#[cfg(test)]
mod tests {
    use inset_embedder::NotificationReplies;
    use objc2_foundation::NSArray;
    use objc2_user_notifications::UNNotificationCategoryOptions;

    use super::*;

    struct Replies;
    impl NotificationReplies for Replies {}

    #[test]
    fn a_set_keeps_the_centers_other_categories_and_puts_the_apps_in_place_of_its_own() {
        let actions = vec![
            NotificationAction::button("later", "Later"),
            Replies.reply_action("answer", "Reply", "Your answer"),
        ];
        let id = category_id(&actions).expect("a category for actions");
        let categories = Categories::default();
        categories.locked().insert(id.clone(), actions);
        let others =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str("another"),
                &NSArray::new(),
                &NSArray::new(),
                UNNotificationCategoryOptions::empty(),
            );
        let stale =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(&id),
                &NSArray::new(),
                &NSArray::new(),
                UNNotificationCategoryOptions::empty(),
            );
        let held = NSSet::from_retained_slice(&[others, stale]);
        let joined = categories.joined_with(&held);
        assert_eq!(joined.count(), 2);
        assert!(holds(&joined, "another") && holds(&joined, &id));
        let own = joined
            .iter()
            .find(|category| category.identifier().to_string() == id)
            .expect("the app's category");
        assert_eq!(own.actions().count(), 2, "the app's, not the stale one");
    }
}

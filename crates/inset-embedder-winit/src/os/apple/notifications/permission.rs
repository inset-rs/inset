//! The user's permission, as the center keeps it (`getNotificationSettings`) and asks for it
//! (`requestAuthorizationWithOptions:`): the system asks once, and answers later requests with
//! the user's first answer.

use std::ptr::NonNull;

use block2::RcBlock;
use inset_embedder::PermissionState;
use objc2::Message;
use objc2::runtime::Bool;
use objc2_foundation::NSError;
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNNotificationSettings, UNUserNotificationCenter,
};

use super::once;

/// What the app asks to be let do: show its notifications, with their sound.
const ASKED_FOR: UNAuthorizationOptions =
    UNAuthorizationOptions::Alert.union(UNAuthorizationOptions::Sound);

/// Hands `then` the permission as it stands, on the center's queue.
pub(super) fn current(
    center: &UNUserNotificationCenter,
    then: impl FnOnce(PermissionState) + 'static,
) {
    let then = once(then);
    let handler = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
        // SAFETY: the center passes its settings, valid for the call.
        let status = unsafe { settings.as_ref() }.authorizationStatus();
        then(state_of(status));
    });
    center.getNotificationSettingsWithCompletionHandler(&handler);
}

/// Asks the user where they have not been asked, then hands `then` the permission, on the center's
/// queue.
pub(super) fn asked(
    center: &UNUserNotificationCenter,
    then: impl FnOnce(PermissionState) + 'static,
) {
    let asking = center.retain();
    current(center, move |state| {
        if state != PermissionState::NotAsked {
            return then(state);
        }
        let then = once(then);
        let handler = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            then(if granted.as_bool() {
                PermissionState::Granted
            } else {
                PermissionState::Denied
            });
        });
        asking.requestAuthorizationWithOptions_completionHandler(ASKED_FOR, &handler);
    });
}

/// The permission an authorization status is: a provisional or an ephemeral one lets the app
/// notify, as a full one does.
fn state_of(status: UNAuthorizationStatus) -> PermissionState {
    match status {
        UNAuthorizationStatus::NotDetermined => PermissionState::NotAsked,
        UNAuthorizationStatus::Authorized
        | UNAuthorizationStatus::Provisional
        | UNAuthorizationStatus::Ephemeral => PermissionState::Granted,
        _ => PermissionState::Denied,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_that_lets_the_app_notify_is_granted() {
        assert_eq!(
            state_of(UNAuthorizationStatus::NotDetermined),
            PermissionState::NotAsked
        );
        assert_eq!(
            state_of(UNAuthorizationStatus::Denied),
            PermissionState::Denied
        );
        for granted in [
            UNAuthorizationStatus::Authorized,
            UNAuthorizationStatus::Provisional,
            UNAuthorizationStatus::Ephemeral,
        ] {
            assert_eq!(state_of(granted), PermissionState::Granted);
        }
    }
}

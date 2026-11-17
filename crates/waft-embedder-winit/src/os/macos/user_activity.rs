//! Work the app is doing that the user is waiting on, which macOS must not throttle: an
//! `NSProcessInfo` activity, begun with `NSActivityUserInitiatedAllowingIdleSystemSleep` and ended
//! when its [`UserActivity`] is dropped. App Nap otherwise gives a process it thinks is in the
//! background low priority and coalesces its timers, which a program that compiles or runs other
//! programs for the user cannot afford.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, NSString};

/// An activity macOS knows the app is doing, begun when it is made and ended when it is dropped.
pub struct UserActivity {
    /// The token `beginActivityWithOptions:reason:` answered, which `endActivity:` takes.
    activity: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl UserActivity {
    /// Begins the activity macOS knows the app is doing, keeping `reason` for the system's logs,
    /// so the app is not throttled while the activity lives. The system may still sleep while it
    /// is idle.
    pub(crate) fn begin(reason: &str) -> UserActivity {
        let activity = NSProcessInfo::processInfo().beginActivityWithOptions_reason(
            NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
            &NSString::from_str(reason),
        );
        UserActivity { activity }
    }
}

impl Drop for UserActivity {
    /// Ends the activity, so macOS may throttle the app again.
    fn drop(&mut self) {
        // SAFETY: the token is the one `beginActivityWithOptions:reason:` answered, ended once.
        unsafe { NSProcessInfo::processInfo().endActivity(&self.activity) }
    }
}

#[cfg(test)]
mod tests {
    use super::UserActivity;

    #[test]
    fn an_activity_begins_and_ends_without_panicking() {
        let activity = UserActivity::begin("Waft's user activity test");
        drop(activity);
    }
}

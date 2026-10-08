//! The application's lifecycle state, as Flutter's macOS embedder derives it from what the
//! system says of the application (`FlutterEngine.mm`'s `handleWillBecomeActive:`,
//! `handleWillResignActive:` and `handleDidChangeOcclusionState:`): resumed while it is active
//! and some of it is visible, inactive while it is visible behind another application, hidden
//! while none of it is visible. The embedder reports each state it enters to the client, and the
//! framework fills in the states between, as Flutter's `ServicesBinding` does.

use inset_embedder::AppLifecycleState;

/// What the system says of the application as a whole, which Flutter's macOS embedder hears
/// through its application delegate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(target_os = "macos"),
    expect(dead_code, reason = "only macOS says these")
)]
pub(crate) enum LifecycleNotice {
    /// The application is about to become active, with whether any of its windows is
    /// visible then: AppKit's `NSApplicationWillBecomeActiveNotification`.
    WillBecomeActive { any_window_visible: bool },
    /// The application is about to stop being active: `NSApplicationWillResignActiveNotification`.
    WillResignActive,
    /// Whether any part of the application is visible changed, and whether it is now:
    /// `NSApplicationDidChangeOcclusionStateNotification`.
    OcclusionChanged { visible: bool },
}

/// Whether the application is active and whether any of it is visible, Flutter's `_active` and
/// `_visible`: neither until the system says so.
#[derive(Default)]
pub(crate) struct AppLifecycle {
    active: bool,
    visible: bool,
}

impl AppLifecycle {
    /// The state `notice` puts the application in, which is reported whether or not it is the
    /// one it was in, as Flutter's embedder reports it.
    pub(crate) fn state_after(&mut self, notice: LifecycleNotice) -> AppLifecycleState {
        match notice {
            LifecycleNotice::WillBecomeActive { any_window_visible } => {
                self.active = true;
                // The application's occlusion state can be stale when it comes back to the
                // front (Flutter issue 155977), so a window ordered in counts as visible; a
                // minimized one is not.
                if any_window_visible {
                    self.visible = true;
                }
                if self.visible {
                    AppLifecycleState::Resumed
                } else {
                    AppLifecycleState::Hidden
                }
            }
            LifecycleNotice::WillResignActive => {
                self.active = false;
                if self.visible {
                    AppLifecycleState::Inactive
                } else {
                    AppLifecycleState::Hidden
                }
            }
            LifecycleNotice::OcclusionChanged { visible } => {
                self.visible = visible;
                match (visible, self.active) {
                    (true, true) => AppLifecycleState::Resumed,
                    (true, false) => AppLifecycleState::Inactive,
                    (false, _) => AppLifecycleState::Hidden,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOWN: LifecycleNotice = LifecycleNotice::OcclusionChanged { visible: true };
    const COVERED_ALL_OVER: LifecycleNotice = LifecycleNotice::OcclusionChanged { visible: false };
    const TO_THE_FRONT: LifecycleNotice = LifecycleNotice::WillBecomeActive {
        any_window_visible: true,
    };

    #[test]
    fn an_application_in_front_and_visible_is_resumed() {
        let mut lifecycle = AppLifecycle::default();
        assert_eq!(
            lifecycle.state_after(TO_THE_FRONT),
            AppLifecycleState::Resumed
        );
        assert_eq!(lifecycle.state_after(SHOWN), AppLifecycleState::Resumed);
    }

    #[test]
    fn an_application_behind_another_is_inactive_while_it_is_visible() {
        let mut lifecycle = AppLifecycle::default();
        lifecycle.state_after(TO_THE_FRONT);
        assert_eq!(
            lifecycle.state_after(LifecycleNotice::WillResignActive),
            AppLifecycleState::Inactive
        );
        assert_eq!(
            lifecycle.state_after(COVERED_ALL_OVER),
            AppLifecycleState::Hidden
        );
        assert_eq!(lifecycle.state_after(SHOWN), AppLifecycleState::Inactive);
    }

    #[test]
    fn an_application_none_of_which_is_visible_is_hidden_even_in_front() {
        let mut lifecycle = AppLifecycle::default();
        lifecycle.state_after(COVERED_ALL_OVER);
        assert_eq!(
            lifecycle.state_after(LifecycleNotice::WillBecomeActive {
                any_window_visible: false,
            }),
            AppLifecycleState::Hidden
        );
        assert_eq!(
            lifecycle.state_after(LifecycleNotice::WillResignActive),
            AppLifecycleState::Hidden
        );
    }

    #[test]
    fn a_window_ordered_in_counts_as_visible_when_the_application_comes_to_the_front() {
        let mut lifecycle = AppLifecycle::default();
        lifecycle.state_after(COVERED_ALL_OVER);
        assert_eq!(
            lifecycle.state_after(TO_THE_FRONT),
            AppLifecycleState::Resumed
        );
    }
}

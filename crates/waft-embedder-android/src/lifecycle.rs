//! The application's lifecycle state, from what Android tells its activities, as Flutter's Android
//! embedding reports an activity's (`FlutterActivityAndFragmentDelegate`'s `onResume`, `onPause`,
//! `onStop`, `onWindowFocusChanged` and `onDetach`, through `LifecycleChannel`): resumed while the
//! activity is resumed and a window has the focus, inactive while it is resumed without the focus
//! or paused, paused once it is stopped, and detached once it is destroyed. `onStart` reports
//! nothing, as in Flutter: a started activity stays paused until it resumes.
//!
//! Flutter's engine is one activity's. Waft's app has a view in every activity, so the app is as far
//! forward as its most forward activity, and has the focus while any of its activities' windows
//! has, which is what `LifecycleChannel` takes "a window is focused" to mean; it is detached only
//! once no activity of it has been told a state. The framework fills in the states between the
//! ones reported, such as hidden, as Flutter's `ServicesBinding` does.

use std::collections::HashMap;

use waft_embedder::{AppLifecycleState, ViewId};

/// What Android last told one activity of its lifecycle, as its callbacks hear it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ActivityState {
    /// `onStop`: out of sight.
    Stopped,
    /// `onPause`: possibly in sight, not in front.
    Paused,
    /// `onResume`: in front.
    Resumed,
}

/// `LifecycleChannel.sendState`'s table: the state Flutter reports for an activity in `state`, or
/// destroyed, while a window of the app has the focus or not. The focus counts only while the
/// activity is resumed, since Android tells the focus and the lifecycle in no fixed order and no
/// other state takes input.
pub(crate) fn app_state_of(state: Option<ActivityState>, focused: bool) -> AppLifecycleState {
    match state {
        Some(ActivityState::Resumed) if focused => AppLifecycleState::Resumed,
        Some(ActivityState::Resumed | ActivityState::Paused) => AppLifecycleState::Inactive,
        Some(ActivityState::Stopped) => AppLifecycleState::Paused,
        None => AppLifecycleState::Detached,
    }
}

/// What one activity was told.
#[derive(Clone, Copy)]
struct Told {
    /// `None` until the activity's first `onResume`, `onPause` or `onStop`.
    state: Option<ActivityState>,
    /// Whether its window has the focus: taken to have it until told otherwise, as
    /// `LifecycleChannel`'s `lastFocus` starts.
    focused: bool,
}

impl Default for Told {
    fn default() -> Self {
        Told {
            state: None,
            focused: true,
        }
    }
}

/// What each activity Android made and has not destroyed was told, by the view it is.
#[derive(Default)]
pub(crate) struct ActivitiesLifecycle {
    activities: HashMap<ViewId, Told>,
}

impl ActivitiesLifecycle {
    /// The activity of the view numbered `view` is in `state` from now on.
    pub(crate) fn set_state(&mut self, view: ViewId, state: ActivityState) {
        self.activities.entry(view).or_default().state = Some(state);
    }

    /// The window of the activity of the view numbered `view` gained or lost the focus.
    pub(crate) fn set_focused(&mut self, view: ViewId, focused: bool) {
        self.activities.entry(view).or_default().focused = focused;
    }

    /// Android destroyed the activity of the view numbered `view`.
    pub(crate) fn remove(&mut self, view: ViewId) {
        self.activities.remove(&view);
    }

    /// The application's state: its most forward activity's, with the focus while any of its
    /// windows has it, or detached where no activity has been told a state.
    pub(crate) fn app_state(&self) -> AppLifecycleState {
        let state = self.activities.values().filter_map(|told| told.state).max();
        let focused = self.activities.values().any(|told| told.focused);
        app_state_of(state, focused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_state_and_focus_reports_flutters_state() {
        use ActivityState::*;
        use AppLifecycleState as App;
        for (state, focused, expected) in [
            (Some(Resumed), true, App::Resumed),
            (Some(Resumed), false, App::Inactive),
            (Some(Paused), true, App::Inactive),
            (Some(Paused), false, App::Inactive),
            (Some(Stopped), true, App::Paused),
            (Some(Stopped), false, App::Paused),
            (None, true, App::Detached),
            (None, false, App::Detached),
        ] {
            assert_eq!(
                app_state_of(state, focused),
                expected,
                "{state:?}, {focused}"
            );
        }
    }

    #[test]
    fn an_activity_goes_through_flutters_states_as_android_tells_it() {
        let view = ViewId(1);
        let mut lifecycle = ActivitiesLifecycle::default();
        // Focus told before any state reports nothing, as in Flutter.
        lifecycle.set_focused(view, false);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Detached);
        lifecycle.set_focused(view, true);
        lifecycle.set_state(view, ActivityState::Resumed);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Resumed);
        lifecycle.set_focused(view, false);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Inactive);
        lifecycle.set_state(view, ActivityState::Paused);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Inactive);
        lifecycle.set_state(view, ActivityState::Stopped);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Paused);
        lifecycle.remove(view);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Detached);
    }

    #[test]
    fn a_resumed_activity_has_the_focus_until_told_otherwise() {
        let mut lifecycle = ActivitiesLifecycle::default();
        lifecycle.set_state(ViewId(1), ActivityState::Resumed);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Resumed);
    }

    #[test]
    fn the_app_is_as_far_forward_as_its_most_forward_activity() {
        let (back, front) = (ViewId(1), ViewId(2));
        let mut lifecycle = ActivitiesLifecycle::default();
        lifecycle.set_state(back, ActivityState::Stopped);
        lifecycle.set_focused(back, false);
        lifecycle.set_state(front, ActivityState::Resumed);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Resumed);
        lifecycle.set_state(front, ActivityState::Paused);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Inactive);
        lifecycle.remove(front);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Paused);
    }
}

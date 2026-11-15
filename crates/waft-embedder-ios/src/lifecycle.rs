//! The application's lifecycle state, from the states of its scenes, as Flutter's iOS embedding
//! reports a scene's (`FlutterViewController.mm`'s `sceneBecameActive:`, `sceneWillResignActive:`,
//! `sceneWillEnterForeground:`, `sceneDidEnterBackground:` and `sceneWillDisconnect:`): resumed while
//! the scene is active, inactive while it is in the foreground but not active, paused in the
//! background, and detached once it is gone.
//!
//! Flutter's engine is one scene's, so its state is that scene's. Waft's app has a view in every
//! scene, so the app is as far forward as its most forward scene: resumed while any scene is
//! active, and detached only once no scene is connected. The framework fills in the states between
//! the ones reported, as Flutter's `ServicesBinding` does.

use std::collections::HashMap;

use waft_embedder::AppLifecycleState;

/// What UIKit says of one scene, as its delegate hears it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SceneState {
    /// In the background, or connected and not yet in the foreground.
    Background,
    /// In the foreground, not taking events: entering it, leaving it, or behind the app switcher.
    Inactive,
    /// In the foreground, taking events.
    Active,
}

/// Each connected scene's state, by its session's id.
#[derive(Default)]
pub(crate) struct ScenesLifecycle {
    scenes: HashMap<String, SceneState>,
}

impl ScenesLifecycle {
    /// The scene of `session` is in `state` from now on.
    pub(crate) fn set(&mut self, session: &str, state: SceneState) {
        self.scenes.insert(session.to_owned(), state);
    }

    /// The scene of `session` is connected no more.
    pub(crate) fn remove(&mut self, session: &str) {
        self.scenes.remove(session);
    }

    /// The application's state: its most forward scene's, or detached with none.
    pub(crate) fn app_state(&self) -> AppLifecycleState {
        match self.scenes.values().max() {
            Some(SceneState::Active) => AppLifecycleState::Resumed,
            Some(SceneState::Inactive) => AppLifecycleState::Inactive,
            Some(SceneState::Background) => AppLifecycleState::Paused,
            None => AppLifecycleState::Detached,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_reports_flutters_state_for_each_of_its_own() {
        let mut lifecycle = ScenesLifecycle::default();
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Detached);
        lifecycle.set("a", SceneState::Background);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Paused);
        lifecycle.set("a", SceneState::Inactive);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Inactive);
        lifecycle.set("a", SceneState::Active);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Resumed);
        lifecycle.remove("a");
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Detached);
    }

    #[test]
    fn the_app_is_as_far_forward_as_its_most_forward_scene() {
        let mut lifecycle = ScenesLifecycle::default();
        lifecycle.set("a", SceneState::Background);
        lifecycle.set("b", SceneState::Active);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Resumed);
        lifecycle.set("b", SceneState::Background);
        assert_eq!(lifecycle.app_state(), AppLifecycleState::Paused);
    }
}

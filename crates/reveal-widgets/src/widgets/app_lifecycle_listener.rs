//! Flutter counterpart: `widgets/app_lifecycle_listener.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_foundation::{App, Handle};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};

pub use reveal_embedder::{AppExitResponse, AppLifecycleState};

/// A callback type that is used by [`AppLifecycleListener::on_exit_requested`] to ask the
/// application if it wants to cancel application termination or not.
pub type AppExitRequestCallback = Rc<dyn Fn(&mut App) -> AppExitResponse>;

/// A listener that can be used to listen to changes in the application lifecycle.
///
/// To listen for requests for the application to exit, and to decide whether or not the
/// application should exit when requested, create an [`AppLifecycleListener`] and set the
/// [`on_exit_requested`](Self::on_exit_requested) callback.
///
/// To listen for changes in the application lifecycle state, define an
/// [`on_state_change`](Self::on_state_change) callback. See the [`AppLifecycleState`] enum
/// for details on the various states.
///
/// The [`on_state_change`](Self::on_state_change) callback is called for each state change,
/// and the individual state transitions ([`on_resume`](Self::on_resume),
/// [`on_inactive`](Self::on_inactive), etc.) are also called if the state transition they
/// represent occurs.
///
/// State changes will occur in accordance with the state machine described by
/// [`AppLifecycleState`].
///
/// The initial state of the state machine is the [`AppLifecycleState::Detached`] state, and
/// the arrows describe valid state transitions.
///
/// See also:
///
/// * `ServicesBinding::exit_application` for a function to call that will request that the
///   application exits.
/// * [`WidgetsBindingObserver::did_request_app_exit`](crate::WidgetsBindingObserver::did_request_app_exit)
///   for the handler which this class uses to receive exit requests.
/// * [`WidgetsBindingObserver::did_change_app_lifecycle_state`](crate::WidgetsBindingObserver::did_change_app_lifecycle_state)
///   for the handler which this class uses to receive lifecycle state changes.
#[allow(clippy::type_complexity)]
pub struct AppLifecycleListener {
    lifecycle_state: Option<AppLifecycleState>,
    /// The [`WidgetsBinding`] to listen to for application lifecycle events.
    ///
    /// Typically, this is set to [`WidgetsBinding::instance`].
    pub binding: Handle<WidgetsBinding>,
    /// Called anytime the state changes, passing the new state.
    pub on_state_change: Option<Rc<dyn Fn(&mut App, AppLifecycleState)>>,
    /// A callback that is called when the application loses input focus.
    ///
    /// On mobile platforms, this can be during a phone call or when a system dialog is
    /// visible.
    ///
    /// On desktop platforms, this is when all views in an application have lost input
    /// focus but at least one view of the application is still visible.
    ///
    /// On the web, this is when the window (or tab) has lost input focus.
    pub on_inactive: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback that is called when a view in the application gains input focus.
    ///
    /// A call to this callback indicates that the application is entering a state where it
    /// is visible, active, and accepting user input.
    pub on_resume: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback that is called when the application is hidden.
    ///
    /// On mobile platforms, this is usually just before the application is replaced by
    /// another application in the foreground.
    ///
    /// On desktop platforms, this is just before the application is hidden by being
    /// minimized or otherwise hiding all views of the application.
    ///
    /// On the web, this is just before a window (or tab) is hidden.
    pub on_hide: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback that is called when the application is shown.
    ///
    /// On mobile platforms, this is usually just before the application replaces another
    /// application in the foreground.
    ///
    /// On desktop platforms, this is just before the application is shown after being
    /// minimized or otherwise made to show at least one view of the application.
    ///
    /// On the web, this is just before a window (or tab) is shown.
    pub on_show: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback that is called when the application is paused.
    ///
    /// On mobile platforms, this happens right before the application is replaced by
    /// another application.
    ///
    /// On desktop platforms and the web, this function is not called.
    pub on_pause: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback that is called when the application is resumed after being paused.
    ///
    /// On mobile platforms, this happens just before this application takes over as the
    /// active application.
    ///
    /// On desktop platforms and the web, this function is not called.
    pub on_restart: Option<Rc<dyn Fn(&mut App)>>,
    /// A callback used to ask the application if it will allow exiting the application for
    /// cases where the exit is cancelable.
    ///
    /// Exiting the application isn't always cancelable, but when it is, this function will
    /// be called before exit occurs.
    ///
    /// Responding [`AppExitResponse::Exit`] will continue termination, and responding
    /// [`AppExitResponse::Cancel`] will cancel it. If termination is not canceled, the
    /// application will immediately exit.
    pub on_exit_requested: Option<AppExitRequestCallback>,
    /// A callback that is called when an application has exited, and detached all host
    /// views from the engine.
    ///
    /// This callback is only called on iOS and Android.
    pub on_detach: Option<Rc<dyn Fn(&mut App)>>,
    observer: Option<WidgetsBindingObserverRef>,
    debug_disposed: bool,
}

impl Debug for AppLifecycleListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppLifecycleListener")
            .field("binding", &self.binding)
            .finish()
    }
}

impl AppLifecycleListener {
    /// Creates an [`AppLifecycleListener`].
    pub fn new(app: &mut App) -> Handle<AppLifecycleListener> {
        let binding = WidgetsBinding::instance(app);
        let this = app.create(AppLifecycleListener {
            lifecycle_state: None,
            binding,
            on_state_change: None,
            on_inactive: None,
            on_resume: None,
            on_hide: None,
            on_show: None,
            on_pause: None,
            on_restart: None,
            on_exit_requested: None,
            on_detach: None,
            observer: None,
            debug_disposed: false,
        });
        let observer: WidgetsBindingObserverRef = Rc::new(this);
        app.get_mut(this).observer = Some(Rc::clone(&observer));
        binding.add_observer(app, observer);
        this
    }

    /// Dart `AppLifecycleListener(onStateChange:)`.
    #[allow(clippy::type_complexity)]
    pub fn on_state_change(
        self: Handle<Self>,
        app: &mut App,
        on_state_change: Rc<dyn Fn(&mut App, AppLifecycleState)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_state_change = Some(on_state_change);
        self
    }

    /// Dart `AppLifecycleListener(onInactive:)`.
    pub fn on_inactive(
        self: Handle<Self>,
        app: &mut App,
        on_inactive: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_inactive = Some(on_inactive);
        self
    }

    /// Dart `AppLifecycleListener(onResume:)`.
    pub fn on_resume(
        self: Handle<Self>,
        app: &mut App,
        on_resume: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_resume = Some(on_resume);
        self
    }

    /// Dart `AppLifecycleListener(onHide:)`.
    pub fn on_hide(
        self: Handle<Self>,
        app: &mut App,
        on_hide: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_hide = Some(on_hide);
        self
    }

    /// Dart `AppLifecycleListener(onShow:)`.
    pub fn on_show(
        self: Handle<Self>,
        app: &mut App,
        on_show: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_show = Some(on_show);
        self
    }

    /// Dart `AppLifecycleListener(onPause:)`.
    pub fn on_pause(
        self: Handle<Self>,
        app: &mut App,
        on_pause: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_pause = Some(on_pause);
        self
    }

    /// Dart `AppLifecycleListener(onRestart:)`.
    pub fn on_restart(
        self: Handle<Self>,
        app: &mut App,
        on_restart: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_restart = Some(on_restart);
        self
    }

    /// Dart `AppLifecycleListener(onExitRequested:)`.
    pub fn on_exit_requested(
        self: Handle<Self>,
        app: &mut App,
        on_exit_requested: AppExitRequestCallback,
    ) -> Handle<Self> {
        app.get_mut(self).on_exit_requested = Some(on_exit_requested);
        self
    }

    /// Dart `AppLifecycleListener(onDetach:)`.
    pub fn on_detach(
        self: Handle<Self>,
        app: &mut App,
        on_detach: Rc<dyn Fn(&mut App)>,
    ) -> Handle<Self> {
        app.get_mut(self).on_detach = Some(on_detach);
        self
    }

    /// Call when the listener is no longer in use.
    ///
    /// Do not use the object after calling [`dispose`](Self::dispose).
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_assert_not_disposed(app));
        let binding = app.get(self).binding;
        if let Some(observer) = app.get_mut(self).observer.take() {
            binding.remove_observer(app, &observer);
        }
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_disposed = true;
        }
    }

    fn debug_assert_not_disposed(self: Handle<Self>, app: &App) -> bool {
        debug_assert!(
            !app.get(self).debug_disposed,
            "A AppLifecycleListener was used after being disposed.\n\
             Once you have called dispose() on a AppLifecycleListener, it can no longer be used."
        );
        true
    }
}

impl WidgetsBindingObserverObject for AppLifecycleListener {
    fn did_request_app_exit(self: Handle<Self>, app: &mut App) -> AppExitResponse {
        debug_assert!(self.debug_assert_not_disposed(app));
        let on_exit_requested = app.get(self).on_exit_requested.clone();
        match on_exit_requested {
            None => AppExitResponse::Exit,
            Some(on_exit_requested) => on_exit_requested(app),
        }
    }

    fn did_change_app_lifecycle_state(self: Handle<Self>, app: &mut App, state: AppLifecycleState) {
        debug_assert!(self.debug_assert_not_disposed(app));
        let previous_state = app.get(self).lifecycle_state;
        if previous_state == Some(state) {
            // Transitioning to the same state twice doesn't produce any
            // notifications (but also won't actually occur).
            return;
        }
        app.get_mut(self).lifecycle_state = Some(state);
        match state {
            AppLifecycleState::Resumed => {
                debug_assert!(
                    previous_state.is_none()
                        || previous_state == Some(AppLifecycleState::Inactive)
                        || previous_state == Some(AppLifecycleState::Detached),
                    "Invalid state transition from {previous_state:?} to {state:?}"
                );
                let on_resume = app.get(self).on_resume.clone();
                if let Some(on_resume) = on_resume {
                    on_resume(app);
                }
            }
            AppLifecycleState::Inactive => {
                debug_assert!(
                    previous_state.is_none()
                        || previous_state == Some(AppLifecycleState::Hidden)
                        || previous_state == Some(AppLifecycleState::Resumed),
                    "Invalid state transition from {previous_state:?} to {state:?}"
                );
                if previous_state == Some(AppLifecycleState::Hidden) {
                    let on_show = app.get(self).on_show.clone();
                    if let Some(on_show) = on_show {
                        on_show(app);
                    }
                } else if previous_state.is_none()
                    || previous_state == Some(AppLifecycleState::Resumed)
                {
                    let on_inactive = app.get(self).on_inactive.clone();
                    if let Some(on_inactive) = on_inactive {
                        on_inactive(app);
                    }
                }
            }
            AppLifecycleState::Hidden => {
                debug_assert!(
                    previous_state.is_none()
                        || previous_state == Some(AppLifecycleState::Paused)
                        || previous_state == Some(AppLifecycleState::Inactive),
                    "Invalid state transition from {previous_state:?} to {state:?}"
                );
                if previous_state == Some(AppLifecycleState::Paused) {
                    let on_restart = app.get(self).on_restart.clone();
                    if let Some(on_restart) = on_restart {
                        on_restart(app);
                    }
                } else if previous_state.is_none()
                    || previous_state == Some(AppLifecycleState::Inactive)
                {
                    let on_hide = app.get(self).on_hide.clone();
                    if let Some(on_hide) = on_hide {
                        on_hide(app);
                    }
                }
            }
            AppLifecycleState::Paused => {
                debug_assert!(
                    previous_state.is_none() || previous_state == Some(AppLifecycleState::Hidden),
                    "Invalid state transition from {previous_state:?} to {state:?}"
                );
                if previous_state.is_none() || previous_state == Some(AppLifecycleState::Hidden) {
                    let on_pause = app.get(self).on_pause.clone();
                    if let Some(on_pause) = on_pause {
                        on_pause(app);
                    }
                }
            }
            AppLifecycleState::Detached => {
                debug_assert!(
                    previous_state.is_none() || previous_state == Some(AppLifecycleState::Paused),
                    "Invalid state transition from {previous_state:?} to {state:?}"
                );
                let on_detach = app.get(self).on_detach.clone();
                if let Some(on_detach) = on_detach {
                    on_detach(app);
                }
            }
        }
        // At this point, it can't be null anymore.
        let on_state_change = app.get(self).on_state_change.clone();
        if let Some(on_state_change) = on_state_change {
            on_state_change(app, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_foundation::AppCell;
    use std::cell::Cell;

    #[test]
    fn app_lifecycle_listener_on_resume_fires_and_dispose_removes_observer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let fired = Rc::new(Cell::new(false));
        let seen = Rc::clone(&fired);
        let listener = AppLifecycleListener::new(&mut app)
            .on_resume(&mut app, Rc::new(move |_app| seen.set(true)));
        let binding = WidgetsBinding::instance(&mut app);
        assert_eq!(binding.observer_count(&app), 1);
        listener.did_change_app_lifecycle_state(&mut app, AppLifecycleState::Resumed);
        assert!(fired.get());
        listener.dispose(&mut app);
        assert_eq!(binding.observer_count(&app), 0);
    }
}

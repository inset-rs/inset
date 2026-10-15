//! Flutter counterpart: `services/binding.dart` (`ServicesBinding`).
//!
//! Only the app life cycle and the application exit are here: the lifecycle messages the host
//! sends, which reach `SchedulerBinding::handle_app_lifecycle_state_changed`,
//! `handleRequestAppExit`, which the widgets binding overrides, and `exitApplication`. The
//! keyboard and the restoration manager are singletons of their own (`KeyEventManager`,
//! `RestorationManager`); the rest waits with platform channels.

use std::rc::Rc;

use inset_embedder::{AppExitResponse, AppExitType, AppLifecycleState};
use inset_foundation::{App, Handle, Task};
use inset_scheduler::SchedulerBinding;

use crate::SystemChrome;

/// The object side of Flutter's `WidgetsBinding` override of
/// `ServicesBinding.handleRequestAppExit`, which asks the widget layer's observers. Implement it
/// on the binding above and register `Rc::new(handle)` with [`ServicesBinding::set_overrides`];
/// `Handle<T>` is then [`ServicesBindingOverrides`].
pub trait ServicesBindingOverridesObject: Sized + 'static {
    /// Answers in place of [`ServicesBinding::handle_request_app_exit`]'s own answer.
    fn handle_request_app_exit(self: Handle<Self>, app: &mut App) -> Task<AppExitResponse>;
}

/// What the binding above this one overrides; kept erased.
pub trait ServicesBindingOverrides {
    /// See [`ServicesBindingOverridesObject::handle_request_app_exit`].
    fn handle_request_app_exit(&self, app: &mut App) -> Task<AppExitResponse>;
}

impl<T: ServicesBindingOverridesObject> ServicesBindingOverrides for Handle<T> {
    fn handle_request_app_exit(&self, app: &mut App) -> Task<AppExitResponse> {
        T::handle_request_app_exit(*self, app)
    }
}

/// Hands the host's lifecycle messages to the scheduler and the layers above, answers the host's
/// requests to exit the application, and asks the host to exit it.
#[derive(Default)]
pub struct ServicesBinding {
    overrides: Option<Rc<dyn ServicesBindingOverrides>>,
    /// Whether `initInstances`'s work is done, which the first `instance` does.
    initialized: bool,
}

impl ServicesBinding {
    /// The current [`ServicesBinding`], created on first use.
    ///
    /// Creating it reads the lifecycle state the host started in, as Dart's `initInstances`
    /// does.
    pub fn instance(app: &mut App) -> Handle<ServicesBinding> {
        let this: Handle<ServicesBinding> = app.singleton();
        if !app.get(this).initialized {
            app.get_mut(this).initialized = true;
            this.read_initial_lifecycle_state_from_native_window(app);
        }
        this
    }

    /// Registers the binding layered above this one, as Flutter's `WidgetsBinding` does by
    /// overriding `handleRequestAppExit`.
    pub fn set_overrides(
        self: Handle<Self>,
        app: &mut App,
        overrides: Rc<dyn ServicesBindingOverrides>,
    ) {
        app.get_mut(self).overrides = Some(overrides);
    }

    // App life cycle

    /// Initializes the [`SchedulerBinding::lifecycle_state`] with the
    /// `Platform::initial_lifecycle_state`.
    ///
    /// Once the lifecycle state is populated through any means (including this method), this
    /// method will do nothing. This is because the `Platform::initial_lifecycle_state` may
    /// already be stale and it no longer makes sense to use the initial state at startup as the
    /// current state anymore.
    ///
    /// The latest state should be obtained by subscribing to
    /// `WidgetsBindingObserver::did_change_app_lifecycle_state`.
    pub fn read_initial_lifecycle_state_from_native_window(self: Handle<Self>, app: &mut App) {
        let initial_lifecycle_state = app.platform().initial_lifecycle_state();
        let Some(initial_lifecycle_state) = initial_lifecycle_state else {
            return;
        };
        if SchedulerBinding::lifecycle_state(app).is_some() {
            return;
        }
        self.handle_lifecycle_message(app, initial_lifecycle_state);
    }

    /// Handles a lifecycle state the host reports: Dart's `_handleLifecycleMessage`, which
    /// `SystemChannels.lifecycle` calls with the state's name, and which the host reaches through
    /// `EmbedderClient::app_lifecycle_state_changed`. The states between the one the app is in
    /// and `state` are handled first, in order.
    pub fn handle_lifecycle_message(self: Handle<Self>, app: &mut App, state: AppLifecycleState) {
        let generated = generate_state_transitions(SchedulerBinding::lifecycle_state(app), state);
        for state_change in generated {
            SchedulerBinding::handle_app_lifecycle_state_changed(app, state_change);
            SystemChrome::handle_app_lifecycle_state_changed(app, state_change);
        }
    }

    /// Handles any requests for application exit that the host sends through
    /// `EmbedderClient::request_app_exit`.
    ///
    /// By default, returns [`AppExitResponse::Exit`].
    ///
    /// Not all exits are cancelable, so not all exits will call this function. Do not rely on
    /// this function as a place to save critical data, because you will be disappointed. There
    /// are a number of ways that the application can exit without letting the application know
    /// first: power can be unplugged, the battery removed, the application can be killed in a
    /// task manager or command line, or the device could have a rapid unplanned disassembly
    /// (i.e. it could explode). In all of those cases (and probably others), no notification
    /// will be given to the application that it is about to exit.
    ///
    /// See also:
    ///
    /// * `WidgetsBindingObserver::did_request_app_exit`, which can be overridden to respond to
    ///   this message.
    /// * `WidgetsBinding`'s override of this method, which notifies its observers.
    pub fn handle_request_app_exit(self: Handle<Self>, app: &mut App) -> Task<AppExitResponse> {
        let overrides = app.get(self).overrides.clone();
        match overrides {
            Some(overrides) => overrides.handle_request_app_exit(app),
            None => Task::ready(AppExitResponse::Exit),
        }
    }

    /// Exits the application by calling the native application API method for exiting an
    /// application cleanly.
    ///
    /// This differs from calling [`std::process::exit`] in that it gives the engine a chance
    /// to clean up resources so that it doesn't crash on exit, so calling this is always
    /// preferred over calling [`std::process::exit`]. It also optionally gives handlers of
    /// [`handle_request_app_exit`](Self::handle_request_app_exit) a chance to cancel the
    /// application exit.
    ///
    /// The `exit_type` indicates what kind of exit to perform. For
    /// [`AppExitType::Cancelable`] exits, the application is queried through a call to
    /// [`handle_request_app_exit`](Self::handle_request_app_exit), where the application can
    /// optionally cancel the request for exit. If the `exit_type` is
    /// [`AppExitType::Required`], then the application exits immediately without querying the
    /// application.
    ///
    /// For [`AppExitType::Cancelable`] exits, the returned response value is the response
    /// obtained from the application as to whether the exit was canceled or not. Practically,
    /// the response will never be [`AppExitResponse::Exit`], since the application will have
    /// already exited by the time the result would have been received.
    ///
    /// The `exit_code` argument will be used as the application exit code on platforms where
    /// an exit code is supported. On other platforms it may be ignored. Dart's default is
    /// zero.
    ///
    /// Flutter's engine asks the framework back for a cancelable exit. Here the framework asks
    /// itself, on a later turn as the engine's request would come, and only a decided exit
    /// reaches the host, as [`AppExit::quit`](inset_embedder::AppExit::quit); a host that cannot
    /// quit cancels every exit.
    ///
    /// See also:
    ///
    /// * `WidgetsBindingObserver::did_request_app_exit` for a handler you can override on a
    ///   `WidgetsBindingObserver` to receive exit requests.
    pub fn exit_application(
        self: Handle<Self>,
        app: &App,
        exit_type: AppExitType,
        exit_code: i32,
    ) -> Task<AppExitResponse> {
        let platform = app.platform();
        let Some(app_exit) = platform.app_exit() else {
            return Task::ready(AppExitResponse::Cancel);
        };
        match exit_type {
            AppExitType::Required => {
                app_exit.quit(exit_code);
                Task::ready(AppExitResponse::Exit)
            }
            AppExitType::Cancelable => app.spawn(async move |cx| {
                let response = cx.update(|app| self.handle_request_app_exit(app)).await;
                if response == AppExitResponse::Exit {
                    cx.update(|app| quit(app, exit_code));
                }
                response
            }),
        }
    }
}

/// Quits through the host, where it can quit.
fn quit(app: &App, exit_code: i32) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(exit_code);
    }
}

/// The states from `previous_state` to `state`, in the order the state machine goes through
/// them, `state` last; `state` alone from no state, and none to the state already held.
fn generate_state_transitions(
    previous_state: Option<AppLifecycleState>,
    state: AppLifecycleState,
) -> Vec<AppLifecycleState> {
    if previous_state == Some(state) {
        return Vec::new();
    }
    let mut state_changes = Vec::new();
    match previous_state {
        // If there was no previous state, just jump directly to the new state.
        None => state_changes.push(state),
        Some(previous_state) => {
            let previous_state_index = state_index(previous_state);
            let state_index = state_index(state);
            if state == AppLifecycleState::Detached {
                state_changes.extend(STATE_ORDER[previous_state_index + 1..].iter().copied());
                state_changes.push(AppLifecycleState::Detached);
            } else if previous_state_index > state_index {
                state_changes.extend(STATE_ORDER[state_index..previous_state_index].iter().rev());
            } else {
                state_changes.extend(&STATE_ORDER[previous_state_index + 1..=state_index]);
            }
        }
    }
    debug_assert!(
        {
            let mut starting = previous_state;
            state_changes.iter().all(|&ending| {
                let valid = debug_verify_lifecycle_change(starting, ending);
                starting = Some(ending);
                valid
            })
        },
        "Invalid lifecycle state transition generated from {previous_state:?} to {state:?} (generated {state_changes:?})"
    );
    state_changes
}

/// Dart's `AppLifecycleState.values`: the order the state machine goes through.
const STATE_ORDER: [AppLifecycleState; 5] = [
    AppLifecycleState::Detached,
    AppLifecycleState::Resumed,
    AppLifecycleState::Inactive,
    AppLifecycleState::Hidden,
    AppLifecycleState::Paused,
];

fn state_index(state: AppLifecycleState) -> usize {
    STATE_ORDER
        .iter()
        .position(|&ordered| ordered == state)
        .expect("every state is in the order")
}

fn debug_verify_lifecycle_change(
    starting: Option<AppLifecycleState>,
    ending: AppLifecycleState,
) -> bool {
    let Some(starting) = starting else {
        // Any transition from null is fine, since it is initializing the state.
        return true;
    };
    if starting == ending {
        // Any transition to itself shouldn't happen.
        return false;
    }
    match starting {
        // Can't go from resumed to detached directly (must go through paused).
        AppLifecycleState::Resumed => ending == AppLifecycleState::Inactive,
        AppLifecycleState::Detached => {
            ending == AppLifecycleState::Resumed || ending == AppLifecycleState::Paused
        }
        AppLifecycleState::Inactive => {
            ending == AppLifecycleState::Resumed || ending == AppLifecycleState::Hidden
        }
        AppLifecycleState::Hidden => {
            ending == AppLifecycleState::Paused || ending == AppLifecycleState::Inactive
        }
        AppLifecycleState::Paused => {
            ending == AppLifecycleState::Hidden || ending == AppLifecycleState::Detached
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use std::future::Future;

    use inset_embedder::AppExit;
    use inset_foundation::{AppCell, Completer};
    use inset_test::TestPlatform;

    use super::*;

    /// A host that records the exit code it was told to quit with.
    #[derive(Default)]
    struct RecordingAppExit {
        quit_with: Cell<Option<i32>>,
    }

    impl AppExit for RecordingAppExit {
        fn quit(&self, exit_code: i32) {
            self.quit_with.set(Some(exit_code));
        }
    }

    /// An app whose host quits through `host`.
    fn quitting_through(host: &Rc<RecordingAppExit>) -> Rc<AppCell> {
        AppCell::with_platform(Rc::new(
            TestPlatform::new().with_app_exit(Rc::clone(host) as Rc<dyn AppExit>),
        ))
    }

    /// Makes `binding` the one above the services binding's own answer.
    fn answer_with<T: ServicesBindingOverridesObject + 'static>(cell: &Rc<AppCell>, binding: T) {
        let mut app = cell.borrow_mut();
        let binding = app.create(binding);
        ServicesBinding::instance(&mut app).set_overrides(&mut app, Rc::new(binding));
    }

    /// Awaits `answer` in a task, and runs the checkpoint that task needs; `None` while the
    /// answer is still on its way.
    fn awaited(
        cell: &Rc<AppCell>,
        answer: impl Future<Output = AppExitResponse> + 'static,
    ) -> Rc<Cell<Option<AppExitResponse>>> {
        let awaited = Rc::new(Cell::new(None));
        let out = Rc::clone(&awaited);
        cell.borrow()
            .spawn(async move |_cx| out.set(Some(answer.await)));
        cell.checkpoint();
        awaited
    }

    fn exit_application(cell: &Rc<AppCell>, exit_type: AppExitType) -> AppExitResponse {
        let response = {
            let mut app = cell.borrow_mut();
            ServicesBinding::instance(&mut app).exit_application(&app, exit_type, 0)
        };
        let answer = awaited(cell, response).get();
        answer.expect("the exit was answered")
    }

    fn handle_request_app_exit(cell: &Rc<AppCell>) -> Rc<Cell<Option<AppExitResponse>>> {
        let response = {
            let mut app = cell.borrow_mut();
            ServicesBinding::instance(&mut app).handle_request_app_exit(&mut app)
        };
        awaited(cell, response)
    }

    // widgets/binding_test.dart 'handleLifecycleStateChanged callback'.
    #[test]
    fn the_states_between_are_generated_in_order() {
        use AppLifecycleState::*;
        let steps = [
            (None, Paused, vec![Paused]),
            (Some(Paused), Resumed, vec![Hidden, Inactive, Resumed]),
            (Some(Resumed), Paused, vec![Inactive, Hidden, Paused]),
            (Some(Paused), Inactive, vec![Hidden, Inactive]),
            (Some(Inactive), Hidden, vec![Hidden]),
            (Some(Hidden), Paused, vec![Paused]),
            (Some(Paused), Detached, vec![Detached]),
            (Some(Detached), Resumed, vec![Resumed]),
            (
                Some(Resumed),
                Detached,
                vec![Inactive, Hidden, Paused, Detached],
            ),
            (Some(Hidden), Hidden, vec![]),
        ];
        for (previous_state, state, generated) in steps {
            assert_eq!(
                generate_state_transitions(previous_state, state),
                generated,
                "from {previous_state:?} to {state:?}"
            );
        }
    }

    #[test]
    fn the_initial_state_is_read_when_the_binding_is_made() {
        let cell = AppCell::with_platform(Rc::new(
            TestPlatform::new().with_initial_lifecycle_state(AppLifecycleState::Hidden),
        ));
        let mut app = cell.borrow_mut();
        ServicesBinding::instance(&mut app);
        assert_eq!(
            SchedulerBinding::lifecycle_state(&mut app),
            Some(AppLifecycleState::Hidden)
        );
        assert!(!SchedulerBinding::frames_enabled(&mut app));
    }

    #[test]
    fn a_host_without_an_initial_state_leaves_the_app_without_one() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        ServicesBinding::instance(&mut app);
        assert_eq!(SchedulerBinding::lifecycle_state(&mut app), None);
    }

    #[test]
    fn a_required_exit_quits_without_asking() {
        let host = Rc::default();
        let cell = quitting_through(&host);
        answer_with(&cell, Refusing);
        let response = exit_application(&cell, AppExitType::Required);
        assert_eq!(host.quit_with.get(), Some(0));
        assert_eq!(response, AppExitResponse::Exit);
    }

    #[test]
    fn a_cancelable_exit_quits_once_the_app_agrees() {
        let host = Rc::default();
        let cell = quitting_through(&host);
        let response = exit_application(&cell, AppExitType::Cancelable);
        assert_eq!(host.quit_with.get(), Some(0));
        assert_eq!(response, AppExitResponse::Exit);
    }

    #[test]
    fn a_cancelable_exit_the_app_refuses_does_not_quit() {
        let host = Rc::default();
        let cell = quitting_through(&host);
        answer_with(&cell, Refusing);
        let response = exit_application(&cell, AppExitType::Cancelable);
        assert_eq!(host.quit_with.get(), None);
        assert_eq!(response, AppExitResponse::Cancel);
    }

    #[test]
    fn exit_application_is_canceled_where_the_host_cannot_exit() {
        let cell = AppCell::with_platform(Rc::new(TestPlatform::new()));
        let response = exit_application(&cell, AppExitType::Cancelable);
        assert_eq!(response, AppExitResponse::Cancel);
    }

    #[test]
    fn default_handle_request_app_exit_returns_exit() {
        let cell = AppCell::new();
        let response = handle_request_app_exit(&cell);
        assert_eq!(response.get(), Some(AppExitResponse::Exit));
    }

    /// Refuses every exit, as a binding whose observer cancels.
    struct Refusing;

    impl ServicesBindingOverridesObject for Refusing {
        fn handle_request_app_exit(self: Handle<Self>, _app: &mut App) -> Task<AppExitResponse> {
            Task::ready(AppExitResponse::Cancel)
        }
    }

    /// Answers once `decision` completes, as a binding whose observers ask the user.
    struct Deciding {
        decision: Completer<AppExitResponse>,
    }

    impl ServicesBindingOverridesObject for Deciding {
        fn handle_request_app_exit(self: Handle<Self>, app: &mut App) -> Task<AppExitResponse> {
            let decision = app.get(self).decision.future();
            app.spawn(async move |_cx| decision.await)
        }
    }

    #[test]
    fn the_binding_above_answers_in_place_of_the_default_once_it_has_decided() {
        let cell = AppCell::new();
        let decision = Completer::new();
        answer_with(
            &cell,
            Deciding {
                decision: decision.clone(),
            },
        );
        let response = handle_request_app_exit(&cell);
        assert_eq!(response.get(), None, "still deciding");

        decision.complete(&mut cell.borrow_mut(), AppExitResponse::Cancel);
        cell.checkpoint();
        assert_eq!(response.get(), Some(AppExitResponse::Cancel));
    }
}

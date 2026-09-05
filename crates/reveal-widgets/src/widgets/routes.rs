//! Flutter counterpart: `widgets/routes.dart`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatus,
    AnimationStatusListener, AnyAnimation, ColorTween, Curve, CurveTween, Curves, ProxyAnimation,
    TrainHoppingAnimation, k_always_complete_animation, k_always_dismissed_animation,
};
use reveal_foundation::{App, Handle, HandleId, Listener, MergingListenable, ValueListenable};
use reveal_painting::Color;
use reveal_physics::Simulation;
use reveal_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase, TickerFuture};

use crate::framework::{
    BuildContext, GlobalKey, InheritedModel, InheritedWidget, IntoWidget, KeyRef, Notification,
    State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::actions::{Action, ActionData, Actions, DismissAction, DismissIntent};
use crate::widgets::basic::{Builder, IgnorePointer, Offstage, RepaintBoundary};
use crate::widgets::focus_manager::{FocusNodeLeaf, FocusScopeNode, TraversalEdgeBehavior};
use crate::widgets::focus_scope::FocusScope;
use crate::widgets::modal_barrier::{AnimatedModalBarrier, ModalBarrier};
use crate::widgets::navigator::{
    AnyRoute, NavigationNotification, Navigator, NavigatorObserver, NavigatorObserverData, Route,
    RouteBase, RouteCompleter, RoutePopDisposition, RoutePredicate, RouteResult,
    RouteResultCallback, RouteSettingsRef,
};
use crate::widgets::overlay::OverlayEntry;
use crate::widgets::page_storage::{PageStorage, PageStorageBucket};
use crate::widgets::primary_scroll_controller::PrimaryScrollController;
use crate::widgets::restoration::RestorationScope;
use crate::widgets::scroll_controller::ScrollController;
use crate::widgets::scroll_controller::ScrollControllerLeaf;
use crate::widgets::transitions::{AnimatedBuilder, DelegatedTransitionBuilder, ListenableBuilder};

// ---------------------------------------------------------------------------------------------
// OverlayRoute

/// The fields Dart's `OverlayRoute` declares; every overlay route carries this bag under the
/// field `overlay_route`.
pub struct OverlayRouteData {
    overlay_entries: Vec<Handle<OverlayEntry>>,
}

impl OverlayRouteData {
    /// The bag of a freshly created overlay route.
    pub fn new() -> OverlayRouteData {
        OverlayRouteData {
            overlay_entries: Vec::new(),
        }
    }
}

impl Default for OverlayRouteData {
    fn default() -> OverlayRouteData {
        OverlayRouteData::new()
    }
}

/// The accessors [`OverlayRoute`] asks for, for a struct whose bag is the field
/// `overlay_route`.
#[macro_export]
macro_rules! overlay_route_accessors {
    () => {
        fn overlay_route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::OverlayRouteData {
            &app.get(self).overlay_route
        }

        fn overlay_route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::OverlayRouteData {
            &mut app.get_mut(self).overlay_route
        }
    };
}

/// A route that displays widgets in the [`Navigator`]'s `Overlay`.
///
/// See also:
///
///  * [`Route`], which documents the meaning of Dart's `T` generic type argument.
pub trait OverlayRoute: Route {
    /// Dart's `OverlayRoute` fields, held under the field `overlay_route`
    /// ([`overlay_route_accessors!`](crate::overlay_route_accessors)).
    fn overlay_route_data(self: Handle<Self>, app: &App) -> &OverlayRouteData;

    /// See [`overlay_route_data`](Self::overlay_route_data).
    fn overlay_route_data_mut(self: Handle<Self>, app: &mut App) -> &mut OverlayRouteData;

    /// Subclasses should override this method to return the builders for the overlay.
    fn create_overlay_entries(self: Handle<Self>, app: &mut App) -> Vec<Handle<OverlayEntry>>;

    /// Dart's `OverlayRoute.overlayEntries`.
    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<OverlayEntry>> {
        self.overlay_route_data(app).overlay_entries.clone()
    }

    /// Dart's `OverlayRoute.install`.
    fn install(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.overlay_route_data(app).overlay_entries.is_empty());
        let entries = OverlayRoute::create_overlay_entries(self, app);
        self.overlay_route_data_mut(app)
            .overlay_entries
            .extend(entries);
        RouteBase::install(self, app);
    }

    /// Controls whether [`did_pop`](Self::did_pop) calls [`NavigatorState::finalize_route`](crate::NavigatorState::finalize_route).
    ///
    /// If true, this route removes its overlay entries during
    /// [`did_pop`](Route::did_pop). Subclasses can override this getter if they want to delay
    /// finalization (for example to animate the route's exit before removing it from the
    /// overlay).
    ///
    /// Subclasses that return false from `finished_when_popped` are responsible for calling
    /// [`NavigatorState::finalize_route`](crate::NavigatorState::finalize_route) themselves.
    fn finished_when_popped(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// Dart's `OverlayRoute.didPop`.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        let return_value = RouteBase::did_pop(self, app, result);
        debug_assert!(return_value);
        if OverlayRoute::finished_when_popped(self, app) {
            let navigator = self
                .as_route()
                .navigator(app)
                .expect("a popped route is installed");
            navigator.finalize_route(app, self.as_route());
        }
        return_value
    }

    /// Dart's `OverlayRoute.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        for entry in self.overlay_route_data(app).overlay_entries.clone() {
            entry.dispose(app);
        }
        self.overlay_route_data_mut(app).overlay_entries.clear();
        RouteBase::dispose(self, app);
    }
}

// ---------------------------------------------------------------------------------------------
// TransitionRoute

/// The fields Dart's `TransitionRoute` declares; every transition route carries this bag under
/// the field `transition_route`.
pub struct TransitionRouteData {
    transition_completer: RouteCompleter,
    pop_finalized: bool,
    animation: Option<AnyAnimation<f64>>,
    controller: Option<Handle<AnimationController>>,
    secondary_animation: Handle<ProxyAnimation>,
    /// Whether to takeover the [`controller`](TransitionRoute::controller) created by
    /// [`create_animation_controller`](TransitionRoute::create_animation_controller).
    ///
    /// If true, this route will call `AnimationController::dispose` when the controller is no
    /// longer needed. If false, the controller should be disposed by whoever owned it.
    ///
    /// It defaults to `true`.
    pub will_dispose_animation_controller: bool,
    result: RouteResult,
    train_hopping_listener_remover: Option<TrainHoppingListenerRemover>,
}

/// Dart's `_trainHoppingListenerRemover`: disposes the running train-hopping animation and
/// removes its listener.
type TrainHoppingListenerRemover = Rc<dyn Fn(&mut App)>;

impl TransitionRouteData {
    /// The bag of a freshly created transition route.
    pub fn new(app: &mut App) -> TransitionRouteData {
        let dismissed = k_always_dismissed_animation(app);
        TransitionRouteData {
            transition_completer: RouteCompleter::new(),
            pop_finalized: false,
            animation: None,
            controller: None,
            secondary_animation: ProxyAnimation::new(app, Some(dismissed)),
            will_dispose_animation_controller: true,
            result: None,
            train_hopping_listener_remover: None,
        }
    }
}

/// The accessors [`TransitionRoute`] asks for, for a struct whose bag is the field
/// `transition_route`.
#[macro_export]
macro_rules! transition_route_accessors {
    () => {
        fn transition_route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::TransitionRouteData {
            &app.get(self).transition_route
        }

        fn transition_route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::TransitionRouteData {
            &mut app.get_mut(self).transition_route
        }
    };
}

/// An interface for a route that supports predictive back gestures.
pub trait PredictiveBackRoute: Route {
    /// Whether a pop gesture can be started by the user for this route.
    fn pop_gesture_enabled(self: Handle<Self>, app: &mut App) -> bool;

    /// Handles a predictive back gesture starting.
    ///
    /// The `progress` parameter indicates the progress of the gesture from 0.0 to 1.0.
    fn handle_start_back_gesture(self: Handle<Self>, app: &mut App, progress: f64);

    /// Handles a predictive back gesture updating as the user drags across the screen.
    ///
    /// The `progress` parameter indicates the progress of the gesture from 0.0 to 1.0.
    fn handle_update_back_gesture_progress(self: Handle<Self>, app: &mut App, progress: f64);

    /// Handles a predictive back gesture ending successfully.
    fn handle_commit_back_gesture(self: Handle<Self>, app: &mut App);

    /// Handles a predictive back gesture ending in cancellation.
    fn handle_cancel_back_gesture(self: Handle<Self>, app: &mut App);
}

/// A route with entrance and exit transitions.
///
/// See also:
///
///  * [`Route`], which documents the meaning of Dart's `T` generic type argument.
pub trait TransitionRoute: OverlayRoute + PredictiveBackRoute {
    /// Dart's `TransitionRoute` fields, held under the field `transition_route`
    /// ([`transition_route_accessors!`](crate::transition_route_accessors)).
    fn transition_route_data(self: Handle<Self>, app: &App) -> &TransitionRouteData;

    /// See [`transition_route_data`](Self::transition_route_data).
    fn transition_route_data_mut(self: Handle<Self>, app: &mut App) -> &mut TransitionRouteData;

    /// This route as the erased [`AnyTransitionRoute`].
    fn as_transition_route(self: Handle<Self>) -> AnyTransitionRoute {
        AnyTransitionRoute {
            id: self.id(),
            vtable: const { &TransitionRouteVTable::of::<Self>() },
        }
    }

    /// Registers a callback that runs once the transition itself has finished, after the
    /// overlay entries have been removed from the navigator's overlay.
    ///
    /// It runs once the animation has been dismissed. That is after `popped`, because `popped`
    /// typically resolves before the animation even starts, as soon as the route is popped.
    ///
    /// Dart's `completed` future.
    fn when_completed(self: Handle<Self>, app: &mut App, callback: RouteResultCallback) {
        match self
            .transition_route_data(app)
            .transition_completer
            .result()
        {
            None => self
                .transition_route_data_mut(app)
                .transition_completer
                .push(callback),
            Some(result) => {
                app.schedule_microtask(Listener::new(move |app| callback(app, result.clone())));
            }
        }
    }

    /// The duration the transition going forwards.
    ///
    /// See also:
    ///
    ///  * [`reverse_transition_duration`](Self::reverse_transition_duration), which controls
    ///    the duration of the transition when it is in reverse.
    fn transition_duration(self: Handle<Self>, app: &App) -> Duration;

    /// The duration the transition going in reverse.
    ///
    /// By default, the reverse transition duration is set to the value of the forwards
    /// [`transition_duration`](Self::transition_duration).
    fn reverse_transition_duration(self: Handle<Self>, app: &App) -> Duration {
        TransitionRoute::transition_duration(self, app)
    }

    /// Whether the route obscures previous routes when the transition is complete.
    ///
    /// When an opaque route's entrance transition is complete, the routes behind the opaque
    /// route will not be built to save resources.
    fn opaque(self: Handle<Self>, app: &App) -> bool;

    /// Whether the route transition will prefer to animate a snapshot of the entering and
    /// exiting routes.
    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// Dart's `TransitionRoute.finishedWhenPopped`.
    ///
    /// This ensures that if we got to the dismissed state while still current, we will still be
    /// disposed when we are eventually popped.
    fn finished_when_popped(self: Handle<Self>, app: &App) -> bool {
        let controller = self
            .transition_route_data(app)
            .controller
            .expect("an installed route");
        controller.status(app).is_dismissed() && !self.transition_route_data(app).pop_finalized
    }

    /// The animation that drives the route's transition and the previous route's forward
    /// transition.
    fn animation(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        self.transition_route_data(app).animation
    }

    /// The animation controller that the route uses to drive the transitions.
    ///
    /// The animation itself is exposed by [`animation`](Self::animation).
    fn controller(self: Handle<Self>, app: &App) -> Option<Handle<AnimationController>> {
        self.transition_route_data(app).controller
    }

    /// The animation for the route being pushed on top of this route.
    ///
    /// This animation lets this route coordinate with the entrance and exit transition of the
    /// route pushed on top of this route.
    fn secondary_animation(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        Some(
            self.transition_route_data(app)
                .secondary_animation
                .as_animation(),
        )
    }

    /// Returns true if the transition has completed.
    ///
    /// It is equivalent to whether the completion registered with
    /// [`when_completed`](Self::when_completed) has run.
    ///
    /// This method only works when debug assertions are enabled. Otherwise it always returns
    /// false.
    fn debug_transition_completed(self: Handle<Self>, app: &App) -> bool {
        let mut disposed = false;
        if cfg!(debug_assertions) {
            disposed = self
                .transition_route_data(app)
                .transition_completer
                .is_completed();
        }
        disposed
    }

    /// Called to create the animation controller that will drive the transitions to this route
    /// from the previous one, and back to the previous route from this one.
    ///
    /// The returned controller will be disposed by `AnimationController::dispose` if
    /// [`will_dispose_animation_controller`](TransitionRouteData::will_dispose_animation_controller)
    /// is true.
    fn create_animation_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> Handle<AnimationController> {
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        let duration = TransitionRoute::transition_duration(self, app);
        let reverse_duration = TransitionRoute::reverse_transition_duration(self, app);
        let navigator = self
            .as_route()
            .navigator(app)
            .expect("an installed route has a navigator");
        AnimationController::create(
            app,
            None,
            Some(duration),
            Some(reverse_duration),
            0.0,
            1.0,
            AnimationBehavior::Normal,
            navigator,
        )
    }

    /// Called to create the animation that exposes the current progress of the transition
    /// controlled by the animation controller created by
    /// [`create_animation_controller`](Self::create_animation_controller).
    fn create_animation(self: Handle<Self>, app: &mut App) -> AnyAnimation<f64> {
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        let controller = self
            .transition_route_data(app)
            .controller
            .expect("createAnimationController ran first");
        controller.view()
    }

    /// Creates the simulation that drives the transition animation for this route.
    ///
    /// By default, this method returns `None`, indicating that the route doesn't use
    /// simulations, but initiates the transition by calling either `AnimationController::forward`
    /// or `AnimationController::reverse` with
    /// [`transition_duration`](Self::transition_duration) and the controller's curve.
    ///
    /// Subclasses can override this method to return a simulation. In that case the
    /// [`controller`](Self::controller) will instead use the provided simulation to animate the
    /// transition using `AnimationController::animate_with` or
    /// `AnimationController::animate_back_with`, and the simulation's `x` is forwarded to the
    /// value of [`animation`](Self::animation). The controller's curve and
    /// [`transition_duration`](Self::transition_duration) are ignored.
    ///
    /// This method is invoked each time the navigator pushes or pops this route. The `forward`
    /// parameter indicates the direction of the transition: true when the route is pushed, and
    /// false when it is popped.
    fn create_simulation(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
    ) -> Option<Box<dyn Simulation>> {
        let _ = forward;
        debug_assert!(
            TransitionRoute::transition_duration(self, app) >= Duration::ZERO,
            "the duration must be positive for a non-simulation animation"
        );
        None
    }

    /// Dart's `_createSimulationAndVerify`.
    fn create_simulation_and_verify(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
    ) -> Option<Box<dyn Simulation>> {
        let simulation = TransitionRoute::create_simulation(self, app, forward);
        debug_assert!(
            TransitionRoute::transition_duration(self, app) >= Duration::ZERO,
            "the duration must be positive for an animation that doesn't use a simulation; \
             either set transition_duration or set create_simulation"
        );
        simulation
    }

    /// Dart's `_handleStatusChanged`.
    fn handle_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        match status {
            AnimationStatus::Completed => {
                let entries = Route::overlay_entries(self, app);
                if let Some(first) = entries.first().copied() {
                    let opaque = TransitionRoute::opaque(self, app);
                    first.set_opaque(app, opaque);
                }
            }
            AnimationStatus::Forward | AnimationStatus::Reverse => {
                let entries = Route::overlay_entries(self, app);
                if let Some(first) = entries.first().copied() {
                    first.set_opaque(app, false);
                }
            }
            AnimationStatus::Dismissed => {
                // We might still be an active route if a subclass is controlling the
                // transition and hits the dismissed status. For example, the iOS back gesture
                // drives this animation to the dismissed status before removing the route and
                // disposing it.
                if !self.as_route().is_active(app) {
                    let navigator = self
                        .as_route()
                        .navigator(app)
                        .expect("an active route has a navigator");
                    navigator.finalize_route(app, self.as_route());
                    self.transition_route_data_mut(app).pop_finalized = true;
                }
            }
        }
    }

    /// Dart's `TransitionRoute.install`.
    fn install(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot install a route after disposing it"
        );
        let controller = TransitionRoute::create_animation_controller(self, app);
        self.transition_route_data_mut(app).controller = Some(controller);
        let animation = TransitionRoute::create_animation(self, app);
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::handle_status_changed),
        );
        self.transition_route_data_mut(app).animation = Some(animation);
        OverlayRoute::install(self, app);
        let entries = Route::overlay_entries(self, app);
        if animation.is_completed(app)
            && let Some(first) = entries.first().copied()
        {
            let opaque = TransitionRoute::opaque(self, app);
            first.set_opaque(app, opaque);
        }
    }

    /// Dart's `TransitionRoute.didPush`.
    fn did_push(self: Handle<Self>, app: &mut App) -> Handle<TickerFuture> {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didPush called before install or after dispose"
        );
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        RouteBase::did_push(self, app);
        let simulation = TransitionRoute::create_simulation_and_verify(self, app, true);
        let controller = self
            .transition_route_data(app)
            .controller
            .expect("installed");
        match simulation {
            None => controller.forward(app, None),
            Some(simulation) => controller.animate_with(app, simulation),
        }
    }

    /// Dart's `TransitionRoute.didAdd`.
    fn did_add(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didAdd called before install or after dispose"
        );
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        RouteBase::did_add(self, app);
        let controller = self
            .transition_route_data(app)
            .controller
            .expect("installed");
        let upper_bound = app.get(controller).upper_bound;
        controller.set_value(app, upper_bound);
    }

    /// Dart's `TransitionRoute.didReplace`.
    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<AnyRoute>) {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didReplace called before install or after dispose"
        );
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        if let Some(old) = old_route.and_then(|route| route.as_transition_route(app)) {
            let value = old.controller_value(app);
            let controller = self
                .transition_route_data(app)
                .controller
                .expect("installed");
            controller.set_value(app, value);
        }
        RouteBase::did_replace(self, app, old_route);
    }

    /// Dart's `TransitionRoute.didPop`.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didPop called before install or after dispose"
        );
        debug_assert!(
            !self
                .transition_route_data(app)
                .transition_completer
                .is_completed(),
            "cannot reuse a route after disposing it"
        );
        self.transition_route_data_mut(app).result = result.clone();
        let simulation = TransitionRoute::create_simulation_and_verify(self, app, false);
        let controller = self
            .transition_route_data(app)
            .controller
            .expect("installed");
        match simulation {
            None => {
                controller.reverse(app, None);
            }
            Some(simulation) => {
                controller.animate_back_with(app, simulation);
            }
        }
        OverlayRoute::did_pop(self, app, result)
    }

    /// Dart's `TransitionRoute.didPopNext`.
    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didPopNext called before install or after dispose"
        );
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        TransitionRoute::update_secondary_animation(self, app, Some(next_route));
        RouteBase::did_pop_next(self, app, next_route);
    }

    /// Dart's `TransitionRoute.didChangeNext`.
    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        debug_assert!(
            self.transition_route_data(app).controller.is_some(),
            "didChangeNext called before install or after dispose"
        );
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        TransitionRoute::update_secondary_animation(self, app, next_route);
        RouteBase::did_change_next(self, app, next_route);
    }

    /// Dart's `_updateSecondaryAnimation`.
    fn update_secondary_animation(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        // There is an existing train hopping in progress. Unfortunately, we cannot dispose the
        // current train hopping animation until we replace it with a new animation.
        let previous_train_hopping_listener_remover = self
            .transition_route_data_mut(app)
            .train_hopping_listener_remover
            .take();

        let next = next_route.and_then(|route| route.as_transition_route(app));
        let coordinates = match next {
            Some(next) => {
                TransitionRoute::can_transition_to(self, app, next)
                    && next.can_transition_from(app, TransitionRoute::as_transition_route(self))
            }
            None => false,
        };

        if let (Some(next), true) = (next, coordinates) {
            let secondary = self.transition_route_data(app).secondary_animation;
            match secondary.parent(app) {
                Some(current) => {
                    let current_train = match current.downcast::<TrainHoppingAnimation>(app) {
                        Some(hopping) => app.get(hopping).current_train().expect("a running train"),
                        None => current,
                    };
                    let next_train = next.animation(app).expect("an installed route");
                    if current_train.value(app) == next_train.value(app)
                        || !next_train.is_animating(app)
                    {
                        TransitionRoute::set_secondary_animation(
                            self,
                            app,
                            Some(next_train),
                            Some(next),
                        );
                    } else {
                        // Two trains animate at different values. We have to do train hopping.
                        // There are three possibilities of train hopping:
                        //  1. We hop on the next train when two trains meet in the middle using
                        //     `TrainHoppingAnimation`.
                        //  2. There is no chance to hop on the next train because two trains
                        //     never cross each other. We have to directly set the animation to
                        //     the next train once the next train stops animating.
                        //  3. A new `update_secondary_animation` is called before train hopping
                        //     finishes. We leave a listener remover for the next call to
                        //     properly clean up the existing train hopping.
                        let slot: Rc<std::cell::Cell<Option<Handle<TrainHoppingAnimation>>>> =
                            Rc::new(std::cell::Cell::new(None));
                        let jump_on_animation_end =
                            AnimationStatusListener::new(move |status: AnimationStatus, app| {
                                if !status.is_animating() {
                                    // The next train has stopped animating without train
                                    // hopping. Directly sets the secondary animation and
                                    // disposes the `TrainHoppingAnimation`.
                                    TransitionRoute::set_secondary_animation(
                                        self,
                                        app,
                                        Some(next_train),
                                        Some(next),
                                    );
                                    if let Some(remover) = self
                                        .transition_route_data_mut(app)
                                        .train_hopping_listener_remover
                                        .take()
                                    {
                                        remover(app);
                                    }
                                }
                            });
                        let remover = {
                            let listener = jump_on_animation_end.clone();
                            let slot = Rc::clone(&slot);
                            Rc::new(move |app: &mut App| {
                                next_train.remove_status_listener(app, &listener);
                                if let Some(animation) = slot.get() {
                                    animation.dispose(app);
                                }
                            })
                        };
                        self.transition_route_data_mut(app)
                            .train_hopping_listener_remover = Some(remover);
                        next_train.add_status_listener(app, jump_on_animation_end);
                        let on_switched_train = {
                            let slot = Rc::clone(&slot);
                            Listener::new(move |app| {
                                let new_animation = slot.get().expect("the hopping animation");
                                debug_assert!(
                                    self.transition_route_data(app)
                                        .secondary_animation
                                        .parent(app)
                                        == Some(new_animation.as_animation())
                                );
                                let current = app.get(new_animation).current_train();
                                debug_assert!(current == next.animation(app));
                                // We can hop on the next train, so we don't need to listen to
                                // whether the next train has stopped.
                                TransitionRoute::set_secondary_animation(
                                    self,
                                    app,
                                    current,
                                    Some(next),
                                );
                                if let Some(remover) = self
                                    .transition_route_data_mut(app)
                                    .train_hopping_listener_remover
                                    .take()
                                {
                                    remover(app);
                                }
                            })
                        };
                        let new_animation = TrainHoppingAnimation::create(
                            app,
                            current_train,
                            Some(next_train),
                            Some(on_switched_train),
                        );
                        slot.set(Some(new_animation));
                        self.set_secondary_animation(
                            app,
                            Some(new_animation.as_animation()),
                            Some(next),
                        );
                    }
                }
                None => {
                    let animation = next.animation(app);
                    TransitionRoute::set_secondary_animation(self, app, animation, Some(next));
                }
            }
        } else {
            let dismissed = k_always_dismissed_animation(app);
            TransitionRoute::set_secondary_animation(self, app, Some(dismissed), None);
        }
        // Finally, we dispose any previous train hopping animation because it has been
        // successfully updated at this point.
        if let Some(remover) = previous_train_hopping_listener_remover {
            remover(app);
        }
    }

    /// Dart's `_setSecondaryAnimation`; `disposed` is the route whose `completed` releases the
    /// reference to `animation`.
    fn set_secondary_animation(
        self: Handle<Self>,
        app: &mut App,
        animation: Option<AnyAnimation<f64>>,
        disposed: Option<AnyTransitionRoute>,
    ) {
        let secondary = self.transition_route_data(app).secondary_animation;
        secondary.set_parent(app, animation);
        // Releases the reference to the next route's animation when that route is disposed.
        if let Some(disposed) = disposed {
            disposed.when_completed(
                app,
                Rc::new(move |app, _result| {
                    if secondary.parent(app) == animation {
                        let dismissed = k_always_dismissed_animation(app);
                        secondary.set_parent(app, Some(dismissed));
                        if let Some(animation) = animation
                            && let Some(hopping) = animation.downcast::<TrainHoppingAnimation>(app)
                        {
                            hopping.dispose(app);
                        }
                    }
                }),
            );
        }
    }

    /// Returns true if this route supports a transition animation that runs when `next_route`
    /// is pushed on top of it or when `next_route` is popped off of it.
    ///
    /// Subclasses can override this method to restrict the set of routes they need to
    /// coordinate transitions with.
    ///
    /// Returns true by default.
    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        let _ = (app, next_route);
        true
    }

    /// Returns true if `previous_route` should animate when this route is pushed on top of it
    /// or when this route is popped off of it.
    ///
    /// Subclasses can override this method to restrict the set of routes they need to
    /// coordinate transitions with.
    ///
    /// Returns true by default.
    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        let _ = (app, previous_route);
        true
    }

    // ---- PredictiveBackRoute ----

    /// Dart's `TransitionRoute.handleStartBackGesture`.
    fn handle_start_back_gesture(self: Handle<Self>, app: &mut App, progress: f64) {
        debug_assert!(self.as_route().is_current(app));
        if let Some(controller) = self.transition_route_data(app).controller {
            controller.set_value(app, progress);
        }
        if let Some(navigator) = self.as_route().navigator(app) {
            navigator.did_start_user_gesture(app);
        }
    }

    /// Dart's `TransitionRoute.handleUpdateBackGestureProgress`.
    fn handle_update_back_gesture_progress(self: Handle<Self>, app: &mut App, progress: f64) {
        // If some other navigation happened during this gesture, don't mess with the transition
        // anymore.
        if !self.as_route().is_current(app) {
            return;
        }
        if let Some(controller) = self.transition_route_data(app).controller {
            controller.set_value(app, progress);
        }
    }

    /// Dart's `TransitionRoute.handleCancelBackGesture`.
    fn handle_cancel_back_gesture(self: Handle<Self>, app: &mut App) {
        TransitionRoute::handle_drag_end(self, app, true);
    }

    /// Dart's `TransitionRoute.handleCommitBackGesture`.
    fn handle_commit_back_gesture(self: Handle<Self>, app: &mut App) {
        TransitionRoute::handle_drag_end(self, app, false);
    }

    /// Dart's `_handleDragEnd`.
    fn handle_drag_end(self: Handle<Self>, app: &mut App, animate_forward: bool) {
        if self.as_route().is_current(app) {
            let controller = self
                .transition_route_data(app)
                .controller
                .expect("installed");
            if animate_forward {
                // Typically, `handle_update_back_gesture_progress` will have already completed
                // the animation. If not, animate to completion.
                if !controller.status(app).is_completed() {
                    controller.forward(app, None);
                }
            } else {
                // This route is destined to pop at this point. Reuse the navigator's pop.
                if let Some(navigator) = self.as_route().navigator(app) {
                    navigator.pop(app, None);
                }
                // The popping may have finished inline if already at the target destination.
                if let Some(controller) = self.transition_route_data(app).controller
                    && controller.is_animating(app)
                {
                    let upper_bound = app.get(controller).upper_bound;
                    controller.reverse(app, Some(upper_bound));
                }
            }
        }

        let controller = self.transition_route_data(app).controller;
        let animating = match controller {
            Some(controller) => controller.is_animating(app),
            None => false,
        };
        if animating {
            // Keep `user_gesture_in_progress` true, since the Android back-gesture page
            // transition depends on it.
            let controller = controller.expect("an animating controller");
            let slot: Rc<std::cell::RefCell<Option<AnimationStatusListener>>> =
                Rc::new(std::cell::RefCell::new(None));
            let listener = {
                let slot = Rc::clone(&slot);
                AnimationStatusListener::new(move |_status, app| {
                    if let Some(navigator) = self.as_route().navigator(app) {
                        navigator.did_stop_user_gesture(app);
                    }
                    let registered = slot.borrow().clone();
                    if let Some(registered) = registered {
                        controller.remove_status_listener(app, &registered);
                    }
                })
            };
            *slot.borrow_mut() = Some(listener.clone());
            controller.add_status_listener(app, listener);
        } else if let Some(navigator) = self.as_route().navigator(app) {
            navigator.did_stop_user_gesture(app);
        }
    }

    /// Dart's `TransitionRoute.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            !self
                .transition_route_data(app)
                .transition_completer
                .is_completed(),
            "cannot dispose a route twice"
        );
        if let Some(animation) = self.transition_route_data(app).animation {
            animation.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self, Self::handle_status_changed),
            );
        }
        if self
            .transition_route_data(app)
            .will_dispose_animation_controller
            && let Some(controller) = self.transition_route_data(app).controller
        {
            controller.dispose(app);
        }
        let result = self.transition_route_data(app).result.clone();
        let callbacks = self
            .transition_route_data_mut(app)
            .transition_completer
            .complete(result.clone());
        for callback in callbacks {
            let result = result.clone();
            app.schedule_microtask(Listener::new(move |app| callback(app, result.clone())));
        }
        OverlayRoute::dispose(self, app);
    }

    /// A short description of this route useful for debugging.
    fn debug_label(self: Handle<Self>) -> String {
        std::any::type_name::<Self>().to_string()
    }
}

/// The vtable of an erased [`AnyTransitionRoute`]: one `&'static` table per route type, built
/// by [`TransitionRouteVTable::of`].
struct TransitionRouteVTable {
    type_name: fn() -> &'static str,
    route: fn(HandleId) -> AnyRoute,
    animation: fn(&App, HandleId) -> Option<AnyAnimation<f64>>,
    controller_value: fn(&App, HandleId) -> f64,
    opaque: fn(&App, HandleId) -> bool,
    allow_snapshotting: fn(&App, HandleId) -> bool,
    transition_duration: fn(&App, HandleId) -> Duration,
    reverse_transition_duration: fn(&App, HandleId) -> Duration,
    can_transition_to: fn(&App, HandleId, AnyTransitionRoute) -> bool,
    can_transition_from: fn(&App, HandleId, AnyTransitionRoute) -> bool,
    when_completed: fn(&mut App, HandleId, RouteResultCallback),
    pop_gesture_enabled: fn(&mut App, HandleId) -> bool,
    handle_start_back_gesture: fn(&mut App, HandleId, f64),
    handle_update_back_gesture_progress: fn(&mut App, HandleId, f64),
    handle_commit_back_gesture: fn(&mut App, HandleId),
    handle_cancel_back_gesture: fn(&mut App, HandleId),
}

impl TransitionRouteVTable {
    /// The table for one transition route type.
    const fn of<R: TransitionRoute>() -> TransitionRouteVTable {
        TransitionRouteVTable {
            type_name: std::any::type_name::<R>,
            route: |id| Route::as_route(resolve::<R>(id)),
            animation: |app, id| R::animation(resolve(id), app),
            controller_value: |app, id| {
                let controller = R::transition_route_data(resolve::<R>(id), app)
                    .controller
                    .expect("an installed route");
                controller.value(app)
            },
            opaque: |app, id| R::opaque(resolve(id), app),
            allow_snapshotting: |app, id| R::allow_snapshotting(resolve(id), app),
            transition_duration: |app, id| R::transition_duration(resolve(id), app),
            reverse_transition_duration: |app, id| R::reverse_transition_duration(resolve(id), app),
            can_transition_to: |app, id, next| R::can_transition_to(resolve(id), app, next),
            can_transition_from: |app, id, prev| R::can_transition_from(resolve(id), app, prev),
            when_completed: |app, id, callback| R::when_completed(resolve(id), app, callback),
            pop_gesture_enabled: |app, id| {
                PredictiveBackRoute::pop_gesture_enabled(resolve::<R>(id), app)
            },
            handle_start_back_gesture: |app, id, progress| {
                PredictiveBackRoute::handle_start_back_gesture(resolve::<R>(id), app, progress)
            },
            handle_update_back_gesture_progress: |app, id, progress| {
                PredictiveBackRoute::handle_update_back_gesture_progress(
                    resolve::<R>(id),
                    app,
                    progress,
                )
            },
            handle_commit_back_gesture: |app, id| {
                PredictiveBackRoute::handle_commit_back_gesture(resolve::<R>(id), app)
            },
            handle_cancel_back_gesture: |app, id| {
                PredictiveBackRoute::handle_cancel_back_gesture(resolve::<R>(id), app)
            },
        }
    }
}

/// Erased [`TransitionRoute`]: what Dart's `route is TransitionRoute<dynamic>` produces.
#[derive(Clone, Copy)]
pub struct AnyTransitionRoute {
    id: HandleId,
    vtable: &'static TransitionRouteVTable,
}

impl PartialEq for AnyTransitionRoute {
    fn eq(&self, other: &AnyTransitionRoute) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyTransitionRoute {}

impl Debug for AnyTransitionRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyTransitionRoute {
    /// This transition route as the erased [`AnyRoute`].
    pub fn as_route(self) -> AnyRoute {
        (self.vtable.route)(self.id)
    }

    /// See [`TransitionRoute::animation`].
    pub fn animation(self, app: &App) -> Option<AnyAnimation<f64>> {
        (self.vtable.animation)(app, self.id)
    }

    /// The value of Dart's `_controller`, which `didReplace` copies.
    pub fn controller_value(self, app: &App) -> f64 {
        (self.vtable.controller_value)(app, self.id)
    }

    /// See [`TransitionRoute::opaque`].
    pub fn opaque(self, app: &App) -> bool {
        (self.vtable.opaque)(app, self.id)
    }

    /// See [`TransitionRoute::allow_snapshotting`].
    pub fn allow_snapshotting(self, app: &App) -> bool {
        (self.vtable.allow_snapshotting)(app, self.id)
    }

    /// See [`TransitionRoute::transition_duration`].
    pub fn transition_duration(self, app: &App) -> Duration {
        (self.vtable.transition_duration)(app, self.id)
    }

    /// See [`TransitionRoute::reverse_transition_duration`].
    pub fn reverse_transition_duration(self, app: &App) -> Duration {
        (self.vtable.reverse_transition_duration)(app, self.id)
    }

    /// See [`TransitionRoute::can_transition_to`].
    pub fn can_transition_to(self, app: &App, next_route: AnyTransitionRoute) -> bool {
        (self.vtable.can_transition_to)(app, self.id, next_route)
    }

    /// See [`TransitionRoute::can_transition_from`].
    pub fn can_transition_from(self, app: &App, previous_route: AnyTransitionRoute) -> bool {
        (self.vtable.can_transition_from)(app, self.id, previous_route)
    }

    /// See [`TransitionRoute::when_completed`].
    pub fn when_completed(self, app: &mut App, callback: RouteResultCallback) {
        (self.vtable.when_completed)(app, self.id, callback)
    }

    /// See [`PredictiveBackRoute::pop_gesture_enabled`].
    pub fn pop_gesture_enabled(self, app: &mut App) -> bool {
        (self.vtable.pop_gesture_enabled)(app, self.id)
    }

    /// See [`PredictiveBackRoute::handle_start_back_gesture`].
    pub fn handle_start_back_gesture(self, app: &mut App, progress: f64) {
        (self.vtable.handle_start_back_gesture)(app, self.id, progress)
    }

    /// See [`PredictiveBackRoute::handle_update_back_gesture_progress`].
    pub fn handle_update_back_gesture_progress(self, app: &mut App, progress: f64) {
        (self.vtable.handle_update_back_gesture_progress)(app, self.id, progress)
    }

    /// See [`PredictiveBackRoute::handle_commit_back_gesture`].
    pub fn handle_commit_back_gesture(self, app: &mut App) {
        (self.vtable.handle_commit_back_gesture)(app, self.id)
    }

    /// See [`PredictiveBackRoute::handle_cancel_back_gesture`].
    pub fn handle_cancel_back_gesture(self, app: &mut App) {
        (self.vtable.handle_cancel_back_gesture)(app, self.id)
    }
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

// ---------------------------------------------------------------------------------------------
// LocalHistoryEntry / LocalHistoryRoute

/// An entry in the history of a [`LocalHistoryRoute`].
///
/// A [`LocalHistoryEntry`] represents a "mini" navigation state within a route. It allows
/// widgets or UI components to handle the back button or pop operations locally without
/// affecting the main navigator stack.
///
/// It is typically used for widgets such as dialogs, bottom sheets, or inline expandable panels
/// that can be dismissed independently of the surrounding route.
///
/// When a local history entry is removed (e.g. via the back button), the
/// [`on_remove`](Self::on_remove) callback is called first. Only after all local history
/// entries have been removed will the route itself be popped.
///
/// See also:
///
///  * [`LocalHistoryRoute`], which manages a stack of local history entries.
///  * [`LocalHistoryRoute::add_local_history_entry`], which adds an entry to a route.
pub struct LocalHistoryEntry {
    /// Called when this entry is removed from the history of its associated
    /// [`LocalHistoryRoute`].
    pub on_remove: Option<Listener>,
    owner: Option<AnyRoute>,
    /// Whether an app bar in the route this entry belongs to should automatically add a back
    /// button or close button.
    ///
    /// Defaults to true.
    pub implies_app_bar_dismissal: bool,
}

impl LocalHistoryEntry {
    /// Creates an entry in the history of a [`LocalHistoryRoute`]; Dart's optional arguments
    /// are the setters.
    ///
    /// [`implies_app_bar_dismissal`](Self::implies_app_bar_dismissal) defaults to true.
    pub fn new(app: &mut App) -> Handle<LocalHistoryEntry> {
        app.create(LocalHistoryEntry {
            on_remove: None,
            owner: None,
            implies_app_bar_dismissal: true,
        })
    }

    /// Dart `LocalHistoryEntry(onRemove:)`.
    pub fn on_remove(self: Handle<Self>, app: &mut App, on_remove: Listener) -> Handle<Self> {
        app.get_mut(self).on_remove = Some(on_remove);
        self
    }

    /// Dart `LocalHistoryEntry(impliesAppBarDismissal:)`.
    pub fn implies_app_bar_dismissal(
        self: Handle<Self>,
        app: &mut App,
        implies_app_bar_dismissal: bool,
    ) -> Handle<Self> {
        app.get_mut(self).implies_app_bar_dismissal = implies_app_bar_dismissal;
        self
    }

    /// Remove this entry from the history of its associated [`LocalHistoryRoute`].
    pub fn remove(self: Handle<Self>, app: &mut App) {
        if let Some(owner) = app.get(self).owner {
            let modal = owner
                .as_modal_route(app)
                .expect("a local history entry belongs to a modal route");
            modal.remove_local_history_entry(app, self);
        }
        debug_assert!(app.get(self).owner.is_none());
    }

    fn notify_removed(self: Handle<Self>, app: &mut App) {
        if let Some(on_remove) = app.get(self).on_remove.clone() {
            on_remove.call(app);
        }
    }
}

impl Debug for LocalHistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalHistoryEntry")
    }
}

/// The fields Dart's `LocalHistoryRoute` mixin declares; every route that mixes it in carries
/// this bag under the field `local_history`.
pub struct LocalHistoryRouteData {
    local_history: Option<Vec<Handle<LocalHistoryEntry>>>,
    entries_implies_app_bar_dismissal: i64,
}

impl LocalHistoryRouteData {
    /// The bag of a route with no local history.
    pub fn new() -> LocalHistoryRouteData {
        LocalHistoryRouteData {
            local_history: None,
            entries_implies_app_bar_dismissal: 0,
        }
    }
}

impl Default for LocalHistoryRouteData {
    fn default() -> LocalHistoryRouteData {
        LocalHistoryRouteData::new()
    }
}

/// The accessors [`LocalHistoryRoute`] asks for, for a struct whose bag is the field
/// `local_history`.
#[macro_export]
macro_rules! local_history_route_accessors {
    () => {
        fn local_history_route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::LocalHistoryRouteData {
            &app.get(self).local_history
        }

        fn local_history_route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::LocalHistoryRouteData {
            &mut app.get_mut(self).local_history
        }
    };
}

/// A mixin used by routes to handle back navigations internally by popping a list.
///
/// When a [`Navigator`] is instructed to pop, the current route is given an opportunity to
/// handle the pop internally. A [`LocalHistoryRoute`] handles the pop internally if its list of
/// local history entries is non-empty. Rather than being removed as the current route, the most
/// recent [`LocalHistoryEntry`] is removed from the list and its
/// [`LocalHistoryEntry::on_remove`] is called.
pub trait LocalHistoryRoute: TransitionRoute {
    /// Dart's `LocalHistoryRoute` fields, held under the field `local_history`
    /// ([`local_history_route_accessors!`](crate::local_history_route_accessors)).
    fn local_history_route_data(self: Handle<Self>, app: &App) -> &LocalHistoryRouteData;

    /// See [`local_history_route_data`](Self::local_history_route_data).
    fn local_history_route_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut LocalHistoryRouteData;

    /// Adds a local history entry to this route.
    ///
    /// When asked to pop, if this route has any local history entries, this route will handle
    /// the pop internally by removing the most recently added local history entry.
    ///
    /// The given local history entry must not already be part of another local history route.
    fn add_local_history_entry(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<LocalHistoryEntry>,
    ) {
        debug_assert!(app.get(entry).owner.is_none());
        app.get_mut(entry).owner = Some(self.as_route());
        let data = self.local_history_route_data_mut(app);
        let history = data.local_history.get_or_insert_with(Vec::new);
        let was_empty = history.is_empty();
        history.push(entry);
        let mut internal_state_changed = false;
        if app.get(entry).implies_app_bar_dismissal {
            let data = self.local_history_route_data_mut(app);
            internal_state_changed = data.entries_implies_app_bar_dismissal == 0;
            data.entries_implies_app_bar_dismissal += 1;
        }
        if was_empty || internal_state_changed {
            Route::changed_internal_state(self, app);
        }
    }

    /// Removes a local history entry from this route.
    ///
    /// The entry's [`LocalHistoryEntry::on_remove`] callback, if any, is called synchronously.
    fn remove_local_history_entry(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<LocalHistoryEntry>,
    ) {
        debug_assert!(app.get(entry).owner == Some(self.as_route()));
        let data = self.local_history_route_data_mut(app);
        let history = data.local_history.as_mut().expect("a local history");
        debug_assert!(history.contains(&entry));
        let position = history.iter().position(|held| *held == entry);
        let mut internal_state_changed = false;
        if let Some(position) = position {
            history.remove(position);
            if app.get(entry).implies_app_bar_dismissal {
                let data = self.local_history_route_data_mut(app);
                data.entries_implies_app_bar_dismissal -= 1;
                internal_state_changed = data.entries_implies_app_bar_dismissal == 0;
            }
        }
        app.get_mut(entry).owner = None;
        entry.notify_removed(app);
        let data = self.local_history_route_data(app);
        if data.local_history.as_ref().is_some_and(Vec::is_empty) || internal_state_changed {
            debug_assert_eq!(
                self.local_history_route_data(app)
                    .entries_implies_app_bar_dismissal,
                0
            );
            if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::PersistentCallbacks {
                // The local history might be removed as a result of disposing inactive elements
                // during `finalize_tree`. The state is locked at this moment, and we can only
                // notify that state has changed in the next frame.
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _duration| {
                        if self.as_route().is_active(app) {
                            Route::changed_internal_state(self, app);
                        }
                    }),
                );
            } else {
                Route::changed_internal_state(self, app);
            }
        }
    }

    /// Dart's `LocalHistoryRoute.willPop`.
    fn will_pop(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        if Route::will_handle_pop_internally(self, app) {
            return RoutePopDisposition::Pop;
        }
        RouteBase::will_pop(self, app)
    }

    /// Dart's `LocalHistoryRoute.popDisposition`.
    fn pop_disposition(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        if Route::will_handle_pop_internally(self, app) {
            return RoutePopDisposition::Pop;
        }
        RouteBase::pop_disposition(self, app)
    }

    /// Dart's `LocalHistoryRoute.didPop`.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        let last = self
            .local_history_route_data_mut(app)
            .local_history
            .as_mut()
            .and_then(Vec::pop);
        if let Some(entry) = last {
            debug_assert!(app.get(entry).owner == Some(self.as_route()));
            app.get_mut(entry).owner = None;
            entry.notify_removed(app);
            let mut internal_state_changed = false;
            if app.get(entry).implies_app_bar_dismissal {
                let data = self.local_history_route_data_mut(app);
                data.entries_implies_app_bar_dismissal -= 1;
                internal_state_changed = data.entries_implies_app_bar_dismissal == 0;
            }
            let empty = self
                .local_history_route_data(app)
                .local_history
                .as_ref()
                .is_some_and(Vec::is_empty);
            if empty || internal_state_changed {
                Route::changed_internal_state(self, app);
            }
            return false;
        }
        TransitionRoute::did_pop(self, app, result)
    }

    /// Dart's `LocalHistoryRoute.willHandlePopInternally`.
    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        self.local_history_route_data(app)
            .local_history
            .as_ref()
            .is_some_and(|history| !history.is_empty())
    }
}

// ---------------------------------------------------------------------------------------------
// _ModalScope

/// Dart's `_DismissModalAction`.
struct DismissModalAction {
    action: ActionData,
    context: BuildContext,
}

impl DismissModalAction {
    fn new(app: &mut App, context: BuildContext) -> Handle<DismissModalAction> {
        app.create(DismissModalAction {
            action: ActionData::new(),
            context,
        })
    }
}

impl Action for DismissModalAction {
    type Intent = DismissIntent;

    crate::action_accessors!();

    fn is_enabled(self: Handle<Self>, app: &mut App, _intent: &DismissIntent) -> bool {
        let context = app.get(self).context;
        let route = AnyModalRoute::of(app, context).expect("a dismiss action inside a modal route");
        route.barrier_dismissible(app)
    }

    fn invoke(self: Handle<Self>, app: &mut App, _intent: &DismissIntent) -> Option<Rc<dyn Any>> {
        let context = app.get(self).context;
        let popped = Navigator::of(app, context, false).maybe_pop(app, None);
        Some(Rc::new(popped))
    }
}

impl DismissAction for DismissModalAction {}

/// Dart's `_ModalRouteAspect`: the aspects of a [`ModalRoute`] a dependent can depend on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ModalRouteAspect {
    /// Specifies the aspect corresponding to [`AnyRoute::is_current`].
    IsCurrent,
    /// Specifies the aspect corresponding to [`ModalRoute::can_pop`].
    CanPop,
    /// Specifies the aspect corresponding to [`AnyRoute::settings`].
    Settings,
    /// Specifies the aspect corresponding to [`AnyRoute::is_active`].
    IsActive,
    /// Specifies the aspect corresponding to [`AnyRoute::is_first`].
    IsFirst,
    /// Specifies the aspect corresponding to [`TransitionRoute::opaque`].
    Opaque,
    /// Specifies the aspect corresponding to [`Route::pop_disposition`].
    PopDisposition,
}

/// Dart's `_ModalScopeStatus`.
#[derive(Debug)]
struct ModalScopeStatus {
    is_current: bool,
    can_pop: bool,
    implies_app_bar_dismissal: bool,
    opaque: bool,
    route: AnyModalRoute,
    child: WidgetRef,
}

impl InheritedWidget for ModalScopeStatus {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &ModalScopeStatus) -> bool {
        self.is_current != old.is_current
            || self.can_pop != old.can_pop
            || self.implies_app_bar_dismissal != old.implies_app_bar_dismissal
            || self.route != old.route
            || self.opaque != old.opaque
    }
}

impl ModalScopeStatus {
    /// The erased widget: an [`InheritedModel`] has two `IntoWidget` impls, so the kind is
    /// named here.
    fn into_widget(self) -> WidgetRef {
        IntoWidget::<crate::framework::InheritedModelKind>::into_widget(self)
    }
}

impl InheritedModel for ModalScopeStatus {
    type Aspect = ModalRouteAspect;

    fn update_should_notify_dependent(
        &self,
        old_widget: &ModalScopeStatus,
        dependencies: &std::collections::HashSet<ModalRouteAspect>,
    ) -> bool {
        // The aspects that read the route need the App; the widget cannot. The route-derived
        // aspects are compared through the aspect's own recorded value instead.
        dependencies.iter().any(|dependency| match dependency {
            ModalRouteAspect::IsCurrent => self.is_current != old_widget.is_current,
            ModalRouteAspect::CanPop => self.can_pop != old_widget.can_pop,
            ModalRouteAspect::Settings
            | ModalRouteAspect::IsActive
            | ModalRouteAspect::IsFirst
            | ModalRouteAspect::PopDisposition => self.route != old_widget.route,
            ModalRouteAspect::Opaque => self.opaque != old_widget.opaque,
        })
    }
}

/// Dart's `_ModalScope`.
#[derive(Debug)]
struct ModalScope {
    key: Option<KeyRef>,
    route: AnyModalRoute,
}

impl StatefulWidget for ModalScope {
    type State = ModalScopeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ModalScopeState {
        ModalScopeState {
            state: StateData::new(),
            page: None,
            listenable: None,
            focus_scope_node: None,
            primary_scroll_controller: None,
            no_gesture_in_progress: None,
        }
    }
}

/// Dart's `_ModalScopeState`.
struct ModalScopeState {
    state: StateData<ModalScope>,
    // We cache the result of calling the route's `build_page`, and clear the cache whenever the
    // dependencies change. This implements the contract described in the documentation for
    // `build_page`, namely that it gets called once, unless something like a `ModalRoute::of`
    // dependency triggers an update.
    page: Option<WidgetRef>,
    // This is the combination of the two animations for the route.
    listenable: Option<Rc<MergingListenable>>,
    focus_scope_node: Option<Handle<FocusScopeNode>>,
    primary_scroll_controller: Option<Handle<ScrollController>>,
    no_gesture_in_progress: Option<Handle<reveal_foundation::ValueNotifier<bool>>>,
}

impl ModalScopeState {
    /// The node this scope uses for its root `FocusScope` widget.
    fn focus_scope_node(self: Handle<Self>, app: &App) -> Handle<FocusScopeNode> {
        app.get(self).focus_scope_node.expect("init_state has run")
    }

    fn route(self: Handle<Self>, app: &App) -> AnyModalRoute {
        self.widget(app).route
    }

    fn update_focus_scope_node(self: Handle<Self>, app: &mut App) {
        let route = self.route(app);
        let navigator = route
            .as_route()
            .navigator(app)
            .expect("a mounted scope has a navigator");
        let traversal_edge_behavior = match route.traversal_edge_behavior(app) {
            Some(behavior) => behavior,
            None => navigator.widget(app).route_traversal_edge_behavior,
        };
        let directional_traversal_edge_behavior =
            match route.directional_traversal_edge_behavior(app) {
                Some(behavior) => behavior,
                None => {
                    navigator
                        .widget(app)
                        .route_directional_traversal_edge_behavior
                }
            };
        let node = self.focus_scope_node(app);
        node.set_traversal_edge_behavior(app, traversal_edge_behavior);
        node.set_directional_traversal_edge_behavior(app, directional_traversal_edge_behavior);
        if route.as_route().is_current(app) && self.should_request_focus(app) {
            let focus_node = navigator.focus_node(app);
            if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                scope.set_first_focus(app, node);
            }
        }
    }

    fn force_rebuild_page(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.page = None;
        });
    }

    fn should_ignore_focus_request(self: Handle<Self>, app: &mut App) -> bool {
        let route = self.route(app);
        let reversing = route
            .animation(app)
            .is_some_and(|animation| animation.status(app) == AnimationStatus::Reverse);
        reversing
            || route
                .as_route()
                .navigator(app)
                .is_some_and(|navigator| navigator.user_gesture_in_progress(app))
    }

    fn should_request_focus(self: Handle<Self>, app: &App) -> bool {
        self.route(app).as_route().request_focus(app)
    }

    /// Dart's `_routeSetState`: wraps any change to the route's `is_current`, `can_pop` or
    /// `offstage`.
    fn route_set_state(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut App)) {
        let route = self.route(app);
        if route.as_route().is_current(app)
            && !self.should_ignore_focus_request(app)
            && self.should_request_focus(app)
        {
            let navigator = route.as_route().navigator(app).expect("a current route");
            let focus_node = navigator.focus_node(app);
            let node = self.focus_scope_node(app);
            if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                scope.set_first_focus(app, node);
            }
        }
        change(app);
        self.set_state(app, |_state| {});
    }
}

impl State for ModalScopeState {
    type Widget = ModalScope;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let node = FocusScopeNode::new(app);
        node.as_node()
            .set_debug_label(app, Some("ModalScopeState Focus Scope".to_string()));
        app.get_mut(self).focus_scope_node = Some(node);
        app.get_mut(self).primary_scroll_controller =
            Some(ScrollController::new(app, 0.0, true, None, None, None));
        app.get_mut(self).no_gesture_in_progress =
            Some(app.create(reveal_foundation::ValueNotifier::new(false)));
        let route = self.route(app);
        let animations: Vec<Option<Rc<dyn reveal_foundation::Listenable>>> = vec![
            route
                .animation(app)
                .map(|animation| Rc::new(animation) as Rc<dyn reveal_foundation::Listenable>),
            route
                .secondary_animation(app)
                .map(|animation| Rc::new(animation) as Rc<dyn reveal_foundation::Listenable>),
        ];
        app.get_mut(self).listenable = Some(Rc::new(MergingListenable::new(animations)));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ModalScope) {
        debug_assert!(self.widget(app).route == old_widget.route);
        self.update_focus_scope_node(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).page = None;
        self.update_focus_scope_node(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let node = self.focus_scope_node(app);
        node.dispose(app);
        let controller = app.get(self).primary_scroll_controller.expect("init_state");
        ScrollController::dispose(controller.as_controller(), app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let route = self.route(app);
        let node = self.focus_scope_node(app);
        // Only the top-most route can participate in focus traversal.
        let is_current = route.as_route().is_current(app);
        node.set_skip_traversal(app, !is_current);

        let page = match app.get(self).page.clone() {
            Some(page) => page,
            None => {
                let page = RepaintBoundary::new()
                    .key(Rc::new(route.subtree_key(app)))
                    .child(Builder::new(move |app, context| {
                        let animation = route.animation(app).expect("an installed route");
                        let secondary = route.secondary_animation(app).expect("an installed route");
                        route.build_page(app, context, animation, secondary)
                    }))
                    .into_widget();
                app.get_mut(self).page = Some(page.clone());
                page
            }
        };

        let gesture_notifier: Rc<dyn reveal_foundation::Listenable> =
            match route.as_route().navigator(app) {
                Some(navigator) => Rc::new(navigator.user_gesture_in_progress_notifier(app)),
                None => Rc::new(app.get(self).no_gesture_in_progress.expect("init_state")),
            };
        let listenable = app.get(self).listenable.clone().expect("init_state");

        let transitions = ListenableBuilder::new(listenable, move |app, context, child| {
            let child = child.cloned().expect("the cached page");
            let animation = route.animation(app).expect("an installed route");
            let secondary = route.secondary_animation(app).expect("an installed route");
            // This additional builder is included because if the value of the user-gesture
            // notifier changes, it is only necessary to rebuild the `IgnorePointer` widget and
            // set the focus node's ability to focus.
            let ignoring = ListenableBuilder::new(Rc::clone(&gesture_notifier), {
                move |app: &mut App, _context: BuildContext, child: Option<&WidgetRef>| {
                    let ignore_events = self.should_ignore_focus_request(app);
                    node.set_can_request_focus(app, !ignore_events);
                    let mut widget = IgnorePointer::new().ignoring(ignore_events);
                    if let Some(child) = child {
                        widget = widget.child(child.clone());
                    }
                    widget.into_widget()
                }
            })
            .child(child)
            .into_widget();
            route.build_flexible_transitions(app, context, animation, secondary, ignoring)
        })
        .child(page)
        .into_widget();

        let scope_child = Builder::new(move |app, context| {
            let mut actions = HashMap::new();
            actions.insert(
                <DismissModalAction as Action>::intent_type(),
                Action::as_action(DismissModalAction::new(app, context)),
            );
            let controller = app.get(self).primary_scroll_controller.expect("init_state");
            Actions::new(
                actions,
                PrimaryScrollController::new(
                    controller.as_controller(),
                    FocusScope::with_external_focus_node(
                        RepaintBoundary::new().child(transitions.clone()),
                        node,
                    ),
                ),
            )
            .into_widget()
        });

        let status = ModalScopeStatus {
            route,
            // `route_set_state` is called if any of these update.
            is_current,
            can_pop: route.can_pop(app),
            opaque: route.opaque(app),
            implies_app_bar_dismissal: route.implies_app_bar_dismissal(app),
            child: Offstage::new()
                .offstage(route.offstage(app))
                .child(PageStorage::new(route.storage_bucket(app), scope_child))
                .into_widget(),
        };

        let restoration_scope_id = route.as_route().restoration_scope_id(app);
        AnimatedBuilder::new(
            Rc::new(restoration_scope_id),
            move |app, _context, child| {
                let child = child.cloned().expect("the modal scope status");
                let id = app.get(restoration_scope_id).value().clone();
                RestorationScope::new(id, child).into_widget()
            },
        )
        .child(status.into_widget())
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// ModalRoute

/// The fields Dart's `ModalRoute` declares; every modal route carries this bag under the field
/// `modal_route`.
pub struct ModalRouteData {
    /// The filter to add to the barrier.
    ///
    /// If given, this filter is applied to the modal barrier using `BackdropFilter`. This
    /// allows blur effects, for example.
    pub filter: Option<reveal_rendering::ImageFilterConfig>,
    /// Controls the transfer of focus beyond the first and the last items of a
    /// [`FocusScopeNode`].
    ///
    /// If `None`, [`Navigator::route_traversal_edge_behavior`] is used.
    pub traversal_edge_behavior: Option<TraversalEdgeBehavior>,
    /// Controls the directional transfer of focus beyond the first and the last items of a
    /// [`FocusScopeNode`].
    ///
    /// If `None`, [`Navigator::route_directional_traversal_edge_behavior`] is used.
    pub directional_traversal_edge_behavior: Option<TraversalEdgeBehavior>,
    /// The `DelegatedTransitionBuilder` received from the route above this one in the
    /// navigation stack.
    ///
    /// It uses the above route's [`ModalRoute::delegated_transition`] in order to show the
    /// right route transition when the above route either enters or leaves the navigation
    /// stack. If not `None`, it wraps the route content.
    pub received_transition: Option<DelegatedTransitionBuilder>,
    offstage: bool,
    animation_proxy: Option<Handle<ProxyAnimation>>,
    secondary_animation_proxy: Option<Handle<ProxyAnimation>>,
    pop_entries: Vec<Rc<dyn PopEntry>>,
    scope_key: GlobalKey,
    subtree_key: GlobalKey,
    storage_bucket: Handle<PageStorageBucket>,
    modal_barrier: Option<Handle<OverlayEntry>>,
    modal_scope: Option<Handle<OverlayEntry>>,
    modal_scope_cache: Option<WidgetRef>,
}

impl ModalRouteData {
    /// The bag of a freshly created modal route; Dart's constructor defaults.
    pub fn new(app: &mut App) -> ModalRouteData {
        ModalRouteData {
            filter: None,
            traversal_edge_behavior: None,
            directional_traversal_edge_behavior: None,
            received_transition: None,
            offstage: false,
            animation_proxy: None,
            secondary_animation_proxy: None,
            pop_entries: Vec::new(),
            scope_key: GlobalKey::new(),
            subtree_key: GlobalKey::new(),
            storage_bucket: PageStorageBucket::new(app),
            modal_barrier: None,
            modal_scope: None,
            modal_scope_cache: None,
        }
    }
}

/// The accessors [`ModalRoute`] asks for, for a struct whose bag is the field `modal_route`.
#[macro_export]
macro_rules! modal_route_accessors {
    () => {
        fn modal_route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ModalRouteData {
            &app.get(self).modal_route
        }

        fn modal_route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ModalRouteData {
            &mut app.get_mut(self).modal_route
        }
    };
}

/// A route that blocks interaction with previous routes.
///
/// [`ModalRoute`]s cover the entire [`Navigator`]. They are not necessarily
/// [`opaque`](TransitionRoute::opaque), however; for example, a pop-up menu uses a
/// [`ModalRoute`] but only shows the menu in a small box overlapping the previous route.
///
/// See also:
///
///  * [`Route`], which further documents the meaning of Dart's `T` generic type argument.
pub trait ModalRoute: LocalHistoryRoute {
    /// Dart's `ModalRoute` fields, held under the field `modal_route`
    /// ([`modal_route_accessors!`](crate::modal_route_accessors)).
    fn modal_route_data(self: Handle<Self>, app: &App) -> &ModalRouteData;

    /// See [`modal_route_data`](Self::modal_route_data).
    fn modal_route_data_mut(self: Handle<Self>, app: &mut App) -> &mut ModalRouteData;

    /// This route as the erased [`AnyModalRoute`].
    fn as_modal_route(self: Handle<Self>) -> AnyModalRoute {
        AnyModalRoute {
            id: self.id(),
            vtable: const { &ModalRouteVTable::of::<Self>() },
        }
    }

    /// Schedules a call to [`build_transitions`](Self::build_transitions).
    ///
    /// Whenever you need to change internal state for a [`ModalRoute`] object, make the change
    /// in the closure you pass to `set_state`. If you just change the state directly without
    /// calling `set_state`, then the route will not be scheduled for rebuilding, meaning that
    /// its rendering will not be updated.
    fn set_state(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut App)) {
        let key = self.modal_route_data(app).scope_key.clone();
        match key.current_state::<ModalScopeState>(app) {
            Some(state) => state.route_set_state(app, change),
            None => {
                // The route isn't currently visible, so we don't have to call its `set_state`
                // method, but we do still need to run the change, otherwise the state in the
                // route won't be updated.
                change(app);
            }
        }
    }

    /// Override this method to build the primary content of this route.
    ///
    /// The arguments have the following meanings:
    ///
    ///  * `context`: The context in which the route is being built.
    ///  * `animation`: The animation for this route's transition. When entering, the animation
    ///    runs forward from 0.0 to 1.0. When exiting, it runs backwards from 1.0 to 0.0.
    ///  * `secondary_animation`: The animation for the route being pushed on top of this route.
    ///
    /// This method is only called when the route is first built, and rarely thereafter. In
    /// particular, it is not automatically called again when the route's state changes unless
    /// it uses [`AnyModalRoute::of`]. For a builder that is called every time the route's state
    /// changes, consider [`build_transitions`](Self::build_transitions).
    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef;

    /// Override this method to wrap the `child` with one or more transition widgets that define
    /// how the route arrives on and leaves the screen.
    ///
    /// By default, the child (which contains the widget returned by
    /// [`build_page`](Self::build_page)) is not wrapped in any transition widgets.
    ///
    /// This method, in contrast to [`build_page`](Self::build_page), is called each time the
    /// route's state changes while it is visible.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = (app, context, animation, secondary_animation);
        child
    }

    /// The `DelegatedTransitionBuilder` provided to the route below this one in the navigation
    /// stack.
    ///
    /// Used for the purposes of coordinating transitions between two routes with different
    /// route transitions. When a route is added to the stack, the original topmost route will
    /// look for this transition, and if available, it will use it to animate off the screen.
    ///
    /// If the builder returns `None`, then by default the original transition of the routes
    /// will be used.
    ///
    /// The [`ModalRoute`] receiving this transition sets it as its
    /// [`received_transition`](ModalRouteData::received_transition).
    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        let _ = app;
        None
    }

    /// Dart's `_buildFlexibleTransitions`: wraps the transitions of this route with the
    /// [`received_transition`](ModalRouteData::received_transition), when there is one.
    fn build_flexible_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let received = self.modal_route_data(app).received_transition.clone();
        let Some(received) = received else {
            return ModalRoute::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            );
        };
        if secondary_animation.is_dismissed(app) {
            return ModalRoute::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            );
        }

        // Create a static proxy animation to suppress the original secondary transition.
        let proxy_animation = ProxyAnimation::new(app, None);
        let proxied_original_transitions = ModalRoute::build_transitions(
            self,
            app,
            context,
            animation,
            proxy_animation.as_animation(),
            child,
        );

        // If the received transition returns nothing, then we want the original transitions,
        // but with the secondary animation still proxied. This keeps a desynched animation from
        // playing.
        let allow_snapshotting = TransitionRoute::allow_snapshotting(self, app);
        received(
            app,
            context,
            animation,
            secondary_animation,
            allow_snapshotting,
            Some(&proxied_original_transitions),
        )
        .unwrap_or(proxied_original_transitions)
    }

    /// Dart's `ModalRoute.install`.
    fn install(self: Handle<Self>, app: &mut App) {
        TransitionRoute::install(self, app);
        let animation = self.transition_route_data(app).animation;
        let secondary = self
            .transition_route_data(app)
            .secondary_animation
            .as_animation();
        self.modal_route_data_mut(app).animation_proxy = Some(ProxyAnimation::new(app, animation));
        self.modal_route_data_mut(app).secondary_animation_proxy =
            Some(ProxyAnimation::new(app, Some(secondary)));
    }

    /// Dart's `ModalRoute.didPush`.
    fn did_push(self: Handle<Self>, app: &mut App) -> Handle<TickerFuture> {
        let key = self.modal_route_data(app).scope_key.clone();
        if let Some(state) = key.current_state::<ModalScopeState>(app) {
            let navigator = self.as_route().navigator(app).expect("an installed route");
            if navigator.widget(app).request_focus {
                let focus_node = navigator.focus_node(app);
                let scope_node = state.focus_scope_node(app);
                if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                    scope.set_first_focus(app, scope_node);
                }
            }
        }
        TransitionRoute::did_push(self, app)
    }

    /// Dart's `ModalRoute.didAdd`.
    fn did_add(self: Handle<Self>, app: &mut App) {
        let key = self.modal_route_data(app).scope_key.clone();
        if let Some(state) = key.current_state::<ModalScopeState>(app) {
            let navigator = self.as_route().navigator(app).expect("an installed route");
            if navigator.widget(app).request_focus {
                let focus_node = navigator.focus_node(app);
                let scope_node = state.focus_scope_node(app);
                if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                    scope.set_first_focus(app, scope_node);
                }
            }
        }
        TransitionRoute::did_add(self, app);
    }

    /// Whether you can dismiss this route by tapping the modal barrier.
    ///
    /// The modal barrier is the scrim that is rendered behind each route, which generally
    /// prevents the user from interacting with the route below the current route, and normally
    /// partially obscures such routes.
    ///
    /// If `barrier_dismissible` is true, then tapping this barrier, pressing the escape key on
    /// the keyboard, or calling route popping functions such as [`Navigator::pop`] will cause
    /// the current route to be popped with no value.
    ///
    /// If this getter would ever start returning a different value, either
    /// [`changed_internal_state`](Route::changed_internal_state) or
    /// [`changed_external_state`](Route::changed_external_state) should be invoked so that the
    /// change can take effect.
    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool;

    /// Whether the semantics of the modal barrier are included in the semantics tree.
    fn semantics_dismissible(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// The color to use for the modal barrier. If this is `None`, the barrier will be
    /// transparent.
    ///
    /// The color is ignored, and the barrier made invisible, when
    /// [`offstage`](Self::offstage) is true.
    ///
    /// While the route is animating into position, the color is animated from transparent to
    /// the specified color.
    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color>;

    /// The semantic label used for a dismissible barrier.
    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String>;

    /// The curve that is used for animating the modal barrier in and out.
    ///
    /// It defaults to [`Curves::ease`].
    fn barrier_curve(self: Handle<Self>, app: &App) -> Rc<dyn Curve> {
        let _ = app;
        Curves::ease()
    }

    /// Whether the route should remain in memory when it is inactive.
    ///
    /// If this is true, then the route is maintained, so that any completions it is holding
    /// from the next route will properly resolve when the next route pops. If this is not
    /// necessary, this can be set to false to allow the framework to entirely discard the
    /// route's widget hierarchy when it is not visible.
    fn maintain_state(self: Handle<Self>, app: &App) -> bool;

    /// True if a back gesture (iOS-style back swipe or Android predictive back) is currently
    /// underway for this route.
    fn pop_gesture_in_progress(self: Handle<Self>, app: &App) -> bool {
        self.as_route()
            .navigator(app)
            .expect("an installed route")
            .user_gesture_in_progress(app)
    }

    /// Dart's `ModalRoute.popGestureEnabled`.
    fn pop_gesture_enabled(self: Handle<Self>, app: &mut App) -> bool {
        // If there's nothing to go back to, then obviously we don't support the back gesture.
        if self.as_route().is_first(app) {
            return false;
        }
        // If the route wouldn't actually pop if we popped it, then the gesture would be really
        // confusing (or would skip internal routes), so disallow it.
        if Route::will_handle_pop_internally(self, app) {
            return false;
        }
        // If attempts to dismiss this route might be vetoed, then do not allow the user to
        // dismiss the route with a swipe.
        if Route::pop_disposition(self, app) == RoutePopDisposition::DoNotPop {
            return false;
        }
        // If we're in an animation already, we cannot be manually swiped.
        let animation = TransitionRoute::animation(self, app).expect("an installed route");
        if !animation.is_completed(app) {
            return false;
        }
        // Looks like a back gesture would be welcome!
        true
    }

    /// Whether this route is currently offstage.
    ///
    /// On the first frame of a route's entrance transition, the route is built `Offstage` using
    /// an animation progress of 1.0. The route is invisible and non-interactive, but each
    /// widget has its final size and position. This mechanism lets the `HeroController`
    /// determine the final location of any hero widgets being animated as part of the
    /// transition.
    ///
    /// The modal barrier, if any, is not rendered if `offstage` is true.
    ///
    /// Whenever this changes value, [`changed_internal_state`](Route::changed_internal_state)
    /// is called.
    fn offstage(self: Handle<Self>, app: &App) -> bool {
        self.modal_route_data(app).offstage
    }

    /// Sets [`offstage`](Self::offstage).
    fn set_offstage(self: Handle<Self>, app: &mut App, value: bool) {
        if self.modal_route_data(app).offstage == value {
            return;
        }
        ModalRoute::set_state(self, app, |app| {
            self.modal_route_data_mut(app).offstage = value;
        });
        let offstage = self.modal_route_data(app).offstage;
        let animation_proxy = self
            .modal_route_data(app)
            .animation_proxy
            .expect("installed");
        let parent = if offstage {
            Some(k_always_complete_animation(app))
        } else {
            self.transition_route_data(app).animation
        };
        animation_proxy.set_parent(app, parent);
        let secondary_proxy = self
            .modal_route_data(app)
            .secondary_animation_proxy
            .expect("installed");
        let parent = if offstage {
            k_always_dismissed_animation(app)
        } else {
            self.transition_route_data(app)
                .secondary_animation
                .as_animation()
        };
        secondary_proxy.set_parent(app, Some(parent));
        Route::changed_internal_state(self, app);
    }

    /// The build context for the subtree containing the primary content of this route.
    fn subtree_context(self: Handle<Self>, app: &mut App) -> Option<BuildContext> {
        let key = self.modal_route_data(app).subtree_key.clone();
        key.current_context(app)
    }

    /// Dart's `ModalRoute.animation`.
    fn animation(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        self.modal_route_data(app)
            .animation_proxy
            .map(Animation::as_animation)
    }

    /// Dart's `ModalRoute.secondaryAnimation`.
    fn secondary_animation(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        self.modal_route_data(app)
            .secondary_animation_proxy
            .map(Animation::as_animation)
    }

    /// Dart's `ModalRoute.popDisposition`.
    ///
    /// Returns [`RoutePopDisposition::DoNotPop`] if any of the [`PopEntry`] instances
    /// registered with [`register_pop_entry`](Self::register_pop_entry) have their
    /// [`PopEntry::can_pop_notifier`] set to false.
    fn pop_disposition(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        for pop_entry in self.modal_route_data(app).pop_entries.clone() {
            if !*pop_entry.can_pop_notifier(app).value(app) {
                return RoutePopDisposition::DoNotPop;
            }
        }
        LocalHistoryRoute::pop_disposition(self, app)
    }

    /// Dart's `ModalRoute.onPopInvokedWithResult`.
    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        for pop_entry in self.modal_route_data(app).pop_entries.clone() {
            pop_entry.on_pop_invoked_with_result(app, did_pop, result.clone());
        }
        RouteBase::on_pop_invoked_with_result(self, app, did_pop, result);
    }

    /// Registers the existence of a [`PopEntry`] in the route.
    ///
    /// [`PopEntry`] instances registered in this way will have their
    /// [`PopEntry::on_pop_invoked_with_result`] callbacks called when a route is popped or a
    /// pop is attempted. They will also be able to block pop operations with
    /// [`PopEntry::can_pop_notifier`] through this route's
    /// [`pop_disposition`](Self::pop_disposition) method.
    fn register_pop_entry(self: Handle<Self>, app: &mut App, pop_entry: Rc<dyn PopEntry>) {
        if !self
            .modal_route_data(app)
            .pop_entries
            .iter()
            .any(|held| Rc::ptr_eq(held, &pop_entry))
        {
            self.modal_route_data_mut(app)
                .pop_entries
                .push(Rc::clone(&pop_entry));
        }
        let notifier = pop_entry.can_pop_notifier(app);
        notifier.add_listener(
            app,
            Listener::handle_method(self, Self::maybe_dispatch_navigation_notification),
        );
        ModalRoute::maybe_dispatch_navigation_notification(self, app);
    }

    /// Unregisters a [`PopEntry`] in the route's widget subtree.
    fn unregister_pop_entry(self: Handle<Self>, app: &mut App, pop_entry: Rc<dyn PopEntry>) {
        self.modal_route_data_mut(app)
            .pop_entries
            .retain(|held| !Rc::ptr_eq(held, &pop_entry));
        let notifier = pop_entry.can_pop_notifier(app);
        notifier.remove_listener(
            app,
            &Listener::handle_method(self, Self::maybe_dispatch_navigation_notification),
        );
        ModalRoute::maybe_dispatch_navigation_notification(self, app);
    }

    /// Dart's `_maybeDispatchNavigationNotification`.
    fn maybe_dispatch_navigation_notification(self: Handle<Self>, app: &mut App) {
        if !self.as_route().is_current(app) {
            return;
        }
        // `can_handle_pop` indicates that the originator of the notification can handle a pop.
        // In the case of `PopScope`, it handles pops when `can_pop` is false. Hence the
        // seemingly backward logic here.
        let notification = NavigationNotification::new(
            Route::pop_disposition(self, app) == RoutePopDisposition::DoNotPop,
        );
        // Avoid dispatching a notification in the middle of a build.
        match SchedulerBinding::scheduler_phase(app) {
            SchedulerPhase::PostFrameCallbacks => {
                let context = ModalRoute::subtree_context(self, app);
                notification.dispatch(app, context);
            }
            SchedulerPhase::Idle
            | SchedulerPhase::MidFrameMicrotasks
            | SchedulerPhase::PersistentCallbacks
            | SchedulerPhase::TransientCallbacks => {
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _time_stamp| {
                        let context = ModalRoute::subtree_context(self, app);
                        if !context.is_some_and(|context| context.mounted(app)) {
                            return;
                        }
                        notification.dispatch(app, context);
                    }),
                );
            }
        }
    }

    /// Dart's `ModalRoute.didChangePrevious`.
    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        RouteBase::did_change_previous(self, app, previous_route);
        Route::changed_internal_state(self, app);
    }

    /// Dart's `ModalRoute.didChangeNext`.
    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        self.update_received_transition(app, next_route);
        TransitionRoute::did_change_next(self, app, next_route);
        Route::changed_internal_state(self, app);
    }

    /// Dart's `ModalRoute.didPopNext`.
    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        self.update_received_transition(app, Some(next_route));
        TransitionRoute::did_pop_next(self, app, next_route);
        Route::changed_internal_state(self, app);
        ModalRoute::maybe_dispatch_navigation_notification(self, app);
    }

    /// The `nextRoute is ModalRoute && canTransitionTo(nextRoute) && …` branch both
    /// [`did_change_next`](Self::did_change_next) and [`did_pop_next`](Self::did_pop_next) run.
    fn update_received_transition(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        let next_modal = next_route.and_then(|route| route.as_modal_route(app));
        let received = match next_modal {
            Some(next) => {
                let next_transition = next
                    .as_route()
                    .as_transition_route(app)
                    .expect("a modal route is a transition route");
                let mine = ModalRoute::delegated_transition(self, app);
                let theirs = next.delegated_transition(app);
                let differs = match (&mine, &theirs) {
                    (None, None) => false,
                    (Some(mine), Some(theirs)) => !Rc::ptr_eq(mine, theirs),
                    _ => true,
                };
                if TransitionRoute::can_transition_to(self, app, next_transition) && differs {
                    theirs
                } else {
                    None
                }
            }
            None => None,
        };
        self.modal_route_data_mut(app).received_transition = received;
    }

    /// Dart's `ModalRoute.changedInternalState`.
    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        RouteBase::changed_internal_state(self, app);
        // No need to mark dirty if this method is called during the build phase.
        if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks {
            ModalRoute::set_state(self, app, |_app| {
                // The internal state already changed.
            });
            let barrier = self.modal_route_data(app).modal_barrier.expect("installed");
            barrier.mark_needs_build(app);
        }
        let maintain_state = ModalRoute::maintain_state(self, app);
        let scope = self.modal_route_data(app).modal_scope.expect("installed");
        scope.set_maintain_state(app, maintain_state);
    }

    /// Dart's `ModalRoute.changedExternalState`.
    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        RouteBase::changed_external_state(self, app);
        let barrier = self.modal_route_data(app).modal_barrier.expect("installed");
        barrier.mark_needs_build(app);
        let key = self.modal_route_data(app).scope_key.clone();
        if let Some(state) = key.current_state::<ModalScopeState>(app) {
            state.force_rebuild_page(app);
        }
    }

    /// Whether this route can be popped.
    ///
    /// A route can be popped if there is at least one active route below it, or if
    /// [`will_handle_pop_internally`](Route::will_handle_pop_internally) returns true.
    fn can_pop(self: Handle<Self>, app: &mut App) -> bool {
        self.as_route().has_active_route_below(app) || Route::will_handle_pop_internally(self, app)
    }

    /// Whether an app bar in the route should automatically add a back button or close button.
    ///
    /// This returns true if there is at least one active route below it, or there is at least
    /// one [`LocalHistoryEntry`] with
    /// [`implies_app_bar_dismissal`](LocalHistoryEntry::implies_app_bar_dismissal) set to true.
    fn implies_app_bar_dismissal(self: Handle<Self>, app: &App) -> bool {
        self.as_route().has_active_route_below(app)
            || self
                .local_history_route_data(app)
                .entries_implies_app_bar_dismissal
                > 0
    }

    /// Whether this route is a full-screen dialog.
    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }

    /// Dart's `_buildModalBarrier`.
    fn build_modal_barrier_entry(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetRef {
        let mut barrier = ModalRoute::build_modal_barrier(self, app, context);
        if let Some(filter) = self.modal_route_data(app).filter.clone() {
            barrier = crate::widgets::basic::BackdropFilter::new(filter)
                .child(barrier)
                .into_widget();
        }
        let animation = TransitionRoute::animation(self, app).expect("an installed route");
        // `changed_internal_state` is called when the animation status updates; dismissed is
        // possible when doing a manual pop gesture.
        IgnorePointer::new()
            .ignoring(!animation.is_forward_or_completed(app))
            .child(barrier)
            .into_widget()
    }

    /// Builds the barrier for this [`ModalRoute`]; subclasses can override this method to
    /// create their own barrier with customized features such as color or accessibility focus
    /// size, and reach this body through [`ModalRouteBase::build_modal_barrier`].
    fn build_modal_barrier(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        ModalRouteBase::build_modal_barrier(self, app, context)
    }

    /// Dart's `_buildModalScope`: the part of the modal scope that does not change from frame
    /// to frame, cached so that the amount of building is minimized.
    fn build_modal_scope(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        match self.modal_route_data(app).modal_scope_cache.clone() {
            Some(cache) => cache,
            None => {
                let key = self.modal_route_data(app).scope_key.clone();
                let scope = ModalScope {
                    key: Some(Rc::new(key)),
                    route: ModalRoute::as_modal_route(self),
                }
                .into_widget();
                self.modal_route_data_mut(app).modal_scope_cache = Some(scope.clone());
                scope
            }
        }
    }

    /// Dart's `ModalRoute.createOverlayEntries`.
    fn create_overlay_entries(self: Handle<Self>, app: &mut App) -> Vec<Handle<OverlayEntry>> {
        let barrier = OverlayEntry::new(
            app,
            Rc::new(move |app, context| ModalRoute::build_modal_barrier_entry(self, app, context)),
            false,
            false,
            false,
        );
        self.modal_route_data_mut(app).modal_barrier = Some(barrier);
        let maintain_state = ModalRoute::maintain_state(self, app);
        let opaque = TransitionRoute::opaque(self, app);
        let scope = OverlayEntry::new(
            app,
            Rc::new(move |app, context| ModalRoute::build_modal_scope(self, app, context)),
            false,
            maintain_state,
            opaque,
        );
        self.modal_route_data_mut(app).modal_scope = Some(scope);
        vec![barrier, scope]
    }
}

/// The bodies of Dart's `ModalRoute` class that a subclass's override runs as `super`.
///
/// Every [`ModalRoute`] implementor gets these bodies, so an override on the leaf never reaches
/// them by accident.
pub trait ModalRouteBase: ModalRoute {
    /// Dart's `ModalRoute.buildModalBarrier`.
    fn build_modal_barrier(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        let barrier_color = ModalRoute::barrier_color(self, app);
        let dismissible = ModalRoute::barrier_dismissible(self, app);
        let semantics_label = ModalRoute::barrier_label(self, app);
        let semantics_dismissible = ModalRoute::semantics_dismissible(self, app);
        match barrier_color {
            Some(barrier_color) if barrier_color.a != 0.0 && !ModalRoute::offstage(self, app) => {
                // `changed_internal_state` is called if the barrier color or offstage updates.
                let transparent = barrier_color.with_values(Some(0.0), None, None, None, None);
                debug_assert!(barrier_color != transparent);
                let animation = TransitionRoute::animation(self, app).expect("an installed route");
                let curve = ModalRoute::barrier_curve(self, app);
                let color_tween = ColorTween::new(app, Some(transparent), Some(barrier_color));
                let curve_tween = CurveTween::new(app, curve);
                let color = animation.drive(app, color_tween.chain(curve_tween));
                AnimatedModalBarrier::new(color)
                    .dismissible(dismissible)
                    .barrier_semantics_dismissible(semantics_dismissible)
                    .maybe_semantics_label(semantics_label)
                    .into_widget()
            }
            _ => ModalBarrier::new()
                .dismissible(dismissible)
                .barrier_semantics_dismissible(semantics_dismissible)
                .maybe_semantics_label(semantics_label)
                .into_widget(),
        }
    }
}

impl<R: ModalRoute> ModalRouteBase for R {}

/// The [`ModalRouteVTable`] slot for [`ModalRoute::build_flexible_transitions`].
type BuildFlexibleTransitionsSlot = fn(
    &mut App,
    HandleId,
    BuildContext,
    AnyAnimation<f64>,
    AnyAnimation<f64>,
    WidgetRef,
) -> WidgetRef;

/// The vtable of an erased [`AnyModalRoute`]: one `&'static` table per route type, built by
/// [`ModalRouteVTable::of`].
struct ModalRouteVTable {
    type_name: fn() -> &'static str,
    route: fn(HandleId) -> AnyRoute,
    transition_route: fn(HandleId) -> AnyTransitionRoute,
    barrier_dismissible: fn(&App, HandleId) -> bool,
    barrier_color: fn(&App, HandleId) -> Option<Color>,
    barrier_label: fn(&App, HandleId) -> Option<String>,
    maintain_state: fn(&App, HandleId) -> bool,
    opaque: fn(&App, HandleId) -> bool,
    offstage: fn(&App, HandleId) -> bool,
    set_offstage: fn(&mut App, HandleId, bool),
    animation: fn(&App, HandleId) -> Option<AnyAnimation<f64>>,
    secondary_animation: fn(&App, HandleId) -> Option<AnyAnimation<f64>>,
    delegated_transition: fn(&App, HandleId) -> Option<DelegatedTransitionBuilder>,
    can_pop: fn(&mut App, HandleId) -> bool,
    implies_app_bar_dismissal: fn(&App, HandleId) -> bool,
    fullscreen_dialog: fn(&App, HandleId) -> bool,
    pop_gesture_in_progress: fn(&App, HandleId) -> bool,
    traversal_edge_behavior: fn(&App, HandleId) -> Option<TraversalEdgeBehavior>,
    directional_traversal_edge_behavior: fn(&App, HandleId) -> Option<TraversalEdgeBehavior>,
    storage_bucket: fn(&App, HandleId) -> Handle<PageStorageBucket>,
    subtree_key: fn(&App, HandleId) -> GlobalKey,
    subtree_context: fn(&mut App, HandleId) -> Option<BuildContext>,
    build_page:
        fn(&mut App, HandleId, BuildContext, AnyAnimation<f64>, AnyAnimation<f64>) -> WidgetRef,
    build_flexible_transitions: BuildFlexibleTransitionsSlot,
    add_local_history_entry: fn(&mut App, HandleId, Handle<LocalHistoryEntry>),
    remove_local_history_entry: fn(&mut App, HandleId, Handle<LocalHistoryEntry>),
    register_pop_entry: fn(&mut App, HandleId, Rc<dyn PopEntry>),
    unregister_pop_entry: fn(&mut App, HandleId, Rc<dyn PopEntry>),
}

impl ModalRouteVTable {
    /// The table for one modal route type.
    const fn of<R: ModalRoute>() -> ModalRouteVTable {
        ModalRouteVTable {
            type_name: std::any::type_name::<R>,
            route: |id| Route::as_route(resolve::<R>(id)),
            transition_route: |id| TransitionRoute::as_transition_route(resolve::<R>(id)),
            barrier_dismissible: |app, id| R::barrier_dismissible(resolve(id), app),
            barrier_color: |app, id| R::barrier_color(resolve(id), app),
            barrier_label: |app, id| R::barrier_label(resolve(id), app),
            maintain_state: |app, id| R::maintain_state(resolve(id), app),
            opaque: |app, id| TransitionRoute::opaque(resolve::<R>(id), app),
            offstage: |app, id| R::offstage(resolve(id), app),
            set_offstage: |app, id, value| R::set_offstage(resolve(id), app, value),
            animation: |app, id| ModalRoute::animation(resolve::<R>(id), app),
            secondary_animation: |app, id| ModalRoute::secondary_animation(resolve::<R>(id), app),
            delegated_transition: |app, id| R::delegated_transition(resolve(id), app),
            can_pop: |app, id| R::can_pop(resolve(id), app),
            implies_app_bar_dismissal: |app, id| R::implies_app_bar_dismissal(resolve(id), app),
            fullscreen_dialog: |app, id| R::fullscreen_dialog(resolve(id), app),
            pop_gesture_in_progress: |app, id| R::pop_gesture_in_progress(resolve(id), app),
            traversal_edge_behavior: |app, id| {
                R::modal_route_data(resolve::<R>(id), app).traversal_edge_behavior
            },
            directional_traversal_edge_behavior: |app, id| {
                R::modal_route_data(resolve::<R>(id), app).directional_traversal_edge_behavior
            },
            storage_bucket: |app, id| R::modal_route_data(resolve::<R>(id), app).storage_bucket,
            subtree_key: |app, id| {
                R::modal_route_data(resolve::<R>(id), app)
                    .subtree_key
                    .clone()
            },
            subtree_context: |app, id| R::subtree_context(resolve::<R>(id), app),
            build_page: |app, id, context, animation, secondary| {
                R::build_page(resolve(id), app, context, animation, secondary)
            },
            build_flexible_transitions: |app, id, context, animation, secondary, child| {
                R::build_flexible_transitions(
                    resolve(id),
                    app,
                    context,
                    animation,
                    secondary,
                    child,
                )
            },
            add_local_history_entry: |app, id, entry| {
                LocalHistoryRoute::add_local_history_entry(resolve::<R>(id), app, entry)
            },
            remove_local_history_entry: |app, id, entry| {
                LocalHistoryRoute::remove_local_history_entry(resolve::<R>(id), app, entry)
            },
            register_pop_entry: |app, id, entry| R::register_pop_entry(resolve(id), app, entry),
            unregister_pop_entry: |app, id, entry| R::unregister_pop_entry(resolve(id), app, entry),
        }
    }
}

/// Erased [`ModalRoute`]: what Dart's `ModalRoute.of(context)` returns and what
/// `route is ModalRoute<dynamic>` produces.
#[derive(Clone, Copy)]
pub struct AnyModalRoute {
    id: HandleId,
    vtable: &'static ModalRouteVTable,
}

impl PartialEq for AnyModalRoute {
    fn eq(&self, other: &AnyModalRoute) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyModalRoute {}

impl std::hash::Hash for AnyModalRoute {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyModalRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyModalRoute {
    /// Returns the modal route most closely associated with the given context.
    ///
    /// Returns `None` if the given context is not associated with a modal route.
    ///
    /// The given [`BuildContext`] will be rebuilt if the state of the route changes while it is
    /// visible (specifically, if [`AnyRoute::is_current`] or [`can_pop`](Self::can_pop) change
    /// value).
    ///
    /// Dart's `ModalRoute.of(context)`.
    pub fn of(app: &mut App, context: BuildContext) -> Option<AnyModalRoute> {
        AnyModalRoute::of_aspect(app, context, None)
    }

    /// Dart's `ModalRoute._of`.
    fn of_aspect(
        app: &mut App,
        context: BuildContext,
        aspect: Option<ModalRouteAspect>,
    ) -> Option<AnyModalRoute> {
        ModalScopeStatus::inherit_from(app, context, aspect).map(|status| status.route)
    }

    /// Returns [`AnyRoute::is_current`] for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn is_current_of(app: &mut App, context: BuildContext) -> Option<bool> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::IsCurrent))?;
        Some(route.as_route().is_current(app))
    }

    /// Returns [`can_pop`](Self::can_pop) for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn can_pop_of(app: &mut App, context: BuildContext) -> Option<bool> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::CanPop))?;
        Some(route.can_pop(app))
    }

    /// Returns [`AnyRoute::settings`] for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn settings_of(app: &mut App, context: BuildContext) -> Option<RouteSettingsRef> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::Settings))?;
        Some(route.as_route().settings(app))
    }

    /// Returns [`AnyRoute::is_active`] for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn is_active_of(app: &mut App, context: BuildContext) -> Option<bool> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::IsActive))?;
        Some(route.as_route().is_active(app))
    }

    /// Returns [`AnyRoute::is_first`] for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn is_first_of(app: &mut App, context: BuildContext) -> Option<bool> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::IsFirst))?;
        Some(route.as_route().is_first(app))
    }

    /// Returns [`opaque`](Self::opaque) for the modal route most closely associated with the
    /// given context, creating a dependency on that aspect.
    pub fn opaque_of(app: &mut App, context: BuildContext) -> Option<bool> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::Opaque))?;
        Some(route.opaque(app))
    }

    /// Returns [`AnyRoute::pop_disposition`] for the modal route most closely associated with
    /// the given context, creating a dependency on that aspect.
    pub fn pop_disposition_of(app: &mut App, context: BuildContext) -> Option<RoutePopDisposition> {
        let route = AnyModalRoute::of_aspect(app, context, Some(ModalRouteAspect::PopDisposition))?;
        Some(route.as_route().pop_disposition(app))
    }

    /// Returns a predicate that is true if the route has the specified name and if popping the
    /// route will not yield the same route, i.e. if the route's
    /// [`Route::will_handle_pop_internally`] is false.
    ///
    /// This function is typically used with [`NavigatorState::pop_until`](crate::NavigatorState::pop_until).
    pub fn with_name(name: &str) -> RoutePredicate {
        let name = name.to_string();
        Rc::new(move |app, route: AnyRoute| {
            !route.will_handle_pop_internally(app)
                && route.as_modal_route(app).is_some()
                && route.settings(app).name() == Some(name.as_str())
        })
    }

    /// This modal route as the erased [`AnyRoute`].
    pub fn as_route(self) -> AnyRoute {
        (self.vtable.route)(self.id)
    }

    /// This modal route as the erased [`AnyTransitionRoute`].
    pub fn as_transition_route(self) -> AnyTransitionRoute {
        (self.vtable.transition_route)(self.id)
    }

    /// See [`ModalRoute::barrier_dismissible`].
    pub fn barrier_dismissible(self, app: &App) -> bool {
        (self.vtable.barrier_dismissible)(app, self.id)
    }

    /// See [`ModalRoute::barrier_color`].
    pub fn barrier_color(self, app: &App) -> Option<Color> {
        (self.vtable.barrier_color)(app, self.id)
    }

    /// See [`ModalRoute::barrier_label`].
    pub fn barrier_label(self, app: &App) -> Option<String> {
        (self.vtable.barrier_label)(app, self.id)
    }

    /// See [`ModalRoute::maintain_state`].
    pub fn maintain_state(self, app: &App) -> bool {
        (self.vtable.maintain_state)(app, self.id)
    }

    /// See [`TransitionRoute::opaque`].
    pub fn opaque(self, app: &App) -> bool {
        (self.vtable.opaque)(app, self.id)
    }

    /// See [`ModalRoute::offstage`].
    pub fn offstage(self, app: &App) -> bool {
        (self.vtable.offstage)(app, self.id)
    }

    /// See [`ModalRoute::set_offstage`].
    pub fn set_offstage(self, app: &mut App, value: bool) {
        (self.vtable.set_offstage)(app, self.id, value)
    }

    /// See [`ModalRoute::animation`].
    pub fn animation(self, app: &App) -> Option<AnyAnimation<f64>> {
        (self.vtable.animation)(app, self.id)
    }

    /// See [`ModalRoute::secondary_animation`].
    pub fn secondary_animation(self, app: &App) -> Option<AnyAnimation<f64>> {
        (self.vtable.secondary_animation)(app, self.id)
    }

    /// See [`ModalRoute::delegated_transition`].
    pub fn delegated_transition(self, app: &App) -> Option<DelegatedTransitionBuilder> {
        (self.vtable.delegated_transition)(app, self.id)
    }

    /// See [`ModalRoute::can_pop`].
    pub fn can_pop(self, app: &mut App) -> bool {
        (self.vtable.can_pop)(app, self.id)
    }

    /// See [`ModalRoute::implies_app_bar_dismissal`].
    pub fn implies_app_bar_dismissal(self, app: &App) -> bool {
        (self.vtable.implies_app_bar_dismissal)(app, self.id)
    }

    /// See [`ModalRoute::fullscreen_dialog`].
    pub fn fullscreen_dialog(self, app: &App) -> bool {
        (self.vtable.fullscreen_dialog)(app, self.id)
    }

    /// See [`ModalRoute::pop_gesture_in_progress`].
    pub fn pop_gesture_in_progress(self, app: &App) -> bool {
        (self.vtable.pop_gesture_in_progress)(app, self.id)
    }

    /// See [`ModalRouteData::traversal_edge_behavior`].
    pub fn traversal_edge_behavior(self, app: &App) -> Option<TraversalEdgeBehavior> {
        (self.vtable.traversal_edge_behavior)(app, self.id)
    }

    /// See [`ModalRouteData::directional_traversal_edge_behavior`].
    pub fn directional_traversal_edge_behavior(self, app: &App) -> Option<TraversalEdgeBehavior> {
        (self.vtable.directional_traversal_edge_behavior)(app, self.id)
    }

    /// Dart's `ModalRoute._storageBucket`.
    pub fn storage_bucket(self, app: &App) -> Handle<PageStorageBucket> {
        (self.vtable.storage_bucket)(app, self.id)
    }

    /// Dart's `ModalRoute._subtreeKey`.
    pub fn subtree_key(self, app: &App) -> GlobalKey {
        (self.vtable.subtree_key)(app, self.id)
    }

    /// See [`ModalRoute::subtree_context`].
    pub fn subtree_context(self, app: &mut App) -> Option<BuildContext> {
        (self.vtable.subtree_context)(app, self.id)
    }

    /// See [`ModalRoute::build_page`].
    pub fn build_page(
        self,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        (self.vtable.build_page)(app, self.id, context, animation, secondary_animation)
    }

    /// See [`ModalRoute::build_flexible_transitions`].
    pub fn build_flexible_transitions(
        self,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        (self.vtable.build_flexible_transitions)(
            app,
            self.id,
            context,
            animation,
            secondary_animation,
            child,
        )
    }

    /// See [`LocalHistoryRoute::add_local_history_entry`].
    pub fn add_local_history_entry(self, app: &mut App, entry: Handle<LocalHistoryEntry>) {
        (self.vtable.add_local_history_entry)(app, self.id, entry)
    }

    /// See [`LocalHistoryRoute::remove_local_history_entry`].
    pub fn remove_local_history_entry(self, app: &mut App, entry: Handle<LocalHistoryEntry>) {
        (self.vtable.remove_local_history_entry)(app, self.id, entry)
    }

    /// See [`ModalRoute::register_pop_entry`].
    pub fn register_pop_entry(self, app: &mut App, pop_entry: Rc<dyn PopEntry>) {
        (self.vtable.register_pop_entry)(app, self.id, pop_entry)
    }

    /// See [`ModalRoute::unregister_pop_entry`].
    pub fn unregister_pop_entry(self, app: &mut App, pop_entry: Rc<dyn PopEntry>) {
        (self.vtable.unregister_pop_entry)(app, self.id, pop_entry)
    }
}

// ---------------------------------------------------------------------------------------------
// PopupRoute

/// A modal route that overlays a widget over the current route.
///
/// See also:
///
///  * [`ModalRoute`], which is the base of this trait.
///  * [`NavigatorState::pop`](crate::NavigatorState::pop), which is used to dismiss the route.
pub trait PopupRoute: ModalRoute {
    /// Dart's `PopupRoute.opaque`.
    fn opaque(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }

    /// Dart's `PopupRoute.maintainState`.
    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// Dart's `PopupRoute.allowSnapshotting`.
    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }
}

// ---------------------------------------------------------------------------------------------
// The forwarders a modal-route leaf writes

/// The `Route` overrides a [`ModalRoute`] leaf inherits, plus
/// [`route_accessors!`](crate::route_accessors). Write it inside `impl Route for MyRoute { .. }`.
#[macro_export]
macro_rules! modal_route_overrides {
    () => {
        $crate::route_accessors!();

        fn as_transition_route(
            self: ::reveal_foundation::Handle<Self>,
        ) -> ::std::option::Option<$crate::AnyTransitionRoute> {
            ::std::option::Option::Some($crate::TransitionRoute::as_transition_route(self))
        }

        fn as_modal_route(
            self: ::reveal_foundation::Handle<Self>,
        ) -> ::std::option::Option<$crate::AnyModalRoute> {
            ::std::option::Option::Some($crate::ModalRoute::as_modal_route(self))
        }

        fn overlay_entries(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::vec::Vec<::reveal_foundation::Handle<$crate::OverlayEntry>> {
            $crate::OverlayRoute::overlay_entries(self, app)
        }

        fn install(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::ModalRoute::install(self, app)
        }

        fn did_push(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> ::reveal_foundation::Handle<::reveal_scheduler::TickerFuture> {
            $crate::ModalRoute::did_push(self, app)
        }

        fn did_add(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::ModalRoute::did_add(self, app)
        }

        fn did_replace(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            old_route: ::std::option::Option<$crate::AnyRoute>,
        ) {
            $crate::TransitionRoute::did_replace(self, app, old_route)
        }

        fn will_pop(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> $crate::RoutePopDisposition {
            $crate::LocalHistoryRoute::will_pop(self, app)
        }

        fn pop_disposition(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> $crate::RoutePopDisposition {
            $crate::ModalRoute::pop_disposition(self, app)
        }

        fn on_pop_invoked_with_result(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            did_pop: bool,
            result: $crate::RouteResult,
        ) {
            $crate::ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
        }

        fn will_handle_pop_internally(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::LocalHistoryRoute::will_handle_pop_internally(self, app)
        }

        fn did_pop(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            result: $crate::RouteResult,
        ) -> bool {
            $crate::LocalHistoryRoute::did_pop(self, app, result)
        }

        fn did_pop_next(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            next_route: $crate::AnyRoute,
        ) {
            $crate::ModalRoute::did_pop_next(self, app, next_route)
        }

        fn did_change_next(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            next_route: ::std::option::Option<$crate::AnyRoute>,
        ) {
            $crate::ModalRoute::did_change_next(self, app, next_route)
        }

        fn did_change_previous(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            previous_route: ::std::option::Option<$crate::AnyRoute>,
        ) {
            $crate::ModalRoute::did_change_previous(self, app, previous_route)
        }

        fn changed_internal_state(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            $crate::ModalRoute::changed_internal_state(self, app)
        }

        fn changed_external_state(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            $crate::ModalRoute::changed_external_state(self, app)
        }

        fn dispose(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::TransitionRoute::dispose(self, app)
        }
    };
}

/// The `OverlayRoute` overrides a [`ModalRoute`] leaf inherits, plus
/// [`overlay_route_accessors!`](crate::overlay_route_accessors). Write it inside
/// `impl OverlayRoute for MyRoute { .. }`.
#[macro_export]
macro_rules! modal_overlay_route_overrides {
    () => {
        $crate::overlay_route_accessors!();

        fn create_overlay_entries(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> ::std::vec::Vec<::reveal_foundation::Handle<$crate::OverlayEntry>> {
            $crate::ModalRoute::create_overlay_entries(self, app)
        }

        fn finished_when_popped(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::TransitionRoute::finished_when_popped(self, app)
        }
    };
}

/// The `PredictiveBackRoute` implementation a [`ModalRoute`] leaf inherits. Write it inside
/// `impl PredictiveBackRoute for MyRoute { .. }`.
#[macro_export]
macro_rules! modal_predictive_back_route_overrides {
    ($pop_gesture_enabled:path) => {
        fn pop_gesture_enabled(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> bool {
            $pop_gesture_enabled(self, app)
        }

        fn handle_start_back_gesture(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            progress: f64,
        ) {
            $crate::TransitionRoute::handle_start_back_gesture(self, app, progress)
        }

        fn handle_update_back_gesture_progress(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            progress: f64,
        ) {
            $crate::TransitionRoute::handle_update_back_gesture_progress(self, app, progress)
        }

        fn handle_commit_back_gesture(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            $crate::TransitionRoute::handle_commit_back_gesture(self, app)
        }

        fn handle_cancel_back_gesture(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            $crate::TransitionRoute::handle_cancel_back_gesture(self, app)
        }
    };
}

/// The `TransitionRoute` overrides a [`ModalRoute`] leaf inherits, plus
/// [`transition_route_accessors!`](crate::transition_route_accessors). Write it inside
/// `impl TransitionRoute for MyRoute { .. }`, next to the leaf's own `opaque`,
/// `transition_duration` and `allow_snapshotting`.
#[macro_export]
macro_rules! modal_transition_route_overrides {
    () => {
        $crate::transition_route_accessors!();

        fn animation(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::reveal_animation::AnyAnimation<f64>> {
            $crate::ModalRoute::animation(self, app)
        }

        fn secondary_animation(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::reveal_animation::AnyAnimation<f64>> {
            $crate::ModalRoute::secondary_animation(self, app)
        }
    };
}

// ---------------------------------------------------------------------------------------------
// RouteObserver / RouteAware

/// A [`Navigator`] observer that notifies [`RouteAware`]s of changes to the state of their
/// [`Route`].
///
/// [`RouteObserver`] informs subscribers whenever a route the `matches` predicate accepts is
/// pushed on top of their own matching route or popped from it. This is for example useful to
/// keep track of page transitions: a `RouteObserver` built with `AnyRoute::as_page_route` will
/// inform subscribed [`RouteAware`]s whenever the user navigates away from the current page
/// route to another page route.
///
/// See also:
///
///  * [`RouteAware`], which is used with [`RouteObserver`] to make a widget aware of changes to
///    the [`Navigator`]'s session history.
pub struct RouteObserver {
    navigator_observer: NavigatorObserverData,
    matches: RoutePredicate,
    listeners: Vec<(AnyRoute, Vec<Rc<dyn RouteAware>>)>,
}

impl RouteObserver {
    /// Creates an observer of the routes `matches` accepts; Dart's type argument `R`.
    pub fn new(app: &mut App, matches: RoutePredicate) -> Handle<RouteObserver> {
        app.create(RouteObserver {
            navigator_observer: NavigatorObserverData::new(),
            matches,
            listeners: Vec::new(),
        })
    }

    /// Whether this observer is managing changes for the specified route.
    ///
    /// If debug assertions are disabled, this method always returns false.
    pub fn debug_observing_route(self: Handle<Self>, app: &App, route: AnyRoute) -> bool {
        let mut contained = false;
        if cfg!(debug_assertions) {
            contained = app
                .get(self)
                .listeners
                .iter()
                .any(|(held, _)| *held == route);
        }
        contained
    }

    /// Subscribes `route_aware` to be informed about changes to `route`.
    ///
    /// Going forward, `route_aware` will be informed about qualifying changes to `route`, e.g.
    /// when `route` is covered by another route or when `route` is popped off the [`Navigator`]
    /// stack.
    pub fn subscribe(
        self: Handle<Self>,
        app: &mut App,
        route_aware: Rc<dyn RouteAware>,
        route: AnyRoute,
    ) {
        let listeners = &mut app.get_mut(self).listeners;
        let subscribers = match listeners.iter_mut().find(|(held, _)| *held == route) {
            Some((_, subscribers)) => subscribers,
            None => {
                listeners.push((route, Vec::new()));
                &mut listeners.last_mut().expect("just pushed").1
            }
        };
        if subscribers
            .iter()
            .any(|held| Rc::ptr_eq(held, &route_aware))
        {
            return;
        }
        subscribers.push(Rc::clone(&route_aware));
        route_aware.did_push(app);
    }

    /// Unsubscribes `route_aware`.
    ///
    /// `route_aware` is no longer informed about changes to its route. If the given argument
    /// was subscribed to multiple routes, this unregisters it (once) from each.
    pub fn unsubscribe(self: Handle<Self>, app: &mut App, route_aware: Rc<dyn RouteAware>) {
        let listeners = &mut app.get_mut(self).listeners;
        for (_, subscribers) in listeners.iter_mut() {
            subscribers.retain(|held| !Rc::ptr_eq(held, &route_aware));
        }
        listeners.retain(|(_, subscribers)| !subscribers.is_empty());
    }

    fn subscribers_of(
        self: Handle<Self>,
        app: &App,
        route: AnyRoute,
    ) -> Option<Vec<Rc<dyn RouteAware>>> {
        app.get(self)
            .listeners
            .iter()
            .find(|(held, _)| *held == route)
            .map(|(_, subscribers)| subscribers.clone())
    }

    fn matches(self: Handle<Self>, app: &mut App, route: Option<AnyRoute>) -> bool {
        let Some(route) = route else {
            return false;
        };
        let matches = Rc::clone(&app.get(self).matches);
        matches(app, route)
    }
}

impl NavigatorObserver for RouteObserver {
    crate::navigator_observer_accessors!();

    fn did_pop(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        if !self.matches(app, Some(route)) || !self.matches(app, previous_route) {
            return;
        }
        let previous_route = previous_route.expect("matched above");
        if let Some(previous_subscribers) = self.subscribers_of(app, previous_route) {
            for route_aware in previous_subscribers {
                route_aware.did_pop_next(app);
            }
        }
        if let Some(subscribers) = self.subscribers_of(app, route) {
            for route_aware in subscribers {
                route_aware.did_pop(app);
            }
        }
    }

    fn did_push(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        if !self.matches(app, Some(route)) || !self.matches(app, previous_route) {
            return;
        }
        let previous_route = previous_route.expect("matched above");
        if let Some(previous_subscribers) = self.subscribers_of(app, previous_route) {
            for route_aware in previous_subscribers {
                route_aware.did_push_next(app);
            }
        }
    }
}

/// An interface for objects that are aware of their current [`Route`].
///
/// This is used with [`RouteObserver`] to make a widget aware of changes to the [`Navigator`]'s
/// session history.
///
/// What Dart types as `RouteAware` becomes an `Rc<dyn RouteAware>`; an arena object implements
/// [`RouteAwareObject`], and `Rc::new(handle)` is its erased form. A [`RouteObserver`] compares
/// subscribers by identity, so a subscriber keeps the `Rc` it subscribed with to unsubscribe.
pub trait RouteAware {
    /// Called when the top route has been popped off, and the current route shows up.
    fn did_pop_next(&self, app: &mut App);

    /// Called when the current route has been pushed.
    fn did_push(&self, app: &mut App);

    /// Called when the current route has been popped off.
    fn did_pop(&self, app: &mut App);

    /// Called when a new route has been pushed on top of this route, temporarily obscuring it.
    ///
    /// This method is called synchronously during the push operation, before the transition
    /// animation completes. The current route may still be partially visible as it animates
    /// out.
    fn did_push_next(&self, app: &mut App);
}

/// The object side of [`RouteAware`]: an arena object a [`RouteObserver`] informs. Implementing
/// it makes `Handle<Self>` a [`RouteAware`]; Dart's empty default bodies are the defaults here.
pub trait RouteAwareObject: Sized + 'static {
    /// See [`RouteAware::did_pop_next`].
    fn did_pop_next(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`RouteAware::did_push`].
    fn did_push(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`RouteAware::did_pop`].
    fn did_pop(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`RouteAware::did_push_next`].
    fn did_push_next(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }
}

impl<T: RouteAwareObject> RouteAware for Handle<T> {
    fn did_pop_next(&self, app: &mut App) {
        T::did_pop_next(*self, app);
    }

    fn did_push(&self, app: &mut App) {
        T::did_push(*self, app);
    }

    fn did_pop(&self, app: &mut App) {
        T::did_pop(*self, app);
    }

    fn did_push_next(&self, app: &mut App) {
        T::did_push_next(*self, app);
    }
}

// ---------------------------------------------------------------------------------------------
// RawDialogRoute and its callbacks

/// Signature for the function that builds a route's primary contents.
///
/// Used in `PageRouteBuilder` and [`show_general_dialog`].
///
/// See [`ModalRoute::build_page`] for the complete definition of the parameters.
pub type RoutePageBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyAnimation<f64>, AnyAnimation<f64>) -> WidgetRef>;

/// Signature for the function that builds a route's transitions.
///
/// Used in `PageRouteBuilder` and [`show_general_dialog`].
///
/// The `animation` argument drives this route's transition when it is pushed onto or popped off
/// the [`Navigator`]. The `secondary_animation` argument lets this route coordinate with the
/// transition of the route above it: when a new route is pushed on top of this one, this
/// route's `secondary_animation` runs from 0.0 to 1.0, and when that route is popped it runs
/// from 1.0 to 0.0.
///
/// A route only receives a running `secondary_animation` when this route's
/// [`TransitionRoute::can_transition_to`] and the next route's
/// [`TransitionRoute::can_transition_from`] both return true. Otherwise `secondary_animation`
/// remains `k_always_dismissed_animation`.
///
/// See [`ModalRoute::build_transitions`] for the complete definition of the parameters.
pub type RouteTransitionsBuilder = Rc<
    dyn Fn(&mut App, BuildContext, AnyAnimation<f64>, AnyAnimation<f64>, WidgetRef) -> WidgetRef,
>;

/// Configuration details for a custom modal barrier.
///
/// Passed to a [`RouteBarrierBuilder`] by the routing framework to provide the ambient
/// variables associated with the route's modal barrier.
#[derive(Clone, Debug)]
pub struct RouteBarrierDetails {
    /// An animation that drives the route's transition.
    ///
    /// Typically used to animate the barrier's opacity from 0.0 to 1.0 when the route is
    /// pushed.
    pub animation: AnyAnimation<f64>,
    /// The color to paint behind the route.
    ///
    /// If `None`, the barrier will be transparent.
    pub barrier_color: Option<Color>,
    /// The semantic label used for the barrier.
    pub barrier_label: Option<String>,
    /// Whether touching the barrier will pop the current route off the [`Navigator`].
    pub barrier_dismissible: bool,
}

impl RouteBarrierDetails {
    /// Creates an object that contains the configuration for a modal barrier; Dart's optional
    /// arguments are the setters.
    pub fn new(animation: AnyAnimation<f64>, barrier_dismissible: bool) -> RouteBarrierDetails {
        RouteBarrierDetails {
            animation,
            barrier_color: None,
            barrier_label: None,
            barrier_dismissible,
        }
    }

    /// Dart `RouteBarrierDetails(barrierColor:)`.
    pub fn barrier_color(mut self, barrier_color: Color) -> RouteBarrierDetails {
        self.barrier_color = Some(barrier_color);
        self
    }

    /// Dart `RouteBarrierDetails(barrierLabel:)`.
    pub fn barrier_label(mut self, barrier_label: String) -> RouteBarrierDetails {
        self.barrier_label = Some(barrier_label);
        self
    }
}

/// Signature for the function that builds a custom modal barrier for a route.
///
/// Used by [`RawDialogRoute::barrier_builder`] and [`show_general_dialog`] to wrap or replace
/// the default modal barrier.
///
/// The `barrier` parameter is the default [`ModalBarrier`] (or [`AnimatedModalBarrier`])
/// constructed by the framework. Custom builders should typically return a widget that wraps
/// this `barrier`, rather than replacing it entirely, to preserve the built-in semantics and
/// gestures.
pub type RouteBarrierBuilder =
    Rc<dyn Fn(&mut App, BuildContext, &RouteBarrierDetails, WidgetRef) -> WidgetRef>;

/// A callback type for informing that a navigation pop has been invoked, whether or not it was
/// handled successfully.
///
/// Accepts a `did_pop` boolean indicating whether or not back navigation succeeded.
///
/// The `result` contains the pop result.
pub type PopInvokedWithResultCallback = Rc<dyn Fn(&mut App, bool, RouteResult)>;

/// Allows listening to and preventing pops.
///
/// Can be registered in a [`ModalRoute`] to listen to pops with
/// [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result) or to enable and disable
/// them with [`can_pop_notifier`](Self::can_pop_notifier).
///
/// See also:
///
///  * `PopScope`, which provides similar functionality in a widget.
///  * [`ModalRoute::register_pop_entry`], which registers instances of this.
///  * [`ModalRoute::unregister_pop_entry`], which unregisters instances of this.
///
/// What Dart types as `PopEntry` becomes an `Rc<dyn PopEntry>`; an arena object implements
/// [`PopEntryObject`], and `Rc::new(handle)` is its erased form. A [`ModalRoute`] compares
/// entries by identity, so an entry keeps the `Rc` it registered to unregister it.
pub trait PopEntry {
    /// Called after a route pop was handled.
    fn on_pop_invoked_with_result(&self, app: &mut App, did_pop: bool, result: RouteResult);

    /// When false, blocks the current route from being popped.
    fn can_pop_notifier(&self, app: &App) -> Rc<dyn ValueListenable<bool>>;
}

/// The object side of [`PopEntry`]: an arena object a [`ModalRoute`] registers. Implementing it
/// makes `Handle<Self>` a [`PopEntry`].
pub trait PopEntryObject: Sized + 'static {
    /// See [`PopEntry::on_pop_invoked_with_result`].
    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    );

    /// See [`PopEntry::can_pop_notifier`].
    fn can_pop_notifier(self: Handle<Self>, app: &App) -> Rc<dyn ValueListenable<bool>>;
}

impl<T: PopEntryObject> PopEntry for Handle<T> {
    fn on_pop_invoked_with_result(&self, app: &mut App, did_pop: bool, result: RouteResult) {
        T::on_pop_invoked_with_result(*self, app, did_pop, result);
    }

    fn can_pop_notifier(&self, app: &App) -> Rc<dyn ValueListenable<bool>> {
        T::can_pop_notifier(*self, app)
    }
}

/// The fields of Dart's `RawDialogRoute`, which its subclasses carry.
pub struct RawDialogRouteData {
    /// Used to build the route's primary contents.
    pub page_builder: RoutePageBuilder,
    /// Whether touching the modal barrier pops the route; see
    /// [`ModalRoute::barrier_dismissible`].
    pub barrier_dismissible: bool,
    /// The semantic label of the modal barrier; see [`ModalRoute::barrier_label`].
    pub barrier_label: Option<String>,
    /// The color painted behind the dialog; see [`ModalRoute::barrier_color`].
    pub barrier_color: Option<Color>,
    /// How long the route takes to arrive on and leave off the screen; see
    /// [`TransitionRoute::transition_duration`].
    pub transition_duration: Duration,
    /// Used to define how the route arrives on and leaves off the screen.
    ///
    /// If `None`, the transition is a linear fade of the page's contents.
    pub transition_builder: Option<RouteTransitionsBuilder>,
    /// Used to define how the route's modal barrier is built.
    ///
    /// If not `None`, this builder is used to wrap or replace the default [`ModalBarrier`].
    pub barrier_builder: Option<RouteBarrierBuilder>,
    /// Whether this route is a full-screen dialog.
    ///
    /// In Material and Cupertino, being fullscreen has the effects of making the app bars have
    /// a close button instead of a back button.
    pub fullscreen_dialog: bool,
}

impl RawDialogRouteData {
    /// The bag of a freshly created dialog route; Dart's constructor defaults.
    pub fn new(page_builder: RoutePageBuilder) -> RawDialogRouteData {
        RawDialogRouteData {
            page_builder,
            barrier_dismissible: true,
            barrier_label: None,
            barrier_color: Some(Color::new(0x8000_0000)),
            transition_duration: Duration::from_millis(200),
            transition_builder: None,
            barrier_builder: None,
            fullscreen_dialog: false,
        }
    }
}

/// The accessors [`RawDialogRouteLeaf`] asks for, for a struct whose bag is the field
/// `raw_dialog_route`.
#[macro_export]
macro_rules! raw_dialog_route_accessors {
    () => {
        fn raw_dialog_route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RawDialogRouteData {
            &app.get(self).raw_dialog_route
        }

        fn raw_dialog_route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RawDialogRouteData {
            &mut app.get_mut(self).raw_dialog_route
        }
    };
}

/// A [`PopupRoute`] with the fields Dart's `RawDialogRoute` declares and the members it
/// overrides.
///
/// The shared bodies are associated functions on [`RawDialogRoute`]; an override calls one
/// where Dart writes `super.…`.
///
/// A leaf writes [`raw_dialog_route_modal_route_overrides!`](crate::raw_dialog_route_modal_route_overrides)
/// and [`raw_dialog_route_transition_route_overrides!`](crate::raw_dialog_route_transition_route_overrides)
/// to route the `ModalRoute` and `TransitionRoute` virtuals here.
pub trait RawDialogRouteLeaf: PopupRoute {
    /// Dart's `RawDialogRoute` fields, held under the field `raw_dialog_route`
    /// ([`raw_dialog_route_accessors!`](crate::raw_dialog_route_accessors)).
    fn raw_dialog_route_data(self: Handle<Self>, app: &App) -> &RawDialogRouteData;

    /// See [`raw_dialog_route_data`](Self::raw_dialog_route_data).
    fn raw_dialog_route_data_mut(self: Handle<Self>, app: &mut App) -> &mut RawDialogRouteData;

    /// Dart's `RawDialogRoute.barrierDismissible`.
    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        self.raw_dialog_route_data(app).barrier_dismissible
    }

    /// Dart's `RawDialogRoute.barrierLabel`.
    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        self.raw_dialog_route_data(app).barrier_label.clone()
    }

    /// Dart's `RawDialogRoute.barrierColor`.
    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        self.raw_dialog_route_data(app).barrier_color
    }

    /// Dart's `RawDialogRoute.transitionDuration`.
    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
        self.raw_dialog_route_data(app).transition_duration
    }

    /// Dart's `RawDialogRoute.fullscreenDialog`.
    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        self.raw_dialog_route_data(app).fullscreen_dialog
    }

    /// Dart's `RawDialogRoute.buildPage`.
    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        RawDialogRoute::build_page(self, app, context, animation, secondary_animation)
    }

    /// Dart's `RawDialogRoute.buildTransitions`.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        RawDialogRoute::build_transitions(self, app, context, animation, secondary_animation, child)
    }

    /// Dart's `RawDialogRoute.buildModalBarrier`.
    fn build_modal_barrier(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawDialogRoute::build_modal_barrier(self, app, context)
    }
}

/// The `ModalRoute` overrides a [`RawDialogRouteLeaf`] inherits, plus
/// [`modal_route_accessors!`](crate::modal_route_accessors). Write it inside
/// `impl ModalRoute for MyRoute { .. }`.
#[macro_export]
macro_rules! raw_dialog_route_modal_route_overrides {
    () => {
        $crate::modal_route_accessors!();

        fn barrier_dismissible(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::RawDialogRouteLeaf::barrier_dismissible(self, app)
        }

        fn barrier_label(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::std::string::String> {
            $crate::RawDialogRouteLeaf::barrier_label(self, app)
        }

        fn barrier_color(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::reveal_painting::Color> {
            $crate::RawDialogRouteLeaf::barrier_color(self, app)
        }

        fn maintain_state(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::PopupRoute::maintain_state(self, app)
        }

        fn fullscreen_dialog(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::RawDialogRouteLeaf::fullscreen_dialog(self, app)
        }

        fn build_page(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            context: $crate::BuildContext,
            animation: ::reveal_animation::AnyAnimation<f64>,
            secondary_animation: ::reveal_animation::AnyAnimation<f64>,
        ) -> $crate::WidgetRef {
            $crate::RawDialogRouteLeaf::build_page(
                self,
                app,
                context,
                animation,
                secondary_animation,
            )
        }

        fn build_transitions(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            context: $crate::BuildContext,
            animation: ::reveal_animation::AnyAnimation<f64>,
            secondary_animation: ::reveal_animation::AnyAnimation<f64>,
            child: $crate::WidgetRef,
        ) -> $crate::WidgetRef {
            $crate::RawDialogRouteLeaf::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            )
        }

        fn build_modal_barrier(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            context: $crate::BuildContext,
        ) -> $crate::WidgetRef {
            $crate::RawDialogRouteLeaf::build_modal_barrier(self, app, context)
        }
    };
}

/// The `TransitionRoute` overrides a [`RawDialogRouteLeaf`] inherits, plus
/// [`modal_transition_route_overrides!`](crate::modal_transition_route_overrides). Write it
/// inside `impl TransitionRoute for MyRoute { .. }`.
#[macro_export]
macro_rules! raw_dialog_route_transition_route_overrides {
    () => {
        $crate::modal_transition_route_overrides!();

        fn transition_duration(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::time::Duration {
            $crate::RawDialogRouteLeaf::transition_duration(self, app)
        }

        fn opaque(self: ::reveal_foundation::Handle<Self>, app: &::reveal_foundation::App) -> bool {
            $crate::PopupRoute::opaque(self, app)
        }

        fn allow_snapshotting(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            $crate::PopupRoute::allow_snapshotting(self, app)
        }
    };
}

/// A general dialog route which allows for customization of the dialog popup.
///
/// It is used internally by [`show_general_dialog`] or can be directly pushed onto the
/// [`Navigator`] stack to enable state restoration.
///
/// This route takes a [`page_builder`](RawDialogRouteData::page_builder), which typically
/// builds a dialog. Content below the dialog is dimmed with a [`ModalBarrier`].
///
/// It is also the base class `CupertinoDialogRoute` extends: its fields are the
/// [`RawDialogRouteData`] bag, the members it overrides are [`RawDialogRouteLeaf`]'s, and the
/// shared bodies are the associated functions below.
pub struct RawDialogRoute {
    route: crate::widgets::navigator::RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    raw_dialog_route: RawDialogRouteData,
}

impl RawDialogRoute {
    /// A general dialog route which allows for customization of the dialog popup; Dart's
    /// optional arguments are the setters.
    ///
    /// Dart's defaults: `barrier_dismissible` true, `barrier_color` `0x80000000`,
    /// `transition_duration` 200 ms, `fullscreen_dialog` false.
    pub fn new(app: &mut App, page_builder: RoutePageBuilder) -> Handle<RawDialogRoute> {
        let route = crate::widgets::navigator::RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        app.create(RawDialogRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            raw_dialog_route: RawDialogRouteData::new(page_builder),
        })
    }

    /// Dart `RawDialogRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `RawDialogRoute(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `RawDialogRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `RawDialogRoute(barrierColor:)`.
    pub fn barrier_color(
        self: Handle<Self>,
        app: &mut App,
        barrier_color: Option<Color>,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_color = barrier_color;
        self
    }

    /// Dart `RawDialogRoute(barrierLabel:)`.
    pub fn barrier_label(self: Handle<Self>, app: &mut App, barrier_label: String) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `RawDialogRoute(transitionDuration:)`.
    pub fn transition_duration(
        self: Handle<Self>,
        app: &mut App,
        transition_duration: Duration,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).transition_duration = transition_duration;
        self
    }

    /// Dart `RawDialogRoute(transitionBuilder:)`.
    pub fn transition_builder(
        self: Handle<Self>,
        app: &mut App,
        transition_builder: RouteTransitionsBuilder,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).transition_builder = Some(transition_builder);
        self
    }

    /// Dart `RawDialogRoute(barrierBuilder:)`.
    pub fn barrier_builder(
        self: Handle<Self>,
        app: &mut App,
        barrier_builder: RouteBarrierBuilder,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_builder = Some(barrier_builder);
        self
    }

    /// Dart `RawDialogRoute(traversalEdgeBehavior:)`.
    pub fn traversal_edge_behavior(
        self: Handle<Self>,
        app: &mut App,
        behavior: TraversalEdgeBehavior,
    ) -> Handle<Self> {
        self.modal_route_data_mut(app).traversal_edge_behavior = Some(behavior);
        self
    }

    /// Dart `RawDialogRoute(directionalTraversalEdgeBehavior:)`.
    pub fn directional_traversal_edge_behavior(
        self: Handle<Self>,
        app: &mut App,
        behavior: TraversalEdgeBehavior,
    ) -> Handle<Self> {
        self.modal_route_data_mut(app)
            .directional_traversal_edge_behavior = Some(behavior);
        self
    }

    /// Dart `RawDialogRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(
        self: Handle<Self>,
        app: &mut App,
        fullscreen_dialog: bool,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart's `RawDialogRoute.buildPage`, which a leaf's
    /// [`RawDialogRouteLeaf::build_page`] calls.
    pub fn build_page<R: RawDialogRouteLeaf>(
        this: Handle<R>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let page_builder = Rc::clone(&this.raw_dialog_route_data(app).page_builder);
        page_builder(app, context, animation, secondary_animation)
    }

    /// Dart's `RawDialogRoute.buildTransitions`, which a leaf's
    /// [`RawDialogRouteLeaf::build_transitions`] calls.
    pub fn build_transitions<R: RawDialogRouteLeaf>(
        this: Handle<R>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        match this.raw_dialog_route_data(app).transition_builder.clone() {
            // Some default transition.
            None => crate::widgets::transitions::FadeTransition::new(animation)
                .child(child)
                .into_widget(),
            Some(transition_builder) => {
                transition_builder(app, context, animation, secondary_animation, child)
            }
        }
    }

    /// Dart's `RawDialogRoute.buildModalBarrier`, which a leaf's
    /// [`RawDialogRouteLeaf::build_modal_barrier`] calls.
    pub fn build_modal_barrier<R: RawDialogRouteLeaf>(
        this: Handle<R>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetRef {
        let barrier = ModalRouteBase::build_modal_barrier(this, app, context);
        let Some(barrier_builder) = this.raw_dialog_route_data(app).barrier_builder.clone() else {
            return barrier;
        };
        Builder::new(move |app, context| {
            let animation = TransitionRoute::animation(this, app).expect("an installed route");
            let mut details =
                RouteBarrierDetails::new(animation, ModalRoute::barrier_dismissible(this, app));
            if let Some(color) = ModalRoute::barrier_color(this, app) {
                details = details.barrier_color(color);
            }
            if let Some(label) = ModalRoute::barrier_label(this, app) {
                details = details.barrier_label(label);
            }
            barrier_builder(app, context, &details, barrier.clone())
        })
        .into_widget()
    }
}

impl Route for RawDialogRoute {
    crate::modal_route_overrides!();
}

impl OverlayRoute for RawDialogRoute {
    crate::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for RawDialogRoute {
    crate::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for RawDialogRoute {
    crate::raw_dialog_route_transition_route_overrides!();
}

impl LocalHistoryRoute for RawDialogRoute {
    crate::local_history_route_accessors!();
}

impl ModalRoute for RawDialogRoute {
    crate::raw_dialog_route_modal_route_overrides!();
}

impl PopupRoute for RawDialogRoute {}

impl RawDialogRouteLeaf for RawDialogRoute {
    crate::raw_dialog_route_accessors!();
}

/// Displays a dialog above the current contents of the app.
///
/// This function allows for customization of aspects of the dialog popup.
///
/// The `context` argument is used to look up the [`Navigator`] for the dialog. It is only used
/// when the method is called. Its corresponding widget can be safely removed from the tree
/// before the dialog is closed.
///
/// The `use_root_navigator` argument is used to determine whether to push the dialog to the
/// [`Navigator`] furthest from or nearest to the given `context`.
///
/// The returned route is the pushed [`RawDialogRoute`]; register a callback on
/// [`AnyRoute::when_popped`] for the value (if any) that was passed to [`NavigatorState::pop`](crate::NavigatorState::pop)
/// when the dialog was closed.
///
/// See also:
///
///  * [`RawDialogRoute`], the route this pushes.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_general_dialog(
    app: &mut App,
    context: BuildContext,
    page_builder: RoutePageBuilder,
    barrier_dismissible: bool,
    barrier_label: Option<String>,
    barrier_color: Color,
    transition_duration: Duration,
    transition_builder: Option<RouteTransitionsBuilder>,
    barrier_builder: Option<RouteBarrierBuilder>,
    use_root_navigator: bool,
    fullscreen_dialog: bool,
    route_settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
) -> AnyRoute {
    debug_assert!(!barrier_dismissible || barrier_label.is_some());
    let route = RawDialogRoute::new(app, page_builder)
        .barrier_dismissible(app, barrier_dismissible)
        .barrier_color(app, Some(barrier_color))
        .transition_duration(app, transition_duration)
        .fullscreen_dialog(app, fullscreen_dialog);
    if let Some(barrier_label) = barrier_label {
        route.barrier_label(app, barrier_label);
    }
    if let Some(transition_builder) = transition_builder {
        route.transition_builder(app, transition_builder);
    }
    if let Some(barrier_builder) = barrier_builder {
        route.barrier_builder(app, barrier_builder);
    }
    if let Some(route_settings) = route_settings {
        route.settings(app, route_settings);
    }
    if let Some(request_focus) = request_focus {
        route.request_focus(app, request_focus);
    }
    let navigator = Navigator::of(app, context, use_root_navigator);
    navigator.push(app, route.as_route())
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::{Cell, RefCell};

    use reveal_embedder::{
        Offset, Picture, Platform, PlatformRef, PointerDeviceKind, TargetPlatform, TextDirection,
        View as EmbedderView, ViewConstraints, ViewId, ViewMetrics, ViewRef,
    };
    use reveal_gestures::{GestureBinding, PointerDownEvent, PointerEvent, PointerUpEvent};
    use reveal_painting::AlignmentGeometry;

    use super::*;
    use crate::binding::run_app;
    use crate::framework::{IntoWidget, StatelessWidget};
    use crate::widgets::basic::{Align, Directionality, SizedBox};
    use crate::widgets::navigator::{NavigatorState, RouteSettings};
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::pop_scope::PopScope;

    const VIEW_WIDTH: f64 = 300.0;
    const VIEW_HEIGHT: f64 = 200.0;

    struct TestView;

    impl EmbedderView for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics {
                physical_size: [VIEW_WIDTH, VIEW_HEIGHT],
                physical_constraints: ViewConstraints::tight(VIEW_WIDTH, VIEW_HEIGHT),
                device_pixel_ratio: 1.0,
                ..ViewMetrics::default()
            }
        }

        fn present(&self, _picture: &Picture) {}
    }

    struct TestPlatform {
        view: ViewRef,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }
    }

    /// An [`App`] with a single view; the navigator needs a binding for its global keys.
    fn app_with_view() -> Rc<AppCell> {
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::new(TestView),
        });
        AppCell::with_platform(platform)
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn mount(cell: &AppCell, child: WidgetRef) {
        run_app(&mut cell.borrow_mut(), child);
        cell.elapse(Duration::ZERO);
        pump_frame(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// Runs frames until every route transition has settled.
    fn settle(app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..40 {
            at += Duration::from_millis(20);
            pump_frame(app, at);
        }
        at
    }

    fn tap_at(app: &mut App, position: Offset) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                position,
                kind: PointerDeviceKind::Touch,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                position,
                kind: PointerDeviceKind::Touch,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    /// A page whose content is a 10x10 box in the top-left corner, so a tap in the middle of
    /// the view falls through to whatever is below it.
    fn corner_page(child: Option<WidgetRef>) -> RoutePageBuilder {
        Rc::new(move |_app, _context, _animation, _secondary| {
            let box_widget = match child.clone() {
                Some(child) => SizedBox::new().width(10.0).height(10.0).child(child),
                None => SizedBox::new().width(10.0).height(10.0),
            };
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(box_widget)
                .into_widget()
        })
    }

    fn page_route(app: &mut App, child: Option<WidgetRef>, name: &str) -> Handle<PageRouteBuilder> {
        PageRouteBuilder::new(app, corner_page(child))
            .settings(app, RouteSettings::new().name(name.to_string()).into())
    }

    /// A navigator whose initial route is a plain page, under a `Directionality` the overlay's
    /// theater needs.
    fn navigator(
        key: &GlobalKey,
        on_generate_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Navigator::new()
                .key(Rc::new(key.clone()))
                .on_generate_route(on_generate_route),
        )
        .into_widget()
    }

    fn state(key: &GlobalKey, app: &mut App) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(app)
            .expect("a mounted navigator")
    }

    /// A stateless widget that runs `read` on every build.
    type ProbeRead = Rc<dyn Fn(&mut App, BuildContext)>;

    struct Probe {
        read: ProbeRead,
    }

    impl Debug for Probe {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Probe")
        }
    }

    impl StatelessWidget for Probe {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            (self.read)(app, context);
            SizedBox::new().into_widget()
        }
    }

    fn probe(read: impl Fn(&mut App, BuildContext) + 'static) -> WidgetRef {
        Probe {
            read: Rc::new(read),
        }
        .into_widget()
    }

    // ---- the tests ----

    #[test]
    fn pushing_a_page_route_runs_its_transition_to_completion_and_popping_runs_it_back() {
        let cell = app_with_view();
        let key = GlobalKey::new();
        mount(
            &cell,
            navigator(&key, |app, _settings| {
                Some(page_route(app, None, "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let pushed = page_route(&mut app, None, "second").as_route();
        navigator_state.push(&mut app, pushed);
        pump_frame(&mut app, at);
        let animation = pushed
            .as_transition_route(&app)
            .expect("a page route is a transition route")
            .animation(&app)
            .expect("an installed route");
        assert!(animation.value(&app) < 1.0, "the transition starts at 0");

        let at = settle(&mut app, at);
        assert_eq!(animation.value(&app), 1.0);
        assert!(pushed.is_current(&app));
        assert!(!pushed.is_first(&app));

        let popped = Rc::new(Cell::new(false));
        pushed.when_popped(&mut app, {
            let popped = Rc::clone(&popped);
            Rc::new(move |_app, _result| popped.set(true))
        });
        navigator_state.pop(&mut app, None);
        app.drain_microtasks();
        assert!(popped.get(), "the popped completion resolves at once");

        settle(&mut app, at);
        assert_eq!(animation.value(&app), 0.0);
    }

    #[test]
    fn modal_route_of_reports_is_current_and_is_first() {
        let cell = app_with_view();
        let app = cell.borrow();
        let key = GlobalKey::new();
        let seen: Rc<RefCell<Vec<(bool, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let watcher = probe({
            let seen = Rc::clone(&seen);
            move |app, context| {
                let route = AnyModalRoute::of(app, context).expect("a modal route");
                let is_current = route.as_route().is_current(app);
                let is_first = route.as_route().is_first(app);
                seen.borrow_mut().push((is_current, is_first));
            }
        });
        drop(app);
        mount(
            &cell,
            navigator(&key, move |app, _settings| {
                Some(page_route(app, Some(watcher.clone()), "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        assert_eq!(seen.borrow().last().copied(), Some((true, true)));

        let second = page_route(&mut app, None, "second").as_route();
        navigator_state.push(&mut app, second);
        settle(&mut app, at);
        assert_eq!(
            seen.borrow().last().copied(),
            Some((false, true)),
            "the first route is no longer current once another is pushed"
        );
    }

    #[test]
    fn a_pop_scope_with_can_pop_false_blocks_maybe_pop() {
        let cell = app_with_view();
        let key = GlobalKey::new();
        let blocked = Rc::new(Cell::new(false));
        let scope = PopScope::new(SizedBox::new())
            .can_pop(false)
            .on_pop_invoked_with_result({
                let blocked = Rc::clone(&blocked);
                Rc::new(move |_app, did_pop, _result| blocked.set(!did_pop))
            })
            .into_widget();
        mount(
            &cell,
            navigator(&key, |app, _settings| {
                Some(page_route(app, None, "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let second = page_route(&mut app, Some(scope), "second").as_route();
        navigator_state.push(&mut app, second);
        let at = settle(&mut app, at);
        assert!(second.is_current(&app));

        assert!(
            navigator_state.maybe_pop(&mut app, None),
            "the pop is handled — by being refused — rather than bubbling"
        );
        settle(&mut app, at);
        assert!(second.is_current(&app), "the route is still on top");
        assert!(blocked.get(), "the pop was reported as not having happened");
    }

    #[test]
    fn tapping_the_barrier_dismisses_a_dismissible_popup_route() {
        let cell = app_with_view();
        let key = GlobalKey::new();
        mount(
            &cell,
            navigator(&key, |app, _settings| {
                Some(page_route(app, None, "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let dialog = RawDialogRoute::new(&mut app, corner_page(None))
            .barrier_dismissible(&mut app, true)
            .barrier_label(&mut app, "dismiss".to_string())
            .as_route();
        navigator_state.push(&mut app, dialog);
        let at = settle(&mut app, at);
        assert!(dialog.is_current(&app));

        // The dialog's own content is the 10x10 box in the corner, so the middle of the view
        // is the barrier.
        tap_at(&mut app, Offset::new(150.0, 100.0));
        pump_frame(&mut app, at);
        settle(&mut app, at);
        assert!(!dialog.is_active(&app), "the barrier popped the dialog");
    }

    #[test]
    fn a_non_dismissible_barrier_keeps_the_route() {
        let cell = app_with_view();
        let key = GlobalKey::new();
        mount(
            &cell,
            navigator(&key, |app, _settings| {
                Some(page_route(app, None, "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let dialog = RawDialogRoute::new(&mut app, corner_page(None))
            .barrier_dismissible(&mut app, false)
            .as_route();
        navigator_state.push(&mut app, dialog);
        let at = settle(&mut app, at);

        tap_at(&mut app, Offset::new(150.0, 100.0));
        pump_frame(&mut app, at);
        settle(&mut app, at);
        assert!(dialog.is_current(&app), "the tap did nothing");
    }

    #[test]
    fn local_history_entries_are_popped_before_the_route() {
        let cell = app_with_view();
        let key = GlobalKey::new();
        mount(
            &cell,
            navigator(&key, |app, _settings| {
                Some(page_route(app, None, "/").as_route())
            }),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let second = page_route(&mut app, None, "second");
        navigator_state.push(&mut app, second.as_route());
        let at = settle(&mut app, at);

        let removed = Rc::new(Cell::new(false));
        let entry = LocalHistoryEntry::new(&mut app).on_remove(&mut app, {
            let removed = Rc::clone(&removed);
            Listener::new(move |_app| removed.set(true))
        });
        LocalHistoryRoute::add_local_history_entry(second, &mut app, entry);
        assert!(Route::will_handle_pop_internally(second, &app));

        navigator_state.pop(&mut app, None);
        pump_frame(&mut app, at);
        assert!(removed.get(), "the local history entry was removed");
        assert!(
            second.as_route().is_current(&app),
            "the route itself is still on top"
        );

        navigator_state.pop(&mut app, None);
        settle(&mut app, at);
        assert!(!second.as_route().is_active(&app), "the route popped next");
    }

    #[test]
    fn a_route_observer_notifies_its_route_aware() {
        struct Watcher {
            pushed: Cell<usize>,
            pushed_next: Cell<usize>,
            popped: Cell<usize>,
            popped_next: Cell<usize>,
        }

        impl RouteAwareObject for Watcher {
            fn did_push(self: Handle<Self>, app: &mut App) {
                let count = app.get(self).pushed.get();
                app.get(self).pushed.set(count + 1);
            }

            fn did_push_next(self: Handle<Self>, app: &mut App) {
                let count = app.get(self).pushed_next.get();
                app.get(self).pushed_next.set(count + 1);
            }

            fn did_pop(self: Handle<Self>, app: &mut App) {
                let count = app.get(self).popped.get();
                app.get(self).popped.set(count + 1);
            }

            fn did_pop_next(self: Handle<Self>, app: &mut App) {
                let count = app.get(self).popped_next.get();
                app.get(self).popped_next.set(count + 1);
            }
        }

        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let observer = RouteObserver::new(
            &mut app,
            Rc::new(|app, route: AnyRoute| route.as_page_route(app).is_some()),
        );
        let initial: Rc<Cell<Option<AnyRoute>>> = Rc::new(Cell::new(None));
        drop(app);
        mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Navigator::new()
                    .key(Rc::new(key.clone()))
                    .on_generate_route({
                        let initial = Rc::clone(&initial);
                        move |app, _settings| {
                            let route = page_route(app, None, "/").as_route();
                            initial.set(Some(route));
                            Some(route)
                        }
                    })
                    .observers(vec![observer.as_observer()]),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let first = initial.get().expect("the initial route");
        let watcher = app.create(Watcher {
            pushed: Cell::new(0),
            pushed_next: Cell::new(0),
            popped: Cell::new(0),
            popped_next: Cell::new(0),
        });
        observer.subscribe(&mut app, Rc::new(watcher), first);
        assert_eq!(app.get(watcher).pushed.get(), 1);

        let second = page_route(&mut app, None, "second").as_route();
        navigator_state.push(&mut app, second);
        let at = settle(&mut app, at);
        assert_eq!(app.get(watcher).pushed_next.get(), 1);

        navigator_state.pop(&mut app, None);
        settle(&mut app, at);
        assert_eq!(app.get(watcher).popped_next.get(), 1);
        assert_eq!(app.get(watcher).popped.get(), 0);
    }
}

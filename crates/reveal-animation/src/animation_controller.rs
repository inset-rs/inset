//! Flutter counterpart: `animation/animation_controller.dart`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_geometry::{clamp_double, lerp_double};
use reveal_physics::{Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance};
use reveal_scheduler::{FrameCallback, Ticker, TickerFuture, TickerProvider};

use crate::animation::{Animation, AnimationNode, AnimationStatus, AnimationStatusListener};
use crate::curves::Curve;
use crate::listener_helpers::{
    AnimationEagerListenerMixin, AnimationLocalListenersData, AnimationLocalListenersMixin,
    AnimationLocalStatusListenersData, AnimationLocalStatusListenersMixin,
};
use crate::tween::Animatable;

/// Dart's private `_AnimationDirection`: the direction in which an animation
/// is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationDirection {
    /// The animation is running from beginning to end.
    Forward,

    /// The animation is running backwards, from end to beginning.
    Reverse,
}

/// Dart's `final _kFlingSpringDescription` — computed, so a function rather
/// than a constant.
fn k_fling_spring_description() -> SpringDescription {
    SpringDescription::with_damping_ratio(1.0, 500.0)
}

const K_FLING_TOLERANCE: Tolerance = Tolerance {
    velocity: f64::INFINITY,
    distance: 0.01,
    time: 1e-3, // Tolerance's default, spelled out — Rust has no partial const
};

/// Configures how an [`AnimationController`] behaves when animations are
/// disabled.
///
/// When accessibility features such as "remove animations" are enabled, the
/// device asks Flutter to reduce or disable animations as much as possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationBehavior {
    /// The [`AnimationController`] will reduce its duration when animations
    /// are disabled by the platform.
    Normal,

    /// The [`AnimationController`] will preserve its behavior.
    Preserve,
}

impl AnimationBehavior {
    /// Dart's `_enableAnimations` reads
    /// `SemanticsBinding.instance.disableAnimations`; the semantics binding
    /// is not ported, so animations are always enabled. See `PORTING.md`.
    fn enable_animations(self) -> bool {
        match self {
            AnimationBehavior::Normal => true,
            AnimationBehavior::Preserve => true,
        }
    }
}

/// A controller for an animation.
///
/// This class lets you perform tasks such as:
///
/// * Play an animation forward or in reverse, or stop an animation.
/// * Set the animation to a specific value.
/// * Define the upper and lower bounds of an animation.
/// * Create a fling animation effect using a physics simulation.
///
/// By default, an [`AnimationController`] linearly produces values that range
/// from 0.0 to 1.0, during a given duration.
///
/// An [`AnimationController`] needs a `TickerProvider`, Dart's `vsync`
/// constructor argument.
#[derive(Clone, Copy)]
pub struct AnimationController(Handle<AnimationControllerData>);

struct AnimationControllerData {
    /// The value at which this animation is deemed to be dismissed.
    pub lower_bound: f64,

    /// The value at which this animation is deemed to be completed.
    pub upper_bound: f64,

    /// The behavior of the controller when accessibility features disable
    /// animations.
    pub animation_behavior: AnimationBehavior,

    /// The length of time this animation should last.
    ///
    /// If [`reverse_duration`](AnimationController::reverse_duration) is
    /// specified, then [`duration`](AnimationController::duration) is only
    /// used when going forward. Otherwise, it specifies the duration going in
    /// both directions.
    pub duration: Option<Duration>,

    /// The length of time this animation should last when going in reverse.
    ///
    /// The value of [`duration`](AnimationController::duration) is used if
    /// this is `None`.
    pub reverse_duration: Option<Duration>,

    ticker: Option<Ticker>,
    simulation: Option<Rc<dyn Simulation>>,
    value: f64,
    last_elapsed_duration: Option<Duration>,
    direction: AnimationDirection,
    status: AnimationStatus,
    last_reported_status: AnimationStatus,

    // Dart's `_directionSetter` tear-off, which `_RepeatingSimulation.x`
    // calls from a context with no App: the setter writes this detached cell
    // and the controller applies it immediately after each `x()` returns,
    // before the value assignment — Dart's observable order.
    pending_direction: Rc<Cell<Option<AnimationDirection>>>,

    // Mixin state: `AnimationEagerListenerMixin` (stateless),
    // `AnimationLocalListenersMixin`, `AnimationLocalStatusListenersMixin`.
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl AnimationController {
    /// Creates an animation controller.
    ///
    /// `value` is the initial value of the animation; `None` means
    /// `lower_bound`. `vsync` is the required `TickerProvider`. Dart's
    /// defaults: `lower_bound` 0.0, `upper_bound` 1.0, `animation_behavior`
    /// `Normal`; `debugLabel` is not ported.
    ///
    /// Dart's constructor registers the `_tick` tear-off with the vended
    /// ticker, so creation goes through the [`App`].
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        app: &mut App,
        value: Option<f64>,
        duration: Option<Duration>,
        reverse_duration: Option<Duration>,
        lower_bound: f64,
        upper_bound: f64,
        animation_behavior: AnimationBehavior,
        vsync: impl TickerProvider,
    ) -> AnimationController {
        debug_assert!(upper_bound >= lower_bound);
        let this = AnimationController(app.create(AnimationControllerData {
            lower_bound,
            upper_bound,
            animation_behavior,
            duration,
            reverse_duration,
            ticker: None,
            simulation: None,
            value: 0.0,
            last_elapsed_duration: None,
            direction: AnimationDirection::Forward,
            status: AnimationStatus::Dismissed,
            last_reported_status: AnimationStatus::Dismissed,
            pending_direction: Rc::new(Cell::new(None)),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }));
        let ticker = vsync.create_ticker(
            app,
            FrameCallback::handle_method(this.0, animation_controller_tick),
        );
        app.get_mut(this.0).ticker = Some(ticker);
        this.internal_set_value(app, value.unwrap_or(lower_bound));
        this
    }

    /// Creates an animation controller with no upper or lower bound for its
    /// value.
    ///
    /// Dart's defaults: `value` 0.0, `animation_behavior` `Preserve`.
    pub fn create_unbounded(
        app: &mut App,
        value: f64,
        duration: Option<Duration>,
        reverse_duration: Option<Duration>,
        animation_behavior: AnimationBehavior,
        vsync: impl TickerProvider,
    ) -> AnimationController {
        let this = AnimationController(app.create(AnimationControllerData {
            lower_bound: f64::NEG_INFINITY,
            upper_bound: f64::INFINITY,
            animation_behavior,
            duration,
            reverse_duration,
            ticker: None,
            simulation: None,
            value: 0.0,
            last_elapsed_duration: None,
            direction: AnimationDirection::Forward,
            status: AnimationStatus::Dismissed,
            last_reported_status: AnimationStatus::Dismissed,
            pending_direction: Rc::new(Cell::new(None)),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }));
        let ticker = vsync.create_ticker(
            app,
            FrameCallback::handle_method(this.0, animation_controller_tick),
        );
        app.get_mut(this.0).ticker = Some(ticker);
        this.internal_set_value(app, value);
        this
    }

    /// The current value of the animation.
    pub fn value(self, app: &App) -> f64 {
        app.get(self.0).value
    }

    /// The current status of this animation.
    pub fn status(self, app: &App) -> AnimationStatus {
        app.get(self.0).status
    }

    /// The amount of time that has passed between the time the animation
    /// started and the most recent tick of the animation.
    ///
    /// If the controller is not animating, the last elapsed duration is
    /// `None`.
    pub fn last_elapsed_duration(self, app: &App) -> Option<Duration> {
        app.get(self.0).last_elapsed_duration
    }
}

impl AnimationController {
    fn internal_set_value(self, app: &mut App, new_value: f64) {
        let controller = app.get_mut(self.0);
        controller.value = clamp_double(new_value, controller.lower_bound, controller.upper_bound);
        if controller.value == controller.lower_bound {
            controller.status = AnimationStatus::Dismissed;
        } else if controller.value == controller.upper_bound {
            controller.status = AnimationStatus::Completed;
        } else {
            controller.status = match controller.direction {
                AnimationDirection::Forward => AnimationStatus::Forward,
                AnimationDirection::Reverse => AnimationStatus::Reverse,
            };
        }
    }

    fn animate_to_internal(
        self,
        app: &mut App,
        target: f64,
        duration: Option<Duration>,
        curve: Rc<dyn Curve>,
    ) -> TickerFuture {
        let controller = app.get(self.0);
        let scale = if controller.animation_behavior.enable_animations() {
            1.0
        } else {
            0.05
        };
        let mut simulation_duration = duration;
        if simulation_duration.is_none() {
            debug_assert!(
                !(controller.duration.is_none()
                    && controller.direction == AnimationDirection::Forward)
            );
            debug_assert!(
                !(controller.duration.is_none()
                    && controller.direction == AnimationDirection::Reverse
                    && controller.reverse_duration.is_none())
            );
            let range = controller.upper_bound - controller.lower_bound;
            let remaining_fraction = if range.is_finite() {
                (target - controller.value).abs() / range
            } else {
                1.0
            };
            // Dart: `(_direction == reverse && reverseDuration != null)
            // ? reverseDuration! : this.duration!`.
            let direction_duration = match (controller.direction, controller.reverse_duration) {
                (AnimationDirection::Reverse, Some(reverse_duration)) => reverse_duration,
                _ => controller.duration.unwrap(),
            };
            simulation_duration = Some(direction_duration.mul_f64(remaining_fraction));
        } else if target == controller.value {
            simulation_duration = Some(Duration::ZERO);
        }
        let simulation_duration = simulation_duration.unwrap();

        self.stop(app, true);
        if simulation_duration == Duration::ZERO {
            let controller = app.get_mut(self.0);
            if controller.value != target {
                controller.value =
                    clamp_double(target, controller.lower_bound, controller.upper_bound);
                self.notify_listeners(app);
            }
            let controller = app.get_mut(self.0);
            controller.status = if controller.direction == AnimationDirection::Forward {
                AnimationStatus::Completed
            } else {
                AnimationStatus::Dismissed
            };
            self.check_status_changed(app);
            return TickerFuture::complete(app);
        }
        debug_assert!(simulation_duration > Duration::ZERO);
        debug_assert!(!self.is_animating(app));
        let value = app.get(self.0).value;
        self.start_simulation(
            app,
            Rc::new(InterpolationSimulation::new(
                value,
                target,
                simulation_duration,
                curve,
                scale,
            )),
        )
    }

    fn direction_setter(self, app: &mut App, direction: AnimationDirection) {
        let controller = app.get_mut(self.0);
        controller.direction = direction;
        controller.status = if direction == AnimationDirection::Forward {
            AnimationStatus::Forward
        } else {
            AnimationStatus::Reverse
        };
        self.check_status_changed(app);
    }

    fn drain_pending_direction(self, app: &mut App) {
        if let Some(direction) = app.get(self.0).pending_direction.take() {
            self.direction_setter(app, direction);
        }
    }

    fn start_simulation(self, app: &mut App, simulation: Rc<dyn Simulation>) -> TickerFuture {
        debug_assert!(!self.is_animating(app));
        let x = simulation.x(0.0);
        // The repeating simulation's direction setter fires during `x(0.0)`;
        // Dart runs it inside that call, before the value assignment.
        {
            let controller = app.get_mut(self.0);
            controller.simulation = Some(simulation);
            controller.last_elapsed_duration = Some(Duration::ZERO);
        }
        self.drain_pending_direction(app);
        let controller = app.get_mut(self.0);
        controller.value = clamp_double(x, controller.lower_bound, controller.upper_bound);
        let ticker = controller.ticker.expect("started after dispose");
        let result = ticker.start(app);
        let controller = app.get_mut(self.0);
        controller.status = if controller.direction == AnimationDirection::Forward {
            AnimationStatus::Forward
        } else {
            AnimationStatus::Reverse
        };
        self.check_status_changed(app);
        result
    }

    fn check_status_changed(self, app: &mut App) {
        let controller = app.get_mut(self.0);
        let new_status = controller.status;
        if controller.last_reported_status != new_status {
            controller.last_reported_status = new_status;
            self.notify_status_listeners(app, new_status);
        }
    }

    fn tick(self, app: &mut App, elapsed: Duration) {
        app.get_mut(self.0).last_elapsed_duration = Some(elapsed);
        let elapsed_in_seconds = elapsed.as_micros() as f64 / 1_000_000.0;
        debug_assert!(elapsed_in_seconds >= 0.0);
        let simulation = app.get(self.0).simulation.clone().unwrap();
        let x = simulation.x(elapsed_in_seconds);
        // The repeating simulation's direction setter fires during `x()`;
        // Dart runs it inside that call, before the value assignment.
        self.drain_pending_direction(app);
        let controller = app.get_mut(self.0);
        controller.value = clamp_double(x, controller.lower_bound, controller.upper_bound);
        // Dart re-derefs `_simulation!` here, so a status listener that
        // replaced the simulation is consulted, and one that stopped the
        // controller crashes the tick — the panic is that crash.
        let simulation = app
            .get(self.0)
            .simulation
            .clone()
            .expect("a listener stopped the controller during its own tick");
        if simulation.is_done(elapsed_in_seconds) {
            let controller = app.get_mut(self.0);
            controller.status = if controller.direction == AnimationDirection::Forward {
                AnimationStatus::Completed
            } else {
                AnimationStatus::Dismissed
            };
            self.stop(app, false);
        }
        self.notify_listeners(app);
        self.check_status_changed(app);
    }
}

use crate::curves::Curves;

impl AnimationEagerListenerMixin for AnimationController {}

impl AnimationLocalListenersMixin for AnimationController {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self.0).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self.0).local_listeners
    }

    // Dart resolves these by mixin order — `AnimationEagerListenerMixin`.
    fn did_register_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for AnimationController {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self.0).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self.0).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

// Dart: `class AnimationController extends Animation<double> with
// AnimationEagerListenerMixin, AnimationLocalListenersMixin,
// AnimationLocalStatusListenersMixin`.
impl AnimationNode<f64> for AnimationControllerData {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(AnimationController(this), app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(AnimationController(this), app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(
            AnimationController(this),
            app,
            listener,
        )
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(
            AnimationController(this),
            app,
            listener,
        )
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        app.get(this).status
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        app.get(this).value
    }

    // Dart overrides `isAnimating` away from the status-derived default
    // (`animation_controller.dart:449`): a muted or off-screen controller is
    // animating even while its status says otherwise.
    fn is_animating(app: &App, this: Handle<Self>) -> bool {
        match app.get(this).ticker {
            Some(ticker) => ticker.is_active(app),
            None => false,
        }
    }
}

fn animation_controller_tick(
    this: Handle<AnimationControllerData>,
    app: &mut App,
    elapsed: Duration,
) {
    AnimationController(this).tick(app, elapsed);
}

/// Dart's private `_InterpolationSimulation`.
struct InterpolationSimulation {
    duration_in_seconds: f64,
    begin: f64,
    end: f64,
    curve: Rc<dyn Curve>,
    tolerance: Tolerance,
}

impl InterpolationSimulation {
    fn new(
        begin: f64,
        end: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        scale: f64,
    ) -> InterpolationSimulation {
        debug_assert!(duration.as_micros() > 0);
        InterpolationSimulation {
            duration_in_seconds: (duration.as_micros() as f64 * scale) / 1_000_000.0,
            begin,
            end,
            curve,
            tolerance: Tolerance::DEFAULT_TOLERANCE,
        }
    }
}

impl Simulation for InterpolationSimulation {
    fn x(&self, time_in_seconds: f64) -> f64 {
        let t = clamp_double(time_in_seconds / self.duration_in_seconds, 0.0, 1.0);
        if t == 0.0 {
            self.begin
        } else if t == 1.0 {
            self.end
        } else {
            self.begin + (self.end - self.begin) * self.curve.transform(t)
        }
    }

    fn dx(&self, time_in_seconds: f64) -> f64 {
        let epsilon = self.tolerance.time;
        (self.x(time_in_seconds + epsilon) - self.x(time_in_seconds - epsilon)) / (2.0 * epsilon)
    }

    fn is_done(&self, time_in_seconds: f64) -> bool {
        time_in_seconds > self.duration_in_seconds
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl std::fmt::Debug for InterpolationSimulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("_InterpolationSimulation")
    }
}

/// Dart's private `_RepeatingSimulation`.
///
/// Dart's `directionSetter` is a tear-off of the controller's
/// `_directionSetter`, called from `x()` — a context with no App here. The
/// setter's counterpart is the shared `pending_direction` cell, which the
/// controller drains immediately after each `x()` call.
struct RepeatingSimulation {
    min: f64,
    max: f64,
    reverse: bool,
    count: Option<u64>,
    direction_setter: Rc<Cell<Option<AnimationDirection>>>,
    period_in_seconds: f64,
    initial_t: f64,
    exit_time_in_seconds: Option<f64>,
    tolerance: Tolerance,
}

impl RepeatingSimulation {
    fn new(
        initial_value: f64,
        min: f64,
        max: f64,
        reverse: bool,
        period: Duration,
        direction_setter: Rc<Cell<Option<AnimationDirection>>>,
        count: Option<u64>,
    ) -> RepeatingSimulation {
        debug_assert!(count.is_none_or(|count| count > 0));
        let period_in_seconds = period.as_micros() as f64 / 1_000_000.0;
        let initial_t = if max == min {
            0.0
        } else {
            ((clamp_double(initial_value, min, max) - min) / (max - min)) * period_in_seconds
        };
        debug_assert!(period_in_seconds > 0.0);
        debug_assert!(initial_t >= 0.0);
        // Dart's `late final _exitTimeInSeconds` dereferences `count!` and is
        // only read when `count != null`.
        let exit_time_in_seconds =
            count.map(|count| (count as f64 * period_in_seconds) - initial_t);
        RepeatingSimulation {
            min,
            max,
            reverse,
            count,
            direction_setter,
            period_in_seconds,
            initial_t,
            exit_time_in_seconds,
            tolerance: Tolerance::DEFAULT_TOLERANCE,
        }
    }
}

impl Simulation for RepeatingSimulation {
    fn x(&self, time_in_seconds: f64) -> f64 {
        debug_assert!(time_in_seconds >= 0.0);
        let total_time_in_seconds = time_in_seconds + self.initial_t;
        let t = (total_time_in_seconds / self.period_in_seconds) % 1.0;
        // Dart: `(total ~/ period).isOdd` — truncating integer division.
        let is_playing_reverse =
            (total_time_in_seconds / self.period_in_seconds).trunc() as i64 % 2 != 0;
        if self.reverse && is_playing_reverse {
            self.direction_setter.set(Some(AnimationDirection::Reverse));
            // Dart: `ui.lerpDouble(max, min, t)!`.
            lerp_double(Some(self.max), Some(self.min), t).unwrap()
        } else {
            self.direction_setter.set(Some(AnimationDirection::Forward));
            lerp_double(Some(self.min), Some(self.max), t).unwrap()
        }
    }

    fn dx(&self, _time_in_seconds: f64) -> f64 {
        (self.max - self.min) / self.period_in_seconds
    }

    fn is_done(&self, time_in_seconds: f64) -> bool {
        self.count.is_some() && time_in_seconds >= self.exit_time_in_seconds.unwrap()
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl std::fmt::Debug for RepeatingSimulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("_RepeatingSimulation")
    }
}

impl AnimationController {
    /// An [`Animation<f64>`] view of this controller — what to pass where a
    /// Dart API wants an `Animation<double>`. Dart's `view` getter returns
    /// `this`; the erased handle is that upcast.
    pub fn view(self) -> Animation<f64> {
        Animation::from_handle(self.0)
    }

    /// Chains a `Tween` (or any [`Animatable`]) to this controller.
    ///
    /// Dart inherits `drive` from `Animation<double>`. The newtype is not a
    /// subtype, so the method is inherent and forwards to [`view`].
    pub fn drive<U: 'static>(
        self,
        app: &mut App,
        child: impl Animatable<U> + Clone + 'static,
    ) -> Animation<U> {
        self.view().drive(app, child)
    }

    /// Recreates the [`Ticker`] with the new `TickerProvider`.
    pub fn resync(self, app: &mut App, vsync: impl TickerProvider) {
        let old_ticker = app.get(self.0).ticker.expect("resync after dispose");
        let ticker = vsync.create_ticker(
            app,
            FrameCallback::handle_method(self.0, animation_controller_tick),
        );
        app.get_mut(self.0).ticker = Some(ticker);
        ticker.absorb_ticker(app, old_ticker);
    }

    /// Stops the animation controller and sets the current value of the
    /// animation.
    pub fn set_value(self, app: &mut App, new_value: f64) {
        self.stop(app, true);
        self.internal_set_value(app, new_value);
        self.notify_listeners(app);
        self.check_status_changed(app);
    }

    /// Sets the controller's value to the lower bound, stopping the animation.
    pub fn reset(self, app: &mut App) {
        let lower_bound = app.get(self.0).lower_bound;
        self.set_value(app, lower_bound);
    }

    /// The rate of change of [`AnimationController::value`] per second.
    pub fn velocity(self, app: &mut App) -> f64 {
        if !self.is_animating(app) {
            return 0.0;
        }
        let controller = app.get(self.0);
        let elapsed = controller.last_elapsed_duration.unwrap();
        controller
            .simulation
            .as_ref()
            .unwrap()
            .dx(elapsed.as_micros() as f64 / 1_000_000.0)
    }

    /// Whether this animation is currently animating in either the forward or
    /// reverse direction.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_animating(self, app: &mut App) -> bool {
        // One body: Dart has a single isAnimating override, and it lives in
        // the AnimationNode impl so the erased handle dispatches it too.
        <AnimationControllerData as AnimationNode<f64>>::is_animating(app, self.0)
    }

    /// Starts running this animation forwards (towards the end).
    pub fn forward(self, app: &mut App, from: Option<f64>) -> TickerFuture {
        debug_assert!(
            app.get(self.0).duration.is_some(),
            "AnimationController::forward() called with no default duration. The \"duration\" \
             property should be set, either in the constructor or later, before calling forward()."
        );
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::forward() called after AnimationController::dispose()."
        );
        app.get_mut(self.0).direction = AnimationDirection::Forward;
        if let Some(from) = from {
            self.set_value(app, from);
        }
        let upper_bound = app.get(self.0).upper_bound;
        self.animate_to_internal(app, upper_bound, None, Curves::linear())
    }

    /// Starts running this animation in reverse (towards the beginning).
    pub fn reverse(self, app: &mut App, from: Option<f64>) -> TickerFuture {
        debug_assert!(
            app.get(self.0).duration.is_some() || app.get(self.0).reverse_duration.is_some(),
            "AnimationController::reverse() called with no default duration or reverseDuration."
        );
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::reverse() called after AnimationController::dispose()."
        );
        app.get_mut(self.0).direction = AnimationDirection::Reverse;
        if let Some(from) = from {
            self.set_value(app, from);
        }
        let lower_bound = app.get(self.0).lower_bound;
        self.animate_to_internal(app, lower_bound, None, Curves::linear())
    }

    /// Toggles the direction of this animation.
    pub fn toggle(self, app: &mut App, from: Option<f64>) -> TickerFuture {
        debug_assert!(
            {
                let controller = app.get(self.0);
                let mut duration = controller.duration;
                if controller.status.is_forward_or_completed() {
                    duration = duration.or(controller.reverse_duration);
                }
                duration.is_some()
            },
            "AnimationController::toggle() called with no default duration."
        );
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::toggle() called after AnimationController::dispose()."
        );
        let forward_or_completed = app.get(self.0).status.is_forward_or_completed();
        app.get_mut(self.0).direction = if forward_or_completed {
            AnimationDirection::Reverse
        } else {
            AnimationDirection::Forward
        };
        if let Some(from) = from {
            self.set_value(app, from);
        }
        let target = match app.get(self.0).direction {
            AnimationDirection::Forward => app.get(self.0).upper_bound,
            AnimationDirection::Reverse => app.get(self.0).lower_bound,
        };
        self.animate_to_internal(app, target, None, Curves::linear())
    }

    /// Drives the animation from its current value to `target`.
    pub fn animate_to(
        self,
        app: &mut App,
        target: f64,
        duration: Option<Duration>,
        curve: Rc<dyn Curve>,
    ) -> TickerFuture {
        debug_assert!(
            app.get(self.0).duration.is_some() || duration.is_some(),
            "AnimationController::animate_to() called with no explicit duration and no default \
             duration."
        );
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::animate_to() called after AnimationController::dispose()."
        );
        app.get_mut(self.0).direction = AnimationDirection::Forward;
        self.animate_to_internal(app, target, duration, curve)
    }

    /// Drives the animation from its current value to `target`, going in reverse.
    pub fn animate_back(
        self,
        app: &mut App,
        target: f64,
        duration: Option<Duration>,
        curve: Rc<dyn Curve>,
    ) -> TickerFuture {
        debug_assert!(
            app.get(self.0).duration.is_some()
                || app.get(self.0).reverse_duration.is_some()
                || duration.is_some(),
            "AnimationController::animate_back() called with no explicit duration and no default \
             duration or reverseDuration."
        );
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::animate_back() called after AnimationController::dispose()."
        );
        app.get_mut(self.0).direction = AnimationDirection::Reverse;
        self.animate_to_internal(app, target, duration, curve)
    }

    /// Starts running this animation in the forward direction, and restarts
    /// the animation when it completes.
    pub fn repeat(
        self,
        app: &mut App,
        min: Option<f64>,
        max: Option<f64>,
        reverse: bool,
        period: Option<Duration>,
        count: Option<u64>,
    ) -> TickerFuture {
        let controller = app.get(self.0);
        let min = min.unwrap_or(controller.lower_bound);
        let max = max.unwrap_or(controller.upper_bound);
        let period = period.or(controller.duration);
        debug_assert!(
            period.is_some(),
            "AnimationController::repeat() called without an explicit period and with no default \
             Duration."
        );
        debug_assert!(max >= min);
        debug_assert!(max <= controller.upper_bound && min >= controller.lower_bound);
        debug_assert!(count.is_none_or(|count| count > 0));
        self.stop(app, true);
        let (value, pending_direction) = {
            let controller = app.get(self.0);
            (controller.value, Rc::clone(&controller.pending_direction))
        };
        self.start_simulation(
            app,
            Rc::new(RepeatingSimulation::new(
                value,
                min,
                max,
                reverse,
                period.unwrap(),
                pending_direction,
                count,
            )),
        )
    }

    /// Drives the animation with a spring and initial `velocity`.
    pub fn fling(
        self,
        app: &mut App,
        velocity: f64,
        spring_description: Option<SpringDescription>,
        animation_behavior: Option<AnimationBehavior>,
    ) -> TickerFuture {
        let spring_description = spring_description.unwrap_or_else(k_fling_spring_description);
        let controller = app.get_mut(self.0);
        controller.direction = if velocity < 0.0 {
            AnimationDirection::Reverse
        } else {
            AnimationDirection::Forward
        };
        let target = if velocity < 0.0 {
            controller.lower_bound - K_FLING_TOLERANCE.distance
        } else {
            controller.upper_bound + K_FLING_TOLERANCE.distance
        };
        let behavior = animation_behavior.unwrap_or(controller.animation_behavior);
        let scale = if behavior.enable_animations() {
            1.0
        } else {
            200.0
        };
        let mut simulation = SpringSimulation::new(
            spring_description,
            controller.value,
            target,
            velocity * scale,
        );
        simulation.set_tolerance(K_FLING_TOLERANCE);
        debug_assert!(
            simulation.spring_type() != SpringType::UnderDamped,
            "The specified spring simulation is of type SpringType.underDamped. An underdamped \
             spring results in oscillation rather than a fling."
        );
        self.stop(app, true);
        self.start_simulation(app, Rc::new(simulation))
    }

    /// Drives the animation according to the given simulation.
    pub fn animate_with(self, app: &mut App, simulation: Box<dyn Simulation>) -> TickerFuture {
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::animate_with() called after AnimationController::dispose()."
        );
        self.stop(app, true);
        app.get_mut(self.0).direction = AnimationDirection::Forward;
        self.start_simulation(app, Rc::from(simulation))
    }

    /// Like [`animate_with`], but the status is reported as [`AnimationStatus::Reverse`].
    pub fn animate_back_with(self, app: &mut App, simulation: Box<dyn Simulation>) -> TickerFuture {
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::animate_back_with() called after AnimationController::dispose()."
        );
        self.stop(app, true);
        app.get_mut(self.0).direction = AnimationDirection::Reverse;
        self.start_simulation(app, Rc::from(simulation))
    }

    /// Stops running this animation.
    pub fn stop(self, app: &mut App, canceled: bool) {
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::stop() called after AnimationController::dispose()."
        );
        let controller = app.get_mut(self.0);
        controller.simulation = None;
        controller.last_elapsed_duration = None;
        let ticker = controller.ticker.unwrap();
        ticker.stop(app, canceled);
    }

    /// Release the resources used by this object. The object is no longer
    /// usable after this method is called.
    pub fn dispose(self, app: &mut App) {
        debug_assert!(
            app.get(self.0).ticker.is_some(),
            "AnimationController::dispose() called more than once."
        );
        let ticker = app.get_mut(self.0).ticker.take().unwrap();
        ticker.dispose(app);
        self.clear_status_listeners(app);
        self.clear_listeners(app);
        AnimationEagerListenerMixin::dispose(self, app);
    }
}

impl Listenable for AnimationController {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        self.view().add_listener(app, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        self.view().remove_listener(app, listener);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use reveal_scheduler::{SchedulerBinding, TickerCallback};

    use super::*;

    fn assert_future_completed(app: &mut App, future: TickerFuture) {
        let ran = Rc::new(Cell::new(false));
        future.when_complete(
            app,
            Listener::new({
                let ran = Rc::clone(&ran);
                move |_app| ran.set(true)
            }),
        );
        app.drain_microtasks();
        assert!(ran.get(), "TickerFuture should have completed");
    }

    fn assert_future_canceled(app: &mut App, future: TickerFuture) {
        let completed = Rc::new(Cell::new(false));
        let either = Rc::new(Cell::new(false));
        future.when_complete(
            app,
            Listener::new({
                let completed = Rc::clone(&completed);
                move |_app| completed.set(true)
            }),
        );
        future.when_complete_or_cancel(
            app,
            Listener::new({
                let either = Rc::clone(&either);
                move |_app| either.set(true)
            }),
        );
        app.drain_microtasks();
        assert!(!completed.get(), "primary future never resolves on cancel");
        assert!(either.get(), "when_complete_or_cancel observes cancel");
    }

    /// flutter_test's `TestVSync`: vends ordinary tickers.
    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(self, app: &mut App, on_tick: TickerCallback) -> Ticker {
            Ticker::new(app, on_tick)
        }
    }

    fn pump(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn controller_with_duration(app: &mut App, ms: u64) -> AnimationController {
        AnimationController::create(
            app,
            None,
            Some(Duration::from_millis(ms)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    // animation_controller_test.dart 'Can set value during status callback'
    // family reduced to the core drive: forward moves the value linearly and
    // ends Completed.
    #[test]
    fn forward_drives_the_value_to_the_upper_bound() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        controller.forward(&mut app, None);
        assert!(controller.is_animating(&mut app));
        assert_eq!(controller.status(&app), AnimationStatus::Forward);

        pump(&mut app, Duration::from_millis(0)); // first tick: elapsed 0
        assert_eq!(controller.value(&app), 0.0);

        pump(&mut app, Duration::from_millis(50)); // elapsed 50ms of 100ms
        assert_eq!(controller.value(&app), 0.5);

        pump(&mut app, Duration::from_millis(150)); // past the end
        assert_eq!(controller.value(&app), 1.0);
        assert_eq!(controller.status(&app), AnimationStatus::Completed);
        assert!(!controller.is_animating(&mut app));
    }

    #[test]
    fn reverse_drives_the_value_to_the_lower_bound() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        controller.reverse(&mut app, Some(1.0));
        assert_eq!(controller.status(&app), AnimationStatus::Reverse);

        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(50));
        assert_eq!(controller.value(&app), 0.5);

        pump(&mut app, Duration::from_millis(150));
        assert_eq!(controller.value(&app), 0.0);
        assert_eq!(controller.status(&app), AnimationStatus::Dismissed);
    }

    #[test]
    fn the_status_listener_sees_forward_then_completed() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let log = Rc::new(RefCell::new(Vec::new()));
        controller.view().add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let log = Rc::clone(&log);
                move |status, _app| log.borrow_mut().push(status)
            }),
        );

        controller.forward(&mut app, None);
        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(150));

        assert_eq!(
            *log.borrow(),
            vec![AnimationStatus::Forward, AnimationStatus::Completed]
        );
    }

    // The value setter stops the animation and notifies even without change.
    #[test]
    fn set_value_stops_the_animation_and_notifies() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let notified = Rc::new(RefCell::new(0));
        controller.view().add_listener(
            &mut app,
            Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| *notified.borrow_mut() += 1
            }),
        );

        controller.forward(&mut app, None);
        assert!(controller.is_animating(&mut app));

        controller.set_value(&mut app, 0.5);
        assert!(!controller.is_animating(&mut app));
        assert_eq!(controller.value(&app), 0.5);
        assert_eq!(*notified.borrow(), 1);
        assert_eq!(controller.status(&app), AnimationStatus::Forward);
    }

    // The zero-duration path returns Dart's `TickerFuture.complete()`.
    #[test]
    fn a_zero_duration_animate_to_completes_synchronously() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let future = controller.animate_to(&mut app, 1.0, Some(Duration::ZERO), Curves::linear());
        assert_eq!(controller.value(&app), 1.0);
        assert_eq!(controller.status(&app), AnimationStatus::Completed);
        assert!(!controller.is_animating(&mut app));
        assert_future_completed(&mut app, future);
    }

    // animation_controller_test.dart 'Repeating animation with reverse: true'
    // (adapted): the direction setter flips the reported status mid-repeat.
    #[test]
    fn a_reversing_repeat_flips_the_status_each_period() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let log = Rc::new(RefCell::new(Vec::new()));
        controller.view().add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let log = Rc::clone(&log);
                move |status, _app| log.borrow_mut().push(status)
            }),
        );

        controller.repeat(&mut app, None, None, true, None, None);
        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(50));
        assert_eq!(controller.value(&app), 0.5);
        assert_eq!(controller.status(&app), AnimationStatus::Forward);

        // Dart's test pins the boundary: just before the period ends the
        // status is still forward; exactly at it, reverse.
        pump(&mut app, Duration::from_millis(99));
        assert_eq!(controller.status(&app), AnimationStatus::Forward);
        pump(&mut app, Duration::from_millis(100));
        assert_eq!(controller.status(&app), AnimationStatus::Reverse);

        // Second period: playing in reverse. (0.15/0.1) % 1.0 carries float
        // residue in Dart too; its test uses moreOrLessEquals.
        pump(&mut app, Duration::from_millis(150));
        assert!((controller.value(&app) - 0.5).abs() < 1e-9);
        assert_eq!(controller.status(&app), AnimationStatus::Reverse);
        assert_eq!(
            *log.borrow(),
            vec![AnimationStatus::Forward, AnimationStatus::Reverse]
        );

        controller.stop(&mut app, true);
    }

    // The headline order claim of the direction-setter translation: a status
    // listener notified by the mid-repeat flip observes the PRE-tick value,
    // because the drain runs before the tick's value assignment — where
    // Dart's in-`x()` setter call sits.
    #[test]
    fn the_direction_flip_notification_sees_the_pre_tick_value() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let observed = Rc::new(RefCell::new(Vec::new()));
        controller.view().add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let observed = Rc::clone(&observed);
                move |status, app: &mut App| {
                    if status == AnimationStatus::Reverse {
                        let value = controller.view().value(app);
                        observed.borrow_mut().push(value);
                    }
                }
            }),
        );

        controller.repeat(&mut app, None, None, true, None, None);
        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(50)); // value 0.5
        pump(&mut app, Duration::from_millis(100)); // flip: tick will write 1.0

        assert_eq!(
            *observed.borrow(),
            vec![0.5],
            "the flip notification ran before the tick assigned the new value"
        );
        controller.stop(&mut app, true);
    }

    // repeat with a count resolves the future once the periods are spent.
    #[test]
    fn a_counted_repeat_completes() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let future = controller.repeat(&mut app, None, None, false, None, Some(1));
        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(50));
        assert!(controller.is_animating(&mut app));

        pump(&mut app, Duration::from_millis(100)); // exit time reached
        assert!(!controller.is_animating(&mut app));
        assert_future_completed(&mut app, future);
        assert_eq!(controller.status(&app), AnimationStatus::Completed);
        // Dart's boundary value: t wraps to 0 at the exact period end, so
        // the final value is min, not max.
        assert_eq!(controller.value(&app), 0.0);
    }

    // fling reaches the bound and reports Completed; the overshoot target is
    // clamped away.
    #[test]
    fn fling_completes_at_the_upper_bound() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        controller.fling(&mut app, 1.0, None, None);
        let mut at = Duration::ZERO;
        for _ in 0..600 {
            if !controller.is_animating(&mut app) {
                break;
            }
            pump(&mut app, at);
            at += Duration::from_millis(16);
        }
        assert!(!controller.is_animating(&mut app));
        assert_eq!(controller.value(&app), 1.0);
        assert_eq!(controller.status(&app), AnimationStatus::Completed);
    }

    // Disposing mid-flight cancels the ticker future without completing it.
    #[test]
    fn dispose_cancels_the_in_flight_future() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);

        let future = controller.forward(&mut app, None);
        let either = Rc::new(RefCell::new(false));
        future.when_complete_or_cancel(
            &mut app,
            Listener::new({
                let either = Rc::clone(&either);
                move |_app| *either.borrow_mut() = true
            }),
        );

        controller.dispose(&mut app);
        assert_future_canceled(&mut app, future);
        assert!(*either.borrow());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "dispose() called more than once")]
    fn a_second_dispose_panics_in_debug() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);
        controller.dispose(&mut app);
        controller.dispose(&mut app);
    }

    // The velocity getter reads the simulation's dx at the last elapsed time.
    #[test]
    fn velocity_is_zero_when_not_animating() {
        let mut app = App::new();
        let controller = controller_with_duration(&mut app, 100);
        assert_eq!(controller.velocity(&mut app), 0.0);

        controller.forward(&mut app, None);
        pump(&mut app, Duration::from_millis(0));
        pump(&mut app, Duration::from_millis(50));
        // Linear 0..1 over 100ms: 10 units/second.
        assert!((controller.velocity(&mut app) - 10.0).abs() < 0.5);
    }
}

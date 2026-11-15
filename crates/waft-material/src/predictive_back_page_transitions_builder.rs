//! Port of material_ui's `predictive_back_page_transitions_builder.dart`.
//!
//! The gesture reaches these transitions through the binding's back-gesture dispatch
//! (`WidgetsBinding::handle_start_back_gesture` and its siblings); until a host reports the
//! system's back gesture there, the Android builders show their fallback transitions.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationRef, AnyAnimation, CurvedAnimation, Curves, Drive, Interval,
    ProxyAnimation, ReverseAnimation, TweenSequence, TweenSequenceItem, ValueConstantTween,
    ValueTween, k_always_dismissed_animation,
};
use waft_embedder::Radius;
use waft_embedder::{Color, Offset, Size};
use waft_foundation::{App, Handle, Listenable, OptionRetained, Ref};
use waft_painting::BorderRadius;
use waft_services::{PredictiveBackEvent, SwipeEdge};
use waft_widgets::{
    AnimatedBuilder, AnimatedOpacity, AnyPageRoute, BuildContext, ClipRRect, IntoWidget,
    MediaQuery, Opacity, PageTransitionsBuilder, State, StateData, StatefulWidget, Transform,
    WidgetRef, WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef,
};

use crate::page_transitions_theme::{
    FadeForwardsPageTransitionsBuilder, ZoomPageTransitionsBuilder,
};

/// Used by `PageTransitionsTheme` to define a `MaterialPageRoute` page transition animation
/// that looks like the default page transition used on Android U and above when using
/// predictive back.
///
/// Predictive back is only supported on Android U and above, and if this
/// [`PageTransitionsBuilder`] is used by any other platform, it will fall back to
/// [`FadeForwardsPageTransitionsBuilder`].
///
/// When used on Android U and above, animates along with the back gesture to reveal the
/// destination route. Can be canceled by dragging back towards the edge of the screen.
///
/// See also:
///
///  * [`PredictiveBackFullscreenPageTransitionsBuilder`], which is another variant of
///    Android's predictive back page transition.
///  * [`FadeForwardsPageTransitionsBuilder`], which defines the default page transition that's
///    similar to the one provided in Android 16.
///  * [`ZoomPageTransitionsBuilder`], which defines the default page transition that's similar
///    to the one provided in Android 10.
///  * `OpenUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android 9.
///  * `FadeUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android 8.
///  * `CupertinoPageTransitionsBuilder`, which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * `PageRoute::allow_snapshotting`, which enables or disables snapshotting on a per route
///    basis similar to the `allow_snapshotting` parameter.
///  * <https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#shared-element-transition>,
///    which is the Android spec for this page transition, called the Shared Element page
///    transition.
#[derive(Clone, Copy, Debug)]
pub struct PredictiveBackPageTransitionsBuilder {
    /// Whether page transitions will prefer to animate a snapshot of the entering and exiting
    /// routes.
    ///
    /// If not specified, defaults to true.
    ///
    /// When this value is true, page transitions will snapshot the entering and exiting
    /// routes. These snapshots are then animated in place of the underlying widgets to improve
    /// performance of the transition.
    ///
    /// Generally this means that animations that occur on the entering/exiting route while the
    /// route animation plays may appear frozen - unless they are a hero animation or something
    /// that is drawn in a separate overlay.
    pub allow_snapshotting: bool,

    /// The color of the scrim (background) when the predictive back transition is not
    /// supported.
    ///
    /// If not provided, the background color of a default
    /// [`FadeForwardsPageTransitionsBuilder`] will be used.
    pub fallback_color: Option<Color>,
}

impl Default for PredictiveBackPageTransitionsBuilder {
    fn default() -> PredictiveBackPageTransitionsBuilder {
        PredictiveBackPageTransitionsBuilder::new()
    }
}

impl PredictiveBackPageTransitionsBuilder {
    /// Creates an instance of a [`PageTransitionsBuilder`] that matches Android U's predictive
    /// back transition.
    pub const fn new() -> PredictiveBackPageTransitionsBuilder {
        PredictiveBackPageTransitionsBuilder {
            allow_snapshotting: true,
            fallback_color: None,
        }
    }

    /// Dart `PredictiveBackPageTransitionsBuilder(fallbackColor:)`.
    pub const fn fallback_color(
        mut self,
        fallback_color: Color,
    ) -> PredictiveBackPageTransitionsBuilder {
        self.fallback_color = Some(fallback_color);
        self
    }

    /// Dart `PredictiveBackPageTransitionsBuilder(allowSnapshotting:)`.
    pub const fn allow_snapshotting(
        mut self,
        allow_snapshotting: bool,
    ) -> PredictiveBackPageTransitionsBuilder {
        self.allow_snapshotting = allow_snapshotting;
        self
    }
}

impl PageTransitionsBuilder for PredictiveBackPageTransitionsBuilder {
    fn transition_duration(&self) -> Duration {
        Duration::from_millis(FadeForwardsPageTransitionsBuilder::K_TRANSITION_MILLISECONDS)
    }

    fn build_transitions(
        &self,
        _app: &mut App,
        route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let this = *self;
        PredictiveBackGestureDetector {
            route,
            builder: Rc::new(
                move |app, context, phase, start_back_event, current_back_event| {
                    // Only do a predictive back transition when the user is performing a pop
                    // gesture. Otherwise, for things like button presses or other programmatic
                    // navigation, fall back to FadeForwardsPageTransitionsBuilder.
                    if route.as_modal_route().pop_gesture_in_progress(app) {
                        return PredictiveBackSharedElementPageTransition {
                            is_delegated_transition: true,
                            animation: animation.clone(),
                            phase,
                            secondary_animation: secondary_animation.clone(),
                            start_back_event,
                            current_back_event,
                            child: child.clone(),
                        }
                        .into_widget();
                    }

                    let mut fallback = FadeForwardsPageTransitionsBuilder::new()
                        .allow_snapshotting(this.allow_snapshotting);
                    fallback.background_color = this.fallback_color;
                    fallback.build_transitions(
                        app,
                        route,
                        context,
                        animation.clone(),
                        secondary_animation.clone(),
                        child.clone(),
                    )
                },
            ),
        }
        .into_widget()
    }
}

/// Used by `PageTransitionsTheme` to define a `MaterialPageRoute` page transition animation
/// that looks like Android's Full Screen page transition.
///
/// Predictive back is only supported on Android U and above, and if this
/// [`PageTransitionsBuilder`] is used by any other platform, it will fall back to
/// [`ZoomPageTransitionsBuilder`].
///
/// When used on Android U and above, animates along with the back gesture to reveal the
/// destination route. Can be canceled by dragging back towards the edge of the screen.
///
/// See also:
///
///  * [`PredictiveBackPageTransitionsBuilder`], which is the default Android predictive back
///    page transition.
///  * [`FadeForwardsPageTransitionsBuilder`], which defines the default page transition that's
///    similar to the one provided in Android 16.
///  * [`ZoomPageTransitionsBuilder`], which defines the default page transition that's similar
///    to the one provided in Android 10.
///  * `OpenUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android 9.
///  * `FadeUpwardsPageTransitionsBuilder`, which defines a page transition that's similar to
///    the one provided by Android 8.
///  * `CupertinoPageTransitionsBuilder`, which defines a horizontal page transition that
///    matches native iOS page transitions.
///  * <https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#full-screen-surfaces>,
///    which is the native Android docs for this page transition.
#[derive(Clone, Copy, Debug, Default)]
pub struct PredictiveBackFullscreenPageTransitionsBuilder {
    /// The color of the scrim (background) when the predictive back transition is not
    /// supported.
    ///
    /// If not provided, the background color of a default [`ZoomPageTransitionsBuilder`] will
    /// be used.
    pub fallback_color: Option<Color>,
}

impl PredictiveBackFullscreenPageTransitionsBuilder {
    /// Creates an instance of a [`PageTransitionsBuilder`] that matches Android U's full screen
    /// predictive back transition.
    pub const fn new() -> PredictiveBackFullscreenPageTransitionsBuilder {
        PredictiveBackFullscreenPageTransitionsBuilder {
            fallback_color: None,
        }
    }

    /// Dart `PredictiveBackFullscreenPageTransitionsBuilder(fallbackColor:)`.
    pub const fn fallback_color(
        mut self,
        fallback_color: Color,
    ) -> PredictiveBackFullscreenPageTransitionsBuilder {
        self.fallback_color = Some(fallback_color);
        self
    }
}

impl PageTransitionsBuilder for PredictiveBackFullscreenPageTransitionsBuilder {
    fn build_transitions(
        &self,
        _app: &mut App,
        route: AnyPageRoute,
        _context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let fallback_color = self.fallback_color;
        PredictiveBackGestureDetector {
            route,
            builder: Rc::new(
                move |app, context, phase, _start_back_event, _current_back_event| {
                    // Only do a predictive back transition when the user is performing a pop
                    // gesture. Otherwise, for things like button presses or other programmatic
                    // navigation, fall back to ZoomPageTransitionsBuilder.
                    if route.as_modal_route().pop_gesture_in_progress(app) {
                        return PredictiveBackFullscreenPageTransition {
                            animation: animation.clone(),
                            secondary_animation: secondary_animation.clone(),
                            get_is_current: Rc::new(move |app| route.as_route().is_current(app)),
                            phase,
                            child: child.clone(),
                        }
                        .into_widget();
                    }

                    let mut fallback = ZoomPageTransitionsBuilder::new();
                    fallback.background_color = fallback_color;
                    fallback.build_transitions(
                        app,
                        route,
                        context,
                        animation.clone(),
                        secondary_animation.clone(),
                        child.clone(),
                    )
                },
            ),
        }
        .into_widget()
    }
}

type PredictiveBackGestureDetectorWidgetBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        PredictiveBackPhase,
        Option<PredictiveBackEvent>,
        Option<PredictiveBackEvent>,
    ) -> WidgetRef,
>;

/// The phases of a predictive back gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PredictiveBackPhase {
    /// There is no active predictive back gesture in progress.
    Idle,

    /// The user pointer has contacted the screen.
    Start,

    /// The user pointer has moved.
    Update,

    /// The user pointer has released in a position in which Android has determined that the
    /// back gesture is successful and the current route should be popped.
    Commit,

    /// The user pointer has released in a position in which Android has determined that the
    /// back gesture should be canceled and the original route should be shown.
    Cancel,
}

pub(crate) struct PredictiveBackGestureDetector {
    builder: PredictiveBackGestureDetectorWidgetBuilder,
    route: AnyPageRoute,
}

impl Debug for PredictiveBackGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_PredictiveBackGestureDetector")
            .field("route", &self.route)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PredictiveBackGestureDetector {
    type State = PredictiveBackGestureDetectorState;

    fn create_state(&self) -> PredictiveBackGestureDetectorState {
        PredictiveBackGestureDetectorState {
            state: StateData::new(),
            phase: PredictiveBackPhase::Idle,
            start_back_event: None,
            current_back_event: None,
            observer: None,
        }
    }
}

/// Dart's `_PredictiveBackGestureDetectorState`. `observer` is the registration of this state
/// with the binding, kept to remove the same one again.
pub(crate) struct PredictiveBackGestureDetectorState {
    state: StateData<PredictiveBackGestureDetector>,
    phase: PredictiveBackPhase,
    start_back_event: Option<PredictiveBackEvent>,
    current_back_event: Option<PredictiveBackEvent>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl PredictiveBackGestureDetectorState {
    /// True when the predictive back gesture is enabled.
    fn is_enabled(self: Handle<Self>, app: &mut App) -> bool {
        let route = self.widget(app).route;
        route.as_route().is_current(app)
            && route
                .as_modal_route()
                .as_transition_route()
                .pop_gesture_enabled(app)
    }

    pub(crate) fn phase(self: Handle<Self>, app: &App) -> PredictiveBackPhase {
        app.get(self).phase
    }

    fn set_phase(self: Handle<Self>, app: &mut App, phase: PredictiveBackPhase) {
        if app.get(self).phase != phase && self.mounted(app) {
            self.set_state(app, |state| state.phase = phase);
        }
    }

    /// The back event when the gesture first started.
    pub(crate) fn start_back_event(self: Handle<Self>, app: &App) -> Option<PredictiveBackEvent> {
        app.get(self).start_back_event
    }

    fn set_start_back_event(
        self: Handle<Self>,
        app: &mut App,
        start_back_event: Option<PredictiveBackEvent>,
    ) {
        if app.get(self).start_back_event != start_back_event && self.mounted(app) {
            self.set_state(app, |state| state.start_back_event = start_back_event);
        }
    }

    /// The most recent back event during the gesture.
    pub(crate) fn current_back_event(self: Handle<Self>, app: &App) -> Option<PredictiveBackEvent> {
        app.get(self).current_back_event
    }

    fn set_current_back_event(
        self: Handle<Self>,
        app: &mut App,
        current_back_event: Option<PredictiveBackEvent>,
    ) {
        if app.get(self).current_back_event != current_back_event && self.mounted(app) {
            self.set_state(app, |state| state.current_back_event = current_back_event);
        }
    }
}

impl WidgetsBindingObserverObject for PredictiveBackGestureDetectorState {
    fn handle_start_back_gesture(
        self: Handle<Self>,
        app: &mut App,
        back_event: PredictiveBackEvent,
    ) -> bool {
        self.set_phase(app, PredictiveBackPhase::Start);
        let gesture_in_progress = !back_event.is_button_event() && self.is_enabled(app);
        if !gesture_in_progress {
            return false;
        }

        let route = self.widget(app).route;
        route
            .as_modal_route()
            .as_transition_route()
            .handle_start_back_gesture(app, 1.0 - back_event.progress);
        self.set_current_back_event(app, Some(back_event));
        self.set_start_back_event(app, Some(back_event));
        true
    }

    fn handle_update_back_gesture_progress(
        self: Handle<Self>,
        app: &mut App,
        back_event: PredictiveBackEvent,
    ) {
        self.set_phase(app, PredictiveBackPhase::Update);

        let route = self.widget(app).route;
        route
            .as_modal_route()
            .as_transition_route()
            .handle_update_back_gesture_progress(app, 1.0 - back_event.progress);
        self.set_current_back_event(app, Some(back_event));
    }

    fn handle_cancel_back_gesture(self: Handle<Self>, app: &mut App) {
        self.set_phase(app, PredictiveBackPhase::Cancel);

        let route = self.widget(app).route;
        route
            .as_modal_route()
            .as_transition_route()
            .handle_cancel_back_gesture(app);
        self.set_current_back_event(app, None);
        self.set_start_back_event(app, None);
    }

    fn handle_commit_back_gesture(self: Handle<Self>, app: &mut App) {
        self.set_phase(app, PredictiveBackPhase::Commit);

        let route = self.widget(app).route;
        route
            .as_modal_route()
            .as_transition_route()
            .handle_commit_back_gesture(app);
        self.set_current_back_event(app, None);
        self.set_start_back_event(app, None);
    }
}

impl State for PredictiveBackGestureDetectorState {
    type Widget = PredictiveBackGestureDetector;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).observer = Some(Rc::clone(&observer));
        WidgetsBinding::instance(app).add_observer(app, observer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let (route, builder) = (widget.route, Rc::clone(&widget.builder));
        let effective_phase = if route.as_modal_route().pop_gesture_in_progress(app) {
            self.phase(app)
        } else {
            PredictiveBackPhase::Idle
        };
        let start_back_event = self.start_back_event(app);
        let current_back_event = self.current_back_event(app);
        builder(
            app,
            context,
            effective_phase,
            start_back_event,
            current_back_event,
        )
    }
}

/// Android's predictive back page shared element transition.
///
/// See also:
///
///  * <https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#shared-element-transition>,
///    which is the Android spec for this transition.
#[derive(Debug)]
pub(crate) struct PredictiveBackSharedElementPageTransition {
    is_delegated_transition: bool,
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    phase: PredictiveBackPhase,
    start_back_event: Option<PredictiveBackEvent>,
    current_back_event: Option<PredictiveBackEvent>,
    child: WidgetRef,
}

impl StatefulWidget for PredictiveBackSharedElementPageTransition {
    type State = PredictiveBackSharedElementPageTransitionState;

    fn create_state(&self) -> PredictiveBackSharedElementPageTransitionState {
        let _ = (
            self.is_delegated_transition,
            self.secondary_animation.clone(),
        );
        PredictiveBackSharedElementPageTransitionState {
            state: StateData::new(),
            commit_animation: None,
            bounce_animation: None,
            last_bounce_animation_value: 0.0,
            animation: None,
            curved_animation: None,
            curved_animation_reversed: None,
            position_animation: None,
            last_drag: Offset::ZERO,
        }
    }
}

/// Dart's `_PredictiveBackSharedElementPageTransitionState`. The three proxies are Dart's field
/// initialisers; they need the `App`, so `init_state` makes them before anything reads them.
pub(crate) struct PredictiveBackSharedElementPageTransitionState {
    state: StateData<PredictiveBackSharedElementPageTransition>,

    // An animation that stays constant at zero before the commit, and after the commit goes
    // from zero to one.
    commit_animation: Option<Ref<ProxyAnimation>>,

    // An animation that goes from zero to a maximum of one during a predictive back gesture,
    // and then at commit, it goes from its current value to zero. Used for animations that
    // follow the gesture and then animate back to their original value after commit.
    bounce_animation: Option<Ref<ProxyAnimation>>,
    last_bounce_animation_value: f64,

    // An animation that proxies to widget.animation during the gesture and then to
    // _commitAnimation after the commit. So, it goes from zero to a maximum of one before
    // commit, and then after commit goes from zero to one again.
    animation: Option<Ref<ProxyAnimation>>,

    /// The same as widget.animation but with a curve applied.
    curved_animation: Option<Ref<CurvedAnimation>>,

    /// The reverse of `curved_animation`.
    curved_animation_reversed: Option<Ref<CurvedAnimation>>,

    position_animation: Option<AnimationRef<Offset>>,

    last_drag: Offset,
}

impl PredictiveBackSharedElementPageTransitionState {
    // Constants as per the motion specs
    // https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#motion-specs
    const K_MIN_SCALE: f64 = 0.90;
    const K_DIVISION_FACTOR: f64 = 20.0;
    const K_MARGIN: f64 = 8.0;
    const K_Y_POSITION_FACTOR: f64 = 0.1;

    // The duration of the commit transition.
    //
    // This is not the same as PredictiveBackPageTransitionsBuilder's duration, which is the
    // duration of widget.animation, so an Interval is used.
    //
    // Eyeballed on a Pixel 9 running Android 16.
    const K_COMMIT_MILLISECONDS: u64 = 400;

    fn k_commit_interval() -> Interval {
        Interval::new(
            0.0,
            PredictiveBackSharedElementPageTransitionState::K_COMMIT_MILLISECONDS as f64
                / FadeForwardsPageTransitionsBuilder::K_TRANSITION_MILLISECONDS as f64,
            Curves::ease_in_out_cubic_emphasized(),
        )
    }

    // A fallback corner radius used when the display corner radii are unavailable (e.g., on
    // Android API levels below 31, iOS, and other platforms). This is a best-guess value that
    // looks reasonable on most devices.
    // See https://github.com/flutter/flutter/issues/97349.
    const K_DEVICE_BORDER_RADIUS: f64 = 32.0;

    // Provides a smooth transition between the default radius and the _kDeviceBorderRadius,
    // when the display corner radii are unavailable.
    const BORDER_RADIUS_TWEEN: ValueTween<f64> = ValueTween::new(
        0.0,
        PredictiveBackSharedElementPageTransitionState::K_DEVICE_BORDER_RADIUS,
    );

    // The route fades out after commit.
    const OPACITY_TWEEN: ValueTween<f64> = ValueTween::new(1.0, 0.0);

    // The route shrinks during the gesture and animates back to normal after commit.
    const SCALE_TWEEN: ValueTween<f64> = ValueTween::new(
        1.0,
        PredictiveBackSharedElementPageTransitionState::K_MIN_SCALE,
    );

    fn commit_animation(self: Handle<Self>, app: &App) -> Handle<ProxyAnimation> {
        app.get(self)
            .commit_animation
            .handle()
            .expect("created in init_state")
    }

    fn bounce_animation(self: Handle<Self>, app: &App) -> Handle<ProxyAnimation> {
        app.get(self)
            .bounce_animation
            .handle()
            .expect("created in init_state")
    }

    fn position_animation(self: Handle<Self>, app: &App) -> AnyAnimation<Offset> {
        app.get(self)
            .position_animation
            .handle()
            .expect("set in did_change_dependencies")
    }

    // This isn't done as an animation because it's based on the vertical drag amount, not the
    // progression of the back gesture like widget.animation is.
    fn get_y_shift_position(self: Handle<Self>, app: &App, screen_height: f64) -> f64 {
        let widget = self.widget(app);
        let start_touch_y = widget
            .start_back_event
            .and_then(|event| event.touch_offset)
            .map_or(0.0, |offset| offset.dy());
        let current_touch_y = widget
            .current_back_event
            .and_then(|event| event.touch_offset)
            .map_or(0.0, |offset| offset.dy());

        let y_shift_max = (screen_height
            / PredictiveBackSharedElementPageTransitionState::K_DIVISION_FACTOR)
            - PredictiveBackSharedElementPageTransitionState::K_MARGIN;

        let raw_y_shift = current_touch_y - start_touch_y;
        // This curve was eyeballed on a Pixel 9 running Android 16.
        let eased_y_shift = Curves::ease_out()
            .transform((raw_y_shift.abs() / screen_height).clamp(0.0, 1.0))
            * dart_sign(raw_y_shift)
            * y_shift_max;

        eased_y_shift.clamp(-y_shift_max, y_shift_max)
    }

    fn update_animations(self: Handle<Self>, app: &mut App, screen_size: Size) {
        let widget = self.widget(app);
        let (phase, widget_animation, swipe_edge) = (
            widget.phase,
            widget.animation.clone(),
            widget.current_back_event.map(|event| event.swipe_edge),
        );
        let state = app.get(self);
        let curved_animation = state
            .curved_animation
            .clone()
            .expect("set in did_change_dependencies");
        let curved_animation_reversed = state
            .curved_animation_reversed
            .clone()
            .expect("set in did_change_dependencies");
        let last_bounce_animation_value = state.last_bounce_animation_value;
        let last_drag = state.last_drag;

        let animation = app
            .get(self)
            .animation
            .clone()
            .expect("created in init_state");
        let animation_parent: AnimationRef<f64> = match phase {
            PredictiveBackPhase::Commit => curved_animation_reversed.erase(),
            _ => widget_animation.clone(),
        };
        animation.set_parent(app, Some(animation_parent));

        let bounce_animation_parent: AnimationRef<f64> = match phase {
            PredictiveBackPhase::Commit => ValueTween::new(0.0, last_bounce_animation_value)
                .animate(app, curved_animation.erase()),
            _ => ReverseAnimation::new(app, widget_animation).into_erased(),
        };
        self.bounce_animation(app)
            .set_parent(app, Some(bounce_animation_parent));

        let commit_animation_parent: AnimationRef<f64> = match phase {
            PredictiveBackPhase::Commit => animation.erase(),
            _ => k_always_dismissed_animation(app),
        };
        self.commit_animation(app)
            .set_parent(app, Some(commit_animation_parent));

        let x_shift = (screen_size.width()
            / PredictiveBackSharedElementPageTransitionState::K_DIVISION_FACTOR)
            - PredictiveBackSharedElementPageTransitionState::K_MARGIN;
        let position_tween = match phase {
            PredictiveBackPhase::Commit => ValueTween::new(
                last_drag,
                Offset::new(
                    screen_size.height()
                        * PredictiveBackSharedElementPageTransitionState::K_Y_POSITION_FACTOR,
                    0.0,
                ),
            ),
            // The y position before commit is given by the vertical drag, not by an animation.
            _ => {
                let y_shift = self.get_y_shift_position(app, screen_size.height());
                let begin = match swipe_edge {
                    Some(SwipeEdge::Left) => Offset::new(x_shift, y_shift),
                    Some(SwipeEdge::Right) => Offset::new(-x_shift, y_shift),
                    None => Offset::new(x_shift, y_shift),
                };
                ValueTween::new(begin, Offset::ZERO)
            }
        };
        let position_animation = animation.drive(app, position_tween);
        app.get_mut(self).position_animation = Some(position_animation);
    }

    fn update_curved_animations(self: Handle<Self>, app: &mut App) {
        self.dispose_curved_animations(app);
        let animation = self.widget(app).animation.clone();
        let commit_interval: Rc<dyn waft_animation::Curve> =
            Rc::new(PredictiveBackSharedElementPageTransitionState::k_commit_interval());
        let curved_animation =
            CurvedAnimation::create(app, animation.clone(), Rc::clone(&commit_interval), None);
        let reversed = ReverseAnimation::new(app, animation).into_erased();
        let curved_animation_reversed =
            CurvedAnimation::create(app, reversed, commit_interval, None);
        let state = app.get_mut(self);
        state.curved_animation = Some(curved_animation);
        state.curved_animation_reversed = Some(curved_animation_reversed);
    }

    /// Dart's `_curvedAnimation?.dispose()` and `_curvedAnimationReversed?.dispose()`.
    fn dispose_curved_animations(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let (curved_animation, curved_animation_reversed) = (
            state.curved_animation.handle(),
            state.curved_animation_reversed.handle(),
        );
        if let Some(curved_animation) = curved_animation {
            curved_animation.dispose(app);
        }
        if let Some(curved_animation_reversed) = curved_animation_reversed {
            curved_animation_reversed.dispose(app);
        }
    }

    fn build_transition(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let bounce_animation = self.bounce_animation(app).as_animation();
        app.get_mut(self).last_bounce_animation_value = bounce_animation.value(app);
        let scale = PredictiveBackSharedElementPageTransitionState::SCALE_TWEEN
            .evaluate(app, bounce_animation);
        let offset = match self.widget(app).phase {
            PredictiveBackPhase::Commit => self.position_animation(app).value(app),
            _ => {
                let dx = self.position_animation(app).value(app).dx();
                let screen_height = MediaQuery::height_of(app, context);
                let last_drag = Offset::new(dx, self.get_y_shift_position(app, screen_height));
                app.get_mut(self).last_drag = last_drag;
                last_drag
            }
        };
        let opacity = PredictiveBackSharedElementPageTransitionState::OPACITY_TWEEN
            .evaluate(app, self.commit_animation(app).as_animation());
        let border_radius =
            MediaQuery::display_corner_radii_of(app, context).unwrap_or_else(|| {
                BorderRadius::circular(
                    PredictiveBackSharedElementPageTransitionState::BORDER_RADIUS_TWEEN
                        .evaluate(app, bounce_animation),
                )
            });
        let child = child.cloned().expect("the builder's child is the page");
        Transform::scale(Some(scale), None, None)
            .child(
                Transform::translate(offset).child(
                    Opacity::new(opacity).child(
                        ClipRRect::new()
                            .border_radius(border_radius.into())
                            .child(child),
                    ),
                ),
            )
            .into_widget()
    }
}

impl State for PredictiveBackSharedElementPageTransitionState {
    type Widget = PredictiveBackSharedElementPageTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let commit_animation = ProxyAnimation::new(app, None);
        let bounce_animation = ProxyAnimation::new(app, None);
        let animation = ProxyAnimation::new(app, None);
        let state = app.get_mut(self);
        state.commit_animation = Some(commit_animation);
        state.bounce_animation = Some(bounce_animation);
        state.animation = Some(animation);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &PredictiveBackSharedElementPageTransition,
    ) {
        let widget = self.widget(app);
        let (animation, phase) = (widget.animation.clone(), widget.phase);
        if animation != old_widget.animation {
            self.update_curved_animations(app);
        }
        if phase != old_widget.phase && phase == PredictiveBackPhase::Commit {
            let context = self.context(app);
            let size = MediaQuery::size_of(app, context);
            self.update_animations(app, size);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_curved_animations(app);
        let context = self.context(app);
        let size = MediaQuery::size_of(app, context);
        self.update_animations(app, size);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            app.get(self).curved_animation.is_some(),
            "set in did_change_dependencies"
        );
        self.dispose_curved_animations(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let animation: Rc<dyn Listenable> = Rc::new(widget.animation.clone());
        let child = widget.child.clone();
        AnimatedBuilder::new(animation, move |app, context, child| {
            self.build_transition(app, context, child)
        })
        .child(child)
        .into_widget()
    }
}

/// Android's predictive back page transition for full screen surfaces.
///
/// See also:
///
///  * <https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#full-screen-surfaces>,
///    which is the Android spec for this transition.
pub(crate) struct PredictiveBackFullscreenPageTransition {
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    phase: PredictiveBackPhase,
    get_is_current: Rc<dyn Fn(&App) -> bool>,
    child: WidgetRef,
}

impl Debug for PredictiveBackFullscreenPageTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_PredictiveBackFullscreenPageTransition")
            .field("animation", &self.animation)
            .field("secondary_animation", &self.secondary_animation)
            .field("phase", &self.phase)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PredictiveBackFullscreenPageTransition {
    type State = PredictiveBackFullscreenPageTransitionState;

    fn create_state(&self) -> PredictiveBackFullscreenPageTransitionState {
        PredictiveBackFullscreenPageTransitionState {
            state: StateData::new(),
            primary_position_tween: None,
            secondary_position_tween: None,
            secondary_current_position_tween: None,
        }
    }
}

/// Dart's `_PredictiveBackFullscreenPageTransitionState`.
pub(crate) struct PredictiveBackFullscreenPageTransitionState {
    state: StateData<PredictiveBackFullscreenPageTransition>,
    primary_position_tween: Option<TweenSequence<Offset>>,
    secondary_position_tween: Option<ValueTween<Offset>>,
    secondary_current_position_tween: Option<ValueConstantTween<Offset>>,
}

impl PredictiveBackFullscreenPageTransitionState {
    // These values were eyeballed to match the Android spec for the Full Screen page transition:
    // https://developer.android.com/design/ui/mobile/guides/patterns/predictive-back#full-screen-surfaces
    const K_SCALE_START: f64 = 1.0;
    const K_SCALE_COMMIT: f64 = 0.95;
    const K_OPACITY_FULLY_OPENED: f64 = 1.0;
    const K_OPACITY_START_TRANSITION: f64 = 0.95;
    // The point at which the drag would cause a commit instead of a cancel if it were released.
    const K_COMMIT_AT: f64 = 0.65;
    const K_WEIGHT_PRE_COMMIT: f64 = PredictiveBackFullscreenPageTransitionState::K_COMMIT_AT;
    const K_WEIGHT_POST_COMMIT: f64 =
        1.0 - PredictiveBackFullscreenPageTransitionState::K_WEIGHT_PRE_COMMIT;
    const K_SCREEN_WIDTH_DIVISION_FACTOR: f64 = 20.0;
    const K_X_SHIFT_ADJUSTMENT: f64 = 8.0;
    const K_COMMIT_DURATION: Duration = Duration::from_millis(100);

    const PRIMARY_OPACITY_TWEEN: ValueTween<f64> = ValueTween::new(
        PredictiveBackFullscreenPageTransitionState::K_OPACITY_START_TRANSITION,
        PredictiveBackFullscreenPageTransitionState::K_OPACITY_FULLY_OPENED,
    );

    fn primary_scale_tween() -> TweenSequence<f64> {
        type S = PredictiveBackFullscreenPageTransitionState;
        TweenSequence::new(vec![
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(S::K_SCALE_START, S::K_SCALE_START)),
                S::K_WEIGHT_PRE_COMMIT,
            ),
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(S::K_SCALE_COMMIT, S::K_SCALE_START)),
                S::K_WEIGHT_POST_COMMIT,
            ),
        ])
    }

    const SECONDARY_SCALE_TWEEN_CURRENT: ValueConstantTween<f64> =
        ValueConstantTween::new(PredictiveBackFullscreenPageTransitionState::K_SCALE_START);

    fn secondary_tween_scale() -> TweenSequence<f64> {
        type S = PredictiveBackFullscreenPageTransitionState;
        TweenSequence::new(vec![
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(S::K_SCALE_COMMIT, S::K_SCALE_START)),
                S::K_WEIGHT_PRE_COMMIT,
            ),
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(S::K_SCALE_START, S::K_SCALE_START)),
                S::K_WEIGHT_POST_COMMIT,
            ),
        ])
    }

    const SECONDARY_OPACITY_TWEEN_CURRENT: ValueConstantTween<f64> = ValueConstantTween::new(
        PredictiveBackFullscreenPageTransitionState::K_OPACITY_FULLY_OPENED,
    );

    fn secondary_opacity_tween() -> TweenSequence<f64> {
        type S = PredictiveBackFullscreenPageTransitionState;
        TweenSequence::new(vec![
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(
                    S::K_OPACITY_FULLY_OPENED,
                    S::K_OPACITY_START_TRANSITION,
                )),
                S::K_WEIGHT_PRE_COMMIT,
            ),
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(
                    S::K_OPACITY_FULLY_OPENED,
                    S::K_OPACITY_FULLY_OPENED,
                )),
                S::K_WEIGHT_POST_COMMIT,
            ),
        ])
    }

    fn secondary_animated_builder(
        self: Handle<Self>,
        app: &mut App,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let (secondary_animation, get_is_current) = (
            widget.secondary_animation.handle(),
            Rc::clone(&widget.get_is_current),
        );
        let is_current = get_is_current(app);

        let state = app.get(self);
        let offset = if is_current {
            state
                .secondary_current_position_tween
                .expect("set in did_change_dependencies")
                .evaluate(app, secondary_animation)
        } else {
            state
                .secondary_position_tween
                .expect("set in did_change_dependencies")
                .evaluate(app, secondary_animation)
        };
        let scale = if is_current {
            Self::SECONDARY_SCALE_TWEEN_CURRENT.evaluate(app, secondary_animation)
        } else {
            Self::secondary_tween_scale().evaluate(app, secondary_animation)
        };
        let opacity = if is_current {
            Self::SECONDARY_OPACITY_TWEEN_CURRENT.evaluate(app, secondary_animation)
        } else {
            Self::secondary_opacity_tween().evaluate(app, secondary_animation)
        };
        let child = child.cloned().expect("the builder's child is the page");
        Transform::translate(offset)
            .child(
                Transform::scale(Some(scale), None, None).child(Opacity::new(opacity).child(child)),
            )
            .into_widget()
    }

    fn primary_animated_builder(
        self: Handle<Self>,
        app: &mut App,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let (animation, phase) = (widget.animation.handle(), widget.phase);
        let offset = app
            .get(self)
            .primary_position_tween
            .clone()
            .expect("set in did_change_dependencies")
            .evaluate(app, animation);
        let scale = Self::primary_scale_tween().evaluate(app, animation);
        let opacity = Self::PRIMARY_OPACITY_TWEEN.evaluate(app, animation);
        // A sudden fadeout at the commit point, driven by time and not the gesture.
        let commit_opacity = match phase {
            PredictiveBackPhase::Commit => 0.0,
            _ => {
                if animation.value(app) < Self::K_COMMIT_AT {
                    0.0
                } else {
                    1.0
                }
            }
        };
        let child = child.cloned().expect("the builder's child is the page");
        Transform::translate(offset)
            .child(Transform::scale(Some(scale), None, None).child(
                // A slight change in opacity before reaching the commit point.
                Opacity::new(opacity).child(
                    AnimatedOpacity::new(commit_opacity, Self::K_COMMIT_DURATION).child(child),
                ),
            ))
            .into_widget()
    }
}

impl State for PredictiveBackFullscreenPageTransitionState {
    type Widget = PredictiveBackFullscreenPageTransition;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let screen_width = MediaQuery::width_of(app, context);
        let x_shift =
            (screen_width / Self::K_SCREEN_WIDTH_DIVISION_FACTOR) - Self::K_X_SHIFT_ADJUSTMENT;
        let primary_position_tween = TweenSequence::new(vec![
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(Offset::ZERO, Offset::ZERO)),
                Self::K_WEIGHT_PRE_COMMIT,
            ),
            TweenSequenceItem::new(
                Rc::new(ValueTween::new(Offset::new(x_shift, 0.0), Offset::ZERO)),
                Self::K_WEIGHT_POST_COMMIT,
            ),
        ]);

        let state = app.get_mut(self);
        state.primary_position_tween = Some(primary_position_tween);
        state.secondary_current_position_tween = Some(ValueConstantTween::new(Offset::ZERO));
        state.secondary_position_tween =
            Some(ValueTween::new(Offset::new(x_shift, 0.0), Offset::ZERO));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let secondary_animation: Rc<dyn Listenable> = Rc::new(widget.secondary_animation.clone());
        let animation: Rc<dyn Listenable> = Rc::new(widget.animation.clone());
        let child = widget.child.clone();
        let border_radius =
            MediaQuery::display_corner_radii_of(app, context).unwrap_or_else(|| {
                BorderRadius::all(Radius::circular(
                    PredictiveBackSharedElementPageTransitionState::K_DEVICE_BORDER_RADIUS,
                ))
            });
        AnimatedBuilder::new(secondary_animation, move |app, _context, child| {
            self.secondary_animated_builder(app, child)
        })
        .child(
            AnimatedBuilder::new(animation, move |app, _context, child| {
                self.primary_animated_builder(app, child)
            })
            .child(
                ClipRRect::new()
                    .border_radius(border_radius.into())
                    .child(child),
            ),
        )
        .into_widget()
    }
}

/// Dart's `double.sign`: -1.0, 0.0 or 1.0, where Rust's `signum` answers 1.0 for zero.
fn dart_sign(value: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        value
    } else {
        value.signum()
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use waft_embedder::{TargetPlatform, ViewId};
    use waft_foundation::Listener;
    use waft_widgets::{
        AnyElement, ColoredBox, Column, CrossAxisAlignment, Navigator, NavigatorState,
        PageTransitionsBuilderRef, RouteSettingsRef, SizedBox, SnapshotWidget, StateSetter,
        StatefulBuilder, Text, WidgetBuilder,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::page::MaterialPageRoute;
    use crate::page_transitions_theme::{
        FadeForwardsPageTransition, PageTransitionsTheme, ZoomEnterTransition, ZoomPageTransition,
    };
    use crate::test_support::{Drawn, WidgetTester, material_page_app, with_main_thread_stack};
    use crate::text_button::TextButton;
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;
    use waft_animation::AnimationWithParent;
    use waft_foundation::HandleId;

    /// Not a Dart case: Dart's collector frees a disposed shared-element transition's proxies
    /// and animations; here they are freed with the state's references to them.
    #[test]
    fn a_disposed_shared_element_transition_frees_its_animations() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let animation = tester
                .cell
                .borrow_mut()
                .create(waft_animation::AlwaysStoppedAnimation::new(0.5))
                .into_erased::<AnyAnimation<f64>>();
            tester.pump_widget(
                PredictiveBackSharedElementPageTransition {
                    is_delegated_transition: false,
                    animation: animation.clone(),
                    secondary_animation: animation.clone(),
                    phase: PredictiveBackPhase::Update,
                    start_back_event: None,
                    current_back_event: None,
                    child: waft_widgets::SizedBox::new().into_widget(),
                }
                .into_widget(),
            );
            let element = tester.find_by_type::<PredictiveBackSharedElementPageTransition>()[0];
            let owned: Vec<HandleId> = {
                let app = tester.cell.borrow();
                let state = element
                    .state_handle::<PredictiveBackSharedElementPageTransitionState>(&app)
                    .expect("the transition's state");
                let curved_animation_reversed = app
                    .get(state)
                    .curved_animation_reversed
                    .as_ref()
                    .expect("set in did_change_dependencies")
                    .handle();
                vec![
                    state.commit_animation(&app).into(),
                    state.bounce_animation(&app).into(),
                    app.get(state)
                        .curved_animation
                        .as_ref()
                        .expect("set")
                        .handle()
                        .into(),
                    curved_animation_reversed.into(),
                    AnimationWithParent::parent(curved_animation_reversed, &app).id(),
                ]
            };
            tester.pump_widget(waft_widgets::SizedBox::new().into_widget());
            tester.cell.checkpoint();
            let app = tester.cell.borrow();
            for id in owned {
                assert!(!app.contains(id), "{id:?} is freed");
            }
        });
    }

    /// Not a Dart case: Dart's collector frees the animations a shared-element transition's
    /// `_updateCurvedAnimations` and `_updateAnimations` replace; here they are freed with their
    /// last reference.
    #[test]
    fn a_shared_element_transition_given_new_dependencies_leaves_no_new_arena_objects() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let animation = tester
                .cell
                .borrow_mut()
                .create(waft_animation::AlwaysStoppedAnimation::new(0.5))
                .into_erased::<AnyAnimation<f64>>();
            let transition = move |width: f64| -> WidgetRef {
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new().size(Size::new(width, 600.0)),
                    PredictiveBackSharedElementPageTransition {
                        is_delegated_transition: false,
                        animation: animation.clone(),
                        secondary_animation: animation.clone(),
                        phase: PredictiveBackPhase::Update,
                        start_back_event: None,
                        current_back_event: None,
                        child: waft_widgets::SizedBox::new().into_widget(),
                    },
                )
                .into_widget()
            };
            tester.pump_widget(transition(800.0));
            tester.pump_widget(transition(700.0));
            let before = tester.cell.borrow().debug_object_count();
            tester.pump_widget(transition(800.0));
            tester.pump_widget(transition(700.0));
            assert_eq!(tester.cell.borrow().debug_object_count(), before);
        });
    }

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        configure: impl FnOnce(MaterialApp) -> MaterialApp,
    ) {
        let root = {
            let theme = theme(&tester.cell.borrow());
            material_page_app(theme, configure)
        };
        tester.pump_widget(root);
    }

    /// `ThemeData(pageTransitionsTheme:)` with `builder` for every platform.
    fn theme_for_every_platform(
        builder: PageTransitionsBuilderRef,
    ) -> impl FnOnce(&App) -> ThemeData {
        move |app| {
            ThemeData::builder()
                .page_transitions_theme(
                    PageTransitionsTheme::new().with_builders(
                        ALL_PLATFORMS
                            .into_iter()
                            .map(|platform| (platform, builder.clone()))
                            .collect(),
                    ),
                )
                .build(app)
        }
    }

    /// The cases' `routes`: `/` a button 'push' that pushes `/b`, and `/b` the text 'page b'.
    fn push_routes() -> Vec<(String, WidgetBuilder)> {
        vec![
            (
                "/".to_string(),
                Rc::new(|_app: &mut App, context: BuildContext| {
                    Material::new()
                        .child(TextButton::new(
                            Some(Listener::new(move |app| {
                                Navigator::push_named(app, context, "/b", None);
                            })),
                            Some(Text::new("push")),
                        ))
                        .into_widget()
                }) as WidgetBuilder,
            ),
            (
                "/b".to_string(),
                Rc::new(|_app: &mut App, _context: BuildContext| Text::new("page b").into_widget())
                    as WidgetBuilder,
            ),
        ]
    }

    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = *tester
            .find_onstage_text(text)
            .first()
            .expect("the text is onstage");
        let center = tester.global_rect(tester.render_box_of(element)).center();
        tester.tap(center, 1);
    }

    fn found(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    /// Dart's `find.byType(W)` with its default `skipOffstage`.
    fn count_of<W: 'static>(tester: &WidgetTester) -> usize {
        tester
            .find_by_type::<W>()
            .into_iter()
            .filter(|element| tester.is_onstage(*element))
            .count()
    }

    /// The case's builder: the shared-element one, or the full-screen one.
    #[derive(Clone, Copy, Debug)]
    enum Variant {
        SharedElement,
        Fullscreen,
    }

    impl Variant {
        fn builder(self) -> PageTransitionsBuilderRef {
            match self {
                Variant::SharedElement => {
                    PageTransitionsBuilderRef::new(PredictiveBackPageTransitionsBuilder::new())
                }
                Variant::Fullscreen => PageTransitionsBuilderRef::new(
                    PredictiveBackFullscreenPageTransitionsBuilder::new(),
                ),
            }
        }

        /// Dart's `_findPredictiveBackPageTransition`.
        fn predictive_back_transitions(self, tester: &WidgetTester) -> usize {
            match self {
                Variant::SharedElement => {
                    count_of::<PredictiveBackSharedElementPageTransition>(tester)
                }
                Variant::Fullscreen => count_of::<PredictiveBackFullscreenPageTransition>(tester),
            }
        }

        /// Dart's `_findFallbackPageTransition`.
        fn fallback_transitions(self, tester: &WidgetTester) -> usize {
            match self {
                Variant::SharedElement => count_of::<FadeForwardsPageTransition>(tester),
                Variant::Fullscreen => count_of::<ZoomPageTransition>(tester),
            }
        }
    }

    fn view(tester: &WidgetTester) -> ViewId {
        let app = tester.cell.borrow();
        app.platform().views().first().expect("a view").id()
    }

    /// The engine's `startBackGesture` on `flutter/backgesture`, from the left edge.
    fn start_back_gesture(tester: &WidgetTester, touch_offset: Offset, progress: f64) {
        let mut app = tester.cell.borrow_mut();
        let event = PredictiveBackEvent::new(Some(touch_offset), progress, SwipeEdge::Left);
        WidgetsBinding::instance(&mut app).handle_start_back_gesture(&mut app, event);
        app.drain_microtasks();
    }

    /// The engine's `updateBackGestureProgress`, from the left edge.
    fn update_back_gesture(tester: &WidgetTester, touch_offset: Option<Offset>, progress: f64) {
        let mut app = tester.cell.borrow_mut();
        let event = PredictiveBackEvent::new(touch_offset, progress, SwipeEdge::Left);
        WidgetsBinding::instance(&mut app).handle_update_back_gesture_progress(&mut app, event);
        app.drain_microtasks();
    }

    /// The engine's `commitBackGesture`.
    fn commit_back_gesture(tester: &WidgetTester) {
        let view = view(tester);
        let mut app = tester.cell.borrow_mut();
        WidgetsBinding::instance(&mut app).handle_commit_back_gesture(&mut app, view);
        app.drain_microtasks();
    }

    /// The engine's `cancelBackGesture`.
    fn cancel_back_gesture(tester: &WidgetTester) {
        let mut app = tester.cell.borrow_mut();
        WidgetsBinding::instance(&mut app).handle_cancel_back_gesture(&mut app);
        app.drain_microtasks();
    }

    /// The engine's `popRoute` on `flutter/navigation`.
    fn pop_route(tester: &WidgetTester) {
        let view = view(tester);
        let mut app = tester.cell.borrow_mut();
        WidgetsBinding::instance(&mut app).handle_pop_route(&mut app, view);
        app.drain_microtasks();
    }

    fn top_left(tester: &WidgetTester, text: &str) -> Offset {
        let element: AnyElement = *tester.find_text(text).first().expect("the text");
        tester.global_rect(tester.render_box_of(element)).top_left()
    }

    /// The first part of the gesture cases: the app with `variant`'s builder on every platform
    /// and `routes` (a `/` with a 'push' button to `/b`, which shows 'page b'), `/b` pushed and
    /// settled; on Android, a back gesture started and dragged to `update_progress`. Answers whether the gesture ran.
    fn push_and_drag(
        tester: &mut WidgetTester,
        platform: TargetPlatform,
        variant: Variant,
        routes: Vec<(String, WidgetBuilder)>,
        update_touch: Option<Offset>,
        update_progress: f64,
    ) -> bool {
        pump_app(tester, theme_for_every_platform(variant.builder()), |app| {
            app.routes(routes)
        });

        assert_eq!(found(tester, "push"), 1);
        assert_eq!(found(tester, "page b"), 0);
        assert_eq!(variant.predictive_back_transitions(tester), 0);
        assert_eq!(variant.fallback_transitions(tester), 1);

        tap_text(tester, "push");
        tester.pump_and_settle();

        assert_eq!(found(tester, "push"), 0);
        assert_eq!(found(tester, "page b"), 1);
        assert_eq!(variant.predictive_back_transitions(tester), 0);
        assert_eq!(variant.fallback_transitions(tester), 1);

        // Only Android supports backGesture channel methods. Other platforms will do nothing.
        if platform != TargetPlatform::Android {
            return false;
        }

        // Start a system pop gesture, which will switch to using the predictive back page
        // transition.
        start_back_gesture(tester, Offset::new(5.0, 300.0), 0.0);
        tester.pump(Duration::ZERO);

        assert_eq!(variant.predictive_back_transitions(tester), 1);
        assert_eq!(variant.fallback_transitions(tester), 0);
        let start_page_b_offset = top_left(tester, "page b");
        assert_eq!(start_page_b_offset.dx(), 0.0);

        // Drag the system back gesture.
        update_back_gesture(tester, update_touch, update_progress);
        tester.pump_and_settle();

        assert_eq!(variant.predictive_back_transitions(tester), 2);
        assert_eq!(variant.fallback_transitions(tester), 0);

        let update_page_b_offset = top_left(tester, "page b");
        assert!(update_page_b_offset.dx() > start_page_b_offset.dx());
        true
    }

    #[test]
    fn the_predictive_back_builders_follow_a_predictive_back_gesture_on_android() {
        for variant in [Variant::SharedElement, Variant::Fullscreen] {
            for platform in ALL_PLATFORMS {
                with_main_thread_stack(|| {
                    let mut tester = tester_on(platform);
                    if !push_and_drag(&mut tester, platform, variant, push_routes(), None, 0.35) {
                        return;
                    }

                    // Commit the system back gesture.
                    commit_back_gesture(&tester);
                    tester.pump_and_settle();

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        0,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 1, "{variant:?}");
                    assert_eq!(found(&tester, "push"), 1);
                    assert_eq!(found(&tester, "page b"), 0);
                });
            }
        }
    }

    #[test]
    fn the_predictive_back_builders_follow_a_canceled_predictive_back_gesture_on_android() {
        for variant in [Variant::SharedElement, Variant::Fullscreen] {
            for platform in ALL_PLATFORMS {
                with_main_thread_stack(|| {
                    let mut tester = tester_on(platform);
                    if !push_and_drag(
                        &mut tester,
                        platform,
                        variant,
                        push_routes(),
                        Some(Offset::new(100.0, 300.0)),
                        0.35,
                    ) {
                        return;
                    }

                    // Cancel the system back gesture.
                    cancel_back_gesture(&tester);
                    tester.pump_and_settle();

                    assert_eq!(found(&tester, "push"), 0);
                    assert_eq!(found(&tester, "page b"), 1);
                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        0,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 1, "{variant:?}");
                });
            }
        }
    }

    /// The multiple-observers case's routes: `/` a button 'push' that pushes `/b`, and `/b` the
    /// text 'page b' over a `StatefulBuilder` that shows, once `including_nested_navigator` is
    /// set, a nested `Navigator` whose one route has `variant`'s builder on every platform.
    fn routes_with_nested_navigator(
        variant: Variant,
        including_nested_navigator: Rc<std::cell::Cell<bool>>,
        set_state: Rc<std::cell::RefCell<Option<StateSetter>>>,
    ) -> Vec<(String, WidgetBuilder)> {
        let mut routes = push_routes();
        routes[1].1 = Rc::new(move |_app: &mut App, _context: BuildContext| {
            let including_nested_navigator = Rc::clone(&including_nested_navigator);
            let set_state = Rc::clone(&set_state);
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children([
                    Text::new("page b").into_widget(),
                    StatefulBuilder::new(move |_app, _context, local_set_state| {
                        *set_state.borrow_mut() = Some(local_set_state);
                        if !including_nested_navigator.get() {
                            return SizedBox::shrink().into_widget();
                        }
                        Navigator::new()
                            .initial_route("b/nested")
                            .on_generate_route(move |app, settings| {
                                let builder: WidgetBuilder = match settings.name.as_deref() {
                                    Some("b/nested") => Rc::new(move |app, _context| {
                                        Material::new()
                                            .child(Theme::new(
                                                theme_for_every_platform(variant.builder())(app),
                                                Column::new().children([Text::new(
                                                    "Nested route inside of page b",
                                                )
                                                .into_widget()]),
                                            ))
                                            .into_widget()
                                    }),
                                    name => panic!("Invalid route: {name:?}"),
                                };
                                Some(
                                    (MaterialPageRoute::new(app, builder).settings(
                                        app,
                                        RouteSettingsRef::Settings(settings.clone()),
                                    ))
                                    .into_erased(),
                                )
                            })
                            .into_widget()
                    })
                    .into_widget(),
                ])
                .into_widget()
        }) as WidgetBuilder;
        routes
    }

    #[test]
    fn if_there_are_multiple_predictive_back_observers_only_one_gets_called_for_a_given_back_gesture()
     {
        for variant in [Variant::SharedElement, Variant::Fullscreen] {
            for platform in ALL_PLATFORMS {
                with_main_thread_stack(|| {
                    let mut tester = tester_on(platform);
                    let including_nested_navigator = Rc::new(std::cell::Cell::new(false));
                    let set_state = Rc::new(std::cell::RefCell::new(None::<StateSetter>));
                    let routes = routes_with_nested_navigator(
                        variant,
                        Rc::clone(&including_nested_navigator),
                        Rc::clone(&set_state),
                    );
                    let dragged = push_and_drag(
                        &mut tester,
                        platform,
                        variant,
                        routes,
                        Some(Offset::new(100.0, 300.0)),
                        0.3,
                    );
                    assert_eq!(found(&tester, "Nested route inside of page b"), 0);
                    if !dragged {
                        return;
                    }
                    let update_page_b_offset = top_left(&tester, "page b");

                    // In the middle of the system back gesture here, add a nested Navigator
                    // that includes a new predictive back gesture observer.
                    let set_state = set_state.borrow().clone().expect("page b built");
                    set_state(&mut tester.cell.borrow_mut(), &mut |_app| {
                        including_nested_navigator.set(true)
                    });
                    tester.pump_and_settle();
                    assert_eq!(found(&tester, "push"), 1);
                    assert_eq!(found(&tester, "page b"), 1);
                    assert_eq!(found(&tester, "Nested route inside of page b"), 1);

                    // Send another drag gesture, and ensure that the original observer still
                    // gets it.
                    update_back_gesture(&tester, Some(Offset::new(110.0, 300.0)), 0.35);
                    tester.pump_and_settle();

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        2,
                        "{variant:?}"
                    );
                    // Despite using a predictive back transition, the new route has not
                    // received a start event, so it is still using the fallback transition.
                    assert_eq!(variant.fallback_transitions(&tester), 1, "{variant:?}");

                    let update2_page_b_offset = top_left(&tester, "page b");
                    assert!(update2_page_b_offset.dx() > update_page_b_offset.dx());

                    // Commit the system back gesture, and the original observer is able to
                    // handle the back without interference.
                    commit_back_gesture(&tester);
                    tester.pump_and_settle();

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        0,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 1, "{variant:?}");
                    assert_eq!(found(&tester, "push"), 1);
                    assert_eq!(found(&tester, "page b"), 0);
                    assert_eq!(found(&tester, "Nested route inside of page b"), 0);
                });
            }
        }
    }

    /// The back-to-back case's routes: `/` a button 'push b' to `/b`, `/b` a button 'push c'
    /// to `/c`, and `/c` the text 'page c'.
    fn three_routes() -> Vec<(String, WidgetBuilder)> {
        let push_button = |label: &'static str, name: &'static str| -> WidgetBuilder {
            Rc::new(move |_app: &mut App, context: BuildContext| {
                Material::new()
                    .child(TextButton::new(
                        Some(Listener::new(move |app| {
                            Navigator::push_named(app, context, name, None);
                        })),
                        Some(Text::new(label)),
                    ))
                    .into_widget()
            })
        };
        vec![
            ("/".to_string(), push_button("push b", "/b")),
            ("/b".to_string(), push_button("push c", "/c")),
            (
                "/c".to_string(),
                Rc::new(|_app: &mut App, _context: BuildContext| Text::new("page c").into_widget())
                    as WidgetBuilder,
            ),
        ]
    }

    #[test]
    fn two_back_gestures_back_to_back_dismiss_two_routes() {
        for variant in [Variant::SharedElement, Variant::Fullscreen] {
            for platform in ALL_PLATFORMS {
                with_main_thread_stack(|| {
                    let mut tester = tester_on(platform);
                    pump_app(
                        &mut tester,
                        theme_for_every_platform(variant.builder()),
                        |app| app.routes(three_routes()),
                    );

                    assert_eq!(found(&tester, "push b"), 1);
                    assert_eq!(found(&tester, "push c"), 0);
                    assert_eq!(found(&tester, "page c"), 0);
                    assert_eq!(variant.predictive_back_transitions(&tester), 0);
                    assert_eq!(variant.fallback_transitions(&tester), 1);

                    tap_text(&mut tester, "push b");
                    tester.pump_and_settle();

                    assert_eq!(found(&tester, "push"), 0);
                    assert_eq!(found(&tester, "push c"), 1);
                    assert_eq!(found(&tester, "page c"), 0);
                    assert_eq!(variant.predictive_back_transitions(&tester), 0);
                    assert_eq!(variant.fallback_transitions(&tester), 1);

                    tap_text(&mut tester, "push c");
                    tester.pump_and_settle();

                    assert_eq!(found(&tester, "push"), 0);
                    assert_eq!(found(&tester, "push c"), 0);
                    assert_eq!(found(&tester, "page c"), 1);
                    assert_eq!(variant.predictive_back_transitions(&tester), 0);
                    assert_eq!(variant.fallback_transitions(&tester), 1);

                    // Only Android supports backGesture channel methods. Other platforms will
                    // do nothing.
                    if platform != TargetPlatform::Android {
                        return;
                    }

                    // Start a system pop gesture, which will switch to using the predictive
                    // back page transition.
                    start_back_gesture(&tester, Offset::new(5.0, 300.0), 0.0);
                    tester.pump(Duration::ZERO);

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        1,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 0, "{variant:?}");

                    // Drag the system back gesture far enough to commit.
                    update_back_gesture(&tester, None, 0.35);
                    tester.pump(Duration::ZERO);

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        2,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 0, "{variant:?}");

                    // Commit the system back gesture.
                    commit_back_gesture(&tester);
                    tester.pump(Duration::ZERO);

                    // The predictive back page transitions still exist because the outgoing
                    // animation has not yet finished.
                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        2,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 0, "{variant:?}");

                    assert_eq!(found(&tester, "push"), 0);
                    assert_eq!(found(&tester, "push c"), 1);
                    assert_eq!(found(&tester, "page c"), 1);

                    // Start another system pop gesture, before the first has finished
                    // animating out.
                    start_back_gesture(&tester, Offset::new(5.0, 300.0), 0.0);
                    tester.pump(Duration::ZERO);

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        2,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 0, "{variant:?}");

                    // Drag the system back gesture far enough to commit.
                    update_back_gesture(&tester, None, 0.35);
                    tester.pump(Duration::ZERO);

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        3,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 0, "{variant:?}");

                    // Commit the system back gesture.
                    commit_back_gesture(&tester);
                    tester.pump_and_settle();

                    assert_eq!(
                        variant.predictive_back_transitions(&tester),
                        0,
                        "{variant:?}"
                    );
                    assert_eq!(variant.fallback_transitions(&tester), 1, "{variant:?}");

                    assert_eq!(found(&tester, "push b"), 1);
                    assert_eq!(found(&tester, "push c"), 0);
                    assert_eq!(found(&tester, "page c"), 0);
                });
            }
        }
    }

    // page_transitions_theme_test 'predictive back gestures pop the route on all platforms
    // regardless of whether their transition handles predictive back'.
    #[test]
    fn a_predictive_back_gesture_pops_the_route_on_every_platform() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                pump_app(&mut tester, ThemeData::new, |app| app.routes(push_routes()));

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);

                tap_text(&mut tester, "push");
                tester.pump_and_settle();

                assert_eq!(found(&tester, "push"), 0);
                assert_eq!(found(&tester, "page b"), 1);

                // Start a system pop gesture.
                start_back_gesture(&tester, Offset::new(5.0, 300.0), 0.0);
                tester.pump(Duration::ZERO);

                assert_eq!(found(&tester, "push"), 0);
                assert_eq!(found(&tester, "page b"), 1);

                // Drag the system back gesture far enough to commit.
                update_back_gesture(&tester, None, 0.35);
                tester.pump_and_settle();

                match platform {
                    // Shows both pages while doing the "peek" predictive back transition.
                    TargetPlatform::Android => assert_eq!(found(&tester, "push"), 1),
                    // Does no transition yet; still shows page b only.
                    _ => assert_eq!(found(&tester, "push"), 0, "{platform:?}"),
                }
                assert_eq!(found(&tester, "page b"), 1);

                // Commit the system back gesture.
                commit_back_gesture(&tester);
                tester.pump_and_settle();
                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);
            });
        }
    }

    #[test]
    fn predictive_back_is_the_default_on_android() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                pump_app(&mut tester, ThemeData::new, |app| app.routes(push_routes()));

                let push = *tester.find_text("push").first().expect("push");
                let builder = {
                    let mut app = tester.cell.borrow_mut();
                    Theme::of(&mut app, push)
                        .page_transitions_theme
                        .builders()
                        .get(&platform)
                        .cloned()
                };
                let is_predictive_back = builder.is_some_and(|builder| {
                    format!("{builder:?}").starts_with("PredictiveBackPageTransitionsBuilder")
                });
                assert_eq!(
                    is_predictive_back,
                    platform == TargetPlatform::Android,
                    "{platform:?}"
                );
            });
        }
    }

    // page_transitions_theme_test 'predictive back falls back to FadeForwardsPageTransition'.
    #[test]
    fn predictive_back_falls_back_to_the_fade_forwards_transition() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let variant = Variant::SharedElement;
                let mut tester = tester_on(platform);
                pump_app(
                    &mut tester,
                    theme_for_every_platform(variant.builder()),
                    |app| app.routes(push_routes()),
                );

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);

                tap_text(&mut tester, "push");
                tester.pump_and_settle();

                assert_eq!(found(&tester, "push"), 0);
                assert_eq!(found(&tester, "page b"), 1);

                // Only Android sends system back gestures.
                let android = platform == TargetPlatform::Android;
                if android {
                    start_back_gesture(&tester, Offset::new(5.0, 300.0), 0.0);
                    tester.pump(Duration::ZERO);
                }

                assert_eq!(
                    variant.predictive_back_transitions(&tester),
                    usize::from(android)
                );
                assert_eq!(variant.fallback_transitions(&tester), usize::from(!android));

                assert_eq!(found(&tester, "push"), 0);
                assert_eq!(found(&tester, "page b"), 1);

                // Drag the system back gesture far enough to commit.
                if android {
                    update_back_gesture(&tester, None, 0.35);
                    tester.pump_and_settle();
                    assert_eq!(found(&tester, "push"), 1);
                } else {
                    assert_eq!(found(&tester, "push"), 0);
                }

                assert_eq!(found(&tester, "page b"), 1);

                if android {
                    assert_eq!(variant.predictive_back_transitions(&tester), 2);
                    assert_eq!(variant.fallback_transitions(&tester), 0);
                } else {
                    assert_eq!(variant.predictive_back_transitions(&tester), 0);
                    assert_eq!(variant.fallback_transitions(&tester), 1);
                }

                if android {
                    // Commit the system back gesture on Android.
                    commit_back_gesture(&tester);
                } else {
                    // On other platforms, send a one-off system pop.
                    pop_route(&tester);
                }
                tester.pump(Duration::ZERO);

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 1);

                if android {
                    assert_eq!(variant.predictive_back_transitions(&tester), 2);
                    assert_eq!(variant.fallback_transitions(&tester), 0);
                } else {
                    assert_eq!(variant.predictive_back_transitions(&tester), 0);
                    assert_eq!(variant.fallback_transitions(&tester), 2);
                }

                tester.pump_and_settle();

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);

                assert_eq!(variant.predictive_back_transitions(&tester), 0);
                assert_eq!(variant.fallback_transitions(&tester), 1);
            });
        }
    }

    #[test]
    fn the_predictive_back_builder_falls_back_with_its_fallback_color() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let variant = Variant::SharedElement;
                let mut tester = tester_on(platform);
                pump_app(
                    &mut tester,
                    theme_for_every_platform(PageTransitionsBuilderRef::new(
                        PredictiveBackPageTransitionsBuilder::new().fallback_color(Colors::BLACK),
                    )),
                    |app| app.routes(push_routes()),
                );

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);
                assert_eq!(variant.predictive_back_transitions(&tester), 0);
                assert_eq!(variant.fallback_transitions(&tester), 1);

                tap_text(&mut tester, "push");
                tester.pump(Duration::from_millis(400));

                let colored_box = *tester
                    .find_by_type::<ColoredBox>()
                    .into_iter()
                    .filter(|element| tester.is_onstage(*element))
                    .collect::<Vec<_>>()
                    .last()
                    .expect("a ColoredBox");
                let color = tester
                    .widget::<ColoredBox, _>(colored_box, |colored_box| colored_box.color.color());
                assert_eq!(color, Colors::BLACK, "{platform:?}");

                tester.pump_and_settle();
            });
        }
    }

    #[test]
    fn the_predictive_back_fullscreen_builder_falls_back_with_its_fallback_color() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let variant = Variant::Fullscreen;
                let mut tester = tester_on(platform);
                pump_app(
                    &mut tester,
                    theme_for_every_platform(PageTransitionsBuilderRef::new(
                        PredictiveBackFullscreenPageTransitionsBuilder::new()
                            .fallback_color(Colors::BLACK),
                    )),
                    |app| app.routes(push_routes()),
                );

                assert_eq!(found(&tester, "push"), 1);
                assert_eq!(found(&tester, "page b"), 0);
                assert_eq!(variant.predictive_back_transitions(&tester), 0);
                assert_eq!(variant.fallback_transitions(&tester), 1);

                // Pump till animation is half-way through.
                tap_text(&mut tester, "push");
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(125));

                // The entering page's zoom transition paints the scrim.
                let enter = *tester
                    .find_by_type::<ZoomEnterTransition>()
                    .iter()
                    .find(|element| {
                        let app = tester.cell.borrow();
                        tester.widget::<ZoomEnterTransition, _>(**element, |enter| {
                            !enter.reverse && enter.animation.is_animating(&app)
                        })
                    })
                    .expect("the entering page's zoom transition");
                let render = tester.render_box_of(enter).as_object();
                let rects: Vec<u32> = tester
                    .painted(render)
                    .into_iter()
                    .filter_map(|drawn| match drawn {
                        Drawn::Rect(color, _) => Some(color),
                        _ => None,
                    })
                    .collect();
                // Expect the color to be at exactly 59.6% opacity at this time.
                #[allow(deprecated)]
                let expected = Colors::BLACK.with_opacity(0.596).to_argb32();
                assert!(rects.contains(&expected), "{platform:?}: {rects:x?}");

                tester.pump_and_settle();
            });
        }
    }

    /// The fallback snapshotting cases: page 1 and page 2 under `builder` on Android; pushes
    /// `/2` and pops, each a tenth into the transition, and answers whether page 1 and page 2
    /// snapshot each time: (push exiting, push entering, pop exiting, pop entering).
    fn push_and_pop_snapshotting(builder: PredictiveBackPageTransitionsBuilder) -> [bool; 4] {
        let mut result = [false; 4];
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .page_transitions_theme(PageTransitionsTheme::new().with_builders(
                            IndexMap::from([(
                                TargetPlatform::Android,
                                PageTransitionsBuilderRef::new(builder),
                            )]),
                        ))
                        .build(app)
                },
                |app| {
                    app.on_generate_route(|app, settings| {
                        let text = if settings.name.as_deref() == Some("/") {
                            "Page 1"
                        } else {
                            "Page 2"
                        };
                        Some(
                            (MaterialPageRoute::new(
                                app,
                                Rc::new(move |_app, _context| {
                                    Material::new().child(Text::new(text)).into_widget()
                                }),
                            ))
                            .into_erased(),
                        )
                    })
                },
            );
            let is_snapshotting = |tester: &WidgetTester, text: &str| {
                let text = *tester.find_text(text).first().expect("the text");
                let ancestors = tester.ancestors(text);
                let app = tester.cell.borrow();
                ancestors.into_iter().any(|ancestor| {
                    waft_widgets::downcast_widget::<SnapshotWidget>(&**ancestor.widget(&app))
                        .is_some_and(|snapshot| snapshot.controller.allow_snapshotting(&app))
                })
            };
            let navigator = {
                let element = *tester
                    .find_by_type::<Navigator>()
                    .first()
                    .expect("a navigator");
                element
                    .state_handle::<NavigatorState>(&tester.cell.borrow())
                    .expect("a navigator's state")
            };
            let partway = Duration::from_millis(
                FadeForwardsPageTransitionsBuilder::K_TRANSITION_MILLISECONDS / 10,
            );

            navigator.push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump(Duration::ZERO);
            tester.pump(partway);
            assert!(Variant::SharedElement.fallback_transitions(&tester) > 0);
            result[0] = is_snapshotting(&tester, "Page 1");
            result[1] = is_snapshotting(&tester, "Page 2");
            tester.pump_and_settle();

            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            tester.pump(partway);
            result[2] = is_snapshotting(&tester, "Page 2");
            result[3] = is_snapshotting(&tester, "Page 1");
            tester.pump_and_settle();
        });
        result
    }

    #[test]
    fn the_fallback_snapshots_the_entering_and_exiting_routes_by_default() {
        assert_eq!(
            push_and_pop_snapshotting(PredictiveBackPageTransitionsBuilder::new()),
            [true, true, true, true]
        );
    }

    #[test]
    fn the_fallback_is_given_the_builders_allow_snapshotting() {
        assert_eq!(
            push_and_pop_snapshotting(
                PredictiveBackPageTransitionsBuilder::new().allow_snapshotting(false)
            ),
            [false, false, false, false]
        );
    }

    #[test]
    fn a_button_back_event_starts_no_predictive_back_gesture() {
        with_main_thread_stack(|| {
            let variant = Variant::SharedElement;
            let mut tester = tester_on(TargetPlatform::Android);
            pump_app(
                &mut tester,
                theme_for_every_platform(variant.builder()),
                |app| app.routes(push_routes()),
            );
            tap_text(&mut tester, "push");
            tester.pump_and_settle();

            let handled = {
                let mut app = tester.cell.borrow_mut();
                let event = PredictiveBackEvent::new(None, 0.0, SwipeEdge::Left);
                WidgetsBinding::instance(&mut app).handle_start_back_gesture(&mut app, event)
            };
            tester.pump(Duration::ZERO);

            assert!(!handled);
            assert_eq!(variant.predictive_back_transitions(&tester), 0);
            assert_eq!(variant.fallback_transitions(&tester), 1);
        });
    }
}

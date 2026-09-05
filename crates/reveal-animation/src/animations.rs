//! Flutter counterpart: `animation/animations.dart`.

use std::rc::Rc;

use reveal_foundation::{App, Handle, ListenableObject, Listener};

use crate::animation::{Animation, AnimationStatus, AnimationStatusListener, AnyAnimation};
use crate::curves::Curve;
use crate::listener_helpers::{
    AnimationEagerListenerMixin, AnimationLazyListenerData, AnimationLazyListenerMixin,
    AnimationLocalListenersData, AnimationLocalListenersMixin, AnimationLocalStatusListenersData,
    AnimationLocalStatusListenersMixin,
};
use crate::tween::Animatable;

/// Dart's `_AlwaysCompleteAnimation`. Private like the original; reach it
/// through [`k_always_complete_animation`].
#[derive(Default)]
struct AlwaysCompleteAnimation;

impl Animation<f64> for AlwaysCompleteAnimation {
    fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}

    fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}

    fn add_status_listener(self: Handle<Self>, _app: &mut App, _listener: AnimationStatusListener) {
    }

    fn remove_status_listener(
        self: Handle<Self>,
        _app: &mut App,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(self: Handle<Self>, _app: &App) -> AnimationStatus {
        AnimationStatus::Completed
    }

    fn value(self: Handle<Self>, _app: &App) -> f64 {
        1.0
    }
}

/// An animation that is always complete.
///
/// Using this involves less overhead than building an `AnimationController`
/// with an initial value of 1.0. This is useful when an API expects an
/// animation but you don't actually want to animate anything.
///
/// Dart's `kAlwaysCompleteAnimation` is a `const` — every mention is one
/// canonical object — and ours is the App's singleton of the same type, so
/// every call returns the same identity, as Dart's mentions do.
pub fn k_always_complete_animation(app: &mut App) -> AnyAnimation<f64> {
    app.singleton::<AlwaysCompleteAnimation>().as_animation()
}

/// Dart's `_AlwaysDismissedAnimation`. Private like the original; reach it
/// through [`k_always_dismissed_animation`].
#[derive(Default)]
struct AlwaysDismissedAnimation;

impl Animation<f64> for AlwaysDismissedAnimation {
    fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}

    fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}

    fn add_status_listener(self: Handle<Self>, _app: &mut App, _listener: AnimationStatusListener) {
    }

    fn remove_status_listener(
        self: Handle<Self>,
        _app: &mut App,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(self: Handle<Self>, _app: &App) -> AnimationStatus {
        AnimationStatus::Dismissed
    }

    fn value(self: Handle<Self>, _app: &App) -> f64 {
        0.0
    }
}

/// An animation that is always dismissed.
///
/// Using this involves less overhead than building an `AnimationController`
/// with an initial value of 0.0. This is useful when an API expects an
/// animation but you don't actually want to animate anything.
///
/// See [`k_always_complete_animation`] for how the identity matches Dart's
/// `const`.
pub fn k_always_dismissed_animation(app: &mut App) -> AnyAnimation<f64> {
    app.singleton::<AlwaysDismissedAnimation>().as_animation()
}

/// An animation that is always stopped at a given value.
///
/// The status is always [`AnimationStatus::Forward`].
pub struct AlwaysStoppedAnimation<T> {
    /// The value at which this animation is stopped.
    ///
    /// Since the value and status of an [`AlwaysStoppedAnimation`] can never
    /// change, the listeners can never be called. It is therefore safe to
    /// reuse one in multiple places.
    pub value: T,
}

impl<T> AlwaysStoppedAnimation<T> {
    /// Creates an [`AlwaysStoppedAnimation`] with the given value.
    pub const fn new(value: T) -> AlwaysStoppedAnimation<T> {
        AlwaysStoppedAnimation { value }
    }
}

impl<T: Clone + 'static> Animation<T> for AlwaysStoppedAnimation<T> {
    fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}

    fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}

    fn add_status_listener(self: Handle<Self>, _app: &mut App, _listener: AnimationStatusListener) {
    }

    fn remove_status_listener(
        self: Handle<Self>,
        _app: &mut App,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(self: Handle<Self>, _app: &App) -> AnimationStatus {
        AnimationStatus::Forward
    }

    fn value(self: Handle<Self>, app: &App) -> T {
        app.get(self).value.clone()
    }
}

/// Implements most of the animation interface by deferring its behavior to a
/// given [`parent`] animation.
///
/// To implement an animation that is driven by a parent, it is only necessary
/// to implement this trait, supply [`parent`], and implement
/// [`Animation::value`].
///
/// To define a mapping from values in the range 0..1, consider subclassing
/// `Tween` instead.
///
/// [`parent`]: AnimationWithParent::parent
pub trait AnimationWithParent<T: 'static>: Sized + 'static {
    /// The animation whose value this animation will proxy.
    ///
    /// This animation must remain the same for the lifetime of this object. If
    /// you wish to proxy a different animation at different times, consider
    /// using [`ProxyAnimation`].
    fn parent(self: Handle<Self>, app: &App) -> AnyAnimation<T>;

    /// Calls the listener every time the value of the animation changes.
    ///
    /// Listeners can be removed with [`remove_listener`].
    ///
    /// [`remove_listener`]: AnimationWithParent::remove_listener
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        let parent = self.parent(app);
        parent.add_listener(app, listener);
    }

    /// Stop calling the listener every time the value of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_listener`].
    ///
    /// [`add_listener`]: AnimationWithParent::add_listener
    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        let parent = self.parent(app);
        parent.remove_listener(app, listener);
    }

    /// Calls listener every time the status of the animation changes.
    ///
    /// Listeners can be removed with [`remove_status_listener`].
    ///
    /// [`remove_status_listener`]: AnimationWithParent::remove_status_listener
    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        let parent = self.parent(app);
        parent.add_status_listener(app, listener);
    }

    /// Stops calling the listener every time the status of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_status_listener`].
    ///
    /// [`add_status_listener`]: AnimationWithParent::add_status_listener
    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        let parent = self.parent(app);
        parent.remove_status_listener(app, listener);
    }

    /// The current status of this animation.
    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        self.parent(app).status(app)
    }
}

/// An animation that is a proxy for another animation.
///
/// A proxy animation is useful because the parent animation can be mutated.
/// For example, one object can create a proxy animation, hand the proxy to
/// another object, and then later change the animation from which the proxy
/// receives its value.
pub struct ProxyAnimation {
    status: Option<AnimationStatus>,
    value: Option<f64>,
    parent: Option<AnyAnimation<f64>>,

    // Dart mixes in `AnimationLazyListenerMixin`,
    // `AnimationLocalListenersMixin` and `AnimationLocalStatusListenersMixin`;
    // their state lives here and reaches the traits through the accessors.
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl ProxyAnimation {
    /// Creates a proxy animation.
    ///
    /// If the animation argument is `None`, the proxy animation has the status
    /// [`AnimationStatus::Dismissed`] and a value of 0.0.
    pub fn new(app: &mut App, animation: Option<AnyAnimation<f64>>) -> Handle<ProxyAnimation> {
        let mut result = ProxyAnimation {
            status: None,
            value: None,
            parent: animation,
            lazy_listener: AnimationLazyListenerData::new(),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        };
        if result.parent.is_none() {
            result.status = Some(AnimationStatus::Dismissed);
            result.value = Some(0.0);
        }
        app.create(result)
    }

    /// Chains a `Tween` (or any [`Animatable`]) to this proxy.
    ///
    /// Dart inherits `drive` from `Animation<double>`. A `Handle<ProxyAnimation>`
    /// is not a subtype, so the method is inherent and forwards to
    /// [`as_animation`](Animation::as_animation).
    pub fn drive<U: 'static>(
        self: Handle<Self>,
        app: &mut App,
        child: impl Animatable<U> + Clone + 'static,
    ) -> AnyAnimation<U> {
        self.as_animation().drive(app, child)
    }

    /// The animation whose value this animation will proxy.
    ///
    /// This value is mutable ([`set_parent`](ProxyAnimation::set_parent)).
    /// When mutated, the listeners on the proxy animation will be
    /// transparently updated to be listening to the new parent animation.
    pub fn parent(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        app.get(self).parent
    }

    /// Changes the animation whose value this animation proxies.
    ///
    /// Dart's `parent` setter, `animations.dart:200`.
    pub fn set_parent(self: Handle<Self>, app: &mut App, value: Option<AnyAnimation<f64>>) {
        if value == app.get(self).parent {
            return;
        }
        if let Some(parent) = app.get(self).parent {
            let (status, current) = (parent.status(app), parent.value(app));
            let proxy = app.get_mut(self);
            proxy.status = Some(status);
            proxy.value = Some(current);
            if self.is_listening(app) {
                self.did_stop_listening(app);
            }
        }
        app.get_mut(self).parent = value;
        if let Some(parent) = app.get(self).parent {
            if self.is_listening(app) {
                self.did_start_listening(app);
            }
            if app.get(self).value != Some(parent.value(app)) {
                self.notify_listeners(app);
            }
            let status = parent.status(app);
            if app.get(self).status != Some(status) {
                self.notify_status_listeners(app, status);
            }
            let proxy = app.get_mut(self);
            proxy.status = None;
            proxy.value = None;
        }
    }
}

impl AnimationLazyListenerMixin for ProxyAnimation {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        if let Some(parent) = app.get(self).parent {
            // Dart: `_parent!.addListener(notifyListeners)` — a tear-off.
            // `handle_method` gives the rebuilt listener the same identity, so
            // `did_stop_listening` removes without a stored handle.
            parent.add_listener(
                app,
                Listener::handle_method(self, ProxyAnimation::notify_listeners),
            );
            parent.add_status_listener(
                app,
                AnimationStatusListener::handle_method(
                    self,
                    ProxyAnimation::notify_status_listeners,
                ),
            );
        }
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        if let Some(parent) = app.get(self).parent {
            parent.remove_listener(
                app,
                &Listener::handle_method(self, ProxyAnimation::notify_listeners),
            );
            parent.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(
                    self,
                    ProxyAnimation::notify_status_listeners,
                ),
            );
        }
    }
}

impl AnimationLocalListenersMixin for ProxyAnimation {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    // Dart resolves these two by mixin order — `AnimationLazyListenerMixin` is
    // the one that declares them in ProxyAnimation's `with` clause. Rust has
    // no such rule, so the choice is written out.
    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for ProxyAnimation {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl ListenableObject for ProxyAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener);
    }
}

// Dart: `class ProxyAnimation extends Animation<double> with
// AnimationLazyListenerMixin, AnimationLocalListenersMixin,
// AnimationLocalStatusListenersMixin`. The forwarding below is the `with`
// clause: the mixins supply the listener protocol.
impl Animation<f64> for ProxyAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        match app.get(self).parent {
            Some(parent) => parent.status(app),
            None => app
                .get(self)
                .status
                .expect("a ProxyAnimation without a parent keeps its last status"),
        }
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        match app.get(self).parent {
            Some(parent) => parent.value(app),
            None => app
                .get(self)
                .value
                .expect("a ProxyAnimation without a parent keeps its last value"),
        }
    }
}

/// An animation that is the reverse of another animation.
///
/// If the parent animation is running forward from 0.0 to 1.0, this animation
/// is running in reverse from 1.0 to 0.0.
///
/// Using a [`ReverseAnimation`] is different from using a `Tween` with a
/// `begin` of 1.0 and an `end` of 0.0 because the tween does not change the
/// status or direction of the animation.
pub struct ReverseAnimation {
    // Dart: `final Animation<double> parent` — private with a getter so it
    // cannot be rebound away from the animation the subscriptions target.
    parent: AnyAnimation<f64>,

    // `AnimationLazyListenerMixin` and `AnimationLocalStatusListenersMixin`
    // state.
    lazy_listener: AnimationLazyListenerData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl ReverseAnimation {
    /// Creates a reverse animation.
    pub fn new(parent: AnyAnimation<f64>) -> ReverseAnimation {
        ReverseAnimation {
            parent,
            lazy_listener: AnimationLazyListenerData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }

    /// The animation whose value and direction this animation is reversing.
    pub fn parent(&self) -> AnyAnimation<f64> {
        self.parent
    }

    fn status_change_handler(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        self.notify_status_listeners(app, Self::reverse_status(status));
    }

    fn reverse_status(status: AnimationStatus) -> AnimationStatus {
        match status {
            AnimationStatus::Forward => AnimationStatus::Reverse,
            AnimationStatus::Reverse => AnimationStatus::Forward,
            AnimationStatus::Completed => AnimationStatus::Dismissed,
            AnimationStatus::Dismissed => AnimationStatus::Completed,
        }
    }
}

impl AnimationLazyListenerMixin for ReverseAnimation {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        let parent = app.get(self).parent;
        parent.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, ReverseAnimation::status_change_handler),
        );
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        let parent = app.get(self).parent;
        parent.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, ReverseAnimation::status_change_handler),
        );
    }
}

impl AnimationLocalStatusListenersMixin for ReverseAnimation {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

// Dart: `class ReverseAnimation extends Animation<double> with
// AnimationLazyListenerMixin, AnimationLocalStatusListenersMixin`. The value
// listeners are the class's own overrides: they forward to the parent, but
// count through the lazy mixin so the status subscription follows them.
impl Animation<f64> for ReverseAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLazyListenerMixin::did_register_listener(self, app);
        let parent = app.get(self).parent;
        parent.add_listener(app, listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        let parent = app.get(self).parent;
        parent.remove_listener(app, listener);
        AnimationLazyListenerMixin::did_unregister_listener(self, app);
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        Self::reverse_status(app.get(self).parent.status(app))
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        1.0 - app.get(self).parent.value(app)
    }
}

/// An animation that applies a curve to another animation.
///
/// [`CurvedAnimation`] is useful when you want to apply a non-linear [`Curve`]
/// to an animation object, especially if you want different curves when the
/// animation is going forward vs when it is going backward.
///
/// Depending on the given curve, the output of the [`CurvedAnimation`] could
/// have a wider range than its input. For example, elastic curves such as
/// `Curves::elastic_in()` will significantly overshoot or undershoot the
/// default range of 0.0 to 1.0.
///
/// A [`CurvedAnimation`] should be disposed when no longer needed to clean up
/// any listeners that it may have added.
pub struct CurvedAnimation {
    // Dart: `final Animation<double> parent` — private with the
    // `AnimationWithParent::parent` getter, so it cannot be rebound away from
    // the animation the constructor subscribed to.
    parent: AnyAnimation<f64>,

    /// The curve to use in the forward direction.
    pub curve: Rc<dyn Curve>,

    /// The curve to use in the reverse direction.
    ///
    /// If the parent animation changes direction without first reaching the
    /// [`AnimationStatus::Completed`] or [`AnimationStatus::Dismissed`]
    /// status, the [`CurvedAnimation`] stays on the same curve (albeit in the
    /// opposite direction) to avoid visual discontinuities.
    ///
    /// If this field is `None`, uses [`curve`](CurvedAnimation::curve) in both
    /// directions.
    pub reverse_curve: Option<Rc<dyn Curve>>,

    /// The direction used to select the current curve.
    ///
    /// The curve direction is only reset when we hit the beginning or the end
    /// of the timeline to avoid discontinuities in the value of any variables
    /// this animation is used to animate.
    curve_direction: Option<AnimationStatus>,

    /// True if this CurvedAnimation has been disposed.
    pub is_disposed: bool,
}

impl CurvedAnimation {
    /// Creates a curved animation.
    ///
    /// Dart's constructor registers `_updateCurveDirection` on `parent` — a
    /// tear-off of the object under construction — so creation goes through
    /// the [`App`]: the entity exists first, then the constructor body runs in
    /// Dart's statement order.
    pub fn create(
        app: &mut App,
        parent: AnyAnimation<f64>,
        curve: Rc<dyn Curve>,
        reverse_curve: Option<Rc<dyn Curve>>,
    ) -> Handle<CurvedAnimation> {
        let this = app.create(CurvedAnimation {
            parent,
            curve,
            reverse_curve,
            curve_direction: None,
            is_disposed: false,
        });
        let status = parent.status(app);
        this.update_curve_direction(app, status);
        parent.add_status_listener(
            app,
            AnimationStatusListener::handle_method(this, CurvedAnimation::update_curve_direction),
        );
        this
    }

    fn update_curve_direction(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        let animation = app.get_mut(self);
        animation.curve_direction = if status.is_animating() {
            animation.curve_direction.or(Some(status))
        } else {
            None
        };
    }

    fn use_forward_curve(self: Handle<Self>, app: &App) -> bool {
        let animation = app.get(self);
        animation.reverse_curve.is_none()
            || animation
                .curve_direction
                .unwrap_or_else(|| animation.parent.status(app))
                != AnimationStatus::Reverse
    }

    /// Cleans up any listeners added by this CurvedAnimation.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).is_disposed = true;
        let parent = app.get(self).parent;
        parent.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, CurvedAnimation::update_curve_direction),
        );
    }
}

impl AnimationWithParent<f64> for CurvedAnimation {
    fn parent(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).parent
    }
}

// Dart: `class CurvedAnimation extends Animation<double> with
// AnimationWithParentMixin<double>`.
impl Animation<f64> for CurvedAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationWithParent::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationWithParent::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationWithParent::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationWithParent::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        AnimationWithParent::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        let animation = app.get(self);
        let active_curve = if self.use_forward_curve(app) {
            Some(Rc::clone(&animation.curve))
        } else {
            animation.reverse_curve.clone()
        };

        let t = animation.parent.value(app);
        let Some(active_curve) = active_curve else {
            return t;
        };
        if t == 0.0 || t == 1.0 {
            #[cfg(debug_assertions)]
            {
                let transformed_value = active_curve.transform(t);
                let rounded_transformed_value = transformed_value.round();
                assert!(
                    rounded_transformed_value == t,
                    "Invalid curve endpoint at {t}. Curves must map 0.0 to near zero and 1.0 to \
                     near one but {active_curve:?} mapped {t} to {transformed_value}, which is \
                     near {rounded_transformed_value}."
                );
            }
            return t;
        }
        active_curve.transform(t)
    }
}

/// Dart's private `_TrainHoppingMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TrainHoppingMode {
    Minimize,
    Maximize,
}

/// This animation starts by proxying one animation, but when the value of that
/// animation crosses the value of the second (either because the second is
/// going in the opposite direction, or because the one overtakes the other),
/// the animation hops over to proxying the second animation.
///
/// When the [`TrainHoppingAnimation`] starts proxying the second animation
/// instead of the first, the [`on_switched_train`] callback is called.
///
/// If the two animations start at the same value, then the
/// [`TrainHoppingAnimation`] immediately hops to the second animation, and the
/// [`on_switched_train`] callback is not called. If only one animation is
/// provided (i.e. if the second is `None`), then the [`TrainHoppingAnimation`]
/// just proxies the first animation.
///
/// Since this object must track the two animations even when it has no
/// listeners of its own, instead of shutting down when all its listeners are
/// removed, it exposes a [`dispose`] method. Call this method to shut this
/// object down.
///
/// [`on_switched_train`]: TrainHoppingAnimation::on_switched_train
/// [`dispose`]: TrainHoppingAnimation::dispose
pub struct TrainHoppingAnimation {
    current_train: Option<AnyAnimation<f64>>,
    next_train: Option<AnyAnimation<f64>>,
    mode: Option<TrainHoppingMode>,

    /// Called when this animation switches to be driven by the second
    /// animation.
    ///
    /// This is not called if the two animations provided to the constructor
    /// have the same value at the time of the call to the constructor. In that
    /// case, the second animation is used from the start, and the first is
    /// ignored.
    pub on_switched_train: Option<Listener>,

    last_status: Option<AnimationStatus>,
    last_value: Option<f64>,

    // `AnimationLocalListenersMixin` and `AnimationLocalStatusListenersMixin`
    // state. `AnimationEagerListenerMixin` declares none.
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl TrainHoppingAnimation {
    /// Creates a train-hopping animation.
    ///
    /// If the next train is `None`, then this object will just proxy the first
    /// animation and never hop. Dart's constructor registers tear-offs of the
    /// object under construction, so creation goes through the [`App`].
    pub fn create(
        app: &mut App,
        current_train: AnyAnimation<f64>,
        next_train: Option<AnyAnimation<f64>>,
        on_switched_train: Option<Listener>,
    ) -> Handle<TrainHoppingAnimation> {
        let mut current_train = current_train;
        let mut next_train = next_train;
        let mut mode = None;
        if let Some(next) = next_train {
            if current_train.value(app) == next.value(app) {
                current_train = next;
                next_train = None;
            } else if current_train.value(app) > next.value(app) {
                mode = Some(TrainHoppingMode::Maximize);
            } else {
                debug_assert!(current_train.value(app) < next.value(app));
                mode = Some(TrainHoppingMode::Minimize);
            }
        }
        let this = app.create(TrainHoppingAnimation {
            current_train: Some(current_train),
            next_train,
            mode,
            on_switched_train,
            last_status: None,
            last_value: None,
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        });
        current_train.add_status_listener(
            app,
            AnimationStatusListener::handle_method(this, Self::status_change_handler),
        );
        current_train.add_listener(
            app,
            Listener::handle_method(this, Self::value_change_handler),
        );
        if let Some(next) = next_train {
            next.add_listener(
                app,
                Listener::handle_method(this, Self::value_change_handler),
            );
        }
        debug_assert!(app.get(this).mode.is_some() || app.get(this).next_train.is_none());
        this
    }

    /// The animation that is currently driving this animation.
    ///
    /// The identity of this object will change from the first animation to the
    /// second animation when [`on_switched_train`] is called.
    ///
    /// [`on_switched_train`]: TrainHoppingAnimation::on_switched_train
    pub fn current_train(&self) -> Option<AnyAnimation<f64>> {
        self.current_train
    }

    fn status_change_handler(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        debug_assert!(app.get(self).current_train.is_some());
        if Some(status) != app.get(self).last_status {
            self.notify_status_listeners(app, status);
            app.get_mut(self).last_status = Some(status);
        }
        debug_assert!(app.get(self).last_status.is_some());
    }

    fn value_change_handler(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).current_train.is_some());
        let mut hop = false;
        if app.get(self).next_train.is_some() {
            debug_assert!(app.get(self).mode.is_some());
            let animation = app.get(self);
            let current = animation.current_train.unwrap();
            let next = animation.next_train.unwrap();
            hop = match animation.mode.unwrap() {
                TrainHoppingMode::Minimize => next.value(app) <= current.value(app),
                TrainHoppingMode::Maximize => next.value(app) >= current.value(app),
            };
            if hop {
                current.remove_status_listener(
                    app,
                    &AnimationStatusListener::handle_method(self, Self::status_change_handler),
                );
                current.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::value_change_handler),
                );
                let animation = app.get_mut(self);
                animation.current_train = animation.next_train.take();
                let new_current = app.get(self).current_train.unwrap();
                new_current.add_status_listener(
                    app,
                    AnimationStatusListener::handle_method(self, Self::status_change_handler),
                );
                let status = new_current.status(app);
                self.status_change_handler(app, status);
            }
        }
        let new_value = self.value(app);
        if Some(new_value) != app.get(self).last_value {
            self.notify_listeners(app);
            app.get_mut(self).last_value = Some(new_value);
        }
        debug_assert!(app.get(self).last_value.is_some());
        if hop && let Some(on_switched_train) = app.get(self).on_switched_train.clone() {
            on_switched_train.call(app);
        }
    }

    /// Frees all the resources used by this performance.
    /// After this is called, this object is no longer usable.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).current_train.is_some());
        let current = app
            .get(self)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train");
        current.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::status_change_handler),
        );
        current.remove_listener(
            app,
            &Listener::handle_method(self, Self::value_change_handler),
        );
        app.get_mut(self).current_train = None;
        if let Some(next) = app.get(self).next_train {
            next.remove_listener(
                app,
                &Listener::handle_method(self, Self::value_change_handler),
            );
        }
        app.get_mut(self).next_train = None;
        self.clear_listeners(app);
        self.clear_status_listeners(app);
        AnimationEagerListenerMixin::dispose(self, app);
    }
}

impl AnimationEagerListenerMixin for TrainHoppingAnimation {}

impl AnimationLocalListenersMixin for TrainHoppingAnimation {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    // Dart resolves these by mixin order — `AnimationEagerListenerMixin` is
    // the one that declares them in this class's `with` clause.
    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for TrainHoppingAnimation {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

// Dart: `class TrainHoppingAnimation extends Animation<double> with
// AnimationEagerListenerMixin, AnimationLocalListenersMixin,
// AnimationLocalStatusListenersMixin`.
impl Animation<f64> for TrainHoppingAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        app.get(self)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train")
            .status(app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train")
            .value(app)
    }
}

/// An interface for combining multiple Animations. Implementors need only
/// implement [`Animation::value`] to control how the child animations are
/// combined. Can be chained to combine more than 2 animations.
///
/// For example, to create an animation that is the sum of two others,
/// implement this trait and define `value` as `first.value + next.value`.
///
/// By default, the status of a compound animation is the status of the
/// [`next`] animation if [`next`] is moving, and the status of the [`first`]
/// animation otherwise.
///
/// Dart's `CompoundAnimation<T>` is an abstract class whose handlers call the
/// subclass's `value` override. The upward call is what makes it a trait here;
/// it reaches the override through [`as_animation`], the erased handle, whose
/// vtable is built from the subclass's [`Animation`] impl. The fields Dart
/// declares on the abstract class belong to the implementing type and reach
/// the trait through the app-mediated accessors.
///
/// [`as_animation`]: Animation::as_animation
/// [`first`]: CompoundAnimation::first
/// [`next`]: CompoundAnimation::next
pub trait CompoundAnimation<T: Clone + PartialEq + 'static>:
    Animation<T>
    + AnimationLazyListenerMixin
    + AnimationLocalListenersMixin
    + AnimationLocalStatusListenersMixin
{
    /// The first sub-animation. Its status takes precedence if neither are
    /// animating.
    fn first(self: Handle<Self>, app: &App) -> AnyAnimation<T>;

    /// The second sub-animation.
    fn next(self: Handle<Self>, app: &App) -> AnyAnimation<T>;

    /// Dart's `_lastStatus`, owned by the implementing type.
    fn last_status(self: Handle<Self>, app: &App) -> Option<AnimationStatus>;

    /// Dart's `_lastStatus`, mutably.
    fn last_status_mut(self: Handle<Self>, app: &mut App) -> &mut Option<AnimationStatus>;

    /// Dart's `_lastValue`, owned by the implementing type.
    fn last_value(self: Handle<Self>, app: &App) -> &Option<T>;

    /// Dart's `_lastValue`, mutably.
    fn last_value_mut(self: Handle<Self>, app: &mut App) -> &mut Option<T>;

    /// Gets the status of this animation based on the [`first`] and [`next`]
    /// status.
    ///
    /// The default is that if the [`next`] animation is moving, use its
    /// status. Otherwise, default to [`first`].
    ///
    /// [`first`]: CompoundAnimation::first
    /// [`next`]: CompoundAnimation::next
    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        let next_status = self.next(app).status(app);
        if next_status.is_animating() {
            next_status
        } else {
            self.first(app).status(app)
        }
    }

    /// Dart's `_maybeNotifyStatusListeners`.
    fn maybe_notify_status_listeners(self: Handle<Self>, app: &mut App, _status: AnimationStatus) {
        let status = self.as_animation().status(app);
        if Some(status) != self.last_status(app) {
            *self.last_status_mut(app) = Some(status);
            self.notify_status_listeners(app, status);
        }
    }

    /// Dart's `_maybeNotifyListeners`.
    fn maybe_notify_listeners(self: Handle<Self>, app: &mut App) {
        let value = self.as_animation().value(app);
        if self.last_value(app).as_ref() != Some(&value) {
            *self.last_value_mut(app) = Some(value);
            self.notify_listeners(app);
        }
    }
}

/// Dart's `CompoundAnimation.didStartListening`: subscribes to both
/// sub-animations.
///
/// A free function rather than a provided method on [`CompoundAnimation`]
/// because the method it implements belongs to the
/// [`AnimationLazyListenerMixin`] supertrait: a provided method of the same
/// name here would make every `did_start_listening` call ambiguous. Each
/// implementor's [`AnimationLazyListenerMixin::did_start_listening`] delegates
/// here, exactly as it would to a trait default.
fn compound_did_start_listening<S, T>(this: Handle<S>, app: &mut App)
where
    S: CompoundAnimation<T>,
    T: Clone + PartialEq + 'static,
{
    let (first, next) = (this.first(app), this.next(app));
    first.add_listener(
        app,
        Listener::handle_method(this, <S as CompoundAnimation<T>>::maybe_notify_listeners),
    );
    first.add_status_listener(
        app,
        AnimationStatusListener::handle_method(
            this,
            <S as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
    next.add_listener(
        app,
        Listener::handle_method(this, <S as CompoundAnimation<T>>::maybe_notify_listeners),
    );
    next.add_status_listener(
        app,
        AnimationStatusListener::handle_method(
            this,
            <S as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
}

/// Dart's `CompoundAnimation.didStopListening`: unsubscribes from both
/// sub-animations. Free for the reason on
/// [`compound_did_start_listening`]; the tear-offs it rebuilds are the same
/// fn items registered there, so they match.
fn compound_did_stop_listening<S, T>(this: Handle<S>, app: &mut App)
where
    S: CompoundAnimation<T>,
    T: Clone + PartialEq + 'static,
{
    let (first, next) = (this.first(app), this.next(app));
    first.remove_listener(
        app,
        &Listener::handle_method(this, <S as CompoundAnimation<T>>::maybe_notify_listeners),
    );
    first.remove_status_listener(
        app,
        &AnimationStatusListener::handle_method(
            this,
            <S as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
    next.remove_listener(
        app,
        &Listener::handle_method(this, <S as CompoundAnimation<T>>::maybe_notify_listeners),
    );
    next.remove_status_listener(
        app,
        &AnimationStatusListener::handle_method(
            this,
            <S as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
}

/// An animation of [`f64`]s that tracks the mean of two other animations.
///
/// The status of this animation is the status of the `right` animation if it
/// is moving, and the `left` animation otherwise.
///
/// The value of this animation is the [`f64`] that represents the mean value
/// of the values of the `left` and `right` animations.
pub struct AnimationMean {
    first: AnyAnimation<f64>,
    next: AnyAnimation<f64>,
    last_status: Option<AnimationStatus>,
    last_value: Option<f64>,
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl AnimationMean {
    /// Creates an animation that tracks the mean of two other animations.
    pub fn new(left: AnyAnimation<f64>, right: AnyAnimation<f64>) -> AnimationMean {
        AnimationMean {
            first: left,
            next: right,
            last_status: None,
            last_value: None,
            lazy_listener: AnimationLazyListenerData::new(),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }
}

impl AnimationLazyListenerMixin for AnimationMean {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        compound_did_start_listening::<AnimationMean, f64>(self, app)
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        compound_did_stop_listening::<AnimationMean, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for AnimationMean {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for AnimationMean {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for AnimationMean {
    fn first(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).first
    }

    fn next(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).next
    }

    fn last_status(self: Handle<Self>, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self: Handle<Self>, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self: Handle<Self>, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self: Handle<Self>, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMean extends CompoundAnimation<double>`.
impl Animation<f64> for AnimationMean {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        CompoundAnimation::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        let animation = app.get(self);
        (animation.first.value(app) + animation.next.value(app)) / 2.0
    }
}

/// `dart:math`'s `max` for doubles, written from its documented contract — the
/// Dart SDK source is not in the Flutter checkout. NaN propagates, and `0.0`
/// is the larger of the two zeros.
fn dart_max(a: f64, b: f64) -> f64 {
    if a > b {
        return a;
    }
    if b > a {
        return b;
    }
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a.is_sign_positive() { a } else { b }
}

/// `dart:math`'s `min` for doubles, from its documented contract. NaN
/// propagates, and `-0.0` is the smaller of the two zeros.
fn dart_min(a: f64, b: f64) -> f64 {
    if a < b {
        return a;
    }
    if b < a {
        return b;
    }
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a.is_sign_negative() { a } else { b }
}

/// An animation that tracks the maximum of two other animations.
///
/// The value of this animation is the maximum of the values of
/// [`first`](CompoundAnimation::first) and [`next`](CompoundAnimation::next).
///
/// Dart's `AnimationMax<T extends num>` is generic over `int` and `double`;
/// Rust has no `num` supertype, so this is `f64`-only until a non-double use
/// appears.
pub struct AnimationMax {
    first: AnyAnimation<f64>,
    next: AnyAnimation<f64>,
    last_status: Option<AnimationStatus>,
    last_value: Option<f64>,
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl AnimationMax {
    /// Creates an [`AnimationMax`].
    ///
    /// Either argument can be an [`AnimationMax`] itself to combine multiple
    /// animations.
    pub fn new(first: AnyAnimation<f64>, next: AnyAnimation<f64>) -> AnimationMax {
        AnimationMax {
            first,
            next,
            last_status: None,
            last_value: None,
            lazy_listener: AnimationLazyListenerData::new(),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }
}

impl AnimationLazyListenerMixin for AnimationMax {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        compound_did_start_listening::<AnimationMax, f64>(self, app)
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        compound_did_stop_listening::<AnimationMax, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for AnimationMax {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for AnimationMax {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for AnimationMax {
    fn first(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).first
    }

    fn next(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).next
    }

    fn last_status(self: Handle<Self>, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self: Handle<Self>, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self: Handle<Self>, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self: Handle<Self>, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMax<T extends num> extends CompoundAnimation<T>`.
impl Animation<f64> for AnimationMax {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        CompoundAnimation::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        let animation = app.get(self);
        dart_max(animation.first.value(app), animation.next.value(app))
    }
}

/// An animation that tracks the minimum of two other animations.
///
/// The value of this animation is the minimum of the values of
/// [`first`](CompoundAnimation::first) and [`next`](CompoundAnimation::next).
///
/// Dart's `AnimationMin<T extends num>` is generic over `int` and `double`;
/// Rust has no `num` supertype, so this is `f64`-only until a non-double use
/// appears.
pub struct AnimationMin {
    first: AnyAnimation<f64>,
    next: AnyAnimation<f64>,
    last_status: Option<AnimationStatus>,
    last_value: Option<f64>,
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl AnimationMin {
    /// Creates an [`AnimationMin`].
    ///
    /// Either argument can be an [`AnimationMin`] itself to combine multiple
    /// animations.
    pub fn new(first: AnyAnimation<f64>, next: AnyAnimation<f64>) -> AnimationMin {
        AnimationMin {
            first,
            next,
            last_status: None,
            last_value: None,
            lazy_listener: AnimationLazyListenerData::new(),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }
}

impl AnimationLazyListenerMixin for AnimationMin {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        compound_did_start_listening::<AnimationMin, f64>(self, app)
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        compound_did_stop_listening::<AnimationMin, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for AnimationMin {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for AnimationMin {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for AnimationMin {
    fn first(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).first
    }

    fn next(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).next
    }

    fn last_status(self: Handle<Self>, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self: Handle<Self>, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self: Handle<Self>, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self: Handle<Self>, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMin<T extends num> extends CompoundAnimation<T>`.
impl Animation<f64> for AnimationMin {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        CompoundAnimation::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        let animation = app.get(self);
        dart_min(animation.first.value(app), animation.next.value(app))
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::curves::Curves;

    #[test]
    fn the_constant_animations_report_flutters_values() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let complete = k_always_complete_animation(&mut app);
        assert_eq!(complete.value(&app), 1.0);
        assert_eq!(complete.status(&app), AnimationStatus::Completed);
        assert!(complete.is_completed(&app));

        let dismissed = k_always_dismissed_animation(&mut app);
        assert_eq!(dismissed.value(&app), 0.0);
        assert_eq!(dismissed.status(&app), AnimationStatus::Dismissed);
        assert!(dismissed.is_dismissed(&app));

        // Dart's overrides are empty bodies: registration is ignored, so
        // removal of something never added is also fine.
        complete.add_listener(&mut app, Listener::new(|_app| {}));
        complete.remove_listener(&mut app, &Listener::new(|_app| {}));
    }

    #[test]
    fn every_mention_of_a_constant_animation_is_one_identity() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = k_always_complete_animation(&mut app);

        // Dart's `const` canonicalization, reproduced per App by the
        // singleton: comparisons like ProxyAnimation's early return answer
        // "unchanged", as Flutter's do.
        assert_eq!(first, k_always_complete_animation(&mut app));
        assert_ne!(first, k_always_dismissed_animation(&mut app));
    }

    #[test]
    fn an_always_stopped_animation_is_forward_at_its_value() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();

        assert_eq!(half.value(&app), 0.5);
        assert_eq!(half.status(&app), AnimationStatus::Forward);
    }

    // animations_test.dart:51 — 'ProxyAnimation.toString control test', the
    // value/status half. `toString` is not ported.
    #[test]
    fn a_parentless_proxy_is_dismissed_at_zero() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let animation = ProxyAnimation::new(&mut app, None).as_animation();

        assert_eq!(animation.value(&app), 0.0);
        assert_eq!(animation.status(&app), AnimationStatus::Dismissed);
    }

    // animations_test.dart:60 — 'ProxyAnimation set parent generates value
    // changed'. The Dart test drives the second notification through an
    // AnimationController, which is not ported; here a chained inner proxy
    // takes that role, which also exercises the tear-off subscription.
    #[test]
    fn set_parent_generates_value_changed() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let did_receive_callback = Rc::new(Cell::new(false));

        let animation = ProxyAnimation::new(&mut app, None);
        animation.as_animation().add_listener(
            &mut app,
            Listener::new({
                let did_receive_callback = Rc::clone(&did_receive_callback);
                move |_app| did_receive_callback.set(true)
            }),
        );

        assert!(!did_receive_callback.get());
        animation.set_parent(&mut app, Some(half));
        assert!(did_receive_callback.get());
        did_receive_callback.set(false);
    }

    #[test]
    fn a_proxy_follows_its_parent_through_the_subscription() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let inner = ProxyAnimation::new(&mut app, None);
        let outer = ProxyAnimation::new(&mut app, Some(inner.as_animation()));

        let notified = Rc::new(Cell::new(0));
        outer.as_animation().add_listener(
            &mut app,
            Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| notified.set(notified.get() + 1)
            }),
        );

        // The inner proxy's own notification reaches the outer proxy's
        // listener through the tear-off the outer registered on it.
        let target = app.create(AlwaysStoppedAnimation::new(0.6)).as_animation();
        inner.set_parent(&mut app, Some(target));

        assert_eq!(notified.get(), 1);
        assert_eq!(outer.as_animation().value(&app), 0.6);
    }

    #[test]
    fn removing_the_last_listener_unsubscribes_from_the_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let inner = ProxyAnimation::new(&mut app, None);
        let outer = ProxyAnimation::new(&mut app, Some(inner.as_animation()));

        let listener = Listener::new(|_app| {});
        outer
            .as_animation()
            .add_listener(&mut app, listener.clone());
        assert!(
            !inner.local_listeners_data(&app).is_empty(),
            "the outer proxy's tear-off is registered on the inner"
        );
        assert!(!inner.local_status_listeners_data(&app).is_empty());

        outer.as_animation().remove_listener(&mut app, &listener);
        assert!(
            inner.local_listeners_data(&app).is_empty(),
            "the rebuilt tear-off matched, so the registration is gone"
        );
        assert!(inner.local_status_listeners_data(&app).is_empty());
    }

    #[test]
    fn set_parent_notifies_the_status_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let statuses = Rc::new(RefCell::new(Vec::new()));

        let animation = ProxyAnimation::new(&mut app, None); // Dismissed
        animation.as_animation().add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let statuses = Rc::clone(&statuses);
                move |status, _app| statuses.borrow_mut().push(status)
            }),
        );

        let complete = k_always_complete_animation(&mut app);
        animation.set_parent(&mut app, Some(complete));

        assert_eq!(*statuses.borrow(), vec![AnimationStatus::Completed]);
    }

    #[test]
    fn a_listener_can_reach_its_own_animation_while_it_notifies() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let animation = ProxyAnimation::new(&mut app, None);

        let observed = Rc::new(Cell::new(f64::NAN));
        animation.as_animation().add_listener(
            &mut app,
            Listener::new({
                let observed = Rc::clone(&observed);
                move |app: &mut App| {
                    // Mid-notification, from inside set_parent: the parent is
                    // already assigned, so the value reads through it — as in
                    // Dart, where the setter notifies after `_parent = value`.
                    observed.set(animation.as_animation().value(app));
                }
            }),
        );

        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        animation.set_parent(&mut app, Some(half));
        assert_eq!(observed.get(), 0.5);
    }

    #[test]
    fn setting_the_same_parent_is_a_no_op() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let animation = ProxyAnimation::new(&mut app, Some(half));

        let notified = Rc::new(Cell::new(0));
        animation.as_animation().add_listener(
            &mut app,
            Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| notified.set(notified.get() + 1)
            }),
        );

        animation.set_parent(&mut app, Some(half));
        assert_eq!(notified.get(), 0, "Dart returns before the swap");
    }

    #[test]
    fn a_reverse_animation_flips_value_and_status() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let stopped = app.create(AlwaysStoppedAnimation::new(0.25)).as_animation();
        let reverse = app.create(ReverseAnimation::new(stopped)).as_animation();

        assert_eq!(reverse.value(&app), 0.75);
        assert_eq!(reverse.status(&app), AnimationStatus::Reverse);
    }

    // animations_test.dart:77 — 'ReverseAnimation calls listeners': the value
    // listener is forwarded to the parent, and removal stops the calls. A
    // ProxyAnimation drives in place of the unported controller.
    #[test]
    fn reverse_animation_calls_listeners() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let driver = ProxyAnimation::new(&mut app, None);
        let animation = app.create(ReverseAnimation::new(driver.as_animation()));

        let did_receive_callback = Rc::new(Cell::new(false));
        let listener = Listener::new({
            let did_receive_callback = Rc::clone(&did_receive_callback);
            move |_app| did_receive_callback.set(true)
        });
        animation
            .as_animation()
            .add_listener(&mut app, listener.clone());

        assert!(!did_receive_callback.get());
        let target = app.create(AlwaysStoppedAnimation::new(0.6)).as_animation();
        driver.set_parent(&mut app, Some(target));
        assert!(did_receive_callback.get());
        did_receive_callback.set(false);

        animation
            .as_animation()
            .remove_listener(&mut app, &listener);
        assert!(!did_receive_callback.get());
        let target = app.create(AlwaysStoppedAnimation::new(0.7)).as_animation();
        driver.set_parent(&mut app, Some(target));
        assert!(!did_receive_callback.get());
    }

    // The `_statusChangeHandler` half: status notifications arrive reversed.
    #[test]
    fn a_reverse_animation_reverses_status_notifications() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let driver = ProxyAnimation::new(&mut app, None); // Dismissed
        let reverse = app.create(ReverseAnimation::new(driver.as_animation()));

        let statuses = Rc::new(RefCell::new(Vec::new()));
        let listener = AnimationStatusListener::new({
            let statuses = Rc::clone(&statuses);
            move |status, _app| statuses.borrow_mut().push(status)
        });
        reverse
            .as_animation()
            .add_status_listener(&mut app, listener.clone());
        assert!(
            !driver.local_status_listeners_data(&app).is_empty(),
            "lazily subscribed to the driver"
        );

        let complete = k_always_complete_animation(&mut app);
        driver.set_parent(&mut app, Some(complete));

        assert_eq!(*statuses.borrow(), vec![AnimationStatus::Dismissed]);
        assert_eq!(
            reverse.as_animation().status(&app),
            AnimationStatus::Dismissed
        );

        reverse
            .as_animation()
            .remove_status_listener(&mut app, &listener);
        assert!(
            driver.local_status_listeners_data(&app).is_empty(),
            "the rebuilt tear-off matched, so the subscription is gone"
        );
    }

    #[test]
    fn a_curved_animation_applies_the_curve() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let curved = CurvedAnimation::create(&mut app, half, Curves::ease(), None);

        assert_eq!(
            curved.as_animation().value(&app),
            Curves::ease().transform(0.5)
        );
        assert_eq!(curved.as_animation().status(&app), AnimationStatus::Forward);
    }

    #[test]
    fn a_curved_animation_uses_the_reverse_curve_when_the_direction_is_reverse() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        // A parent with status Reverse: a ReverseAnimation over an
        // always-Forward stopped animation at 0.25, so its value is 0.75.
        let stopped = app.create(AlwaysStoppedAnimation::new(0.25)).as_animation();
        let parent = app.create(ReverseAnimation::new(stopped)).as_animation();
        let curved =
            CurvedAnimation::create(&mut app, parent, Curves::ease(), Some(Curves::ease_out()));

        // The constructor saw status Reverse while animating, so the curve
        // direction is pinned to Reverse and the reverse curve is active.
        assert_eq!(
            curved.as_animation().value(&app),
            Curves::ease_out().transform(0.75)
        );
    }

    // animations_test.dart:251 — 'CurvedAnimation with bogus curve'. Dart
    // throws a FlutterError from inside an assert; ours is the debug-only
    // panic, so the test exists only in debug builds.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Invalid curve endpoint")]
    fn a_curve_that_moves_an_endpoint_panics_in_debug() {
        #[derive(Debug)]
        struct BogusCurve;

        impl Curve for BogusCurve {
            fn transform(&self, _t: f64) -> f64 {
                100.0
            }
        }

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let zero = app.create(AlwaysStoppedAnimation::new(0.0)).as_animation();
        let curved = CurvedAnimation::create(&mut app, zero, Rc::new(BogusCurve), None);
        let _ = curved.as_animation().value(&app);
    }

    // animations_test.dart:326 — 'CurvedAnimation stops listening to parent
    // when disposed', the subscription half.
    #[test]
    fn a_disposed_curved_animation_unsubscribes_from_its_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let driver = ProxyAnimation::new(&mut app, None);
        let curved = CurvedAnimation::create(&mut app, driver.as_animation(), Curves::ease(), None);
        assert!(!driver.local_status_listeners_data(&app).is_empty());

        curved.dispose(&mut app);
        assert!(driver.local_status_listeners_data(&app).is_empty());
        assert!(app.get(curved).is_disposed);
    }

    // animations_test.dart:97 — 'TrainHoppingAnimation', with ProxyAnimations
    // as the trains in place of the unported controllers.
    #[test]
    fn a_train_hopping_animation_hops_when_the_next_train_crosses() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let low = app.create(AlwaysStoppedAnimation::new(0.3)).as_animation();
        let train_a = ProxyAnimation::new(&mut app, Some(half));
        let train_b = ProxyAnimation::new(&mut app, Some(low));

        let switched = Rc::new(Cell::new(false));
        let train = TrainHoppingAnimation::create(
            &mut app,
            train_a.as_animation(),
            Some(train_b.as_animation()),
            Some(Listener::new({
                let switched = Rc::clone(&switched);
                move |_app| switched.set(true)
            })),
        );

        assert_eq!(train.as_animation().value(&app), 0.5);
        assert!(!switched.get());

        // Drive the next train past the current one: 0.3 → 0.75.
        let high = app.create(AlwaysStoppedAnimation::new(0.75)).as_animation();
        train_b.set_parent(&mut app, Some(high));

        assert!(switched.get(), "the hop fired the callback");
        assert_eq!(app.get(train).current_train(), Some(train_b.as_animation()));
        assert_eq!(train.as_animation().value(&app), 0.75);
    }

    #[test]
    fn trains_starting_at_the_same_value_hop_immediately_without_the_callback() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let b = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();

        let switched = Rc::new(Cell::new(false));
        let train = TrainHoppingAnimation::create(
            &mut app,
            a,
            Some(b),
            Some(Listener::new({
                let switched = Rc::clone(&switched);
                move |_app| switched.set(true)
            })),
        );

        assert_eq!(app.get(train).current_train(), Some(b));
        assert!(!switched.get(), "Dart never calls onSwitchedTrain here");
    }

    #[test]
    fn a_disposed_train_hopping_animation_releases_both_trains() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let low = app.create(AlwaysStoppedAnimation::new(0.3)).as_animation();
        let train_a = ProxyAnimation::new(&mut app, Some(half));
        let train_b = ProxyAnimation::new(&mut app, Some(low));

        let train = TrainHoppingAnimation::create(
            &mut app,
            train_a.as_animation(),
            Some(train_b.as_animation()),
            None,
        );
        assert!(!train_a.local_listeners_data(&app).is_empty());
        assert!(!train_a.local_status_listeners_data(&app).is_empty());
        assert!(!train_b.local_listeners_data(&app).is_empty());

        train.dispose(&mut app);
        assert!(train_a.local_listeners_data(&app).is_empty());
        assert!(train_a.local_status_listeners_data(&app).is_empty());
        assert!(train_b.local_listeners_data(&app).is_empty());
        assert!(app.get(train).current_train().is_none());
    }

    // animations_test.dart:161 — 'AnimationMean control test': Dart's values,
    // with ProxyAnimations as the mutable controllers, including the
    // removeListener half.
    #[test]
    fn animation_mean_control_test() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let left = ProxyAnimation::new(&mut app, Some(half));
        let right = ProxyAnimation::new(&mut app, None); // 0.0

        let mean = app.create(AnimationMean::new(
            left.as_animation(),
            right.as_animation(),
        ));
        let mean_handle = mean.as_animation();
        assert_eq!(mean_handle.value(&app), 0.25);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_value = Listener::new({
            let log = Rc::clone(&log);
            move |app: &mut App| {
                let value = mean.as_animation().value(app);
                log.borrow_mut().push(value);
            }
        });
        mean_handle.add_listener(&mut app, log_value.clone());

        let one = app.create(AlwaysStoppedAnimation::new(1.0)).as_animation();
        right.set_parent(&mut app, Some(one));

        assert_eq!(mean_handle.value(&app), 0.75);
        assert_eq!(*log.borrow(), vec![0.75]);
        log.borrow_mut().clear();

        mean_handle.remove_listener(&mut app, &log_value);

        let zero = app.create(AlwaysStoppedAnimation::new(0.0)).as_animation();
        left.set_parent(&mut app, Some(zero));

        assert_eq!(mean_handle.value(&app), 0.50);
        assert!(log.borrow().is_empty());
    }

    // animations_test.dart:136 — 'TrainHoppingAnimation notifies status
    // listeners'. Dart drives two opposed tweens off one controller; here the
    // controller's place is taken by a ProxyAnimation whose parent swaps
    // change its value and status together.
    #[test]
    fn train_hopping_notifies_status_listeners() {
        use crate::tween::{Animatable, Tween};

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let driver = ProxyAnimation::new(&mut app, None); // Dismissed, 0.0
        let falling = Tween::new(&mut app, Some(1.0), Some(-1.0));
        let rising = Tween::new(&mut app, Some(-1.0), Some(1.0));
        let current = falling.animate(&mut app, driver.as_animation());
        let next = rising.animate(&mut app, driver.as_animation());

        let animation = TrainHoppingAnimation::create(&mut app, current, Some(next), None);

        let status_log = Rc::new(RefCell::new(Vec::new()));
        animation.as_animation().add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let status_log = Rc::clone(&status_log);
                move |status, _app| status_log.borrow_mut().push(status)
            }),
        );
        assert!(status_log.borrow().is_empty());

        // Dart: `controller.forward()` — status Forward; the trains cross at
        // t = 0.5, which hops to the rising train.
        let half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        driver.set_parent(&mut app, Some(half));
        assert_eq!(*status_log.borrow(), vec![AnimationStatus::Forward]);
        status_log.borrow_mut().clear();

        // Dart: `controller.reverse()` from 0.0 — status Dismissed.
        let dismissed = k_always_dismissed_animation(&mut app);
        driver.set_parent(&mut app, Some(dismissed));
        assert_eq!(*status_log.borrow(), vec![AnimationStatus::Dismissed]);
    }

    // animations_test.dart:326 — 'CurvedAnimation stops listening to parent
    // when disposed', including the frozen curve direction afterwards.
    //
    #[test]
    fn curved_animation_stops_listening_to_parent_when_disposed() {
        use crate::curves::Interval;

        let forward_curve = Rc::new(Interval::new(0.0, 0.5, Curves::linear()));
        let reverse_curve = Rc::new(Interval::new(0.5, 1.0, Curves::linear()));
        assert_eq!(forward_curve.transform(0.5), 1.0);
        assert_eq!(reverse_curve.transform(0.5), 0.0);

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        // Forward at 0.5, and Reverse at 0.5 (a ReverseAnimation over 0.5
        // keeps the value while flipping the direction).
        let forward_half = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
        let reverse_half = {
            let stopped = app.create(AlwaysStoppedAnimation::new(0.5)).as_animation();
            app.create(ReverseAnimation::new(stopped)).as_animation()
        };

        let parent = ProxyAnimation::new(&mut app, Some(forward_half));
        let curved = CurvedAnimation::create(
            &mut app,
            parent.as_animation(),
            forward_curve,
            Some(reverse_curve),
        );
        let curved_handle = curved.as_animation();

        // Forward at 0.5: the forward interval maps it to 1.0.
        assert_eq!(curved_handle.value(&app), 1.0);

        // Reach Completed so the curve direction resets, as the Dart test
        // does with `controller.value = 1.0`, then go Reverse.
        let complete = k_always_complete_animation(&mut app);
        parent.set_parent(&mut app, Some(complete));
        parent.set_parent(&mut app, Some(reverse_half));
        assert_eq!(curved_handle.value(&app), 0.0);

        assert!(!app.get(curved).is_disposed);
        curved.dispose(&mut app);
        assert!(app.get(curved).is_disposed);

        // Dismissed, then Forward at 0.5 again: were it still listening the
        // direction would reset and the forward curve would give 1.0; frozen
        // at Reverse, the reverse curve gives 0.0.
        let dismissed = k_always_dismissed_animation(&mut app);
        parent.set_parent(&mut app, Some(dismissed));
        parent.set_parent(&mut app, Some(forward_half));
        assert_eq!(curved_handle.value(&app), 0.0);
    }

    #[test]
    fn animation_max_and_min_pick_their_extremes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let low = app.create(AlwaysStoppedAnimation::new(0.25)).as_animation();
        let high = app.create(AlwaysStoppedAnimation::new(0.75)).as_animation();

        let max = app.create(AnimationMax::new(low, high)).as_animation();
        let min = app.create(AnimationMin::new(low, high)).as_animation();

        assert_eq!(max.value(&app), 0.75);
        assert_eq!(min.value(&app), 0.25);
    }

    #[test]
    fn dart_max_and_min_follow_the_documented_contract() {
        // NaN propagates from either side — unlike `f64::max`, which drops it.
        assert!(dart_max(f64::NAN, 1.0).is_nan());
        assert!(dart_max(1.0, f64::NAN).is_nan());
        assert!(dart_min(f64::NAN, 1.0).is_nan());
        assert!(dart_min(1.0, f64::NAN).is_nan());

        // 0.0 is the larger zero, -0.0 the smaller.
        assert!(dart_max(0.0, -0.0).is_sign_positive());
        assert!(dart_max(-0.0, 0.0).is_sign_positive());
        assert!(dart_min(0.0, -0.0).is_sign_negative());
        assert!(dart_min(-0.0, 0.0).is_sign_negative());

        assert_eq!(dart_max(0.25, 0.75), 0.75);
        assert_eq!(dart_min(0.25, 0.75), 0.25);
    }
}

//! Flutter counterpart: `animation/animations.dart`.

use std::rc::Rc;

use reveal_foundation::{App, Handle, Listenable, Listener};

use crate::animation::{Animation, AnimationNode, AnimationStatus, AnimationStatusListener};
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

impl AnimationNode<f64> for AlwaysCompleteAnimation {
    fn add_listener(_app: &mut App, _this: Handle<Self>, _listener: Listener) {}

    fn remove_listener(_app: &mut App, _this: Handle<Self>, _listener: &Listener) {}

    fn add_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: AnimationStatusListener,
    ) {
    }

    fn remove_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(_app: &App, _this: Handle<Self>) -> AnimationStatus {
        AnimationStatus::Completed
    }

    fn value(_app: &App, _this: Handle<Self>) -> f64 {
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
pub fn k_always_complete_animation(app: &mut App) -> Animation<f64> {
    Animation::from_handle(app.singleton::<AlwaysCompleteAnimation>())
}

/// Dart's `_AlwaysDismissedAnimation`. Private like the original; reach it
/// through [`k_always_dismissed_animation`].
#[derive(Default)]
struct AlwaysDismissedAnimation;

impl AnimationNode<f64> for AlwaysDismissedAnimation {
    fn add_listener(_app: &mut App, _this: Handle<Self>, _listener: Listener) {}

    fn remove_listener(_app: &mut App, _this: Handle<Self>, _listener: &Listener) {}

    fn add_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: AnimationStatusListener,
    ) {
    }

    fn remove_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(_app: &App, _this: Handle<Self>) -> AnimationStatus {
        AnimationStatus::Dismissed
    }

    fn value(_app: &App, _this: Handle<Self>) -> f64 {
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
pub fn k_always_dismissed_animation(app: &mut App) -> Animation<f64> {
    Animation::from_handle(app.singleton::<AlwaysDismissedAnimation>())
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

impl<T: Clone + 'static> AnimationNode<T> for AlwaysStoppedAnimation<T> {
    fn add_listener(_app: &mut App, _this: Handle<Self>, _listener: Listener) {}

    fn remove_listener(_app: &mut App, _this: Handle<Self>, _listener: &Listener) {}

    fn add_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: AnimationStatusListener,
    ) {
    }

    fn remove_status_listener(
        _app: &mut App,
        _this: Handle<Self>,
        _listener: &AnimationStatusListener,
    ) {
    }

    fn status(_app: &App, _this: Handle<Self>) -> AnimationStatus {
        AnimationStatus::Forward
    }

    fn value(app: &App, this: Handle<Self>) -> T {
        app.get(this).value.clone()
    }
}

/// Implements most of the animation interface by deferring its behavior to a
/// given [`parent`] animation.
///
/// To implement an animation that is driven by a parent, it is only necessary
/// to implement this trait, supply [`parent`], and implement
/// [`AnimationNode::value`].
///
/// To define a mapping from values in the range 0..1, consider subclassing
/// `Tween` instead.
///
/// [`parent`]: AnimationWithParent::parent
pub trait AnimationWithParent<T: 'static>: Copy + 'static {
    /// The animation whose value this animation will proxy.
    ///
    /// This animation must remain the same for the lifetime of this object. If
    /// you wish to proxy a different animation at different times, consider
    /// using [`ProxyAnimation`].
    fn parent(self, app: &App) -> Animation<T>;

    /// Calls the listener every time the value of the animation changes.
    ///
    /// Listeners can be removed with [`remove_listener`].
    ///
    /// [`remove_listener`]: AnimationWithParent::remove_listener
    fn add_listener(self, app: &mut App, listener: Listener) {
        let parent = self.parent(app);
        parent.add_listener(app, listener);
    }

    /// Stop calling the listener every time the value of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_listener`].
    ///
    /// [`add_listener`]: AnimationWithParent::add_listener
    fn remove_listener(self, app: &mut App, listener: &Listener) {
        let parent = self.parent(app);
        parent.remove_listener(app, listener);
    }

    /// Calls listener every time the status of the animation changes.
    ///
    /// Listeners can be removed with [`remove_status_listener`].
    ///
    /// [`remove_status_listener`]: AnimationWithParent::remove_status_listener
    fn add_status_listener(self, app: &mut App, listener: AnimationStatusListener) {
        let parent = self.parent(app);
        parent.add_status_listener(app, listener);
    }

    /// Stops calling the listener every time the status of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_status_listener`].
    ///
    /// [`add_status_listener`]: AnimationWithParent::add_status_listener
    fn remove_status_listener(self, app: &mut App, listener: &AnimationStatusListener) {
        let parent = self.parent(app);
        parent.remove_status_listener(app, listener);
    }

    /// The current status of this animation.
    fn status(self, app: &App) -> AnimationStatus {
        self.parent(app).status(app)
    }
}

/// An animation that is a proxy for another animation.
///
/// A proxy animation is useful because the parent animation can be mutated.
/// For example, one object can create a proxy animation, hand the proxy to
/// another object, and then later change the animation from which the proxy
/// receives its value.
#[derive(Clone, Copy)]
pub struct ProxyAnimation(Handle<ProxyAnimationData>);

struct ProxyAnimationData {
    status: Option<AnimationStatus>,
    value: Option<f64>,
    parent: Option<Animation<f64>>,

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
    pub fn new(app: &mut App, animation: Option<Animation<f64>>) -> ProxyAnimation {
        let mut result = ProxyAnimationData {
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
        ProxyAnimation(app.create(result))
    }

    /// The animation whose value this animation will proxy.
    ///
    /// This value is mutable ([`set_parent`]). When mutated, the listeners on
    /// the proxy animation will be transparently updated to be listening to
    /// the new parent animation.
    /// This proxy as an [`Animation<f64>`].
    pub fn as_animation(self) -> Animation<f64> {
        Animation::from_handle(self.0)
    }

    /// Chains a `Tween` (or any [`Animatable`]) to this proxy.
    ///
    /// Dart inherits `drive` from `Animation<double>`. The newtype is not a
    /// subtype, so the method is inherent and forwards to [`as_animation`].
    pub fn drive<U: 'static>(
        self,
        app: &mut App,
        child: impl Animatable<U> + Clone + 'static,
    ) -> Animation<U> {
        self.as_animation().drive(app, child)
    }

    /// The animation whose value this animation will proxy.
    pub fn parent(self, app: &App) -> Option<Animation<f64>> {
        app.get(self.0).parent
    }

    /// Changes the animation whose value this animation proxies.
    ///
    /// Dart's `parent` setter, `animations.dart:200`.
    pub fn set_parent(self, app: &mut App, value: Option<Animation<f64>>) {
        if value == app.get(self.0).parent {
            return;
        }
        if let Some(parent) = app.get(self.0).parent {
            let (status, current) = (parent.status(app), parent.value(app));
            let proxy = app.get_mut(self.0);
            proxy.status = Some(status);
            proxy.value = Some(current);
            if self.is_listening(app) {
                self.did_stop_listening(app);
            }
        }
        app.get_mut(self.0).parent = value;
        if let Some(parent) = app.get(self.0).parent {
            if self.is_listening(app) {
                self.did_start_listening(app);
            }
            if app.get(self.0).value != Some(parent.value(app)) {
                self.notify_listeners(app);
            }
            let status = parent.status(app);
            if app.get(self.0).status != Some(status) {
                self.notify_status_listeners(app, status);
            }
            let proxy = app.get_mut(self.0);
            proxy.status = None;
            proxy.value = None;
        }
    }
}

impl AnimationLazyListenerMixin for ProxyAnimation {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData {
        &app.get(self.0).lazy_listener
    }

    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self.0).lazy_listener
    }

    fn did_start_listening(self, app: &mut App) {
        if let Some(parent) = app.get(self.0).parent {
            // Dart: `_parent!.addListener(notifyListeners)` — a tear-off.
            // `handle_method` gives the rebuilt listener the same identity, so
            // `did_stop_listening` removes without a stored handle.
            parent.add_listener(app, Listener::handle_method(self.0, proxy_notify_listeners));
            parent.add_status_listener(
                app,
                AnimationStatusListener::handle_method(self.0, proxy_notify_status_listeners),
            );
        }
    }

    fn did_stop_listening(self, app: &mut App) {
        if let Some(parent) = app.get(self.0).parent {
            parent.remove_listener(
                app,
                &Listener::handle_method(self.0, proxy_notify_listeners),
            );
            parent.remove_status_listener(
                app,
                &AnimationStatusListener::handle_method(self.0, proxy_notify_status_listeners),
            );
        }
    }
}

impl AnimationLocalListenersMixin for ProxyAnimation {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self.0).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self.0).local_listeners
    }

    // Dart resolves these two by mixin order — `AnimationLazyListenerMixin` is
    // the one that declares them in ProxyAnimation's `with` clause. Rust has
    // no such rule, so the choice is written out.
    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for ProxyAnimation {
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
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

fn proxy_notify_listeners(this: Handle<ProxyAnimationData>, app: &mut App) {
    AnimationLocalListenersMixin::notify_listeners(ProxyAnimation(this), app);
}

fn proxy_notify_status_listeners(
    this: Handle<ProxyAnimationData>,
    app: &mut App,
    status: AnimationStatus,
) {
    AnimationLocalStatusListenersMixin::notify_status_listeners(ProxyAnimation(this), app, status);
}

impl Listenable for ProxyAnimation {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        self.as_animation().add_listener(app, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        self.as_animation().remove_listener(app, listener);
    }
}

// Dart: `class ProxyAnimation extends Animation<double> with
// AnimationLazyListenerMixin, AnimationLocalListenersMixin,
// AnimationLocalStatusListenersMixin`. The forwarding below is the `with`
// clause: the mixins supply the listener protocol.
impl AnimationNode<f64> for ProxyAnimationData {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(ProxyAnimation(this), app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(ProxyAnimation(this), app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(ProxyAnimation(this), app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(
            ProxyAnimation(this),
            app,
            listener,
        )
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        match app.get(this).parent {
            Some(parent) => parent.status(app),
            None => app
                .get(this)
                .status
                .expect("a ProxyAnimation without a parent keeps its last status"),
        }
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        match app.get(this).parent {
            Some(parent) => parent.value(app),
            None => app
                .get(this)
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
    parent: Animation<f64>,

    // `AnimationLazyListenerMixin` and `AnimationLocalStatusListenersMixin`
    // state.
    lazy_listener: AnimationLazyListenerData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl ReverseAnimation {
    /// Creates a reverse animation.
    pub fn new(parent: Animation<f64>) -> ReverseAnimation {
        ReverseAnimation {
            parent,
            lazy_listener: AnimationLazyListenerData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }

    /// The animation whose value and direction this animation is reversing.
    pub fn parent(&self) -> Animation<f64> {
        self.parent
    }

    fn status_change_handler(this: Handle<Self>, app: &mut App, status: AnimationStatus) {
        this.notify_status_listeners(app, Self::reverse_status(status));
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

impl AnimationLazyListenerMixin for Handle<ReverseAnimation> {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self, app: &mut App) {
        let parent = app.get(self).parent;
        parent.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, ReverseAnimation::status_change_handler),
        );
    }

    fn did_stop_listening(self, app: &mut App) {
        let parent = app.get(self).parent;
        parent.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, ReverseAnimation::status_change_handler),
        );
    }
}

impl AnimationLocalStatusListenersMixin for Handle<ReverseAnimation> {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

// Dart: `class ReverseAnimation extends Animation<double> with
// AnimationLazyListenerMixin, AnimationLocalStatusListenersMixin`. The value
// listeners are the class's own overrides: they forward to the parent, but
// count through the lazy mixin so the status subscription follows them.
impl AnimationNode<f64> for ReverseAnimation {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLazyListenerMixin::did_register_listener(this, app);
        let parent = app.get(this).parent;
        parent.add_listener(app, listener);
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        let parent = app.get(this).parent;
        parent.remove_listener(app, listener);
        AnimationLazyListenerMixin::did_unregister_listener(this, app);
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        Self::reverse_status(app.get(this).parent.status(app))
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        1.0 - app.get(this).parent.value(app)
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
    parent: Animation<f64>,

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
        parent: Animation<f64>,
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
        Self::update_curve_direction(this, app, status);
        parent.add_status_listener(
            app,
            AnimationStatusListener::handle_method(this, Self::update_curve_direction),
        );
        this
    }

    fn update_curve_direction(this: Handle<Self>, app: &mut App, status: AnimationStatus) {
        let animation = app.get_mut(this);
        animation.curve_direction = if status.is_animating() {
            animation.curve_direction.or(Some(status))
        } else {
            None
        };
    }

    fn use_forward_curve(app: &App, this: Handle<Self>) -> bool {
        let animation = app.get(this);
        animation.reverse_curve.is_none()
            || animation
                .curve_direction
                .unwrap_or_else(|| animation.parent.status(app))
                != AnimationStatus::Reverse
    }

    /// Cleans up any listeners added by this CurvedAnimation.
    pub fn dispose(app: &mut App, this: Handle<Self>) {
        app.get_mut(this).is_disposed = true;
        let parent = app.get(this).parent;
        parent.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(this, Self::update_curve_direction),
        );
    }
}

impl AnimationWithParent<f64> for Handle<CurvedAnimation> {
    fn parent(self, app: &App) -> Animation<f64> {
        app.get(self).parent
    }
}

// Dart: `class CurvedAnimation extends Animation<double> with
// AnimationWithParentMixin<double>`.
impl AnimationNode<f64> for CurvedAnimation {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationWithParent::add_listener(this, app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationWithParent::remove_listener(this, app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationWithParent::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationWithParent::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        AnimationWithParent::status(this, app)
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        let animation = app.get(this);
        let active_curve = if Self::use_forward_curve(app, this) {
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
    current_train: Option<Animation<f64>>,
    next_train: Option<Animation<f64>>,
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
        current_train: Animation<f64>,
        next_train: Option<Animation<f64>>,
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
    pub fn current_train(&self) -> Option<Animation<f64>> {
        self.current_train
    }

    fn status_change_handler(this: Handle<Self>, app: &mut App, status: AnimationStatus) {
        debug_assert!(app.get(this).current_train.is_some());
        if Some(status) != app.get(this).last_status {
            this.notify_status_listeners(app, status);
            app.get_mut(this).last_status = Some(status);
        }
        debug_assert!(app.get(this).last_status.is_some());
    }

    fn value_change_handler(this: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(this).current_train.is_some());
        let mut hop = false;
        if app.get(this).next_train.is_some() {
            debug_assert!(app.get(this).mode.is_some());
            let animation = app.get(this);
            let current = animation.current_train.unwrap();
            let next = animation.next_train.unwrap();
            hop = match animation.mode.unwrap() {
                TrainHoppingMode::Minimize => next.value(app) <= current.value(app),
                TrainHoppingMode::Maximize => next.value(app) >= current.value(app),
            };
            if hop {
                current.remove_status_listener(
                    app,
                    &AnimationStatusListener::handle_method(this, Self::status_change_handler),
                );
                current.remove_listener(
                    app,
                    &Listener::handle_method(this, Self::value_change_handler),
                );
                let animation = app.get_mut(this);
                animation.current_train = animation.next_train.take();
                let new_current = app.get(this).current_train.unwrap();
                new_current.add_status_listener(
                    app,
                    AnimationStatusListener::handle_method(this, Self::status_change_handler),
                );
                let status = new_current.status(app);
                Self::status_change_handler(this, app, status);
            }
        }
        let new_value = Self::value(app, this);
        if Some(new_value) != app.get(this).last_value {
            this.notify_listeners(app);
            app.get_mut(this).last_value = Some(new_value);
        }
        debug_assert!(app.get(this).last_value.is_some());
        if hop && let Some(on_switched_train) = app.get(this).on_switched_train.clone() {
            on_switched_train.call(app);
        }
    }

    /// Frees all the resources used by this performance.
    /// After this is called, this object is no longer usable.
    pub fn dispose(app: &mut App, this: Handle<Self>) {
        debug_assert!(app.get(this).current_train.is_some());
        let current = app
            .get(this)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train");
        current.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(this, Self::status_change_handler),
        );
        current.remove_listener(
            app,
            &Listener::handle_method(this, Self::value_change_handler),
        );
        app.get_mut(this).current_train = None;
        if let Some(next) = app.get(this).next_train {
            next.remove_listener(
                app,
                &Listener::handle_method(this, Self::value_change_handler),
            );
        }
        app.get_mut(this).next_train = None;
        this.clear_listeners(app);
        this.clear_status_listeners(app);
        AnimationEagerListenerMixin::dispose(this, app);
    }
}

impl AnimationEagerListenerMixin for Handle<TrainHoppingAnimation> {}

impl AnimationLocalListenersMixin for Handle<TrainHoppingAnimation> {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    // Dart resolves these by mixin order — `AnimationEagerListenerMixin` is
    // the one that declares them in this class's `with` clause.
    fn did_register_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for Handle<TrainHoppingAnimation> {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationEagerListenerMixin::did_unregister_listener(self, app)
    }
}

// Dart: `class TrainHoppingAnimation extends Animation<double> with
// AnimationEagerListenerMixin, AnimationLocalListenersMixin,
// AnimationLocalStatusListenersMixin`.
impl AnimationNode<f64> for TrainHoppingAnimation {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(this, app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(this, app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        app.get(this)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train")
            .status(app)
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        app.get(this)
            .current_train
            .expect("a disposed TrainHoppingAnimation has no current train")
            .value(app)
    }
}

/// An interface for combining multiple Animations. Implementors need only
/// implement [`AnimationNode::value`] to control how the child animations are
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
/// it reaches the override through [`animation`], the erased handle, whose ops
/// table is built from the subclass's [`AnimationNode`] impl. The fields Dart
/// declares on the abstract class belong to the implementing state type and
/// reach the trait through the app-mediated accessors.
///
/// [`animation`]: CompoundAnimation::animation
/// [`first`]: CompoundAnimation::first
/// [`next`]: CompoundAnimation::next
pub trait CompoundAnimation<T: Clone + PartialEq + 'static>:
    AnimationLazyListenerMixin + AnimationLocalListenersMixin + AnimationLocalStatusListenersMixin
{
    /// This animation as the erased [`Animation`] handle —
    /// `Animation::from_handle(self)` in every implementor.
    ///
    /// [`AnimationNode`] stays on the state type, since that is what
    /// `from_handle` erases, so this is how the handlers below reach a
    /// subclass's `value` and `status` overrides: the same dispatch target the
    /// virtual call had.
    fn animation(self) -> Animation<T>;

    /// The first sub-animation. Its status takes precedence if neither are
    /// animating.
    fn first(self, app: &App) -> Animation<T>;

    /// The second sub-animation.
    fn next(self, app: &App) -> Animation<T>;

    /// Dart's `_lastStatus`, owned by the implementing type.
    fn last_status(self, app: &App) -> Option<AnimationStatus>;

    /// Dart's `_lastStatus`, mutably.
    fn last_status_mut(self, app: &mut App) -> &mut Option<AnimationStatus>;

    /// Dart's `_lastValue`, owned by the implementing type.
    fn last_value(self, app: &App) -> &Option<T>;

    /// Dart's `_lastValue`, mutably.
    fn last_value_mut(self, app: &mut App) -> &mut Option<T>;

    /// Gets the status of this animation based on the [`first`] and [`next`]
    /// status.
    ///
    /// The default is that if the [`next`] animation is moving, use its
    /// status. Otherwise, default to [`first`].
    ///
    /// [`first`]: CompoundAnimation::first
    /// [`next`]: CompoundAnimation::next
    fn status(self, app: &App) -> AnimationStatus {
        let next_status = self.next(app).status(app);
        if next_status.is_animating() {
            next_status
        } else {
            self.first(app).status(app)
        }
    }

    /// Dart's `_maybeNotifyStatusListeners`.
    fn maybe_notify_status_listeners(self, app: &mut App, _status: AnimationStatus) {
        let status = self.animation().status(app);
        if Some(status) != self.last_status(app) {
            *self.last_status_mut(app) = Some(status);
            self.notify_status_listeners(app, status);
        }
    }

    /// Dart's `_maybeNotifyListeners`.
    fn maybe_notify_listeners(self, app: &mut App) {
        let value = self.animation().value(app);
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
/// because the tear-offs it registers name the entity, and inside the trait
/// `Self` is only known to be *some* `Copy + 'static` handle — Rust cannot see
/// that it is an `Handle<S>`, which is what `Listener::handle_method` takes.
/// Each implementor's [`AnimationLazyListenerMixin::did_start_listening`] delegates
/// here, exactly as it would to a trait default.
fn compound_did_start_listening<S: 'static, T>(this: Handle<S>, app: &mut App)
where
    T: Clone + PartialEq + 'static,
    Handle<S>: CompoundAnimation<T>,
{
    let (first, next) = (this.first(app), this.next(app));
    first.add_listener(
        app,
        Listener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_listeners,
        ),
    );
    first.add_status_listener(
        app,
        AnimationStatusListener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
    next.add_listener(
        app,
        Listener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_listeners,
        ),
    );
    next.add_status_listener(
        app,
        AnimationStatusListener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
}

/// Dart's `CompoundAnimation.didStopListening`: unsubscribes from both
/// sub-animations. Free for the reason on
/// [`compound_did_start_listening`]; the tear-offs it rebuilds are the same
/// fn items registered there, so they match.
fn compound_did_stop_listening<S: 'static, T>(this: Handle<S>, app: &mut App)
where
    T: Clone + PartialEq + 'static,
    Handle<S>: CompoundAnimation<T>,
{
    let (first, next) = (this.first(app), this.next(app));
    first.remove_listener(
        app,
        &Listener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_listeners,
        ),
    );
    first.remove_status_listener(
        app,
        &AnimationStatusListener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_status_listeners,
        ),
    );
    next.remove_listener(
        app,
        &Listener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_listeners,
        ),
    );
    next.remove_status_listener(
        app,
        &AnimationStatusListener::handle_method(
            this,
            <Handle<S> as CompoundAnimation<T>>::maybe_notify_status_listeners,
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
    first: Animation<f64>,
    next: Animation<f64>,
    last_status: Option<AnimationStatus>,
    last_value: Option<f64>,
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl AnimationMean {
    /// Creates an animation that tracks the mean of two other animations.
    pub fn new(left: Animation<f64>, right: Animation<f64>) -> AnimationMean {
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

impl AnimationLazyListenerMixin for Handle<AnimationMean> {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self, app: &mut App) {
        compound_did_start_listening::<AnimationMean, f64>(self, app)
    }

    fn did_stop_listening(self, app: &mut App) {
        compound_did_stop_listening::<AnimationMean, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for Handle<AnimationMean> {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for Handle<AnimationMean> {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for Handle<AnimationMean> {
    fn animation(self) -> Animation<f64> {
        Animation::from_handle(self)
    }

    fn first(self, app: &App) -> Animation<f64> {
        app.get(self).first
    }

    fn next(self, app: &App) -> Animation<f64> {
        app.get(self).next
    }

    fn last_status(self, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMean extends CompoundAnimation<double>`.
impl AnimationNode<f64> for AnimationMean {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(this, app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(this, app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        CompoundAnimation::status(this, app)
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        let animation = app.get(this);
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
    first: Animation<f64>,
    next: Animation<f64>,
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
    pub fn new(first: Animation<f64>, next: Animation<f64>) -> AnimationMax {
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

impl AnimationLazyListenerMixin for Handle<AnimationMax> {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self, app: &mut App) {
        compound_did_start_listening::<AnimationMax, f64>(self, app)
    }

    fn did_stop_listening(self, app: &mut App) {
        compound_did_stop_listening::<AnimationMax, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for Handle<AnimationMax> {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for Handle<AnimationMax> {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for Handle<AnimationMax> {
    fn animation(self) -> Animation<f64> {
        Animation::from_handle(self)
    }

    fn first(self, app: &App) -> Animation<f64> {
        app.get(self).first
    }

    fn next(self, app: &App) -> Animation<f64> {
        app.get(self).next
    }

    fn last_status(self, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMax<T extends num> extends CompoundAnimation<T>`.
impl AnimationNode<f64> for AnimationMax {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(this, app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(this, app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        CompoundAnimation::status(this, app)
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        let animation = app.get(this);
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
    first: Animation<f64>,
    next: Animation<f64>,
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
    pub fn new(first: Animation<f64>, next: Animation<f64>) -> AnimationMin {
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

impl AnimationLazyListenerMixin for Handle<AnimationMin> {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self, app: &mut App) {
        compound_did_start_listening::<AnimationMin, f64>(self, app)
    }

    fn did_stop_listening(self, app: &mut App) {
        compound_did_stop_listening::<AnimationMin, f64>(self, app)
    }
}

impl AnimationLocalListenersMixin for Handle<AnimationMin> {
    fn local_listeners_data(self, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(self, app: &mut App) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl AnimationLocalStatusListenersMixin for Handle<AnimationMin> {
    fn local_status_listeners_data(self, app: &App) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl CompoundAnimation<f64> for Handle<AnimationMin> {
    fn animation(self) -> Animation<f64> {
        Animation::from_handle(self)
    }

    fn first(self, app: &App) -> Animation<f64> {
        app.get(self).first
    }

    fn next(self, app: &App) -> Animation<f64> {
        app.get(self).next
    }

    fn last_status(self, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self, app: &App) -> &Option<f64> {
        &app.get(self).last_value
    }

    fn last_value_mut(self, app: &mut App) -> &mut Option<f64> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class AnimationMin<T extends num> extends CompoundAnimation<T>`.
impl AnimationNode<f64> for AnimationMin {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(this, app, listener)
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(this, app, listener)
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(this, app, listener)
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(this, app, listener)
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        CompoundAnimation::status(this, app)
    }

    fn value(app: &App, this: Handle<Self>) -> f64 {
        let animation = app.get(this);
        dart_min(animation.first.value(app), animation.next.value(app))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::curves::Curves;

    #[test]
    fn the_constant_animations_report_flutters_values() {
        let mut app = App::new();

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
        let mut app = App::new();
        let first = k_always_complete_animation(&mut app);

        // Dart's `const` canonicalization, reproduced per App by the
        // singleton: comparisons like ProxyAnimation's early return answer
        // "unchanged", as Flutter's do.
        assert_eq!(first, k_always_complete_animation(&mut app));
        assert_ne!(first, k_always_dismissed_animation(&mut app));
    }

    #[test]
    fn an_always_stopped_animation_is_forward_at_its_value() {
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));

        assert_eq!(half.value(&app), 0.5);
        assert_eq!(half.status(&app), AnimationStatus::Forward);
    }

    // animations_test.dart:51 — 'ProxyAnimation.toString control test', the
    // value/status half. `toString` is not ported.
    #[test]
    fn a_parentless_proxy_is_dismissed_at_zero() {
        let mut app = App::new();
        let animation = Animation::from_handle(ProxyAnimation::new(&mut app, None).0);

        assert_eq!(animation.value(&app), 0.0);
        assert_eq!(animation.status(&app), AnimationStatus::Dismissed);
    }

    // animations_test.dart:60 — 'ProxyAnimation set parent generates value
    // changed'. The Dart test drives the second notification through an
    // AnimationController, which is not ported; here a chained inner proxy
    // takes that role, which also exercises the tear-off subscription.
    #[test]
    fn set_parent_generates_value_changed() {
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let did_receive_callback = Rc::new(Cell::new(false));

        let animation = ProxyAnimation::new(&mut app, None);
        Animation::from_handle(animation.0).add_listener(
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
        let mut app = App::new();
        let inner = ProxyAnimation::new(&mut app, None);
        let outer = ProxyAnimation::new(&mut app, Some(Animation::from_handle(inner.0)));

        let notified = Rc::new(Cell::new(0));
        Animation::from_handle(outer.0).add_listener(
            &mut app,
            Listener::new({
                let notified = Rc::clone(&notified);
                move |_app| notified.set(notified.get() + 1)
            }),
        );

        // The inner proxy's own notification reaches the outer proxy's
        // listener through the tear-off the outer registered on it.
        let target = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.6)));
        inner.set_parent(&mut app, Some(target));

        assert_eq!(notified.get(), 1);
        assert_eq!(Animation::from_handle(outer.0).value(&app), 0.6);
    }

    #[test]
    fn removing_the_last_listener_unsubscribes_from_the_parent() {
        let mut app = App::new();
        let inner = ProxyAnimation::new(&mut app, None);
        let outer = ProxyAnimation::new(&mut app, Some(Animation::from_handle(inner.0)));

        let listener = Listener::new(|_app| {});
        Animation::from_handle(outer.0).add_listener(&mut app, listener.clone());
        assert!(
            !inner.local_listeners_data(&app).is_empty(),
            "the outer proxy's tear-off is registered on the inner"
        );
        assert!(!inner.local_status_listeners_data(&app).is_empty());

        Animation::from_handle(outer.0).remove_listener(&mut app, &listener);
        assert!(
            inner.local_listeners_data(&app).is_empty(),
            "the rebuilt tear-off matched, so the registration is gone"
        );
        assert!(inner.local_status_listeners_data(&app).is_empty());
    }

    #[test]
    fn set_parent_notifies_the_status_change() {
        let mut app = App::new();
        let statuses = Rc::new(RefCell::new(Vec::new()));

        let animation = ProxyAnimation::new(&mut app, None); // Dismissed
        Animation::from_handle(animation.0).add_status_listener(
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
        let mut app = App::new();
        let animation = ProxyAnimation::new(&mut app, None);

        let observed = Rc::new(Cell::new(f64::NAN));
        Animation::from_handle(animation.0).add_listener(
            &mut app,
            Listener::new({
                let observed = Rc::clone(&observed);
                move |app: &mut App| {
                    // Mid-notification, from inside set_parent: the parent is
                    // already assigned, so the value reads through it — as in
                    // Dart, where the setter notifies after `_parent = value`.
                    observed.set(Animation::from_handle(animation.0).value(app));
                }
            }),
        );

        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        animation.set_parent(&mut app, Some(half));
        assert_eq!(observed.get(), 0.5);
    }

    #[test]
    fn setting_the_same_parent_is_a_no_op() {
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let animation = ProxyAnimation::new(&mut app, Some(half));

        let notified = Rc::new(Cell::new(0));
        Animation::from_handle(animation.0).add_listener(
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
        let mut app = App::new();
        let stopped = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.25)));
        let reverse = Animation::from_handle(app.create(ReverseAnimation::new(stopped)));

        assert_eq!(reverse.value(&app), 0.75);
        assert_eq!(reverse.status(&app), AnimationStatus::Reverse);
    }

    // animations_test.dart:77 — 'ReverseAnimation calls listeners': the value
    // listener is forwarded to the parent, and removal stops the calls. A
    // ProxyAnimation drives in place of the unported controller.
    #[test]
    fn reverse_animation_calls_listeners() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None);
        let animation = app.create(ReverseAnimation::new(Animation::from_handle(driver.0)));

        let did_receive_callback = Rc::new(Cell::new(false));
        let listener = Listener::new({
            let did_receive_callback = Rc::clone(&did_receive_callback);
            move |_app| did_receive_callback.set(true)
        });
        Animation::from_handle(animation).add_listener(&mut app, listener.clone());

        assert!(!did_receive_callback.get());
        let target = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.6)));
        driver.set_parent(&mut app, Some(target));
        assert!(did_receive_callback.get());
        did_receive_callback.set(false);

        Animation::from_handle(animation).remove_listener(&mut app, &listener);
        assert!(!did_receive_callback.get());
        let target = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.7)));
        driver.set_parent(&mut app, Some(target));
        assert!(!did_receive_callback.get());
    }

    // The `_statusChangeHandler` half: status notifications arrive reversed.
    #[test]
    fn a_reverse_animation_reverses_status_notifications() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None); // Dismissed
        let reverse = app.create(ReverseAnimation::new(Animation::from_handle(driver.0)));

        let statuses = Rc::new(RefCell::new(Vec::new()));
        let listener = AnimationStatusListener::new({
            let statuses = Rc::clone(&statuses);
            move |status, _app| statuses.borrow_mut().push(status)
        });
        Animation::from_handle(reverse).add_status_listener(&mut app, listener.clone());
        assert!(
            !driver.local_status_listeners_data(&app).is_empty(),
            "lazily subscribed to the driver"
        );

        let complete = k_always_complete_animation(&mut app);
        driver.set_parent(&mut app, Some(complete));

        assert_eq!(*statuses.borrow(), vec![AnimationStatus::Dismissed]);
        assert_eq!(
            Animation::from_handle(reverse).status(&app),
            AnimationStatus::Dismissed
        );

        Animation::from_handle(reverse).remove_status_listener(&mut app, &listener);
        assert!(
            driver.local_status_listeners_data(&app).is_empty(),
            "the rebuilt tear-off matched, so the subscription is gone"
        );
    }

    #[test]
    fn a_curved_animation_applies_the_curve() {
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let curved = CurvedAnimation::create(&mut app, half, Curves::ease(), None);

        assert_eq!(
            Animation::from_handle(curved).value(&app),
            Curves::ease().transform(0.5)
        );
        assert_eq!(
            Animation::from_handle(curved).status(&app),
            AnimationStatus::Forward
        );
    }

    #[test]
    fn a_curved_animation_uses_the_reverse_curve_when_the_direction_is_reverse() {
        let mut app = App::new();
        // A parent with status Reverse: a ReverseAnimation over an
        // always-Forward stopped animation at 0.25, so its value is 0.75.
        let stopped = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.25)));
        let parent = Animation::from_handle(app.create(ReverseAnimation::new(stopped)));
        let curved =
            CurvedAnimation::create(&mut app, parent, Curves::ease(), Some(Curves::ease_out()));

        // The constructor saw status Reverse while animating, so the curve
        // direction is pinned to Reverse and the reverse curve is active.
        assert_eq!(
            Animation::from_handle(curved).value(&app),
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

        let mut app = App::new();
        let zero = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.0)));
        let curved = CurvedAnimation::create(&mut app, zero, Rc::new(BogusCurve), None);
        let _ = Animation::from_handle(curved).value(&app);
    }

    // animations_test.dart:326 — 'CurvedAnimation stops listening to parent
    // when disposed', the subscription half.
    #[test]
    fn a_disposed_curved_animation_unsubscribes_from_its_parent() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None);
        let curved = CurvedAnimation::create(
            &mut app,
            Animation::from_handle(driver.0),
            Curves::ease(),
            None,
        );
        assert!(!driver.local_status_listeners_data(&app).is_empty());

        CurvedAnimation::dispose(&mut app, curved);
        assert!(driver.local_status_listeners_data(&app).is_empty());
        assert!(app.get(curved).is_disposed);
    }

    // animations_test.dart:97 — 'TrainHoppingAnimation', with ProxyAnimations
    // as the trains in place of the unported controllers.
    #[test]
    fn a_train_hopping_animation_hops_when_the_next_train_crosses() {
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let low = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.3)));
        let train_a = ProxyAnimation::new(&mut app, Some(half));
        let train_b = ProxyAnimation::new(&mut app, Some(low));

        let switched = Rc::new(Cell::new(false));
        let train = TrainHoppingAnimation::create(
            &mut app,
            Animation::from_handle(train_a.0),
            Some(Animation::from_handle(train_b.0)),
            Some(Listener::new({
                let switched = Rc::clone(&switched);
                move |_app| switched.set(true)
            })),
        );

        assert_eq!(Animation::from_handle(train).value(&app), 0.5);
        assert!(!switched.get());

        // Drive the next train past the current one: 0.3 → 0.75.
        let high = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.75)));
        train_b.set_parent(&mut app, Some(high));

        assert!(switched.get(), "the hop fired the callback");
        assert_eq!(
            app.get(train).current_train(),
            Some(Animation::from_handle(train_b.0))
        );
        assert_eq!(Animation::from_handle(train).value(&app), 0.75);
    }

    #[test]
    fn trains_starting_at_the_same_value_hop_immediately_without_the_callback() {
        let mut app = App::new();
        let a = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let b = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));

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
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let low = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.3)));
        let train_a = ProxyAnimation::new(&mut app, Some(half));
        let train_b = ProxyAnimation::new(&mut app, Some(low));

        let train = TrainHoppingAnimation::create(
            &mut app,
            Animation::from_handle(train_a.0),
            Some(Animation::from_handle(train_b.0)),
            None,
        );
        assert!(!train_a.local_listeners_data(&app).is_empty());
        assert!(!train_a.local_status_listeners_data(&app).is_empty());
        assert!(!train_b.local_listeners_data(&app).is_empty());

        TrainHoppingAnimation::dispose(&mut app, train);
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
        let mut app = App::new();
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let left = ProxyAnimation::new(&mut app, Some(half));
        let right = ProxyAnimation::new(&mut app, None); // 0.0

        let mean = app.create(AnimationMean::new(
            Animation::from_handle(left.0),
            Animation::from_handle(right.0),
        ));
        let mean_handle = Animation::from_handle(mean);
        assert_eq!(mean_handle.value(&app), 0.25);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_value = Listener::new({
            let log = Rc::clone(&log);
            move |app: &mut App| {
                let value = Animation::from_handle(mean).value(app);
                log.borrow_mut().push(value);
            }
        });
        mean_handle.add_listener(&mut app, log_value.clone());

        let one = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(1.0)));
        right.set_parent(&mut app, Some(one));

        assert_eq!(mean_handle.value(&app), 0.75);
        assert_eq!(*log.borrow(), vec![0.75]);
        log.borrow_mut().clear();

        mean_handle.remove_listener(&mut app, &log_value);

        let zero = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.0)));
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

        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None); // Dismissed, 0.0
        let falling = Tween::new(&mut app, Some(1.0), Some(-1.0));
        let rising = Tween::new(&mut app, Some(-1.0), Some(1.0));
        let current = falling.animate(&mut app, Animation::from_handle(driver.0));
        let next = rising.animate(&mut app, Animation::from_handle(driver.0));

        let animation = TrainHoppingAnimation::create(&mut app, current, Some(next), None);

        let status_log = Rc::new(RefCell::new(Vec::new()));
        Animation::from_handle(animation).add_status_listener(
            &mut app,
            AnimationStatusListener::new({
                let status_log = Rc::clone(&status_log);
                move |status, _app| status_log.borrow_mut().push(status)
            }),
        );
        assert!(status_log.borrow().is_empty());

        // Dart: `controller.forward()` — status Forward; the trains cross at
        // t = 0.5, which hops to the rising train.
        let half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
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

        let mut app = App::new();
        // Forward at 0.5, and Reverse at 0.5 (a ReverseAnimation over 0.5
        // keeps the value while flipping the direction).
        let forward_half = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
        let reverse_half = {
            let stopped = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.5)));
            Animation::from_handle(app.create(ReverseAnimation::new(stopped)))
        };

        let parent = ProxyAnimation::new(&mut app, Some(forward_half));
        let curved = CurvedAnimation::create(
            &mut app,
            Animation::from_handle(parent.0),
            forward_curve,
            Some(reverse_curve),
        );
        let curved_handle = Animation::from_handle(curved);

        // Forward at 0.5: the forward interval maps it to 1.0.
        assert_eq!(curved_handle.value(&app), 1.0);

        // Reach Completed so the curve direction resets, as the Dart test
        // does with `controller.value = 1.0`, then go Reverse.
        let complete = k_always_complete_animation(&mut app);
        parent.set_parent(&mut app, Some(complete));
        parent.set_parent(&mut app, Some(reverse_half));
        assert_eq!(curved_handle.value(&app), 0.0);

        assert!(!app.get(curved).is_disposed);
        CurvedAnimation::dispose(&mut app, curved);
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
        let mut app = App::new();
        let low = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.25)));
        let high = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(0.75)));

        let max = Animation::from_handle(app.create(AnimationMax::new(low, high)));
        let min = Animation::from_handle(app.create(AnimationMin::new(low, high)));

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

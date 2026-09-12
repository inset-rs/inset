//! Flutter counterpart: `widgets/scroll_physics.dart`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::clamp_double;
use inset_foundation::App;
use inset_gestures::{K_MAX_FLING_VELOCITY, K_MIN_FLING_VELOCITY, K_TOUCH_SLOP};
use inset_painting::AxisDirection;
use inset_physics::{ScrollSpringSimulation, Simulation, SpringDescription, Tolerance};

use crate::framework::BuildContext;
use crate::view::View;
use crate::widgets::scroll_metrics::{FixedScrollMetrics, ScrollMetrics};
use crate::widgets::scroll_simulation::{BouncingScrollSimulation, ClampingScrollSimulation};

/// A [`ScrollPhysics`] held by value: what a Dart field or parameter typed `ScrollPhysics`
/// becomes, and what [`ScrollPhysics::apply_to`] returns.
pub type ScrollPhysicsRef = Rc<dyn ScrollPhysics>;

/// The rate at which scroll momentum will be decelerated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollDecelerationRate {
    /// Standard deceleration, aligned with mobile software expectations.
    Normal,

    /// Increased deceleration, aligned with desktop software expectations.
    ///
    /// Appropriate for use with input devices more precise than touch screens,
    /// such as trackpads or mouse wheels.
    Fast,
}

/// Determines the physics of a `Scrollable` widget.
///
/// For example, determines how the `Scrollable` will behave when the user
/// reaches the maximum scroll extent or when the user stops scrolling.
///
/// When starting a physics [`Simulation`], the current scroll position and
/// velocity are used as the initial conditions for the particle in the
/// simulation. The movement of the particle in the simulation is then used to
/// determine the scroll position for the widget.
///
/// Instead of writing your own implementors, [`parent`](Self::parent) can be used to
/// combine [`ScrollPhysics`] objects of different types to get the desired scroll physics.
/// For example:
///
/// ```text
/// BouncingScrollPhysics::new().with_parent(Rc::new(AlwaysScrollableScrollPhysics::new()))
/// ```
///
/// You can also use [`apply_to`](Self::apply_to), which is useful when you already have an
/// instance of [`ScrollPhysics`]:
///
/// ```text
/// let physics = BouncingScrollPhysics::new();
/// let merged = physics.apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
/// ```
///
/// When writing an implementor, you must override [`apply_to`](Self::apply_to) so that it
/// returns an appropriate instance of your own type. Otherwise, types like `Scrollable`
/// that inform a `ScrollPosition` will combine them with the default [`ScrollPhysics`]
/// object instead of yours.
///
/// The base-class bodies Dart reaches with `super` live on [`ScrollPhysicsBase`], which
/// every implementor gets for free.
pub trait ScrollPhysics: Debug {
    /// The value behind the erased physics, for Dart's `physics.runtimeType` — which
    /// `ScrollableState._shouldUpdatePosition` compares along the whole parent chain.
    fn as_any(&self) -> &dyn Any;

    /// If non-null, determines the default behavior for each method.
    ///
    /// If an implementor of [`ScrollPhysics`] does not override a method, that implementor
    /// will inherit an implementation from [`ScrollPhysicsBase`] that defers to
    /// [`parent`](Self::parent). This mechanism lets you assemble novel combinations of
    /// [`ScrollPhysics`] implementors at runtime. For example:
    ///
    /// ```text
    /// BouncingScrollPhysics::new().with_parent(Rc::new(AlwaysScrollableScrollPhysics::new()))
    /// ```
    ///
    /// will result in a [`ScrollPhysics`] that has the combined behavior
    /// of [`BouncingScrollPhysics`] and [`AlwaysScrollableScrollPhysics`]:
    /// behaviors that are not specified in [`BouncingScrollPhysics`]
    /// (e.g. [`should_accept_user_offset`](Self::should_accept_user_offset)) will defer to
    /// [`AlwaysScrollableScrollPhysics`].
    fn parent(&self) -> Option<&ScrollPhysicsRef>;

    /// Combines this [`ScrollPhysics`] instance with the given physics.
    ///
    /// The returned object uses this instance's physics when it has an
    /// opinion, and defers to the given `ancestor` object's physics
    /// when it does not.
    ///
    /// If [`parent`](Self::parent) is null then this returns a [`ScrollPhysics`] of the
    /// same type, but where the parent has been replaced with the `ancestor`.
    ///
    /// If this scroll physics object already has a parent, then this
    /// method is applied recursively and ancestor will appear at the
    /// end of the existing chain of parents.
    ///
    /// Calling this method with `None` will copy the current object. This is inefficient.
    ///
    /// ## Implementing [`apply_to`](Self::apply_to)
    ///
    /// When writing a custom [`ScrollPhysics`] implementor, this method must be
    /// implemented. If the physics type has no constructor arguments, then implementing
    /// this method is merely a matter of calling the constructor with a parent constructed
    /// using [`build_parent`](ScrollPhysicsBase::build_parent). If it has constructor
    /// arguments, they must be passed here as well, so as to create a clone.
    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        ScrollPhysicsBase::apply_to(self, ancestor)
    }

    /// Used by `DragScrollActivity` and other user-driven activities to convert
    /// an offset in logical pixels as provided by the `DragUpdateDetails` into a
    /// delta to apply (subtract from the current position) using
    /// `ScrollActivityDelegate.setPixels`.
    ///
    /// This is used by some `ScrollPosition` implementors to apply friction during
    /// overscroll situations.
    ///
    /// This method must not adjust parts of the offset that are entirely within
    /// the bounds described by the given `position`.
    ///
    /// The given `position` is only valid during this method call. Do not keep a
    /// reference to it to use later, as the values may update, may not update, or
    /// may update to reflect an entirely unrelated scrollable.
    fn apply_physics_to_user_offset(&self, position: &dyn ScrollMetrics, offset: f64) -> f64 {
        ScrollPhysicsBase::apply_physics_to_user_offset(self, position, offset)
    }

    /// Whether the scrollable should let the user adjust the scroll offset, for
    /// example by dragging. If [`allow_user_scrolling`](Self::allow_user_scrolling) is
    /// false, the scrollable will never allow user input to change the scroll position.
    ///
    /// By default, the user can manipulate the scroll offset if, and only if,
    /// there is actually content outside the viewport to reveal.
    ///
    /// The given `position` is only valid during this method call. Do not keep a
    /// reference to it to use later, as the values may update, may not update, or
    /// may update to reflect an entirely unrelated scrollable.
    fn should_accept_user_offset(&self, position: &dyn ScrollMetrics) -> bool {
        ScrollPhysicsBase::should_accept_user_offset(self, position)
    }

    /// Provides a heuristic to determine if expensive frame-bound tasks should be
    /// deferred.
    ///
    /// The `velocity` parameter may be positive, negative, or zero.
    ///
    /// The `context` parameter normally refers to the [`BuildContext`] of the widget
    /// making the call, such as an `Image` widget in a `ListView`.
    ///
    /// This can be used to determine whether decoding or fetching complex data
    /// for the currently visible part of the viewport should be delayed
    /// to avoid doing work that will not have a chance to appear before a new
    /// frame is rendered.
    ///
    /// The default implementation is a heuristic that compares the current
    /// scroll velocity in local logical pixels to the longest side of the window
    /// in physical pixels. Implementors can change this heuristic by overriding
    /// this method and providing their custom physics to the scrollable widget.
    ///
    /// The default implementation is stateless, and provides a point-in-time
    /// decision about how fast the scrollable is scrolling. It would always
    /// return true for a scrollable that is animating back and forth at high
    /// velocity in a loop. It is assumed that callers will handle such
    /// a case, or that a custom stateful implementation would be written that
    /// tracks the sign of the velocity on successive calls.
    ///
    /// Returning true from this method indicates that the current scroll velocity
    /// is great enough that expensive operations impacting the UI should be
    /// deferred.
    fn recommend_deferred_loading(
        &self,
        app: &mut App,
        velocity: f64,
        metrics: &dyn ScrollMetrics,
        context: BuildContext,
    ) -> bool {
        ScrollPhysicsBase::recommend_deferred_loading(self, app, velocity, metrics, context)
    }

    /// Determines the overscroll by applying the boundary conditions.
    ///
    /// Called by `ScrollPosition.applyBoundaryConditions`, which is called by
    /// `ScrollPosition.setPixels` just before the `ScrollPosition.pixels` value
    /// is updated, to determine how much of the offset is to be clamped off and
    /// sent to `ScrollPosition.didOverscrollBy`.
    ///
    /// The `value` argument is guaranteed to not equal the
    /// [`ScrollMetrics::pixels`] of the `position` argument when this is called.
    ///
    /// It is possible for this method to be called when the `position` describes
    /// an already-out-of-bounds position. In that case, the boundary conditions
    /// should usually only prevent a further increase in the extent to which the
    /// position is out of bounds, allowing a decrease to be applied successfully,
    /// so that (for instance) an animation can smoothly snap an out of bounds
    /// position to the bounds.
    ///
    /// This method must not clamp parts of the offset that are entirely within
    /// the bounds described by the given `position`.
    ///
    /// The given `position` is only valid during this method call. Do not keep a
    /// reference to it to use later, as the values may update, may not update, or
    /// may update to reflect an entirely unrelated scrollable.
    ///
    /// ## Examples
    ///
    /// [`BouncingScrollPhysics`] returns zero. In other words, it allows scrolling
    /// past the boundary unhindered.
    ///
    /// [`ClampingScrollPhysics`] returns the amount by which the value is beyond
    /// the position or the boundary, whichever is furthest from the content. In
    /// other words, it disallows scrolling past the boundary, but allows
    /// scrolling back from being overscrolled, if for some reason the position
    /// ends up overscrolled.
    fn apply_boundary_conditions(&self, position: &dyn ScrollMetrics, value: f64) -> f64 {
        ScrollPhysicsBase::apply_boundary_conditions(self, position, value)
    }

    /// Describes what the scroll position should be given new viewport dimensions.
    ///
    /// This is called by `ScrollPosition.correctForNewDimensions`.
    ///
    /// The arguments consist of the scroll metrics as they stood in the previous
    /// frame and the scroll metrics as they now stand after the last layout,
    /// including the position and minimum and maximum scroll extents; a flag
    /// indicating if the current `ScrollActivity` considers that the user is
    /// actively scrolling; and the current velocity of the scroll position, if it is being
    /// driven by the scroll activity (this is 0.0 during a user gesture).
    ///
    /// The scroll metrics will be identical except for the
    /// [`ScrollMetrics::min_scroll_extent`] and [`ScrollMetrics::max_scroll_extent`].
    /// They are referred to as the `old_position` and `new_position` (even though they
    /// both technically have the same "position", in the form of [`ScrollMetrics::pixels`])
    /// because they are generated from the `ScrollPosition` before and after updating the
    /// scroll extents.
    ///
    /// If the returned value does not exactly match the scroll offset given by
    /// the `new_position` argument, then the `ScrollPosition` will call
    /// `ScrollPosition.correctPixels` to update the new scroll position to the returned
    /// value, and layout will be re-run. This is expensive. The new value is subject to
    /// further manipulation by [`apply_boundary_conditions`](Self::apply_boundary_conditions).
    ///
    /// If the returned value _does_ match the `new_position.pixels()` scroll offset
    /// exactly, then `ScrollPosition.applyNewDimensions` will be called next. In
    /// that case, [`apply_boundary_conditions`](Self::apply_boundary_conditions) is not
    /// applied to the return value.
    ///
    /// The given [`ScrollMetrics`] are only valid during this method call. Do not
    /// keep references to them to use later, as the values may update, may not
    /// update, or may update to reflect an entirely unrelated scrollable.
    ///
    /// The default implementation returns the [`ScrollMetrics::pixels`] of the
    /// `new_position`, which indicates that the current scroll offset is acceptable.
    ///
    /// See also:
    ///
    ///  * [`RangeMaintainingScrollPhysics`], which is enabled by default, and
    ///    which prevents unexpected changes to the content dimensions from
    ///    causing the scroll position to get any further out of bounds.
    fn adjust_position_for_new_dimensions(
        &self,
        old_position: &dyn ScrollMetrics,
        new_position: &dyn ScrollMetrics,
        is_scrolling: bool,
        velocity: f64,
    ) -> f64 {
        ScrollPhysicsBase::adjust_position_for_new_dimensions(
            self,
            old_position,
            new_position,
            is_scrolling,
            velocity,
        )
    }

    /// Returns a simulation for ballistic scrolling starting from the given
    /// position with the given velocity.
    ///
    /// This is used by `ScrollPositionWithSingleContext` in its `goBallistic` method. If
    /// the result is non-null, `ScrollPositionWithSingleContext` will begin a
    /// `BallisticScrollActivity` with the returned value. Otherwise, it will
    /// begin an idle activity instead.
    ///
    /// The given `position` is only valid during this method call. Do not keep a
    /// reference to it to use later, as the values may update, may not update, or
    /// may update to reflect an entirely unrelated scrollable.
    ///
    /// This method can potentially be called in every frame, even in the middle
    /// of what the user perceives as a single ballistic scroll. For example, in
    /// a `ListView` when previously off-screen items come into view and are laid
    /// out, this method may be called with a new [`ScrollMetrics::max_scroll_extent`].
    /// The implementation should ensure that when the same ballistic scroll motion is still
    /// intended, these calls have no side effects on the physics beyond continuing that
    /// motion.
    ///
    /// Generally this is ensured by having the [`Simulation`] conform to a physical
    /// metaphor of a particle in ballistic flight, where the forces on the
    /// particle depend only on its position, velocity, and environment, and not
    /// on the current time or any internal state. This means that the
    /// time-derivative of [`Simulation::dx`] should be possible to write
    /// mathematically as a function purely of the values of [`Simulation::x`],
    /// [`Simulation::dx`], and the parameters used to construct the [`Simulation`],
    /// independent of the time.
    fn create_ballistic_simulation(
        &self,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        ScrollPhysicsBase::create_ballistic_simulation(self, position, velocity)
    }

    /// The spring to use for ballistic simulations.
    fn spring(&self) -> SpringDescription {
        ScrollPhysicsBase::spring(self)
    }

    /// Deprecated. Call [`tolerance_for`](Self::tolerance_for) instead.
    #[deprecated(
        note = "Call tolerance_for instead. This feature was deprecated after v3.7.0-13.0.pre."
    )]
    fn tolerance(&self, app: &App) -> Tolerance {
        let device_pixel_ratio = app
            .platform()
            .implicit_view()
            .expect("ScrollPhysics.tolerance reads the implicit view's device pixel ratio")
            .metrics()
            .device_pixel_ratio;
        self.tolerance_for(&FixedScrollMetrics::new(
            None,
            None,
            None,
            None,
            AxisDirection::Down,
            device_pixel_ratio,
        ))
    }

    /// The tolerance to use for ballistic simulations.
    fn tolerance_for(&self, metrics: &dyn ScrollMetrics) -> Tolerance {
        ScrollPhysicsBase::tolerance_for(self, metrics)
    }

    /// The minimum distance an input pointer drag must have moved to be
    /// considered a scroll fling gesture.
    ///
    /// This value is typically compared with the distance traveled along the
    /// scrolling axis.
    fn min_fling_distance(&self) -> f64 {
        ScrollPhysicsBase::min_fling_distance(self)
    }

    /// The minimum velocity for an input pointer drag to be considered a
    /// scroll fling.
    ///
    /// This value is typically compared with the magnitude of fling gesture's
    /// velocity along the scrolling axis.
    fn min_fling_velocity(&self) -> f64 {
        ScrollPhysicsBase::min_fling_velocity(self)
    }

    /// Scroll fling velocity magnitudes will be clamped to this value.
    fn max_fling_velocity(&self) -> f64 {
        ScrollPhysicsBase::max_fling_velocity(self)
    }

    /// Returns the velocity carried on repeated flings.
    ///
    /// The function is applied to the existing scroll velocity when another
    /// scroll drag is applied in the same direction.
    ///
    /// By default, physics for platforms other than iOS doesn't carry momentum.
    fn carried_momentum(&self, existing_velocity: f64) -> f64 {
        ScrollPhysicsBase::carried_momentum(self, existing_velocity)
    }

    /// The minimum amount of pixel distance drags must move by to start motion
    /// the first time or after each time the drag motion stopped.
    ///
    /// If null, no minimum threshold is enforced.
    fn drag_start_distance_motion_threshold(&self) -> Option<f64> {
        ScrollPhysicsBase::drag_start_distance_motion_threshold(self)
    }

    /// Whether a viewport is allowed to change its scroll position implicitly in
    /// response to a call to `RenderObject.showOnScreen`.
    ///
    /// `RenderObject.showOnScreen` is for example used to bring a text field
    /// fully on screen after it has received focus. This property controls
    /// whether the viewport associated with this object is allowed to change the
    /// scroll position to fulfill such a request.
    fn allow_implicit_scrolling(&self) -> bool {
        true
    }

    /// Whether a viewport is allowed to change the scroll position as the result of user
    /// input.
    fn allow_user_scrolling(&self) -> bool {
        true
    }
}

/// The bodies of Dart's `ScrollPhysics` base class: what `super.foo(..)` calls.
///
/// A trait default cannot be reached from an implementor that overrides it, so these are
/// associated functions on a namespace rather than a second trait: a second trait carrying
/// Dart's method names would make every `physics.foo()` call ambiguous.
pub struct ScrollPhysicsBase;

impl ScrollPhysicsBase {
    /// If [`parent`](ScrollPhysics::parent) is null then return `ancestor`, otherwise
    /// recursively build a [`ScrollPhysics`] that has `ancestor` as its parent.
    ///
    /// This function is typically used to define [`apply_to`](ScrollPhysics::apply_to).
    pub fn build_parent<P: ScrollPhysics + ?Sized>(
        physics: &P,
        ancestor: Option<ScrollPhysicsRef>,
    ) -> Option<ScrollPhysicsRef> {
        match physics.parent() {
            Some(parent) => Some(parent.apply_to(ancestor)),
            None => ancestor,
        }
    }

    /// See [`ScrollPhysics::apply_to`].
    pub fn apply_to<P: ScrollPhysics + ?Sized>(
        physics: &P,
        ancestor: Option<ScrollPhysicsRef>,
    ) -> ScrollPhysicsRef {
        Rc::new(PlainScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(physics, ancestor),
        })
    }

    /// See [`ScrollPhysics::apply_physics_to_user_offset`].
    pub fn apply_physics_to_user_offset<P: ScrollPhysics + ?Sized>(
        physics: &P,
        position: &dyn ScrollMetrics,
        offset: f64,
    ) -> f64 {
        match physics.parent() {
            Some(parent) => parent.apply_physics_to_user_offset(position, offset),
            None => offset,
        }
    }

    /// See [`ScrollPhysics::should_accept_user_offset`].
    pub fn should_accept_user_offset<P: ScrollPhysics + ?Sized>(
        physics: &P,
        position: &dyn ScrollMetrics,
    ) -> bool {
        if !physics.allow_user_scrolling() {
            return false;
        }

        match physics.parent() {
            None => {
                position.pixels() != 0.0
                    || position.min_scroll_extent() != position.max_scroll_extent()
            }
            Some(parent) => parent.should_accept_user_offset(position),
        }
    }

    /// See [`ScrollPhysics::recommend_deferred_loading`].
    pub fn recommend_deferred_loading<P: ScrollPhysics + ?Sized>(
        physics: &P,
        app: &mut App,
        velocity: f64,
        metrics: &dyn ScrollMetrics,
        context: BuildContext,
    ) -> bool {
        match physics.parent() {
            None => {
                let physical_size = View::of(app, context).metrics().physical_size;
                let max_physical_pixels = physical_size[0].max(physical_size[1]);
                velocity.abs() > max_physical_pixels
            }
            Some(parent) => {
                let parent = Rc::clone(parent);
                parent.recommend_deferred_loading(app, velocity, metrics, context)
            }
        }
    }

    /// See [`ScrollPhysics::apply_boundary_conditions`].
    pub fn apply_boundary_conditions<P: ScrollPhysics + ?Sized>(
        physics: &P,
        position: &dyn ScrollMetrics,
        value: f64,
    ) -> f64 {
        match physics.parent() {
            Some(parent) => parent.apply_boundary_conditions(position, value),
            None => 0.0,
        }
    }

    /// See [`ScrollPhysics::adjust_position_for_new_dimensions`].
    pub fn adjust_position_for_new_dimensions<P: ScrollPhysics + ?Sized>(
        physics: &P,
        old_position: &dyn ScrollMetrics,
        new_position: &dyn ScrollMetrics,
        is_scrolling: bool,
        velocity: f64,
    ) -> f64 {
        match physics.parent() {
            None => new_position.pixels(),
            Some(parent) => parent.adjust_position_for_new_dimensions(
                old_position,
                new_position,
                is_scrolling,
                velocity,
            ),
        }
    }

    /// See [`ScrollPhysics::create_ballistic_simulation`].
    pub fn create_ballistic_simulation<P: ScrollPhysics + ?Sized>(
        physics: &P,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        physics
            .parent()
            .and_then(|parent| parent.create_ballistic_simulation(position, velocity))
    }

    /// See [`ScrollPhysics::spring`].
    pub fn spring<P: ScrollPhysics + ?Sized>(physics: &P) -> SpringDescription {
        match physics.parent() {
            Some(parent) => parent.spring(),
            None => k_default_spring(),
        }
    }

    /// See [`ScrollPhysics::tolerance_for`].
    pub fn tolerance_for<P: ScrollPhysics + ?Sized>(
        physics: &P,
        metrics: &dyn ScrollMetrics,
    ) -> Tolerance {
        match physics.parent() {
            Some(parent) => parent.tolerance_for(metrics),
            None => Tolerance {
                // logical pixels per second
                velocity: 1.0 / (0.050 * metrics.device_pixel_ratio()),
                // logical pixels
                distance: 1.0 / metrics.device_pixel_ratio(),
                ..Tolerance::DEFAULT_TOLERANCE
            },
        }
    }

    /// See [`ScrollPhysics::min_fling_distance`].
    pub fn min_fling_distance<P: ScrollPhysics + ?Sized>(physics: &P) -> f64 {
        match physics.parent() {
            Some(parent) => parent.min_fling_distance(),
            None => K_TOUCH_SLOP,
        }
    }

    /// See [`ScrollPhysics::min_fling_velocity`].
    pub fn min_fling_velocity<P: ScrollPhysics + ?Sized>(physics: &P) -> f64 {
        match physics.parent() {
            Some(parent) => parent.min_fling_velocity(),
            None => K_MIN_FLING_VELOCITY,
        }
    }

    /// See [`ScrollPhysics::max_fling_velocity`].
    pub fn max_fling_velocity<P: ScrollPhysics + ?Sized>(physics: &P) -> f64 {
        match physics.parent() {
            Some(parent) => parent.max_fling_velocity(),
            None => K_MAX_FLING_VELOCITY,
        }
    }

    /// See [`ScrollPhysics::carried_momentum`].
    pub fn carried_momentum<P: ScrollPhysics + ?Sized>(physics: &P, existing_velocity: f64) -> f64 {
        match physics.parent() {
            Some(parent) => parent.carried_momentum(existing_velocity),
            None => 0.0,
        }
    }

    /// See [`ScrollPhysics::drag_start_distance_motion_threshold`].
    pub fn drag_start_distance_motion_threshold<P: ScrollPhysics + ?Sized>(
        physics: &P,
    ) -> Option<f64> {
        physics
            .parent()
            .and_then(|parent| parent.drag_start_distance_motion_threshold())
    }
}

fn k_default_spring() -> SpringDescription {
    SpringDescription::with_damping_ratio_value(0.5, 100.0, 1.1)
}

/// Dart's `ScrollPhysics.toString`: the runtime type, then the parent chain.
fn fmt_scroll_physics(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    parent: Option<&ScrollPhysicsRef>,
) -> fmt::Result {
    match parent {
        None => write!(f, "{name}"),
        Some(parent) => write!(f, "{name} -> {parent:?}"),
    }
}

/// Dart's concrete `ScrollPhysics` base class: physics that only ever defer to their
/// parent.
///
/// This is what Dart writes as `const ScrollPhysics()`, and what
/// [`ScrollPhysicsBase::apply_to`] returns.
#[derive(Default)]
pub struct PlainScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl PlainScrollPhysics {
    /// Creates an object with the default scroll physics.
    pub fn new() -> PlainScrollPhysics {
        PlainScrollPhysics::default()
    }

    /// Dart `ScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> PlainScrollPhysics {
        self.parent = Some(parent);
        self
    }
}

impl ScrollPhysics for PlainScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }
}

impl Debug for PlainScrollPhysics {
    /// Dart's `objectRuntimeType(this, 'ScrollPhysics')`: this is that class.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "ScrollPhysics", self.parent.as_ref())
    }
}

/// Scroll physics that attempt to keep the scroll position in range when the
/// contents change dimensions suddenly.
///
/// This attempts to maintain the amount of overscroll or underscroll already present,
/// if the scroll position is already out of range _and_ the extents
/// have decreased, meaning that some content was removed. The reason for this
/// condition is that when new content is added, keeping the same overscroll
/// would mean that instead of showing it to the user, all of it is
/// being skipped by jumping right to the max extent.
///
/// If the scroll activity is animating the scroll position, sudden changes to
/// the scroll dimensions are allowed to happen (so as to prevent animations
/// from jumping back and forth between in-range and out-of-range values).
///
/// These physics should be combined with other scroll physics, e.g.
/// [`BouncingScrollPhysics`] or [`ClampingScrollPhysics`], to obtain a complete
/// description of typical scroll physics. See [`ScrollPhysics::apply_to`].
///
/// ## Implementation details
///
/// Specifically, these physics perform two adjustments.
///
/// The first is to maintain overscroll when the position is out of range.
///
/// The second is to enforce the boundary when the position is in range.
///
/// If the current velocity is non-zero, neither adjustment is made. The
/// assumption is that there is an ongoing animation and therefore
/// further changing the scroll position would disrupt the experience.
///
/// If the extents haven't changed, then the overscroll adjustment is
/// not made. The assumption is that if the position is overscrolled,
/// it is intentional, otherwise the position could not have reached
/// that position. (Consider [`ClampingScrollPhysics`] vs
/// [`BouncingScrollPhysics`] for example.)
///
/// If the position itself changed since the last animation frame,
/// then the overscroll is not maintained. The assumption is similar
/// to the previous case: the position would not have been placed out
/// of range unless it was intentional.
///
/// In addition, if the position changed and the boundaries were and
/// still are finite, then the boundary isn't enforced either, for
/// the same reason. However, if any of the boundaries were or are
/// now infinite, the boundary _is_ enforced, on the assumption that
/// infinite boundaries indicate a lazy-loading scroll view, which
/// cannot enforce boundaries while the full list has not loaded.
///
/// If the range was out of range, then the boundary is not enforced
/// even if the range is not maintained. If the range is maintained,
/// then the distance between the old position and the old boundary is
/// applied to the new boundary to obtain the new position.
///
/// If the range was in range, and the boundary is to be enforced,
/// then the new position is obtained by deferring to the other physics,
/// if any, and then clamped to the new range.
#[derive(Default)]
pub struct RangeMaintainingScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl RangeMaintainingScrollPhysics {
    /// Creates scroll physics that maintain the scroll position in range.
    pub fn new() -> RangeMaintainingScrollPhysics {
        RangeMaintainingScrollPhysics::default()
    }

    /// Dart `RangeMaintainingScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> RangeMaintainingScrollPhysics {
        self.parent = Some(parent);
        self
    }
}

impl ScrollPhysics for RangeMaintainingScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(RangeMaintainingScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn adjust_position_for_new_dimensions(
        &self,
        old_position: &dyn ScrollMetrics,
        new_position: &dyn ScrollMetrics,
        is_scrolling: bool,
        velocity: f64,
    ) -> f64 {
        let mut maintain_overscroll = true;
        let mut enforce_boundary = true;
        if velocity != 0.0 {
            // Don't try to adjust an animating position, the jumping around
            // would be distracting.
            maintain_overscroll = false;
            enforce_boundary = false;
        }
        if old_position.min_scroll_extent() == new_position.min_scroll_extent()
            && old_position.max_scroll_extent() == new_position.max_scroll_extent()
        {
            // If the extents haven't changed then ignore overscroll.
            maintain_overscroll = false;
        }
        if old_position.pixels() != new_position.pixels() {
            // If the position has been changed already, then it might have
            // been adjusted to expect new overscroll, so don't try to
            // maintain the relative overscroll.
            maintain_overscroll = false;
            if old_position.min_scroll_extent().is_finite()
                && old_position.max_scroll_extent().is_finite()
                && new_position.min_scroll_extent().is_finite()
                && new_position.max_scroll_extent().is_finite()
            {
                // In addition, if the position changed then we don't enforce the new
                // boundary if both the new and previous boundaries are entirely finite.
                // A common case where the position changes while one
                // of the extents is infinite is a lazily-loaded list. (If the
                // boundaries were finite, and the position changed, then we
                // assume it was intentional.)
                enforce_boundary = false;
            }
        }
        if old_position.pixels() < old_position.min_scroll_extent()
            || old_position.pixels() > old_position.max_scroll_extent()
        {
            // If the old position was out of range, then we should
            // not try to keep the new position in range.
            enforce_boundary = false;
        }
        if maintain_overscroll {
            // Force the new position to be no more out of range than it was before, if:
            //  * it was overscrolled, and
            //  * the extents have decreased, meaning that some content was removed. The
            //    reason for this condition is that when new content is added, keeping
            //    the same overscroll would mean that instead of showing it to the user,
            //    all of it is being skipped by jumping right to the max extent.
            if old_position.pixels() < old_position.min_scroll_extent()
                && new_position.min_scroll_extent() > old_position.min_scroll_extent()
            {
                let old_delta = old_position.min_scroll_extent() - old_position.pixels();
                return new_position.min_scroll_extent() - old_delta;
            }
            if old_position.pixels() > old_position.max_scroll_extent()
                && new_position.max_scroll_extent() < old_position.max_scroll_extent()
            {
                let old_delta = old_position.pixels() - old_position.max_scroll_extent();
                return new_position.max_scroll_extent() + old_delta;
            }
        }
        // If we're not forcing the overscroll, defer to other physics.
        let mut result = ScrollPhysicsBase::adjust_position_for_new_dimensions(
            self,
            old_position,
            new_position,
            is_scrolling,
            velocity,
        );
        if enforce_boundary {
            // ...but if they put us out of range then reinforce the boundary.
            result = clamp_double(
                result,
                new_position.min_scroll_extent(),
                new_position.max_scroll_extent(),
            );
        }
        result
    }
}

impl Debug for RangeMaintainingScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "RangeMaintainingScrollPhysics", self.parent.as_ref())
    }
}

/// Scroll physics for environments that allow the scroll offset to go beyond
/// the bounds of the content, but then bounce the content back to the edge of
/// those bounds.
///
/// This is the behavior typically seen on iOS.
///
/// [`BouncingScrollPhysics`] by itself will not create an overscroll effect if
/// the contents of the scroll view do not extend beyond the size of the
/// viewport. To create the overscroll and bounce effect regardless of the
/// length of your scroll view, combine with [`AlwaysScrollableScrollPhysics`].
///
/// See also:
///
///  * `ScrollConfiguration`, which uses this to provide the default
///    scroll behavior on iOS.
///  * [`ClampingScrollPhysics`], which is the analogous physics for Android's
///    clamping behavior.
///  * [`ScrollPhysics`], for more examples of combining [`ScrollPhysics`] objects
///    of different types to get the desired scroll physics.
pub struct BouncingScrollPhysics {
    /// Used to determine parameters for friction simulations.
    pub deceleration_rate: ScrollDecelerationRate,

    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl Default for BouncingScrollPhysics {
    fn default() -> BouncingScrollPhysics {
        BouncingScrollPhysics {
            deceleration_rate: ScrollDecelerationRate::Normal,
            parent: None,
        }
    }
}

impl BouncingScrollPhysics {
    /// Creates scroll physics that bounce back from the edge.
    pub fn new() -> BouncingScrollPhysics {
        BouncingScrollPhysics::default()
    }

    /// Dart `BouncingScrollPhysics(decelerationRate:)`.
    pub fn with_deceleration_rate(
        mut self,
        deceleration_rate: ScrollDecelerationRate,
    ) -> BouncingScrollPhysics {
        self.deceleration_rate = deceleration_rate;
        self
    }

    /// Dart `BouncingScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> BouncingScrollPhysics {
        self.parent = Some(parent);
        self
    }

    /// The multiple applied to overscroll to make it appear that scrolling past
    /// the edge of the scrollable contents is harder than scrolling within bounds.
    /// This is done by reducing the ratio of the scroll effect output vs the
    /// scroll gesture input.
    ///
    /// This factor starts at 0.52 for [`ScrollDecelerationRate::Normal`] and 0.26 for
    /// [`ScrollDecelerationRate::Fast`].
    ///
    /// The `overscroll_fraction` represents how far past the edge the user has
    /// dragged, where 0.0 means no overscroll. As this value increases, the
    /// friction factor decreases quadratically, making further overscroll harder.
    pub fn friction_factor(&self, overscroll_fraction: f64) -> f64 {
        (1.0 - overscroll_fraction).powi(2)
            * match self.deceleration_rate {
                ScrollDecelerationRate::Fast => 0.26,
                ScrollDecelerationRate::Normal => 0.52,
            }
    }
}

fn apply_friction(extent_outside: f64, abs_delta: f64, gamma: f64) -> f64 {
    debug_assert!(abs_delta > 0.0);
    let mut abs_delta = abs_delta;
    let mut total = 0.0;
    if extent_outside > 0.0 {
        let delta_to_limit = extent_outside / gamma;
        if abs_delta < delta_to_limit {
            return abs_delta * gamma;
        }
        total += extent_outside;
        abs_delta -= delta_to_limit;
    }
    total + abs_delta
}

impl ScrollPhysics for BouncingScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(BouncingScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
            deceleration_rate: self.deceleration_rate,
        })
    }

    fn apply_physics_to_user_offset(&self, position: &dyn ScrollMetrics, offset: f64) -> f64 {
        debug_assert!(offset != 0.0);
        debug_assert!(position.min_scroll_extent() <= position.max_scroll_extent());

        if !position.out_of_range() {
            return offset;
        }

        let overscroll_past_start = (position.min_scroll_extent() - position.pixels()).max(0.0);
        let overscroll_past_end = (position.pixels() - position.max_scroll_extent()).max(0.0);
        let overscroll_past = overscroll_past_start.max(overscroll_past_end);
        let easing = (overscroll_past_start > 0.0 && offset < 0.0)
            || (overscroll_past_end > 0.0 && offset > 0.0);

        let friction = if easing {
            // Apply less resistance when easing the overscroll vs tensioning.
            self.friction_factor((overscroll_past - offset.abs()) / position.viewport_dimension())
        } else {
            self.friction_factor(overscroll_past / position.viewport_dimension())
        };
        let direction = offset.signum();

        if easing && self.deceleration_rate == ScrollDecelerationRate::Fast {
            return direction * offset.abs();
        }
        direction * apply_friction(overscroll_past, offset.abs(), friction)
    }

    fn apply_boundary_conditions(&self, _position: &dyn ScrollMetrics, _value: f64) -> f64 {
        0.0
    }

    fn create_ballistic_simulation(
        &self,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        let tolerance = self.tolerance_for(position);
        if velocity.abs() >= tolerance.velocity || position.out_of_range() {
            return Some(Box::new(BouncingScrollSimulation::with_options(
                position.pixels(),
                velocity,
                position.min_scroll_extent(),
                position.max_scroll_extent(),
                self.spring(),
                match self.deceleration_rate {
                    ScrollDecelerationRate::Fast => 1400.0,
                    ScrollDecelerationRate::Normal => 0.0,
                },
                tolerance,
            )));
        }
        None
    }

    // The ballistic simulation here decelerates more slowly than the one for
    // ClampingScrollPhysics so we require a more deliberate input gesture
    // to trigger a fling.
    fn min_fling_velocity(&self) -> f64 {
        K_MIN_FLING_VELOCITY * 2.0
    }

    // Methodology:
    // 1- Use https://github.com/flutter/platform_tests/tree/main/scroll_overlay to test with
    //    Flutter and platform scroll views superimposed.
    // 3- If the scrollables stopped overlapping at any moment, adjust the desired
    //    output value of this function at that input speed.
    // 4- Feed new input/output set into a power curve fitter. Change function
    //    and repeat from 2.
    // 5- Repeat from 2 with medium and slow flings.
    /// Momentum build-up function that mimics iOS's scroll speed increase with repeated
    /// flings.
    ///
    /// The velocity of the last fling is not an important factor. Existing speed
    /// and (related) time since last fling are factors for the velocity transfer
    /// calculations.
    fn carried_momentum(&self, existing_velocity: f64) -> f64 {
        existing_velocity.signum() * (0.000816 * existing_velocity.abs().powf(1.967)).min(40000.0)
    }

    // Eyeballed from observation to counter the effect of an unintended scroll
    // from the natural motion of lifting the finger after a scroll.
    fn drag_start_distance_motion_threshold(&self) -> Option<f64> {
        Some(3.5)
    }

    fn max_fling_velocity(&self) -> f64 {
        match self.deceleration_rate {
            ScrollDecelerationRate::Fast => K_MAX_FLING_VELOCITY * 8.0,
            ScrollDecelerationRate::Normal => ScrollPhysicsBase::max_fling_velocity(self),
        }
    }

    fn spring(&self) -> SpringDescription {
        match self.deceleration_rate {
            ScrollDecelerationRate::Fast => {
                SpringDescription::with_damping_ratio_value(0.3, 75.0, 1.3)
            }
            ScrollDecelerationRate::Normal => ScrollPhysicsBase::spring(self),
        }
    }
}

impl Debug for BouncingScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "BouncingScrollPhysics", self.parent.as_ref())
    }
}

/// Scroll physics for environments that prevent the scroll offset from reaching
/// beyond the bounds of the content.
///
/// This is the behavior typically seen on Android.
///
/// See also:
///
///  * `ScrollConfiguration`, which uses this to provide the default
///    scroll behavior on Android.
///  * [`BouncingScrollPhysics`], which is the analogous physics for iOS' bouncing
///    behavior.
///  * `GlowingOverscrollIndicator`, which is used by `ScrollConfiguration` to
///    provide the glowing effect that is usually found with this clamping effect
///    on Android.
#[derive(Default)]
pub struct ClampingScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl ClampingScrollPhysics {
    /// Creates scroll physics that prevent the scroll offset from exceeding the
    /// bounds of the content.
    pub fn new() -> ClampingScrollPhysics {
        ClampingScrollPhysics::default()
    }

    /// Dart `ClampingScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> ClampingScrollPhysics {
        self.parent = Some(parent);
        self
    }
}

impl ScrollPhysics for ClampingScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(ClampingScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn apply_boundary_conditions(&self, position: &dyn ScrollMetrics, value: f64) -> f64 {
        debug_assert!(
            value != position.pixels(),
            "ClampingScrollPhysics.applyBoundaryConditions() was called redundantly.\n\
             The proposed new position, {value}, is exactly equal to the current position \
             of the given ScrollMetrics, {}.\n\
             The applyBoundaryConditions method should only be called when the value is \
             going to actually change the pixels, otherwise it is redundant.",
            position.pixels()
        );
        if value < position.pixels() && position.pixels() <= position.min_scroll_extent() {
            // Underscroll.
            return value - position.pixels();
        }
        if position.max_scroll_extent() <= position.pixels() && position.pixels() < value {
            // Overscroll.
            return value - position.pixels();
        }
        if value < position.min_scroll_extent() && position.min_scroll_extent() < position.pixels()
        {
            // Hit top edge.
            return value - position.min_scroll_extent();
        }
        if position.pixels() < position.max_scroll_extent() && position.max_scroll_extent() < value
        {
            // Hit bottom edge.
            return value - position.max_scroll_extent();
        }
        0.0
    }

    fn create_ballistic_simulation(
        &self,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        let tolerance = self.tolerance_for(position);
        if position.out_of_range() {
            let mut end = None;
            if position.pixels() > position.max_scroll_extent() {
                end = Some(position.max_scroll_extent());
            }
            if position.pixels() < position.min_scroll_extent() {
                end = Some(position.min_scroll_extent());
            }
            let end = end.expect("an out-of-range position is past one of the two extents");
            return Some(Box::new(ScrollSpringSimulation::new(
                self.spring(),
                position.pixels(),
                end,
                velocity.min(0.0),
                tolerance,
            )));
        }
        if velocity.abs() < tolerance.velocity {
            return None;
        }
        if velocity > 0.0 && position.pixels() >= position.max_scroll_extent() {
            return None;
        }
        if velocity < 0.0 && position.pixels() <= position.min_scroll_extent() {
            return None;
        }
        Some(Box::new(ClampingScrollSimulation::with_options(
            position.pixels(),
            velocity,
            ClampingScrollSimulation::DEFAULT_FRICTION,
            tolerance,
        )))
    }
}

impl Debug for ClampingScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "ClampingScrollPhysics", self.parent.as_ref())
    }
}

/// Scroll physics that always lets the user scroll.
///
/// This overrides the default behavior which is to disable scrolling
/// when there is no content to scroll. It does not override the
/// handling of overscrolling.
///
/// On Android, overscrolls will be clamped by default and result in an
/// overscroll glow. On iOS, overscrolls will load a spring that will return the
/// scroll view to its normal range when released.
///
/// See also:
///
///  * [`ScrollPhysics`], which can be used instead of this type when the default
///    behavior is desired instead.
///  * [`BouncingScrollPhysics`], which provides the bouncing overscroll behavior
///    found on iOS.
///  * [`ClampingScrollPhysics`], which provides the clamping overscroll behavior
///    found on Android.
#[derive(Default)]
pub struct AlwaysScrollableScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl AlwaysScrollableScrollPhysics {
    /// Creates scroll physics that always lets the user scroll.
    pub fn new() -> AlwaysScrollableScrollPhysics {
        AlwaysScrollableScrollPhysics::default()
    }

    /// Dart `AlwaysScrollableScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> AlwaysScrollableScrollPhysics {
        self.parent = Some(parent);
        self
    }
}

impl ScrollPhysics for AlwaysScrollableScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(AlwaysScrollableScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn should_accept_user_offset(&self, _position: &dyn ScrollMetrics) -> bool {
        true
    }
}

impl Debug for AlwaysScrollableScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "AlwaysScrollableScrollPhysics", self.parent.as_ref())
    }
}

/// Scroll physics that does not allow the user to scroll.
///
/// See also:
///
///  * [`ScrollPhysics`], which can be used instead of this type when the default
///    behavior is desired instead.
///  * [`BouncingScrollPhysics`], which provides the bouncing overscroll behavior
///    found on iOS.
///  * [`ClampingScrollPhysics`], which provides the clamping overscroll behavior
///    found on Android.
#[derive(Default)]
pub struct NeverScrollableScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl NeverScrollableScrollPhysics {
    /// Creates scroll physics that does not let the user scroll.
    pub fn new() -> NeverScrollableScrollPhysics {
        NeverScrollableScrollPhysics::default()
    }

    /// Dart `NeverScrollableScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> NeverScrollableScrollPhysics {
        self.parent = Some(parent);
        self
    }
}

impl ScrollPhysics for NeverScrollableScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(NeverScrollableScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn allow_implicit_scrolling(&self) -> bool {
        false
    }

    fn allow_user_scrolling(&self) -> bool {
        false
    }
}

impl Debug for NeverScrollableScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_scroll_physics(f, "NeverScrollableScrollPhysics", self.parent.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use inset_painting::AxisDirection;

    use super::*;

    /// `scroll_physics_test.dart`'s `TestScrollPhysics`: physics that name themselves so a
    /// chain of parents can be read back.
    struct TestScrollPhysics {
        name: &'static str,
        parent: Option<ScrollPhysicsRef>,
    }

    impl TestScrollPhysics {
        fn new(name: &'static str) -> TestScrollPhysics {
            TestScrollPhysics { name, parent: None }
        }
    }

    impl ScrollPhysics for TestScrollPhysics {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn parent(&self) -> Option<&ScrollPhysicsRef> {
            self.parent.as_ref()
        }

        fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
            Rc::new(TestScrollPhysics {
                name: self.name,
                parent: ScrollPhysicsBase::build_parent(self, ancestor),
            })
        }
    }

    impl Debug for TestScrollPhysics {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let name = format!("TestScrollPhysics({})", self.name);
            fmt_scroll_physics(f, &name, self.parent.as_ref())
        }
    }

    fn metrics(pixels: f64) -> FixedScrollMetrics {
        FixedScrollMetrics::new(
            Some(0.0),
            Some(1000.0),
            Some(pixels),
            Some(100.0),
            AxisDirection::Down,
            3.0,
        )
    }

    /// `scroll_physics_test.dart`: "ScrollPhysics applyTo()".
    #[test]
    fn apply_to_appends_the_ancestor_to_the_end_of_the_parent_chain() {
        let a = TestScrollPhysics::new("a");
        let b = TestScrollPhysics::new("b");
        let c = TestScrollPhysics::new("c");
        let d = TestScrollPhysics::new("d");
        let e = TestScrollPhysics::new("e");

        assert!(a.parent().is_none());
        assert!(b.parent().is_none());
        assert!(c.parent().is_none());

        let ab = a.apply_to(Some(Rc::new(b)));
        assert_eq!(
            format!("{ab:?}"),
            "TestScrollPhysics(a) -> TestScrollPhysics(b)"
        );

        let abc = ab.apply_to(Some(Rc::new(c)));
        assert_eq!(
            format!("{abc:?}"),
            "TestScrollPhysics(a) -> TestScrollPhysics(b) -> TestScrollPhysics(c)"
        );

        let de = d.apply_to(Some(Rc::new(e)));
        assert_eq!(
            format!("{de:?}"),
            "TestScrollPhysics(d) -> TestScrollPhysics(e)"
        );

        let abcde = abc.apply_to(Some(de));
        assert_eq!(
            format!("{abcde:?}"),
            "TestScrollPhysics(a) -> TestScrollPhysics(b) -> TestScrollPhysics(c) \
             -> TestScrollPhysics(d) -> TestScrollPhysics(e)"
        );
    }

    /// `scroll_physics_test.dart`: "ScrollPhysics subclasses applyTo()".
    #[test]
    fn apply_to_keeps_each_type_and_its_own_arguments() {
        let chain = BouncingScrollPhysics::new().apply_to(Some(
            ClampingScrollPhysics::new().apply_to(Some(
                NeverScrollableScrollPhysics::new()
                    .apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new()))),
            )),
        ));
        assert_eq!(
            format!("{chain:?}"),
            "BouncingScrollPhysics -> ClampingScrollPhysics -> NeverScrollableScrollPhysics \
             -> AlwaysScrollableScrollPhysics"
        );

        assert_eq!(format!("{:?}", PlainScrollPhysics::new()), "ScrollPhysics");
        assert_eq!(
            format!(
                "{:?}",
                RangeMaintainingScrollPhysics::new()
                    .apply_to(Some(Rc::new(ClampingScrollPhysics::new())))
            ),
            "RangeMaintainingScrollPhysics -> ClampingScrollPhysics"
        );

        let desktop = BouncingScrollPhysics::new()
            .with_deceleration_rate(ScrollDecelerationRate::Fast)
            .apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
        // The `decelerationRate` survives the copy: only `fast` raises the fling ceiling.
        assert_eq!(desktop.max_fling_velocity(), K_MAX_FLING_VELOCITY * 8.0);
    }

    /// `scroll_physics_test.dart`: "overscroll is progressively harder" and "no resistance
    /// when not overscrolled".
    #[test]
    fn bouncing_scroll_physics_applies_friction_only_past_the_edge() {
        let physics = BouncingScrollPhysics::new();

        let inside = metrics(300.0);
        assert_eq!(physics.apply_physics_to_user_offset(&inside, 10.0), 10.0);
        assert_eq!(physics.apply_physics_to_user_offset(&inside, -10.0), -10.0);

        let less = physics.apply_physics_to_user_offset(&metrics(-20.0), 10.0);
        let more = physics.apply_physics_to_user_offset(&metrics(-40.0), 10.0);
        assert!(less > 1.0 && less < 20.0);
        assert!(more > 1.0 && more < 20.0);
        // Scrolling from a more overscrolled position meets more resistance.
        assert!(less.abs() > more.abs());

        // Easing an overscroll still has resistance, but less than tensioning it.
        let easing = physics.apply_physics_to_user_offset(&metrics(-20.0), -10.0);
        assert!(easing < -1.0 && easing > -10.0);
        assert!(easing.abs() > less.abs());

        // `frictionFactor` starts at 0.52 (0.26 for the desktop deceleration rate).
        let desktop =
            BouncingScrollPhysics::new().with_deceleration_rate(ScrollDecelerationRate::Fast);
        assert_eq!(physics.friction_factor(0.0), 0.52);
        assert_eq!(desktop.friction_factor(0.0), 0.26);
        assert_eq!(
            desktop.apply_physics_to_user_offset(&metrics(-20.0), -10.0),
            -10.0
        );

        // The boundary never clamps: overscroll is allowed unhindered.
        assert_eq!(
            physics.apply_boundary_conditions(&metrics(-20.0), -30.0),
            0.0
        );
    }

    #[test]
    fn bouncing_scroll_physics_builds_a_simulation_that_bounces_back() {
        let physics = BouncingScrollPhysics::new();
        let overscrolled = metrics(-20.0);
        let simulation = physics
            .create_ballistic_simulation(&overscrolled, 0.0)
            .expect("an out-of-range position always gets a simulation");
        assert_eq!(simulation.x(0.0), -20.0);

        let mut time = 0.0;
        while !simulation.is_done(time) {
            time += 1.0 / 60.0;
            assert!(time < 10.0, "the spring must settle");
        }
        assert!(
            simulation.x(time).abs() < 0.1,
            "settled at {}",
            simulation.x(time)
        );

        // In range and at rest, there is nothing to simulate.
        assert!(
            physics
                .create_ballistic_simulation(&metrics(300.0), 0.0)
                .is_none()
        );
        // Creating the simulation does not alter the velocity at time zero.
        let flung = physics
            .create_ballistic_simulation(&metrics(20.0), 1000.0)
            .expect("a fling gets a simulation");
        assert!((flung.dx(0.0) - 1000.0).abs() < 1e-9);
    }

    /// `scroll_physics_test.dart`: "ClampingScrollPhysics assertion test".
    #[test]
    fn clamping_scroll_physics_clamps_at_the_boundary() {
        let physics = ClampingScrollPhysics::new();
        let inside = metrics(500.0);

        // Hitting an edge reports only the part beyond it.
        assert_eq!(physics.apply_boundary_conditions(&inside, 1500.0), 500.0);
        assert_eq!(physics.apply_boundary_conditions(&inside, -500.0), -500.0);
        // Movement that stays in range is not clamped at all.
        assert_eq!(physics.apply_boundary_conditions(&inside, 600.0), 0.0);
        assert_eq!(physics.apply_boundary_conditions(&inside, 400.0), 0.0);

        // Already out of range: a further increase is reported, a decrease is allowed.
        let over = metrics(1200.0);
        assert_eq!(physics.apply_boundary_conditions(&over, 1300.0), 100.0);
        assert_eq!(physics.apply_boundary_conditions(&over, 1100.0), 0.0);
    }

    #[test]
    fn clamping_scroll_physics_builds_a_simulation_that_stops_at_the_edge() {
        let physics = ClampingScrollPhysics::new();

        let over = metrics(1200.0);
        let spring = physics
            .create_ballistic_simulation(&over, 0.0)
            .expect("an out-of-range position always gets a simulation");
        let mut time = 0.0;
        while !spring.is_done(time) {
            time += 1.0 / 60.0;
            assert!(time < 10.0, "the spring must settle");
        }
        assert!(
            (spring.x(time) - 1000.0).abs() < 0.1,
            "settled at {}",
            spring.x(time)
        );

        // A fling inside the range decelerates and stops before the edge is passed.
        let fling = physics
            .create_ballistic_simulation(&metrics(20.0), 1000.0)
            .expect("a fling gets a simulation");
        assert!((fling.dx(0.0) - 1000.0).abs() < 1e-9);
        assert!(fling.is_done(1.0));

        // Nothing to simulate when at rest, or when flung away from a reached edge.
        assert!(
            physics
                .create_ballistic_simulation(&metrics(300.0), 0.0)
                .is_none()
        );
        assert!(
            physics
                .create_ballistic_simulation(&metrics(1000.0), 1000.0)
                .is_none()
        );
        assert!(
            physics
                .create_ballistic_simulation(&metrics(0.0), -1000.0)
                .is_none()
        );
    }

    #[test]
    fn always_and_never_scrollable_decide_whether_the_user_may_scroll() {
        let empty = FixedScrollMetrics::new(
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(600.0),
            AxisDirection::Down,
            3.0,
        );

        // The default: only when there is content outside the viewport to reveal.
        assert!(!PlainScrollPhysics::new().should_accept_user_offset(&empty));
        assert!(PlainScrollPhysics::new().should_accept_user_offset(&metrics(0.0)));

        assert!(AlwaysScrollableScrollPhysics::new().should_accept_user_offset(&empty));

        let never = NeverScrollableScrollPhysics::new();
        assert!(!never.should_accept_user_offset(&metrics(0.0)));
        assert!(!never.allow_user_scrolling());
        assert!(!never.allow_implicit_scrolling());
        // `allowUserScrolling` wins over an ancestor that always scrolls.
        let never_over_always = NeverScrollableScrollPhysics::new()
            .apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
        assert!(!never_over_always.should_accept_user_offset(&empty));
    }

    #[test]
    fn a_parent_answers_what_this_physics_has_no_opinion_on() {
        let bouncing_over_always = BouncingScrollPhysics::new()
            .apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
        // `BouncingScrollPhysics` does not override `shouldAcceptUserOffset`.
        let empty = FixedScrollMetrics::new(
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(600.0),
            AxisDirection::Down,
            3.0,
        );
        assert!(bouncing_over_always.should_accept_user_offset(&empty));
        // ...but it keeps its own fling floor and momentum.
        assert_eq!(
            bouncing_over_always.min_fling_velocity(),
            K_MIN_FLING_VELOCITY * 2.0
        );
        assert_eq!(
            bouncing_over_always.drag_start_distance_motion_threshold(),
            Some(3.5)
        );
        assert_eq!(PlainScrollPhysics::new().carried_momentum(1000.0), 0.0);
        assert!(BouncingScrollPhysics::new().carried_momentum(1000.0) > 0.0);
        assert_eq!(PlainScrollPhysics::new().min_fling_distance(), K_TOUCH_SLOP);
    }

    #[test]
    fn range_maintaining_physics_keeps_the_overscroll_when_content_shrinks() {
        let physics = RangeMaintainingScrollPhysics::new();
        let old = FixedScrollMetrics::new(
            Some(0.0),
            Some(1000.0),
            Some(1100.0),
            Some(600.0),
            AxisDirection::Down,
            3.0,
        );
        let new = old.copy_with().with_max_scroll_extent(800.0);
        // The 100 pixels of overscroll survive the smaller extent.
        assert_eq!(
            physics.adjust_position_for_new_dimensions(&old, &new, false, 0.0),
            900.0
        );

        // An in-range position is pushed back inside the new boundary.
        let old_inside = old.copy_with().with_pixels(900.0);
        let new_inside = old_inside.copy_with().with_max_scroll_extent(800.0);
        assert_eq!(
            physics.adjust_position_for_new_dimensions(&old_inside, &new_inside, false, 0.0),
            800.0
        );

        // An animating position is left alone.
        assert_eq!(
            physics.adjust_position_for_new_dimensions(&old_inside, &new_inside, true, 10.0),
            900.0
        );
    }
}

//! Flutter counterpart: `widgets/scroll_configuration.dart`.

use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::ops::Deref;
use std::rc::Rc;

use inset_embedder::{PointerDeviceKind, TargetPlatform};
use inset_foundation::App;
use inset_gestures::{
    GestureVelocityTrackerBuilder, IOSScrollViewFlingVelocityTracker,
    MacOSScrollViewFlingVelocityTracker, MultitouchDragStrategy, VelocityTracker,
};
use inset_services::LogicalKeyboardKey;

use crate::framework::{BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef};
use crate::widgets::scroll_physics::{
    BouncingScrollPhysics, ClampingScrollPhysics, RangeMaintainingScrollPhysics,
    ScrollDecelerationRate, ScrollPhysicsRef,
};
use crate::widgets::scroll_view::ScrollViewKeyboardDismissBehavior;
use crate::widgets::scrollable_helpers::ScrollableDetails;
use crate::widgets::scrollbar::RawScrollbar;

/// Device types that scrollables should accept drag gestures from by default.
///
/// Dart's `_kTouchLikeDeviceTypes`, a function because a `HashSet` has no `const`
/// constructor.
fn k_touch_like_device_types() -> HashSet<PointerDeviceKind> {
    HashSet::from([
        PointerDeviceKind::Touch,
        PointerDeviceKind::Stylus,
        PointerDeviceKind::InvertedStylus,
        PointerDeviceKind::Trackpad,
        // The VoiceAccess sends pointer events with unknown type when scrolling
        // scrollables.
        PointerDeviceKind::Unknown,
    ])
}

/// Types of overscroll indicators supported by [`TargetPlatform::Android`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AndroidOverscrollIndicator {
    /// Utilizes a `StretchingOverscrollIndicator`, which transforms the contents
    /// of a `ScrollView` when overscrolled.
    Stretch,

    /// Utilizes a `GlowingOverscrollIndicator`, painting a glowing semi circle on
    /// top of the `ScrollView` in response to overscrolling.
    Glow,
}

/// Describes how `Scrollable` widgets should behave.
///
/// Used by [`ScrollConfiguration`] to configure the `Scrollable` widgets in a
/// subtree.
///
/// This trait can be implemented to further customize a [`ScrollBehavior`] for a
/// subtree. For example, overriding [`get_scroll_physics`](Self::get_scroll_physics) sets the
/// default [`ScrollPhysics`](crate::ScrollPhysics) for `Scrollable`s that inherit this [`ScrollConfiguration`].
///
/// When looking to easily toggle the default decorations, use
/// [`ScrollBehaviorRef::copy_with`] instead of writing a new [`ScrollBehavior`].
/// The `scrollbars` and `overscroll` flags can turn these decorations off.
///
/// See also:
///
///   * [`ScrollConfiguration`], the inherited widget that controls how
///     `Scrollable` widgets behave in a subtree.
pub trait ScrollBehavior: Debug + 'static {
    /// The value behind the erased behavior, for Dart's `behavior.runtimeType` and the
    /// covariant `oldDelegate` a [`should_notify`](Self::should_notify) override casts.
    fn as_any(&self) -> &dyn Any;

    /// The platform whose scroll physics should be implemented.
    ///
    /// Defaults to the current platform.
    fn get_platform(&self, app: &App, context: BuildContext) -> TargetPlatform {
        ScrollBehaviorBase::get_platform(app, context)
    }

    /// The device kinds that the scrollable will accept drag gestures from.
    ///
    /// By default only [`PointerDeviceKind::Touch`], [`PointerDeviceKind::Stylus`],
    /// [`PointerDeviceKind::InvertedStylus`], and [`PointerDeviceKind::Trackpad`]
    /// are configured to create drag gestures. Enabling this for
    /// [`PointerDeviceKind::Mouse`] will make it difficult or impossible to select
    /// text in scrollable containers and is not recommended.
    fn drag_devices(&self) -> HashSet<PointerDeviceKind> {
        ScrollBehaviorBase::drag_devices()
    }

    /// The multi-finger drag strategy on multi-touch devices.
    ///
    /// By default, [`MultitouchDragStrategy::LatestPointer`] is configured to
    /// create drag gestures for non-Apple platforms, and
    /// [`MultitouchDragStrategy::AverageBoundaryPointers`] for Apple platforms.
    fn get_multitouch_drag_strategy(
        &self,
        app: &App,
        context: BuildContext,
    ) -> MultitouchDragStrategy {
        ScrollBehaviorBase::get_multitouch_drag_strategy(self, app, context)
    }

    /// A set of [`LogicalKeyboardKey`]s that, when any or all are pressed in
    /// combination with a [`PointerDeviceKind::Mouse`] pointer scroll event, will
    /// flip the axes of the scroll input.
    ///
    /// This will for example, result in the input of a vertical mouse wheel, to
    /// move the `ScrollPosition` of a `ScrollView` with an
    /// [`inset_painting::Axis::Horizontal`] scroll direction.
    ///
    /// If other keys exclusive of this set are pressed during a scroll event, in
    /// conjunction with keys from this set, the scroll input will still be
    /// flipped.
    ///
    /// Defaults to [`LogicalKeyboardKey::SHIFT_LEFT`] and
    /// [`LogicalKeyboardKey::SHIFT_RIGHT`].
    fn pointer_axis_modifiers(&self) -> HashSet<LogicalKeyboardKey> {
        ScrollBehaviorBase::pointer_axis_modifiers()
    }

    /// Applies a [`RawScrollbar`] to the child widget on desktop platforms.
    fn build_scrollbar(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        ScrollBehaviorBase::build_scrollbar(self, app, context, child, details)
    }

    /// Applies a `GlowingOverscrollIndicator` to the child widget on
    /// [`TargetPlatform::Android`] and [`TargetPlatform::Fuchsia`].
    fn build_overscroll_indicator(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        ScrollBehaviorBase::build_overscroll_indicator(app, context, child, details)
    }

    /// Specifies the type of velocity tracker to use in the descendant
    /// `Scrollable`s' drag gesture recognizers, for estimating the velocity of a
    /// drag gesture.
    ///
    /// This can be used to, for example, apply different fling velocity
    /// estimation methods on different platforms, in order to match the
    /// platform's native behavior.
    ///
    /// The default implementation provides a new
    /// [`IOSScrollViewFlingVelocityTracker`] on iOS and macOS for each new pointer,
    /// and a new [`VelocityTracker`] on other platforms for each new pointer.
    fn velocity_tracker_builder(
        &self,
        app: &App,
        context: BuildContext,
    ) -> GestureVelocityTrackerBuilder {
        ScrollBehaviorBase::velocity_tracker_builder(self, app, context)
    }

    /// The scroll physics to use for the platform given by
    /// [`get_platform`](Self::get_platform).
    ///
    /// Defaults to [`RangeMaintainingScrollPhysics`] mixed with
    /// [`BouncingScrollPhysics`] on iOS and [`ClampingScrollPhysics`] on
    /// Android.
    fn get_scroll_physics(&self, app: &App, context: BuildContext) -> ScrollPhysicsRef {
        ScrollBehaviorBase::get_scroll_physics(self, app, context)
    }

    /// Called whenever a [`ScrollConfiguration`] is rebuilt with a new
    /// [`ScrollBehavior`] of the same runtime type.
    ///
    /// If the new instance represents different information than the old
    /// instance, then the method should return true, otherwise it should return
    /// false.
    ///
    /// If this method returns true, all the widgets that inherit from the
    /// [`ScrollConfiguration`] will rebuild using the new [`ScrollBehavior`]. If this
    /// method returns false, the rebuilds might be optimized away.
    fn should_notify(&self, old_delegate: &dyn ScrollBehavior) -> bool {
        let _ = old_delegate;
        false
    }

    /// The default keyboard dismissal behavior for `ScrollView` widgets.
    ///
    /// Defaults to [`ScrollViewKeyboardDismissBehavior::Manual`].
    fn get_keyboard_dismiss_behavior(
        &self,
        app: &App,
        context: BuildContext,
    ) -> ScrollViewKeyboardDismissBehavior {
        let _ = (app, context);
        ScrollViewKeyboardDismissBehavior::Manual
    }
}

/// The shared bodies of Dart's `ScrollBehavior`: call one where Dart writes `super.…`.
pub struct ScrollBehaviorBase;

impl ScrollBehaviorBase {
    /// See [`ScrollBehavior::get_platform`].
    pub fn get_platform(app: &App, context: BuildContext) -> TargetPlatform {
        let _ = context;
        app.platform().target_platform()
    }

    /// See [`ScrollBehavior::drag_devices`].
    pub fn drag_devices() -> HashSet<PointerDeviceKind> {
        k_touch_like_device_types()
    }

    /// See [`ScrollBehavior::get_multitouch_drag_strategy`].
    pub fn get_multitouch_drag_strategy<B: ScrollBehavior + ?Sized>(
        behavior: &B,
        app: &App,
        context: BuildContext,
    ) -> MultitouchDragStrategy {
        match behavior.get_platform(app, context) {
            TargetPlatform::MacOS | TargetPlatform::IOS => {
                MultitouchDragStrategy::AverageBoundaryPointers
            }
            TargetPlatform::Linux
            | TargetPlatform::Windows
            | TargetPlatform::Android
            | TargetPlatform::Fuchsia => MultitouchDragStrategy::LatestPointer,
        }
    }

    /// See [`ScrollBehavior::pointer_axis_modifiers`].
    pub fn pointer_axis_modifiers() -> HashSet<LogicalKeyboardKey> {
        HashSet::from([
            LogicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_RIGHT,
        ])
    }

    /// See [`ScrollBehavior::build_scrollbar`].
    pub fn build_scrollbar<B: ScrollBehavior + ?Sized>(
        behavior: &B,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // When modifying this function, consider modifying the implementation in the
        // Cupertino subclass as well.
        match behavior.get_platform(app, context) {
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                debug_assert!(details.controller.is_some());
                let mut scrollbar = RawScrollbar::new(child);
                if let Some(controller) = details.controller {
                    scrollbar = scrollbar.controller(controller);
                }
                scrollbar.into_widget()
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => child,
        }
    }

    /// See [`ScrollBehavior::build_overscroll_indicator`].
    pub fn build_overscroll_indicator(
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // Dart wraps the child in a `GlowingOverscrollIndicator` on Android and Fuchsia; the
        // indicator widgets wait, so every platform takes Dart's other fallthrough.
        let _ = (app, context, details);
        child
    }

    /// See [`ScrollBehavior::velocity_tracker_builder`].
    pub fn velocity_tracker_builder<B: ScrollBehavior + ?Sized>(
        behavior: &B,
        app: &App,
        context: BuildContext,
    ) -> GestureVelocityTrackerBuilder {
        match behavior.get_platform(app, context) {
            TargetPlatform::IOS => {
                Rc::new(|event| Box::new(IOSScrollViewFlingVelocityTracker::new(event.kind())))
            }
            TargetPlatform::MacOS => {
                Rc::new(|event| Box::new(MacOSScrollViewFlingVelocityTracker::new(event.kind())))
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                Rc::new(|event| Box::new(VelocityTracker::with_kind(event.kind())))
            }
        }
    }

    /// Dart's `ScrollBehavior._bouncingPhysics`.
    fn bouncing_physics() -> ScrollPhysicsRef {
        Rc::new(
            BouncingScrollPhysics::new().with_parent(Rc::new(RangeMaintainingScrollPhysics::new())),
        )
    }

    /// Dart's `ScrollBehavior._bouncingDesktopPhysics`.
    fn bouncing_desktop_physics() -> ScrollPhysicsRef {
        Rc::new(
            BouncingScrollPhysics::new()
                .with_deceleration_rate(ScrollDecelerationRate::Fast)
                .with_parent(Rc::new(RangeMaintainingScrollPhysics::new())),
        )
    }

    /// Dart's `ScrollBehavior._clampingPhysics`.
    fn clamping_physics() -> ScrollPhysicsRef {
        Rc::new(
            ClampingScrollPhysics::new().with_parent(Rc::new(RangeMaintainingScrollPhysics::new())),
        )
    }

    /// See [`ScrollBehavior::get_scroll_physics`].
    pub fn get_scroll_physics<B: ScrollBehavior + ?Sized>(
        behavior: &B,
        app: &App,
        context: BuildContext,
    ) -> ScrollPhysicsRef {
        match behavior.get_platform(app, context) {
            TargetPlatform::IOS => ScrollBehaviorBase::bouncing_physics(),
            TargetPlatform::MacOS => ScrollBehaviorBase::bouncing_desktop_physics(),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => ScrollBehaviorBase::clamping_physics(),
        }
    }
}

/// Dart's instantiable `const ScrollBehavior()`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct PlainScrollBehavior;

impl PlainScrollBehavior {
    /// Creates a description of how `Scrollable` widgets should behave.
    pub const fn new() -> PlainScrollBehavior {
        PlainScrollBehavior
    }
}

impl ScrollBehavior for PlainScrollBehavior {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for PlainScrollBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart's `objectRuntimeType(this, 'ScrollBehavior')`.
        f.write_str("ScrollBehavior")
    }
}

/// A [`ScrollBehavior`] held by value: what a Dart field or parameter typed `ScrollBehavior`
/// becomes.
///
/// It is a newtype rather than a bare `Rc` so that [`copy_with`](Self::copy_with) — which
/// Dart writes as a method capturing `this` as the new wrapper's delegate — has an `Rc` to
/// capture.
#[derive(Clone)]
pub struct ScrollBehaviorRef(Rc<dyn ScrollBehavior>);

impl ScrollBehaviorRef {
    /// Erases a [`ScrollBehavior`].
    pub fn new(behavior: impl ScrollBehavior) -> ScrollBehaviorRef {
        ScrollBehaviorRef(Rc::new(behavior))
    }

    /// Dart's `const ScrollBehavior()`.
    pub fn plain() -> ScrollBehaviorRef {
        ScrollBehaviorRef::new(PlainScrollBehavior::new())
    }

    /// Creates a copy of this behavior, making it possible to
    /// easily toggle `scrollbars` and `overscroll` effects.
    ///
    /// This is used by widgets like `PageView` and `ListWheelScrollView` to
    /// override the current [`ScrollBehavior`] and manage how they are decorated.
    ///
    /// The result is a [`WrappedScrollBehavior`], whose fluent setters are Dart's named
    /// arguments; copying a wrapper flattens it, as Dart's
    /// `_WrappedScrollBehavior.copyWith` does.
    pub fn copy_with(&self) -> WrappedScrollBehavior {
        match self.0.as_any().downcast_ref::<WrappedScrollBehavior>() {
            Some(wrapped) => WrappedScrollBehavior {
                delegate: wrapped.delegate.clone(),
                scrollbars: wrapped.scrollbars,
                overscroll: wrapped.overscroll,
                drag_devices: Some(ScrollBehavior::drag_devices(wrapped)),
                multitouch_drag_strategy: wrapped.multitouch_drag_strategy,
                pointer_axis_modifiers: Some(ScrollBehavior::pointer_axis_modifiers(wrapped)),
                physics: wrapped.physics.clone(),
                platform: wrapped.platform,
                keyboard_dismiss_behavior: wrapped.keyboard_dismiss_behavior,
            },
            None => WrappedScrollBehavior::new(self.clone()),
        }
    }

    /// Dart's `behavior.runtimeType`.
    pub fn type_id(&self) -> TypeId {
        self.0.as_any().type_id()
    }
}

impl Deref for ScrollBehaviorRef {
    type Target = dyn ScrollBehavior;

    fn deref(&self) -> &dyn ScrollBehavior {
        &*self.0
    }
}

impl PartialEq for ScrollBehaviorRef {
    fn eq(&self, other: &ScrollBehaviorRef) -> bool {
        // Dart's `==` on a `ScrollBehavior` is object identity.
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Debug for ScrollBehaviorRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

/// The [`ScrollBehavior`] that [`ScrollBehaviorRef::copy_with`] returns; Dart's
/// `_WrappedScrollBehavior`.
#[derive(Clone)]
pub struct WrappedScrollBehavior {
    delegate: ScrollBehaviorRef,
    scrollbars: bool,
    overscroll: bool,
    drag_devices: Option<HashSet<PointerDeviceKind>>,
    multitouch_drag_strategy: Option<MultitouchDragStrategy>,
    pointer_axis_modifiers: Option<HashSet<LogicalKeyboardKey>>,
    physics: Option<ScrollPhysicsRef>,
    platform: Option<TargetPlatform>,
    keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,
}

impl WrappedScrollBehavior {
    /// Wraps `delegate` with Dart's `copyWith` defaults: both decorations on and every other
    /// override unset.
    pub fn new(delegate: ScrollBehaviorRef) -> WrappedScrollBehavior {
        WrappedScrollBehavior {
            delegate,
            scrollbars: true,
            overscroll: true,
            drag_devices: None,
            multitouch_drag_strategy: None,
            pointer_axis_modifiers: None,
            physics: None,
            platform: None,
            keyboard_dismiss_behavior: None,
        }
    }

    /// Dart `copyWith(scrollbars:)`.
    pub fn scrollbars(mut self, scrollbars: bool) -> WrappedScrollBehavior {
        self.scrollbars = scrollbars;
        self
    }

    /// Dart `copyWith(overscroll:)`.
    pub fn overscroll(mut self, overscroll: bool) -> WrappedScrollBehavior {
        self.overscroll = overscroll;
        self
    }

    /// Dart `copyWith(dragDevices:)`.
    pub fn drag_devices(
        mut self,
        drag_devices: HashSet<PointerDeviceKind>,
    ) -> WrappedScrollBehavior {
        self.drag_devices = Some(drag_devices);
        self
    }

    /// Dart `copyWith(multitouchDragStrategy:)`.
    pub fn multitouch_drag_strategy(
        mut self,
        multitouch_drag_strategy: MultitouchDragStrategy,
    ) -> WrappedScrollBehavior {
        self.multitouch_drag_strategy = Some(multitouch_drag_strategy);
        self
    }

    /// Dart `copyWith(pointerAxisModifiers:)`.
    pub fn pointer_axis_modifiers(
        mut self,
        pointer_axis_modifiers: HashSet<LogicalKeyboardKey>,
    ) -> WrappedScrollBehavior {
        self.pointer_axis_modifiers = Some(pointer_axis_modifiers);
        self
    }

    /// Dart `copyWith(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> WrappedScrollBehavior {
        self.physics = Some(physics);
        self
    }

    /// Dart `copyWith(platform:)`.
    pub fn platform(mut self, platform: TargetPlatform) -> WrappedScrollBehavior {
        self.platform = Some(platform);
        self
    }

    /// Dart `copyWith(keyboardDismissBehavior:)`.
    pub fn keyboard_dismiss_behavior(
        mut self,
        keyboard_dismiss_behavior: ScrollViewKeyboardDismissBehavior,
    ) -> WrappedScrollBehavior {
        self.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
        self
    }

    /// The behavior this one decorates.
    pub fn delegate(&self) -> &ScrollBehaviorRef {
        &self.delegate
    }
}

impl ScrollBehavior for WrappedScrollBehavior {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_platform(&self, app: &App, context: BuildContext) -> TargetPlatform {
        self.platform
            .unwrap_or_else(|| self.delegate.get_platform(app, context))
    }

    fn drag_devices(&self) -> HashSet<PointerDeviceKind> {
        self.drag_devices
            .clone()
            .unwrap_or_else(|| self.delegate.drag_devices())
    }

    fn get_multitouch_drag_strategy(
        &self,
        app: &App,
        context: BuildContext,
    ) -> MultitouchDragStrategy {
        self.multitouch_drag_strategy
            .unwrap_or_else(|| self.delegate.get_multitouch_drag_strategy(app, context))
    }

    fn pointer_axis_modifiers(&self) -> HashSet<LogicalKeyboardKey> {
        self.pointer_axis_modifiers
            .clone()
            .unwrap_or_else(|| self.delegate.pointer_axis_modifiers())
    }

    fn build_scrollbar(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        if self.scrollbars {
            return self.delegate.build_scrollbar(app, context, child, details);
        }
        child
    }

    fn build_overscroll_indicator(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        if self.overscroll {
            return self
                .delegate
                .build_overscroll_indicator(app, context, child, details);
        }
        child
    }

    fn velocity_tracker_builder(
        &self,
        app: &App,
        context: BuildContext,
    ) -> GestureVelocityTrackerBuilder {
        self.delegate.velocity_tracker_builder(app, context)
    }

    fn get_scroll_physics(&self, app: &App, context: BuildContext) -> ScrollPhysicsRef {
        self.physics
            .clone()
            .unwrap_or_else(|| self.delegate.get_scroll_physics(app, context))
    }

    fn get_keyboard_dismiss_behavior(
        &self,
        app: &App,
        context: BuildContext,
    ) -> ScrollViewKeyboardDismissBehavior {
        self.keyboard_dismiss_behavior
            .unwrap_or_else(|| self.delegate.get_keyboard_dismiss_behavior(app, context))
    }

    fn should_notify(&self, old_delegate: &dyn ScrollBehavior) -> bool {
        let old = old_delegate
            .as_any()
            .downcast_ref::<WrappedScrollBehavior>()
            .expect("a _WrappedScrollBehavior only compares against another one");
        old.delegate.type_id() != self.delegate.type_id()
            || old.scrollbars != self.scrollbars
            || old.overscroll != self.overscroll
            || ScrollBehavior::drag_devices(old) != ScrollBehavior::drag_devices(self)
            || old.multitouch_drag_strategy != self.multitouch_drag_strategy
            || ScrollBehavior::pointer_axis_modifiers(old)
                != ScrollBehavior::pointer_axis_modifiers(self)
            || !match (&old.physics, &self.physics) {
                (None, None) => true,
                (Some(old), Some(physics)) => Rc::ptr_eq(old, physics),
                _ => false,
            }
            || old.platform != self.platform
            || self.delegate.should_notify(&*old.delegate)
    }
}

impl Debug for WrappedScrollBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart's `objectRuntimeType(this, '_WrappedScrollBehavior')`.
        f.write_str("_WrappedScrollBehavior")
    }
}

/// Controls how `Scrollable` widgets behave in a subtree.
///
/// The scroll configuration determines the [`ScrollPhysics`](crate::ScrollPhysics) and viewport
/// decorations used by descendants of [`child`](Self::child).
#[derive(Debug)]
pub struct ScrollConfiguration {
    pub key: Option<KeyRef>,

    /// How `Scrollable` widgets that are descendants of [`child`](Self::child) should behave.
    pub behavior: ScrollBehaviorRef,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl ScrollConfiguration {
    /// Creates a widget that controls how `Scrollable` widgets behave in a subtree.
    pub fn new<K>(
        behavior: ScrollBehaviorRef,
        child: impl crate::framework::IntoWidget<K>,
    ) -> ScrollConfiguration {
        ScrollConfiguration {
            key: None,
            behavior,
            child: child.into_widget(),
        }
    }

    /// Dart `ScrollConfiguration(key:)`.
    pub fn key(mut self, key: KeyRef) -> ScrollConfiguration {
        self.key = Some(key);
        self
    }

    /// The [`ScrollBehavior`] for `Scrollable` widgets in the given [`BuildContext`].
    ///
    /// If no [`ScrollConfiguration`] widget is in scope of the given `context`,
    /// a default [`PlainScrollBehavior`] is returned.
    pub fn of(app: &mut App, context: BuildContext) -> ScrollBehaviorRef {
        context
            .depend_on_inherited_widget_of_exact_type::<ScrollConfiguration>(app)
            .map(|configuration| configuration.behavior.clone())
            .unwrap_or_else(ScrollBehaviorRef::plain)
    }
}

impl InheritedWidget for ScrollConfiguration {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ScrollConfiguration) -> bool {
        self.behavior.type_id() != old_widget.behavior.type_id()
            || (self.behavior != old_widget.behavior
                && self.behavior.should_notify(&*old_widget.behavior))
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::time::Instant;

    use inset_embedder::{InertPlatform, Platform, PlatformRef, ViewId, ViewRef};

    use super::*;
    use crate::framework::{AnyElement, Element, IntoWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    /// A host that claims to be one particular platform.
    struct FixedPlatform(TargetPlatform);

    impl Platform for FixedPlatform {
        fn target_platform(&self) -> TargetPlatform {
            self.0
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            InertPlatform.now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }
    }

    fn app_on(platform: TargetPlatform) -> Rc<AppCell> {
        AppCell::with_platform(Rc::new(FixedPlatform(platform)) as PlatformRef)
    }

    fn descendant(harness: &Harness, app: &App, depth: usize) -> AnyElement {
        let mut element = harness.root.as_element();
        for _ in 0..=depth {
            element = element.children(app)[0];
        }
        element
    }

    fn mount(app: &mut App, widget: crate::framework::WidgetRef) -> Harness {
        let harness = Harness::mount(app, widget);
        harness.pump(app);
        harness
    }

    #[test]
    fn the_default_behavior_picks_its_physics_from_the_platform() {
        for (platform, expected) in [
            (TargetPlatform::IOS, "BouncingScrollPhysics"),
            (TargetPlatform::MacOS, "BouncingScrollPhysics"),
            (TargetPlatform::Android, "ClampingScrollPhysics"),
            (TargetPlatform::Fuchsia, "ClampingScrollPhysics"),
            (TargetPlatform::Linux, "ClampingScrollPhysics"),
            (TargetPlatform::Windows, "ClampingScrollPhysics"),
        ] {
            let cell = app_on(platform);
            let mut app = cell.borrow_mut();
            let harness = mount(&mut app, SizedBox::shrink().into_widget());
            let context = descendant(&harness, &app, 0);

            let behavior = ScrollConfiguration::of(&mut app, context);
            assert_eq!(behavior.get_platform(&app, context), platform);
            let physics = behavior.get_scroll_physics(&app, context);
            let described = format!("{physics:?}");
            assert!(described.contains(expected), "{platform:?}: {described}");
            assert!(
                described.contains("RangeMaintainingScrollPhysics"),
                "{platform:?}: {described}"
            );
        }
    }

    #[test]
    fn of_falls_back_to_a_plain_behavior_and_reads_the_ambient_one() {
        let cell = app_on(TargetPlatform::Android);
        let mut app = cell.borrow_mut();
        let harness = mount(&mut app, SizedBox::shrink().into_widget());
        let context = descendant(&harness, &app, 0);
        assert_eq!(
            format!("{:?}", ScrollConfiguration::of(&mut app, context)),
            "ScrollBehavior"
        );

        let behavior = ScrollBehaviorRef::new(
            ScrollBehaviorRef::plain()
                .copy_with()
                .platform(TargetPlatform::IOS),
        );
        let harness = mount(
            &mut app,
            ScrollConfiguration::new(behavior, SizedBox::shrink()).into_widget(),
        );
        let context = descendant(&harness, &app, 1);
        let ambient = ScrollConfiguration::of(&mut app, context);
        assert_eq!(ambient.get_platform(&app, context), TargetPlatform::IOS);
        // As in Dart, `_WrappedScrollBehavior.getScrollPhysics` defers to the delegate, which
        // reads its own platform; only `copyWith(physics:)` replaces the physics.
        assert!(
            format!("{:?}", ambient.get_scroll_physics(&app, context))
                .contains("ClampingScrollPhysics")
        );

        let bouncing: ScrollPhysicsRef = Rc::new(BouncingScrollPhysics::new());
        let overridden = ScrollBehaviorRef::new(
            ScrollBehaviorRef::plain()
                .copy_with()
                .physics(bouncing.clone()),
        );
        assert!(Rc::ptr_eq(
            &overridden.get_scroll_physics(&app, context),
            &bouncing
        ));
    }

    #[test]
    fn copy_with_overrides_the_delegate_and_flattens_a_second_copy() {
        let plain = ScrollBehaviorRef::plain();
        let once = plain.copy_with().scrollbars(false);
        assert!(!once.scrollbars);
        assert!(once.overscroll);
        assert_eq!(once.delegate(), &plain);

        let twice = ScrollBehaviorRef::new(once).copy_with().overscroll(false);
        assert!(!twice.scrollbars);
        assert!(!twice.overscroll);
        // The second copy keeps the original delegate rather than nesting a wrapper.
        assert_eq!(twice.delegate().type_id(), plain.type_id());
    }

    #[test]
    fn the_default_behavior_flips_the_axes_for_shift_and_takes_touch_like_devices() {
        let cell = app_on(TargetPlatform::Android);
        let mut app = cell.borrow_mut();
        let harness = mount(&mut app, SizedBox::shrink().into_widget());
        let context = descendant(&harness, &app, 0);
        let behavior = ScrollConfiguration::of(&mut app, context);

        assert_eq!(
            behavior.pointer_axis_modifiers(),
            HashSet::from([
                LogicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_RIGHT
            ])
        );
        assert!(behavior.drag_devices().contains(&PointerDeviceKind::Touch));
        assert!(!behavior.drag_devices().contains(&PointerDeviceKind::Mouse));
        assert_eq!(
            behavior.get_multitouch_drag_strategy(&app, context),
            MultitouchDragStrategy::LatestPointer
        );
    }

    #[test]
    fn the_desktop_platforms_wrap_the_child_in_a_scrollbar() {
        for platform in [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            let cell = app_on(platform);
            let mut app = cell.borrow_mut();
            let harness = mount(&mut app, SizedBox::shrink().into_widget());
            let context = descendant(&harness, &app, 0);
            let behavior = ScrollConfiguration::of(&mut app, context);
            let controller = crate::widgets::scroll_controller::ScrollController::default(&mut app);
            let details = ScrollableDetails::vertical(false).controller(
                crate::widgets::scroll_controller::ScrollControllerLeaf::as_controller(controller),
            );

            let child = SizedBox::shrink().into_widget();
            let scrollbar = behavior.build_scrollbar(&mut app, context, child.clone(), &details);
            assert!(
                crate::framework::downcast_widget::<RawScrollbar>(&*scrollbar).is_some(),
                "{platform:?} decorates the child with a RawScrollbar"
            );
            let indicator =
                behavior.build_overscroll_indicator(&mut app, context, child.clone(), &details);
            assert!(
                Rc::ptr_eq(&indicator, &child),
                "the overscroll indicator waits"
            );
        }
    }

    #[test]
    fn the_decorations_fall_through_to_the_child_on_the_mobile_platforms() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
        ] {
            let cell = app_on(platform);
            let mut app = cell.borrow_mut();
            let harness = mount(&mut app, SizedBox::shrink().into_widget());
            let context = descendant(&harness, &app, 0);
            let behavior = ScrollConfiguration::of(&mut app, context);
            let details = ScrollableDetails::vertical(false);

            let child = SizedBox::shrink().into_widget();
            let scrollbar = behavior.build_scrollbar(&mut app, context, child.clone(), &details);
            assert!(Rc::ptr_eq(&scrollbar, &child), "{platform:?}");
            let indicator =
                behavior.build_overscroll_indicator(&mut app, context, child.clone(), &details);
            assert!(Rc::ptr_eq(&indicator, &child), "{platform:?}");
        }
    }
}

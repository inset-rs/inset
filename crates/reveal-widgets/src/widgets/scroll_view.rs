//! Flutter counterpart: `widgets/scroll_view.dart`.
//!
//! [`ScrollView`] is Dart's abstract stateless widget: a trait over a
//! [`ScrollViewData`] bag whose shared `build` body lives on [`ScrollViewBase`].
//! [`CustomScrollView`], [`BoxScrollView`] and [`ListView`] follow.

use std::fmt::Debug;
use std::rc::Rc;

use reveal_embedder::Clip;
use reveal_foundation::App;
use reveal_gestures::DragStartBehavior;
use reveal_painting::{Axis, AxisDirection, EdgeInsetsGeometry};
use reveal_rendering::{
    AnyViewportOffset, HitTestBehavior, ItemExtentBuilder, ScrollCacheExtent, SliverPaintOrder,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, StatelessKind, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{
    SliverPadding, get_axis_direction_from_axis_reverse_and_directionality,
};
use crate::widgets::focus_manager::{FocusNodeLeaf, UnfocusDisposition, primary_focus};
use crate::widgets::focus_scope::FocusScope;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::primary_scroll_controller::PrimaryScrollController;
use crate::widgets::scroll_configuration::{ScrollBehaviorRef, ScrollConfiguration};
use crate::widgets::scroll_controller::AnyScrollController;
use crate::widgets::scroll_delegate::{
    ChildIndexGetter, NullableIndexedWidgetBuilder, SliverChildBuilderDelegate,
    SliverChildDelegateRef, SliverChildListDelegate,
};
use crate::widgets::scroll_notification::ScrollUpdateNotification;
use crate::widgets::scroll_physics::{AlwaysScrollableScrollPhysics, ScrollPhysicsRef};
use crate::widgets::scrollable::{Scrollable, ViewportBuilder};
use crate::widgets::sliver::{SliverFixedExtentList, SliverList, SliverVariedExtentList};
use crate::widgets::viewport::{ShrinkWrappingViewport, Viewport};

/// A representation of how a [`ScrollView`] should dismiss the on-screen
/// keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollViewKeyboardDismissBehavior {
    /// There is no automatic dismissal of the on-screen keyboard. It is up to the
    /// client to dismiss the keyboard.
    #[default]
    Manual,

    /// The [`ScrollView`] will dismiss an on-screen keyboard when a drag begins.
    OnDrag,
}

/// The fields of Dart's `ScrollView`, which its subclasses carry.
///
/// Dart's `physics` initializer — the one that substitutes an
/// [`AlwaysScrollableScrollPhysics`] for an omitted physics — runs in
/// [`ScrollViewBase::build`] instead, because a fluent setter cannot see the
/// `primary`, `controller` and `scroll_direction` a later setter will supply.
#[derive(Clone, Debug)]
pub struct ScrollViewData {
    pub key: Option<KeyRef>,

    /// The [`Axis`] along which the scroll view's offset increases.
    ///
    /// For the direction in which active scrolling may be occurring, see
    /// `ScrollDirection`.
    ///
    /// Defaults to [`Axis::Vertical`].
    pub scroll_direction: Axis,

    /// Whether the scroll view scrolls in the reading direction.
    ///
    /// For example, if the reading direction is left-to-right and
    /// [`scroll_direction`](Self::scroll_direction) is [`Axis::Horizontal`], then the scroll
    /// view scrolls from left to right when [`reverse`](Self::reverse) is false and from
    /// right to left when it is true.
    ///
    /// Defaults to false.
    pub reverse: bool,

    /// An object that can be used to control the position to which this scroll
    /// view is scrolled.
    ///
    /// Must be `None` if [`primary`](Self::primary) is true.
    pub controller: Option<AnyScrollController>,

    /// Whether this is the primary scroll view associated with the parent
    /// [`PrimaryScrollController`].
    ///
    /// When this is true, the scroll view is scrollable even if it does not have
    /// sufficient content to actually scroll. Otherwise, by default the user can
    /// only scroll the view if it has sufficient content.
    ///
    /// Cannot be true while a controller is provided, only one controller can be
    /// associated with a scroll view.
    ///
    /// Setting to false will explicitly prevent inheriting any
    /// [`PrimaryScrollController`].
    ///
    /// Defaults to `None`. When `None`, and a controller is not provided,
    /// [`PrimaryScrollController::should_inherit`] is used to decide automatic
    /// inheritance.
    pub primary: Option<bool>,

    /// How the scroll view should respond to user input.
    ///
    /// For example, determines how the scroll view continues to animate after the
    /// user stops dragging the scroll view.
    ///
    /// Defaults to matching platform conventions. Furthermore, if
    /// [`primary`](Self::primary) is false, then the user cannot scroll if there is
    /// insufficient content to scroll, while if it is true, they can always attempt to
    /// scroll.
    ///
    /// If an explicit [`ScrollBehaviorRef`] is provided to
    /// [`scroll_behavior`](Self::scroll_behavior), the physics provided by that behavior
    /// will take precedence after this one.
    pub physics: Option<ScrollPhysicsRef>,

    /// A [`ScrollBehaviorRef`] that will be applied to this widget individually.
    ///
    /// Defaults to `None`, wherein the inherited behavior is copied and modified to
    /// alter the viewport decoration.
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// Whether the extent of the scroll view in the
    /// [`scroll_direction`](Self::scroll_direction) should be determined by the contents
    /// being viewed.
    ///
    /// If the scroll view does not shrink wrap, then the scroll view will expand
    /// to the maximum allowed size in the scroll direction. If the scroll view
    /// has unbounded constraints in the scroll direction, then this must be true.
    ///
    /// Shrink wrapping the content of the scroll view is significantly more
    /// expensive than expanding to the maximum allowed size because the content
    /// can expand and contract during scrolling, which means the size of the
    /// scroll view needs to be recomputed whenever the scroll position changes.
    ///
    /// Defaults to false.
    pub shrink_wrap: bool,

    /// The first child in the `GrowthDirection::Forward` growth direction.
    ///
    /// Children after the center will be placed in the [`AxisDirection`] determined
    /// by [`scroll_direction`](Self::scroll_direction) and [`reverse`](Self::reverse)
    /// relative to the center. Children before the center will be placed in the opposite
    /// of the axis direction relative to the center. This makes the center the inflection
    /// point of the growth direction.
    ///
    /// The center must be the key of one of the slivers built by
    /// [`ScrollView::build_slivers`].
    ///
    /// Of the built-in subclasses of [`ScrollView`], only [`CustomScrollView`]
    /// supports a center.
    pub center: Option<KeyRef>,

    /// The relative position of the zero scroll offset.
    ///
    /// For example, if the anchor is 0.5 and the [`AxisDirection`] determined by
    /// [`scroll_direction`](Self::scroll_direction) and [`reverse`](Self::reverse) is
    /// [`AxisDirection::Down`] or [`AxisDirection::Up`], then the zero scroll offset is
    /// vertically centered within the viewport. If the anchor is 1.0, and the axis
    /// direction is [`AxisDirection::Right`], then the zero scroll offset is on the left
    /// edge of the viewport.
    pub anchor: f64,

    /// Deprecated: use [`scroll_cache_extent`](Self::scroll_cache_extent) instead.
    ///
    /// The viewport has an area before and after the visible area to cache items
    /// that are about to become visible when the user scrolls.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub cache_extent: Option<f64>,

    /// The viewport has an area before and after the visible area to cache items
    /// that are about to become visible when the user scrolls.
    pub scroll_cache_extent: Option<ScrollCacheExtent>,

    /// The number of children that will contribute semantic information.
    ///
    /// Some subtypes of [`ScrollView`] can infer this value automatically. For
    /// example [`ListView`] will use the number of widgets in the child list,
    /// while the [`ListView::separated`] constructor will use half that amount.
    ///
    /// For [`CustomScrollView`] and other types which do not receive a builder
    /// or list of widgets, the child count must be explicitly provided. If the
    /// number is unknown or unbounded this should be left unset.
    pub semantic_child_count: Option<i32>,

    /// The order in which the viewport paints its slivers.
    ///
    /// Defaults to [`SliverPaintOrder::FirstIsTop`].
    pub paint_order: SliverPaintOrder,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// How this [`ScrollView`] will dismiss the keyboard automatically.
    ///
    /// If `None` then it will fall back to
    /// [`scroll_behavior`](Self::scroll_behavior). If that is also `None`, the inherited
    /// [`ScrollBehavior::get_keyboard_dismiss_behavior`](crate::ScrollBehavior::get_keyboard_dismiss_behavior)
    /// will be used.
    pub keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,

    /// Restoration ID to save and restore the scroll offset of the scrollable.
    pub restoration_id: Option<String>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,

    /// How to behave during hit testing when deciding how the hit test propagates
    /// to children and whether to consider targets behind this scroll view.
    ///
    /// Defaults to [`HitTestBehavior::Opaque`].
    pub hit_test_behavior: HitTestBehavior,
}

impl ScrollViewData {
    /// Dart's `ScrollView` constructor defaults.
    pub fn new() -> ScrollViewData {
        #[allow(deprecated)]
        ScrollViewData {
            key: None,
            scroll_direction: Axis::Vertical,
            reverse: false,
            controller: None,
            primary: None,
            physics: None,
            scroll_behavior: None,
            shrink_wrap: false,
            center: None,
            anchor: 0.0,
            cache_extent: None,
            scroll_cache_extent: None,
            semantic_child_count: None,
            paint_order: SliverPaintOrder::FirstIsTop,
            drag_start_behavior: DragStartBehavior::Start,
            keyboard_dismiss_behavior: None,
            restoration_id: None,
            clip_behavior: Clip::HardEdge,
            hit_test_behavior: HitTestBehavior::Opaque,
        }
    }
}

impl Default for ScrollViewData {
    fn default() -> ScrollViewData {
        ScrollViewData::new()
    }
}

/// The [`ScrollViewData`] accessors a [`ScrollView`] implementor writes; the bag lives under
/// the field `scroll_view`.
#[macro_export]
macro_rules! scroll_view_accessors {
    () => {
        fn scroll_view_data(&self) -> &$crate::ScrollViewData {
            &self.scroll_view
        }

        fn scroll_view_data_mut(&mut self) -> &mut $crate::ScrollViewData {
            &mut self.scroll_view
        }
    };
}

/// The fluent setters of the `ScrollView` named arguments every subclass takes; the ones
/// only `CustomScrollView` exposes (`scrollBehavior`, `center`, `anchor`, `paintOrder`) are
/// written on it.
#[macro_export]
macro_rules! scroll_view_setters {
    ($name:ident) => {
        /// Dart `key:`.
        pub fn key(mut self, key: $crate::KeyRef) -> $name {
            self.scroll_view.key = Some(key);
            self
        }

        /// Dart `scrollDirection:`.
        pub fn scroll_direction(mut self, scroll_direction: reveal_painting::Axis) -> $name {
            self.scroll_view.scroll_direction = scroll_direction;
            self
        }

        /// Dart `reverse:`.
        pub fn reverse(mut self, reverse: bool) -> $name {
            self.scroll_view.reverse = reverse;
            self
        }

        /// Dart `controller:`.
        pub fn controller(mut self, controller: $crate::AnyScrollController) -> $name {
            debug_assert!(
                !self.scroll_view.primary.unwrap_or(false),
                "Primary ScrollViews obtain their ScrollController via inheritance from a \
                 PrimaryScrollController widget. You cannot both set primary to true and pass \
                 an explicit controller."
            );
            self.scroll_view.controller = Some(controller);
            self
        }

        /// Dart `primary:`.
        pub fn primary(mut self, primary: bool) -> $name {
            debug_assert!(
                !(self.scroll_view.controller.is_some() && primary),
                "Primary ScrollViews obtain their ScrollController via inheritance from a \
                 PrimaryScrollController widget. You cannot both set primary to true and pass \
                 an explicit controller."
            );
            self.scroll_view.primary = Some(primary);
            self
        }

        /// Dart `physics:`.
        pub fn physics(mut self, physics: $crate::ScrollPhysicsRef) -> $name {
            self.scroll_view.physics = Some(physics);
            self
        }

        /// Dart `shrinkWrap:`.
        pub fn shrink_wrap(mut self, shrink_wrap: bool) -> $name {
            debug_assert!(!shrink_wrap || self.scroll_view.center.is_none());
            self.scroll_view.shrink_wrap = shrink_wrap;
            self
        }

        /// Dart `cacheExtent:`.
        #[deprecated(note = "Use scroll_cache_extent instead.")]
        pub fn cache_extent(mut self, cache_extent: f64) -> $name {
            #[allow(deprecated)]
            {
                self.scroll_view.cache_extent = Some(cache_extent);
            }
            self
        }

        /// Dart `scrollCacheExtent:`.
        pub fn scroll_cache_extent(
            mut self,
            scroll_cache_extent: reveal_rendering::ScrollCacheExtent,
        ) -> $name {
            self.scroll_view.scroll_cache_extent = Some(scroll_cache_extent);
            self
        }

        /// Dart `semanticChildCount:`.
        pub fn semantic_child_count(mut self, semantic_child_count: i32) -> $name {
            debug_assert!(semantic_child_count >= 0);
            self.scroll_view.semantic_child_count = Some(semantic_child_count);
            self
        }

        /// Dart `dragStartBehavior:`.
        pub fn drag_start_behavior(
            mut self,
            drag_start_behavior: reveal_gestures::DragStartBehavior,
        ) -> $name {
            self.scroll_view.drag_start_behavior = drag_start_behavior;
            self
        }

        /// Dart `keyboardDismissBehavior:`.
        pub fn keyboard_dismiss_behavior(
            mut self,
            keyboard_dismiss_behavior: $crate::ScrollViewKeyboardDismissBehavior,
        ) -> $name {
            self.scroll_view.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
            self
        }

        /// Dart `restorationId:`.
        pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> $name {
            self.scroll_view.restoration_id = Some(restoration_id.into());
            self
        }

        /// Dart `clipBehavior:`.
        pub fn clip_behavior(mut self, clip_behavior: reveal_embedder::Clip) -> $name {
            self.scroll_view.clip_behavior = clip_behavior;
            self
        }

        /// Dart `hitTestBehavior:`.
        pub fn hit_test_behavior(
            mut self,
            hit_test_behavior: reveal_rendering::HitTestBehavior,
        ) -> $name {
            self.scroll_view.hit_test_behavior = hit_test_behavior;
            self
        }
    };
}

/// A widget that combines a `Scrollable` and a [`Viewport`] to create an
/// interactive scrolling pane of content in one dimension.
///
/// Scrollable widgets consist of three pieces:
///
///  1. A `Scrollable` widget, which listens for various user gestures and
///     implements the interaction design for scrolling.
///  2. A viewport widget, such as [`Viewport`] or [`ShrinkWrappingViewport`], which
///     implements the visual design for scrolling by displaying only a portion
///     of the widgets inside the scroll view.
///  3. One or more slivers, which are widgets that can be composed to create
///     various scrolling effects, such as lists, grids, and expanding headers.
///
/// [`ScrollView`] helps orchestrate these pieces by creating the `Scrollable` and
/// the viewport and deferring to its implementor to create the slivers.
///
/// Dart's abstract `ScrollView` is this trait over a [`ScrollViewData`] bag; the shared
/// `build` body lives on [`ScrollViewBase`], which an implementor's
/// [`StatelessWidget::build`] calls.
///
/// ## Persisting the scroll position during a session
///
/// Scroll views attempt to persist their scroll position using `PageStorage`.
/// This can be disabled by setting
/// [`ScrollControllerLeaf::keep_scroll_offset`](crate::ScrollControllerLeaf::keep_scroll_offset)
/// to false on the controller.
pub trait ScrollView: Clone + Debug + Sized + 'static {
    /// The [`ScrollViewData`] bag ([`scroll_view_accessors!`](crate::scroll_view_accessors)).
    fn scroll_view_data(&self) -> &ScrollViewData;

    /// The [`ScrollViewData`] bag, mutably.
    fn scroll_view_data_mut(&mut self) -> &mut ScrollViewData;

    /// Returns the [`AxisDirection`] in which the scroll view scrolls.
    ///
    /// Combines the [`ScrollViewData::scroll_direction`] with the
    /// [`ScrollViewData::reverse`] boolean to obtain the concrete [`AxisDirection`].
    ///
    /// If the scroll direction is [`Axis::Horizontal`], the ambient `Directionality` is
    /// also considered when selecting the concrete [`AxisDirection`].
    fn get_direction(&self, app: &mut App, context: BuildContext) -> AxisDirection {
        ScrollViewBase::get_direction(self, app, context)
    }

    /// Build the list of widgets to place inside the viewport.
    ///
    /// Implementors override this method to build the slivers for the inside of
    /// the viewport.
    fn build_slivers(&self, app: &mut App, context: BuildContext) -> Vec<WidgetRef>;

    /// Build the viewport.
    ///
    /// Implementors may override this method to change how the viewport is built.
    /// The default implementation uses a [`ShrinkWrappingViewport`] if
    /// [`ScrollViewData::shrink_wrap`] is true, and a regular [`Viewport`] otherwise.
    ///
    /// The `offset` argument is the value obtained from
    /// [`Scrollable::viewport_builder`].
    ///
    /// The `axis_direction` argument is the value obtained from
    /// [`get_direction`](Self::get_direction).
    ///
    /// The `slivers` argument is the value obtained from
    /// [`build_slivers`](Self::build_slivers).
    fn build_viewport(
        &self,
        app: &mut App,
        context: BuildContext,
        offset: AnyViewportOffset,
        axis_direction: AxisDirection,
        slivers: Vec<WidgetRef>,
    ) -> WidgetRef {
        ScrollViewBase::build_viewport(self, app, context, offset, axis_direction, slivers)
    }
}

/// The shared bodies of Dart's `ScrollView`: call one where Dart writes `super.…`.
pub struct ScrollViewBase;

impl ScrollViewBase {
    /// See [`ScrollView::get_direction`].
    pub fn get_direction<V: ScrollView>(
        view: &V,
        app: &mut App,
        context: BuildContext,
    ) -> AxisDirection {
        let data = view.scroll_view_data();
        get_axis_direction_from_axis_reverse_and_directionality(
            app,
            context,
            data.scroll_direction,
            data.reverse,
        )
    }

    /// See [`ScrollView::build_viewport`].
    pub fn build_viewport<V: ScrollView>(
        view: &V,
        app: &mut App,
        context: BuildContext,
        offset: AnyViewportOffset,
        axis_direction: AxisDirection,
        slivers: Vec<WidgetRef>,
    ) -> WidgetRef {
        let _ = (app, context);
        let data = view.scroll_view_data();
        #[allow(deprecated)]
        let effective_scroll_cache_extent = data
            .scroll_cache_extent
            .or(data.cache_extent.map(ScrollCacheExtent::Pixels));
        if data.shrink_wrap {
            let mut viewport = ShrinkWrappingViewport::new(offset)
                .axis_direction(axis_direction)
                .slivers(slivers)
                .paint_order(data.paint_order)
                .clip_behavior(data.clip_behavior);
            if let Some(cache_extent) = effective_scroll_cache_extent {
                viewport = viewport.scroll_cache_extent(cache_extent);
            }
            return viewport.into_widget();
        }
        let mut viewport = Viewport::new(offset)
            .axis_direction(axis_direction)
            .slivers(slivers)
            .anchor(data.anchor)
            .paint_order(data.paint_order)
            .clip_behavior(data.clip_behavior);
        if let Some(cache_extent) = effective_scroll_cache_extent {
            viewport = viewport.scroll_cache_extent(cache_extent);
        }
        if let Some(center) = data.center.clone() {
            viewport = viewport.center(center);
        }
        viewport.into_widget()
    }

    /// Dart's `ScrollView.build`, which an implementor's [`StatelessWidget::build`] calls.
    pub fn build<V: ScrollView>(view: &V, app: &mut App, context: BuildContext) -> WidgetRef {
        let slivers = view.build_slivers(app, context);
        let axis_direction = view.get_direction(app, context);

        let data = view.scroll_view_data().clone();
        let effective_primary = data.primary.unwrap_or_else(|| {
            data.controller.is_none()
                && PrimaryScrollController::should_inherit(app, context, data.scroll_direction)
        });

        let scroll_controller = if effective_primary {
            PrimaryScrollController::maybe_of(app, context)
        } else {
            data.controller
        };

        let this = view.clone();
        let viewport_builder: ViewportBuilder = Rc::new(move |app, context, offset| {
            this.build_viewport(app, context, offset, axis_direction, slivers.clone())
        });

        let mut scrollable = Scrollable::new(viewport_builder)
            .axis_direction(axis_direction)
            .drag_start_behavior(data.drag_start_behavior)
            .hit_test_behavior(data.hit_test_behavior)
            .clip_behavior(data.clip_behavior);
        if let Some(controller) = scroll_controller {
            scrollable = scrollable.controller(controller);
        }
        if let Some(physics) = ScrollViewBase::effective_physics(view) {
            scrollable = scrollable.physics(physics);
        }
        if let Some(scroll_behavior) = data.scroll_behavior.clone() {
            scrollable = scrollable.scroll_behavior(scroll_behavior);
        }
        if let Some(restoration_id) = data.restoration_id.clone() {
            scrollable = scrollable.restoration_id(restoration_id);
        }

        let scrollable_result = if effective_primary && scroll_controller.is_some() {
            // Further descendant scroll views will not inherit the same
            // `PrimaryScrollController`.
            PrimaryScrollController::none(scrollable).into_widget()
        } else {
            scrollable.into_widget()
        };

        let effective_keyboard_dismiss_behavior = data
            .keyboard_dismiss_behavior
            .or_else(|| {
                data.scroll_behavior
                    .as_ref()
                    .map(|behavior| behavior.get_keyboard_dismiss_behavior(app, context))
            })
            .unwrap_or_else(|| {
                ScrollConfiguration::of(app, context).get_keyboard_dismiss_behavior(app, context)
            });

        if effective_keyboard_dismiss_behavior == ScrollViewKeyboardDismissBehavior::OnDrag {
            NotificationListener::<ScrollUpdateNotification>::new(scrollable_result)
                .on_notification(move |app, notification: &ScrollUpdateNotification| {
                    let current_scope = FocusScope::of(app, context, true).as_node();
                    if notification.drag_details.is_some()
                        && !current_scope.has_primary_focus(app)
                        && current_scope.has_focus(app)
                        && let Some(primary) = primary_focus(app)
                    {
                        primary.unfocus(app, UnfocusDisposition::Scope);
                    }
                    false
                })
                .into_widget()
        } else {
            scrollable_result
        }
    }

    /// Dart's `physics` constructor initializer: an omitted physics becomes an
    /// [`AlwaysScrollableScrollPhysics`] for a primary scroll view, and for a vertical one
    /// that neither names a controller nor opts out of inheriting the primary one.
    fn effective_physics<V: ScrollView>(view: &V) -> Option<ScrollPhysicsRef> {
        let data = view.scroll_view_data();
        if let Some(physics) = data.physics.clone() {
            return Some(physics);
        }
        let implied = data.primary.unwrap_or(false)
            || (data.primary.is_none()
                && data.controller.is_none()
                && data.scroll_direction == Axis::Vertical);
        implied.then(|| Rc::new(AlwaysScrollableScrollPhysics::new()) as ScrollPhysicsRef)
    }
}

/// A [`ScrollView`] that combines multiple [`slivers`](Self::slivers) in one scrollable view.
///
/// A [`CustomScrollView`] lets you combine lists, grids, and other widgets in a
/// single scrollable view by supplying slivers directly. Slivers can
/// represent lists, grids, and expanding headers. Widgets that use the box
/// layout model can be included using a `SliverToBoxAdapter`.
///
/// ```ignore
/// CustomScrollView::new().slivers([
///     SliverToBoxAdapter::new().child(header).into_widget(),
///     SliverList::list(items).into_widget(),
/// ])
/// ```
#[derive(Clone, Debug, Default)]
pub struct CustomScrollView {
    scroll_view: ScrollViewData,

    /// The slivers to place inside the viewport.
    ///
    /// A _sliver_ is a widget backed by a `RenderSliver`, i.e. one that implements the
    /// constraint/geometry protocol that uses `SliverConstraints` and `SliverGeometry`.
    /// This is as distinct from those widgets that are backed by `RenderBox`, which use
    /// `BoxConstraints` and `Size` respectively, and are known as box widgets.
    ///
    /// While boxes are much more straightforward (implementing a simple two-dimensional
    /// Cartesian layout system), slivers are much more powerful, and are optimized for
    /// one-axis scrolling environments.
    ///
    /// In general, slivers always wrap box widgets to actually render anything; the sliver
    /// part of the equation is mostly about how these boxes should be laid out in a
    /// viewport.
    pub slivers: Vec<WidgetRef>,
}

impl CustomScrollView {
    /// Creates a [`ScrollView`] that combines multiple slivers in one scrollable
    /// view.
    pub fn new() -> CustomScrollView {
        CustomScrollView {
            scroll_view: ScrollViewData::new(),
            slivers: Vec::new(),
        }
    }

    crate::scroll_view_setters!(CustomScrollView);

    /// Dart `CustomScrollView(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> CustomScrollView {
        self.scroll_view.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `CustomScrollView(center:)`.
    pub fn center(mut self, center: KeyRef) -> CustomScrollView {
        debug_assert!(!self.scroll_view.shrink_wrap);
        self.scroll_view.center = Some(center);
        self
    }

    /// Dart `CustomScrollView(anchor:)`.
    pub fn anchor(mut self, anchor: f64) -> CustomScrollView {
        debug_assert!((0.0..=1.0).contains(&anchor));
        self.scroll_view.anchor = anchor;
        self
    }

    /// Dart `CustomScrollView(paintOrder:)`.
    pub fn paint_order(mut self, paint_order: SliverPaintOrder) -> CustomScrollView {
        self.scroll_view.paint_order = paint_order;
        self
    }

    /// Dart `CustomScrollView(slivers:)`.
    pub fn slivers(mut self, slivers: impl IntoIterator<Item = WidgetRef>) -> CustomScrollView {
        self.slivers = slivers.into_iter().collect();
        self
    }
}

impl ScrollView for CustomScrollView {
    crate::scroll_view_accessors!();

    fn build_slivers(&self, _app: &mut App, _context: BuildContext) -> Vec<WidgetRef> {
        self.slivers.clone()
    }
}

impl StatelessWidget for CustomScrollView {
    fn key(&self) -> Option<&KeyRef> {
        self.scroll_view.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        ScrollViewBase::build(self, app, context)
    }
}

/// The fields Dart's `BoxScrollView` adds to a [`ScrollView`].
#[derive(Clone, Debug, Default)]
pub struct BoxScrollViewData {
    /// The amount of space by which to inset the children.
    pub padding: Option<EdgeInsetsGeometry>,
}

impl BoxScrollViewData {
    /// Dart's `BoxScrollView` constructor defaults.
    pub fn new() -> BoxScrollViewData {
        BoxScrollViewData { padding: None }
    }
}

/// The [`BoxScrollViewData`] accessors a [`BoxScrollView`] implementor writes; the bag lives
/// under the field `box_scroll_view`.
#[macro_export]
macro_rules! box_scroll_view_accessors {
    () => {
        fn box_scroll_view_data(&self) -> &$crate::BoxScrollViewData {
            &self.box_scroll_view
        }

        fn box_scroll_view_data_mut(&mut self) -> &mut $crate::BoxScrollViewData {
            &mut self.box_scroll_view
        }
    };
}

/// A [`ScrollView`] that uses a single child layout model.
///
/// Scroll views are often decorated with scrollbars and overscroll indicators,
/// which are managed by the inherited
/// [`ScrollBehavior`](crate::ScrollBehavior). Placing a [`ScrollConfiguration`] above a
/// scroll view can modify these behaviors for that scroll view.
///
/// The shared `buildSlivers` body lives on [`BoxScrollViewBase`], which an implementor's
/// [`ScrollView::build_slivers`] calls.
pub trait BoxScrollView: ScrollView {
    /// The [`BoxScrollViewData`] bag
    /// ([`box_scroll_view_accessors!`](crate::box_scroll_view_accessors)).
    fn box_scroll_view_data(&self) -> &BoxScrollViewData;

    /// The [`BoxScrollViewData`] bag, mutably.
    fn box_scroll_view_data_mut(&mut self) -> &mut BoxScrollViewData;

    /// Implementors override this method to build the layout model.
    fn build_child_layout(&self, app: &mut App, context: BuildContext) -> WidgetRef;
}

/// The shared bodies of Dart's `BoxScrollView`: call one where Dart writes `super.…`.
pub struct BoxScrollViewBase;

impl BoxScrollViewBase {
    /// Dart's `BoxScrollView.buildSlivers`, which an implementor's
    /// [`ScrollView::build_slivers`] calls.
    pub fn build_slivers<V: BoxScrollView>(
        view: &V,
        app: &mut App,
        context: BuildContext,
    ) -> Vec<WidgetRef> {
        let mut sliver = view.build_child_layout(app, context);
        let padding = view.box_scroll_view_data().padding;
        let mut effective_padding = padding;
        if padding.is_none()
            && let Some(media_query) = MediaQuery::maybe_of(app, context)
        {
            // Automatically pad sliver with padding from `MediaQuery`.
            let media_query_horizontal_padding =
                media_query
                    .padding
                    .copy_with(None, Some(0.0), None, Some(0.0));
            let media_query_vertical_padding =
                media_query
                    .padding
                    .copy_with(Some(0.0), None, Some(0.0), None);
            let vertical = view.scroll_view_data().scroll_direction == Axis::Vertical;
            // Consume the main axis padding with a `SliverPadding`.
            effective_padding = Some(EdgeInsetsGeometry::from(if vertical {
                media_query_vertical_padding
            } else {
                media_query_horizontal_padding
            }));
            // Leave behind the cross axis padding.
            sliver = MediaQuery::new(
                media_query.copy_with().padding(if vertical {
                    media_query_horizontal_padding
                } else {
                    media_query_vertical_padding
                }),
                sliver,
            )
            .into_widget();
        }

        if let Some(effective_padding) = effective_padding {
            sliver = SliverPadding::new(effective_padding)
                .sliver(sliver)
                .into_widget();
        }
        vec![sliver]
    }
}

/// A scrollable list of widgets arranged linearly.
///
/// [`ListView`] is the most commonly used scrolling widget. It displays its
/// children one after another in the scroll direction. In the cross axis, the
/// children are required to fill the [`ListView`].
///
/// If non-`None`, the [`item_extent`](Self::item_extent) forces the children to have the
/// given extent in the scroll direction. Specifying an item extent is more efficient than
/// letting the children determine their own extent because the scrolling machinery can make
/// use of the foreknowledge of the children's extent to save work, for example when the
/// scroll position changes drastically.
///
/// There are four ways to construct a [`ListView`]:
///
///  1. [`ListView::new`] takes an explicit list of children through the
///     [`children`](Self::children) setter. This is appropriate for list views with a small
///     number of children because constructing the list requires doing work for every child
///     that could possibly be displayed in the list view instead of just those children that
///     are actually visible.
///
///  2. [`ListView::builder`] takes an item builder, which builds the children on demand.
///     This is appropriate for list views with a large (or infinite) number of children
///     because the builder is called only for those children that are actually visible.
///
///  3. [`ListView::separated`] takes two builders: `item_builder` builds child items on
///     demand, and `separator_builder` similarly builds separator children which appear in
///     between the child items. This is appropriate for list views with a fixed number of
///     children.
///
///  4. [`ListView::custom`] takes a
///     [`SliverChildDelegateRef`], which provides the ability to customize additional
///     aspects of the child model.
///
/// By default, a [`ListView`] will automatically pad the list's scrollable
/// extremities to avoid partial obstructions indicated by the `MediaQuery`'s
/// padding. To avoid this behavior, override with a zero
/// [`padding`](Self::padding).
///
/// ## Transitioning to [`CustomScrollView`]
///
/// A [`ListView`] is basically a [`CustomScrollView`] with a single [`SliverList`] in
/// its [`CustomScrollView::slivers`] property. The
/// [`padding`](Self::padding) corresponds to having a `SliverPadding` in the slivers
/// instead of the list itself, and having the [`SliverList`] instead be a child of the
/// `SliverPadding`.
pub struct ListView {
    scroll_view: ScrollViewData,
    box_scroll_view: BoxScrollViewData,

    /// If non-`None`, forces the children to have the given extent in the scroll
    /// direction.
    ///
    /// Specifying an item extent is more efficient than letting the children
    /// determine their own extent because the scrolling machinery can make use of
    /// the foreknowledge of the children's extent to save work, for example when
    /// the scroll position changes drastically.
    pub item_extent: Option<f64>,

    /// If non-`None`, forces the children to have the corresponding extent returned
    /// by the builder.
    ///
    /// Specifying an item extent builder is more efficient than letting the
    /// children determine their own extent because the scrolling machinery can
    /// make use of the foreknowledge of the children's extent to save work, for
    /// example when the scroll position changes drastically.
    pub item_extent_builder: Option<ItemExtentBuilder>,

    /// A delegate that provides the children for the [`ListView`].
    ///
    /// The [`SliverList::delegate`] property corresponds to this one.
    pub children_delegate: SliverChildDelegateRef,

    // Dart's default constructor's `children` and `add*` arguments, kept so that whichever
    // of their setters runs last can rebuild `children_delegate` out of all of them. `None`
    // for the constructors that take (or build) a delegate.
    list_arguments: Option<ListArguments>,
}

/// The `SliverChildListDelegate` arguments of Dart's `ListView` default constructor.
#[derive(Clone, Debug)]
struct ListArguments {
    children: Vec<WidgetRef>,
    add_automatic_keep_alives: bool,
    add_repaint_boundaries: bool,
    add_semantic_indexes: bool,
}

impl ListArguments {
    fn new() -> ListArguments {
        ListArguments {
            children: Vec::new(),
            add_automatic_keep_alives: true,
            add_repaint_boundaries: true,
            add_semantic_indexes: true,
        }
    }

    fn into_delegate(self) -> SliverChildDelegateRef {
        SliverChildListDelegate::new(self.children)
            .add_automatic_keep_alives(self.add_automatic_keep_alives)
            .add_repaint_boundaries(self.add_repaint_boundaries)
            .add_semantic_indexes(self.add_semantic_indexes)
            .into_delegate()
    }
}

impl ListView {
    /// Creates a scrollable, linear array of widgets from an explicit list, which
    /// the [`children`](Self::children) setter supplies.
    ///
    /// This constructor is appropriate for list views with a small number of
    /// children because constructing the list requires doing work for every
    /// child that could possibly be displayed in the list view instead of just
    /// those children that are actually visible.
    ///
    /// It is usually more efficient to create children on demand using
    /// [`ListView::builder`].
    pub fn new() -> ListView {
        let list_arguments = ListArguments::new();
        ListView {
            scroll_view: ScrollViewData {
                semantic_child_count: Some(0),
                ..ScrollViewData::new()
            },
            box_scroll_view: BoxScrollViewData::new(),
            item_extent: None,
            item_extent_builder: None,
            children_delegate: list_arguments.clone().into_delegate(),
            list_arguments: Some(list_arguments),
        }
    }

    /// Creates a scrollable, linear array of widgets that are created on demand.
    ///
    /// This constructor is appropriate for list views with a large (or infinite)
    /// number of children because the builder is called only for those children
    /// that are actually visible.
    ///
    /// The item builder will be called only with indices greater than or equal to
    /// zero and less than [`item_count`](ListViewBuilder::item_count).
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
    ) -> ListViewBuilder {
        ListViewBuilder::new(Rc::new(item_builder))
    }

    /// Creates a fixed-length scrollable linear array of list "items" separated
    /// by list item "separators".
    ///
    /// This constructor is appropriate for list views with a large number of
    /// item and separator children because the builders are only called for
    /// the children that are actually visible.
    ///
    /// The `separator_builder` is similar to `item_builder`, except it is the widget that
    /// gets placed between `item_builder(context, index)` and
    /// `item_builder(context, index + 1)`.
    pub fn separated(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        separator_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        item_count: i32,
    ) -> ListViewSeparated {
        ListViewSeparated::new(
            Rc::new(item_builder),
            Rc::new(separator_builder),
            item_count,
        )
    }

    /// Creates a scrollable, linear array of widgets with a custom child model.
    ///
    /// For example, a custom child model can control the algorithm used to
    /// estimate the size of children that are not actually visible.
    pub fn custom(children_delegate: SliverChildDelegateRef) -> ListView {
        ListView {
            scroll_view: ScrollViewData::new(),
            box_scroll_view: BoxScrollViewData::new(),
            item_extent: None,
            item_extent_builder: None,
            children_delegate,
            list_arguments: None,
        }
    }

    crate::scroll_view_setters!(ListView);

    /// Dart `ListView(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> ListView {
        self.box_scroll_view.padding = Some(padding);
        self
    }

    /// Dart `ListView(itemExtent:)`.
    pub fn item_extent(mut self, item_extent: f64) -> ListView {
        debug_assert!(
            self.item_extent_builder.is_none(),
            "You can only pass one of itemExtent and itemExtentBuilder."
        );
        self.item_extent = Some(item_extent);
        self
    }

    /// Dart `ListView(itemExtentBuilder:)`.
    pub fn item_extent_builder(mut self, item_extent_builder: ItemExtentBuilder) -> ListView {
        debug_assert!(
            self.item_extent.is_none(),
            "You can only pass one of itemExtent and itemExtentBuilder."
        );
        self.item_extent_builder = Some(item_extent_builder);
        self
    }

    /// Dart `ListView(children:)`.
    ///
    /// The children also fill in [`ScrollViewData::semantic_child_count`] while it still
    /// holds the constructor's default, as Dart's `semanticChildCount ?? children.length`
    /// does.
    pub fn children(self, children: impl IntoIterator<Item = WidgetRef>) -> ListView {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        let count = children.len() as i32;
        let mut this = self.with_list_arguments(|arguments| arguments.children = children);
        if this.scroll_view.semantic_child_count == Some(0) {
            this.scroll_view.semantic_child_count = Some(count);
        }
        this
    }

    /// Dart `ListView(addAutomaticKeepAlives:)` on the default constructor.
    pub fn add_automatic_keep_alives(self, value: bool) -> ListView {
        self.with_list_arguments(|arguments| arguments.add_automatic_keep_alives = value)
    }

    /// Dart `ListView(addRepaintBoundaries:)` on the default constructor.
    pub fn add_repaint_boundaries(self, value: bool) -> ListView {
        self.with_list_arguments(|arguments| arguments.add_repaint_boundaries = value)
    }

    /// Dart `ListView(addSemanticIndexes:)` on the default constructor.
    pub fn add_semantic_indexes(self, value: bool) -> ListView {
        self.with_list_arguments(|arguments| arguments.add_semantic_indexes = value)
    }

    /// Applies one of the default constructor's delegate arguments and rebuilds
    /// [`children_delegate`](Self::children_delegate) out of all of them.
    fn with_list_arguments(mut self, apply: impl FnOnce(&mut ListArguments)) -> ListView {
        let mut arguments = self
            .list_arguments
            .expect("the children and add* setters belong to ListView::new");
        apply(&mut arguments);
        self.children_delegate = arguments.clone().into_delegate();
        self.list_arguments = Some(arguments);
        self
    }
}

impl Default for ListView {
    fn default() -> ListView {
        ListView::new()
    }
}

impl ScrollView for ListView {
    crate::scroll_view_accessors!();

    fn build_slivers(&self, app: &mut App, context: BuildContext) -> Vec<WidgetRef> {
        BoxScrollViewBase::build_slivers(self, app, context)
    }
}

impl BoxScrollView for ListView {
    crate::box_scroll_view_accessors!();

    fn build_child_layout(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if let Some(item_extent) = self.item_extent {
            return SliverFixedExtentList::new(self.children_delegate.clone(), item_extent)
                .into_widget();
        }
        if let Some(item_extent_builder) = self.item_extent_builder.clone() {
            return SliverVariedExtentList::new(
                self.children_delegate.clone(),
                item_extent_builder,
            )
            .into_widget();
        }
        SliverList::new(self.children_delegate.clone()).into_widget()
    }
}

impl StatelessWidget for ListView {
    fn key(&self) -> Option<&KeyRef> {
        self.scroll_view.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        ScrollViewBase::build(self, app, context)
    }
}

/// Dart's `ListView.builder(..)` arguments; the value converts to the [`ListView`].
pub struct ListViewBuilder {
    list_view: ListView,
    delegate: SliverChildBuilderDelegate,
}

impl ListViewBuilder {
    fn new(item_builder: NullableIndexedWidgetBuilder) -> ListViewBuilder {
        ListViewBuilder {
            list_view: ListView::custom(SliverChildListDelegate::new(Vec::new()).into_delegate()),
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
        }
    }

    /// Dart `itemCount:`, which also fills in
    /// [`ScrollViewData::semantic_child_count`], as Dart's
    /// `semanticChildCount ?? itemCount` does.
    pub fn item_count(mut self, item_count: i32) -> ListViewBuilder {
        debug_assert!(item_count >= 0);
        self.delegate = self.delegate.child_count(item_count);
        if self.list_view.scroll_view.semantic_child_count.is_none() {
            self.list_view.scroll_view.semantic_child_count = Some(item_count);
        }
        self
    }

    /// Dart `findChildIndexCallback:`.
    pub fn find_child_index_callback(
        mut self,
        find_child_index_callback: ChildIndexGetter,
    ) -> ListViewBuilder {
        self.delegate = self
            .delegate
            .find_child_index_callback(find_child_index_callback);
        self
    }

    /// Dart `addAutomaticKeepAlives:`.
    pub fn add_automatic_keep_alives(mut self, value: bool) -> ListViewBuilder {
        self.delegate = self.delegate.add_automatic_keep_alives(value);
        self
    }

    /// Dart `addRepaintBoundaries:`.
    pub fn add_repaint_boundaries(mut self, value: bool) -> ListViewBuilder {
        self.delegate = self.delegate.add_repaint_boundaries(value);
        self
    }

    /// Dart `addSemanticIndexes:`.
    pub fn add_semantic_indexes(mut self, value: bool) -> ListViewBuilder {
        self.delegate = self.delegate.add_semantic_indexes(value);
        self
    }

    /// The [`ListView`] this builder describes.
    pub fn build(self) -> ListView {
        ListView {
            children_delegate: self.delegate.into_delegate(),
            ..self.list_view
        }
    }
}

/// Dart's `ListView.separated(..)` arguments; the value converts to the [`ListView`].
pub struct ListViewSeparated {
    list_view: ListView,
    delegate: SliverChildBuilderDelegate,
}

impl ListViewSeparated {
    fn new(
        item_builder: NullableIndexedWidgetBuilder,
        separator_builder: NullableIndexedWidgetBuilder,
        item_count: i32,
    ) -> ListViewSeparated {
        debug_assert!(item_count >= 0);
        let delegate = SliverChildBuilderDelegate::from_builder(Rc::new(
            move |app: &mut App, context, index: i32| {
                let item_index = index.div_euclid(2);
                if index % 2 == 0 {
                    item_builder(app, context, item_index)
                } else {
                    separator_builder(app, context, item_index)
                }
            },
        ))
        .child_count(compute_actual_child_count(item_count))
        .semantic_index_callback(Rc::new(|_widget, index| {
            (index % 2 == 0).then_some(index / 2)
        }));
        ListViewSeparated {
            list_view: ListView {
                scroll_view: ScrollViewData {
                    semantic_child_count: Some(item_count),
                    ..ScrollViewData::new()
                },
                ..ListView::custom(SliverChildListDelegate::new(Vec::new()).into_delegate())
            },
            delegate,
        }
    }

    /// Dart `findItemIndexCallback:`, which reports item indices (separators excluded).
    pub fn find_item_index_callback(
        mut self,
        find_item_index_callback: ChildIndexGetter,
    ) -> ListViewSeparated {
        self.delegate = self.delegate.find_child_index_callback(Rc::new(move |key| {
            find_item_index_callback(key).map(|item_index| item_index * 2)
        }));
        self
    }

    /// Dart `addAutomaticKeepAlives:`.
    pub fn add_automatic_keep_alives(mut self, value: bool) -> ListViewSeparated {
        self.delegate = self.delegate.add_automatic_keep_alives(value);
        self
    }

    /// Dart `addRepaintBoundaries:`.
    pub fn add_repaint_boundaries(mut self, value: bool) -> ListViewSeparated {
        self.delegate = self.delegate.add_repaint_boundaries(value);
        self
    }

    /// Dart `addSemanticIndexes:`.
    pub fn add_semantic_indexes(mut self, value: bool) -> ListViewSeparated {
        self.delegate = self.delegate.add_semantic_indexes(value);
        self
    }

    /// The [`ListView`] this value describes.
    pub fn build(self) -> ListView {
        ListView {
            children_delegate: self.delegate.into_delegate(),
            ..self.list_view
        }
    }
}

impl IntoWidget<StatelessKind> for ListViewBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

impl IntoWidget<StatelessKind> for ListViewSeparated {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Helper method to compute the actual child count for the separated constructor.
fn compute_actual_child_count(item_count: i32) -> i32 {
    (item_count * 2 - 1).max(0)
}

impl Clone for ListView {
    fn clone(&self) -> ListView {
        ListView {
            scroll_view: self.scroll_view.clone(),
            box_scroll_view: self.box_scroll_view.clone(),
            item_extent: self.item_extent,
            item_extent_builder: self.item_extent_builder.clone(),
            children_delegate: self.children_delegate.clone(),
            list_arguments: self.list_arguments.clone(),
        }
    }
}

impl Debug for ListView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListView")
            .field("itemExtent", &self.item_extent)
            .field("childrenDelegate", &self.children_delegate)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use reveal_embedder::{Size, TextDirection};
    use reveal_painting::EdgeInsets;
    use reveal_rendering::{AnyRenderObject, RenderSliverPadding};

    use super::*;
    use crate::test_harness::{Harness, VIEW_HEIGHT, binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::{Directionality, SizedBox};
    use crate::widgets::media_query::MediaQueryData;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};

    const ITEM_EXTENT: f64 = 50.0;
    const ITEM_COUNT: usize = 40;

    /// The tree every scroll view test mounts: a text direction and a media query, which a
    /// `Scrollable` reads for its gesture settings.
    fn wrap<K>(child: impl IntoWidget<K>) -> WidgetRef {
        wrap_with(
            MediaQueryData::new().size(Size::new(300.0, VIEW_HEIGHT)),
            child,
        )
    }

    fn wrap_with<K>(data: MediaQueryData, child: impl IntoWidget<K>) -> WidgetRef {
        let query: WidgetRef = MediaQuery::new(data, child).into_widget();
        Directionality::new(TextDirection::Ltr, query).into_widget()
    }

    fn boxes(count: usize) -> Vec<WidgetRef> {
        (0..count)
            .map(|_| SizedBox::new().height(ITEM_EXTENT).into_widget())
            .collect()
    }

    /// Counts the render objects under `object` for which `matches` holds.
    fn count_render_objects(
        app: &App,
        object: AnyRenderObject,
        matches: &dyn Fn(&App, AnyRenderObject) -> bool,
    ) -> usize {
        let mut total = usize::from(matches(app, object));
        object.visit_children(app, &mut |child| {
            total += count_render_objects(app, child, matches);
        });
        total
    }

    fn item_count(app: &App, harness: &Harness) -> usize {
        count_render_objects(app, harness.render_root(app).as_object(), &|app, object| {
            object
                .as_box()
                .is_some_and(|render_box| render_box.size(app).height() == ITEM_EXTENT)
        })
    }

    #[test]
    fn a_list_view_lays_out_only_the_visible_children_and_scrolls_through_its_controller() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        let list = ListView::new()
            .controller(controller.as_controller())
            .children(boxes(ITEM_COUNT));
        drop(app);
        binding_mount(&cell, wrap(list));
        let mut app = cell.borrow_mut();

        let visible = (VIEW_HEIGHT / ITEM_EXTENT) as usize;
        let laid_out = item_count_under_view(&mut app);
        assert!(
            laid_out >= visible && laid_out < ITEM_COUNT,
            "only the visible children (plus the cache) are laid out, not all of them: {laid_out}"
        );

        controller.jump_to(&mut app, 500.0);
        binding_pump(&mut app, std::time::Duration::ZERO);
        assert_eq!(controller.offset(&app), 500.0);
        let scrolled = item_count_under_view(&mut app);
        assert!(
            scrolled >= visible && scrolled < ITEM_COUNT,
            "still only the visible children (plus the cache) after scrolling: {scrolled}"
        );
    }

    fn item_count_under_view(app: &mut App) -> usize {
        let root = crate::binding::WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted root element")
            .find_render_object(app)
            .expect("a mounted view has a render object");
        count_render_objects(app, root, &|app, object| {
            object.as_box().is_some_and(|render_box| {
                render_box.has_size(app) && render_box.size(app).height() == ITEM_EXTENT
            })
        })
    }

    #[test]
    fn a_list_view_builder_creates_its_children_lazily() {
        let built: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
        let recorded = built.clone();
        let list = ListView::builder(move |_app, _context, index| {
            recorded.borrow_mut().push(index);
            Some(SizedBox::new().height(ITEM_EXTENT).into_widget())
        })
        .item_count(1000)
        .build();

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, wrap(list));
        harness.pump(&mut app);

        let built = built.borrow();
        assert!(!built.is_empty(), "the visible children are built");
        assert!(
            built.len() < 1000,
            "only the visible children are built, not all 1000: {}",
            built.len()
        );
        assert!(
            built.iter().all(|index| *index < built.len() as i32),
            "the builder is called for the leading indices only: {built:?}"
        );
    }

    #[test]
    fn a_list_view_separated_interleaves_items_and_separators() {
        let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let items = calls.clone();
        let separators = calls.clone();
        let list = ListView::separated(
            move |_app, _context, index| {
                items.borrow_mut().push(format!("item {index}"));
                Some(SizedBox::new().height(ITEM_EXTENT).into_widget())
            },
            move |_app, _context, index| {
                separators.borrow_mut().push(format!("separator {index}"));
                Some(SizedBox::new().height(ITEM_EXTENT).into_widget())
            },
            10,
        )
        .build();
        assert_eq!(list.scroll_view.semantic_child_count, Some(10));

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, wrap(list));
        harness.pump(&mut app);

        let calls = calls.borrow();
        assert_eq!(calls[0], "item 0");
        assert_eq!(calls[1], "separator 0");
        assert_eq!(calls[2], "item 1");
        assert_eq!(calls[3], "separator 1");
    }

    #[test]
    fn a_custom_scroll_view_lays_out_a_box_adapter_above_a_sliver_list() {
        use crate::widgets::basic::SliverToBoxAdapter;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let view = CustomScrollView::new().slivers([
            SliverToBoxAdapter::new()
                .child(SizedBox::new().height(120.0))
                .into_widget(),
            SliverList::list(boxes(10)).into_widget(),
        ]);
        let harness = Harness::mount(&mut app, wrap(view));
        harness.pump(&mut app);

        let header = count_render_objects(
            &app,
            harness.render_root(&app).as_object(),
            &|app, object| {
                object
                    .as_box()
                    .is_some_and(|render_box| render_box.size(app).height() == 120.0)
            },
        );
        assert_eq!(header, 1, "the box adapter's child is laid out");
        assert!(
            item_count(&app, &harness) > 0,
            "the sliver list's children are laid out below it"
        );
    }

    #[test]
    fn a_box_scroll_view_consumes_the_media_query_padding_along_the_scroll_axis() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let padding = EdgeInsets::from_ltrb(8.0, 16.0, 24.0, 32.0);
        let list = ListView::new().children(boxes(10));
        let harness = Harness::mount(
            &mut app,
            wrap_with(
                MediaQueryData::new()
                    .size(Size::new(300.0, VIEW_HEIGHT))
                    .padding(padding),
                list,
            ),
        );
        harness.pump(&mut app);

        let sliver_padding = find_sliver_padding(&app, harness.render_root(&app).as_object())
            .expect("the list wraps its sliver in a SliverPadding");
        assert_eq!(
            sliver_padding.padding(&app).resolve(None),
            EdgeInsets::from_ltrb(0.0, 16.0, 0.0, 32.0),
            "the main axis padding is consumed by the SliverPadding"
        );
    }

    fn find_sliver_padding(
        app: &App,
        object: AnyRenderObject,
    ) -> Option<reveal_rendering::RenderHandle<RenderSliverPadding>> {
        if let Some(padding) = object.downcast::<RenderSliverPadding>(app) {
            return Some(padding);
        }
        let mut found = None;
        object.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find_sliver_padding(app, child);
            }
        });
        found
    }
}

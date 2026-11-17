//! Flutter counterpart: `widgets/reorderable_list.dart`.
//!
//! A [`SliverReorderableList`] keeps the state of every item it has built under its index;
//! an item drag replaces the item by a gap of its extent, shows the item in the [`Overlay`]
//! as a proxy that follows the pointer, and slides the items between the gap and the item's
//! old place by the gap's extent. The map is the order Dart's `LinkedHashMap` keeps, which
//! the search for the insertion index depends on.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatusListener,
    Curves,
};
use waft_embedder::{Clip, Offset, Rect, Size, TextDirection};
use waft_foundation::{
    App, Handle, HandleId, Key, Listenable, Listener, OptionRetained, Ref, VoidCallback,
};
use waft_gestures::{
    DelayedMultiDragGestureRecognizer, Drag, DragEndDetails, DragObject, DragStartBehavior,
    DragUpdateDetails, GestureMultiDragStartCallback, ImmediateMultiDragGestureRecognizer,
    MultiDragGestureRecognizer, PointerDownEvent, RecognizerRef,
};
use waft_painting::{
    AlignmentGeometry, Axis, EdgeInsetsGeometry, axis_direction_is_reversed, axis_direction_to_axis,
};
use waft_rendering::{BoxConstraints, CustomSemanticsAction, ItemExtentBuilder, ScrollCacheExtent};
use waft_scheduler::{Ticker, TickerCallback, TickerProvider, TickerProviderObject};

use crate::framework::{
    BuildContext, GlobalKey, IntoWidget, KeyRef, State, StateData, StatefulWidget, StatelessWidget,
    WidgetRef,
};
use crate::widgets::basic::{
    Directionality, IndexedWidgetBuilder, Listener as PointerListener, OverflowBox, Positioned,
    Semantics, SizedBox, SliverPadding, Transform,
};
use crate::widgets::debug::debug_check_has_overlay;
use crate::widgets::drag_boundary::{DragBoundary, DragBoundaryDelegate};
use crate::widgets::inherited_theme::{CapturedThemes, InheritedTheme};
use crate::widgets::localizations::WidgetsLocalizations;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::overlay::{Overlay, OverlayEntry};
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_controller::ScrollControllerRef;
use crate::widgets::scroll_delegate::{ChildIndexGetter, SliverChildBuilderDelegate};
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_view::{CustomScrollView, ScrollViewKeyboardDismissBehavior};
use crate::widgets::scrollable::{Scrollable, ScrollableState};
use crate::widgets::scrollable_helpers::EdgeDraggingAutoScroller;
use crate::widgets::sliver::{SliverFixedExtentList, SliverList, SliverVariedExtentList};
use crate::widgets::sliver_prototype_extent_list::SliverPrototypeExtentList;
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};
use crate::widgets::transitions::AnimatedBuilder;

/// A callback used by [`ReorderableList`] to report that a list item has moved to a new
/// position in the list.
///
/// Implementations should remove the corresponding list item at `old_index` and reinsert it
/// at `new_index`.
///
/// See also:
///
///  * [`ReorderableList`], a widget list that allows the user to reorder its items.
///  * [`SliverReorderableList`], a sliver list that allows the user to reorder its items.
///  * `ReorderableListView`, a Material Design list that allows the user to reorder its items.
pub type ReorderCallback = Rc<dyn Fn(&mut App, i32, i32)>;

/// Signature for the builder callback used to decorate the dragging item in
/// [`ReorderableList`] and [`SliverReorderableList`].
///
/// The `child` will be the item that is being dragged, and `index` is the position of the
/// item in the list.
///
/// The `animation` will be driven forward from 0.0 to 1.0 while the item is being picked up
/// during a drag operation, and reversed from 1.0 to 0.0 when the item is dropped. This can be
/// used to animate properties of the proxy like an elevation or border.
///
/// The returned value will typically be the `child` wrapped in other widgets.
pub type ReorderItemProxyDecorator =
    Rc<dyn Fn(&mut App, WidgetRef, i32, AnimationRef<f64>) -> WidgetRef>;

/// Used to provide drag boundaries during drag-and-drop reordering.
///
/// See also:
/// * [`DragBoundary`], a widget that provides drag boundaries.
pub type ReorderDragBoundaryProvider =
    Rc<dyn Fn(&mut App, BuildContext) -> Option<Box<dyn DragBoundaryDelegate<Rect>>>>;

/// Dart's `void Function(int index)` of `onReorderStart` and `onReorderEnd`.
pub type ReorderIndexCallback = Rc<dyn Fn(&mut App, i32)>;

/// A scrolling container that allows the user to interactively reorder the list items.
///
/// This widget is similar to one created by `ListView.builder`, and uses an
/// [`IndexedWidgetBuilder`] to create each item.
///
/// It is up to the application to wrap each child (or an internal part of the child such as a
/// drag handle) with a drag listener that will recognize the start of an item drag and then
/// start the reorder by calling [`ReorderableListState::start_item_drag_reorder`]. This is
/// most easily achieved by wrapping each child in a [`ReorderableDragStartListener`] or a
/// [`ReorderableDelayedDragStartListener`]. These will take care of recognizing the start of a
/// drag gesture and call the list state's
/// [`ReorderableListState::start_item_drag_reorder`] method.
///
/// This widget's [`ReorderableListState`] can be used to manually start an item reorder, or
/// cancel a current drag. To refer to the [`ReorderableListState`] either provide a
/// [`GlobalKey`] or use the static [`ReorderableList::of`] method from an item's build method.
///
/// See also:
///
///  * [`SliverReorderableList`], a sliver list that allows the user to reorder its items.
///  * `ReorderableListView`, a Material Design list that allows the user to reorder its items.
pub struct ReorderableList {
    pub key: Option<KeyRef>,

    /// Called, as needed, to build list item widgets.
    ///
    /// List items are only built when they're scrolled into view.
    ///
    /// The [`IndexedWidgetBuilder`] index parameter indicates the item's position in the list.
    /// The value of the index parameter will be between zero and one less than
    /// [`item_count`](Self::item_count). All items in the list must have a unique [`Key`],
    /// and should have some kind of listener to start the drag (usually a
    /// [`ReorderableDragStartListener`] or [`ReorderableDelayedDragStartListener`]).
    pub item_builder: IndexedWidgetBuilder,

    /// The number of items in the list.
    ///
    /// It must be a non-negative integer. When zero, nothing is displayed and the widget
    /// occupies no space.
    pub item_count: i32,

    /// A callback used by the list to report that a list item has been dragged to a new
    /// location in the list and the application should update the order of the items.
    ///
    /// If `old_index` is before `new_index`, removing the item at `old_index` from the list
    /// will reduce the list's length by one. Implementations will need to account for this
    /// when inserting before `new_index`.
    ///
    /// Deprecated in favor of [`on_reorder_item`](Self::on_reorder_item), which simplifies
    /// the reordering logic by automatically handling index adjustments. To migrate, remove the
    /// manual adjustment of `new_index` when items are moved downward in the list.
    pub on_reorder: Option<ReorderCallback>,

    /// A callback used by the list to report that a list item has been dragged to a new
    /// location in the list and the application should update the order of the items.
    pub on_reorder_item: Option<ReorderCallback>,

    /// A callback that is called when an item drag has started.
    ///
    /// The index parameter of the callback is the index of the selected item.
    ///
    /// See also:
    ///
    ///   * [`on_reorder_end`](Self::on_reorder_end), which is a called when the dragged item
    ///     is dropped.
    ///   * [`on_reorder_item`](Self::on_reorder_item), which reports that a list item has been
    ///     dragged to a new location.
    pub on_reorder_start: Option<ReorderIndexCallback>,

    /// A callback that is called when the dragged item is dropped.
    ///
    /// The index parameter of the callback is the index where the item is dropped. Unlike
    /// [`on_reorder_item`](Self::on_reorder_item), this is called even when the list item is
    /// dropped in the same location.
    ///
    /// See also:
    ///
    ///   * [`on_reorder_start`](Self::on_reorder_start), which is a called when an item drag
    ///     has started.
    ///   * [`on_reorder_item`](Self::on_reorder_item), which reports that a list item has been
    ///     dragged to a new location.
    pub on_reorder_end: Option<ReorderIndexCallback>,

    /// A callback that allows the app to add an animated decoration around an item when it is
    /// being dragged.
    pub proxy_decorator: Option<ReorderItemProxyDecorator>,

    /// The amount of space by which to inset the list contents.
    ///
    /// It defaults to `EdgeInsets.all(0)`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The [`Axis`] along which the scroll view's offset increases.
    pub scroll_direction: Axis,

    /// Whether the scroll view scrolls in the reading direction.
    pub reverse: bool,

    /// An object that can be used to control the position to which this scroll view is
    /// scrolled.
    pub controller: Option<ScrollControllerRef>,

    /// Whether this is the primary scroll view associated with the parent
    /// `PrimaryScrollController`.
    pub primary: Option<bool>,

    /// How the scroll view should respond to user input.
    pub physics: Option<ScrollPhysicsRef>,

    /// Whether the extent of the scroll view in the scroll direction should be determined by
    /// the contents being viewed.
    pub shrink_wrap: bool,

    /// The relative position of the zero scroll offset.
    pub anchor: f64,

    /// The viewport has an area before and after the visible area to cache items that are
    /// about to become visible when the user scrolls.
    ///
    /// Deprecated: use [`scroll_cache_extent`](Self::scroll_cache_extent) instead.
    pub cache_extent: Option<f64>,

    /// The amount of additional content to display and lay out around the viewport.
    pub scroll_cache_extent: Option<ScrollCacheExtent>,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// The [`ScrollViewKeyboardDismissBehavior`] defines how this scroll view will dismiss
    /// the keyboard automatically.
    ///
    /// If it is `None` then it will fallback to the inherited
    /// `ScrollBehavior::get_keyboard_dismiss_behavior`.
    pub keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,

    /// Restoration ID to save and restore the scroll offset of the scrollable.
    pub restoration_id: Option<String>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,

    /// If non-`None`, forces the children to have the given extent in the scroll direction.
    pub item_extent: Option<f64>,

    /// Called to get the item extent of the children in the scroll direction.
    pub item_extent_builder: Option<ItemExtentBuilder>,

    /// If non-`None`, forces the children to have the same extent as the given widget in the
    /// scroll direction.
    pub prototype_item: Option<WidgetRef>,

    /// The velocity scalar per pixel over scroll of the auto scroller.
    ///
    /// Defaults to 50 if not set or set to `None`.
    pub auto_scroller_velocity_scalar: Option<f64>,

    /// A callback used to provide drag boundaries during drag-and-drop reordering.
    ///
    /// If `None`, the delegate returned by [`DragBoundary::for_rect_maybe_of`] will be used.
    /// Defaults to `None`.
    pub drag_boundary_provider: Option<ReorderDragBoundaryProvider>,
}

impl ReorderableList {
    /// Creates a scrolling container that allows the user to interactively reorder the list
    /// items.
    ///
    /// The `item_count` must be greater than or equal to zero.
    pub fn new(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> WidgetRef + 'static,
        item_count: i32,
    ) -> ReorderableList {
        debug_assert!(item_count >= 0);
        ReorderableList {
            key: None,
            item_builder: Rc::new(item_builder),
            item_count,
            on_reorder: None,
            on_reorder_item: None,
            on_reorder_start: None,
            on_reorder_end: None,
            proxy_decorator: None,
            padding: None,
            scroll_direction: Axis::Vertical,
            reverse: false,
            controller: None,
            primary: None,
            physics: None,
            shrink_wrap: false,
            anchor: 0.0,
            cache_extent: None,
            scroll_cache_extent: None,
            drag_start_behavior: DragStartBehavior::Start,
            keyboard_dismiss_behavior: None,
            restoration_id: None,
            clip_behavior: Clip::HardEdge,
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            auto_scroller_velocity_scalar: None,
            drag_boundary_provider: None,
        }
    }

    /// Dart `ReorderableList(key:)`.
    pub fn key(mut self, key: KeyRef) -> ReorderableList {
        self.key = Some(key);
        self
    }

    /// Dart `ReorderableList(onReorder:)`.
    #[deprecated(
        note = "Use the on_reorder_item callback instead. The on_reorder_item callback adjusts \
                the new_index parameter for a removed item at the old_index."
    )]
    pub fn on_reorder(
        mut self,
        on_reorder: impl Fn(&mut App, i32, i32) + 'static,
    ) -> ReorderableList {
        self.on_reorder = Some(Rc::new(on_reorder));
        self
    }

    /// Dart `ReorderableList(onReorderItem:)`.
    pub fn on_reorder_item(
        mut self,
        on_reorder_item: impl Fn(&mut App, i32, i32) + 'static,
    ) -> ReorderableList {
        self.on_reorder_item = Some(Rc::new(on_reorder_item));
        self
    }

    /// Dart `ReorderableList(onReorderStart:)`.
    pub fn on_reorder_start(
        mut self,
        on_reorder_start: impl Fn(&mut App, i32) + 'static,
    ) -> ReorderableList {
        self.on_reorder_start = Some(Rc::new(on_reorder_start));
        self
    }

    /// Dart `ReorderableList(onReorderEnd:)`.
    pub fn on_reorder_end(
        mut self,
        on_reorder_end: impl Fn(&mut App, i32) + 'static,
    ) -> ReorderableList {
        self.on_reorder_end = Some(Rc::new(on_reorder_end));
        self
    }

    /// Dart `ReorderableList(itemExtent:)`.
    pub fn item_extent(mut self, item_extent: f64) -> ReorderableList {
        self.item_extent = Some(item_extent);
        self
    }

    /// Dart `ReorderableList(itemExtentBuilder:)`.
    pub fn item_extent_builder(
        mut self,
        item_extent_builder: ItemExtentBuilder,
    ) -> ReorderableList {
        self.item_extent_builder = Some(item_extent_builder);
        self
    }

    /// Dart `ReorderableList(prototypeItem:)`.
    pub fn prototype_item<K>(mut self, prototype_item: impl IntoWidget<K>) -> ReorderableList {
        self.prototype_item = Some(prototype_item.into_widget());
        self
    }

    /// Dart `ReorderableList(proxyDecorator:)`.
    pub fn proxy_decorator(
        mut self,
        proxy_decorator: impl Fn(&mut App, WidgetRef, i32, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> ReorderableList {
        self.proxy_decorator = Some(Rc::new(proxy_decorator));
        self
    }

    /// Dart `ReorderableList(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> ReorderableList {
        self.padding = Some(padding);
        self
    }

    /// Dart `ReorderableList(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> ReorderableList {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Dart `ReorderableList(reverse:)`.
    pub fn reverse(mut self, reverse: bool) -> ReorderableList {
        self.reverse = reverse;
        self
    }

    /// Dart `ReorderableList(controller:)`.
    pub fn controller(mut self, controller: ScrollControllerRef) -> ReorderableList {
        self.controller = Some(controller);
        self
    }

    /// Dart `ReorderableList(primary:)`.
    pub fn primary(mut self, primary: bool) -> ReorderableList {
        self.primary = Some(primary);
        self
    }

    /// Dart `ReorderableList(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> ReorderableList {
        self.physics = Some(physics);
        self
    }

    /// Dart `ReorderableList(shrinkWrap:)`.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> ReorderableList {
        self.shrink_wrap = shrink_wrap;
        self
    }

    /// Dart `ReorderableList(anchor:)`.
    pub fn anchor(mut self, anchor: f64) -> ReorderableList {
        self.anchor = anchor;
        self
    }

    /// Dart `ReorderableList(cacheExtent:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent(mut self, cache_extent: f64) -> ReorderableList {
        self.cache_extent = Some(cache_extent);
        self
    }

    /// Dart `ReorderableList(scrollCacheExtent:)`.
    pub fn scroll_cache_extent(
        mut self,
        scroll_cache_extent: ScrollCacheExtent,
    ) -> ReorderableList {
        self.scroll_cache_extent = Some(scroll_cache_extent);
        self
    }

    /// Dart `ReorderableList(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> ReorderableList {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `ReorderableList(keyboardDismissBehavior:)`.
    pub fn keyboard_dismiss_behavior(
        mut self,
        keyboard_dismiss_behavior: ScrollViewKeyboardDismissBehavior,
    ) -> ReorderableList {
        self.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
        self
    }

    /// Dart `ReorderableList(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> ReorderableList {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `ReorderableList(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ReorderableList {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ReorderableList(autoScrollerVelocityScalar:)`.
    pub fn auto_scroller_velocity_scalar(mut self, value: f64) -> ReorderableList {
        self.auto_scroller_velocity_scalar = Some(value);
        self
    }

    /// Dart `ReorderableList(dragBoundaryProvider:)`.
    pub fn drag_boundary_provider(
        mut self,
        drag_boundary_provider: impl Fn(
            &mut App,
            BuildContext,
        ) -> Option<Box<dyn DragBoundaryDelegate<Rect>>>
        + 'static,
    ) -> ReorderableList {
        self.drag_boundary_provider = Some(Rc::new(drag_boundary_provider));
        self
    }

    /// The state from the closest instance of this class that encloses the given context.
    ///
    /// This method is typically used by [`ReorderableList`] item widgets that insert or remove
    /// items in response to user input.
    ///
    /// If no [`ReorderableList`] surrounds the given context, then this function will panic.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), a similar function that will return `None` if no
    ///    [`ReorderableList`] ancestor is found.
    pub fn of(app: &App, context: BuildContext) -> Handle<ReorderableListState> {
        let result = context.find_ancestor_state_of_type::<ReorderableListState>(app);
        result.unwrap_or_else(|| {
            panic!(
                "ReorderableList.of() called with a context that does not contain a \
                 ReorderableList.\nNo ReorderableList ancestor could be found starting from the \
                 context that was passed to ReorderableList.of().\nThis can happen when the \
                 context provided is from the same StatefulWidget that built the \
                 ReorderableList. Please see the ReorderableList documentation for examples of \
                 how to refer to an ReorderableListState object:\n  \
                 https://api.flutter.dev/flutter/widgets/ReorderableListState-class.html"
            )
        })
    }

    /// The state from the closest instance of this class that encloses the given context.
    ///
    /// This method is typically used by [`ReorderableList`] item widgets that insert or remove
    /// items in response to user input.
    ///
    /// If no [`ReorderableList`] surrounds the context given, then this function will return
    /// `None`.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), a similar function that will panic if no [`ReorderableList`]
    ///    ancestor is found.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<Handle<ReorderableListState>> {
        context.find_ancestor_state_of_type::<ReorderableListState>(app)
    }
}

/// Dart's three constructor asserts that relate two or more arguments, shared by
/// [`ReorderableList`] and [`SliverReorderableList`].
fn debug_check_reorderable_arguments(
    item_extent: Option<f64>,
    prototype_item: Option<&WidgetRef>,
    item_extent_builder: Option<&ItemExtentBuilder>,
    on_reorder: Option<&ReorderCallback>,
    on_reorder_item: Option<&ReorderCallback>,
) {
    debug_assert!(
        (item_extent.is_none() && prototype_item.is_none())
            || (item_extent.is_none() && item_extent_builder.is_none())
            || (prototype_item.is_none() && item_extent_builder.is_none()),
        "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
    );
    debug_assert!(
        (on_reorder_item.is_some() && on_reorder.is_none())
            || (on_reorder_item.is_none() && on_reorder.is_some()),
        "The onReorder callback is obsolete and is replaced by onReorderItem. Remove the \
         onReorder callback when both callbacks are provided."
    );
}

impl Debug for ReorderableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReorderableList")
            .field("key", &self.key)
            .field("item_count", &self.item_count)
            .field("scroll_direction", &self.scroll_direction)
            .field("reverse", &self.reverse)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ReorderableList {
    type State = ReorderableListState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_check_reorderable_arguments(
            self.item_extent,
            self.prototype_item.as_ref(),
            self.item_extent_builder.as_ref(),
            self.on_reorder.as_ref(),
            self.on_reorder_item.as_ref(),
        );
    }

    fn create_state(&self) -> ReorderableListState {
        ReorderableListState {
            state: StateData::new(),
            sliver_reorderable_list_key: GlobalKey::new(),
        }
    }
}

/// The state for a list that allows the user to interactively reorder the list items.
///
/// An app that needs to start a new item drag or cancel an existing one can refer to the
/// [`ReorderableList`]'s state with a global key, and call
/// `list_key.current_state::<ReorderableListState>(app).unwrap().cancel_reorder(app)`.
pub struct ReorderableListState {
    state: StateData<ReorderableList>,
    sliver_reorderable_list_key: GlobalKey,
}

impl ReorderableListState {
    fn effective_scroll_cache_extent(self: Handle<Self>, app: &App) -> Option<ScrollCacheExtent> {
        let widget = self.widget(app);
        if widget.scroll_cache_extent.is_some() {
            return widget.scroll_cache_extent;
        }

        if let Some(cache_extent) = widget.cache_extent {
            return Some(ScrollCacheExtent::Pixels(cache_extent));
        }
        None
    }

    /// Initiate the dragging of the item at `index` that was started with the pointer down
    /// `event`.
    ///
    /// The given `recognizer` will be used to recognize and start the drag item tracking and
    /// lead to either an item reorder, or a canceled drag. The list will take ownership of the
    /// returned recognizer and will dispose it when it is no longer needed.
    ///
    /// Most applications will not use this directly, but will wrap the item (or part of the
    /// item, like a drag handle) in either a [`ReorderableDragStartListener`] or
    /// [`ReorderableDelayedDragStartListener`] which call this for the application.
    pub fn start_item_drag_reorder<R: MultiDragGestureRecognizer>(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        event: PointerDownEvent,
        recognizer: Ref<R>,
    ) {
        app.get(self)
            .sliver_reorderable_list_key
            .current_state::<SliverReorderableListState>(app)
            .expect("the SliverReorderableList is mounted")
            .start_item_drag_reorder(app, index, event, recognizer);
    }

    /// Cancel any item drag in progress.
    ///
    /// This should be called before any major changes to the item list occur so that any item
    /// drags will not get confused by changes to the underlying list.
    ///
    /// If no drag is active, this will do nothing.
    pub fn cancel_reorder(self: Handle<Self>, app: &mut App) {
        app.get(self)
            .sliver_reorderable_list_key
            .current_state::<SliverReorderableListState>(app)
            .expect("the SliverReorderableList is mounted")
            .cancel_reorder(app);
    }
}

impl State for ReorderableListState {
    type Widget = ReorderableList;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let scroll_cache_extent = self.effective_scroll_cache_extent(app);
        let widget = self.widget(app);
        let mut sliver =
            SliverReorderableList::from_builder(widget.item_builder.clone(), widget.item_count)
                .key(app.get(self).sliver_reorderable_list_key.clone().into());
        sliver.item_extent = widget.item_extent;
        sliver.prototype_item = widget.prototype_item.clone();
        sliver.item_extent_builder = widget.item_extent_builder.clone();
        sliver.on_reorder = widget.on_reorder.clone();
        sliver.on_reorder_item = widget.on_reorder_item.clone();
        sliver.on_reorder_start = widget.on_reorder_start.clone();
        sliver.on_reorder_end = widget.on_reorder_end.clone();
        sliver.proxy_decorator = widget.proxy_decorator.clone();
        if let Some(auto_scroller_velocity_scalar) = widget.auto_scroller_velocity_scalar {
            sliver = sliver.auto_scroller_velocity_scalar(auto_scroller_velocity_scalar);
        }
        sliver.drag_boundary_provider = widget.drag_boundary_provider.clone();

        let mut scroll_view = CustomScrollView::new()
            .scroll_direction(widget.scroll_direction)
            .reverse(widget.reverse);
        if let Some(controller) = widget.controller.clone() {
            scroll_view = scroll_view.controller(controller);
        }
        if let Some(primary) = widget.primary {
            scroll_view = scroll_view.primary(primary);
        }
        if let Some(physics) = widget.physics.clone() {
            scroll_view = scroll_view.physics(physics);
        }
        scroll_view = scroll_view
            .shrink_wrap(widget.shrink_wrap)
            .anchor(widget.anchor);
        if let Some(scroll_cache_extent) = scroll_cache_extent {
            scroll_view = scroll_view.scroll_cache_extent(scroll_cache_extent);
        }
        scroll_view = scroll_view.drag_start_behavior(widget.drag_start_behavior);
        if let Some(keyboard_dismiss_behavior) = widget.keyboard_dismiss_behavior {
            scroll_view = scroll_view.keyboard_dismiss_behavior(keyboard_dismiss_behavior);
        }
        if let Some(restoration_id) = widget.restoration_id.clone() {
            scroll_view = scroll_view.restoration_id(restoration_id);
        }
        scroll_view
            .clip_behavior(widget.clip_behavior)
            .slivers([
                SliverPadding::new(widget.padding.unwrap_or(EdgeInsetsGeometry::ZERO))
                    .sliver(sliver)
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// A sliver list that allows the user to interactively reorder the list items.
///
/// It is up to the application to wrap each child (or an internal part of the child) with a
/// drag listener that will recognize the start of an item drag and then start the reorder by
/// calling [`SliverReorderableListState::start_item_drag_reorder`]. This is most easily
/// achieved by wrapping each child in a [`ReorderableDragStartListener`] or a
/// [`ReorderableDelayedDragStartListener`]. These will take care of recognizing the start of a
/// drag gesture and call the list state's start item drag method.
///
/// This widget's [`SliverReorderableListState`] can be used to manually start an item reorder,
/// or cancel a current drag that's already underway. To refer to the
/// [`SliverReorderableListState`] either provide a [`GlobalKey`] or use the static
/// [`SliverReorderableList::of`] method from an item's build method.
///
/// See also:
///
///  * [`ReorderableList`], a regular widget list that allows the user to reorder its items.
///  * `ReorderableListView`, a Material Design list that allows the user to reorder its items.
pub struct SliverReorderableList {
    pub key: Option<KeyRef>,

    /// Called, as needed, to build list item widgets; see [`ReorderableList::item_builder`].
    pub item_builder: IndexedWidgetBuilder,

    /// Called to find the new index of a child based on its key in case of reordering.
    pub find_child_index_callback: Option<ChildIndexGetter>,

    /// The number of items in the list; see [`ReorderableList::item_count`].
    pub item_count: i32,

    /// See [`ReorderableList::on_reorder`]. Deprecated: use
    /// [`on_reorder_item`](Self::on_reorder_item).
    pub on_reorder: Option<ReorderCallback>,

    /// See [`ReorderableList::on_reorder_item`].
    pub on_reorder_item: Option<ReorderCallback>,

    /// See [`ReorderableList::on_reorder_start`].
    pub on_reorder_start: Option<ReorderIndexCallback>,

    /// See [`ReorderableList::on_reorder_end`].
    pub on_reorder_end: Option<ReorderIndexCallback>,

    /// See [`ReorderableList::proxy_decorator`].
    pub proxy_decorator: Option<ReorderItemProxyDecorator>,

    /// See [`ReorderableList::item_extent`].
    pub item_extent: Option<f64>,

    /// See [`ReorderableList::item_extent_builder`].
    pub item_extent_builder: Option<ItemExtentBuilder>,

    /// See [`ReorderableList::prototype_item`].
    pub prototype_item: Option<WidgetRef>,

    /// The velocity scalar per pixel over scroll of the auto scroller.
    ///
    /// Defaults to 50 if not set or set to `None`.
    pub auto_scroller_velocity_scalar: f64,

    /// See [`ReorderableList::drag_boundary_provider`].
    pub drag_boundary_provider: Option<ReorderDragBoundaryProvider>,
}

impl SliverReorderableList {
    // An eyeballed value for a smooth scrolling experience.
    const K_DEFAULT_AUTO_SCROLL_VELOCITY_SCALAR: f64 = 50.0;

    /// Creates a sliver list that allows the user to interactively reorder its items.
    ///
    /// The `item_count` must be greater than or equal to zero.
    pub fn new(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> WidgetRef + 'static,
        item_count: i32,
    ) -> SliverReorderableList {
        SliverReorderableList::from_builder(Rc::new(item_builder), item_count)
    }

    /// [`new`](Self::new) from a shared builder, which [`ReorderableList`] passes on.
    pub fn from_builder(
        item_builder: IndexedWidgetBuilder,
        item_count: i32,
    ) -> SliverReorderableList {
        debug_assert!(item_count >= 0);
        SliverReorderableList {
            key: None,
            item_builder,
            find_child_index_callback: None,
            item_count,
            on_reorder: None,
            on_reorder_item: None,
            on_reorder_start: None,
            on_reorder_end: None,
            proxy_decorator: None,
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            auto_scroller_velocity_scalar:
                SliverReorderableList::K_DEFAULT_AUTO_SCROLL_VELOCITY_SCALAR,
            drag_boundary_provider: None,
        }
    }

    /// Dart `SliverReorderableList(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverReorderableList {
        self.key = Some(key);
        self
    }

    /// Dart `SliverReorderableList(findChildIndexCallback:)`.
    pub fn find_child_index_callback(
        mut self,
        find_child_index_callback: impl Fn(&KeyRef) -> Option<i32> + 'static,
    ) -> SliverReorderableList {
        self.find_child_index_callback = Some(Rc::new(find_child_index_callback));
        self
    }

    /// Dart `SliverReorderableList(onReorder:)`.
    #[deprecated(
        note = "Use the on_reorder_item callback instead. The on_reorder_item callback adjusts \
                the new_index parameter for a removed item at the old_index."
    )]
    pub fn on_reorder(
        mut self,
        on_reorder: impl Fn(&mut App, i32, i32) + 'static,
    ) -> SliverReorderableList {
        self.on_reorder = Some(Rc::new(on_reorder));
        self
    }

    /// Dart `SliverReorderableList(onReorderItem:)`.
    pub fn on_reorder_item(
        mut self,
        on_reorder_item: impl Fn(&mut App, i32, i32) + 'static,
    ) -> SliverReorderableList {
        self.on_reorder_item = Some(Rc::new(on_reorder_item));
        self
    }

    /// Dart `SliverReorderableList(onReorderStart:)`.
    pub fn on_reorder_start(
        mut self,
        on_reorder_start: impl Fn(&mut App, i32) + 'static,
    ) -> SliverReorderableList {
        self.on_reorder_start = Some(Rc::new(on_reorder_start));
        self
    }

    /// Dart `SliverReorderableList(onReorderEnd:)`.
    pub fn on_reorder_end(
        mut self,
        on_reorder_end: impl Fn(&mut App, i32) + 'static,
    ) -> SliverReorderableList {
        self.on_reorder_end = Some(Rc::new(on_reorder_end));
        self
    }

    /// Dart `SliverReorderableList(itemExtent:)`.
    pub fn item_extent(mut self, item_extent: f64) -> SliverReorderableList {
        self.item_extent = Some(item_extent);
        self
    }

    /// Dart `SliverReorderableList(itemExtentBuilder:)`.
    pub fn item_extent_builder(
        mut self,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverReorderableList {
        self.item_extent_builder = Some(item_extent_builder);
        self
    }

    /// Dart `SliverReorderableList(prototypeItem:)`.
    pub fn prototype_item<K>(
        mut self,
        prototype_item: impl IntoWidget<K>,
    ) -> SliverReorderableList {
        self.prototype_item = Some(prototype_item.into_widget());
        self
    }

    /// Dart `SliverReorderableList(proxyDecorator:)`.
    pub fn proxy_decorator(
        mut self,
        proxy_decorator: impl Fn(&mut App, WidgetRef, i32, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> SliverReorderableList {
        self.proxy_decorator = Some(Rc::new(proxy_decorator));
        self
    }

    /// Dart `SliverReorderableList(dragBoundaryProvider:)`.
    pub fn drag_boundary_provider(
        mut self,
        drag_boundary_provider: impl Fn(
            &mut App,
            BuildContext,
        ) -> Option<Box<dyn DragBoundaryDelegate<Rect>>>
        + 'static,
    ) -> SliverReorderableList {
        self.drag_boundary_provider = Some(Rc::new(drag_boundary_provider));
        self
    }

    /// Dart `SliverReorderableList(autoScrollerVelocityScalar:)`.
    pub fn auto_scroller_velocity_scalar(mut self, value: f64) -> SliverReorderableList {
        self.auto_scroller_velocity_scalar = value;
        self
    }

    /// The state from the closest instance of this class that encloses the given context.
    ///
    /// This method is typically used by [`SliverReorderableList`] item widgets to start or
    /// cancel an item drag operation.
    ///
    /// If no [`SliverReorderableList`] surrounds the context given, this function will panic.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), a similar function that will return `None` if no
    ///    [`SliverReorderableList`] ancestor is found.
    pub fn of(app: &App, context: BuildContext) -> Handle<SliverReorderableListState> {
        let result = context.find_ancestor_state_of_type::<SliverReorderableListState>(app);
        result.unwrap_or_else(|| {
            panic!(
                "SliverReorderableList.of() called with a context that does not contain a \
                 SliverReorderableList.\nNo SliverReorderableList ancestor could be found \
                 starting from the context that was passed to SliverReorderableList.of().\nThis \
                 can happen when the context provided is from the same StatefulWidget that built \
                 the SliverReorderableList. Please see the SliverReorderableList documentation \
                 for examples of how to refer to an SliverReorderableList object:\n  \
                 https://api.flutter.dev/flutter/widgets/SliverReorderableListState-class.html"
            )
        })
    }

    /// The state from the closest instance of this class that encloses the given context.
    ///
    /// This method is typically used by [`SliverReorderableList`] item widgets that insert or
    /// remove items in response to user input.
    ///
    /// If no [`SliverReorderableList`] surrounds the context given, this function will return
    /// `None`.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), a similar function that will panic if no
    ///    [`SliverReorderableList`] ancestor is found.
    pub fn maybe_of(
        app: &App,
        context: BuildContext,
    ) -> Option<Handle<SliverReorderableListState>> {
        context.find_ancestor_state_of_type::<SliverReorderableListState>(app)
    }
}

impl Debug for SliverReorderableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SliverReorderableList")
            .field("key", &self.key)
            .field("item_count", &self.item_count)
            .field("item_extent", &self.item_extent)
            .field(
                "auto_scroller_velocity_scalar",
                &self.auto_scroller_velocity_scalar,
            )
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SliverReorderableList {
    type State = SliverReorderableListState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_check_reorderable_arguments(
            self.item_extent,
            self.prototype_item.as_ref(),
            self.item_extent_builder.as_ref(),
            self.on_reorder.as_ref(),
            self.on_reorder_item.as_ref(),
        );
    }

    fn create_state(&self) -> SliverReorderableListState {
        SliverReorderableListState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            items: IndexMap::new(),
            overlay_entry: None,
            drag_index: None,
            drag_info: None,
            insert_index: None,
            final_drop_position: None,
            recognizer: None,
            recognizer_pointer: None,
            auto_scroller: None,
            scrollable: None,
        }
    }
}

/// The state for a sliver list that allows the user to interactively reorder the list items.
///
/// An app that needs to start a new item drag or cancel an existing one can refer to the
/// [`SliverReorderableList`]'s state with a global key, and call
/// `list_key.current_state::<SliverReorderableListState>(app).unwrap().cancel_reorder(app)`.
///
/// [`ReorderableDragStartListener`] and [`ReorderableDelayedDragStartListener`] refer to their
/// [`SliverReorderableList`] with the static [`SliverReorderableList::of`] method.
pub struct SliverReorderableListState {
    state: StateData<SliverReorderableList>,
    ticker_provider: TickerProviderStateMixinData,

    // Map of index -> child state used manage where the dragging item will need to be
    // inserted.
    items: IndexMap<i32, Handle<ReorderableItemState>>,

    overlay_entry: Option<Ref<OverlayEntry>>,
    drag_index: Option<i32>,
    drag_info: Option<Ref<DragInfo>>,
    insert_index: Option<i32>,
    final_drop_position: Option<Offset>,
    recognizer: Option<RecognizerRef>,
    recognizer_pointer: Option<i64>,

    auto_scroller: Option<Ref<EdgeDraggingAutoScroller>>,

    // Dart's `late`: set in `did_change_dependencies`, before anything reads it.
    scrollable: Option<Handle<ScrollableState>>,
}

impl SliverReorderableListState {
    fn scrollable(self: Handle<Self>, app: &App) -> Handle<ScrollableState> {
        app.get(self)
            .scrollable
            .expect("set in did_change_dependencies")
    }

    fn scroll_direction(self: Handle<Self>, app: &App) -> Axis {
        axis_direction_to_axis(self.scrollable(app).axis_direction(app))
    }

    fn reverse(self: Handle<Self>, app: &App) -> bool {
        axis_direction_is_reversed(self.scrollable(app).axis_direction(app))
    }

    fn drag_info(self: Handle<Self>, app: &App) -> Handle<DragInfo> {
        app.get(self)
            .drag_info
            .handle()
            .expect("a drag is in progress")
    }

    fn new_auto_scroller(self: Handle<Self>, app: &mut App) -> Ref<EdgeDraggingAutoScroller> {
        let scrollable = self.scrollable(app);
        let velocity_scalar = self.widget(app).auto_scroller_velocity_scalar;
        EdgeDraggingAutoScroller::new(
            app,
            scrollable,
            Some(Listener::handle_method(
                self,
                Self::handle_scrollable_auto_scrolled,
            )),
            velocity_scalar,
        )
    }

    /// Initiate the dragging of the item at `index` that was started with the pointer down
    /// `event`.
    ///
    /// The given `recognizer` will be used to recognize and start the drag item tracking and
    /// lead to either an item reorder, or a canceled drag.
    ///
    /// Most applications will not use this directly, but will wrap the item (or part of the
    /// item, like a drag handle) in either a [`ReorderableDragStartListener`] or
    /// [`ReorderableDelayedDragStartListener`] which call this method when they detect the
    /// gesture that triggers a drag start.
    pub fn start_item_drag_reorder<R: MultiDragGestureRecognizer>(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        event: PointerDownEvent,
        recognizer: Ref<R>,
    ) {
        debug_assert!(0 <= index && index < self.widget(app).item_count);
        if app.get(self).drag_info.is_some() {
            self.cancel_reorder(app);
        } else if app.get(self).recognizer.is_some()
            && app.get(self).recognizer_pointer != Some(event.pointer)
        {
            if let Some(recognizer) = app.get_mut(self).recognizer.take() {
                recognizer.dispose(app);
            }
            app.get_mut(self).recognizer_pointer = None;
        }

        if app.get(self).items.contains_key(&index) {
            app.get_mut(self).drag_index = Some(index);
            let on_start: GestureMultiDragStartCallback =
                Rc::new(move |app, position| self.drag_start(app, position));
            recognizer.set_on_start(app, Some(on_start));
            let pointer = event.pointer;
            recognizer.add_pointer(app, event);
            let state = app.get_mut(self);
            state.recognizer = Some(recognizer.into_erased());
            state.recognizer_pointer = Some(pointer);
        } else {
            panic!("Attempting to start a drag on a non-visible item");
        }
        self.set_state(app, |_| {});
    }

    /// Cancel any item drag in progress.
    ///
    /// This should be called before any major changes to the item list occur so that any item
    /// drags will not get confused by changes to the underlying list.
    ///
    /// If a drag operation is in progress, this will immediately reset the list to back to its
    /// pre-drag state.
    ///
    /// If no drag is active, this will do nothing.
    pub fn cancel_reorder(self: Handle<Self>, app: &mut App) {
        self.drag_reset(app);
        self.set_state(app, |_| {});
    }

    fn register_item(self: Handle<Self>, app: &mut App, item: Handle<ReorderableItemState>) {
        let item_index = item.index(app);
        if let Some(drag_info) = app.get(self).drag_info.handle()
            && app.get(self).items.get(&item_index) != Some(&item)
        {
            let (index, item_extent) = (app.get(drag_info).index, app.get(drag_info).item_extent);
            let reverse = self.reverse(app);
            item.update_for_gap(app, index, index, item_extent, false, reverse);
        }
        app.get_mut(self).items.insert(item_index, item);
        let drag_info_index = app
            .get(self)
            .drag_info
            .handle()
            .map(|drag_info| app.get(drag_info).index);
        if Some(item_index) == drag_info_index {
            item.set_dragging(app, true);
            item.rebuild(app);
        }
    }

    fn unregister_item(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        item: Handle<ReorderableItemState>,
    ) {
        let current_item = app.get(self).items.get(&index).copied();
        if current_item == Some(item) {
            app.get_mut(self).items.shift_remove(&index);
        }
    }

    fn drag_start(self: Handle<Self>, app: &mut App, position: Offset) -> Option<Rc<dyn Drag>> {
        debug_assert!(app.get(self).drag_info.is_none());
        let drag_index = app
            .get(self)
            .drag_index
            .expect("set when the drag was started");
        let item = app.get(self).items[&drag_index];
        item.set_dragging(app, true);
        if let Some(on_reorder_start) = self.widget(app).on_reorder_start.clone() {
            on_reorder_start(app, drag_index);
        }
        item.rebuild(app);

        let item_index = item.index(app);
        app.get_mut(self).insert_index = Some(item_index);
        let scroll_direction = self.scroll_direction(app);
        let on_update: DragItemUpdate =
            Rc::new(move |app, item, position, delta| self.drag_update(app, item, position, delta));
        let on_cancel: DragItemCallback = Rc::new(move |app, item| self.drag_cancel(app, item));
        let on_end: DragItemCallback = Rc::new(move |app, item| self.drag_end(app, item));
        let on_drop_completed: VoidCallback = Rc::new(move |app| self.drop_completed(app));
        let proxy_decorator = self.widget(app).proxy_decorator.clone();
        let drag_info = DragInfo::new(
            app,
            item,
            position,
            scroll_direction,
            Some(on_update),
            Some(on_end),
            Some(on_cancel),
            Some(on_drop_completed),
            proxy_decorator,
            Rc::new(self),
        );
        app.get_mut(self).drag_info = Some(drag_info.clone());
        drag_info.start_drag(app);

        let overlay = Overlay::of(app, self.context(app), false);
        debug_assert!(app.get(self).overlay_entry.is_none());
        let proxy_source = drag_info.clone();
        let overlay_entry = OverlayEntry::new(
            app,
            Rc::new(move |app, context| proxy_source.create_proxy(app, context)),
            false,
            false,
            false,
        );
        app.get_mut(self).overlay_entry = Some(overlay_entry.clone());
        overlay.insert(app, overlay_entry, None, None);

        let insert_index = item_index;
        let item_extent = app.get(&drag_info).item_extent;
        let reverse = self.reverse(app);
        let child_items: Vec<Handle<ReorderableItemState>> =
            app.get(self).items.values().copied().collect();
        for child_item in child_items {
            if child_item == item || !child_item.mounted(app) {
                continue;
            }
            child_item.update_for_gap(app, insert_index, insert_index, item_extent, false, reverse);
        }
        Some(Rc::new(drag_info))
    }

    fn drag_update(
        self: Handle<Self>,
        app: &mut App,
        _item: Handle<DragInfo>,
        _position: Offset,
        _delta: Offset,
    ) {
        if let Some(overlay_entry) = app.get(self).overlay_entry.handle() {
            overlay_entry.mark_needs_build(app);
        }
        self.drag_update_items(app);
        if let Some(auto_scroller) = app.get(self).auto_scroller.handle() {
            let drag_target_rect = self.drag_target_rect(app);
            auto_scroller.start_auto_scroll_if_necessary(app, drag_target_rect);
        }
        self.set_state(app, |_| {});
    }

    fn drag_cancel(self: Handle<Self>, app: &mut App, _item: Handle<DragInfo>) {
        self.drag_reset(app);
        self.set_state(app, |_| {});
    }

    fn drag_end(self: Handle<Self>, app: &mut App, item: Handle<DragInfo>) {
        let insert_index = app
            .get(self)
            .insert_index
            .expect("set when the drag started");
        let (item_index, item_item_extent) = (app.get(item).index, app.get(item).item_extent);
        let scroll_direction = self.scroll_direction(app);
        let final_drop_position = if insert_index - item_index == 1 {
            // When returning to original position from below, insert_index equals
            // item.index + 1 because insertion index is calculated with the dragged
            // item still present. Use the actual target position for animation.
            self.item_offset_at(app, insert_index - 1)
        } else if insert_index == item_index {
            // No movement - animate to current position
            self.item_offset_at(app, insert_index)
        } else if self.reverse(app) {
            let items_length = app.get(self).items.len() as i32;
            if insert_index >= items_length {
                // Drop at the starting position of the last element and offset its own extent
                self.item_offset_at(app, items_length - 1)
                    - extent_offset(item_item_extent, scroll_direction)
            } else {
                // Drop at the end of the current element occupying the insert position
                self.item_offset_at(app, insert_index)
                    + extent_offset(self.item_extent_at(app, insert_index), scroll_direction)
            }
        } else if insert_index == 0 {
            // Drop at the starting position of the first element and offset its own extent
            self.item_offset_at(app, 0) - extent_offset(item_item_extent, scroll_direction)
        } else {
            // Drop at the end of the previous element occupying the insert position
            let at_index = insert_index - 1;
            self.item_offset_at(app, at_index)
                + extent_offset(self.item_extent_at(app, at_index), scroll_direction)
        };
        app.get_mut(self).final_drop_position = Some(final_drop_position);
        self.set_state(app, |_| {});
        if let Some(on_reorder_end) = self.widget(app).on_reorder_end.clone() {
            on_reorder_end(app, insert_index);
        }
    }

    fn drop_completed(self: Handle<Self>, app: &mut App) {
        let old_index = app.get(self).drag_index.expect("set when the drag started");
        let new_index = app
            .get(self)
            .insert_index
            .expect("set when the drag started");

        self.handle_reorder_item(app, old_index, new_index);

        self.drag_reset(app);
        self.set_state(app, |_| {});
    }

    fn drag_reset(self: Handle<Self>, app: &mut App) {
        let Some(drag_info) = app.get(self).drag_info.handle() else {
            return;
        };
        if let Some(drag_index) = app.get(self).drag_index
            && let Some(drag_item) = app.get(self).items.get(&drag_index).copied()
        {
            app.get_mut(drag_item).dragging = false;
            drag_item.rebuild(app);
            app.get_mut(self).drag_index = None;
        }
        drag_info.dispose(app);
        app.get_mut(self).drag_info = None;
        if let Some(auto_scroller) = app.get(self).auto_scroller.handle() {
            auto_scroller.stop_auto_scroll(app);
        }
        self.reset_item_gap(app);
        if let Some(recognizer) = app.get_mut(self).recognizer.take() {
            recognizer.dispose(app);
        }
        if let Some(overlay_entry) = app.get_mut(self).overlay_entry.take() {
            overlay_entry.remove(app);
            overlay_entry.dispose(app);
        }
        app.get_mut(self).final_drop_position = None;
    }

    fn reset_item_gap(self: Handle<Self>, app: &mut App) {
        let items: Vec<Handle<ReorderableItemState>> =
            app.get(self).items.values().copied().collect();
        for item in items {
            item.reset_gap(app);
        }
    }

    fn handle_reorder_item(self: Handle<Self>, app: &mut App, old_index: i32, mut new_index: i32) {
        if let Some(on_reorder) = self.widget(app).on_reorder.clone()
            && old_index != new_index
        {
            on_reorder(app, old_index, new_index);
            return;
        }

        // Removing an item at the old index shortens the list by one.
        if new_index > old_index {
            new_index -= 1;
        }

        if old_index != new_index
            && let Some(on_reorder_item) = self.widget(app).on_reorder_item.clone()
        {
            on_reorder_item(app, old_index, new_index);
        }
    }

    fn handle_scrollable_auto_scrolled(self: Handle<Self>, app: &mut App) {
        if app.get(self).drag_info.is_none() {
            return;
        }
        self.drag_update_items(app);
        // Continue scrolling if the drag is still in progress.
        if let Some(auto_scroller) = app.get(self).auto_scroller.handle() {
            let drag_target_rect = self.drag_target_rect(app);
            auto_scroller.start_auto_scroll_if_necessary(app, drag_target_rect);
        }
    }

    fn drag_update_items(self: Handle<Self>, app: &mut App) {
        let drag_info = self.drag_info(app);
        let scroll_direction = self.scroll_direction(app);
        let reverse = self.reverse(app);
        let drag_index = app.get(self).drag_index.expect("set when the drag started");
        let gap_extent = app.get(drag_info).item_extent;
        let proxy_item_start = offset_extent(
            app.get(drag_info).drag_position - app.get(drag_info).drag_offset,
            scroll_direction,
        );
        let proxy_item_end = proxy_item_start + gap_extent;

        // Find the new index for inserting the item being dragged.
        let mut new_index = app
            .get(self)
            .insert_index
            .expect("set when the drag started");
        let items: Vec<Handle<ReorderableItemState>> =
            app.get(self).items.values().copied().collect();
        for &item in &items {
            let index = item.index(app);
            if (reverse && index == drag_index) || !item.mounted(app) {
                continue;
            }

            let geometry = item.target_geometry(app);
            let item_start = if scroll_direction == Axis::Vertical {
                geometry.top
            } else {
                geometry.left
            };
            let item_extent = if scroll_direction == Axis::Vertical {
                geometry.height()
            } else {
                geometry.width()
            };
            let item_end = item_start + item_extent;
            let item_middle = item_start + item_extent / 2.0;

            if reverse {
                if item_end >= proxy_item_end && proxy_item_end >= item_middle {
                    // The start of the proxy is in the beginning half of the item, so
                    // we should swap the item with the gap and we are done looking for
                    // the new index.
                    new_index = index;
                    break;
                } else if item_middle >= proxy_item_start && proxy_item_start >= item_start {
                    // The end of the proxy is in the ending half of the item, so
                    // we should swap the item with the gap and we are done looking for
                    // the new index.
                    new_index = index + 1;
                    break;
                } else if item_start > proxy_item_end && new_index < (index + 1) {
                    new_index = index + 1;
                } else if proxy_item_start > item_end && new_index > index {
                    new_index = index;
                }
            } else if index == drag_index {
                // If end of the proxy is not in ending half of item,
                // we don't process, because it's original dragged item.
                if item_middle <= proxy_item_end && proxy_item_end <= item_end {
                    new_index = drag_index;
                }
            } else if item_start <= proxy_item_start && proxy_item_start <= item_middle {
                // The start of the proxy is in the beginning half of the item, so
                // we should swap the item with the gap and we are done looking for
                // the new index.
                new_index = index;
                break;
            } else if item_middle <= proxy_item_end && proxy_item_end <= item_end {
                // The end of the proxy is in the ending half of the item, so
                // we should swap the item with the gap and we are done looking for
                // the new index.
                new_index = index + 1;
                break;
            } else if item_end < proxy_item_start && new_index < (index + 1) {
                new_index = index + 1;
            } else if proxy_item_end < item_start && new_index > index {
                new_index = index;
            }
        }

        if Some(new_index) != app.get(self).insert_index {
            app.get_mut(self).insert_index = Some(new_index);
            for &item in &items {
                if item.index(app) == drag_index || !item.mounted(app) {
                    continue;
                }
                item.update_for_gap(app, drag_index, new_index, gap_extent, true, reverse);
            }
        }
    }

    fn drag_target_rect(self: Handle<Self>, app: &App) -> Rect {
        let drag_info = app.get(self.drag_info(app));
        let origin = drag_info.drag_position - drag_info.drag_offset;
        Rect::from_ltwh(
            origin.dx(),
            origin.dy(),
            drag_info.item_size.width(),
            drag_info.item_size.height(),
        )
    }

    fn item_offset_at(self: Handle<Self>, app: &App, index: i32) -> Offset {
        app.get(self).items[&index].target_geometry(app).top_left()
    }

    fn item_extent_at(self: Handle<Self>, app: &App, index: i32) -> f64 {
        size_extent(
            app.get(self).items[&index].target_geometry(app).size(),
            self.scroll_direction(app),
        )
    }

    fn item_builder(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: i32,
    ) -> WidgetRef {
        if let Some(drag_info) = app.get(self).drag_info.handle()
            && index >= self.widget(app).item_count
        {
            let item_extent = app.get(drag_info).item_extent;
            return match self.scroll_direction(app) {
                Axis::Horizontal => SizedBox::new().width(item_extent).into_widget(),
                Axis::Vertical => SizedBox::new().height(item_extent).into_widget(),
            };
        }
        let item_builder = self.widget(app).item_builder.clone();
        let child = item_builder(app, context, index);
        debug_assert!(child.key().is_some(), "All list items must have a key");
        let overlay = Overlay::of(app, context, false);
        let overlay_context = overlay.context(app);
        let sub_key = child.key().expect("All list items must have a key").clone();
        ReorderableItem {
            key: GlobalKey::wrapping(Rc::new(ReorderableItemGlobalKey {
                sub_key,
                index,
                state: self.id(),
            }))
            .into(),
            index,
            captured_themes: Rc::new(<dyn InheritedTheme>::capture(
                app,
                context,
                Some(overlay_context),
            )),
            child: self.wrap_with_semantics(app, child, index),
        }
        .into_widget()
    }

    fn wrap_with_semantics(
        self: Handle<Self>,
        app: &mut App,
        child: WidgetRef,
        index: i32,
    ) -> WidgetRef {
        // First, determine which semantics actions apply.
        let mut semantics_actions: IndexMap<CustomSemanticsAction, VoidCallback> = IndexMap::new();

        // Create the appropriate semantics actions.
        let move_to_start: VoidCallback =
            Rc::new(move |app| self.handle_reorder_item(app, index, 0));
        let move_to_end: VoidCallback = Rc::new(move |app| {
            let item_count = self.widget(app).item_count;
            self.handle_reorder_item(app, index, item_count);
        });
        let move_before: VoidCallback =
            Rc::new(move |app| self.handle_reorder_item(app, index, index - 1));
        // To move after, go to index+2 because it is moved to the space
        // before index+2, which is after the space at index+1.
        let move_after: VoidCallback =
            Rc::new(move |app| self.handle_reorder_item(app, index, index + 2));

        let context = self.context(app);
        let localizations = <dyn WidgetsLocalizations>::of(app, context);
        let is_horizontal = self.scroll_direction(app) == Axis::Horizontal;
        // If the item can move to before its current position in the list.
        if index > 0 {
            semantics_actions.insert(
                CustomSemanticsAction::new(localizations.reorder_item_to_start()),
                move_to_start,
            );
            let mut reorder_item_before = localizations.reorder_item_up();
            if is_horizontal {
                reorder_item_before = if Directionality::of(app, context) == TextDirection::Ltr {
                    localizations.reorder_item_left()
                } else {
                    localizations.reorder_item_right()
                };
            }
            semantics_actions.insert(CustomSemanticsAction::new(reorder_item_before), move_before);
        }

        // If the item can move to after its current position in the list.
        if index < self.widget(app).item_count - 1 {
            let mut reorder_item_after = localizations.reorder_item_down();
            if is_horizontal {
                reorder_item_after = if Directionality::of(app, context) == TextDirection::Ltr {
                    localizations.reorder_item_right()
                } else {
                    localizations.reorder_item_left()
                };
            }
            semantics_actions.insert(CustomSemanticsAction::new(reorder_item_after), move_after);
            semantics_actions.insert(
                CustomSemanticsAction::new(localizations.reorder_item_to_end()),
                move_to_end,
            );
        }

        // Pass toWrap with a GlobalKey into the item so that when it
        // gets dragged, the accessibility framework can preserve the selected
        // state of the dragging item.
        //
        // Also apply the relevant custom accessibility actions for moving the item
        // up, down, to the start, and to the end of the list.
        Semantics::new()
            .container(true)
            .custom_semantics_actions(semantics_actions)
            .child(child)
            .into_widget()
    }
}

impl TickerProviderObject for SliverReorderableListState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for SliverReorderableListState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl State for SliverReorderableListState {
    type Widget = SliverReorderableList;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let scrollable = Scrollable::of(app, self.context(app), None);
        app.get_mut(self).scrollable = Some(scrollable);
        let current = app
            .get(self)
            .auto_scroller
            .handle()
            .map(|auto_scroller| app.get(auto_scroller).scrollable);
        if current != Some(scrollable) {
            if let Some(auto_scroller) = app.get(self).auto_scroller.handle() {
                auto_scroller.stop_auto_scroll(app);
            }
            let auto_scroller = self.new_auto_scroller(app);
            app.get_mut(self).auto_scroller = Some(auto_scroller);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SliverReorderableList) {
        if self.widget(app).item_count != old_widget.item_count {
            self.cancel_reorder(app);
        }

        if self.widget(app).auto_scroller_velocity_scalar
            != old_widget.auto_scroller_velocity_scalar
        {
            if let Some(auto_scroller) = app.get(self).auto_scroller.handle() {
                auto_scroller.stop_auto_scroll(app);
            }
            let auto_scroller = self.new_auto_scroller(app);
            app.get_mut(self).auto_scroller = Some(auto_scroller);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.drag_reset(app);
        if let Some(recognizer) = app.get_mut(self).recognizer.take() {
            recognizer.dispose(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_overlay(app, context));
        let widget = self.widget(app);
        let mut children_delegate = SliverChildBuilderDelegate::new(move |app, context, index| {
            Some(self.item_builder(app, context, index))
        })
        .child_count(widget.item_count);
        if let Some(find_child_index_callback) = widget.find_child_index_callback.clone() {
            children_delegate =
                children_delegate.find_child_index_callback(find_child_index_callback);
        }
        let children_delegate = children_delegate.into_delegate();
        if let Some(item_extent) = widget.item_extent {
            return SliverFixedExtentList::new(children_delegate, item_extent).into_widget();
        } else if let Some(item_extent_builder) = widget.item_extent_builder.clone() {
            return SliverVariedExtentList::new(children_delegate, item_extent_builder)
                .into_widget();
        } else if let Some(prototype_item) = widget.prototype_item.clone() {
            return SliverPrototypeExtentList::new(children_delegate, prototype_item).into_widget();
        }
        SliverList::new(children_delegate).into_widget()
    }
}

/// Dart's `_ReorderableItem`.
#[derive(Debug)]
struct ReorderableItem {
    key: KeyRef,
    index: i32,
    child: WidgetRef,
    captured_themes: Rc<CapturedThemes>,
}

impl StatefulWidget for ReorderableItem {
    type State = ReorderableItemState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_state(&self) -> ReorderableItemState {
        ReorderableItemState {
            state: StateData::new(),
            list_state: None,
            start_offset: Offset::ZERO,
            target_offset: Offset::ZERO,
            offset_animation: None,
            dragging: false,
        }
    }
}

/// Dart's `_ReorderableItemState`.
struct ReorderableItemState {
    state: StateData<ReorderableItem>,
    // Dart's `late`: set in `init_state`.
    list_state: Option<Handle<SliverReorderableListState>>,

    start_offset: Offset,
    target_offset: Offset,
    offset_animation: Option<Ref<AnimationController>>,

    dragging: bool,
}

impl ReorderableItemState {
    fn list_state(self: Handle<Self>, app: &App) -> Handle<SliverReorderableListState> {
        app.get(self).list_state.expect("set in init_state")
    }

    fn index(self: Handle<Self>, app: &App) -> i32 {
        self.widget(app).index
    }

    fn set_dragging(self: Handle<Self>, app: &mut App, dragging: bool) {
        if self.mounted(app) {
            self.set_state(app, |state| state.dragging = dragging);
        }
    }

    fn offset(self: Handle<Self>, app: &App) -> Offset {
        let state = app.get(self);
        if let Some(offset_animation) = &state.offset_animation {
            let anim_value = Curves::ease_in_out().transform(offset_animation.value(app));
            return Offset::lerp(
                Some(state.start_offset),
                Some(state.target_offset),
                anim_value,
            )
            .expect("both offsets are given");
        }
        state.target_offset
    }

    fn update_for_gap(
        self: Handle<Self>,
        app: &mut App,
        drag_index: i32,
        gap_index: i32,
        gap_extent: f64,
        animate: bool,
        reverse: bool,
    ) {
        // An offset needs to be added to create a gap when we are between the
        // moving element (drag_index) and the current gap position (gap_index).
        // For how to update the gap position, refer to `drag_update_items`.
        let index = self.index(app);
        let scroll_direction = self.list_state(app).scroll_direction(app);
        let new_target_offset =
            if gap_index < drag_index && index < drag_index && index >= gap_index {
                extent_offset(
                    if reverse { -gap_extent } else { gap_extent },
                    scroll_direction,
                )
            } else if gap_index > drag_index && index > drag_index && index < gap_index {
                extent_offset(
                    if reverse { gap_extent } else { -gap_extent },
                    scroll_direction,
                )
            } else {
                Offset::ZERO
            };
        if new_target_offset != app.get(self).target_offset {
            let previous_target = app.get(self).target_offset;
            app.get_mut(self).target_offset = new_target_offset;
            if animate {
                if let Some(offset_animation) = app.get(self).offset_animation.handle() {
                    // Animation interrupted - calculate current position from previous animation
                    let current_anim_value =
                        Curves::ease_in_out().transform(offset_animation.value(app));
                    let current_position = Offset::lerp(
                        Some(app.get(self).start_offset),
                        Some(previous_target),
                        current_anim_value,
                    )
                    .expect("both offsets are given");
                    app.get_mut(self).start_offset = current_position;
                    drop(offset_animation.forward(app, Some(0.0)));
                } else {
                    let list_state = self.list_state(app);
                    let offset_animation = AnimationController::new(
                        app,
                        None,
                        Some(Duration::from_millis(250)),
                        None,
                        0.0,
                        1.0,
                        AnimationBehavior::Normal,
                        list_state,
                    );
                    offset_animation
                        .add_listener(app, Listener::handle_method(self, Self::rebuild));
                    offset_animation.add_status_listener(
                        app,
                        AnimationStatusListener::new(move |status, app| {
                            if status.is_completed() {
                                let state = app.get_mut(self);
                                state.start_offset = state.target_offset;
                                if let Some(offset_animation) = state.offset_animation.take() {
                                    offset_animation.dispose(app);
                                }
                            }
                        }),
                    );
                    app.get_mut(self).offset_animation = Some(offset_animation.clone());
                    drop(offset_animation.forward(app, None));
                }
            } else {
                if let Some(offset_animation) = app.get_mut(self).offset_animation.take() {
                    offset_animation.dispose(app);
                }
                let state = app.get_mut(self);
                state.start_offset = state.target_offset;
            }
            self.rebuild(app);
        }
    }

    fn reset_gap(self: Handle<Self>, app: &mut App) {
        if let Some(offset_animation) = app.get_mut(self).offset_animation.take() {
            offset_animation.dispose(app);
        }
        let state = app.get_mut(self);
        state.start_offset = Offset::ZERO;
        state.target_offset = Offset::ZERO;
        self.rebuild(app);
    }

    fn target_geometry(self: Handle<Self>, app: &App) -> Rect {
        let item_render_box = self
            .context(app)
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box())
            .expect("a reorderable item is a RenderBox");
        let item_position =
            item_render_box.local_to_global(app, Offset::ZERO, None) + app.get(self).target_offset;
        item_position & item_render_box.size(app)
    }

    fn rebuild(self: Handle<Self>, app: &mut App) {
        if self.mounted(app) {
            self.set_state(app, |_| {});
        }
    }
}

impl State for ReorderableItemState {
    type Widget = ReorderableItem;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let list_state = SliverReorderableList::of(app, self.context(app));
        app.get_mut(self).list_state = Some(list_state);
        list_state.register_item(app, self);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(offset_animation) = app.get(self).offset_animation.handle() {
            offset_animation.dispose(app);
        }
        let index = self.index(app);
        self.list_state(app).unregister_item(app, index, self);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ReorderableItem) {
        if old_widget.index != self.widget(app).index {
            let list_state = self.list_state(app);
            list_state.unregister_item(app, old_widget.index, self);
            list_state.register_item(app, self);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let list_state = self.list_state(app);
        if app.get(self).dragging {
            let item_extent = app.get(list_state.drag_info(app)).item_extent;
            let size = extent_size(item_extent, list_state.scroll_direction(app));
            return SizedBox::from_size(Some(size)).into_widget();
        }
        list_state.register_item(app, self);
        Transform::translate(self.offset(app))
            .child(self.widget(app).child.clone())
            .into_widget()
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        let index = self.index(app);
        self.list_state(app).unregister_item(app, index, self);
    }
}

/// A wrapper widget that will recognize the start of a drag on the wrapped widget by a
/// [`PointerDownEvent`], and immediately initiate dragging the wrapped item to a new location
/// in a reorderable list.
///
/// See also:
///
///  * [`ReorderableDelayedDragStartListener`], a similar wrapper that will only recognize the
///    start after a long press event.
///  * [`ReorderableList`], a widget list that allows the user to reorder its items.
///  * [`SliverReorderableList`], a sliver list that allows the user to reorder its items.
///  * `ReorderableListView`, a Material Design list that allows the user to reorder its items.
#[derive(Clone, Debug)]
pub struct ReorderableDragStartListener {
    pub key: Option<KeyRef>,

    /// The widget for which the application would like to respond to a tap and drag gesture
    /// by starting a reordering drag on a reorderable list.
    pub child: WidgetRef,

    /// The index of the associated item that will be dragged in the list.
    pub index: i32,

    /// Whether the [`child`](Self::child) item can be dragged and moved in the list.
    ///
    /// If true, the item can be moved to another location in the list when the user taps on
    /// the child. If false, tapping on the child will be ignored.
    pub enabled: bool,
}

impl ReorderableDragStartListener {
    /// Creates a listener for a drag immediately following a pointer down event over the given
    /// child widget.
    ///
    /// This is most commonly used to wrap part of a list item like a drag handle.
    pub fn new<K>(child: impl IntoWidget<K>, index: i32) -> ReorderableDragStartListener {
        ReorderableDragStartListener {
            key: None,
            child: child.into_widget(),
            index,
            enabled: true,
        }
    }

    /// Dart `ReorderableDragStartListener(key:)`.
    pub fn key(mut self, key: KeyRef) -> ReorderableDragStartListener {
        self.key = Some(key);
        self
    }

    /// Dart `ReorderableDragStartListener(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ReorderableDragStartListener {
        self.enabled = enabled;
        self
    }
}

/// Dart's `ReorderableDragStartListener` as the base class of the listeners: a subclass, such
/// as [`ReorderableDelayedDragStartListener`], overrides
/// [`create_recognizer`](Self::create_recognizer), and its build is
/// [`build`](Self::build).
pub trait ReorderableDragStartListenerWidget: Clone + 'static {
    /// The recognizer [`create_recognizer`](Self::create_recognizer) provides.
    type Recognizer: MultiDragGestureRecognizer;

    /// The base class's fields.
    fn reorderable_drag_start_listener(&self) -> &ReorderableDragStartListener;

    /// Provides the gesture recognizer used to indicate the start of a reordering drag
    /// operation.
    ///
    /// [`ReorderableDragStartListener`] answers an [`ImmediateMultiDragGestureRecognizer`];
    /// a subclass customizes the drag start gesture here.
    fn create_recognizer(&self, app: &mut App) -> Ref<Self::Recognizer>;

    /// Dart's `ReorderableDragStartListener.build`, which every subclass inherits.
    fn build(&self, _app: &mut App, context: BuildContext) -> WidgetRef {
        let listener = self.reorderable_drag_start_listener();
        let mut pointer_listener = PointerListener::new().child(listener.child.clone());
        if listener.enabled {
            let this = self.clone();
            pointer_listener = pointer_listener.on_pointer_down(Rc::new(
                move |app: &mut App, event: PointerDownEvent| {
                    start_dragging(&this, app, context, event);
                },
            ));
        }
        pointer_listener.into_widget()
    }
}

/// Dart's `ReorderableDragStartListener._startDragging`.
fn start_dragging<W: ReorderableDragStartListenerWidget>(
    widget: &W,
    app: &mut App,
    context: BuildContext,
    event: PointerDownEvent,
) {
    let gesture_settings = MediaQuery::maybe_gesture_settings_of(app, context);
    let list = SliverReorderableList::maybe_of(app, context);
    if let Some(list) = list {
        let recognizer = widget.create_recognizer(app);
        recognizer.set_gesture_settings(app, gesture_settings);
        let index = widget.reorderable_drag_start_listener().index;
        list.start_item_drag_reorder(app, index, event, recognizer);
    }
}

impl ReorderableDragStartListenerWidget for ReorderableDragStartListener {
    type Recognizer = ImmediateMultiDragGestureRecognizer;

    fn reorderable_drag_start_listener(&self) -> &ReorderableDragStartListener {
        self
    }

    fn create_recognizer(&self, app: &mut App) -> Ref<ImmediateMultiDragGestureRecognizer> {
        ImmediateMultiDragGestureRecognizer::new(app)
    }
}

impl StatelessWidget for ReorderableDragStartListener {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        ReorderableDragStartListenerWidget::build(self, app, context)
    }
}

/// A wrapper widget that will recognize the start of a drag operation by looking for a long
/// press event. Once it is recognized, it will start a drag operation on the wrapped item in
/// the reorderable list.
///
/// See also:
///
///  * [`ReorderableDragStartListener`], a similar wrapper that will recognize the start of the
///    drag immediately after a pointer down event.
///  * [`ReorderableList`], a widget list that allows the user to reorder its items.
///  * [`SliverReorderableList`], a sliver list that allows the user to reorder its items.
///  * `ReorderableListView`, a Material Design list that allows the user to reorder its items.
#[derive(Clone, Debug)]
pub struct ReorderableDelayedDragStartListener {
    /// The [`ReorderableDragStartListener`] superclass fields.
    pub listener: ReorderableDragStartListener,
}

impl ReorderableDelayedDragStartListener {
    /// Creates a listener for an drag following a long press event over the given child
    /// widget.
    ///
    /// This is most commonly used to wrap an entire list item in a reorderable list.
    pub fn new<K>(child: impl IntoWidget<K>, index: i32) -> ReorderableDelayedDragStartListener {
        ReorderableDelayedDragStartListener {
            listener: ReorderableDragStartListener::new(child, index),
        }
    }

    /// Dart `ReorderableDelayedDragStartListener(key:)`.
    pub fn key(mut self, key: KeyRef) -> ReorderableDelayedDragStartListener {
        self.listener.key = Some(key);
        self
    }

    /// Dart `ReorderableDelayedDragStartListener(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ReorderableDelayedDragStartListener {
        self.listener.enabled = enabled;
        self
    }
}

impl ReorderableDragStartListenerWidget for ReorderableDelayedDragStartListener {
    type Recognizer = DelayedMultiDragGestureRecognizer;

    fn reorderable_drag_start_listener(&self) -> &ReorderableDragStartListener {
        &self.listener
    }

    fn create_recognizer(&self, app: &mut App) -> Ref<DelayedMultiDragGestureRecognizer> {
        DelayedMultiDragGestureRecognizer::new(app)
    }
}

impl StatelessWidget for ReorderableDelayedDragStartListener {
    fn key(&self) -> Option<&KeyRef> {
        self.listener.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        ReorderableDragStartListenerWidget::build(self, app, context)
    }
}

/// Dart's `_DragItemUpdate`.
type DragItemUpdate = Rc<dyn Fn(&mut App, Handle<DragInfo>, Offset, Offset)>;

/// Dart's `_DragItemCallback`.
type DragItemCallback = Rc<dyn Fn(&mut App, Handle<DragInfo>)>;

/// Dart's `_DragInfo`: the [`Drag`] the recognizer reports an item drag's moves to.
struct DragInfo {
    scroll_direction: Axis,
    on_update: Option<DragItemUpdate>,
    on_end: Option<DragItemCallback>,
    on_cancel: Option<DragItemCallback>,
    on_drop_completed: Option<VoidCallback>,
    proxy_decorator: Option<ReorderItemProxyDecorator>,
    ticker_provider: Rc<dyn TickerProvider>,

    boundary: Option<Box<dyn DragBoundaryDelegate<Rect>>>,
    list_state: Handle<SliverReorderableListState>,
    index: i32,
    child: WidgetRef,
    drag_position: Offset,
    drag_offset: Offset,
    item_size: Size,
    item_layout_constraints: BoxConstraints,
    item_extent: f64,
    captured_themes: Rc<CapturedThemes>,
    #[expect(dead_code, reason = "Dart's field, which nothing in the file reads")]
    scrollable: Option<Handle<ScrollableState>>,
    proxy_animation: Option<Ref<AnimationController>>,
    raw_drag_position: Offset,
}

impl DragInfo {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's named constructor arguments, all of which its one caller passes"
    )]
    fn new(
        app: &mut App,
        item: Handle<ReorderableItemState>,
        initial_position: Offset,
        scroll_direction: Axis,
        on_update: Option<DragItemUpdate>,
        on_end: Option<DragItemCallback>,
        on_cancel: Option<DragItemCallback>,
        on_drop_completed: Option<VoidCallback>,
        proxy_decorator: Option<ReorderItemProxyDecorator>,
        ticker_provider: Rc<dyn TickerProvider>,
    ) -> Ref<DragInfo> {
        let item_context = item.context(app);
        let item_render_box = item_context
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box())
            .expect("a reorderable item is a RenderBox");
        let list_state = item.list_state(app);
        let index = item.index(app);
        let child = item.widget(app).child.clone();
        let captured_themes = Rc::clone(&item.widget(app).captured_themes);
        let drag_offset = item_render_box.global_to_local(app, initial_position, None);
        let item_size = item_context.size(app).expect("a laid-out item");
        let raw_drag_position = initial_position;
        let list_context = list_state.context(app);
        let boundary = match list_state.widget(app).drag_boundary_provider.clone() {
            Some(drag_boundary_provider) => drag_boundary_provider(app, list_context),
            None => DragBoundary::for_rect_maybe_of(app, list_context, true),
        };
        let drag_position = adjusted_drag_offset(
            boundary.as_deref(),
            drag_offset,
            item_size,
            initial_position,
        );
        let item_extent = size_extent(item_size, scroll_direction);
        let item_layout_constraints = item_render_box.constraints(app);
        let scrollable = Some(Scrollable::of(app, item_context, None));
        let this = app.create(DragInfo {
            scroll_direction,
            on_update,
            on_end,
            on_cancel,
            on_drop_completed,
            proxy_decorator,
            ticker_provider,
            boundary,
            list_state,
            index,
            child,
            drag_position,
            drag_offset,
            item_size,
            item_layout_constraints,
            item_extent,
            captured_themes,
            scrollable,
            proxy_animation: None,
            raw_drag_position,
        });
        app.expect_dispose(&this);
        this
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.mark_disposed(self);
        if let Some(proxy_animation) = app.get(self).proxy_animation.handle() {
            proxy_animation.dispose(app);
        }
    }

    fn start_drag(self: Handle<Self>, app: &mut App) {
        let ticker_provider = Rc::clone(&app.get(self).ticker_provider);
        let proxy_animation = AnimationController::new(
            app,
            None,
            Some(Duration::from_millis(250)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            ticker_provider,
        );
        proxy_animation.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                if status.is_dismissed() {
                    self.drop_completed(app);
                }
            }),
        );
        app.get_mut(self).proxy_animation = Some(proxy_animation.clone());
        drop(proxy_animation.forward(app, None));
    }

    fn adjusted_drag_offset(self: Handle<Self>, app: &App, offset: Offset) -> Offset {
        let info = app.get(self);
        adjusted_drag_offset(
            info.boundary.as_deref(),
            info.drag_offset,
            info.item_size,
            offset,
        )
    }

    fn drop_completed(self: Handle<Self>, app: &mut App) {
        if let Some(proxy_animation) = app.get_mut(self).proxy_animation.take() {
            proxy_animation.dispose(app);
        }
        if let Some(on_drop_completed) = app.get(self).on_drop_completed.clone() {
            on_drop_completed(app);
        }
    }

    fn create_proxy(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let position =
            app.get(self).drag_position - app.get(self).drag_offset - overlay_origin(app, context);
        let info = app.get(self);
        info.captured_themes.wrap(
            DragItemProxy {
                list_state: info.list_state,
                index: info.index,
                child: info.child.clone(),
                position,
                size: info.item_size,
                constraints: info.item_layout_constraints,
                animation: info
                    .proxy_animation
                    .clone()
                    .expect("the drag started its proxy animation"),
                proxy_decorator: info.proxy_decorator.clone(),
            }
            .into_widget(),
        )
    }
}

/// Dart's `_DragInfo._adjustedDragOffset`, over the fields it reads, so the constructor can
/// call it before the object exists.
fn adjusted_drag_offset(
    boundary: Option<&dyn DragBoundaryDelegate<Rect>>,
    drag_offset: Offset,
    item_size: Size,
    offset: Offset,
) -> Offset {
    let Some(boundary) = boundary else {
        return offset;
    };
    boundary
        .nearest_position_within_boundary((offset - drag_offset) & item_size)
        .shift(drag_offset)
        .top_left()
}

impl DragObject for DragInfo {
    fn update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        let delta = restrict_axis(details.delta, app.get(self).scroll_direction);
        let raw_drag_position = app.get(self).raw_drag_position + delta;
        app.get_mut(self).raw_drag_position = raw_drag_position;
        let drag_position = self.adjusted_drag_offset(app, raw_drag_position);
        app.get_mut(self).drag_position = drag_position;
        if let Some(on_update) = app.get(self).on_update.clone() {
            on_update(app, self, drag_position, details.delta);
        }
    }

    fn end(self: Handle<Self>, app: &mut App, _details: DragEndDetails) {
        let proxy_animation = app
            .get(self)
            .proxy_animation
            .handle()
            .expect("the drag started its proxy animation");
        drop(proxy_animation.reverse(app, None));
        if let Some(on_end) = app.get(self).on_end.clone() {
            on_end(app, self);
        }
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        if let Some(proxy_animation) = app.get_mut(self).proxy_animation.take() {
            proxy_animation.dispose(app);
        }
        if let Some(on_cancel) = app.get(self).on_cancel.clone() {
            on_cancel(app, self);
        }
    }
}

/// Dart's `_overlayOrigin`.
fn overlay_origin(app: &mut App, context: BuildContext) -> Offset {
    let overlay = Overlay::of(app, context, false);
    let overlay_box = overlay
        .context(app)
        .find_render_object(app)
        .and_then(|render_object| render_object.as_box())
        .expect("an Overlay's render object is a RenderBox");
    overlay_box.local_to_global(app, Offset::ZERO, None)
}

/// Dart's `_DragItemProxy`.
struct DragItemProxy {
    list_state: Handle<SliverReorderableListState>,
    index: i32,
    child: WidgetRef,
    position: Offset,
    size: Size,
    constraints: BoxConstraints,
    animation: Ref<AnimationController>,
    proxy_decorator: Option<ReorderItemProxyDecorator>,
}

impl Debug for DragItemProxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragItemProxy")
            .field("index", &self.index)
            .field("position", &self.position)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for DragItemProxy {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let proxy_child = match &self.proxy_decorator {
            Some(proxy_decorator) => {
                proxy_decorator(app, self.child.clone(), self.index, self.animation.erase())
            }
            None => self.child.clone(),
        };
        let overlay_origin = overlay_origin(app, context);

        let list_state = self.list_state;
        let position = self.position;
        let size = self.size;
        let constraints = self.constraints;
        let animation = self.animation.handle();
        let listenable: Rc<dyn Listenable> = Rc::new(self.animation.clone());
        MediaQuery::new(
            // Remove the top padding so that any nested list views in the item
            // won't pick up the scaffold's padding in the overlay.
            MediaQuery::of(app, context).remove_padding(false, true, false, false),
            AnimatedBuilder::new(listenable, move |app, _context, child| {
                let mut effective_position = position;
                let drop_position = app.get(list_state).final_drop_position;
                if let Some(drop_position) = drop_position {
                    effective_position = Offset::lerp(
                        Some(drop_position - overlay_origin),
                        Some(effective_position),
                        Curves::ease_out().transform(animation.value(app)),
                    )
                    .expect("both offsets are given");
                }
                let mut overflow_box = OverflowBox::new()
                    .min_width(constraints.min_width)
                    .min_height(constraints.min_height)
                    .max_width(constraints.max_width)
                    .max_height(constraints.max_height)
                    .alignment(if list_state.scroll_direction(app) == Axis::Horizontal {
                        AlignmentGeometry::CENTER_LEFT
                    } else {
                        AlignmentGeometry::TOP_CENTER
                    });
                if let Some(child) = child {
                    overflow_box = overflow_box.child(child.clone());
                }
                Positioned::new(
                    SizedBox::new()
                        .width(size.width())
                        .height(size.height())
                        .child(overflow_box),
                )
                .left(effective_position.dx())
                .top(effective_position.dy())
                .into_widget()
            })
            .child(proxy_child),
        )
        .into_widget()
    }
}

fn size_extent(size: Size, scroll_direction: Axis) -> f64 {
    match scroll_direction {
        Axis::Horizontal => size.width(),
        Axis::Vertical => size.height(),
    }
}

fn extent_size(extent: f64, scroll_direction: Axis) -> Size {
    match scroll_direction {
        Axis::Horizontal => Size::new(extent, 0.0),
        Axis::Vertical => Size::new(0.0, extent),
    }
}

fn offset_extent(offset: Offset, scroll_direction: Axis) -> f64 {
    match scroll_direction {
        Axis::Horizontal => offset.dx(),
        Axis::Vertical => offset.dy(),
    }
}

fn extent_offset(extent: f64, scroll_direction: Axis) -> Offset {
    match scroll_direction {
        Axis::Horizontal => Offset::new(extent, 0.0),
        Axis::Vertical => Offset::new(0.0, extent),
    }
}

fn restrict_axis(offset: Offset, scroll_direction: Axis) -> Offset {
    match scroll_direction {
        Axis::Horizontal => Offset::new(offset.dx(), 0.0),
        Axis::Vertical => Offset::new(0.0, offset.dy()),
    }
}

/// Dart's `_ReorderableItemGlobalKey`: a global key that takes its identity from the object
/// and uses a value of a particular type to identify itself.
///
/// The difference with `GlobalObjectKey` is that it uses `==` instead of `identical` of the
/// objects used to generate widgets.
#[derive(Debug)]
struct ReorderableItemGlobalKey {
    sub_key: KeyRef,
    index: i32,
    state: HandleId,
}

impl Key for ReorderableItemGlobalKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<ReorderableItemGlobalKey>()
            .is_some_and(|other| {
                other.sub_key == self.sub_key
                    && other.index == self.index
                    && other.state == self.state
            })
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<ReorderableItemGlobalKey>().hash(&mut state);
        self.sub_key.hash(&mut state);
        self.index.hash(&mut state);
        self.state.hash(&mut state);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    use waft_animation::{Drive, Tween};
    use waft_embedder::Color;
    use waft_foundation::ValueKey;
    use waft_gestures::{DeviceGestureSettings, K_LONG_PRESS_TIMEOUT, K_PRESS_TIMEOUT};
    use waft_painting::{EdgeInsets, TextStyle};
    use waft_rendering::CrossAxisAlignment;

    use super::*;
    use crate::testing::{
        TestPointer, TestWidgetsApp, WidgetTester, expect, find, finds_nothing, finds_one_widget,
        test_widgets,
    };
    use crate::widgets::basic::{
        Align, Center, Column, Expanded, Row, StateSetter, StatefulBuilder,
    };
    use crate::widgets::container::Container;
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::icon::Icon;
    use crate::widgets::icon_data::IconData;
    use crate::widgets::icon_theme::IconTheme;
    use crate::widgets::icon_theme_data::IconThemeData;
    use crate::widgets::layout_builder::LayoutBuilder;
    use crate::widgets::media_query::MediaQueryData;
    use crate::widgets::scroll_controller::ScrollController;
    use crate::widgets::scroll_physics::ClampingScrollPhysics;
    use crate::widgets::scroll_view::ListView;
    use crate::widgets::text::{DefaultTextStyle, RichText, Text};
    use crate::widgets::transitions::FadeTransition;

    const K_RED_COLOR: Color = Color::new(0xFFFF0000);
    const K_GREEN_COLOR: Color = Color::new(0xFF00FF00);

    fn k_drag_handle_icon_data() -> IconData {
        IconData::new(0xe25d).font_family("MaterialIcons")
    }

    /// The list a case reorders, shared with its callbacks.
    type Items = Rc<RefCell<Vec<i32>>>;

    /// Dart's `List<int>.generate(count, (int index) => index)`.
    fn items(count: i32) -> Items {
        Rc::new(RefCell::new((0..count).collect()))
    }

    /// Dart's `items.insert(toIndex, items.removeAt(fromIndex))`, counting the calls.
    fn handle_reorder(
        items: &Items,
        calls: &Rc<Cell<i32>>,
    ) -> impl Fn(&mut App, i32, i32) + 'static {
        let items = Rc::clone(items);
        let calls = Rc::clone(calls);
        move |_app, from_index, to_index| {
            calls.set(calls.get() + 1);
            let mut items = items.borrow_mut();
            let item = items.remove(from_index as usize);
            items.insert(to_index as usize, item);
        }
    }

    /// Dart's `tester.startGesture(at)`.
    fn start_gesture(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        let mut pointer = TestPointer::touch(tester.next_pointer());
        tester.down(&mut pointer, at);
        pointer
    }

    /// The cases' `SizedBox(key: ValueKey<int>(items[index]), height: 100, child: listener)`,
    /// the listener over `Text('item ${items[index]}')`.
    fn sized_item(
        items: &Items,
        delayed: bool,
        enabled: bool,
    ) -> impl Fn(&mut App, BuildContext, i32) -> WidgetRef + 'static {
        let items = Rc::clone(items);
        move |_app, _context, index| {
            let item = items.borrow()[index as usize];
            let text = Text::new(format!("item {item}"));
            let listener = if delayed {
                ReorderableDelayedDragStartListener::new(text, index)
                    .enabled(enabled)
                    .into_widget()
            } else {
                ReorderableDragStartListener::new(text, index)
                    .enabled(enabled)
                    .into_widget()
            };
            SizedBox::new()
                .key(ValueKey::new(item).into())
                .height(100.0)
                .child(listener)
                .into_widget()
        }
    }

    /// Dart's `TestListTile` from `list_tile_tester.dart`, without `onTap`.
    fn test_list_tile<K>(title: impl IntoWidget<K>) -> WidgetRef {
        Container::new()
            .constraints(BoxConstraints::new().min_height(56.0))
            .padding(EdgeInsetsGeometry::symmetric(4.0, 16.0))
            .child(title)
            .into_widget()
    }

    /// Dart's `TestList` widget's arguments besides `items`.
    #[derive(Default)]
    struct TestList {
        text_color: Option<Color>,
        icon_color: Option<Color>,
        proxy_decorator: Option<ReorderItemProxyDecorator>,
        reverse: bool,
        on_reorder_start: Option<ReorderIndexCallback>,
        on_reorder_end: Option<ReorderIndexCallback>,
    }

    /// Dart's `TestList(items: items, ..)`: 800x100 items of alternating colours, each a row of
    /// `Text('item $item')` and a drag-handle icon under a `ReorderableDragStartListener`.
    fn test_list(items: &Items, options: TestList) -> WidgetRef {
        let items = Rc::clone(items);
        let TestList {
            text_color,
            icon_color,
            proxy_decorator,
            reverse,
            on_reorder_start,
            on_reorder_end,
        } = options;
        let mut style = TextStyle::new();
        if let Some(text_color) = text_color {
            style = style.color(text_color);
        }
        let mut icon_theme_data = IconThemeData::new();
        if let Some(icon_color) = icon_color {
            icon_theme_data = icon_theme_data.color(icon_color);
        }
        TestWidgetsApp::new()
            .home(DefaultTextStyle::new(
                style,
                IconTheme::new(
                    icon_theme_data,
                    StatefulBuilder::new(move |_app, _outer_context, set_state: StateSetter| {
                        let builder_items = Rc::clone(&items);
                        let reorder_items = Rc::clone(&items);
                        let item_count = items.borrow().len() as i32;
                        let mut list = SliverReorderableList::new(
                            move |_app, _context, index| {
                                let item = builder_items.borrow()[index as usize];
                                Container::new()
                                    .key(ValueKey::new(item).into())
                                    .height(100.0)
                                    .color(if item % 2 == 1 {
                                        K_RED_COLOR
                                    } else {
                                        K_GREEN_COLOR
                                    })
                                    .child(ReorderableDragStartListener::new(
                                        Row::new()
                                            .cross_axis_alignment(CrossAxisAlignment::Start)
                                            .children([
                                                Text::new(format!("item {item}")).into_widget(),
                                                Icon::new(Some(k_drag_handle_icon_data()))
                                                    .into_widget(),
                                            ]),
                                        index,
                                    ))
                                    .into_widget()
                            },
                            item_count,
                        )
                        .on_reorder_item(
                            move |app, from_index, to_index| {
                                let items = Rc::clone(&reorder_items);
                                set_state(app, &mut |_app| {
                                    let mut items = items.borrow_mut();
                                    let item = items.remove(from_index as usize);
                                    items.insert(to_index as usize, item);
                                });
                            },
                        );
                        list.proxy_decorator = proxy_decorator.clone();
                        list.on_reorder_start = on_reorder_start.clone();
                        list.on_reorder_end = on_reorder_end.clone();
                        CustomScrollView::new()
                            .reverse(reverse)
                            .slivers([list.into_widget()])
                            .into_widget()
                    }),
                ),
            ))
            .into_widget()
    }

    #[test]
    fn sliver_reorderable_list_works_well_when_having_gesture_settings() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/103404
            let item_count = 5;
            let on_reorder_call_count = Rc::new(Cell::new(0));
            let items = items(item_count);

            // The list has five elements of height 100
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    MediaQuery::new(
                        MediaQueryData::new()
                            .gesture_settings(DeviceGestureSettings::new(Some(8.0))),
                        CustomScrollView::new()
                            .slivers([SliverReorderableList::new(
                                sized_item(&items, false, true),
                                item_count,
                            )
                            .on_reorder_item(handle_reorder(&items, &on_reorder_call_count))
                            .into_widget()])
                            .into_widget(),
                    )
                    .into_widget(),
                ),
            );

            // Start gesture on first item
            let at = tester.get_center(find::text("item 0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);

            // Drag a little bit to make `ImmediateMultiDragGestureRecognizer` compete with
            // `VerticalDragGestureRecognizer`
            tester.move_by(&mut drag, Offset::new(0.0, 10.0), true);
            tester.pump();
            // Drag enough to move down the first item
            tester.move_by(&mut drag, Offset::new(0.0, 40.0), true);
            tester.pump();
            tester.up(&drag);
            tester.pump_and_settle();

            assert_eq!(on_reorder_call_count.get(), 1);
            assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4]);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/100451
    #[test]
    fn sliver_reorderable_list_builder_respects_find_child_index_callback() {
        test_widgets(|tester| {
            let finder_called = Rc::new(Cell::new(false));
            let item_count = Rc::new(Cell::new(7));
            let state_setter: Rc<RefCell<Option<StateSetter>>> = Rc::default();

            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new({
                let finder_called = Rc::clone(&finder_called);
                let item_count = Rc::clone(&item_count);
                let state_setter = Rc::clone(&state_setter);
                move |_app, _context, set_state| {
                    *state_setter.borrow_mut() = Some(set_state);
                    let finder_called = Rc::clone(&finder_called);
                    CustomScrollView::new()
                        .slivers([SliverReorderableList::new(
                            |_app, _context, index| {
                                Container::new()
                                    .key(ValueKey::new(format!("{index}")).into())
                                    .height(2000.0)
                                    .into_widget()
                            },
                            item_count.get(),
                        )
                        .find_child_index_callback(move |_key| {
                            finder_called.set(true);
                            None
                        })
                        .on_reorder_item(|_app, _from, _to| {})
                        .into_widget()])
                        .into_widget()
                }
            })));
            assert!(!finder_called.get());

            // Trigger update.
            let set_state = state_setter.borrow().clone().expect("built");
            tester.update(|app| set_state(app, &mut |_app| item_count.set(77)));
            tester.pump();

            assert!(finder_called.get());
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/88191
    #[test]
    fn do_not_crash_when_dragging_with_two_fingers_simultaneously() {
        test_widgets(|tester| {
            let items = items(3);
            let calls = Rc::new(Cell::new(0));

            let builder_items = Rc::clone(&items);
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    ReorderableList::new(
                        move |_app, _context, index| {
                            let item = builder_items.borrow()[index as usize];
                            ReorderableDragStartListener::new(
                                SizedBox::new().height(100.0).child(
                                    Row::new()
                                        .cross_axis_alignment(CrossAxisAlignment::Start)
                                        .children(
                                            [Text::new(format!("item {item}")).into_widget()],
                                        ),
                                ),
                                index,
                            )
                            .key(ValueKey::new(item).into())
                            .into_widget()
                        },
                        3,
                    )
                    .on_reorder_item(handle_reorder(&items, &calls)),
                ),
            );

            let at = tester.get_center(find::text("item 0"));
            let mut drag1 = start_gesture(tester, at);
            let mut drag2 = start_gesture(tester, at);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);

            tester.move_by(&mut drag1, Offset::new(0.0, 100.0), true);
            tester.move_by(&mut drag2, Offset::new(0.0, 100.0), true);
            tester.pump_and_settle();

            tester.up(&drag1);
            tester.up(&drag2);
            tester.pump_and_settle();
        });
    }

    #[test]
    #[should_panic(expected = "item_count >= 0")]
    fn negative_item_count_should_assert() {
        test_widgets(|tester| {
            let items = Rc::new(RefCell::new(vec![1, 2, 3]));
            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new(
                move |_app, _outer_context, _set_state| {
                    let items = Rc::clone(&items);
                    CustomScrollView::new()
                        .slivers([SliverReorderableList::new(
                            move |_app, _context, index| {
                                SizedBox::new()
                                    .height(100.0)
                                    .child(Text::new(format!(
                                        "item {}",
                                        items.borrow()[index as usize]
                                    )))
                                    .into_widget()
                            },
                            -1,
                        )
                        .on_reorder_item(|_app, _from, _to| {})
                        .into_widget()])
                        .into_widget()
                },
            )));
        });
    }

    #[test]
    fn zero_item_count_should_not_build_widget() {
        test_widgets(|tester| {
            let items = Rc::new(RefCell::new(vec![1, 2, 3]));
            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new(
                move |_app, _outer_context, _set_state| {
                    let items = Rc::clone(&items);
                    CustomScrollView::new()
                        .slivers([
                            SliverFixedExtentList::list(
                                vec![Text::new("before").into_widget()],
                                50.0,
                            )
                            .build()
                            .into_widget(),
                            SliverReorderableList::new(
                                move |_app, _context, index| {
                                    SizedBox::new()
                                        .height(100.0)
                                        .child(Text::new(format!(
                                            "item {}",
                                            items.borrow()[index as usize]
                                        )))
                                        .into_widget()
                                },
                                0,
                            )
                            .on_reorder_item(|_app, _from, _to| {})
                            .into_widget(),
                            SliverFixedExtentList::list(
                                vec![Text::new("after").into_widget()],
                                50.0,
                            )
                            .build()
                            .into_widget(),
                        ])
                        .into_widget()
                },
            )));

            expect(tester, &find::text("before"), finds_one_widget());
            expect(
                tester,
                &find::by_type::<SliverReorderableList>(),
                finds_nothing(),
            );
            expect(tester, &find::text("after"), finds_one_widget());
        });
    }

    /// The fixed-height case's `pressDragRelease`.
    fn press_drag_release_settle(tester: &mut WidgetTester, start: Offset, delta: Offset) {
        let mut drag = start_gesture(tester, start);
        tester.pump_by(K_PRESS_TIMEOUT);
        tester.move_by(&mut drag, delta, true);
        tester.pump_by(K_PRESS_TIMEOUT);
        tester.up(&drag);
        tester.pump_and_settle();
    }

    /// The fixed-height case's `check`.
    fn check(tester: &WidgetTester, visible: &[i32], hidden: &[i32]) {
        for i in visible {
            expect(
                tester,
                &find::text(&format!("item {i}")),
                finds_one_widget(),
            );
        }
        for i in hidden {
            expect(tester, &find::text(&format!("item {i}")), finds_nothing());
        }
    }

    #[test]
    fn sliver_reorderable_list_drag_and_drop_fixed_height_items() {
        test_widgets(|tester| {
            let items = items(8);

            // The SliverReorderableList is 800x600, 8 items, each item is 800x100 with
            // an "item $index" text widget at the item's origin.  Drags are initiated by
            // a simple press on the text widget.
            tester.pump_widget(test_list(&items, TestList::default()));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);

            // Drag item 0 downwards less than halfway and let it snap back. List
            // should remain as it is.
            press_drag_release_settle(tester, Offset::new(12.0, 50.0), Offset::new(12.0, 60.0));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );
            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4, 5, 6, 7]);

            // Drag item 0 downwards more than halfway to displace item 1.
            let start = tester.get_center(find::text("item 0"));
            press_drag_release_settle(tester, start, Offset::new(0.0, 51.0));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);
            assert_eq!(tester.get_top_left(find::text("item 1")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 100.0)
            );
            assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4, 5, 6, 7]);

            // Drag item 0 back to where it was.
            let start = tester.get_center(find::text("item 0"));
            press_drag_release_settle(tester, start, Offset::new(0.0, -51.0));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );
            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4, 5, 6, 7]);

            // Drag item 1 to item 3
            let start = tester.get_center(find::text("item 1"));
            press_drag_release_settle(tester, start, Offset::new(0.0, 151.0));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 300.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 3")),
                Offset::new(0.0, 200.0)
            );
            assert_eq!(*items.borrow(), vec![0, 2, 3, 1, 4, 5, 6, 7]);

            // Drag item 1 back to where it was
            let start = tester.get_center(find::text("item 1"));
            press_drag_release_settle(tester, start, Offset::new(0.0, -200.0));
            check(tester, &[0, 1, 2, 3, 4, 5], &[6, 7]);
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 3")),
                Offset::new(0.0, 300.0)
            );
            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4, 5, 6, 7]);
        });
    }

    #[test]
    fn sliver_reorderable_list_items_inherit_default_text_style_icon_theme() {
        test_widgets(|tester| {
            let text_color = Color::new(0xffffffff);
            let icon_color = Color::new(0xff0000ff);

            fn get_icon_style(tester: &WidgetTester) -> TextStyle {
                tester.widget(
                    find::descendant(find::by_type::<Icon>(), find::by_type::<RichText>()),
                    |rich_text: &RichText| rich_text.text.style().cloned().expect("a style"),
                )
            }

            fn get_text_style(tester: &WidgetTester) -> TextStyle {
                tester.widget(
                    find::descendant(find::text("item 0"), find::by_type::<RichText>()),
                    |rich_text: &RichText| rich_text.text.style().cloned().expect("a style"),
                )
            }

            fn color(style: TextStyle) -> Option<Color> {
                style.color.as_deref().copied()
            }

            // This SliverReorderableList has just one item: "item 0".
            tester.pump_widget(test_list(
                &items(1),
                TestList {
                    text_color: Some(text_color),
                    icon_color: Some(icon_color),
                    ..TestList::default()
                },
            ));
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(color(get_icon_style(tester)), Some(icon_color));
            assert_eq!(color(get_text_style(tester)), Some(text_color));

            // Dragging item 0 causes it to be reparented in the overlay. The item
            // should still inherit the IconTheme and DefaultTextStyle because they are
            // InheritedThemes.
            let at = tester.get_center(find::text("item 0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);
            tester.move_by(&mut drag, Offset::new(0.0, 50.0), true);
            tester.pump_by(K_PRESS_TIMEOUT);
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 50.0)
            );
            assert_eq!(color(get_icon_style(tester)), Some(icon_color));
            assert_eq!(color(get_text_style(tester)), Some(text_color));

            // Drag is complete, item 0 returns to where it was.
            tester.up(&drag);
            tester.pump_and_settle();
            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(color(get_icon_style(tester)), Some(icon_color));
            assert_eq!(color(get_text_style(tester)), Some(text_color));
        });
    }

    #[test]
    fn sliver_reorderable_list_custom_proxy_decorator() {
        test_widgets(|tester| {
            fn fade_transition_key() -> ValueKey<String> {
                ValueKey::new("reordered-fade".to_string())
            }

            let proxy_decorator: ReorderItemProxyDecorator =
                Rc::new(|_app, child, _index, animation: AnimationRef<f64>| {
                    let listenable: Rc<dyn Listenable> = Rc::new(animation.clone());
                    AnimatedBuilder::new(listenable, move |app, _context, child| {
                        let fade_values = Tween::new(app, Some(1.0), Some(0.5));
                        let fade_animation = animation.drive(app, fade_values);
                        let mut fade =
                            FadeTransition::new(fade_animation).key(fade_transition_key().into());
                        if let Some(child) = child {
                            fade = fade.child(child.clone());
                        }
                        fade.into_widget()
                    })
                    .child(child)
                    .into_widget()
                });
            tester.pump_widget(test_list(
                &items(4),
                TestList {
                    proxy_decorator: Some(proxy_decorator),
                    ..TestList::default()
                },
            ));

            let get_item_fade_transition = || find::by_key(fade_transition_key());
            let opacity = |tester: &WidgetTester| {
                let opacity = tester.widget(get_item_fade_transition(), |fade: &FadeTransition| {
                    fade.opacity.clone()
                });
                tester.update(|app| opacity.value(app))
            };

            expect(tester, &get_item_fade_transition(), finds_nothing());

            // Start gesture on first item
            let at = tester.get_center(find::text("item 0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);

            // Drag enough for transition animation defined in proxyDecorator to start.
            tester.move_by(&mut drag, Offset::new(0.0, 50.0), true);
            tester.pump();

            // At the start, opacity should be at 1.0.
            expect(tester, &get_item_fade_transition(), finds_one_widget());
            assert_eq!(opacity(tester), 1.0);

            // Let animation run halfway.
            tester.pump_by(Duration::from_millis(125));
            let halfway = opacity(tester);
            assert!(halfway > 0.5);
            assert!(halfway < 1.0);

            // Allow animation to run to the end.
            tester.pump_and_settle();
            expect(
                tester,
                &find::by_key(fade_transition_key()),
                finds_one_widget(),
            );
            assert_eq!(opacity(tester), 0.5);

            // Finish reordering.
            tester.up(&drag);
            tester.pump_and_settle();
            expect(tester, &get_item_fade_transition(), finds_nothing());
        });
    }

    #[test]
    fn reorderable_list_supports_items_with_nested_list_views_without_throwing_layout_exception() {
        test_widgets(|tester| {
            tester.pump_widget(
                TestWidgetsApp::new()
                    .home(
                        ReorderableList::new(
                            |_app, _context, index| {
                                ReorderableDragStartListener::new(
                                    Column::new().children([ListView::new()
                                        .shrink_wrap(true)
                                        .physics(Rc::new(ClampingScrollPhysics::new()))
                                        .children([
                                            Text::new("Other data").into_widget(),
                                            Text::new("Other data").into_widget(),
                                            Text::new("Other data").into_widget(),
                                        ])
                                        .into_widget()]),
                                    index,
                                )
                                .key(ValueKey::new(index).into())
                                .into_widget()
                            },
                            10,
                        )
                        .on_reorder_item(|_app, _from, _to| {}),
                    )
                    .builder(|app, context, child| {
                        // Ensure there is always a top padding to simulate a phone with
                        // safe area at the top. If the nested list doesn't have the
                        // padding removed before it is put into the overlay it will
                        // overflow the layout by the top padding.
                        MediaQuery::new(
                            MediaQuery::of(app, context)
                                .copy_with()
                                .padding(EdgeInsets::only(0.0, 50.0, 0.0, 0.0)),
                            child.expect("a child").clone(),
                        )
                        .into_widget()
                    }),
            );

            // Start gesture on first item.
            let at = tester.get_center(find::by_key(ValueKey::new(0)));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);

            // Drag enough for move to start.
            tester.move_by(&mut drag, Offset::new(0.0, 50.0), true);
            tester.pump_and_settle();
        });
    }

    /// The drop-animation cases' `pressDragRelease`: settles before the release and pumps one
    /// frame after it.
    fn press_drag_release(tester: &mut WidgetTester, start: Offset, delta: Offset) {
        let mut drag = start_gesture(tester, start);
        tester.pump_by(K_PRESS_TIMEOUT);
        tester.move_by(&mut drag, delta, true);
        tester.pump_and_settle();
        tester.up(&drag);
        tester.pump();
    }

    /// Dart's `pumpFor` of 'SliverReorderableList auto scrolls speed is configurable': a frame
    /// now, then a frame each `interval` plus a millisecond until `duration` has passed.
    fn pump_for(tester: &mut WidgetTester, duration: Duration, interval: Duration) {
        tester.pump();
        let mut times = duration.as_millis().div_ceil(interval.as_millis());
        while times > 0 {
            tester.pump_by(interval + Duration::from_millis(1));
            times -= 1;
        }
    }

    #[test]
    fn sliver_reorderable_list_auto_scroll_speed_is_configurable() {
        test_widgets(|tester| {
            fn pump_list_and_drag(
                tester: &mut WidgetTester,
                auto_scroller_velocity_scalar: f64,
            ) -> f64 {
                let items = items(10);
                let controller: ScrollControllerRef = ScrollController::new(
                    &mut tester.cell.borrow_mut(),
                    0.0,
                    true,
                    None,
                    None,
                    None,
                )
                .erase();
                let builder_items = Rc::clone(&items);
                tester.pump_widget(
                    TestWidgetsApp::new().home(
                        CustomScrollView::new()
                            .controller(controller.clone())
                            .slivers([SliverReorderableList::new(
                                move |_app, _context, index| {
                                    let item = builder_items.borrow()[index as usize];
                                    Container::new()
                                        .key(ValueKey::new(item).into())
                                        .height(100.0)
                                        .color(if item % 2 == 1 {
                                            K_RED_COLOR
                                        } else {
                                            K_GREEN_COLOR
                                        })
                                        .child(
                                            ReorderableDragStartListener::new(
                                                Row::new()
                                                    .cross_axis_alignment(CrossAxisAlignment::Start)
                                                    .children([
                                                        Text::new(format!("item {item}"))
                                                            .into_widget(),
                                                        Icon::new(Some(k_drag_handle_icon_data()))
                                                            .into_widget(),
                                                    ]),
                                                index,
                                            )
                                            .into_widget(),
                                        )
                                        .into_widget()
                                },
                                items.borrow().len() as i32,
                            )
                            .on_reorder_item(|_app, _from, _to| {})
                            .auto_scroller_velocity_scalar(auto_scroller_velocity_scalar)
                            .into_widget()]),
                    ),
                );

                assert_eq!(controller.offset(&tester.cell.borrow()), 0.0);

                let item = find::text("item 0");
                let mut drag = start_gesture(tester, tester.get_center(item));

                // Drag just enough to touch the edge but not surpass it, so the auto scroller
                // is not yet triggered.
                tester.move_by(&mut drag, Offset::new(0.0, 500.0), true);
                pump_for(
                    tester,
                    Duration::from_millis(200),
                    Duration::from_millis(50),
                );

                assert_eq!(controller.offset(&tester.cell.borrow()), 0.0);

                // Now drag a little bit more so the auto scroller triggers.
                tester.move_by(&mut drag, Offset::new(0.0, 50.0), true);
                pump_for(
                    tester,
                    Duration::from_millis(600),
                    Duration::from_millis((1000.0 / auto_scroller_velocity_scalar).round() as u64),
                );

                controller.offset(&tester.cell.borrow())
            }

            const FAST_VELOCITY_SCALAR: f64 = 20.0;
            let offset_for_fast_scroller = pump_list_and_drag(tester, FAST_VELOCITY_SCALAR);

            // Reset widget tree
            tester.pump_widget(SizedBox::new());

            const SLOW_VELOCITY_SCALAR: f64 = 5.0;
            let offset_for_slow_scroller = pump_list_and_drag(tester, SLOW_VELOCITY_SCALAR);

            assert_eq!(
                offset_for_fast_scroller / offset_for_slow_scroller,
                FAST_VELOCITY_SCALAR / SLOW_VELOCITY_SCALAR
            );
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_in_a_reversed_list() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/110949
            let items = items(8);

            // The TestList is 800x600 SliverReorderableList with 8 items 800x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test we are reversing the order so
            // the first item is at the bottom.
            tester.pump_widget(test_list(
                &items,
                TestList {
                    reverse: true,
                    ..TestList::default()
                },
            ));

            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 500.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 2")),
                Offset::new(0.0, 300.0)
            );

            // Drag item 0 up and insert it between item 1 and item 2. It should
            // smoothly animate.
            let start = tester.get_center(find::text("item 0"));
            press_drag_release(tester, start, Offset::new(0.0, -50.0));
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 450.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 500.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 2")),
                Offset::new(0.0, 300.0)
            );

            // After the first several frames we should be moving closer to the final position,
            // not further away as was the case with the original bug.
            tester.pump_by(Duration::from_millis(10));
            let dy = tester.get_top_left(find::text("item 0")).dy();
            assert!(dy < 450.0);
            assert!(dy > 400.0);

            // Sample the middle (don't use exact values as it depends on the internal
            // curve being used).
            tester.pump_by(Duration::from_millis(125));
            let dy = tester.get_top_left(find::text("item 0")).dy();
            assert!(dy < 450.0);
            assert!(dy > 400.0);

            // Sample the end of the animation.
            tester.pump_by(Duration::from_millis(100));
            let dy = tester.get_top_left(find::text("item 0")).dy();
            assert!(dy < 450.0);
            assert!(dy > 400.0);

            // Wait for it to finish, it should be back to the original position
            tester.pump_and_settle();
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 400.0)
            );
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_at_starting_position_in_a_reversed_list()
    {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/84625
            let items = items(8);

            // The TestList is 800x600 SliverReorderableList with 8 items 800x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test we are reversing the order so
            // the first item is at the bottom.
            tester.pump_widget(test_list(
                &items,
                TestList {
                    reverse: true,
                    ..TestList::default()
                },
            ));

            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 500.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 400.0)
            );

            // Drag item 0 downwards off the edge and let it snap back. It should
            // smoothly animate back up.
            let start = tester.get_center(find::text("item 0"));
            press_drag_release(tester, start, Offset::new(0.0, 50.0));
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 550.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 400.0)
            );

            // After the first several frames we should be moving closer to the final position,
            // not further away as was the case with the original bug.
            tester.pump_by(Duration::from_millis(10));
            assert!(tester.get_top_left(find::text("item 0")).dy() < 550.0);

            // Sample the middle (don't use exact values as it depends on the internal
            // curve being used).
            tester.pump_by(Duration::from_millis(125));
            assert!(tester.get_top_left(find::text("item 0")).dy() < 550.0);

            // Wait for it to finish, it should be back to the original position
            tester.pump_and_settle();
            assert_eq!(
                tester.get_top_left(find::text("item 0")),
                Offset::new(0.0, 500.0)
            );
        });
    }

    /// The starting-position cases: drag `item` down 100, settle, then back by `back`, and let
    /// go; answers the item's top after 200ms of the drop.
    fn drag_down_then_back(tester: &mut WidgetTester, item: &str, back: f64) -> (Offset, Offset) {
        let at = tester.get_center(find::text(item));
        let mut drag = start_gesture(tester, at);
        tester.pump_by(K_PRESS_TIMEOUT);
        tester.move_by(&mut drag, Offset::new(0.0, 100.0), true);
        tester.pump_and_settle();
        tester.move_by(&mut drag, Offset::new(0.0, back), true);
        tester.pump();
        let dragged = tester.get_top_left(find::text(item));
        let item_1 = tester.get_top_left(find::text("item 1"));
        let released = (dragged, item_1);

        // Now leave the drag.
        tester.up(&drag);
        tester.pump();
        released
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_at_starting_position() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/150843
            // This tests the overshoot case where an item is dragged back 110% of the way
            let items = items(3);

            // The TestList is 300x100 SliverReorderableList with 3 items 100x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test the first item is at the top
            tester.pump_widget(test_list(&items, TestList::default()));

            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );

            // Drag item 0 downwards and then upwards.
            let (item_0, item_1) = drag_down_then_back(tester, "item 0", -110.0);
            assert_eq!(item_0, Offset::new(0.0, -10.0));
            assert_eq!(item_1, Offset::new(0.0, 100.0));

            // It should not go to index 1 and come back
            tester.pump_by(Duration::from_millis(200));
            assert!(tester.get_top_left(find::text("item 0")).dy() < 50.0);
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_at_starting_position_with_reverse_true() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/150843
            // This tests the overshoot case where an item is dragged back 110% of the way in a
            // reverse list
            let items = items(3);

            // The TestList is 300x100 SliverReorderableList with 3 items 100x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test the first item is at the top
            tester.pump_widget(test_list(
                &items,
                TestList {
                    reverse: true,
                    ..TestList::default()
                },
            ));

            assert_eq!(
                tester.get_top_left(find::text("item 2")),
                Offset::new(0.0, 300.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 400.0)
            );

            // Drag item 2 downwards and then upwards.
            let (item_2, item_1) = drag_down_then_back(tester, "item 2", -110.0);
            assert_eq!(item_2, Offset::new(0.0, 290.0));
            assert_eq!(item_1, Offset::new(0.0, 300.0));

            // It should not go to index 0 and come back
            tester.pump_by(Duration::from_millis(200));
            assert!(tester.get_top_left(find::text("item 2")).dy() > 350.0);
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_at_starting_position_undershoot() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/88331
            // This tests the edge case where an item is dragged back only 90% of the way
            let items = items(3);

            // The TestList is 300x100 SliverReorderableList with 3 items 100x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test the first item is at the top
            tester.pump_widget(test_list(&items, TestList::default()));

            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );

            // Drag item 0 downwards and then upwards (but not all the way back).
            let (item_0, item_1) = drag_down_then_back(tester, "item 0", -90.0);
            assert_eq!(item_0, Offset::new(0.0, 10.0));
            assert_eq!(item_1, Offset::new(0.0, 100.0));

            // It should animate back to the original position
            tester.pump_by(Duration::from_millis(200));
            assert!(tester.get_top_left(find::text("item 0")).dy() < 10.0);
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_animates_the_drop_at_starting_position_with_reverse_true_undershoot()
     {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/88331
            // This tests the edge case where an item is dragged back only 90% of the way in a
            // reverse list
            let items = items(3);

            // The TestList is 300x100 SliverReorderableList with 3 items 100x100 each.
            // Each item has a text widget with 'item $index' that can be moved by a
            // press and drag gesture. For this test the first item is at the top
            tester.pump_widget(test_list(
                &items,
                TestList {
                    reverse: true,
                    ..TestList::default()
                },
            ));

            assert_eq!(
                tester.get_top_left(find::text("item 2")),
                Offset::new(0.0, 300.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 400.0)
            );

            // Drag item 2 downwards and then upwards (but not all the way back).
            let (item_2, item_1) = drag_down_then_back(tester, "item 2", -90.0);
            assert_eq!(item_2, Offset::new(0.0, 310.0));
            assert_eq!(item_1, Offset::new(0.0, 300.0));

            // It should animate back to the original position
            tester.pump_by(Duration::from_millis(200));
            let dy = tester.get_top_left(find::text("item 2")).dy();
            assert!((dy - 300.0).abs() <= 10.0, "{dy} is not within 10 of 300");
        });
    }

    #[test]
    fn reorderable_list_animation_jumping_on_interruption() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/173243
            let items = items(3);

            tester.pump_widget(test_list(&items, TestList::default()));
            tester.pump_and_settle();

            assert_eq!(tester.get_top_left(find::text("item 0")), Offset::ZERO);
            assert_eq!(
                tester.get_top_left(find::text("item 1")),
                Offset::new(0.0, 100.0)
            );

            let at = tester.get_center(find::text("item 0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);

            // Drag item 0 down past the swap threshold at 50px.
            for _ in 0..6 {
                tester.move_by(&mut drag, Offset::new(0.0, 10.0), true);
                tester.pump_by(Duration::from_millis(50));
            }

            let item_1_y_before_interrupt = tester.get_center(find::text("item 1")).dy();

            // Drag back to trigger swap reversal.
            tester.move_by(&mut drag, Offset::new(0.0, -10.0), true);
            tester.pump_by(Duration::from_millis(50));

            let item_1_y_after_interrupt = tester.get_center(find::text("item 1")).dy();

            // Position should not jump when animation is interrupted.
            assert!(
                (item_1_y_after_interrupt - item_1_y_before_interrupt).abs() <= 5.0,
                "Animation jumping detected! Position changed from {item_1_y_before_interrupt} \
                 to {item_1_y_after_interrupt}"
            );

            for _ in 0..5 {
                tester.move_by(&mut drag, Offset::new(0.0, -10.0), true);
                tester.pump_by(Duration::from_millis(50));
            }

            tester.up(&drag);
            tester.pump_and_settle();
        });
    }

    /// The start and end cases' two drags of `item 0`: from index 0 to 3, then from 2 to 0,
    /// each started after `press`; checks the reported indices.
    fn check_reorder_start_and_end(
        tester: &mut WidgetTester,
        press: Duration,
        start_index: &Rc<Cell<Option<i32>>>,
        end_index: &Rc<Cell<Option<i32>>>,
    ) {
        let item_0 = || find::text_containing("item 0");

        let at = tester.get_center(item_0());
        let mut drag = start_gesture(tester, at);
        tester.pump_by(press);
        // Drag enough for move to start.
        tester.move_by(&mut drag, Offset::new(0.0, 20.0), true);

        assert_eq!(start_index.get(), Some(0));
        assert_eq!(end_index.get(), None);

        // Move item0 from index 0 to index 3
        tester.move_by(&mut drag, Offset::new(0.0, 300.0), true);
        tester.pump_and_settle();
        tester.up(&drag);
        tester.pump_and_settle();

        assert_eq!(end_index.get(), Some(3));

        start_index.set(None);
        end_index.set(None);

        let at = tester.get_center(item_0());
        let mut drag = start_gesture(tester, at);
        tester.pump_by(press);
        // Drag enough for move to start.
        tester.move_by(&mut drag, Offset::new(0.0, 20.0), true);

        assert_eq!(start_index.get(), Some(2));
        assert_eq!(end_index.get(), None);

        // Move item0 from index 2 to index 0
        tester.move_by(&mut drag, Offset::new(0.0, -200.0), true);
        tester.pump_and_settle();
        tester.up(&drag);
        tester.pump_and_settle();

        assert_eq!(end_index.get(), Some(0));
    }

    /// An `onReorderStart` or `onReorderEnd` that records its index.
    fn record_index(index: &Rc<Cell<Option<i32>>>) -> ReorderIndexCallback {
        let index = Rc::clone(index);
        Rc::new(move |_app, value| index.set(Some(value)))
    }

    #[test]
    fn sliver_reorderable_list_calls_on_reorder_start_and_on_reorder_end_correctly() {
        test_widgets(|tester| {
            let items = items(8);
            let start_index = Rc::new(Cell::new(None));
            let end_index = Rc::new(Cell::new(None));

            tester.pump_widget(test_list(
                &items,
                TestList {
                    on_reorder_start: Some(record_index(&start_index)),
                    on_reorder_end: Some(record_index(&end_index)),
                    ..TestList::default()
                },
            ));

            check_reorder_start_and_end(tester, K_PRESS_TIMEOUT, &start_index, &end_index);
        });
    }

    #[test]
    fn reorderable_list_calls_on_reorder_start_and_on_reorder_end_correctly() {
        test_widgets(|tester| {
            let items = items(8);
            let start_index = Rc::new(Cell::new(None));
            let end_index = Rc::new(Cell::new(None));
            let calls = Rc::new(Cell::new(0));

            let mut list = ReorderableList::new(sized_item(&items, true, true), 8)
                .on_reorder_item(handle_reorder(&items, &calls));
            list.on_reorder_start = Some(record_index(&start_index));
            list.on_reorder_end = Some(record_index(&end_index));
            tester.pump_widget(TestWidgetsApp::new().home(list));

            check_reorder_start_and_end(tester, K_LONG_PRESS_TIMEOUT, &start_index, &end_index);
        });
    }

    /// The callback cases' two drags of `item 0`: down by 50, then back up by 50.
    fn drag_item_0_down_then_up(tester: &mut WidgetTester, after_first: impl FnOnce()) {
        // Start gesture on the first item.
        let at = tester.get_center(find::text("item 0"));
        let mut drag_down = start_gesture(tester, at);
        tester.pump_by(K_PRESS_TIMEOUT);

        // Drag enough to move down the first item.
        tester.move_by(&mut drag_down, Offset::new(0.0, 50.0), true);
        tester.pump();
        tester.up(&drag_down);
        tester.pump_and_settle();

        after_first();

        // Now do the reverse.
        let at = tester.get_center(find::text("item 0"));
        let mut drag_up = start_gesture(tester, at);
        tester.pump_by(K_PRESS_TIMEOUT);

        // Drag enough to move up the first item.
        tester.move_by(&mut drag_up, Offset::new(0.0, -50.0), true);
        tester.pump();
        tester.up(&drag_up);
        tester.pump_and_settle();
    }

    /// The old `onReorder` of the callback cases, which adjusts `toIndex` itself.
    fn handle_old_reorder(
        items: &Items,
        calls: &Rc<Cell<i32>>,
    ) -> impl Fn(&mut App, i32, i32) + 'static {
        let items = Rc::clone(items);
        let calls = Rc::clone(calls);
        move |_app, from_index, mut to_index| {
            calls.set(calls.get() + 1);

            if from_index < to_index {
                to_index -= 1;
            }

            let mut items = items.borrow_mut();
            let item = items.remove(from_index as usize);
            items.insert(to_index as usize, item);
        }
    }

    #[test]
    #[expect(deprecated, reason = "Dart's case exercises the deprecated onReorder")]
    fn sliver_reorderable_list_calls_old_on_reorder_callback_correctly() {
        test_widgets(|tester| {
            let on_reorder_call_count = Rc::new(Cell::new(0));
            let items = items(5);

            tester.pump_widget(
                TestWidgetsApp::new().home(
                    CustomScrollView::new().slivers([SliverReorderableList::new(
                        sized_item(&items, false, true),
                        5,
                    )
                    .on_reorder(handle_old_reorder(&items, &on_reorder_call_count))
                    .into_widget()]),
                ),
            );

            drag_item_0_down_then_up(tester, || {
                assert_eq!(on_reorder_call_count.get(), 1);
                assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4]);
            });

            assert_eq!(on_reorder_call_count.get(), 2);
            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn sliver_reorderable_list_calls_on_reorder_item_callback_correctly() {
        test_widgets(|tester| {
            let calls = Rc::new(Cell::new(0));
            let items = items(5);

            tester.pump_widget(
                TestWidgetsApp::new().home(
                    CustomScrollView::new().slivers([SliverReorderableList::new(
                        sized_item(&items, false, true),
                        5,
                    )
                    .on_reorder_item(handle_reorder(&items, &calls))
                    .into_widget()]),
                ),
            );

            drag_item_0_down_then_up(tester, || {
                assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4]);
            });

            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    #[should_panic(expected = "The onReorder callback is obsolete")]
    fn sliver_reorderable_list_asserts_if_neither_on_reorder_and_on_reorder_item_are_provided() {
        let _ =
            SliverReorderableList::new(|_app, _context, _index| SizedBox::new().into_widget(), 0)
                .into_widget();
    }

    #[test]
    #[should_panic(expected = "The onReorder callback is obsolete")]
    #[expect(deprecated, reason = "Dart's case passes the deprecated onReorder")]
    fn sliver_reorderable_list_asserts_if_both_on_reorder_and_on_reorder_item_are_provided() {
        let _ =
            SliverReorderableList::new(|_app, _context, _index| SizedBox::new().into_widget(), 0)
                .on_reorder(|_app, _from, _to| {})
                .on_reorder_item(|_app, _from, _to| {})
                .into_widget();
    }

    #[test]
    #[expect(deprecated, reason = "Dart's case exercises the deprecated onReorder")]
    fn reorderable_list_calls_old_on_reorder_callback_correctly() {
        test_widgets(|tester| {
            let on_reorder_call_count = Rc::new(Cell::new(0));
            let items = items(5);

            tester.pump_widget(
                TestWidgetsApp::new().home(
                    ReorderableList::new(sized_item(&items, false, true), 5)
                        .on_reorder(handle_old_reorder(&items, &on_reorder_call_count)),
                ),
            );

            drag_item_0_down_then_up(tester, || {
                assert_eq!(on_reorder_call_count.get(), 1);
                assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4]);
            });

            assert_eq!(on_reorder_call_count.get(), 2);
            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_list_calls_on_reorder_item_callback_correctly() {
        test_widgets(|tester| {
            let calls = Rc::new(Cell::new(0));
            let items = items(5);

            tester.pump_widget(
                TestWidgetsApp::new().home(
                    ReorderableList::new(sized_item(&items, false, true), 5)
                        .on_reorder_item(handle_reorder(&items, &calls)),
                ),
            );

            drag_item_0_down_then_up(tester, || {
                assert_eq!(*items.borrow(), vec![1, 0, 2, 3, 4]);
            });

            assert_eq!(*items.borrow(), vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    #[should_panic(expected = "The onReorder callback is obsolete")]
    fn reorderable_list_asserts_if_neither_on_reorder_and_on_reorder_item_are_provided() {
        let _ = ReorderableList::new(|_app, _context, _index| SizedBox::new().into_widget(), 0)
            .into_widget();
    }

    #[test]
    #[should_panic(expected = "The onReorder callback is obsolete")]
    #[expect(deprecated, reason = "Dart's case passes the deprecated onReorder")]
    fn reorderable_list_asserts_if_both_on_reorder_and_on_reorder_item_are_provided() {
        let _ = ReorderableList::new(|_app, _context, _index| SizedBox::new().into_widget(), 0)
            .on_reorder(|_app, _from, _to| {})
            .on_reorder_item(|_app, _from, _to| {})
            .into_widget();
    }

    /// The extent cases' items: `SizedBox(key: ValueKey(numbers[index]), height: 20 +
    /// numbers[index] * 10)` over a drag listener of `Text('${numbers[index]}')`.
    fn number_item(numbers: &Items) -> impl Fn(&mut App, BuildContext, i32) -> WidgetRef + 'static {
        let numbers = Rc::clone(numbers);
        move |_app, _context, index| {
            let number = numbers.borrow()[index as usize];
            SizedBox::new()
                .key(ValueKey::new(number).into())
                .height(f64::from(20 + number * 10))
                .child(ReorderableDragStartListener::new(
                    Text::new(number.to_string()),
                    index,
                ))
                .into_widget()
        }
    }

    #[test]
    #[should_panic(
        expected = "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
    )]
    fn reorderable_list_asserts_on_both_non_null_item_extent_and_prototype_item() {
        let numbers = Rc::new(RefCell::new(vec![0, 1, 2]));
        let _ = ReorderableList::new(number_item(&numbers), 3)
            .item_extent(30.0)
            .prototype_item(SizedBox::new())
            .on_reorder_item(|_app, _from, _to| {})
            .into_widget();
    }

    #[test]
    fn reorderable_list_passes_item_extent_builder_to_sliver_reorderable_list() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/155936
            let item_count = 5;
            let items: Rc<Vec<f64>> = Rc::new(vec![10.0, 20.0, 30.0, 40.0, 50.0]);

            // The list has five elements, that indicate the extent for the item at the given
            // index.
            let builder_items = Rc::clone(&items);
            let extent_items = Rc::clone(&items);
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    ReorderableList::new(
                        move |_app, _context, index| {
                            // Dart's `ValueKey<double>`: a double's bits, as `f64` has no `Hash`.
                            SizedBox::new()
                                .key(ValueKey::new(builder_items[index as usize].to_bits()).into())
                                .child(Text::new(format!("Item {index}")))
                                .into_widget()
                        },
                        item_count,
                    )
                    .on_reorder_item(|_app, _from, _to| {})
                    .item_extent_builder(Rc::new(move |index, _dimensions| {
                        Some(extent_items[index as usize])
                    })),
                ),
            );

            let expected_extents: HashMap<i32, f64> =
                HashMap::from([(0, 10.0), (1, 20.0), (2, 30.0), (3, 40.0), (4, 50.0)]);

            let item_extents: HashMap<i32, f64> = (0..item_count)
                .map(|i| {
                    (
                        i,
                        tester.get_size(find::text(&format!("Item {i}"))).height(),
                    )
                })
                .collect();

            assert_eq!(item_extents, expected_extents);
        });
    }

    #[test]
    #[should_panic(
        expected = "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
    )]
    fn sliver_reorderable_list_asserts_on_both_non_null_item_extent_and_prototype_item() {
        let numbers = Rc::new(RefCell::new(vec![0, 1, 2]));
        let _ = SliverReorderableList::new(number_item(&numbers), 3)
            .item_extent(30.0)
            .prototype_item(SizedBox::new())
            .on_reorder_item(|_app, _from, _to| {})
            .into_widget();
    }

    /// The height of each of the extent cases' three items: Dart's
    /// `tester.getSize(find.text('$i').hitTestable()).height`, the one `Text` each shows.
    fn number_heights(tester: &WidgetTester) -> [f64; 3] {
        [0, 1, 2].map(|i| tester.get_size(find::text(&i.to_string())).height())
    }

    #[test]
    fn if_item_extent_is_non_null_children_have_same_extent_in_the_scroll_direction() {
        test_widgets(|tester| {
            let numbers = Rc::new(RefCell::new(vec![0, 1, 2]));

            let reorder_numbers = Rc::clone(&numbers);
            let builder_numbers = Rc::clone(&numbers);
            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new(
                move |_app, _context, _set_state| {
                    let reorder_numbers = Rc::clone(&reorder_numbers);
                    ReorderableList::new(number_item(&builder_numbers), 3)
                        .item_extent(30.0)
                        .on_reorder_item(move |_app, from_index, to_index| {
                            let mut numbers = reorder_numbers.borrow_mut();
                            let value = numbers.remove(from_index as usize);
                            numbers.insert(to_index as usize, value);
                        })
                        .into_widget()
                },
            )));

            assert_eq!(number_heights(tester), [30.0, 30.0, 30.0]);
        });
    }

    #[test]
    fn if_prototype_item_is_non_null_children_have_same_extent_in_the_scroll_direction() {
        test_widgets(|tester| {
            let numbers = Rc::new(RefCell::new(vec![0, 1, 2]));

            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new(
                move |_app, _context, _set_state| {
                    ReorderableList::new(number_item(&numbers), 3)
                        .prototype_item(SizedBox::new().height(30.0).child(Text::new("3")))
                        .on_reorder_item(|_app, _from, _to| {})
                        .into_widget()
                },
            )));

            assert_eq!(number_heights(tester), [30.0, 30.0, 30.0]);
        });
    }

    /// The listener cases: drag `item 0` down by 50 after `press`, with the listener kind and
    /// `enabled` given; answers the reorder calls and the items.
    fn drag_with_listener(
        tester: &mut WidgetTester,
        delayed: bool,
        enabled: bool,
        press: Duration,
        settle_first: bool,
    ) -> (i32, Vec<i32>) {
        let on_reorder_call_count = Rc::new(Cell::new(0));
        let items = items(5);

        // The list has five elements of height 100
        tester.pump_widget(
            TestWidgetsApp::new().home(
                ReorderableList::new(sized_item(&items, delayed, enabled), 5)
                    .on_reorder_item(handle_reorder(&items, &on_reorder_call_count)),
            ),
        );
        if settle_first {
            tester.pump_and_settle();
        }

        // Start gesture on first item
        let at = tester.get_center(find::text("item 0"));
        let mut drag = start_gesture(tester, at);
        tester.pump_by(press);

        // Drag enough to move down the first item
        tester.move_by(&mut drag, Offset::new(0.0, 50.0), true);
        tester.pump();
        tester.up(&drag);
        tester.pump_and_settle();

        let items = items.borrow().clone();
        (on_reorder_call_count.get(), items)
    }

    #[test]
    fn reorderable_drag_start_listener_it_should_allow_the_item_to_be_dragged_when_enabled_is_true()
    {
        test_widgets(|tester| {
            let (calls, items) = drag_with_listener(tester, false, true, K_PRESS_TIMEOUT, false);
            assert_eq!(calls, 1);
            assert_eq!(items, vec![1, 0, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_drag_start_listener_it_should_not_allow_the_item_to_be_dragged_when_enabled_is_false()
     {
        test_widgets(|tester| {
            let (calls, items) =
                drag_with_listener(tester, false, false, K_LONG_PRESS_TIMEOUT, false);
            assert_eq!(calls, 0);
            assert_eq!(items, vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_delayed_drag_start_listener_it_should_allow_the_item_to_be_dragged_when_enabled_is_true()
     {
        test_widgets(|tester| {
            let (calls, items) = drag_with_listener(tester, true, true, K_LONG_PRESS_TIMEOUT, true);
            assert_eq!(calls, 1);
            assert_eq!(items, vec![1, 0, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_delayed_drag_start_listener_it_should_not_allow_the_item_to_be_dragged_when_enabled_is_false()
     {
        test_widgets(|tester| {
            let (calls, items) =
                drag_with_listener(tester, true, false, K_LONG_PRESS_TIMEOUT, false);
            assert_eq!(calls, 0);
            assert_eq!(items, vec![0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn sliver_reorderable_list_properly_disposes_items() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/105010
            let item_count = 5;
            let items = items(item_count);
            let show_list = Rc::new(Cell::new(false));

            tester.pump_widget(TestWidgetsApp::new().home(StatefulBuilder::new(
                move |_app, _context, set_state| {
                    if !show_list.get() {
                        let show_list = Rc::clone(&show_list);
                        return GestureDetector::new()
                            .on_tap(Listener::new(move |app| {
                                set_state(app, &mut |_app| show_list.set(true));
                            }))
                            .child(Text::new("Show list"))
                            .into_widget();
                    }
                    let builder_items = Rc::clone(&items);
                    let show_list = Rc::clone(&show_list);
                    Column::new()
                        .children([
                            Expanded::new(
                                CustomScrollView::new().slivers([SliverReorderableList::new(
                                    move |_app, _context, index| {
                                        ReorderableDragStartListener::new(
                                            test_list_tile(Text::new(format!(
                                                "item {}",
                                                builder_items.borrow()[index as usize]
                                            ))),
                                            index,
                                        )
                                        .key(ValueKey::new(format!("item-{index}")).into())
                                        .into_widget()
                                    },
                                    item_count,
                                )
                                .on_reorder_item(|_app, _from, _to| {})
                                .into_widget()]),
                            )
                            .into_widget(),
                            GestureDetector::new()
                                .on_tap(Listener::new(move |app| {
                                    set_state(app, &mut |_app| show_list.set(false));
                                }))
                                .child(Text::new("Close list"))
                                .into_widget(),
                        ])
                        .into_widget()
                },
            )));

            tester.tap(find::text("Show list"));
            tester.pump_and_settle();

            let item_0 = || find::text("item 0");
            expect(tester, &item_0(), finds_one_widget());

            // Start gesture on first item without drag up event.
            let at = tester.get_center(item_0());
            let mut drag = start_gesture(tester, at);
            tester.move_by(&mut drag, Offset::new(0.0, 200.0), true);
            tester.pump();

            tester.tap(find::text("Close list"));
            tester.pump_and_settle();

            expect(tester, &item_0(), finds_nothing());
        });
    }

    #[test]
    fn null_check_error_when_dragging_and_dropping_last_element_into_last_index_with_reverse_true()
    {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/132077
            let item_count = 5;
            let items: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(
                (0..item_count)
                    .map(|index| format!("Item {}", index + 1))
                    .collect(),
            ));

            let reorder_items = Rc::clone(&items);
            let builder_items = Rc::clone(&items);
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    ReorderableList::new(
                        move |_app, _context, index| {
                            ReorderableDragStartListener::new(
                                test_list_tile(Text::new(
                                    builder_items.borrow()[index as usize].clone(),
                                )),
                                index,
                            )
                            .key(ValueKey::new(format!("{index}")).into())
                            .into_widget()
                        },
                        item_count,
                    )
                    .on_reorder_item(move |_app, old_index, new_index| {
                        let mut items = reorder_items.borrow_mut();
                        let item = items.remove(old_index as usize);
                        items.insert(new_index as usize, item);
                    })
                    .reverse(true),
                ),
            );

            // Start gesture on last item
            let at = tester.get_center(find::text("Item 5"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);

            // Drag to move up the last item, and drop at the last index
            tester.move_by(&mut drag, Offset::new(0.0, -50.0), true);
            tester.pump();
            tester.up(&drag);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn tests_that_the_item_position_is_correct_when_prototype_item_or_item_extent_are_set() {
        test_widgets(|tester| {
            fn test_for(
                tester: &mut WidgetTester,
                prototype_item: Option<WidgetRef>,
                item_extent: Option<f64>,
            ) {
                let mut list = SliverReorderableList::new(
                    |_app, _context, index| {
                        ReorderableDragStartListener::new(
                            SizedBox::new()
                                .height(100.0)
                                .child(Text::new(index.to_string())),
                            index,
                        )
                        .key(ValueKey::new(index).into())
                        .into_widget()
                    },
                    5,
                )
                .on_reorder_item(|_app, _from, _to| {});
                list.item_extent = item_extent;
                list.prototype_item = prototype_item;
                tester.pump_widget(
                    TestWidgetsApp::new()
                        .home(CustomScrollView::new().slivers([list.into_widget()])),
                );

                let at = tester.get_center(find::text("0"));
                let mut drag = start_gesture(tester, at);
                tester.pump_by(K_LONG_PRESS_TIMEOUT);
                tester.move_by(&mut drag, Offset::new(0.0, 20.0), true);
                tester.pump();
                assert_eq!(
                    tester.get_top_left(find::text("1")),
                    Offset::new(0.0, 100.0)
                );
                tester.up(&drag);
                tester.pump_and_settle();
            }

            test_for(tester, None, None);
            test_for(
                tester,
                Some(
                    SizedBox::new()
                        .height(100.0)
                        .width(100.0)
                        .child(Text::new("prototype"))
                        .into_widget(),
                ),
                None,
            );
            test_for(tester, None, Some(100.0));
        });
    }

    #[test]
    fn the_item_being_dragged_will_not_be_affected_by_layout_constraints() {
        test_widgets(|tester| {
            let item_layout_constraints: Rc<RefCell<HashMap<i32, BoxConstraints>>> = Rc::default();
            let recorded = Rc::clone(&item_layout_constraints);
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    CustomScrollView::new().slivers([SliverReorderableList::new(
                        move |_app, _context, index| {
                            let recorded = Rc::clone(&recorded);
                            LayoutBuilder::new(move |_app, _context, constraints| {
                                recorded
                                    .borrow_mut()
                                    .insert(index, constraints.constraints());
                                SizedBox::new()
                                    .height(100.0)
                                    .child(ReorderableDragStartListener::new(
                                        Text::new(index.to_string()),
                                        index,
                                    ))
                                    .into_widget()
                            })
                            .key(ValueKey::new(index).into())
                            .into_widget()
                        },
                        5,
                    )
                    .on_reorder_item(|_app, _from, _to| {})
                    .into_widget()]),
                ),
            );
            let pre_drag_layout_constraints = item_layout_constraints.borrow().clone();
            item_layout_constraints.borrow_mut().clear();
            let at = tester.get_center(find::text("0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);
            tester.move_by(&mut drag, Offset::new(0.0, 20.0), true);
            tester.pump();
            assert_eq!(
                *item_layout_constraints.borrow(),
                pre_drag_layout_constraints
            );
            tester.up(&drag);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn drag_boundary_defines_the_boundary_for_reorderable_list() {
        test_widgets(|tester| {
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                        Container::new()
                            .margin(EdgeInsetsGeometry::only(0.0, 100.0, 0.0, 0.0))
                            .height(300.0)
                            .child(DragBoundary::new(
                                CustomScrollView::new().slivers([SliverReorderableList::new(
                                    |_app, _context, index| {
                                        ReorderableDragStartListener::new(
                                            Text::new(index.to_string()),
                                            index,
                                        )
                                        .key(ValueKey::new(index).into())
                                        .into_widget()
                                    },
                                    5,
                                )
                                .on_reorder_item(|_app, _from, _to| {})
                                .into_widget()]),
                            )),
                    ),
                ),
            );
            let at = tester.get_center(find::text("0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);
            tester.move_by(&mut drag, Offset::new(0.0, -400.0), true);
            tester.pump_and_settle();
            assert_eq!(
                tester.get_top_left(find::text("0")),
                Offset::new(0.0, 100.0)
            );
            tester.up(&drag);
            tester.pump_and_settle();

            let at = tester.get_center(find::text("0"));
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);
            tester.move_by(&mut drag, Offset::new(0.0, 800.0), true);
            tester.pump_and_settle();
            assert_eq!(
                tester.get_bottom_left(find::text("0")),
                Offset::new(0.0, 400.0)
            );
            tester.up(&drag);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn reorderable_list_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    Center::new().child(
                        SizedBox::shrink().child(
                            ReorderableList::new(
                                |_app, _context, _index| {
                                    Text::new("X")
                                        .key(ValueKey::new("x".to_string()).into())
                                        .into_widget()
                                },
                                3,
                            )
                            .on_reorder_item(|_app, _from, _to| {}),
                        ),
                    ),
                ),
            );
            assert_eq!(
                tester.get_size(find::by_type::<ReorderableList>()),
                Size::ZERO
            );
        });
    }

    #[test]
    fn reorderable_drag_start_listener_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(TestWidgetsApp::new().home(Center::new().child(
                SizedBox::shrink().child(ReorderableDragStartListener::new(Text::new("X"), 1)),
            )));
            assert_eq!(
                tester.get_size(find::by_type::<ReorderableDragStartListener>()),
                Size::ZERO
            );
        });
    }

    #[test]
    fn reorderable_delayed_drag_start_listener_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(
                TestWidgetsApp::new().home(
                    Center::new().child(
                        SizedBox::shrink()
                            .child(ReorderableDelayedDragStartListener::new(Text::new("X"), 1)),
                    ),
                ),
            );
            assert_eq!(
                tester.get_size(find::by_type::<ReorderableDelayedDragStartListener>()),
                Size::ZERO
            );
        });
    }
}

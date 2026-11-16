//! Flutter counterpart: material_ui's `reorderable_list.dart`.

use std::any::{Any, TypeId};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use waft_animation::{AnimationRef, Curves};
use waft_embedder::{Clip, TargetPlatform, lerp_double};
use waft_foundation::{App, Handle, HandleId, Key, Listenable, Ref, ValueNotifier};
use waft_gestures::DragStartBehavior;
use waft_painting::{AlignmentDirectional, Axis, EdgeInsets, EdgeInsetsGeometry};
use waft_rendering::{ItemExtentBuilder, ScrollCacheExtent};
use waft_services::{MouseCursorRef, SystemMouseCursors};
use waft_widgets::{
    Align, AnimatedBuilder, BuildContext, CustomScrollView, Directionality, GlobalKey, Icon,
    IndexedWidgetBuilder, IntoWidget, KeyRef, KeyedSubtree, ListenableBuilder, MouseRegion,
    Positioned, ReorderCallback, ReorderDragBoundaryProvider, ReorderIndexCallback,
    ReorderItemProxyDecorator, ReorderableDelayedDragStartListener, ReorderableDragStartListener,
    ScrollControllerRef, ScrollPhysicsRef, ScrollViewKeyboardDismissBehavior, SliverPadding,
    SliverReorderableList, SliverToBoxAdapter, Stack, State, StateData, StatefulWidget, WidgetRef,
    WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStates,
    debug_check_has_overlay,
};

use crate::debug::debug_check_has_material_localizations;
use crate::icons::Icons;
use crate::material::Material;
use crate::theme::Theme;

/// A list whose items the user can interactively reorder by dragging.
///
/// This sample shows by dragging the user can reorder the items of the list. The
/// [`on_reorder_item`](Self::on_reorder_item) parameter will be called when a child widget is
/// dragged to a new position.
///
/// By default, on desktop platforms each item will have a drag handle added on top of it that
/// will allow the user to grab it to move the item. On mobile platforms, no drag handle will be
/// added, but when the user long presses anywhere on the item it will start moving the item.
/// Displaying drag handles can be controlled with
/// [`build_default_drag_handles`](Self::build_default_drag_handles).
///
/// All list items must have a key.
///
/// While a drag is underway, the widget returned by the
/// [`proxy_decorator`](Self::proxy_decorator) callback serves as a "proxy" (a substitute) for
/// the item in the list. The proxy is created with the original list item as its child. The
/// default one wraps the dragged item in a [`Material`] widget and animates its elevation.
pub struct ReorderableListView {
    pub key: Option<KeyRef>,

    /// See `ReorderableList::item_builder`.
    pub item_builder: IndexedWidgetBuilder,

    /// See `ReorderableList::item_count`.
    pub item_count: i32,

    /// See `ReorderableList::on_reorder`.
    ///
    /// Deprecated: use [`on_reorder_item`](Self::on_reorder_item).
    pub on_reorder: Option<ReorderCallback>,

    /// See `ReorderableList::on_reorder_item`.
    pub on_reorder_item: Option<ReorderCallback>,

    /// See `ReorderableList::on_reorder_start`.
    pub on_reorder_start: Option<ReorderIndexCallback>,

    /// See `ReorderableList::on_reorder_end`.
    pub on_reorder_end: Option<ReorderIndexCallback>,

    /// See `ReorderableList::proxy_decorator`.
    pub proxy_decorator: Option<ReorderItemProxyDecorator>,

    /// If true: on desktop platforms, a drag handle is stacked over the center of each item's
    /// trailing edge; on mobile platforms, a long press anywhere on the item starts a drag.
    ///
    /// The default desktop drag handle is just an [`Icons::drag_handle`] wrapped by a
    /// [`ReorderableDragStartListener`]. On mobile platforms, the entire item is wrapped with a
    /// [`ReorderableDelayedDragStartListener`].
    ///
    /// To change the appearance or the layout of the drag handles, make this parameter false and
    /// wrap each list item, or a widget within each list item, with
    /// [`ReorderableDragStartListener`] or [`ReorderableDelayedDragStartListener`].
    pub build_default_drag_handles: bool,

    /// See `ReorderableList::padding`.
    pub padding: Option<EdgeInsets>,

    /// A non-reorderable header item to show before the items of the list.
    ///
    /// If `None`, no header will appear before the list.
    pub header: Option<WidgetRef>,

    /// A non-reorderable footer item to show after the items of the list.
    ///
    /// If `None`, no footer will appear after the list.
    pub footer: Option<WidgetRef>,

    /// The `Axis` along which the scroll view's offset increases.
    pub scroll_direction: Axis,

    /// Whether the scroll view scrolls in the reading direction.
    pub reverse: bool,

    /// An object that can be used to control the position to which this scroll view is
    /// scrolled.
    pub scroll_controller: Option<ScrollControllerRef>,

    /// Whether this is the primary scroll view associated with the parent
    /// `PrimaryScrollController`.
    ///
    /// Defaults to true when [`scroll_direction`](Self::scroll_direction) is [`Axis::Vertical`]
    /// and [`scroll_controller`](Self::scroll_controller) is `None`.
    pub primary: Option<bool>,

    /// How the scroll view should respond to user input.
    pub physics: Option<ScrollPhysicsRef>,

    /// Whether the extent of the scroll view in the scroll direction should be determined by
    /// the contents being viewed.
    pub shrink_wrap: bool,

    /// The relative position of the zero scroll offset.
    ///
    /// For more information, see `ScrollView::anchor`.
    pub anchor: f64,

    /// The viewport has an area before and after the visible area to cache items that are about
    /// to become visible when the user scrolls.
    ///
    /// Deprecated: use [`scroll_cache_extent`](Self::scroll_cache_extent) instead.
    pub cache_extent: Option<f64>,

    /// The amount of additional content to display and lay out around the viewport.
    pub scroll_cache_extent: Option<ScrollCacheExtent>,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// How this scroll view dismisses the keyboard automatically.
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

    /// See `ReorderableList::item_extent`.
    pub item_extent: Option<f64>,

    /// See `ReorderableList::item_extent_builder`.
    pub item_extent_builder: Option<ItemExtentBuilder>,

    /// See `ReorderableList::prototype_item`.
    pub prototype_item: Option<WidgetRef>,

    /// The velocity scalar per pixel over scroll of the auto scroller.
    ///
    /// Defaults to 50 if not set or set to `None`.
    pub auto_scroller_velocity_scalar: Option<f64>,

    /// See `ReorderableList::drag_boundary_provider`.
    pub drag_boundary_provider: Option<ReorderDragBoundaryProvider>,

    /// The cursor for a mouse pointer when it enters or is hovering over the drag handle.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Dragged`].
    ///
    /// If this property is `None`, [`SystemMouseCursors::GRAB`] will be used when hovering, and
    /// [`SystemMouseCursors::GRABBING`] when dragging.
    pub mouse_cursor: Option<MouseCursorRef>,
}

impl ReorderableListView {
    /// Creates a reorderable list from a pre-built list of widgets.
    ///
    /// This constructor is appropriate for lists with a small number of children because
    /// constructing the list requires doing work for every child that could possibly be
    /// displayed in the list view instead of just those children that are actually visible.
    ///
    /// See also:
    ///
    ///   * [`builder`](Self::builder), which allows you to build a reorderable list where the
    ///     items are built as needed when scrolling the list.
    pub fn new(children: impl IntoIterator<Item = WidgetRef>) -> ReorderableListView {
        let children: Vec<WidgetRef> = children.into_iter().collect();
        debug_assert!(
            children.iter().all(|child| child.key().is_some()),
            "All children of this widget must have a key."
        );
        let item_count = children.len() as i32;
        ReorderableListView::from_item_builder(
            Rc::new(move |_app, _context, index| children[index as usize].clone()),
            item_count,
        )
    }

    /// Creates a reorderable list from widget items that are created on demand.
    ///
    /// This constructor is appropriate for list views with a large number of children because
    /// the builder is called only for those children that are actually visible.
    ///
    /// The `item_builder` callback will be called only with indices greater than or equal to
    /// zero and less than `item_count`.
    ///
    /// The `item_builder` should always return a widget, and actually create the widget
    /// instances when called. Avoid using a builder that returns a previously-constructed
    /// widget; if the list view's children are created in advance, or all at once when the
    /// [`ReorderableListView`] itself is created, it is more efficient to use
    /// [`new`](Self::new). Even more efficient, however, is to create the instances on demand
    /// using this constructor's `item_builder` callback.
    ///
    /// See also:
    ///
    ///   * [`new`](Self::new), which allows you to build a reorderable list with all the items
    ///     passed into the constructor.
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> WidgetRef + 'static,
        item_count: i32,
    ) -> ReorderableListView {
        debug_assert!(item_count >= 0);
        ReorderableListView::from_item_builder(Rc::new(item_builder), item_count)
    }

    /// The fields both of Dart's constructors give their defaults.
    fn from_item_builder(
        item_builder: IndexedWidgetBuilder,
        item_count: i32,
    ) -> ReorderableListView {
        ReorderableListView {
            key: None,
            item_builder,
            item_count,
            on_reorder: None,
            on_reorder_item: None,
            on_reorder_start: None,
            on_reorder_end: None,
            proxy_decorator: None,
            build_default_drag_handles: true,
            padding: None,
            header: None,
            footer: None,
            scroll_direction: Axis::Vertical,
            reverse: false,
            scroll_controller: None,
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
            mouse_cursor: None,
        }
    }

    /// Dart `ReorderableListView(key:)`.
    pub fn key(mut self, key: KeyRef) -> ReorderableListView {
        self.key = Some(key);
        self
    }

    /// Dart `ReorderableListView(onReorder:)`.
    #[deprecated(
        note = "Use the on_reorder_item callback instead. The on_reorder_item callback adjusts \
                the new_index parameter for a removed item at the old_index."
    )]
    pub fn on_reorder(
        mut self,
        on_reorder: impl Fn(&mut App, i32, i32) + 'static,
    ) -> ReorderableListView {
        self.on_reorder = Some(Rc::new(on_reorder));
        self
    }

    /// Dart `ReorderableListView(onReorderItem:)`.
    pub fn on_reorder_item(
        mut self,
        on_reorder_item: impl Fn(&mut App, i32, i32) + 'static,
    ) -> ReorderableListView {
        self.on_reorder_item = Some(Rc::new(on_reorder_item));
        self
    }

    /// Dart `ReorderableListView(onReorderStart:)`.
    pub fn on_reorder_start(
        mut self,
        on_reorder_start: impl Fn(&mut App, i32) + 'static,
    ) -> ReorderableListView {
        self.on_reorder_start = Some(Rc::new(on_reorder_start));
        self
    }

    /// Dart `ReorderableListView(onReorderEnd:)`.
    pub fn on_reorder_end(
        mut self,
        on_reorder_end: impl Fn(&mut App, i32) + 'static,
    ) -> ReorderableListView {
        self.on_reorder_end = Some(Rc::new(on_reorder_end));
        self
    }

    /// Dart `ReorderableListView(itemExtent:)`.
    pub fn item_extent(mut self, item_extent: f64) -> ReorderableListView {
        self.item_extent = Some(item_extent);
        self
    }

    /// Dart `ReorderableListView(itemExtentBuilder:)`.
    pub fn item_extent_builder(
        mut self,
        item_extent_builder: ItemExtentBuilder,
    ) -> ReorderableListView {
        self.item_extent_builder = Some(item_extent_builder);
        self
    }

    /// Dart `ReorderableListView(prototypeItem:)`.
    pub fn prototype_item<K>(mut self, prototype_item: impl IntoWidget<K>) -> ReorderableListView {
        self.prototype_item = Some(prototype_item.into_widget());
        self
    }

    /// Dart `ReorderableListView(proxyDecorator:)`.
    pub fn proxy_decorator(
        mut self,
        proxy_decorator: impl Fn(&mut App, WidgetRef, i32, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> ReorderableListView {
        self.proxy_decorator = Some(Rc::new(proxy_decorator));
        self
    }

    /// Dart `ReorderableListView(buildDefaultDragHandles:)`.
    pub fn build_default_drag_handles(
        mut self,
        build_default_drag_handles: bool,
    ) -> ReorderableListView {
        self.build_default_drag_handles = build_default_drag_handles;
        self
    }

    /// Dart `ReorderableListView(padding:)`.
    pub fn padding(mut self, padding: EdgeInsets) -> ReorderableListView {
        self.padding = Some(padding);
        self
    }

    /// Dart `ReorderableListView(header:)`.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> ReorderableListView {
        self.header = Some(header.into_widget());
        self
    }

    /// Dart `ReorderableListView(footer:)`.
    pub fn footer<K>(mut self, footer: impl IntoWidget<K>) -> ReorderableListView {
        self.footer = Some(footer.into_widget());
        self
    }

    /// Dart `ReorderableListView(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> ReorderableListView {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Dart `ReorderableListView(reverse:)`.
    pub fn reverse(mut self, reverse: bool) -> ReorderableListView {
        self.reverse = reverse;
        self
    }

    /// Dart `ReorderableListView(scrollController:)`.
    pub fn scroll_controller(
        mut self,
        scroll_controller: ScrollControllerRef,
    ) -> ReorderableListView {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `ReorderableListView(primary:)`.
    pub fn primary(mut self, primary: bool) -> ReorderableListView {
        self.primary = Some(primary);
        self
    }

    /// Dart `ReorderableListView(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> ReorderableListView {
        self.physics = Some(physics);
        self
    }

    /// Dart `ReorderableListView(shrinkWrap:)`.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> ReorderableListView {
        self.shrink_wrap = shrink_wrap;
        self
    }

    /// Dart `ReorderableListView(anchor:)`.
    pub fn anchor(mut self, anchor: f64) -> ReorderableListView {
        self.anchor = anchor;
        self
    }

    /// Dart `ReorderableListView(cacheExtent:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent(mut self, cache_extent: f64) -> ReorderableListView {
        self.cache_extent = Some(cache_extent);
        self
    }

    /// Dart `ReorderableListView(scrollCacheExtent:)`.
    pub fn scroll_cache_extent(
        mut self,
        scroll_cache_extent: ScrollCacheExtent,
    ) -> ReorderableListView {
        self.scroll_cache_extent = Some(scroll_cache_extent);
        self
    }

    /// Dart `ReorderableListView(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> ReorderableListView {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `ReorderableListView(keyboardDismissBehavior:)`.
    pub fn keyboard_dismiss_behavior(
        mut self,
        keyboard_dismiss_behavior: ScrollViewKeyboardDismissBehavior,
    ) -> ReorderableListView {
        self.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
        self
    }

    /// Dart `ReorderableListView(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> ReorderableListView {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `ReorderableListView(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ReorderableListView {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ReorderableListView(autoScrollerVelocityScalar:)`.
    pub fn auto_scroller_velocity_scalar(
        mut self,
        auto_scroller_velocity_scalar: f64,
    ) -> ReorderableListView {
        self.auto_scroller_velocity_scalar = Some(auto_scroller_velocity_scalar);
        self
    }

    /// Dart `ReorderableListView(dragBoundaryProvider:)`.
    pub fn drag_boundary_provider(
        mut self,
        drag_boundary_provider: ReorderDragBoundaryProvider,
    ) -> ReorderableListView {
        self.drag_boundary_provider = Some(drag_boundary_provider);
        self
    }

    /// Dart `ReorderableListView(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> ReorderableListView {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }
}

impl fmt::Debug for ReorderableListView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReorderableListView")
            .field("key", &self.key)
            .field("item_count", &self.item_count)
            .field("scroll_direction", &self.scroll_direction)
            .field("reverse", &self.reverse)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ReorderableListView {
    type State = ReorderableListViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            (self.item_extent.is_none() && self.prototype_item.is_none())
                || (self.item_extent.is_none() && self.item_extent_builder.is_none())
                || (self.prototype_item.is_none() && self.item_extent_builder.is_none()),
            "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
        );
        debug_assert!(
            (self.on_reorder_item.is_some() && self.on_reorder.is_none())
                || (self.on_reorder_item.is_none() && self.on_reorder.is_some()),
            "The onReorder callback is obsolete and is replaced by onReorderItem. Remove the \
             onReorder callback when both callbacks are provided."
        );
    }

    fn create_state(&self) -> ReorderableListViewState {
        ReorderableListViewState {
            state: StateData::new(),
            dragging: None,
        }
    }
}

/// Dart's `_ReorderableListViewState`.
pub struct ReorderableListViewState {
    state: StateData<ReorderableListView>,
    /// Created in `init_state`, because a state is created without the [`App`].
    dragging: Option<Ref<ValueNotifier<bool>>>,
}

impl ReorderableListViewState {
    fn dragging(self: Handle<Self>, app: &App) -> Ref<ValueNotifier<bool>> {
        app.get(self)
            .dragging
            .clone()
            .expect("created in init_state")
    }

    fn item_builder(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: i32,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let item_builder = widget.item_builder.clone();
        let build_default_drag_handles = widget.build_default_drag_handles;
        let scroll_direction = widget.scroll_direction;
        let mouse_cursor = widget.mouse_cursor.clone();
        let item = item_builder(app, context, index);
        if cfg!(debug_assertions) && item.key().is_none() {
            panic!("Every item of ReorderableListView must have a key.");
        }

        let item_global_key: KeyRef =
            GlobalKey::wrapping(Rc::new(ReorderableListViewChildGlobalKey {
                sub_key: item
                    .key()
                    .expect("Every item of ReorderableListView must have a key.")
                    .clone(),
                state: self.id(),
            }))
            .into();

        if build_default_drag_handles {
            match Theme::of(app, context).platform {
                TargetPlatform::Linux | TargetPlatform::Windows | TargetPlatform::MacOS => {
                    let dragging = self.dragging(app);
                    let drag_handle = ListenableBuilder::new(
                        Rc::new(dragging.clone()),
                        move |app, _context, child| {
                            let mouse_cursor: MouseCursorRef =
                                mouse_cursor.clone().unwrap_or_else(|| {
                                    Rc::new(WidgetStateMouseCursor::from_map(vec![
                                        (
                                            WidgetState::Dragged.into(),
                                            SystemMouseCursors::GRABBING.into(),
                                        ),
                                        (WidgetState::any(), SystemMouseCursors::GRAB.into()),
                                    ]))
                                });
                            let mut states = WidgetStates::new();
                            if *app.get(&dragging).value() {
                                states.insert(WidgetState::Dragged);
                            }
                            let effective_mouse_cursor =
                                <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                                    &mouse_cursor,
                                    &states,
                                );
                            let mut region = MouseRegion::new().cursor(effective_mouse_cursor);
                            if let Some(child) = child {
                                region = region.child(child.clone());
                            }
                            region.into_widget()
                        },
                    )
                    .child(Icon::new(Some(Icons::drag_handle())))
                    .into_widget();
                    let text_direction = Directionality::of(app, context);
                    return match scroll_direction {
                        Axis::Horizontal => Stack::new()
                            .key(item_global_key)
                            .children([
                                item,
                                Positioned::directional(
                                    text_direction,
                                    Some(0.0),
                                    Some(0.0),
                                    Align::new()
                                        .alignment(AlignmentDirectional::BOTTOM_CENTER.into())
                                        .child(ReorderableDragStartListener::new(
                                            drag_handle,
                                            index,
                                        )),
                                )
                                .bottom(8.0)
                                .into_widget(),
                            ])
                            .into_widget(),
                        Axis::Vertical => Stack::new()
                            .key(item_global_key)
                            .children([
                                item,
                                Positioned::directional(
                                    text_direction,
                                    None,
                                    Some(8.0),
                                    Align::new()
                                        .alignment(AlignmentDirectional::CENTER_END.into())
                                        .child(ReorderableDragStartListener::new(
                                            drag_handle,
                                            index,
                                        )),
                                )
                                .top(0.0)
                                .bottom(0.0)
                                .into_widget(),
                            ])
                            .into_widget(),
                    };
                }

                TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => {
                    return ReorderableDelayedDragStartListener::new(item, index)
                        .key(item_global_key)
                        .into_widget();
                }
            }
        }

        KeyedSubtree::new(item).key(item_global_key).into_widget()
    }

    fn proxy_decorator(
        _app: &mut App,
        child: WidgetRef,
        _index: i32,
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let listenable: Rc<dyn Listenable> = Rc::new(animation.clone());
        AnimatedBuilder::new(listenable, move |app, _context, child| {
            let anim_value = Curves::ease_in_out().transform(animation.value(app));
            let elevation =
                lerp_double(Some(0.0), Some(6.0), anim_value).expect("both ends are given");
            let mut material = Material::new().elevation(elevation);
            if let Some(child) = child {
                material = material.child(child.clone());
            }
            material.into_widget()
        })
        .child(child)
        .into_widget()
    }
}

impl State for ReorderableListViewState {
    type Widget = ReorderableListView;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let dragging = app.create(ValueNotifier::new(false));
        app.get_mut(self).dragging = Some(dragging);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let dragging = self.dragging(app);
        dragging.dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(debug_check_has_overlay(app, context));

        let widget = self.widget(app);
        let scroll_direction = widget.scroll_direction;
        let reverse = widget.reverse;
        let item_extent = widget.item_extent;
        let item_extent_builder = widget.item_extent_builder.clone();
        let prototype_item = widget.prototype_item.clone();
        let item_count = widget.item_count;
        let on_reorder = widget.on_reorder.clone();
        let on_reorder_item = widget.on_reorder_item.clone();
        let on_reorder_start = widget.on_reorder_start.clone();
        let on_reorder_end = widget.on_reorder_end.clone();
        let proxy_decorator = widget.proxy_decorator.clone();
        let auto_scroller_velocity_scalar = widget.auto_scroller_velocity_scalar;
        let drag_boundary_provider = widget.drag_boundary_provider.clone();
        let header = widget.header.clone();
        let footer = widget.footer.clone();
        let scroll_controller = widget.scroll_controller.clone();
        let primary = widget.primary;
        let physics = widget.physics.clone();
        let shrink_wrap = widget.shrink_wrap;
        let anchor = widget.anchor;
        let drag_start_behavior = widget.drag_start_behavior;
        let keyboard_dismiss_behavior = widget.keyboard_dismiss_behavior;
        let restoration_id = widget.restoration_id.clone();
        let clip_behavior = widget.clip_behavior;

        // If there is a header or footer we can't just apply the padding to the list,
        // so we break it up into padding for the header, footer and padding for the list.
        let padding = widget.padding.unwrap_or(EdgeInsets::ZERO);
        let mut start = header.as_ref().map(|_| 0.0);
        let mut end = footer.as_ref().map(|_| 0.0);
        if reverse {
            (start, end) = (end, start);
        }

        let (start_padding, end_padding, list_padding) = match scroll_direction {
            Axis::Horizontal | Axis::Vertical if start.or(end).is_none() => {
                (EdgeInsets::ZERO, EdgeInsets::ZERO, padding)
            }
            Axis::Horizontal => (
                padding.copy_with(Some(0.0), None, None, None),
                padding.copy_with(None, None, Some(0.0), None),
                padding.copy_with(start, None, end, None),
            ),
            Axis::Vertical => (
                padding.copy_with(None, Some(0.0), None, None),
                padding.copy_with(None, None, None, Some(0.0)),
                padding.copy_with(None, start, None, end),
            ),
        };
        let (header_padding, footer_padding) = if reverse {
            (start_padding, end_padding)
        } else {
            (end_padding, start_padding)
        };

        let scroll_cache_extent = widget
            .scroll_cache_extent
            .or_else(|| widget.cache_extent.map(ScrollCacheExtent::Pixels));

        let mut list = SliverReorderableList::from_builder(
            Rc::new(move |app, context, index| self.item_builder(app, context, index)),
            item_count,
        );
        list.item_extent = item_extent;
        list.item_extent_builder = item_extent_builder;
        list.prototype_item = prototype_item;
        list.on_reorder = on_reorder;
        list.on_reorder_item = on_reorder_item;
        list.on_reorder_start = Some(Rc::new(move |app, index| {
            self.dragging(app).set_value(app, true);
            if let Some(on_reorder_start) = &on_reorder_start {
                on_reorder_start(app, index);
            }
        }));
        list.on_reorder_end = Some(Rc::new(move |app, index| {
            self.dragging(app).set_value(app, false);
            if let Some(on_reorder_end) = &on_reorder_end {
                on_reorder_end(app, index);
            }
        }));
        list.proxy_decorator = Some(
            proxy_decorator.unwrap_or_else(|| Rc::new(ReorderableListViewState::proxy_decorator)),
        );
        if let Some(auto_scroller_velocity_scalar) = auto_scroller_velocity_scalar {
            list = list.auto_scroller_velocity_scalar(auto_scroller_velocity_scalar);
        }
        list.drag_boundary_provider = drag_boundary_provider;

        let mut slivers = Vec::new();
        if let Some(header) = header {
            slivers.push(
                SliverPadding::new(EdgeInsetsGeometry::from(header_padding))
                    .sliver(SliverToBoxAdapter::new().child(header))
                    .into_widget(),
            );
        }
        slivers.push(
            SliverPadding::new(EdgeInsetsGeometry::from(list_padding))
                .sliver(list)
                .into_widget(),
        );
        if let Some(footer) = footer {
            slivers.push(
                SliverPadding::new(EdgeInsetsGeometry::from(footer_padding))
                    .sliver(SliverToBoxAdapter::new().child(footer))
                    .into_widget(),
            );
        }

        let mut scroll_view = CustomScrollView::new()
            .scroll_direction(scroll_direction)
            .reverse(reverse);
        if let Some(scroll_controller) = scroll_controller {
            scroll_view = scroll_view.controller(scroll_controller);
        }
        if let Some(primary) = primary {
            scroll_view = scroll_view.primary(primary);
        }
        if let Some(physics) = physics {
            scroll_view = scroll_view.physics(physics);
        }
        scroll_view = scroll_view.shrink_wrap(shrink_wrap).anchor(anchor);
        if let Some(scroll_cache_extent) = scroll_cache_extent {
            scroll_view = scroll_view.scroll_cache_extent(scroll_cache_extent);
        }
        scroll_view = scroll_view.drag_start_behavior(drag_start_behavior);
        if let Some(keyboard_dismiss_behavior) = keyboard_dismiss_behavior {
            scroll_view = scroll_view.keyboard_dismiss_behavior(keyboard_dismiss_behavior);
        }
        if let Some(restoration_id) = restoration_id {
            scroll_view = scroll_view.restoration_id(restoration_id);
        }
        scroll_view
            .clip_behavior(clip_behavior)
            .slivers(slivers)
            .into_widget()
    }
}

impl fmt::Debug for ReorderableListViewState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReorderableListViewState")
            .field("dragging", &self.dragging)
            .finish_non_exhaustive()
    }
}

/// Dart's `_ReorderableListViewChildGlobalKey`: a global key that takes its identity from the
/// object and uses a value of a particular type to identify itself.
///
/// The difference with `GlobalObjectKey` is that it uses `==` instead of `identical` of the
/// objects used to generate widgets.
#[derive(Debug)]
struct ReorderableListViewChildGlobalKey {
    sub_key: KeyRef,
    state: HandleId,
}

impl Key for ReorderableListViewChildGlobalKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<ReorderableListViewChildGlobalKey>()
            .is_some_and(|other| other.sub_key == self.sub_key && other.state == self.state)
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<ReorderableListViewChildGlobalKey>().hash(&mut state);
        self.sub_key.hash(&mut state);
        self.state.hash(&mut state);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::time::Duration;

    use indexmap::IndexMap;
    use waft_animation::Curves;
    use waft_embedder::{Locale, Offset, Rect, Size, TextDirection};
    use waft_foundation::{ValueKey, VoidCallback};
    use waft_gestures::{K_LONG_PRESS_TIMEOUT, K_PRESS_TIMEOUT, K_PRIMARY_BUTTON};
    use waft_rendering::{
        CrossAxisAlignment, CustomSemanticsAction, RenderSliver, RenderSliverList,
    };
    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_n_widgets,
        finds_nothing, finds_one_widget, test_widgets, test_widgets_variant,
    };
    use waft_widgets::{
        Center, Column, Container, DefaultWidgetsLocalizations, DragBoundary, Flexible,
        Localizations, Overlay, OverlayEntry, PrimaryScrollController, Row, ScrollController,
        Scrollable, Semantics, SizedBox, SliverList, StateSetter, StatefulBuilder, Text,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::checkbox::Checkbox;
    use crate::list_tile::ListTile;
    use crate::material_localizations::DefaultMaterialLocalizations;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    const ITEM_HEIGHT: f64 = 48.0;
    const ORIGINAL_LIST_ITEMS: [&str; 4] = ["Item 1", "Item 2", "Item 3", "Item 4"];

    /// Dart's `late List<String> listItems`.
    type ListItems = Rc<RefCell<Vec<String>>>;

    /// Dart's `listItems = originalListItems.toList()`.
    fn original_list_items() -> ListItems {
        Rc::new(RefCell::new(
            ORIGINAL_LIST_ITEMS
                .iter()
                .map(ToString::to_string)
                .collect(),
        ))
    }

    fn items_are(items: &ListItems, expected: &[&str]) {
        assert_eq!(*items.borrow(), expected);
    }

    /// Dart's `Key(listItem)`.
    fn string_key(value: &str) -> KeyRef {
        ValueKey::new(value.to_string()).into()
    }

    /// Dart's `onReorderItem`: `listItems.insert(newIndex, listItems.removeAt(oldIndex))`.
    fn on_reorder_item(items: &ListItems) -> impl Fn(&mut App, i32, i32) + 'static {
        let items = Rc::clone(items);
        move |_app, old_index, new_index| {
            let mut items = items.borrow_mut();
            let element = items.remove(old_index as usize);
            items.insert(new_index as usize, element);
        }
    }

    /// Dart's `listItemToWidget`.
    fn list_item_to_widget(list_item: &str) -> WidgetRef {
        SizedBox::new()
            .key(string_key(list_item))
            .height(ITEM_HEIGHT)
            .width(ITEM_HEIGHT)
            .child(Text::new(list_item))
            .into_widget()
    }

    /// The arguments of Dart's `build({..})`.
    struct BuildOptions {
        header: Option<WidgetRef>,
        footer: Option<WidgetRef>,
        scroll_direction: Axis,
        reverse: bool,
        padding: EdgeInsets,
        text_direction: TextDirection,
        platform: Option<TargetPlatform>,
    }

    impl Default for BuildOptions {
        fn default() -> BuildOptions {
            BuildOptions {
                header: None,
                footer: None,
                scroll_direction: Axis::Vertical,
                reverse: false,
                padding: EdgeInsets::ZERO,
                text_direction: TextDirection::Ltr,
                platform: None,
            }
        }
    }

    /// Dart's `build({..})`.
    fn build(tester: &WidgetTester, items: &ListItems, options: BuildOptions) -> WidgetRef {
        let theme = {
            let app = tester.cell.borrow();
            let mut theme = ThemeData::builder();
            if let Some(platform) = options.platform {
                theme = theme.platform(platform);
            }
            theme.build(&app)
        };
        let mut list =
            ReorderableListView::new(items.borrow().iter().map(|item| list_item_to_widget(item)))
                .scroll_direction(options.scroll_direction)
                .on_reorder_item(on_reorder_item(items))
                .reverse(options.reverse)
                .padding(options.padding);
        if let Some(header) = options.header {
            list = list.header(header);
        }
        if let Some(footer) = options.footer {
            list = list.footer(footer);
        }
        material_app(
            theme,
            Directionality::new(
                options.text_direction,
                SizedBox::square(Some(ITEM_HEIGHT * 10.0)).child(list),
            )
            .into_widget(),
        )
    }

    fn build_default(tester: &WidgetTester, items: &ListItems) -> WidgetRef {
        build(tester, items, BuildOptions::default())
    }

    /// Dart's `tester.startGesture(at)`.
    fn start_gesture(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        let mut pointer = TestPointer::touch(tester.next_pointer());
        tester.down(&mut pointer, at);
        pointer
    }

    /// Dart's `longPressDrag`.
    fn long_press_drag(tester: &mut WidgetTester, start: Offset, end: Offset) {
        let mut drag = start_gesture(tester, start);
        tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
        tester.move_to(&mut drag, end, true);
        tester.pump_by(K_PRESS_TIMEOUT);
        tester.up(&drag);
    }

    /// Dart's `tester.pumpWidget(MaterialApp(home: home))`.
    fn pump_home<K>(tester: &mut WidgetTester, home: impl IntoWidget<K>) {
        tester.pump_widget(MaterialApp::new().home(home));
    }

    fn panic_message(panic: Box<dyn Any + Send>) -> String {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(ToString::to_string))
            .unwrap_or_default()
    }

    /// Dart's `ValueKey<int>(index)` items of 100 pixels: `ReorderableDragStartListener(index:,
    /// child: Text('item $item'))` over the numbers in `items`.
    fn numbered_children(items: &Rc<RefCell<Vec<i32>>>) -> Vec<WidgetRef> {
        let count = items.borrow().len() as i32;
        (0..count)
            .map(|index| numbered_item(items, index))
            .collect()
    }

    fn numbered_item(items: &Rc<RefCell<Vec<i32>>>, index: i32) -> WidgetRef {
        let item = items.borrow()[index as usize];
        SizedBox::new()
            .key(ValueKey::new(item).into())
            .height(100.0)
            .child(ReorderableDragStartListener::new(
                Text::new(format!("item {item}")),
                index,
            ))
            .into_widget()
    }

    // The group 'in vertical mode' and 'in horizontal mode' run the same cases on two axes.
    // `Axis::Vertical` offsets an item's centre by `(0, d)`, `Axis::Horizontal` by `(d, 0)`.
    fn along(axis: Axis, distance: f64) -> Offset {
        match axis {
            Axis::Vertical => Offset::new(0.0, distance),
            Axis::Horizontal => Offset::new(distance, 0.0),
        }
    }

    fn options_along(axis: Axis) -> BuildOptions {
        BuildOptions {
            scroll_direction: axis,
            ..BuildOptions::default()
        }
    }

    fn reorder_is_not_triggered_when_children_length_is_one(axis: Axis) {
        test_widgets(|tester| {
            let on_reorder_was_called = Rc::new(Cell::new(false));
            let current_list_items = vec!["Item 1".to_string()];
            let list = ReorderableListView::new(
                current_list_items
                    .iter()
                    .map(|item| list_item_to_widget(item)),
            )
            .header(Text::new("Header"))
            .scroll_direction(axis)
            .on_reorder_item({
                let on_reorder_was_called = Rc::clone(&on_reorder_was_called);
                move |_app, _old, _new| on_reorder_was_called.set(true)
            });
            pump_home(
                tester,
                SizedBox::new().height(ITEM_HEIGHT * 10.0).child(list),
            );
            let center = tester.get_center(find::text("Item 1"));
            let mut drag = start_gesture(tester, center);
            tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
            let bottom_left = tester.get_bottom_left(find::text("Item 1"));
            tester.move_to(&mut drag, bottom_left * 2.0, true);
            tester.up(&drag);
            assert!(!on_reorder_was_called.get());
            assert_eq!(current_list_items, ["Item 1"]);
        });
    }

    fn drag_between_items(tester: &mut WidgetTester, from: &str, to: &str, past: Option<Offset>) {
        let start = tester.get_center(find::text(from));
        let end = tester.get_center(find::text(to)) + past.unwrap_or(Offset::ZERO);
        long_press_drag(tester, start, end);
        tester.pump_and_settle();
    }

    mod in_vertical_mode {
        use super::*;

        #[test]
        fn reorder_is_not_triggered_when_children_length_is_less_or_equals_to_1() {
            reorder_is_not_triggered_when_children_length_is_one(Axis::Vertical);
        }

        #[test]
        fn reorders_its_contents_only_when_a_drag_finishes() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build_default(tester, &items));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                let center = tester.get_center(find::text("Item 1"));
                let mut drag = start_gesture(tester, center);
                tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                let item_4 = tester.get_center(find::text("Item 4"));
                tester.move_to(&mut drag, item_4, true);
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                tester.up(&drag);
                tester.pump_and_settle();
                items_are(&items, &["Item 2", "Item 3", "Item 1", "Item 4"]);
            });
        }

        #[test]
        fn allows_reordering_from_the_very_top_to_the_very_bottom() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build_default(tester, &items));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(Offset::new(0.0, ITEM_HEIGHT * 2.0)),
                );
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
            });
        }

        #[test]
        fn allows_reordering_from_the_very_bottom_to_the_very_top() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build_default(tester, &items));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(tester, "Item 4", "Item 1", None);
                items_are(&items, &["Item 4", "Item 1", "Item 2", "Item 3"]);
            });
        }

        #[test]
        fn allows_reordering_inside_the_middle_of_the_widget() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build_default(tester, &items));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(tester, "Item 3", "Item 2", None);
                items_are(&items, &["Item 1", "Item 3", "Item 2", "Item 4"]);
            });
        }

        #[test]
        fn properly_reorders_with_a_header() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build(
                    tester,
                    &items,
                    BuildOptions {
                        header: Some(Text::new("Header Text").into_widget()),
                        ..BuildOptions::default()
                    },
                ));
                expect(tester, &find::text("Header Text"), finds_one_widget());
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(Offset::new(0.0, ITEM_HEIGHT * 2.0)),
                );
                expect(tester, &find::text("Header Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
            });
        }

        #[test]
        fn properly_reorders_with_a_footer() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build(
                    tester,
                    &items,
                    BuildOptions {
                        footer: Some(Text::new("Footer Text").into_widget()),
                        ..BuildOptions::default()
                    },
                ));
                expect(tester, &find::text("Footer Text"), finds_one_widget());
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(Offset::new(0.0, ITEM_HEIGHT * 2.0)),
                );
                expect(tester, &find::text("Footer Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
            });
        }

        #[test]
        fn properly_determines_the_vertical_drop_area_extents() {
            drop_area_extents(Axis::Vertical);
        }

        #[test]
        fn preserves_children_states_when_the_list_parent_changes_the_order() {
            check_preserves_children_states_when_the_list_parent_changes_the_order(Axis::Vertical);
        }

        #[test]
        fn preserves_children_states_when_rebuilt() {
            check_preserves_children_states_when_rebuilt(Axis::Vertical);
        }

        #[test]
        fn uses_the_primary_scroll_controller_when_available() {
            test_widgets(|tester| {
                let primary = ScrollController::new(
                    &mut tester.cell.borrow_mut(),
                    0.0,
                    true,
                    None,
                    None,
                    None,
                );
                let reorderable_list = || {
                    ReorderableListView::new(["C", "B", "A"].map(|label| {
                        SizedBox::new()
                            .width(100.0)
                            .height(100.0)
                            .key(string_key(label))
                            .child(Text::new(label))
                            .into_widget()
                    }))
                    .on_reorder_item(|_app, _old, _new| {})
                };
                let build_with_scroll_controller = |controller: &Ref<ScrollController>| {
                    MaterialApp::new().home(PrimaryScrollController::new(
                        controller.erase(),
                        SizedBox::new()
                            .height(100.0)
                            .width(100.0)
                            .child(reorderable_list()),
                    ))
                };
                tester.pump_widget(build_with_scroll_controller(&primary));
                let scroll_view_controller = tester
                    .widget(find::by_type::<Scrollable>(), |scrollable: &Scrollable| {
                        scrollable.controller.clone()
                    });
                assert_eq!(scroll_view_controller, Some(primary.erase()));

                // Now try changing the primary scroll controller and checking that the scroll
                // view gets updated.
                let primary2 = ScrollController::new(
                    &mut tester.cell.borrow_mut(),
                    0.0,
                    true,
                    None,
                    None,
                    None,
                );
                tester.pump_widget(build_with_scroll_controller(&primary2));
                let scroll_view_controller = tester
                    .widget(find::by_type::<Scrollable>(), |scrollable: &Scrollable| {
                        scrollable.controller.clone()
                    });
                assert_eq!(scroll_view_controller, Some(primary2.erase()));
            });
        }

        #[test]
        fn test_custom_scroll_controller_behavior_when_set() {
            test_widgets(|tester| {
                let first_box = string_key("C");
                let custom_controller: ScrollControllerRef = ScrollController::new(
                    &mut tester.cell.borrow_mut(),
                    0.0,
                    true,
                    None,
                    None,
                    None,
                )
                .erase();
                pump_home(
                    tester,
                    Scaffold::new().body(
                        SizedBox::new().height(150.0).child(
                            ReorderableListView::new(["C", "B", "A"].map(|label| {
                                SizedBox::new()
                                    .width(100.0)
                                    .height(100.0)
                                    .key(string_key(label))
                                    .child(Text::new(label))
                                    .into_widget()
                            }))
                            .scroll_controller(custom_controller.clone())
                            .on_reorder_item(|_app, _old, _new| {}),
                        ),
                    ),
                );

                // Check initial scroll offset of first list item relative to the offset of the
                // list view.
                drop(custom_controller.animate_to(
                    &mut tester.cell.borrow_mut(),
                    40.0,
                    Duration::from_millis(200),
                    Curves::linear(),
                ));
                tester.pump_and_settle();
                let list_view_top_left =
                    tester.get_top_left(find::by_type::<ReorderableListView>());
                let first_box_top_left = tester.get_top_left(find::by_key(first_box.clone()));
                assert_eq!(first_box_top_left.dy(), list_view_top_left.dy() - 40.0);

                // Drag the UI to see if the scroll controller updates accordingly
                tester.drag(find::text("B"), Offset::new(0.0, -100.0));
                // Initial scroll controller offset: 40.0
                // Drag UI by 100.0 upwards vertically
                // First 20.0 px always ignored, so scroll offset is only
                // shifted by 80.0.
                // Final offset: 40.0 + 80.0 = 120.0
                // The total distance available to scroll is 300.0 - 150.0 = 150.0, or
                // height of the ReorderableListView minus height of the SizedBox. Since
                // The final offset is less than this, it's not limited.
                assert_eq!(custom_controller.offset(&tester.cell.borrow()), 120.0);
            });
        }

        #[test]
        fn reorderable_list_auto_scrolling_is_fast_enough() {
            // Regression test for https://github.com/flutter/flutter/issues/121603.
            test_widgets(|tester| {
                let controller: ScrollControllerRef = ScrollController::new(
                    &mut tester.cell.borrow_mut(),
                    0.0,
                    true,
                    None,
                    None,
                    None,
                )
                .erase();
                pump_home(
                    tester,
                    Scaffold::new().body(
                        ReorderableListView::builder(
                            |_app, _context, index| {
                                Text::new("data")
                                    .key(ValueKey::new(index).into())
                                    .into_widget()
                            },
                            100,
                        )
                        .scroll_controller(controller.clone())
                        .on_reorder_item(|_app, _old, _new| {}),
                    ),
                );

                // Start gesture on first item.
                let first_item = tester.get_center(find::by_key(ValueKey::new(0)));
                let mut drag = start_gesture(tester, first_item);
                tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
                let bottom_right = tester.get_bottom_right(find::by_type::<ReorderableListView>());
                // Drag enough for move to start.
                tester.move_to(
                    &mut drag,
                    Offset::new(bottom_right.dx() / 2.0, bottom_right.dy()),
                    true,
                );
                tester.pump();
                // Use a fixed value to make sure the default velocity scalar is bigger
                // than a certain amount.
                const K_MINIMUM_ALLOWED_AUTO_SCROLL_DISTANCE_PER_5MS: f64 = 1.7;

                for frames in 1..=4 {
                    tester.pump_by(Duration::from_millis(5));
                    assert!(
                        controller.offset(&tester.cell.borrow())
                            > K_MINIMUM_ALLOWED_AUTO_SCROLL_DISTANCE_PER_5MS * f64::from(frames)
                    );
                }
            });
        }

        #[test]
        fn still_builds_when_no_primary_scroll_controller_is_available() {
            test_widgets(|tester| {
                let reorderable_list = ReorderableListView::new(["C", "B", "A"].map(|label| {
                    SizedBox::new()
                        .width(100.0)
                        .height(100.0)
                        .key(string_key(label))
                        .child(Text::new(label))
                        .into_widget()
                }))
                .on_reorder_item(|_app, _old, _new| {})
                .into_widget();

                let entry = OverlayEntry::new(
                    &mut tester.cell.borrow_mut(),
                    Rc::new(move |_app, _context| reorderable_list.clone()),
                    false,
                    false,
                    false,
                );
                tester.pump_widget(localized(SizedBox::square(Some(100.0)).child(
                    Directionality::new(
                        TextDirection::Ltr,
                        Overlay::new().initial_entries([entry]),
                    ),
                )));
            });
        }

        mod accessibility {
            use super::*;

            const MOVE_TO_START: &str = "Move to the start";
            const MOVE_TO_END: &str = "Move to the end";
            const MOVE_UP: &str = "Move up";
            const MOVE_DOWN: &str = "Move down";

            #[test]
            fn provides_the_correct_accessibility_actions_in_ltr_and_rtl_modes() {
                // The a11y actions for a vertical list are the same in LTR and RTL modes.
                test_widgets(|tester| {
                    let items = original_list_items();
                    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
                        tester.pump_widget(build(
                            tester,
                            &items,
                            BuildOptions {
                                text_direction: direction,
                                ..BuildOptions::default()
                            },
                        ));
                        assert_actions(
                            tester,
                            &items,
                            &[
                                // The first item can be moved down or to the end.
                                (0, &[MOVE_DOWN, MOVE_TO_END]),
                                // Items in the middle can be moved to the start, end, up or down.
                                (1, &[MOVE_TO_START, MOVE_UP, MOVE_DOWN, MOVE_TO_END]),
                                (2, &[MOVE_TO_START, MOVE_UP, MOVE_DOWN, MOVE_TO_END]),
                                // The last item can be moved up or to the start.
                                (3, &[MOVE_TO_START, MOVE_UP]),
                            ],
                        );
                    }
                });
            }

            #[test]
            fn first_item_accessibility_a11y_actions_work() {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to end: move Item 1 to the end of the list.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 0, MOVE_TO_END);
                    items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);

                    // Test out move after: move Item 2 (the current first item) one space down.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 0, MOVE_DOWN);
                    items_are(&items, &["Item 3", "Item 2", "Item 4", "Item 1"]);
                });
            }

            #[test]
            fn middle_item_accessibility_a11y_actions_work() {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to end: move Item 2 to the end of the list.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 1, MOVE_TO_END);
                    items_are(&items, &["Item 1", "Item 3", "Item 4", "Item 2"]);

                    // Test out move after: move Item 3 (the current second item) one space down.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 1, MOVE_DOWN);
                    items_are(&items, &["Item 1", "Item 4", "Item 3", "Item 2"]);

                    // Test out move after: move Item 3 (the current third item) one space up.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 2, MOVE_UP);
                    items_are(&items, &["Item 1", "Item 3", "Item 4", "Item 2"]);

                    // Test out move to start: move Item 4 (the current third item) to the start
                    // of the list.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 2, MOVE_TO_START);
                    items_are(&items, &["Item 4", "Item 1", "Item 3", "Item 2"]);
                });
            }

            #[test]
            fn last_item_accessibility_a11y_actions_work() {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to start: move Item 4 to the start of the list.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 3, MOVE_TO_START);
                    items_are(&items, &["Item 4", "Item 1", "Item 2", "Item 3"]);

                    // Test out move up: move Item 3 (the current last item) one space up.
                    tester.pump_widget(build_default(tester, &items));
                    perform_action(tester, &items, 3, MOVE_UP);
                    items_are(&items, &["Item 4", "Item 1", "Item 3", "Item 2"]);
                });
            }
        }
    }

    /// Dart's `Localizations(locale: en, delegates: [DefaultMaterialLocalizations.delegate,
    /// DefaultWidgetsLocalizations.delegate], child:)`.
    fn localized<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Localizations::new(
            Locale::new("en"),
            vec![
                DefaultMaterialLocalizations::delegate(),
                DefaultWidgetsLocalizations::delegate(),
            ],
        )
        .child(child)
        .into_widget()
    }

    /// Dart's `getSemanticsActions(index)`: the custom semantics actions of the nearest
    /// `Semantics` above the item at `index`.
    fn semantics_actions(
        tester: &WidgetTester,
        items: &ListItems,
        index: usize,
    ) -> IndexMap<CustomSemanticsAction, VoidCallback> {
        let item = items.borrow()[index].clone();
        let semantics = find::ancestor(
            find::by_key(string_key(&item)),
            find::by_type::<Semantics>(),
        )
        .first();
        tester.widget(semantics, |semantics: &Semantics| {
            semantics
                .properties
                .custom_semantics_actions
                .clone()
                .expect("the item has custom semantics actions")
        })
    }

    /// The labels each listed item must offer, and no others.
    fn assert_actions(tester: &WidgetTester, items: &ListItems, expected: &[(usize, &[&str])]) {
        for (index, labels) in expected {
            let actions = semantics_actions(tester, items, *index);
            assert_eq!(actions.len(), labels.len(), "list item {index}");
            for label in *labels {
                assert!(
                    actions.contains_key(&CustomSemanticsAction::new(*label)),
                    "list item {index} should be able to `{label}`"
                );
            }
        }
    }

    /// Dart's `getSemanticsActions(index)[action]!()` followed by `pumpAndSettle`.
    fn perform_action(tester: &mut WidgetTester, items: &ListItems, index: usize, label: &str) {
        let actions = semantics_actions(tester, items, index);
        let action = actions
            .get(&CustomSemanticsAction::new(label))
            .unwrap_or_else(|| panic!("list item {index} has no `{label}` action"))
            .clone();
        tester.update(|app| action(app));
        tester.pump_and_settle();
    }

    fn drop_area_extents(axis: Axis) {
        test_widgets(|tester| {
            let item = |key: &str, extent_factor: f64| {
                let mut item = SizedBox::new().key(string_key(key)).child(Text::new(key));
                item = match axis {
                    Axis::Vertical => item.height(ITEM_HEIGHT * extent_factor),
                    Axis::Horizontal => item.width(ITEM_HEIGHT * extent_factor),
                };
                item.into_widget()
            };
            let reorderable_list_view = ReorderableListView::new([
                item("Normal item", 1.0),
                item("Tall item", 2.0),
                item("Last item", 1.0),
            ])
            .scroll_direction(axis)
            .on_reorder_item(|_app, _old, _new| {});
            let home = match axis {
                Axis::Vertical => SizedBox::new()
                    .height(ITEM_HEIGHT * 10.0)
                    .child(reorderable_list_view),
                Axis::Horizontal => SizedBox::new()
                    .width(ITEM_HEIGHT * 10.0)
                    .child(reorderable_list_view),
            };
            pump_home(tester, home);

            let list_extent = |tester: &WidgetTester| {
                let list = tester.render_object(find::by_type::<SliverList>());
                let app = tester.cell.borrow();
                list.downcast::<RenderSliverList>(&app)
                    .expect("a SliverList renders a RenderSliverList")
                    .geometry(&app)
                    .max_paint_extent
            };

            const K_DRAGGING_LIST_EXTENT: f64 = 4.0 * ITEM_HEIGHT;
            // Drag a normal text item
            assert_eq!(list_extent(tester), K_DRAGGING_LIST_EXTENT);
            for dragged in ["Normal item", "Tall item"] {
                let at = tester.get_center(find::text(dragged));
                let mut drag = start_gesture(tester, at);
                tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
                tester.pump_and_settle();
                assert_eq!(list_extent(tester), K_DRAGGING_LIST_EXTENT);

                // Move it
                let last = tester.get_center(find::text("Last item"));
                tester.move_to(&mut drag, last, true);
                tester.pump_and_settle();
                assert_eq!(list_extent(tester), K_DRAGGING_LIST_EXTENT);

                // Drop it
                tester.up(&drag);
                tester.pump_and_settle();
                assert_eq!(list_extent(tester), K_DRAGGING_LIST_EXTENT);
            }
        });
    }

    /// Dart's `_Stateful`: a checkbox that records what it is told without rebuilding.
    struct Stateful {
        key: KeyRef,
    }

    impl fmt::Debug for Stateful {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Stateful").finish()
        }
    }

    struct StatefulState {
        state: StateData<Stateful>,
        checked: Option<bool>,
    }

    impl StatefulWidget for Stateful {
        type State = StatefulState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> StatefulState {
            StatefulState {
                state: StateData::new(),
                checked: Some(false),
            }
        }
    }

    impl State for StatefulState {
        type Widget = Stateful;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let checked = app.get(self).checked;
            SizedBox::square(Some(48.0))
                .child(Material::new().child(Checkbox::new(
                    checked,
                    Some(Rc::new(move |app, new_value| {
                        app.get_mut(self).checked = new_value;
                    })),
                )))
                .into_widget()
        }
    }

    /// Dart's `findState(key).checked`.
    fn checked(tester: &WidgetTester, key: &str) -> Option<bool> {
        let element = tester.element(find::by_key(string_key(key)));
        let app = tester.cell.borrow();
        let state = element
            .state_handle::<StatefulState>(&app)
            .expect("the key names a Stateful");
        app.get(state).checked
    }

    fn stateful_children(order: [&str; 3]) -> Vec<WidgetRef> {
        order
            .map(|key| {
                Stateful {
                    key: string_key(key),
                }
                .into_widget()
            })
            .to_vec()
    }

    fn check_preserves_children_states_when_the_list_parent_changes_the_order(axis: Axis) {
        test_widgets(|tester| {
            pump_home(
                tester,
                ReorderableListView::new(stateful_children(["A", "B", "C"]))
                    .scroll_direction(axis)
                    .on_reorder_item(|_app, _old, _new| {}),
            );
            tester.tap(find::by_key(string_key("A")));
            tester.pump_and_settle();
            // Only the 'A' widget should be checked.
            assert_eq!(checked(tester, "A"), Some(true));
            assert_eq!(checked(tester, "B"), Some(false));
            assert_eq!(checked(tester, "C"), Some(false));

            pump_home(
                tester,
                ReorderableListView::new(stateful_children(["B", "C", "A"]))
                    .scroll_direction(axis)
                    .on_reorder_item(|_app, _old, _new| {}),
            );
            // Only the 'A' widget should be checked.
            assert_eq!(checked(tester, "B"), Some(false));
            assert_eq!(checked(tester, "C"), Some(false));
            assert_eq!(checked(tester, "A"), Some(true));
        });
    }

    fn check_preserves_children_states_when_rebuilt(axis: Axis) {
        test_widgets(|tester| {
            let first_box = string_key("key");
            let build = |tester: &mut WidgetTester| {
                pump_home(
                    tester,
                    Directionality::new(
                        TextDirection::Ltr,
                        SizedBox::square(Some(100.0)).child(
                            ReorderableListView::new([SizedBox::new()
                                .key(first_box.clone())
                                .width(10.0)
                                .height(10.0)
                                .into_widget()])
                            .scroll_direction(axis)
                            .on_reorder_item(|_app, _old, _new| {}),
                        ),
                    ),
                );
            };

            // When the widget is rebuilt, the state of child should be consistent.
            build(tester);
            let e0 = tester.element(find::by_key(first_box.clone()));
            build(tester);
            let e1 = tester.element(find::by_key(first_box.clone()));
            assert_eq!(e0, e1);
        });
    }

    mod in_horizontal_mode {
        use super::*;

        #[test]
        fn reorder_is_not_triggered_when_children_length_is_less_or_equals_to_1() {
            reorder_is_not_triggered_when_children_length_is_one(Axis::Horizontal);
        }

        #[test]
        fn allows_reordering_from_the_very_top_to_the_very_bottom() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build(tester, &items, options_along(Axis::Horizontal)));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(along(Axis::Horizontal, ITEM_HEIGHT * 2.0)),
                );
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
            });
        }

        #[test]
        fn allows_reordering_from_the_very_bottom_to_the_very_top() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build(tester, &items, options_along(Axis::Horizontal)));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(tester, "Item 4", "Item 1", None);
                items_are(&items, &["Item 4", "Item 1", "Item 2", "Item 3"]);
            });
        }

        #[test]
        fn allows_reordering_inside_the_middle_of_the_widget() {
            test_widgets(|tester| {
                let items = original_list_items();
                tester.pump_widget(build(tester, &items, options_along(Axis::Horizontal)));
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(tester, "Item 3", "Item 2", None);
                items_are(&items, &["Item 1", "Item 3", "Item 2", "Item 4"]);
            });
        }

        #[test]
        fn properly_reorders_with_a_header() {
            test_widgets(|tester| {
                let items = original_list_items();
                let with_header = |tester: &WidgetTester| {
                    build(
                        tester,
                        &items,
                        BuildOptions {
                            header: Some(Text::new("Header Text").into_widget()),
                            scroll_direction: Axis::Horizontal,
                            ..BuildOptions::default()
                        },
                    )
                };
                tester.pump_widget(with_header(tester));
                expect(tester, &find::text("Header Text"), finds_one_widget());
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(along(Axis::Horizontal, ITEM_HEIGHT * 2.0)),
                );
                expect(tester, &find::text("Header Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
                tester.pump_widget(with_header(tester));
                drag_between_items(tester, "Item 4", "Item 3", None);
                expect(tester, &find::text("Header Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 4", "Item 3", "Item 1"]);
            });
        }

        #[test]
        fn properly_reorders_with_a_footer() {
            test_widgets(|tester| {
                let items = original_list_items();
                let with_footer = |tester: &WidgetTester| {
                    build(
                        tester,
                        &items,
                        BuildOptions {
                            footer: Some(Text::new("Footer Text").into_widget()),
                            scroll_direction: Axis::Horizontal,
                            ..BuildOptions::default()
                        },
                    )
                };
                tester.pump_widget(with_footer(tester));
                expect(tester, &find::text("Footer Text"), finds_one_widget());
                items_are(&items, &ORIGINAL_LIST_ITEMS);
                drag_between_items(
                    tester,
                    "Item 1",
                    "Item 4",
                    Some(along(Axis::Horizontal, ITEM_HEIGHT * 2.0)),
                );
                expect(tester, &find::text("Footer Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);
                tester.pump_widget(with_footer(tester));
                drag_between_items(tester, "Item 4", "Item 3", None);
                expect(tester, &find::text("Footer Text"), finds_one_widget());
                items_are(&items, &["Item 2", "Item 4", "Item 3", "Item 1"]);
            });
        }

        #[test]
        fn properly_determines_the_horizontal_drop_area_extents() {
            drop_area_extents(Axis::Horizontal);
        }

        #[test]
        fn preserves_children_states_when_the_list_parent_changes_the_order() {
            check_preserves_children_states_when_the_list_parent_changes_the_order(
                Axis::Horizontal,
            );
        }

        #[test]
        fn preserves_children_states_when_rebuilt() {
            check_preserves_children_states_when_rebuilt(Axis::Horizontal);
        }

        /// What each direction calls the motion toward the start and the end of the list.
        struct Motion {
            before: &'static str,
            after: &'static str,
        }

        const LTR: Motion = Motion {
            before: "Move left",
            after: "Move right",
        };
        const RTL: Motion = Motion {
            before: "Move right",
            after: "Move left",
        };

        mod accessibility {
            use super::*;

            const MOVE_TO_START: &str = "Move to the start";
            const MOVE_TO_END: &str = "Move to the end";

            fn horizontal(text_direction: TextDirection) -> BuildOptions {
                BuildOptions {
                    scroll_direction: Axis::Horizontal,
                    text_direction,
                    ..BuildOptions::default()
                }
            }

            /// In RTL mode, the right is the start and the left is the end. The array
            /// representation is unchanged (LTR), but the direction of the motion actions is
            /// reversed.
            fn provides_the_correct_accessibility_actions(
                text_direction: TextDirection,
                motion: &Motion,
            ) {
                test_widgets(|tester| {
                    let items = original_list_items();
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    assert_actions(
                        tester,
                        &items,
                        &[
                            // The first item can be moved after or to the end.
                            (0, &[motion.after, MOVE_TO_END]),
                            // Items in the middle can be moved to the start, end, before or
                            // after.
                            (
                                1,
                                &[MOVE_TO_START, motion.before, motion.after, MOVE_TO_END],
                            ),
                            (
                                2,
                                &[MOVE_TO_START, motion.before, motion.after, MOVE_TO_END],
                            ),
                            // The last item can be moved before or to the start.
                            (3, &[MOVE_TO_START, motion.before]),
                        ],
                    );
                });
            }

            #[test]
            fn provides_the_correct_accessibility_actions_in_ltr_mode() {
                provides_the_correct_accessibility_actions(TextDirection::Ltr, &LTR);
            }

            #[test]
            fn provides_the_correct_accessibility_actions_in_right_to_left_directionality() {
                provides_the_correct_accessibility_actions(TextDirection::Rtl, &RTL);
            }

            fn first_item_actions_work(text_direction: TextDirection, motion: &Motion) {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to end: move Item 1 to the end of the list.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 0, MOVE_TO_END);
                    items_are(&items, &["Item 2", "Item 3", "Item 4", "Item 1"]);

                    // Test out move after: move Item 2 (the current first item) one space.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 0, motion.after);
                    items_are(&items, &["Item 3", "Item 2", "Item 4", "Item 1"]);
                });
            }

            #[test]
            fn first_item_accessibility_a11y_actions_work_in_ltr_mode() {
                first_item_actions_work(TextDirection::Ltr, &LTR);
            }

            #[test]
            fn first_item_accessibility_a11y_actions_work_in_right_to_left_directionality() {
                first_item_actions_work(TextDirection::Rtl, &RTL);
            }

            fn middle_item_actions_work(text_direction: TextDirection, motion: &Motion) {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to end: move Item 2 to the end of the list.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 1, MOVE_TO_END);
                    items_are(&items, &["Item 1", "Item 3", "Item 4", "Item 2"]);

                    // Test out move after: move Item 3 (the current second item) one space after.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 1, motion.after);
                    items_are(&items, &["Item 1", "Item 4", "Item 3", "Item 2"]);

                    // Test out move before: move Item 3 (the current third item) one space before.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 2, motion.before);
                    items_are(&items, &["Item 1", "Item 3", "Item 4", "Item 2"]);

                    // Test out move to start: move Item 4 (the current third item) to the start
                    // of the list.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 2, MOVE_TO_START);
                    items_are(&items, &["Item 4", "Item 1", "Item 3", "Item 2"]);
                });
            }

            #[test]
            fn middle_item_accessibility_a11y_actions_work_in_ltr_mode() {
                middle_item_actions_work(TextDirection::Ltr, &LTR);
            }

            #[test]
            fn middle_item_accessibility_a11y_actions_work_in_right_to_left_directionality() {
                middle_item_actions_work(TextDirection::Rtl, &RTL);
            }

            fn last_item_actions_work(text_direction: TextDirection, motion: &Motion) {
                test_widgets(|tester| {
                    let items = original_list_items();
                    items_are(&items, &ORIGINAL_LIST_ITEMS);

                    // Test out move to start: move Item 4 to the start of the list.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 3, MOVE_TO_START);
                    items_are(&items, &["Item 4", "Item 1", "Item 2", "Item 3"]);

                    // Test out move before: move Item 3 (the current last item) one space.
                    tester.pump_widget(build(tester, &items, horizontal(text_direction)));
                    perform_action(tester, &items, 3, motion.before);
                    items_are(&items, &["Item 4", "Item 1", "Item 3", "Item 2"]);
                });
            }

            #[test]
            fn last_item_accessibility_a11y_actions_work_in_ltr_mode() {
                last_item_actions_work(TextDirection::Ltr, &LTR);
            }

            #[test]
            fn last_item_accessibility_a11y_actions_work_in_right_to_left_directionality() {
                last_item_actions_work(TextDirection::Rtl, &RTL);
            }
        }
    }

    #[test]
    #[should_panic(expected = "item_count >= 0")]
    fn builder_asserts_on_negative_child_count() {
        let _ = ReorderableListView::builder(
            |_app, _context, _index| SizedBox::new().into_widget(),
            -1,
        )
        .on_reorder_item(|_app, _old, _new| {})
        .into_widget();
    }

    #[test]
    fn builder_only_creates_the_children_it_needs() {
        test_widgets(|tester| {
            let items_created: Rc<RefCell<HashSet<i32>>> = Rc::default();
            pump_home(
                tester,
                ReorderableListView::builder(
                    {
                        let items_created = Rc::clone(&items_created);
                        move |_app, _context, index| {
                            items_created.borrow_mut().insert(index);
                            Text::new(index.to_string())
                                .key(ValueKey::new(index).into())
                                .into_widget()
                        }
                    },
                    1000,
                )
                .on_reorder_item(|_app, _old, _new| {}),
            );

            // Should have only created the first 18 items.
            assert_eq!(*items_created.borrow(), (0..=17).collect::<HashSet<i32>>());
        });
    }

    mod padding {
        use super::*;

        fn rect_of(tester: &WidgetTester, key: &str) -> Rect {
            tester.get_rect(find::by_key(string_key(key)))
        }

        #[test]
        fn padding_with_no_header_and_footer() {
            test_widgets(|tester| {
                let padding = EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0);
                let items = original_list_items();
                let padded = |axis| BuildOptions {
                    padding,
                    scroll_direction: axis,
                    ..BuildOptions::default()
                };

                // Vertical
                tester.pump_widget(build(tester, &items, padded(Axis::Vertical)));
                assert_eq!(
                    rect_of(tester, "Item 1"),
                    Rect::from_ltrb(10.0, 20.0, 770.0, 68.0)
                );
                assert_eq!(
                    rect_of(tester, "Item 4"),
                    Rect::from_ltrb(10.0, 164.0, 770.0, 212.0)
                );

                // Horizontal
                tester.pump_widget(build(tester, &items, padded(Axis::Horizontal)));
                assert_eq!(
                    rect_of(tester, "Item 1"),
                    Rect::from_ltrb(10.0, 20.0, 58.0, 560.0)
                );
                assert_eq!(
                    rect_of(tester, "Item 4"),
                    Rect::from_ltrb(154.0, 20.0, 202.0, 560.0)
                );
            });
        }

        #[test]
        fn padding_with_header_or_footer() {
            test_widgets(|tester| {
                let padding = EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0);
                let items = original_list_items();
                let vertical_header = || {
                    SizedBox::new()
                        .key(string_key("Header"))
                        .height(10.0)
                        .into_widget()
                };
                let horizontal_header = || {
                    SizedBox::new()
                        .key(string_key("Header"))
                        .width(10.0)
                        .into_widget()
                };
                let vertical_footer = || {
                    SizedBox::new()
                        .key(string_key("Footer"))
                        .height(10.0)
                        .into_widget()
                };
                let horizontal_footer = || {
                    SizedBox::new()
                        .key(string_key("Footer"))
                        .width(10.0)
                        .into_widget()
                };
                let options = |header: Option<WidgetRef>,
                               footer: Option<WidgetRef>,
                               scroll_direction: Axis,
                               reverse: bool| BuildOptions {
                    header,
                    footer,
                    padding,
                    scroll_direction,
                    reverse,
                    ..BuildOptions::default()
                };
                let expect_rects =
                    |tester: &WidgetTester, extra: (&str, Rect), item_1: Rect, item_4: Rect| {
                        assert_eq!(rect_of(tester, extra.0), extra.1);
                        assert_eq!(rect_of(tester, "Item 1"), item_1);
                        assert_eq!(rect_of(tester, "Item 4"), item_4);
                    };

                // Vertical Header
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(Some(vertical_header()), None, Axis::Vertical, false),
                ));
                expect_rects(
                    tester,
                    ("Header", Rect::from_ltrb(10.0, 20.0, 770.0, 30.0)),
                    Rect::from_ltrb(10.0, 30.0, 770.0, 78.0),
                    Rect::from_ltrb(10.0, 174.0, 770.0, 222.0),
                );

                // Vertical Footer
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(None, Some(vertical_footer()), Axis::Vertical, false),
                ));
                expect_rects(
                    tester,
                    ("Footer", Rect::from_ltrb(10.0, 212.0, 770.0, 222.0)),
                    Rect::from_ltrb(10.0, 20.0, 770.0, 68.0),
                    Rect::from_ltrb(10.0, 164.0, 770.0, 212.0),
                );

                // Vertical Header, reversed
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(Some(vertical_header()), None, Axis::Vertical, true),
                ));
                expect_rects(
                    tester,
                    ("Header", Rect::from_ltrb(10.0, 550.0, 770.0, 560.0)),
                    Rect::from_ltrb(10.0, 502.0, 770.0, 550.0),
                    Rect::from_ltrb(10.0, 358.0, 770.0, 406.0),
                );

                // Vertical Footer, reversed
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(None, Some(vertical_footer()), Axis::Vertical, true),
                ));
                expect_rects(
                    tester,
                    ("Footer", Rect::from_ltrb(10.0, 358.0, 770.0, 368.0)),
                    Rect::from_ltrb(10.0, 512.0, 770.0, 560.0),
                    Rect::from_ltrb(10.0, 368.0, 770.0, 416.0),
                );

                // Horizontal Header
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(Some(horizontal_header()), None, Axis::Horizontal, false),
                ));
                expect_rects(
                    tester,
                    ("Header", Rect::from_ltrb(10.0, 20.0, 20.0, 560.0)),
                    Rect::from_ltrb(20.0, 20.0, 68.0, 560.0),
                    Rect::from_ltrb(164.0, 20.0, 212.0, 560.0),
                );

                // Horizontal Footer
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(None, Some(horizontal_footer()), Axis::Horizontal, false),
                ));
                expect_rects(
                    tester,
                    ("Footer", Rect::from_ltrb(202.0, 20.0, 212.0, 560.0)),
                    Rect::from_ltrb(10.0, 20.0, 58.0, 560.0),
                    Rect::from_ltrb(154.0, 20.0, 202.0, 560.0),
                );

                // Horizontal Header, reversed
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(Some(horizontal_header()), None, Axis::Horizontal, true),
                ));
                expect_rects(
                    tester,
                    ("Header", Rect::from_ltrb(760.0, 20.0, 770.0, 560.0)),
                    Rect::from_ltrb(712.0, 20.0, 760.0, 560.0),
                    Rect::from_ltrb(568.0, 20.0, 616.0, 560.0),
                );

                // Horizontal Footer, reversed
                tester.pump_widget(build(
                    tester,
                    &items,
                    options(None, Some(horizontal_footer()), Axis::Horizontal, true),
                ));
                expect_rects(
                    tester,
                    ("Footer", Rect::from_ltrb(568.0, 20.0, 578.0, 560.0)),
                    Rect::from_ltrb(722.0, 20.0, 770.0, 560.0),
                    Rect::from_ltrb(578.0, 20.0, 626.0, 560.0),
                );
            });
        }
    }

    #[test]
    fn reorderable_list_view_can_be_reversed() {
        test_widgets(|tester| {
            let item = |label: &str| {
                SizedBox::new()
                    .key(string_key(label))
                    .child(Text::new(label))
                    .into_widget()
            };
            pump_home(
                tester,
                ReorderableListView::new([item("A"), item("B"), item("C")])
                    .reverse(true)
                    .on_reorder_item(|_app, _old, _new| {}),
            );
            assert!(
                tester.get_center(find::text("A")).dy() > tester.get_center(find::text("B")).dy()
            );
        });
    }

    #[test]
    #[allow(deprecated)]
    fn reorderable_list_view_in_flexible_with_one_item_does_not_assert_when_dragged_to_edge() {
        test_widgets(|tester| {
            let items = Rc::new(RefCell::new(vec!["Item 1".to_string()]));
            pump_home(
                tester,
                Scaffold::new().body(
                    Column::new().children([Flexible::new(StatefulBuilder::new({
                        let items = Rc::clone(&items);
                        move |_app, _context, set_state: StateSetter| {
                            let on_reorder_items = Rc::clone(&items);
                            let first = items.borrow()[0].clone();
                            ReorderableListView::new([ListTile::new()
                                .key(ValueKey::new("Item 1".to_string()).into())
                                .title(Text::new(first))
                                .into_widget()])
                            .on_reorder(move |app, old_index, mut new_index| {
                                let items = Rc::clone(&on_reorder_items);
                                set_state(app, &mut |_app| {
                                    if new_index > old_index {
                                        new_index -= 1;
                                    }
                                    let mut items = items.borrow_mut();
                                    let item = items.remove(old_index as usize);
                                    items.insert(new_index as usize, item);
                                });
                            })
                            .into_widget()
                        }
                    }))
                    .into_widget()]),
                ),
            );
            tester.pump_and_settle();

            let start_location = tester.get_center(find::by_key(string_key("Item 1")));
            let mut gesture = start_gesture(tester, start_location);
            tester.pump();
            let scaffold_corner = tester.get_bottom_right(find::by_type::<Scaffold>());
            tester.move_to(
                &mut gesture,
                scaffold_corner - Offset::new(10.0, 10.0),
                true,
            );
            tester.pump_by(Duration::from_secs(1));

            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn animation_test_when_placing_an_item_in_place() {
        test_widgets(|tester| {
            let test_item_key = string_key("Test item");
            let item = |label: &str| {
                SizedBox::new()
                    .key(string_key(label))
                    .height(ITEM_HEIGHT)
                    .child(Text::new(label))
                    .into_widget()
            };
            pump_home(
                tester,
                SizedBox::new().height(ITEM_HEIGHT * 10.0).child(
                    ReorderableListView::new([
                        item("First item"),
                        item("Test item"),
                        item("Last item"),
                    ])
                    .on_reorder_item(|_app, _old, _new| {}),
                ),
            );

            let test_item_position =
                |tester: &WidgetTester| tester.get_top_left(find::by_key(test_item_key.clone()));

            // Before pick it up.
            let start_position = test_item_position(tester);

            // Pick it up.
            let center = tester.get_center(find::by_key(test_item_key.clone()));
            let gesture = start_gesture(tester, center);
            tester.pump_by(K_LONG_PRESS_TIMEOUT + K_PRESS_TIMEOUT);
            assert_eq!(test_item_position(tester), start_position);

            // Put it down.
            tester.up(&gesture);
            tester.pump();
            assert_eq!(test_item_position(tester), start_position);

            // After put it down.
            tester.pump_and_settle();
            assert_eq!(test_item_position(tester), start_position);
        });
    }

    #[test]
    fn reorderable_list_view_on_desktop_platforms_should_have_drag_handles() {
        test_widgets_variant(TargetPlatformVariant::desktop(), |tester| {
            let items = original_list_items();
            tester.pump_widget(build_default(tester, &items));
            // All four items should have drag handles and not delayed listeners.
            expect(
                tester,
                &find::by_icon(&Icons::drag_handle()),
                finds_n_widgets(4),
            );
            expect(
                tester,
                &find::by_type::<ReorderableDelayedDragStartListener>(),
                finds_nothing(),
            );
        });
    }

    #[test]
    fn reorderable_list_view_on_mobile_platforms_should_not_have_drag_handles() {
        test_widgets_variant(TargetPlatformVariant::mobile(), |tester| {
            let items = original_list_items();
            tester.pump_widget(build_default(tester, &items));
            // All four items should have delayed listeners and not drag handles.
            expect(
                tester,
                &find::by_type::<ReorderableDelayedDragStartListener>(),
                finds_n_widgets(4),
            );
            expect(
                tester,
                &find::by_icon(&Icons::drag_handle()),
                finds_nothing(),
            );
        });
    }

    #[test]
    fn vertical_list_renders_drag_handle_in_correct_position() {
        test_widgets(|tester| {
            let items = original_list_items();
            tester.pump_widget(build(
                tester,
                &items,
                BuildOptions {
                    platform: Some(TargetPlatform::MacOS),
                    ..BuildOptions::default()
                },
            ));
            let list_view = find::by_type::<ReorderableListView>();
            let item_1 = find::by_key(string_key("Item 1"));
            let drag_handle = find::by_icon(&Icons::drag_handle()).first();

            // Should be centered vertically within the item and 8 pixels from the right edge of
            // the list.
            assert_eq!(
                tester.get_center(drag_handle.clone()).dy(),
                tester.get_center(item_1).dy()
            );
            assert_eq!(
                tester.get_top_right(drag_handle).dx(),
                tester.get_size(list_view).width() - 8.0
            );
        });
    }

    #[test]
    fn horizontal_list_renders_drag_handle_in_correct_position() {
        test_widgets(|tester| {
            let items = original_list_items();
            tester.pump_widget(build(
                tester,
                &items,
                BuildOptions {
                    scroll_direction: Axis::Horizontal,
                    platform: Some(TargetPlatform::MacOS),
                    ..BuildOptions::default()
                },
            ));
            let list_view = find::by_type::<ReorderableListView>();
            let item_1 = find::by_key(string_key("Item 1"));
            let drag_handle = find::by_icon(&Icons::drag_handle()).first();

            // Should be centered horizontally within the item and 8 pixels from the bottom of
            // the list.
            assert_eq!(
                tester.get_center(drag_handle.clone()).dx(),
                tester.get_center(item_1).dx()
            );
            assert_eq!(
                tester.get_bottom_right(drag_handle).dy(),
                tester.get_size(list_view).height() - 8.0
            );
        });
    }

    #[test]
    fn reorderable_list_view_can_deal_with_the_dragged_item_getting_unmounted_and_rebuilt_during_drag()
     {
        // See https://github.com/flutter/flutter/issues/74840 for more details.
        test_widgets(|tester| {
            let items: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new((0..100).collect()));

            // The list is 800x600, 8 items, each item is 800x100 with an "item $index" text
            // widget at the item's origin. Drags are initiated by a simple press on the text
            // widget.
            pump_home(
                tester,
                ReorderableListView::builder(
                    {
                        let items = Rc::clone(&items);
                        move |_app, _context, index| {
                            let item = items.borrow()[index as usize];
                            SizedBox::new()
                                .key(ValueKey::new(item).into())
                                .height(100.0)
                                .child(ReorderableDragStartListener::new(
                                    Row::new()
                                        .cross_axis_alignment(CrossAxisAlignment::Start)
                                        .children(
                                            [Text::new(format!("item {item}")).into_widget()],
                                        ),
                                    index,
                                ))
                                .into_widget()
                        }
                    },
                    100,
                )
                .build_default_drag_handles(false)
                .on_reorder_item({
                    let items = Rc::clone(&items);
                    move |_app, from_index, to_index| {
                        let mut items = items.borrow_mut();
                        let item = items.remove(from_index as usize);
                        items.insert(to_index as usize, item);
                    }
                }),
            );

            // Drag item 0 downwards and force an auto scroll off the end of the list
            // far enough that item zeros original entry in the list is unmounted.
            let center = tester.get_center(find::text("item 0"));
            let mut drag = start_gesture(tester, center);
            tester.pump_by(K_PRESS_TIMEOUT);
            // Off the bottom of the screen, which should autoscroll until we hit the
            // end of the list
            tester.move_by(&mut drag, Offset::new(0.0, 700.0), true);
            tester.pump_by(Duration::from_secs(30));
            tester.pump_and_settle();
            // Ensure we made it to the bottom (only 4 should be showing as there should
            // be a gap at the end for the drop area of the dragged item.
            for i in [95, 96, 97, 98, 99] {
                expect(
                    tester,
                    &find::text(&format!("item {i}")),
                    finds_one_widget(),
                );
            }

            // Drag back to off the top of the list, which should autoscroll until
            // we hit the beginning of the list. This should cause the first item's
            // entry to be rebuilt. However, the contents should not be in both places.
            tester.move_by(&mut drag, Offset::new(0.0, -1400.0), true);
            tester.pump_by(Duration::from_secs(30));
            tester.pump_and_settle();
            // Release back at the top so item 0 should drop where it was
            tester.up(&drag);
            tester.pump_and_settle();

            // Should not have changed anything
            for i in [0, 1, 2, 3, 4, 5] {
                expect(
                    tester,
                    &find::text(&format!("item {i}")),
                    finds_one_widget(),
                );
            }
            assert_eq!(items.borrow()[..8], [0, 1, 2, 3, 4, 5, 6, 7]);
        });
    }

    #[test]
    fn reorderable_list_view_calls_on_reorder_start_and_on_reorder_end_correctly() {
        test_widgets(|tester| {
            let items: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new((0..8).collect()));
            let start_index: Rc<Cell<Option<i32>>> = Rc::default();
            let end_index: Rc<Cell<Option<i32>>> = Rc::default();
            let item_0 = || find::text_containing("item 0");

            pump_home(
                tester,
                ReorderableListView::builder(
                    {
                        let items = Rc::clone(&items);
                        move |_app, _context, index| {
                            let item = items.borrow()[index as usize];
                            SizedBox::new()
                                .key(ValueKey::new(item).into())
                                .height(100.0)
                                .child(ReorderableDragStartListener::new(
                                    Row::new()
                                        .cross_axis_alignment(CrossAxisAlignment::Start)
                                        .children(
                                            [Text::new(format!("item {item}")).into_widget()],
                                        ),
                                    index,
                                ))
                                .into_widget()
                        }
                    },
                    8,
                )
                .build_default_drag_handles(false)
                .on_reorder_item({
                    let items = Rc::clone(&items);
                    move |_app, from_index, to_index| {
                        let mut items = items.borrow_mut();
                        let item = items.remove(from_index as usize);
                        items.insert(to_index as usize, item);
                    }
                })
                .on_reorder_start({
                    let start_index = Rc::clone(&start_index);
                    move |_app, index| start_index.set(Some(index))
                })
                .on_reorder_end({
                    let end_index = Rc::clone(&end_index);
                    move |_app, index| end_index.set(Some(index))
                }),
            );

            let at = tester.get_center(item_0());
            let mut drag = start_gesture(tester, at);
            tester.pump_by(K_PRESS_TIMEOUT);
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
            tester.pump_by(K_PRESS_TIMEOUT);
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
        });
    }

    /// The two drags of the callback cases: `item 0` down by 50, then back up by 50.
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

    /// Dart's `handleReorder` of the old `onReorder` cases, which adjusts the index of an item
    /// moved down.
    fn handle_old_reorder<T: 'static>(
        items: &Rc<RefCell<Vec<T>>>,
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

    fn handle_reorder_item<T: 'static>(
        items: &Rc<RefCell<Vec<T>>>,
    ) -> impl Fn(&mut App, i32, i32) + 'static {
        let items = Rc::clone(items);
        move |_app, from_index, to_index| {
            let mut items = items.borrow_mut();
            let item = items.remove(from_index as usize);
            items.insert(to_index as usize, item);
        }
    }

    fn five_items() -> Rc<RefCell<Vec<i32>>> {
        Rc::new(RefCell::new((0..5).collect()))
    }

    /// The children cases' list: Dart's `children` is one `List` that `handleReorder` mutates and
    /// the `itemBuilder` reads, so the list shows the new order after a drop. A `Vec` is moved
    /// into the widget, so the list is rebuilt from `items` after each reorder instead.
    fn rebuilding_children_list(
        items: &Rc<RefCell<Vec<i32>>>,
        old_callback: bool,
        on_reorder: impl Fn(&mut App, i32, i32) + 'static,
    ) -> StatefulBuilder {
        let items = Rc::clone(items);
        let on_reorder: ReorderCallback = Rc::new(on_reorder);
        StatefulBuilder::new(move |_app, _context, set_state: StateSetter| {
            let on_reorder = Rc::clone(&on_reorder);
            let reorder: ReorderCallback = Rc::new(move |app, from_index, to_index| {
                set_state(app, &mut |app| on_reorder(app, from_index, to_index));
            });
            let mut list = ReorderableListView::new(numbered_children(&items));
            if old_callback {
                list.on_reorder = Some(reorder);
            } else {
                list.on_reorder_item = Some(reorder);
            }
            list.into_widget()
        })
    }

    #[test]
    fn reorderable_list_view_calls_old_on_reorder_callback_correctly() {
        test_widgets(|tester| {
            let on_reorder_call_count = Rc::new(Cell::new(0));
            let items = five_items();
            pump_home(
                tester,
                rebuilding_children_list(
                    &items,
                    true,
                    handle_old_reorder(&items, &on_reorder_call_count),
                ),
            );
            drag_item_0_down_then_up(tester, || {
                assert_eq!(on_reorder_call_count.get(), 1);
                assert_eq!(*items.borrow(), [1, 0, 2, 3, 4]);
            });
            assert_eq!(on_reorder_call_count.get(), 2);
            assert_eq!(*items.borrow(), [0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_list_view_calls_on_reorder_item_callback_correctly() {
        test_widgets(|tester| {
            let items = five_items();
            pump_home(
                tester,
                rebuilding_children_list(&items, false, handle_reorder_item(&items)),
            );
            drag_item_0_down_then_up(tester, || {
                assert_eq!(*items.borrow(), [1, 0, 2, 3, 4]);
            });
            assert_eq!(*items.borrow(), [0, 1, 2, 3, 4]);
        });
    }

    #[test]
    #[should_panic(
        expected = "The onReorder callback is obsolete and is replaced by onReorderItem."
    )]
    fn reorderable_list_view_asserts_if_neither_on_reorder_and_on_reorder_item_are_provided() {
        let _ = ReorderableListView::new(Vec::new()).into_widget();
    }

    #[test]
    #[should_panic(
        expected = "The onReorder callback is obsolete and is replaced by onReorderItem."
    )]
    #[allow(deprecated)]
    fn reorderable_list_view_asserts_if_both_on_reorder_and_on_reorder_item_are_provided() {
        let _ = ReorderableListView::new(Vec::new())
            .on_reorder(|_app, _old, _new| {})
            .on_reorder_item(|_app, _old, _new| {})
            .into_widget();
    }

    fn builder_numbered(items: &Rc<RefCell<Vec<i32>>>) -> ReorderableListView {
        let count = items.borrow().len() as i32;
        let items = Rc::clone(items);
        ReorderableListView::builder(
            move |_app, _context, index| {
                let item = items.borrow()[index as usize];
                SizedBox::new()
                    .key(ValueKey::new(item).into())
                    .height(100.0)
                    .child(ReorderableDragStartListener::new(
                        Text::new(format!("item {item}")),
                        index,
                    ))
                    .into_widget()
            },
            count,
        )
    }

    #[test]
    #[allow(deprecated)]
    fn reorderable_list_view_builder_calls_old_on_reorder_callback_correctly() {
        test_widgets(|tester| {
            let on_reorder_call_count = Rc::new(Cell::new(0));
            let items = five_items();
            pump_home(
                tester,
                builder_numbered(&items)
                    .on_reorder(handle_old_reorder(&items, &on_reorder_call_count)),
            );
            drag_item_0_down_then_up(tester, || {
                assert_eq!(on_reorder_call_count.get(), 1);
                assert_eq!(*items.borrow(), [1, 0, 2, 3, 4]);
            });
            assert_eq!(on_reorder_call_count.get(), 2);
            assert_eq!(*items.borrow(), [0, 1, 2, 3, 4]);
        });
    }

    #[test]
    fn reorderable_list_view_builder_calls_on_reorder_item_callback_correctly() {
        test_widgets(|tester| {
            let items = five_items();
            pump_home(
                tester,
                builder_numbered(&items).on_reorder_item(handle_reorder_item(&items)),
            );
            drag_item_0_down_then_up(tester, || {
                assert_eq!(*items.borrow(), [1, 0, 2, 3, 4]);
            });
            assert_eq!(*items.borrow(), [0, 1, 2, 3, 4]);
        });
    }

    #[test]
    #[should_panic(
        expected = "The onReorder callback is obsolete and is replaced by onReorderItem."
    )]
    fn reorderable_list_view_builder_asserts_if_neither_on_reorder_and_on_reorder_item_are_provided()
     {
        let _ =
            ReorderableListView::builder(|_app, _context, _index| SizedBox::new().into_widget(), 0)
                .into_widget();
    }

    #[test]
    #[should_panic(
        expected = "The onReorder callback is obsolete and is replaced by onReorderItem."
    )]
    #[allow(deprecated)]
    fn reorderable_list_view_builder_asserts_if_both_on_reorder_and_on_reorder_item_are_provided() {
        let _ =
            ReorderableListView::builder(|_app, _context, _index| SizedBox::new().into_widget(), 0)
                .on_reorder(|_app, _old, _new| {})
                .on_reorder_item(|_app, _old, _new| {})
                .into_widget();
    }

    #[test]
    fn reorderable_list_view_throws_an_error_when_key_is_not_passed_to_its_children() {
        test_widgets(|tester| {
            let reorderable_list_view = ReorderableListView::builder(
                |_app, _context, index| {
                    SizedBox::new()
                        .child(Text::new(format!("Item {index}")))
                        .into_widget()
                },
                3,
            )
            .on_reorder_item(|_app, _old, _new| {});
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                pump_home(tester, reorderable_list_view);
            }));
            let message = panic_message(outcome.expect_err("the items have no key"));
            assert!(
                message.contains("Every item of ReorderableListView must have a key."),
                "{message}"
            );
        });
    }

    #[test]
    fn throws_an_error_if_no_overlay_present() {
        test_widgets(|tester| {
            let reorderable_list = ReorderableListView::new(["C", "B", "A"].map(|label| {
                SizedBox::new()
                    .width(100.0)
                    .height(100.0)
                    .key(string_key(label))
                    .child(Text::new(label))
                    .into_widget()
            }))
            .on_reorder_item(|_app, _old, _new| {});
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                tester.pump_widget(localized(
                    SizedBox::square(Some(100.0))
                        .child(Directionality::new(TextDirection::Ltr, reorderable_list)),
                ));
            }));
            let message = panic_message(outcome.expect_err("there is no Overlay"));
            assert!(message.contains("No Overlay widget found"), "{message}");
            assert!(
                message.contains("ReorderableListView widgets require an Overlay widget ancestor"),
                "{message}"
            );
        });
    }

    #[test]
    #[should_panic(
        expected = "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
    )]
    fn reorderable_list_view_asserts_on_both_non_null_item_extent_and_prototype_item() {
        let _ = ReorderableListView::new(Vec::new())
            .item_extent(30.0)
            .prototype_item(SizedBox::new())
            .on_reorder_item(|_app, _old, _new| {})
            .into_widget();
    }

    /// The extent cases' items: `SizedBox(key: ValueKey(numbers[index]), height: 20 +
    /// numbers[index] * 10)` over a drag listener of `Text('${numbers[index]}')`.
    fn number_builder() -> ReorderableListView {
        let numbers = [0, 1, 2];
        ReorderableListView::builder(
            move |_app, _context, index| {
                let number = numbers[index as usize];
                SizedBox::new()
                    .key(ValueKey::new(number).into())
                    // children with different heights
                    .height(f64::from(20 + number * 10))
                    .child(ReorderableDragStartListener::new(
                        Text::new(number.to_string()),
                        index,
                    ))
                    .into_widget()
            },
            3,
        )
    }

    #[test]
    #[should_panic(
        expected = "You can only pass one of itemExtent, prototypeItem and itemExtentBuilder."
    )]
    fn reorderable_list_view_builder_asserts_on_both_non_null_item_extent_and_prototype_item() {
        let _ = number_builder()
            .item_extent(30.0)
            .prototype_item(SizedBox::new())
            .on_reorder_item(|_app, _old, _new| {})
            .into_widget();
    }

    fn number_heights(tester: &WidgetTester) -> [f64; 3] {
        [0, 1, 2].map(|i| tester.get_size(find::text(&i.to_string())).height())
    }

    #[test]
    fn if_item_extent_is_non_null_children_have_same_extent_in_the_scroll_direction() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Scaffold::new().body(
                    number_builder()
                        .item_extent(30.0)
                        .on_reorder_item(|_app, _old, _new| {}),
                ),
            );
            assert_eq!(number_heights(tester), [30.0, 30.0, 30.0]);
        });
    }

    #[test]
    fn if_prototype_item_is_non_null_children_have_same_extent_in_the_scroll_direction() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Scaffold::new().body(
                    number_builder()
                        .prototype_item(SizedBox::new().height(30.0).child(Text::new("3")))
                        .on_reorder_item(|_app, _old, _new| {}),
                ),
            );
            assert_eq!(number_heights(tester), [30.0, 30.0, 30.0]);
        });
    }

    #[test]
    fn drag_boundary_defines_the_boundary_for_reorderable_list_view() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Scaffold::new().body(
                    Container::new()
                        .margin(EdgeInsetsGeometry::only(0.0, 100.0, 0.0, 0.0))
                        .height(300.0)
                        .child(DragBoundary::new(
                            ReorderableListView::builder(
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
                            .on_reorder_item(|_app, _old, _new| {}),
                        )),
                ),
            );
            let center = tester.get_center(find::text("0"));
            let mut drag = start_gesture(tester, center);
            tester.pump_by(K_LONG_PRESS_TIMEOUT);
            tester.move_by(&mut drag, Offset::new(0.0, -400.0), true);
            tester.pump_and_settle();
            assert_eq!(
                tester.get_top_left(find::text("0")),
                Offset::new(0.0, 100.0)
            );
            tester.up(&drag);
            tester.pump_and_settle();

            let center = tester.get_center(find::text("0"));
            let mut drag = start_gesture(tester, center);
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

    /// Dart's two mouse cursor cases: hover the first drag handle, press and hold it, release.
    fn assert_drag_handle_cursors(
        tester: &mut WidgetTester,
        list: ReorderableListView,
        hovering: MouseCursorRef,
        dragging: MouseCursorRef,
    ) {
        pump_home(tester, Scaffold::new().body(list));

        let handle = tester.get_center(find::by_icon(&Icons::drag_handle()).first());
        let mut mouse = TestPointer::mouse(1, K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, handle);
        tester.pump();
        assert!(*tester.active_cursor(1) == *hovering);
        tester.down(&mut mouse, handle);
        tester.pump_by(K_LONG_PRESS_TIMEOUT);
        assert!(*tester.active_cursor(1) == *dragging);
        tester.up(&mouse);
        tester.pump_and_settle();
        assert!(*tester.active_cursor(1) == *hovering);
    }

    fn five_text_items() -> ReorderableListView {
        ReorderableListView::builder(
            |_app, _context, index| {
                ReorderableDragStartListener::new(Text::new(index.to_string()), index)
                    .key(ValueKey::new(index).into())
                    .into_widget()
            },
            5,
        )
        .on_reorder_item(|_app, _old, _new| {})
    }

    #[test]
    fn mouse_cursor_behavior_on_the_drag_handle() {
        test_widgets_variant(TargetPlatformVariant::desktop(), |tester| {
            assert_drag_handle_cursors(
                tester,
                five_text_items(),
                SystemMouseCursors::GRAB.into(),
                SystemMouseCursors::GRABBING.into(),
            );
        });
    }

    #[test]
    fn mouse_cursor_behavior_on_the_drag_handle_can_be_provided() {
        test_widgets_variant(TargetPlatformVariant::desktop(), |tester| {
            assert_drag_handle_cursors(
                tester,
                five_text_items().mouse_cursor(WidgetStateMouseCursor::from_map(vec![
                    (WidgetState::Dragged.into(), SystemMouseCursors::COPY.into()),
                    (WidgetState::any(), SystemMouseCursors::RESIZE_COLUMN.into()),
                ])),
                SystemMouseCursors::RESIZE_COLUMN.into(),
                SystemMouseCursors::COPY.into(),
            );
        });
    }

    #[test]
    fn reorderable_list_view_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Center::new().child(
                    SizedBox::shrink().child(
                        ReorderableListView::new([
                            Text::new("X").key(string_key("x")).into_widget(),
                            Text::new("Y").key(string_key("y")).into_widget(),
                        ])
                        .on_reorder_item(|_app, _old, _new| {}),
                    ),
                ),
            );
            assert_eq!(
                tester.get_size(find::by_type::<ReorderableListView>()),
                Size::ZERO
            );
        });
    }
}
